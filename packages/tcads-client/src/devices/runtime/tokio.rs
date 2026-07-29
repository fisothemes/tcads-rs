use super::symbol_cache::{SymbolCache, SymbolEntry};
use super::{multi, rpc};
use crate::devices::tokio::{AdsDevice, AdsSubsystem};
use crate::notif_guard::tokio::NotificationGuard;
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::net::ToSocketAddrs;
use std::rc::Rc;
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
/// [`AdsRuntime`] provides specialized methods for querying a target runtime device's memory
/// layout, including Symbol (variable) metadata, values, and Data Type definitions.
///
/// It is bound to a single target address ([`AmsAddr`]). Common target ports include:
/// - **851** (and **801–899**): PLC runtimes (851 is the default first TC3 PLC task).
/// - **301–399**: FreeTasks.
#[derive(Clone)]
pub struct AdsRuntime {
    device: AdsDevice,
    target: AmsAddr,
    symbols: OnceCell<Arc<SymbolCache>>,
}

impl AdsRuntime {
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
    /// [`AdsRuntime::connect`] for those.
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

    /// Creates an instance of the [`AdsRuntime`] by wrapping an existing [`AdsDevice`] and
    /// target address.
    ///
    /// Useful if you are sharing a connection with other ADS devices
    /// i.e. the [`Logger`](crate::devices::blocking::AdsLogger) ADS device.
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
    pub async fn get_multi_handles_by_name<'a, I, S>(
        &self,
        names: I,
    ) -> crate::Result<impl Iterator<Item = crate::Result<SymbolHandle>>>
    where
        I: IntoIterator<Item = &'a S>,
        I::IntoIter: ExactSizeIterator,
        S: AsRef<str> + 'a + ?Sized,
    {
        let reqs = names.into_iter().map(|name| {
            SumReadWriteRequest::new(
                IndexGroup::SYMBOL_HANDLE_BY_NAME,
                IndexOffset::ZERO,
                4,
                name.as_ref().as_bytes(),
            )
        });

        let resp = self.device.read_write_multi(self.target, reqs).await?;

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
    pub async fn release_multi_handles<'a, I>(
        &self,
        handles: I,
    ) -> crate::Result<impl Iterator<Item = crate::Result<()>>>
    where
        I: IntoIterator<Item = &'a SymbolHandle>,
        I::IntoIter: ExactSizeIterator,
    {
        let reqs = handles.into_iter().map(|handle| {
            SumWriteRequest::new(
                IndexGroup::SYMBOL_RELEASE_HANDLE,
                IndexOffset::ZERO,
                handle.as_bytes(),
            )
        });

        let resp = self.device.write_multi(self.target, reqs).await?;

        let results = resp
            .into_iter()
            .map(|res| res.map_err(crate::Error::from))
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
    pub async fn read_multi_as_bytes_by_handle<I>(
        &self,
        items: I,
    ) -> crate::Result<impl Iterator<Item = crate::Result<Vec<u8>>>>
    where
        I: IntoIterator<Item = (SymbolHandle, usize)>,
        I::IntoIter: ExactSizeIterator,
    {
        let reqs = items.into_iter().map(|(handle, len)| {
            SumReadRequest::new(
                IndexGroup::SYMBOL_VALUE_BY_HANDLE,
                handle.as_u32().into(),
                len as u32,
            )
        });

        let resp = self.device.read_multi(self.target, reqs).await?;

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
    pub async fn read_multi_as_bytes_by_info<'a, I>(
        &self,
        infos: I,
    ) -> crate::Result<impl Iterator<Item = crate::Result<Vec<u8>>>>
    where
        I: IntoIterator<Item = &'a AdsSymbolInfo>,
        I::IntoIter: ExactSizeIterator,
    {
        let reqs = infos
            .into_iter()
            .map(|info| SumReadRequest::new(info.index_group(), info.index_offset(), info.size()));

        let resp = self.device.read_multi(self.target, reqs).await?;

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

    /// Reads a batch of symbol values.
    ///
    /// See [`read_value`](Self::read_value) for more details.
    pub async fn read_multi_values<S: AsRef<str>>(
        &self,
        paths: impl IntoIterator<Item = S>,
    ) -> crate::Result<ReadMultiValues> {
        ReadMultiValues::read(self, paths).await
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

    pub async fn write_multi_as_bytes_by_handle<'a, I>(
        &self,
        items: I,
    ) -> crate::Result<impl Iterator<Item = crate::Result<()>>>
    where
        I: IntoIterator<Item = (SymbolHandle, &'a [u8])>,
        I::IntoIter: ExactSizeIterator,
    {
        let reqs = items.into_iter().map(|(handle, data)| {
            SumWriteRequest::new(
                IndexGroup::SYMBOL_VALUE_BY_HANDLE,
                handle.as_u32().into(),
                data,
            )
        });

        let resp = self.device.write_multi(self.target, reqs).await?;

        let results = resp
            .into_iter()
            .map(|res| res.map_err(crate::Error::from))
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
    pub async fn write_multi_as_bytes_by_info<'a, I, S, D>(
        &self,
        items: I,
    ) -> crate::Result<impl Iterator<Item = crate::Result<()>>>
    where
        I: IntoIterator<Item = &'a (S, D)>,
        I::IntoIter: ExactSizeIterator,
        S: AsRef<AdsSymbolInfo> + 'a,
        D: AsRef<[u8]> + 'a,
    {
        let reqs = items.into_iter().map(|(info, data)| {
            SumWriteRequest::new(
                info.as_ref().index_group(),
                info.as_ref().index_offset(),
                data.as_ref(),
            )
        });

        let resp = self.device.write_multi(self.target, reqs).await?;

        let results = resp
            .into_iter()
            .map(|res| res.map_err(crate::Error::from))
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
        T: serde::Serialize + ?Sized,
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

    /// Creates a new instance of [`WriteMultiValues`] for executing a
    /// heterogeneous batch write operation.
    ///
    /// See [`write_value`](Self::write_value) for more details.
    pub fn write_multi_values(&self) -> WriteMultiValues<'_> {
        WriteMultiValues::new(self)
    }

    /// Calls an RPC method on a Function Block or Interface instance.
    ///
    /// `fb_path` is the instance's path (e.g. `"MAIN.fbSomeInterface"`), not
    /// the method itself; `method_name` is looked up case-insensitively
    /// against that instance's type. Requires the method to have
    /// `{attribute 'TcRpcEnable'}` in its PLC declaration.
    ///
    /// `I`/`O` are plain tuples (or a bare value when only one element is
    /// relevant on that side):
    /// - `I`: one element per `IN` or `IN_OUT` parameter, in declared order.
    /// - `O`: the return value first (if the method has one), then one
    ///   element per `OUT` or `IN_OUT` parameter, in declared order.
    ///
    /// An `IN_OUT` parameter appears on *both* sides: you send a value in
    /// `I`, and get the PLC's (possibly different) value for it back out
    /// through `O`, at its own declared position among the parameters
    /// relevant to each side, not necessarily the same position on both.
    ///
    /// A method with no relevant parameters on a side uses `()` for that
    /// side, e.g. a method with no return and no `OUT`/`IN_OUT` parameters
    /// has `O = ()`.
    pub async fn rpc<I, O>(
        &self,
        fb_path: impl AsRef<str>,
        method_name: impl AsRef<str>,
        inputs: &I,
    ) -> crate::Result<O>
    where
        I: serde::Serialize,
        O: serde::de::DeserializeOwned,
    {
        let fb_path = fb_path.as_ref();
        let method_name = method_name.as_ref();

        let (cache, entry) = self.resolve_symbol(fb_path).await?;
        let type_info = { entry.read()?.type_info().clone() };

        let method = rpc::find_method(&type_info, method_name)?;

        let mut frontier = rpc::missing_method_types(&cache, method)?;
        for _ in 0..=multi::MAX_FETCH_LEVELS {
            if frontier.is_empty() {
                break;
            }
            let fetched: Vec<_> = self
                .get_multi_type_infos(&frontier)
                .await?
                .collect::<crate::Result<_>>()?;
            cache.insert_types(fetched)?;

            let mut next = rpc::missing_nested_types(&cache, frontier.iter().map(String::as_str))?;
            next.extend(rpc::missing_rpc_indirections(
                &cache,
                frontier.iter().map(String::as_str),
            )?);
            frontier = next;
        }

        let types_guard = cache.types()?;
        let in_fields = rpc::input_fields(method, &*types_guard)?;
        let out_fields = rpc::output_fields(method, &*types_guard)?;

        let input_bytes: Vec<u8> = if in_fields.len() == 1 {
            let field = &in_fields[0];
            let mut buf = vec![0u8; field.size() as usize];
            tcads_serde::to_bytes(inputs, &mut buf, field.type_info(), &*types_guard)?;
            buf
        } else {
            let total: u32 = in_fields.iter().map(|f| f.size()).sum();
            let mut buf = vec![0u8; total as usize];
            tcads_serde::to_rpc_fields(inputs, &mut buf, Rc::from(in_fields), &*types_guard)?;
            buf
        };

        let output_size: u32 = out_fields.iter().map(|f| f.size()).sum();

        let cached_handle = { entry.read()?.method_handle(method.name()) };
        let handle = match cached_handle {
            Some(handle) => handle,
            None => {
                let handle = self
                    .get_handle_by_name(format!("{fb_path}#{}", method.name()))
                    .await?;
                entry
                    .write()
                    .map_err(|_| crate::Error::PoisonedLock)?
                    .set_method_handle(method.name(), handle);
                handle
            }
        };

        let response = self
            .device
            .read_write(
                self.target,
                IndexGroup::SYMBOL_VALUE_BY_HANDLE,
                handle.as_u32().into(),
                output_size,
                input_bytes.as_slice(),
            )
            .await
            .map_err(|err| self.map_stale(err, &cache, fb_path))?;

        if out_fields.len() == 1 {
            let field = &out_fields[0];
            Ok(tcads_serde::from_bytes(
                &response,
                field.type_info(),
                &*types_guard,
            )?)
        } else {
            Ok(tcads_serde::from_rpc_fields(
                &response,
                Rc::from(out_fields),
                &*types_guard,
            )?)
        }
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
                .collect::<crate::Result<Vec<_>>>()?,
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
    pub async fn get_multi_symbol_infos<'a, I, S>(
        &self,
        names: I,
    ) -> crate::Result<impl Iterator<Item = crate::Result<AdsSymbolInfo>>>
    where
        I: IntoIterator<Item = &'a S>,
        I::IntoIter: ExactSizeIterator,
        S: AsRef<str> + 'a + ?Sized,
    {
        let reqs = names.into_iter().map(|name| {
            SumReadWriteRequest::new(
                IndexGroup::SYMBOL_INFO_BY_NAME_EX,
                IndexOffset::ZERO,
                1_048_576, // assumed max size of a single entry, router will return the actual size
                name.as_ref().as_bytes(),
            )
        });

        let resp = self.device.read_write_multi(self.target, reqs).await?;

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
    pub async fn get_multi_type_infos<'a, I, S>(
        &self,
        names: I,
    ) -> crate::Result<impl Iterator<Item = crate::Result<AdsTypeInfo>>>
    where
        I: IntoIterator<Item = &'a S>,
        I::IntoIter: ExactSizeIterator,
        S: AsRef<str> + 'a + ?Sized,
    {
        let reqs = names.into_iter().map(|name| {
            SumReadWriteRequest::new(
                IndexGroup::DATA_TYPE_INFO_BY_NAME_EX,
                IndexOffset::ZERO,
                1_048_576, // assumed max size of a single entry, router will return the actual size
                name.as_ref().as_bytes(),
            )
        });

        let resp = self.device.read_write_multi(self.target, reqs).await?;

        let results = resp
            .into_iter()
            .map(|res| match res {
                Ok(chunk) => AdsTypeInfo::try_from(chunk).map_err(crate::Error::from),
                Err(err) => Err(crate::Error::from(err)),
            })
            .collect::<Vec<_>>();

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

        let mut entry = None;
        for _ in 0..=multi::MAX_FETCH_LEVELS {
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
                "type closure of '{}' did not resolve within {} fetch levels",
                info.type_name(),
                multi::MAX_FETCH_LEVELS
            )))
        })?;

        let handle = self.get_handle_by_name(path).await?;

        cache.insert(Arc::from(path), entry.with_handle(handle))?;

        let entry = cache
            .get(path)?
            .expect("just inserted, or raced with a concurrent insert of the same path");
        Ok((cache, entry))
    }

    /// Resolves multiple symbols in a batched manner to prevent O(N) network
    /// round-trips on cold starts.
    ///
    /// See [`resolve_symbol`](Self::resolve_symbol) for more details.
    async fn resolve_multi_symbols<S: AsRef<str>>(
        &self,
        paths: impl AsRef<[S]>,
    ) -> crate::Result<(Arc<SymbolCache>, Vec<Arc<RwLock<SymbolEntry>>>)> {
        let paths = paths.as_ref();
        let cache = self.symbol_cache().await?;

        let (missing_info_paths, missing_handle_paths) = multi::partition_missing(&cache, paths)?;

        let mut fetched_infos = Vec::new();
        if !missing_info_paths.is_empty() {
            for info_res in self.get_multi_symbol_infos(&missing_info_paths).await? {
                fetched_infos.push(info_res?);
            }

            for _ in 0..=multi::MAX_FETCH_LEVELS {
                let missing = multi::batch_missing_types(&cache, &fetched_infos)?;
                if missing.is_empty() {
                    break;
                }
                let fetched_types: Vec<_> = self
                    .get_multi_type_infos(&missing)
                    .await?
                    .collect::<crate::Result<_>>()?;
                cache.insert_types(fetched_types)?;
            }
        }

        let mut fetched_handles = Vec::new();
        if !missing_handle_paths.is_empty() {
            for handle_res in self
                .get_multi_handles_by_name(&missing_handle_paths)
                .await?
            {
                fetched_handles.push(handle_res?);
            }
        }

        multi::apply_resolved(
            &cache,
            &missing_info_paths,
            &missing_handle_paths,
            fetched_infos,
            fetched_handles,
        )?;

        let entries = multi::collect_entries(&cache, paths)?;
        Ok((cache, entries))
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
/// Obtain one by calling [`AdsRuntime::subscribe_symbol_version`].
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
/// Obtain one by calling [`AdsRuntime::subscribe_value`].
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
    /// [`subscribe_value`](AdsRuntime::subscribe_value)'s doc comment).
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

/// The result of a batch read obtained by calling [`AdsRuntime::read_multi_values`]
/// or [`ReadMultiValues::read`].
#[derive(Clone)]
pub struct ReadMultiValues {
    cache: Arc<SymbolCache>,
    entries: VecDeque<(Vec<u8>, Arc<AdsTypeInfo>)>,
}

impl ReadMultiValues {
    /// Resolves and reads every symbol's value in one call.
    pub async fn read<S: AsRef<str>>(
        device: &AdsRuntime,
        paths: impl IntoIterator<Item = S>,
    ) -> crate::Result<Self> {
        let paths_vec: Vec<S> = paths.into_iter().collect();

        if paths_vec.is_empty() {
            return Ok(Self {
                cache: device.symbol_cache().await?,
                entries: VecDeque::new(),
            });
        }

        let (cache, entries) = device.resolve_multi_symbols(&paths_vec).await?;

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

        let results = device
            .read_multi_as_bytes_by_handle(handle_requests.iter().copied())
            .await?;

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

    /// Adapts this into a plain [`Iterator`] for the common case where every
    /// entry is the same type `T`. Backed by repeated [`pop_front`](Self::pop_front).
    pub fn into_iter_as<T: serde::de::DeserializeOwned>(
        self,
    ) -> impl Iterator<Item = crate::Result<T>> {
        ReadMultiValuesIter::new(self)
    }

    fn decode<T: serde::de::DeserializeOwned>(
        &self,
        bytes: &[u8],
        type_info: &AdsTypeInfo,
    ) -> crate::Result<T> {
        multi::decode(&self.cache, bytes, type_info)
    }
}

/// Iterator adapter over a [`ReadMultiValues`] where every entry decodes to the same `T`.
/// Obtained via [`ReadMultiValues::into_iter_as`].
pub struct ReadMultiValuesIter<T> {
    inner: ReadMultiValues,
    _marker: PhantomData<T>,
}

impl<T> ReadMultiValuesIter<T> {
    /// Creates a new instance of [`ReadMultiValuesIter`].
    pub fn new(r: ReadMultiValues) -> Self {
        Self {
            inner: r,
            _marker: PhantomData,
        }
    }
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

/// A builder for executing a heterogeneous batch write operation.
pub struct WriteMultiValues<'a> {
    device: &'a AdsRuntime,
    paths: Vec<String>,
    serializers: Vec<multi::SerializerFn<'a>>,
}

impl<'a> WriteMultiValues<'a> {
    /// Creates a new instance of [`WriteMultiValues`].
    pub fn new(device: &'a AdsRuntime) -> Self {
        Self {
            device,
            paths: Vec::new(),
            serializers: Vec::new(),
        }
    }

    /// Adds a symbol path and its value to the batch.
    pub fn push<T: serde::Serialize + ?Sized>(
        mut self,
        path: impl Into<String>,
        value: &'a T,
    ) -> Self {
        self.paths.push(path.into());
        self.serializers.push(multi::make_serializer(value));
        self
    }

    /// Adds many symbol paths and values of the same type in one call.
    /// Equivalent to calling [`push`](Self::push) in a loop.
    pub fn push_all<S, T>(self, items: impl IntoIterator<Item = (S, &'a T)>) -> Self
    where
        S: Into<String>,
        T: serde::Serialize + ?Sized + 'a,
    {
        items
            .into_iter()
            .fold(self, |acc, (path, value)| acc.push(path, value))
    }

    /// Executes the batch write operation.
    ///
    /// Returns `Ok(())` only if all variables were successfully read (for RMW),
    /// serialized, and written.
    pub async fn execute(self) -> crate::Result<()> {
        if self.paths.is_empty() {
            return Ok(());
        }

        let (cache, entries) = self.device.resolve_multi_symbols(&self.paths).await?;
        let (resolved, mut failures) = multi::gather_resolved(&entries);
        let (mut buf, slots) = multi::plan_write_buffer(&resolved, &failures);

        let (rmw_requests, rmw_indices) = multi::collect_rmw_requests(&resolved, &failures);
        if !rmw_requests.is_empty() {
            let rmw_results = self
                .device
                .read_multi_as_bytes_by_handle(rmw_requests.iter().copied())
                .await?;
            for (result, index) in rmw_results.zip(rmw_indices) {
                match result {
                    Ok(bytes) => multi::apply_rmw_bytes(&mut buf, &slots, index, &bytes),
                    Err(e) => {
                        failures[index] =
                            Some(Err(self.device.map_stale(e, &cache, &self.paths[index])));
                    }
                }
            }
        }

        multi::serialize_all(
            self.serializers,
            &resolved,
            &slots,
            &cache,
            &mut buf,
            &mut failures,
        );

        let write_items = multi::collect_write_items(&resolved, &failures, &slots, &buf);
        if !write_items.is_empty() {
            let write_results = self
                .device
                .write_multi_as_bytes_by_handle(write_items.iter().map(|(h, b, _)| (*h, *b)))
                .await?;
            for (result, (_, _, idx)) in write_results.zip(&write_items) {
                if let Err(e) = result {
                    failures[*idx] = Some(Err(self.device.map_stale(e, &cache, &self.paths[*idx])));
                }
            }
        }

        multi::first_failure(failures)
    }
}

impl AdsSubsystem for AdsRuntime {
    fn device(&self) -> &AdsDevice {
        &self.device
    }

    fn target(&self) -> AmsAddr {
        self.target
    }
}
