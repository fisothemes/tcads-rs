use super::symbol_cache::{SymbolCache, SymbolEntry};
use crate::devices::blocking::AdsDevice;
use crate::notif_guard::blocking::NotificationGuard;
use indexmap::IndexSet;
use std::collections::VecDeque;
use std::hash::Hash;
use std::marker::PhantomData;
use std::net::ToSocketAddrs;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::{Arc, OnceLock, RwLock};
use std::time::Duration;
use tcads_core::{
    AdsError, AdsNotificationAttrib, AdsNotificationSampleOwned, AdsSymbolInfo,
    AdsSymbolInfoIteratorOwned, AdsSymbolUploadInfo, AdsSymbolUploadInfoV3, AdsTransMode,
    AdsTypeInfo, AdsTypeInfoIteratorOwned, AmsAddr, AmsPort, IndexGroup, IndexOffset,
    NotificationHandle, SumReadRequest, SumReadWriteRequest, SumWriteRequest, SymbolHandle,
};

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
    symbols: OnceLock<Arc<SymbolCache>>,
}

impl RuntimeDevice {
    /// Connects to the target run-time ADS device using its AMS address via the local AMS router.
    ///
    /// See [`AdsDevice::connect`] for further details.
    pub fn connect(
        target: impl Into<AmsAddr>,
        timeout: impl Into<Option<Duration>>,
    ) -> crate::Result<Self> {
        Ok(Self::new(AdsDevice::connect(timeout)?, target.into()))
    }

    /// Connects to a target ADS device whose runtime [`AmsNetId`](tcads_core::AmsNetId) is the
    /// same as the local Net ID via the local AMS router.
    ///
    /// This is usually the case when you have configured the Target System on TwinCAT to be
    /// `<Local>`. This will not work for UmRT (User-Mode Runtime) or other target systems. Use
    /// [`RuntimeDevice::connect`] for those.
    ///
    /// See [`AdsDevice::connect`] and [`AdsDevice::get_local_net_id`] for more details.
    pub fn connect_local(
        port: impl Into<AmsPort>,
        timeout: impl Into<Option<Duration>>,
    ) -> crate::Result<Self> {
        let device = AdsDevice::connect(timeout)?;
        let target = AmsAddr::new(device.get_local_net_id()?, port);
        Ok(Self::new(device, target))
    }

    /// Connects to a target ADS device using a remote AMS router.
    ///
    /// Use this if the system doesn't have a local router. The `source` address must be
    /// pre-configured as a static route on the remote router. This is usually found in the
    /// `StaticRoutes.xml` file on the device's disk.
    ///
    /// See [`AdsDevice::connect_remote`] for details.
    pub fn connect_remote(
        addr: impl ToSocketAddrs,
        source: impl Into<AmsAddr>,
        target: impl Into<AmsAddr>,
        timeout: impl Into<Option<Duration>>,
    ) -> crate::Result<Self> {
        Ok(Self::new(
            AdsDevice::connect_remote(addr, source.into(), timeout)?,
            target.into(),
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
            symbols: OnceLock::new(),
        }
    }

    /// Shuts down the underlying connection.
    ///
    /// See [`AdsDevice::shutdown`] for more details.
    pub fn shutdown(self) -> crate::Result<()> {
        self.device.shutdown()
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
    pub fn get_upload_info(&self) -> crate::Result<AdsSymbolUploadInfo> {
        let bytes = self.device.read(
            self.target,
            IndexGroup::SYMBOL_UPLOAD_INFO2,
            IndexOffset::ZERO,
            // Using the largest version because server will return the largest version it supports.
            AdsSymbolUploadInfoV3::LENGTH as u32,
        )?;

        let info = AdsSymbolUploadInfo::try_from_slice(&bytes).map_err(AdsError::from)?;

        Ok(info)
    }

    /// Fetches the current Symbol Version of the PLC runtime.
    ///
    /// The Symbol Version changes whenever the PLC's symbol configuration is
    /// updated (e.g. during a Login with download or a complete program reactivation).
    pub fn get_symbol_version(&self) -> crate::Result<u8> {
        let bytes = self.device.read(
            self.target,
            IndexGroup::SYMBOL_VERSION,
            IndexOffset::ZERO,
            1,
        )?;

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
    pub fn subscribe_symbol_version(
        &self,
    ) -> crate::Result<(SymbolVersionReceiver, NotificationHandle)> {
        let (rx, notif_handle) = self.device.add_notification(
            self.target,
            IndexGroup::SYMBOL_VERSION,
            IndexOffset::ZERO,
            AdsNotificationAttrib::new(1, AdsTransMode::ServerOnChange, 0, 0),
        )?;

        let guard = NotificationGuard::new(notif_handle, self.target, self.device.clone());
        let rx = SymbolVersionReceiver::new(rx, guard);
        Ok((rx, notif_handle))
    }

    /// Fetches a symbol handle by its instance path (e.g. `"MAIN.nCount"`)
    pub fn get_handle_by_name(&self, name: impl AsRef<str>) -> crate::Result<SymbolHandle> {
        let resp = self.device.read_write(
            self.target,
            IndexGroup::SYMBOL_HANDLE_BY_NAME,
            IndexOffset::ZERO,
            4,
            name.as_ref(),
        )?;

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
    pub fn get_multi_handles_by_name<'a, S: AsRef<str> + 'a + ?Sized>(
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

        let resp = self.device.read_write_multi(self.target, reqs)?;

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
    pub fn release_handle(&self, handle: SymbolHandle) -> crate::Result<()> {
        self.device.write(
            self.target,
            IndexGroup::SYMBOL_RELEASE_HANDLE,
            IndexOffset::ZERO,
            handle.to_bytes(),
        )
    }

    /// Releases multiple symbol handles.
    pub fn release_multi_handles(
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

        let resp = self.device.write_multi(self.target, reqs)?;

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
    pub fn read_bytes_by_handle(
        &self,
        handle: SymbolHandle,
        length: usize,
    ) -> crate::Result<Vec<u8>> {
        self.device.read(
            self.target,
            IndexGroup::SYMBOL_VALUE_BY_HANDLE,
            handle.as_u32().into(),
            length as u32,
        )
    }

    /// Reads multiple values as bytes using their handles.
    pub fn read_multi_as_bytes_by_handle(
        &self,
        items: impl AsRef<[(SymbolHandle, usize)]>,
    ) -> crate::Result<impl Iterator<Item = crate::Result<Vec<u8>>>> {
        let reqs: Vec<_> = items
            .as_ref()
            .iter()
            .map(|(handle, len)| {
                SumReadRequest::new(
                    IndexGroup::SYMBOL_VALUE_BY_HANDLE,
                    handle.as_u32().into(),
                    *len as u32,
                )
            })
            .collect();

        let resp = self.device.read_multi(self.target, &reqs)?;

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
    pub fn read_bytes_by_info(&self, info: &AdsSymbolInfo) -> crate::Result<Vec<u8>> {
        self.device.read(
            self.target,
            info.index_group(),
            info.index_offset(),
            info.size(),
        )
    }

    /// Reads raw bytes from multiple symbols directly using their absolute memory locations
    /// ([`IndexGroup`] and [`IndexOffset`]) provided by their [`AdsSymbolInfo`]s in a single
    /// network transaction.
    pub fn read_multi_as_bytes_by_info(
        &self,
        infos: impl AsRef<[AdsSymbolInfo]>,
    ) -> crate::Result<impl Iterator<Item = crate::Result<Vec<u8>>>> {
        let reqs: Vec<_> = infos
            .as_ref()
            .iter()
            .map(|info| SumReadRequest::new(info.index_group(), info.index_offset(), info.size()))
            .collect();

        let resp = self.device.read_multi(self.target, &reqs)?;

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
    pub fn read_value<T>(&self, path: impl AsRef<str>) -> crate::Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let path = path.as_ref();
        let (cache, entry) = self.resolve_symbol(path)?;

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
            .map_err(|err| self.map_stale(err, &cache, path))?;

        tcads_serde::from_bytes(&bytes, &type_info, &*cache.types()?).map_err(Into::into)
    }

    /// Reads a batch of symbol values.
    pub fn read_multi_values<S: AsRef<str> + Eq + Hash>(
        &self,
        paths: impl IntoIterator<Item = S>,
    ) -> crate::Result<ReadMultiValues> {
        ReadMultiValues::read(self, paths)
    }

    /// Subscribes to value-change notifications for a symbol by instance path.
    ///
    /// Resolves the symbol the same way [`read_value`](Self::read_value) does
    /// (cached after the first call), then subscribes on its handle via
    /// [`SYMBOL_VALUE_BY_HANDLE`](IndexGroup::SYMBOL_VALUE_BY_HANDLE). `trans_mode`, `max_delay`,
    /// and `cycle_time` are passed straight through to [`AdsNotificationAttrib`]; use
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
    /// call of its own that can fail with [`AdsErrDeviceSymbolVersionInvalid`](tcads_core::AdsReturnCode::AdsErrDeviceSymbolVersionInvalid)
    /// when a symbol version change invalidates the handle backing it: it's push,
    /// not pull. Instead, the PLC pushes a single zero-length sample on this
    /// subscription to signal exactly that. [`ValueReceiver`] treats a
    /// zero-length sample as [`Error::HandleInvalidated`](crate::Error::HandleInvalidated),
    /// flushing the symbol cache the same as a failed `read_value`/`write_value`
    /// call would. It does **not** auto-resubscribe: the online change may have
    /// altered the symbol's type or size, so silently reinterpreting whatever
    /// arrives next under a possibly-changed layout would be worse than
    /// surfacing the problem. Call `subscribe_value` again once you've decided
    /// that's safe.
    pub fn subscribe_value<T>(
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
        let (cache, entry) = self.resolve_symbol(path)?;

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

        let (rx, notif_handle) = self.device.add_notification(
            self.target,
            IndexGroup::SYMBOL_VALUE_BY_HANDLE,
            handle.as_u32().into(),
            AdsNotificationAttrib::new(size, trans_mode, max_delay, cycle_time),
        )?;

        let guard = NotificationGuard::new(notif_handle, self.target, self.device.clone());
        let rx = ValueReceiver::new(rx, guard, cache, Arc::from(path), type_info);
        Ok((rx, notif_handle))
    }

    /// Writes a value to the symbol as raw bytes using a handle.
    pub fn write_bytes_by_handle(
        &self,
        handle: SymbolHandle,
        data: impl Into<Vec<u8>>,
    ) -> crate::Result<()> {
        self.device.write(
            self.target,
            IndexGroup::SYMBOL_VALUE_BY_HANDLE,
            handle.as_u32().into(),
            data,
        )
    }

    pub fn write_multi_as_bytes_by_handle<D: AsRef<[u8]>>(
        &self,
        items: impl AsRef<[(SymbolHandle, D)]>,
    ) -> crate::Result<impl Iterator<Item = crate::Result<()>>> {
        let reqs: Vec<_> = items
            .as_ref()
            .iter()
            .map(|(handle, data)| {
                SumWriteRequest::new(
                    IndexGroup::SYMBOL_VALUE_BY_HANDLE,
                    handle.as_u32().into(),
                    data.as_ref(),
                )
            })
            .collect();

        let resp = self.device.write_multi(self.target, reqs)?;

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
    pub fn write_bytes_by_info(
        &self,
        info: &AdsSymbolInfo,
        data: impl Into<Vec<u8>>,
    ) -> crate::Result<()> {
        self.device
            .write(self.target, info.index_group(), info.index_offset(), data)
    }

    /// Writes raw bytes to multiple symbols directly using their absolute memory locations
    /// provided by their [`AdsSymbolInfo`] in a single network transaction.
    pub fn write_multi_as_bytes_by_info<S: AsRef<AdsSymbolInfo>, D: AsRef<[u8]>>(
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

        let resp = self.device.write_multi(self.target, reqs)?;

        let results = resp
            .into_iter()
            .map(|res| match res {
                Ok(()) => Ok(()),
                Err(err) => Err(crate::Error::from(err)),
            })
            .collect::<Vec<_>>();

        Ok(results.into_iter())
    }

    /// Serializes `value` and writes it to the symbol at the given `path`.
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
    pub fn write_value<T>(&self, path: impl AsRef<str>, value: &T) -> crate::Result<()>
    where
        T: serde::Serialize,
    {
        let path = path.as_ref();
        let (cache, entry) = self.resolve_symbol(path)?;

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
                .map_err(|err| self.map_stale(err, &cache, path))?
        } else {
            vec![0u8; size as usize]
        };

        tcads_serde::to_bytes(value, &mut buf, &type_info, &*cache.types()?)?;

        self.write_bytes_by_handle(handle, buf)
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
    /// size, whether it needs read-modify-write) is resolved before calling
    /// [`read_value`](Self::read_value)/[`write_value`](Self::write_value).
    ///
    /// # Note
    ///
    /// This operation costs two bulk transfers proportional to the project size (hundreds of KB
    /// to a few MB on large projects), paid up front. Call this once after
    /// connecting if you'd rather pay that eagerly than have each symbol's
    /// first access pay its own smaller lazy cost.
    pub fn preload(&self) -> crate::Result<()> {
        let cache = self.symbol_cache()?;

        cache.insert_types(self.get_all_type_infos()?.filter_map(|res| res.ok()))?;

        for info in self.get_all_symbol_infos()? {
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
    pub fn get_symbol_info(&self, name: impl AsRef<str>) -> crate::Result<AdsSymbolInfo> {
        let bytes = self.device.read_write(
            self.target,
            IndexGroup::SYMBOL_INFO_BY_NAME_EX,
            IndexOffset::ZERO,
            1_048_576,
            name.as_ref(),
        )?;

        Ok(AdsSymbolInfo::try_from(bytes.as_ref())?)
    }

    /// Fetches metadata for multiple TwinCAT symbols by their instance paths in
    /// a single network transaction.
    pub fn get_multi_symbol_infos<'a, S: AsRef<str> + 'a + ?Sized>(
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

        let resp = self.device.read_write_multi(self.target, reqs)?;

        let results = resp
            .into_iter()
            .map(|res| match res {
                Ok(chunk) => AdsSymbolInfo::try_from(chunk).map_err(crate::Error::from),
                Err(err) => Err(crate::Error::from(err)),
            })
            .collect::<Vec<_>>();

        Ok(results.into_iter())
    }

    /// Downloads the entire Symbol dictionary from the PLC.
    ///
    /// This executes a single bulk transfer. This method returns a lazy iterator that parses each
    /// [`AdsSymbolInfo`] struct from the returned network binary blob sequentially as you
    /// consume it.
    pub fn get_all_symbol_infos(
        &self,
    ) -> crate::Result<impl Iterator<Item = crate::Result<AdsSymbolInfo>>> {
        let info = self.get_upload_info()?;

        let blob_size = info.symbol_blob_size();

        let raw_blob = self.device.read(
            self.target,
            IndexGroup::SYMBOL_UPLOAD,
            IndexOffset::ZERO,
            blob_size,
        )?;

        Ok(AdsSymbolInfoIteratorOwned::new(raw_blob).map(|res| res.map_err(crate::Error::from)))
    }

    /// Fetches the Data Type definition of a specific type by name (e.g. `"BOOL"`).
    pub fn get_type_info(&self, name: impl AsRef<str>) -> crate::Result<AdsTypeInfo> {
        let bytes = self.device.read_write(
            self.target,
            IndexGroup::DATA_TYPE_INFO_BY_NAME_EX,
            IndexOffset::ZERO,
            1_048_576, // assumed max size of a single entry, router will return the actual size
            name.as_ref(),
        )?;

        Ok(AdsTypeInfo::try_from(bytes.as_ref())?)
    }

    /// Fetches multiple Data Type definitions in a single network transaction.
    pub fn get_multi_type_infos<'a, S: AsRef<str> + 'a + ?Sized>(
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
            .read_write_multi(self.target, reqs)?
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
    pub fn get_all_type_infos(
        &self,
    ) -> crate::Result<impl Iterator<Item = crate::Result<AdsTypeInfo>>> {
        // There is no data type blob size for V1, so we just use a huge number.
        // This is safe because we know the size of the upload info is 8 bytes
        // and the data type blob size is 4 bytes
        let size = self
            .get_upload_info()?
            .data_type_blob_size()
            .unwrap_or(1_048_576 * 4);

        let raw_blob = self.device.read(
            self.target,
            IndexGroup::DATA_TYPE_UPLOAD,
            IndexOffset::ZERO,
            size,
        )?;

        Ok(AdsTypeInfoIteratorOwned::new(raw_blob).map(|res| res.map_err(crate::Error::from)))
    }

    /// Explicitly cancels a subscription to any notification created by this device.
    ///
    /// The receiver (i.e. [`ValueReceiver`] or [`SymbolVersionReceiver`]) associated with this
    /// handle will return [`Err(Error::Disconnected)`](crate::Error::Disconnected) on its next call.
    pub fn unsubscribe_notification(&self, handle: NotificationHandle) -> crate::Result<()> {
        self.device.delete_notification(self.target, handle)
    }

    /// Returns the lazily initialized [`SymbolCache`], creating it on first use.
    ///
    /// Creation needs the target's platform pointer size (one `get_upload_info`
    /// round trip). Falls back to 8 bytes if the target doesn't report one (V1/V2
    /// upload info), matching 64-bit runtimes.
    fn symbol_cache(&self) -> crate::Result<Arc<SymbolCache>> {
        if let Some(cache) = self.symbols.get() {
            return Ok(cache.clone());
        }
        let ptr_size = self.get_upload_info()?.platform_ptr_size().unwrap_or(8);
        Ok(self
            .symbols
            .get_or_init(|| Arc::new(SymbolCache::new(ptr_size)))
            .clone())
    }

    /// Resolves a symbol path to a cached, handle-bearing [`SymbolEntry`],
    /// fetching symbol info, the missing part of its type closure, and a handle
    /// from the PLC on a cache miss.
    fn resolve_symbol(
        &self,
        path: &str,
    ) -> crate::Result<(Arc<SymbolCache>, Arc<RwLock<SymbolEntry>>)> {
        let cache = self.symbol_cache()?;

        if let Some(entry) = cache.get(path)? {
            let has_handle = entry
                .read()
                .map_err(|_| crate::Error::PoisonedLock)?
                .handle()
                .is_some();
            if has_handle {
                return Ok((cache, entry));
            }

            let handle = self.get_handle_by_name(path)?;
            if cache.set_handle(path, handle)? {
                return Ok((cache, entry));
            }
        }

        let info = self.get_symbol_info(path)?;
        const MAX_FETCH_LEVELS: usize = 128;
        let mut entry = None;
        for _ in 0..=MAX_FETCH_LEVELS {
            let missing = cache.missing_types(info.type_name())?;
            if missing.is_empty() {
                entry = Some(cache.resolve_entry(info.type_name(), info.size())?);
                break;
            }
            let fetched: Vec<_> = self
                .get_multi_type_infos(&missing)?
                .collect::<Result<_, _>>()?;
            cache.insert_types(fetched)?;
        }

        let entry = entry.ok_or_else(|| {
            crate::Error::Serde(tcads_serde::Error::Custom(format!(
                "type closure of '{}' did not resolve within {MAX_FETCH_LEVELS} fetch levels",
                info.type_name(),
            )))
        })?;

        let handle = self.get_handle_by_name(path)?;

        cache.insert(Arc::from(path), entry.with_handle(handle))?;

        let entry = cache
            .get(path)?
            .expect("just inserted, or raced with a concurrent insert of the same path");
        Ok((cache, entry))
    }

    /// Resolves multiple symbols in a batched manner to prevent `O(N)` network
    /// round-trips on cold starts.
    ///
    /// See [`resolve_symbol`](Self::resolve_symbol) for more details.
    fn resolve_multi_symbols<S: AsRef<str> + Eq + Hash>(
        &self,
        paths: impl AsRef<[S]>,
    ) -> crate::Result<(Arc<SymbolCache>, Vec<Arc<RwLock<SymbolEntry>>>)> {
        let paths = paths.as_ref();
        let cache = self.symbol_cache()?;

        let mut missing_info_paths = IndexSet::new();
        let mut missing_handle_paths = IndexSet::new();

        for path in paths {
            if let Some(entry) = cache.get(path.as_ref())? {
                let has_handle = entry.read()?.handle().is_some();

                if !has_handle {
                    missing_handle_paths.insert(path);
                }
            } else {
                missing_info_paths.insert(path);
                missing_handle_paths.insert(path);
            }
        }

        let mut fetched_infos = Vec::new();
        if !missing_info_paths.is_empty() {
            let infos = self.get_multi_symbol_infos(&missing_info_paths)?;

            for info_res in infos {
                fetched_infos.push(info_res?);
            }

            const MAX_FETCH_LEVELS: usize = 128;
            for _ in 0..=MAX_FETCH_LEVELS {
                let mut batch_missing_types = IndexSet::new();

                for info in &fetched_infos {
                    let missing = cache.missing_types(info.type_name())?;
                    for t in missing {
                        batch_missing_types.insert(t);
                    }
                }

                if batch_missing_types.is_empty() {
                    break;
                }

                let fetched_types: Vec<_> = self
                    .get_multi_type_infos(&batch_missing_types)?
                    .collect::<crate::Result<_>>()?;

                cache.insert_types(fetched_types)?;
            }
        }

        let mut fetched_handles = Vec::new();
        if !missing_handle_paths.is_empty() {
            let handles = self.get_multi_handles_by_name(&missing_handle_paths)?;

            for handle_res in handles {
                fetched_handles.push(handle_res?);
            }
        }

        let mut info_iter = fetched_infos.into_iter();
        let mut handle_iter = fetched_handles.into_iter();

        for path in paths {
            if missing_info_paths.contains(&path) {
                let info = info_iter.next().unwrap();
                let handle = handle_iter.next().unwrap();
                let entry = cache.resolve_entry(info.type_name(), info.size())?;
                cache.insert(Arc::from(path.as_ref()), entry.with_handle(handle))?;
            } else if missing_handle_paths.contains(&path) {
                let handle = handle_iter.next().unwrap();
                cache.set_handle(path.as_ref(), handle)?;
            }
        }

        let mut final_entries = Vec::with_capacity(paths.len());
        for path in paths {
            let entry = cache
                .get(path.as_ref())?
                .expect("Guaranteed by insertion above");
            final_entries.push(entry);
        }

        Ok((cache, final_entries))
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

    /// Blocks until the symbol version changes.
    ///
    /// Returns [`Err`] when the subscription is cancelled or the connection is lost.
    pub fn recv(&self) -> crate::Result<u8> {
        Self::decode(self.rx.recv()?)
    }

    /// Blocks until the symbol version changes or `timeout` elapses.
    ///
    /// Returns [`Err(Error::Timeout)`](crate::Error::Timeout) if the timeout expires,
    /// or [`Err(Error::Disconnected)`](crate::Error::Disconnected) if the subscription
    /// is cancelled or the connection is lost.
    pub fn recv_timeout(&self, timeout: Duration) -> crate::Result<u8> {
        Self::decode(self.rx.recv_timeout(timeout)?)
    }

    /// Returns the new symbol version if a change is immediately available, without
    /// blocking.
    ///
    /// Returns `Ok(None)` if no sample is currently available,
    /// or [`Err(Error::Disconnected)`](crate::Error::Disconnected) if the
    /// subscription is cancelled or the connection is lost.
    pub fn try_recv(&self) -> crate::Result<Option<u8>> {
        match self.rx.try_recv() {
            Ok(sample) => Ok(Some(Self::decode(sample)?)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(crate::Error::Disconnected),
        }
    }

    /// Returns an iterator that blocks on each call, yielding new symbol versions
    /// until the subscription is cancelled or the connection is lost.
    pub fn iter(&self) -> impl Iterator<Item = crate::Result<u8>> + '_ {
        std::iter::from_fn(move || match self.recv() {
            Err(crate::Error::Disconnected) => None,
            result => Some(result),
        })
    }

    /// Explicitly cancels the subscription, returning any error from the router.
    ///
    /// Dropping the receiver has the same effect but discards the result; prefer this
    /// when you want to know cancellation actually succeeded.
    pub fn unsubscribe(self) -> crate::Result<()> {
        self.guard.cancel()
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

    /// Blocks until the next value change.
    ///
    /// Returns [`Err(Error::Disconnected)`](crate::Error::Disconnected) when the subscription is
    /// cancelled or the connection is lost, or
    /// [`Error::HandleInvalidated`](crate::Error::HandleInvalidated) if an online change
    /// invalidated the handle backing this subscription (see
    /// [`subscribe_value`](RuntimeDevice::subscribe_value)'s doc comment).
    pub fn recv(&self) -> crate::Result<T> {
        self.decode(self.rx.recv()?)
    }

    /// Blocks until the next value change or `timeout` elapses.
    ///
    /// Returns [`Err(Error::Timeout)`](crate::Error::Timeout) if the timeout expires,
    /// [`Err(Error::Disconnected)`](crate::Error::Disconnected) if the subscription
    /// is cancelled or the connection is lost, or
    /// [`Error::HandleInvalidated`](crate::Error::HandleInvalidated) per [`recv`](Self::recv).
    pub fn recv_timeout(&self, timeout: Duration) -> crate::Result<T> {
        self.decode(self.rx.recv_timeout(timeout)?)
    }

    /// Returns the next value if a sample is immediately available, without blocking.
    ///
    /// Returns `Ok(None)` if no sample is currently available,
    /// [`Err(Error::Disconnected)`](crate::Error::Disconnected) if the
    /// subscription is cancelled or the connection is lost, or
    /// [`Error::HandleInvalidated`](crate::Error::HandleInvalidated) per [`recv`](Self::recv).
    pub fn try_recv(&self) -> crate::Result<Option<T>> {
        match self.rx.try_recv() {
            Ok(sample) => Ok(Some(self.decode(sample)?)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(crate::Error::Disconnected),
        }
    }

    /// Returns an iterator that blocks on each call, yielding new values until
    /// the subscription is cancelled, the connection is lost, or a handle
    /// invalidation is observed (each ending the iterator after yielding that
    /// final `Err`, per [`recv`](Self::recv)).
    pub fn iter(&self) -> impl Iterator<Item = crate::Result<T>> + '_ {
        std::iter::from_fn(move || match self.recv() {
            Err(crate::Error::Disconnected) => None,
            result => Some(result),
        })
    }

    /// Explicitly cancels the subscription, returning any error from the router.
    ///
    /// Dropping the receiver has the same effect but discards the result; prefer this
    /// when you want to know cancellation actually succeeded.
    pub fn unsubscribe(self) -> crate::Result<()> {
        self.guard.cancel()
    }
}

#[derive(Clone)]
pub struct ReadMultiValues {
    cache: Arc<SymbolCache>,
    entries: VecDeque<(Vec<u8>, Arc<AdsTypeInfo>)>,
}

impl ReadMultiValues {
    /// Resolves and reads every symbol's value in one call.
    pub fn read<S: AsRef<str> + Eq + Hash>(
        device: &RuntimeDevice,
        paths: impl IntoIterator<Item = S>,
    ) -> crate::Result<Self> {
        let paths_vec: Vec<S> = paths.into_iter().collect();

        if paths_vec.is_empty() {
            return Ok(Self {
                cache: device.symbol_cache()?,
                entries: VecDeque::new(),
            });
        }

        let (cache, entries) = device.resolve_multi_symbols(&paths_vec)?;

        let mut handle_requests = Vec::with_capacity(entries.len());
        let mut type_infos = Vec::with_capacity(entries.len());

        for entry_lock in entries {
            let guard = entry_lock.read()?;
            let handle = guard
                .handle()
                .expect("resolve_multi_symbols attaches handles");

            handle_requests.push((handle, guard.size() as usize));
            type_infos.push(guard.type_info().clone());
        }

        let results = device.read_multi_as_bytes_by_handle(&handle_requests)?;

        let mut read_entries = VecDeque::with_capacity(handle_requests.len());

        for ((result, type_info), path) in results.zip(type_infos).zip(&paths_vec) {
            match result {
                Ok(bytes) => read_entries.push_back((bytes, type_info)),
                Err(err) => {
                    return Err(device.map_stale(err, &cache, path.as_ref()));
                }
            }
        }

        Ok(Self {
            cache,
            entries: read_entries,
        })
    }

    /// The number of not-yet-popped entries remaining.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if there are no more entries remaining.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Decodes and removes the earliest not-yet-popped entry (declaration
    /// order). `None` once every entry has been popped; `Some(Err(_))` if
    /// decoding this specific entry as `T` fails (e.g. `T` doesn't match the
    /// PLC type), which doesn't affect any other entry.
    pub fn pop_front<T>(&mut self) -> Option<crate::Result<T>>
    where
        T: serde::de::DeserializeOwned,
    {
        let (bytes, type_info) = self.entries.pop_front()?;
        Some(self.decode(&bytes, &type_info))
    }

    /// Same as [`pop_front`](Self::pop_front), but from the latest end instead.
    pub fn pop_back<T>(&mut self) -> Option<crate::Result<T>>
    where
        T: serde::de::DeserializeOwned,
    {
        let (bytes, type_info) = self.entries.pop_back()?;
        Some(self.decode(&bytes, &type_info))
    }

    /// Decodes the entry at `index` (declaration order) without consuming it.
    pub fn get<T: serde::de::DeserializeOwned>(&self, index: usize) -> Option<crate::Result<T>> {
        let (bytes, type_info) = self.entries.get(index)?;
        Some(self.decode(bytes, type_info))
    }

    fn decode<T: serde::de::DeserializeOwned>(
        &self,
        bytes: &[u8],
        type_info: &AdsTypeInfo,
    ) -> crate::Result<T> {
        tcads_serde::from_bytes(bytes, type_info, &*self.cache.types()?).map_err(Into::into)
    }
}

/// Iterator adapter over a [`ReadMulti`] where every entry decodes to the same `T`.
/// Obtained via [`ReadMultiValues::into_iter_as`].
pub struct ReadMultiValuesIter<T> {
    inner: ReadMultiValues,
    _marker: PhantomData<T>,
}

impl<T: serde::de::DeserializeOwned> Iterator for ReadMultiValuesIter<T> {
    type Item = crate::Result<T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.pop_front()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.inner.len();
        (len, Some(len))
    }
}
