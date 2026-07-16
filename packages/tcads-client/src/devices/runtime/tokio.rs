use crate::devices::tokio::AdsDevice;
use crate::notif_guard::tokio::NotificationGuard;
use std::net::ToSocketAddrs;
use std::time::Duration;
use tcads_core::{
    AdsError, AdsNotificationAttrib, AdsNotificationSampleOwned, AdsSymbolInfo,
    AdsSymbolInfoIteratorOwned, AdsSymbolUploadInfo, AdsSymbolUploadInfoV3, AdsTransMode,
    AdsTypeInfo, AdsTypeInfoIteratorOwned, AmsAddr, AmsPort, IndexGroup, IndexOffset,
    NotificationHandle, SumReadRequest, SumReadWriteRequest, SumWriteRequest, SymbolHandle,
};
use tokio::sync::mpsc::UnboundedReceiver as Receiver;
use tokio::sync::mpsc::error::TryRecvError;

pub struct RuntimeDevice {
    device: AdsDevice,
    target: AmsAddr,
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
        Self { device, target }
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
    pub async fn subscribe_symbol_version(&self) -> crate::Result<SymbolVersionReceiver> {
        let (rx, handle) = self
            .device
            .add_notification(
                self.target,
                IndexGroup::SYMBOL_VERSION,
                IndexOffset::ZERO,
                AdsNotificationAttrib::new(1, AdsTransMode::ServerOnChange, 0, 0),
            )
            .await?;

        let guard = NotificationGuard::new(handle, self.target, self.device.clone());
        Ok(SymbolVersionReceiver::new(rx, guard))
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
    pub async fn get_multi_handles_by_name<S: AsRef<str>>(
        &self,
        names: impl AsRef<[S]>,
    ) -> crate::Result<impl Iterator<Item = crate::Result<SymbolHandle>>> {
        let reqs: Vec<_> = names
            .as_ref()
            .iter()
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

    pub async fn read_multi_as_bytes_by_handle(
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
    pub async fn get_multi_symbol_infos<S: AsRef<str>>(
        &self,
        names: impl AsRef<[S]>,
    ) -> crate::Result<impl Iterator<Item = crate::Result<AdsSymbolInfo>>> {
        let reqs: Vec<_> = names
            .as_ref()
            .iter()
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
    pub async fn get_multi_type_infos<S: AsRef<str>>(
        &self,
        names: impl AsRef<[S]>,
    ) -> crate::Result<impl Iterator<Item = crate::Result<AdsTypeInfo>>> {
        let reqs: Vec<_> = names
            .as_ref()
            .iter()
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
