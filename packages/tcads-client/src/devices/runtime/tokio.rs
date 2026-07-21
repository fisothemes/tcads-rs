use super::symbol_cache::{SymbolCache, SymbolEntry};
use crate::devices::tokio::AdsDevice;
use crate::notif_guard::tokio::NotificationGuard;
use std::marker::PhantomData;
use std::net::ToSocketAddrs;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tcads_core::{
    AdsError, AdsNotificationAttrib, AdsNotificationSampleOwned, AdsSymbolInfo,
    AdsSymbolInfoIteratorOwned, AdsSymbolUploadInfo, AdsSymbolUploadInfoV3, AdsTransMode,
    AdsTypeInfo, AdsTypeInfoIteratorOwned, AmsAddr, AmsPort, IndexGroup, IndexOffset,
    NotificationHandle, SumReadRequest, SumReadWriteRequest, SumWriteRequest, SymbolHandle,
};
use tokio::sync::OnceCell;
use tokio::sync::mpsc::UnboundedReceiver as Receiver;
use tokio::sync::mpsc::error::TryRecvError;

/// A high-level client for interacting with a specific TwinCAT runtime.
///
/// `RuntimeDevice` provides specialized methods for querying a target runtime device's memory
/// layout, including Symbol (variable) metadata, values, and Data Type definitions.
///
/// It is bound to a single target address ([`AmsAddr`]). Common target ports include:
/// - **851** (and **801–899**): PLC runtimes (851 is the default first TC3 PLC task).
/// - **301–399**: FreeTasks.
#[derive(Clone)]
pub struct RuntimeDevice {
    device: AdsDevice,
    target: AmsAddr,
    symbols: OnceCell<Arc<SymbolCache>>,
}

impl RuntimeDevice {
    /// Connects to the target run-time ADS device using its AMS address via the local AMS router.
    ///
    /// See [`AdsDevice::connect`] for further details.
    pub async fn connect(
        target: impl Into<AmsAddr>,
        timeout: impl Into<Option<Duration>>,
    ) -> crate::Result<Self> {
        Ok(Self::new(AdsDevice::connect(timeout).await?, target.into()))
    }

    /// Connects to a target ADS device whose runtime [`AmsNetId`](tcads_core::AmsNetId) is the
    /// same as the local Net ID via the local AMS router.
    ///
    /// This is usually the case when you have configured the Target System on TwinCAT to be
    /// `<Local>`. This will not work for UmRT (User-Mode Runtime) or other target systems. Use
    /// [`RuntimeDevice::connect`] for those.
    ///
    /// See [`AdsDevice::connect`] and [`AdsDevice::get_local_net_id`] for more details.
    pub async fn connect_local(
        port: impl Into<AmsPort>,
        timeout: impl Into<Option<Duration>>,
    ) -> crate::Result<Self> {
        let device = AdsDevice::connect(timeout.into()).await?;
        let target = AmsAddr::new(device.get_local_net_id().await?, port);
        Ok(Self::new(device, target))
    }

    /// Connects to a target ADS device using a remote AMS router.
    ///
    /// Use this if the system doesn't have a local router. The `source` address must be
    /// pre-configured as a static route on the remote router. This is usually found in the
    /// `StaticRoutes.xml` file on the device's disk.
    ///
    /// See [`AdsDevice::connect_remote`] for details.
    pub async fn connect_remote(
        addr: impl ToSocketAddrs,
        source: AmsAddr,
        timeout: impl Into<Option<Duration>>,
    ) -> crate::Result<Self> {
        Ok(Self::new(
            AdsDevice::connect_remote(addr, source, timeout).await?,
            source,
        ))
    }

    /// Creates an instance of the [`RuntimeDevice`] by wrapping an existing [`AdsDevice`] and
    /// target address.
    ///
    /// Useful if you are sharing a connection with other ADS devices
    /// i.e. the [`Logger`](crate::devices::blocking::Logger) ADS device.
    pub const fn new(device: AdsDevice, target: AmsAddr) -> Self {
        Self {
            device,
            target,
            symbols: OnceCell::const_new(),
        }
    }

    /// Shuts down the underlying connection.
    ///
    /// See [`AdsDevice::shutdown`] for more details.
    pub async fn shutdown(self) {
        self.device.shutdown().await
    }

    /// Returns the target AMS Address.
    pub fn target(&self) -> AmsAddr {
        self.target
    }

    /// Returns a reference to the underlying [`AdsDevice`].
    pub fn get_ref(&self) -> &AdsDevice {
        &self.device
    }

    /// Fetches and caches the symbol upload metadata from the PLC.
    ///
    /// This is metadata describing the symbols and data types available on a PLC runtime.
    pub async fn get_upload_info(&self) -> crate::Result<AdsSymbolUploadInfo> {
        let bytes = self
            .device
            .read(
                self.target,
                IndexGroup::SYMBOL_UPLOAD_INFO2,
                IndexOffset::ZERO,
                AdsSymbolUploadInfoV3::LENGTH as u32,
            )
            .await?;
        Ok(AdsSymbolUploadInfo::try_from(bytes.as_ref()).map_err(AdsError::from)?)
    }

    /// Fetches the current Symbol Version of the PLC runtime.
    ///
    /// The Symbol Version changes whenever the PLC's symbol configuration is
    /// updated (e.g. during a Login with download or a complete program reactivation).
    pub async fn get_symbol_version(&self) -> crate::Result<u8> {
        let bytes = self
            .device
            .read(
                self.target,
                IndexGroup::SYMBOL_VERSION,
                IndexOffset::ZERO,
                1,
            )
            .await?;

        if bytes.len() != 1 {
            return Err(AdsError::UnexpectedDataLength {
                expected: 1,
                got: bytes.len(),
            }
            .into());
        }

        Ok(bytes[0])
    }

    /// Subscribes to Symbol Version change notifications.
    ///
    /// The Symbol Version increments on a Login with download. An Online Change does not
    /// change it, confirmed by reading [`get_symbol_version`](Self::get_symbol_version)
    /// before and after an Online Change: the value was unchanged and no notification
    /// fired. This lines up with what Online Change is for, preserving the existing
    /// symbol layout for already-connected clients, so nothing here needs to invalidate.
    ///
    /// A Reactivation tears down the underlying connection, which ends this subscription:
    /// the receiver's next call returns [`Err(Error::Disconnected)`](crate::Error::Disconnected)
    /// rather than a version byte. Treat that error as a stronger signal than any
    /// individual notification. It means you must reconnect and unconditionally re-fetch
    /// the symbol version (and any [`AdsSymbolInfo`]/[`AdsTypeInfo`]/[`SymbolHandle`] you
    /// were relying on) rather than assume nothing changed just because you didn't see a
    /// notification for it.
    ///
    /// Returns a [`SymbolVersionReceiver`] that decodes each notification into the new
    /// version byte. The subscription is cancelled automatically when the receiver is
    /// dropped, or explicitly via [`SymbolVersionReceiver::unsubscribe`].
    pub async fn subscribe_symbol_version(
        &self,
    ) -> crate::Result<(SymbolVersionReceiver, NotificationHandle)> {
        let (rx, notif_handle) = self
            .device
            .add_notification(
                self.target,
                IndexGroup::SYMBOL_VERSION,
                IndexOffset::ZERO,
                AdsNotificationAttrib::new(1, AdsTransMode::ServerOnChange, 0, 0),
            )
            .await?;

        let guard = NotificationGuard::new(notif_handle, self.target, self.device.clone());
        let rx = SymbolVersionReceiver::new(rx, guard);
        Ok((rx, notif_handle))
    }

    /// Fetches a symbol handle by its instance path (e.g. `"MAIN.nCount"`)
    pub async fn get_handle_by_name(&self, name: impl AsRef<str>) -> crate::Result<SymbolHandle> {
        let resp = self
            .device
            .read_write(
                self.target,
                IndexGroup::SYMBOL_HANDLE_BY_NAME,
                IndexOffset::ZERO,
                4,
                name.as_ref(),
            )
            .await?;

        if resp.len() != 4 {
            return Err(AdsError::UnexpectedDataLength {
                expected: 4,
                got: resp.len(),
            }
            .into());
        }

        Ok(SymbolHandle::from_bytes([
            resp[0], resp[1], resp[2], resp[3],
        ]))
    }

    /// Fetches multiple symbol handles in a single network transaction.
    pub async fn get_multi_handles_by_name<'a, S: AsRef<str> + 'a + ?Sized>(
        &self,
        names: impl IntoIterator<Item = &'a S>,
    ) -> crate::Result<impl Iterator<Item = crate::Result<SymbolHandle>>> {
        let reqs: Vec<_> = names
            .into_iter()
            .map(|name| {
                SumReadWriteRequest::new(
                    IndexGroup::SYMBOL_HANDLE_BY_NAME,
                    IndexOffset::ZERO,
                    4,
                    name.as_ref().as_bytes(),
                )
            })
            .collect();

        let resp = self.device.read_write_multi(self.target, &reqs).await?;

        let results = resp
            .into_iter()
            .map(|res| match res {
                Ok(chunk) => Ok(SymbolHandle::from_bytes([
                    chunk[0], chunk[1], chunk[2], chunk[3],
                ])),
                Err(err) => Err(crate::Error::from(err)),
            })
            .collect::<Vec<_>>();

        Ok(results.into_iter())
    }

    /// Releases a symbol handle.
    pub async fn release_handle(&self, handle: SymbolHandle) -> crate::Result<()> {
        self.device
            .write(
                self.target,
                IndexGroup::SYMBOL_RELEASE_HANDLE,
                IndexOffset::ZERO,
                handle.to_bytes(),
            )
            .await
    }

    /// Releases multiple symbol handles.
    pub async fn release_multi_handles(
        &self,
        handles: impl AsRef<[SymbolHandle]>,
    ) -> crate::Result<impl Iterator<Item = crate::Result<()>>> {
        let reqs: Vec<_> = handles
            .as_ref()
            .iter()
            .map(|handle| {
                SumWriteRequest::new(
                    IndexGroup::SYMBOL_RELEASE_HANDLE,
                    IndexOffset::ZERO,
                    handle.as_bytes(),
                )
            })
            .collect();

        let resp = self.device.write_multi(self.target, &reqs).await?;

        let results = resp
            .into_iter()
            .map(|res| match res {
                Ok(()) => Ok(()),
                Err(err) => Err(crate::Error::from(err)),
            })
            .collect::<Vec<_>>();

        Ok(results.into_iter())
    }

    /// Reads a value from the symbol as raw bytes using a handle.
    pub async fn read_bytes_by_handle(
        &self,
        handle: SymbolHandle,
        length: usize,
    ) -> crate::Result<Vec<u8>> {
        self.device
            .read(
                self.target,
                IndexGroup::SYMBOL_VALUE_BY_HANDLE,
                handle.as_u32().into(),
                length as u32,
            )
            .await
    }

    /// Reads multiple values as bytes using their handles.
    pub async fn read_multi_as_bytes_by_handle<S: AsRef<SymbolHandle>>(
        &self,
        items: impl AsRef<[(S, usize)]>,
    ) -> crate::Result<impl Iterator<Item = crate::Result<Vec<u8>>>> {
        let reqs: Vec<_> = items
            .as_ref()
            .iter()
            .map(|(handle, len)| {
                SumReadRequest::new(
                    IndexGroup::SYMBOL_VALUE_BY_HANDLE,
                    handle.as_ref().as_u32().into(),
                    *len as u32,
                )
            })
            .collect();

        let resp = self.device.read_multi(self.target, &reqs).await?;

        let results = resp
            .into_iter()
            .map(|res| match res {
                Ok(chunk) => Ok(chunk.to_vec()),
                Err(err) => Err(crate::Error::from(err)),
            })
            .collect::<Vec<_>>();

        Ok(results.into_iter())
    }

    /// Reads raw bytes from a symbol directly using its absolute memory location
    /// ([`IndexGroup`] and [`IndexOffset`]) provided by its [`AdsSymbolInfo`].
    pub async fn read_bytes_by_info(&self, info: &AdsSymbolInfo) -> crate::Result<Vec<u8>> {
        self.device
            .read(
                self.target,
                info.index_group(),
                info.index_offset(),
                info.size(),
            )
            .await
    }

    /// Reads raw bytes from multiple symbols directly using their absolute memory locations
    /// ([`IndexGroup`] and [`IndexOffset`]) provided by their [`AdsSymbolInfo`]s in a single
    /// network transaction.
    pub async fn read_multi_as_bytes_by_info(
        &self,
        infos: impl AsRef<[AdsSymbolInfo]>,
    ) -> crate::Result<impl Iterator<Item = crate::Result<Vec<u8>>>> {
        let reqs: Vec<_> = infos
            .as_ref()
            .iter()
            .map(|info| SumReadRequest::new(info.index_group(), info.index_offset(), info.size()))
            .collect();

        let resp = self.device.read_multi(self.target, &reqs).await?;

        let results = resp
            .into_iter()
            .map(|res| match res {
                Ok(chunk) => Ok(chunk.to_vec()),
                Err(err) => Err(crate::Error::from(err)),
            })
            .collect::<Vec<_>>();

        Ok(results.into_iter())
    }

    /// Reads a symbol's value by instance path and deserializes it into `T`.
    ///
    /// Symbol info, type closure, and handle are resolved on first use and cached;
    /// subsequent reads of the same path cost a single ADS read. Paths support
    /// TwinCAT pointer dereference syntax (`"MAIN.pValue^.nValue"`); reference
    /// (`REFERENCE TO`) symbols read as their base type automatically.
    ///
    /// Whenever a symbol version changes, the PLC invalidates all handles; this returns
    /// [`Error::HandleInvalidated`](crate::Error::HandleInvalidated) after flushing
    /// the cache, and the caller decides whether to retry.
    ///
    /// See [`subscribe_symbol_version`](Self::subscribe_symbol_version) for more info on version
    /// changes.
    pub async fn read_value<T>(&self, path: impl AsRef<str>) -> crate::Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let path = path.as_ref();
        let (cache, entry) = self.resolve_symbol(path).await?;

        let (handle, size, type_info) = {
            let guard = entry.read()?;
            (
                guard
                    .handle()
                    .expect("resolve_symbol always attaches a handle"),
                guard.size(),
                guard.type_info().clone(),
            )
        };

        let bytes = self
            .read_bytes_by_handle(handle, size as usize)
            .await
            .map_err(|err| self.map_stale(err, &cache, path))?;

        tcads_serde::from_bytes(&bytes, &type_info, &*cache.types()?).map_err(Into::into)
    }

    /// Subscribes to value-change notifications for a symbol by instance path.
    ///
    /// Resolves the symbol the same way [`read_value`](Self::read_value) does
    /// (cached after the first call), then subscribes on its handle via
    /// `SYMBOL_VALUE_BY_HANDLE`. `trans_mode`, `max_delay`, and `cycle_time`
    /// are passed straight through to [`AdsNotificationAttrib`]; use
    /// [`AdsTransMode::ServerOnChange`] for push-on-change, or
    /// [`AdsTransMode::ServerCyclic`] with a `cycle_time` for polling at a
    /// fixed interval, useful for fast-changing values where on-change would
    /// flood the subscription.
    ///
    /// Returns a [`ValueReceiver<T>`] that decodes each sample into `T`.
    ///
    /// # Handle invalidation
    ///
    /// Unlike `read_value`/`write_value`, this subscription has no read/write
    /// call of its own that can fail with `AdsErrDeviceSymbolVersionInvalid`
    /// when an online change invalidates the handle backing it: it's push,
    /// not pull. Instead, the PLC pushes a single zero-length sample on this
    /// subscription to signal exactly that. [`ValueReceiver`] treats a
    /// zero-length sample as [`Error::HandleInvalidated`](crate::Error::HandleInvalidated),
    /// flushing the symbol cache the same as a failed `read_value`/`write_value`
    /// call would. It does **not** auto-resubscribe: the online change may have
    /// altered the symbol's type or size, so silently reinterpreting whatever
    /// arrives next under a possibly-changed layout would be worse than
    /// surfacing the problem. Call `subscribe_value` again once you've decided
    /// that's safe.
    pub async fn subscribe_value<T>(
        &self,
        path: impl AsRef<str>,
        trans_mode: AdsTransMode,
        max_delay: u32,
        cycle_time: u32,
    ) -> crate::Result<(ValueReceiver<T>, NotificationHandle)>
    where
        T: serde::de::DeserializeOwned,
    {
        let path = path.as_ref();
        let (cache, entry) = self.resolve_symbol(path).await?;

        let (handle, size, type_info) = {
            let guard = entry.read()?;
            (
                guard
                    .handle()
                    .expect("resolve_symbol always attaches a handle"),
                guard.size(),
                guard.type_info().clone(),
            )
        };

        let (rx, notif_handle) = self
            .device
            .add_notification(
                self.target,
                IndexGroup::SYMBOL_VALUE_BY_HANDLE,
                handle.as_u32().into(),
                AdsNotificationAttrib::new(size, trans_mode, max_delay, cycle_time),
            )
            .await?;

        let guard = NotificationGuard::new(notif_handle, self.target, self.device.clone());
        let rx = ValueReceiver::new(rx, guard, cache, Arc::from(path), type_info);
        Ok((rx, notif_handle))
    }

    /// Writes a value to the symbol as raw bytes using a handle.
    pub async fn write_bytes_by_handle(
        &self,
        handle: SymbolHandle,
        data: impl Into<Vec<u8>>,
    ) -> crate::Result<()> {
        self.device
            .write(
                self.target,
                IndexGroup::SYMBOL_VALUE_BY_HANDLE,
                handle.as_u32().into(),
                data,
            )
            .await
    }

    pub async fn write_multi_as_bytes_by_handle<S: AsRef<SymbolHandle>, D: AsRef<[u8]>>(
        &self,
        items: impl AsRef<[(S, D)]>,
    ) -> crate::Result<impl Iterator<Item = crate::Result<()>>> {
        let reqs: Vec<_> = items
            .as_ref()
            .iter()
            .map(|(handle, data)| {
                SumWriteRequest::new(
                    IndexGroup::SYMBOL_VALUE_BY_HANDLE,
                    handle.as_ref().as_u32().into(),
                    data.as_ref(),
                )
            })
            .collect();

        let resp = self.device.write_multi(self.target, &reqs).await?;

        let results = resp
            .into_iter()
            .map(|res| match res {
                Ok(()) => Ok(()),
                Err(err) => Err(crate::Error::from(err)),
            })
            .collect::<Vec<_>>();

        Ok(results.into_iter())
    }

    /// Writes raw bytes to a symbol directly using its absolute memory location
    /// ([`IndexGroup`] and [`IndexOffset`]) provided by its [`AdsSymbolInfo`].
    pub async fn write_bytes_by_info(
        &self,
        info: &AdsSymbolInfo,
        data: impl Into<Vec<u8>>,
    ) -> crate::Result<()> {
        self.device
            .write(self.target, info.index_group(), info.index_offset(), data)
            .await
    }

    /// Writes raw bytes to multiple symbols directly using their absolute memory locations
    /// provided by their [`AdsSymbolInfo`] in a single network transaction.
    pub async fn write_multi_as_bytes_by_info<S: AsRef<AdsSymbolInfo>, D: AsRef<[u8]>>(
        &self,
        items: impl AsRef<[(S, D)]>,
    ) -> crate::Result<impl Iterator<Item = crate::Result<()>>> {
        let reqs: Vec<_> = items
            .as_ref()
            .iter()
            .map(|(info, data)| {
                SumWriteRequest::new(
                    info.as_ref().index_group(),
                    info.as_ref().index_offset(),
                    data.as_ref(),
                )
            })
            .collect();

        let resp = self.device.write_multi(self.target, &reqs).await?;

        let results = resp
            .into_iter()
            .map(|res| match res {
                Ok(()) => Ok(()),
                Err(err) => Err(crate::Error::from(err)),
            })
            .collect::<Vec<_>>();

        Ok(results.into_iter())
    }

    /// Serializes `value` and writes it to the symbol at `path`.
    ///
    /// `value` must cover every field of the symbol's type. For types containing
    /// function block instances (directly, in fields, or as array elements) the
    /// write is a read-modify-write cycle: current bytes are read, the value is
    /// serialized over them (preserving vtable headers and any other hidden
    /// bytes), and the result is written back. Plain structs, arrays, strings,
    /// and primitives are written directly with no extra read.
    ///
    /// Pointer-typed fields are written as the opaque integer the value supplies;
    /// preserving their contents is the caller's responsibility.
    ///
    /// See [`read_value`](Self::read_value) for caching and invalidation behavior.
    pub async fn write_value<T>(&self, path: impl AsRef<str>, value: &T) -> crate::Result<()>
    where
        T: serde::Serialize,
    {
        let path = path.as_ref();
        let (cache, entry) = self.resolve_symbol(path).await?;

        let (handle, size, requires_rmw, type_info) = {
            let guard = entry.read()?;
            (
                guard
                    .handle()
                    .expect("resolve_symbol always attaches a handle"),
                guard.size(),
                guard.requires_rmw(),
                guard.type_info().clone(),
            )
        };

        let mut buf = if requires_rmw {
            self.read_bytes_by_handle(handle, size as usize)
                .await
                .map_err(|err| self.map_stale(err, &cache, path))?
        } else {
            vec![0u8; size as usize]
        };

        tcads_serde::to_bytes(value, &mut buf, &type_info, &*cache.types()?)?;

        self.write_bytes_by_handle(handle, buf)
            .await
            .map_err(|err| self.map_stale(err, &cache, path))
    }

    /// Flushes the symbol cache: every cached handle, symbol entry, and type.
    ///
    /// Call after a symbol-version notification or reconnect. Stale handles are
    /// not released on the PLC; it already discarded them.
    pub fn invalidate_symbol_cache(&self) {
        if let Some(cache) = self.symbols.get() {
            cache.clear();
        }
    }

    /// Fetches the entire type dictionary and symbol table in two bulk round
    /// trips and populates the cache, so every symbol's metadata (type,
    /// size, whether it needs read-modify-write) is resolved before any
    /// [`read_value`](Self::read_value)/[`write_value`](Self::write_value)
    /// call touches it.
    ///
    /// # Note
    ///
    /// Costs two bulk transfers proportional to project size (hundreds of KB
    /// to a few MB on large projects), paid up front. Call it once after
    /// connecting if you'd rather pay that eagerly than have each symbol's
    /// first access pay its own smaller lazy cost.
    pub async fn preload(&self) -> crate::Result<()> {
        let cache = self.symbol_cache().await?;

        cache.insert_types(
            self.get_all_type_infos()
                .await?
                .collect::<Result<Vec<_>, _>>()?,
        )?;

        for info in self.get_all_symbol_infos().await? {
            let info = info?;

            if cache.get(info.name())?.is_some() {
                continue;
            }

            let entry = cache.resolve_entry(info.type_name(), info.size())?;
            cache.insert(Arc::from(info.name()), entry)?;
        }

        Ok(())
    }

    /// Fetches the metadata for a specific Symbol by its instance path (e.g. `"MAIN.nCount"`).
    pub async fn get_symbol_info(&self, name: impl AsRef<str>) -> crate::Result<AdsSymbolInfo> {
        let bytes = self
            .device
            .read_write(
                self.target,
                IndexGroup::SYMBOL_INFO_BY_NAME_EX,
                IndexOffset::ZERO,
                1_048_576,
                name.as_ref(),
            )
            .await?;

        Ok(AdsSymbolInfo::try_from(bytes.as_ref())?)
    }

    /// Fetches metadata for multiple TwinCAT symbols by their instance paths in
    /// a single network transaction.
    pub async fn get_multi_symbol_infos<'a, S: AsRef<str> + 'a + ?Sized>(
        &self,
        names: impl IntoIterator<Item = &'a S>,
    ) -> crate::Result<impl Iterator<Item = crate::Result<AdsSymbolInfo>>> {
        let reqs: Vec<_> = names
            .into_iter()
            .map(|name| {
                SumReadWriteRequest::new(
                    IndexGroup::SYMBOL_INFO_BY_NAME_EX,
                    IndexOffset::ZERO,
                    1_048_576,
                    name.as_ref().as_bytes(),
                )
            })
            .collect();

        let results: Vec<crate::Result<AdsSymbolInfo>> = self
            .device
            .read_write_multi(self.target, &reqs)
            .await?
            .iter()
            .map(|res| match res {
                Ok(chunk) => AdsSymbolInfo::try_from(chunk).map_err(crate::Error::from),
                Err(err) => Err(crate::Error::from(err)),
            })
            .collect();

        Ok(results.into_iter())
    }

    /// Downloads the entire Symbol dictionary from the PLC.
    ///
    /// This executes a single bulk transfer. This method returns a lazy iterator that parses each
    /// [`AdsSymbolInfo`] struct from the returned network binary blob sequentially as you
    /// consume it.
    pub async fn get_all_symbol_infos(
        &self,
    ) -> crate::Result<impl Iterator<Item = crate::Result<AdsSymbolInfo>>> {
        let info = self.get_upload_info().await?;

        let blob_size = info.symbol_blob_size();

        let raw_blob = self
            .device
            .read(
                self.target,
                IndexGroup::SYMBOL_UPLOAD,
                IndexOffset::ZERO,
                blob_size,
            )
            .await?;

        Ok(AdsSymbolInfoIteratorOwned::new(raw_blob).map(|res| res.map_err(crate::Error::from)))
    }

    /// Fetches the Data Type definition of a specific type by name (e.g. `"BOOL"`).
    pub async fn get_type_info(&self, name: impl AsRef<str>) -> crate::Result<AdsTypeInfo> {
        let bytes = self
            .device
            .read_write(
                self.target,
                IndexGroup::DATA_TYPE_INFO_BY_NAME_EX,
                IndexOffset::ZERO,
                1_048_576,
                name.as_ref(),
            )
            .await?;

        Ok(AdsTypeInfo::try_from(bytes.as_ref())?)
    }

    /// Fetches multiple Data Type definitions in a single network transaction.
    pub async fn get_multi_type_infos<'a, S: AsRef<str> + 'a + ?Sized>(
        &self,
        names: impl IntoIterator<Item = &'a S>,
    ) -> crate::Result<impl Iterator<Item = crate::Result<AdsTypeInfo>>> {
        let reqs: Vec<_> = names
            .into_iter()
            .map(|name| {
                SumReadWriteRequest::new(
                    IndexGroup::DATA_TYPE_INFO_BY_NAME_EX,
                    IndexOffset::ZERO,
                    1_048_576, // assumed max size of a single entry, router will return the actual size
                    name.as_ref().as_bytes(),
                )
            })
            .collect();

        let results: Vec<crate::Result<AdsTypeInfo>> = self
            .device
            .read_write_multi(self.target, &reqs)
            .await?
            .iter()
            .map(|res| match res {
                Ok(chunk) => AdsTypeInfo::try_from(chunk).map_err(crate::Error::from),
                Err(err) => Err(crate::Error::from(err)),
            })
            .collect();

        Ok(results.into_iter())
    }

    /// Downloads the entire Data Type dictionary from the PLC.
    ///
    /// The network request is made immediately to fetch the entire type blob,
    /// but the parsing happens lazily as you consume the returned iterator.
    pub async fn get_all_type_infos(
        &self,
    ) -> crate::Result<impl Iterator<Item = crate::Result<AdsTypeInfo>>> {
        let size = self
            .get_upload_info()
            .await?
            .data_type_blob_size()
            .unwrap_or(1_048_576);

        let raw_blob = self
            .device
            .read(
                self.target,
                IndexGroup::DATA_TYPE_UPLOAD,
                IndexOffset::ZERO,
                size,
            )
            .await?;

        Ok(AdsTypeInfoIteratorOwned::new(raw_blob).map(|res| res.map_err(crate::Error::from)))
    }

    /// Explicitly cancels a subscription to any notification created by this device.
    ///
    /// The receiver (i.e. [`ValueReceiver`] or [`SymbolVersionReceiver`]) associated with this
    /// handle will return [`Err(Error::Disconnected)`](crate::Error::Disconnected) on its next call.
    pub async fn unsubscribe_notification(&self, handle: NotificationHandle) -> crate::Result<()> {
        self.device.delete_notification(self.target, handle).await
    }

    /// Returns the lazily initialised [`SymbolCache`], creating it on first use.
    ///
    /// Creation needs the target's platform pointer size (one `get_upload_info`
    /// round trip). Falls back to 8 bytes if the target doesn't report one (V1/V2
    /// upload info), matching 64-bit runtimes.
    async fn symbol_cache(&self) -> crate::Result<Arc<SymbolCache>> {
        self.symbols
            .get_or_try_init(|| async {
                let ptr_size = self
                    .get_upload_info()
                    .await?
                    .platform_ptr_size()
                    .unwrap_or(8);
                Ok::<_, crate::Error>(Arc::new(SymbolCache::new(ptr_size)))
            })
            .await
            .cloned()
    }

    /// Resolves a symbol path to a cached, handle-bearing [`SymbolEntry`],
    /// fetching symbol info, the missing part of its type closure, and a handle
    /// from the PLC on a cache miss.
    async fn resolve_symbol(
        &self,
        path: &str,
    ) -> crate::Result<(Arc<SymbolCache>, Arc<RwLock<SymbolEntry>>)> {
        let cache = self.symbol_cache().await?;

        if let Some(entry) = cache.get(path)? {
            let has_handle = entry
                .read()
                .map_err(|_| crate::Error::PoisonedLock)?
                .handle()
                .is_some();
            if has_handle {
                return Ok((cache, entry));
            }

            let handle = self.get_handle_by_name(path).await?;
            if cache.set_handle(path, handle)? {
                return Ok((cache, entry));
            }
        }

        let info = self.get_symbol_info(path).await?;

        const MAX_FETCH_LEVELS: usize = 128;
        let mut entry = None;
        for _ in 0..=MAX_FETCH_LEVELS {
            let missing = cache.missing_types(info.type_name())?;
            if missing.is_empty() {
                entry = Some(cache.resolve_entry(info.type_name(), info.size())?);
                break;
            }
            let fetched: Vec<_> = self
                .get_multi_type_infos(&missing)
                .await?
                .collect::<Result<_, _>>()?;
            cache.insert_types(fetched)?;
        }
        let entry = entry.ok_or_else(|| {
            crate::Error::Serde(tcads_serde::Error::Custom(format!(
                "type closure of '{}' did not resolve within {MAX_FETCH_LEVELS} fetch levels",
                info.type_name(),
            )))
        })?;

        let handle = self.get_handle_by_name(path).await?;

        cache.insert(Arc::from(path), entry.with_handle(handle))?;

        let entry = cache
            .get(path)?
            .expect("just inserted, or raced with a concurrent insert of the same path");
        Ok((cache, entry))
    }

    /// Maps a stale-symbol-version failure to [`Error::HandleInvalidated`](crate::Error::HandleInvalidated),
    /// flushing the cache so the caller's retry re-resolves everything fresh.
    fn map_stale(&self, err: crate::Error, cache: &SymbolCache, path: &str) -> crate::Error {
        if matches!(
            err,
            crate::Error::AdsReturnCode(
                tcads_core::AdsReturnCode::AdsErrDeviceSymbolVersionInvalid
            )
        ) {
            cache.clear();
            crate::Error::HandleInvalidated(Arc::from(path))
        } else {
            err
        }
    }
}

/// A receiver for decoded Symbol Version change notifications.
///
/// Wraps the raw ADS notification channel and decodes each sample into the new version
/// byte on demand. The subscription is cancelled automatically when this is dropped.
///
/// Obtain one by calling [`RuntimeDevice::subscribe_symbol_version`].
pub struct SymbolVersionReceiver {
    rx: Receiver<AdsNotificationSampleOwned>,
    guard: NotificationGuard,
}

impl SymbolVersionReceiver {
    /// Creates a new instance of the [`SymbolVersionReceiver`].
    pub fn new(rx: Receiver<AdsNotificationSampleOwned>, guard: NotificationGuard) -> Self {
        Self { rx, guard }
    }

    /// Returns the notification handle for this subscription.
    pub fn handle(&self) -> NotificationHandle {
        self.guard.handle()
    }

    fn decode(sample: AdsNotificationSampleOwned) -> crate::Result<u8> {
        let data = sample.data();
        if data.len() != 1 {
            return Err(AdsError::UnexpectedDataLength {
                expected: 1,
                got: data.len(),
            }
            .into());
        }
        Ok(data[0])
    }

    /// Asynchronously awaits the next symbol version change.
    ///
    /// Returns [`Err(Error::Disconnected)`](crate::Error::Disconnected) when the
    /// subscription is cancelled or the connection is lost.
    ///
    /// # Note
    ///
    /// Unlike the blocking [`recv`](super::blocking::SymbolVersionReceiver::recv), this
    /// method requires `&mut self` because Tokio's [`Receiver`] does not support
    /// shared references.
    pub async fn recv(&mut self) -> crate::Result<u8> {
        let sample = self.rx.recv().await.ok_or(crate::Error::Disconnected)?;
        Self::decode(sample)
    }

    /// Returns the new symbol version if a change is immediately available, without
    /// awaiting.
    ///
    /// Returns `Ok(None)` if no sample is currently available,
    /// or [`Err(Error::Disconnected)`](crate::Error::Disconnected) if the
    /// subscription is cancelled or the connection is lost.
    pub fn try_recv(&mut self) -> crate::Result<Option<u8>> {
        match self.rx.try_recv() {
            Ok(sample) => Ok(Some(Self::decode(sample)?)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(crate::Error::Disconnected),
        }
    }

    /// Explicitly cancels the subscription, returning any error from the router.
    ///
    /// Dropping the receiver has the same effect but discards the result; prefer this
    /// when you want to know cancellation actually succeeded.
    pub async fn unsubscribe(self) -> crate::Result<()> {
        self.guard.cancel().await
    }
}

/// A receiver for decoded value-change notifications on a single symbol.
///
/// Wraps the raw ADS notification channel and decodes each sample into `T` on
/// demand. The subscription is cancelled automatically when this is
/// dropped, or explicitly via [`unsubscribe`](Self::unsubscribe).
///
/// Obtain one by calling [`RuntimeDevice::subscribe_value`].
pub struct ValueReceiver<T> {
    rx: Receiver<AdsNotificationSampleOwned>,
    guard: NotificationGuard,
    cache: Arc<SymbolCache>,
    path: Arc<str>,
    type_info: Arc<AdsTypeInfo>,
    _value: PhantomData<fn() -> T>,
}

impl<T> ValueReceiver<T>
where
    T: serde::de::DeserializeOwned,
{
    /// Creates a new instance of the [`ValueReceiver`].
    pub fn new(
        rx: Receiver<AdsNotificationSampleOwned>,
        guard: NotificationGuard,
        cache: Arc<SymbolCache>,
        path: Arc<str>,
        type_info: Arc<AdsTypeInfo>,
    ) -> Self {
        Self {
            rx,
            guard,
            cache,
            path,
            type_info,
            _value: PhantomData,
        }
    }

    /// Returns the notification handle for this subscription.
    pub fn handle(&self) -> NotificationHandle {
        self.guard.handle()
    }

    /// Returns the symbol path this subscription was created for.
    pub fn path(&self) -> &str {
        &self.path
    }

    fn decode(&self, sample: AdsNotificationSampleOwned) -> crate::Result<T> {
        let data = sample.data();
        if data.is_empty() {
            self.cache.clear();
            return Err(crate::Error::HandleInvalidated(self.path.clone()));
        }
        tcads_serde::from_bytes(data, &self.type_info, &*self.cache.types()?).map_err(Into::into)
    }

    /// Asynchronously awaits the next value change.
    ///
    /// Returns [`Err(Error::Disconnected)`](crate::Error::Disconnected) when the subscription is
    /// cancelled or the connection is lost, or
    /// [`Err(Error::HandleInvalidated)`](crate::Error::HandleInvalidated) if an online change
    /// invalidated the handle backing this subscription (see
    /// [`subscribe_value`](RuntimeDevice::subscribe_value)'s doc comment).
    pub async fn recv(&mut self) -> crate::Result<T> {
        let sample = self.rx.recv().await.ok_or(crate::Error::Disconnected)?;
        self.decode(sample)
    }

    /// Returns the next value if a sample is immediately available, without awaiting.
    ///
    /// Returns `Ok(None)` if no sample is currently available,
    /// [`Err(Error::Disconnected)`](crate::Error::Disconnected) if the
    /// subscription is cancelled or the connection is lost, or
    /// [`Err(Error::HandleInvalidated)`](crate::Error::HandleInvalidated) per [`recv`](Self::recv).
    pub fn try_recv(&mut self) -> crate::Result<Option<T>> {
        match self.rx.try_recv() {
            Ok(sample) => Ok(Some(self.decode(sample)?)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(crate::Error::Disconnected),
        }
    }

    /// Explicitly cancels the subscription, returning any error from the router.
    ///
    /// Dropping the receiver has the same effect but discards the result; prefer this
    /// when you want to know cancellation actually succeeded.
    pub async fn unsubscribe(self) -> crate::Result<()> {
        self.guard.cancel().await
    }
}
