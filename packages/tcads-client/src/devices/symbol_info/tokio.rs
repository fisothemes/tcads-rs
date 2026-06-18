use crate::devices::tokio::AdsDevice;
use std::net::ToSocketAddrs;
use std::time::Duration;
use tcads_core::{
    AdsError, AdsSymbolInfo, AdsSymbolInfoIteratorOwned, AdsSymbolUploadInfo,
    AdsSymbolUploadInfoV3, AmsAddr, AmsPort, IndexGroup, IndexOffset, SumReadWriteRequest,
};

/// An asynchronous ADS device client for accessing TwinCAT Symbol Information.
///
/// # Ports
///
/// Any PLC runtime port that exposes the symbol information:
/// - Ports 801–899: PLC runtimes (851 is usually the default first runtime for TC3)
/// - Ports 301–399: FreeTasks
#[derive(Clone)]
pub struct SymbolInfoDevice {
    device: AdsDevice,
    target: AmsAddr,
}

impl SymbolInfoDevice {
    /// Connects to a specific `port` on the local AMS router at `127.0.0.1:48898`.
    ///
    /// Automatically discovers the local Net ID and targets `local_net_id:port`.
    /// The `port` is the [`AmsPort`] of the target PLC runtime (e.g. 851).
    ///
    /// # Note
    ///
    /// On Windows, connecting via `127.0.0.1` requires the `EnableAmsTcpLoopback`
    /// registry key to be set. This is enabled by default in TwinCAT 4024.5 and newer.
    pub async fn connect(
        port: impl Into<AmsPort>,
        timeout: impl Into<Option<Duration>>,
    ) -> crate::Result<Self> {
        let device = AdsDevice::connect(timeout).await?;
        let target = AmsAddr::new(device.get_local_net_id().await?, port.into());
        Ok(Self::new(device, target))
    }

    /// Connects to the explicit `target` address via the local AMS router.
    ///
    /// Use this when the target ADS device has a different AMS Net ID than the local router's
    /// primary Net ID (e.g. targeting a specific UmRT).
    pub async fn connect_to(
        target: AmsAddr,
        timeout: impl Into<Option<Duration>>,
    ) -> crate::Result<Self> {
        let device = AdsDevice::connect(timeout).await?;
        Ok(Self::new(device, target))
    }

    /// Connects directly to a remote AMS router on a system without a local router.
    ///
    /// The `source` address must be pre-configured as a static route on the remote router.
    /// The `target` address is the full address of the PLC runtime.
    ///
    /// See [`AdsDevice::connect_remote`] for more details.
    pub async fn connect_remote(
        addr: impl ToSocketAddrs,
        source: AmsAddr,
        target: AmsAddr,
        timeout: impl Into<Option<Duration>>,
    ) -> crate::Result<Self> {
        let device = AdsDevice::connect_remote(addr, source, timeout).await?;
        Ok(Self::new(device, target))
    }

    /// Creates a new instance from an existing [`AdsDevice`] and target address.
    ///
    /// Use this when sharing a connection with other device clients.
    pub fn new(device: AdsDevice, target: AmsAddr) -> Self {
        Self { device, target }
    }

    /// Shuts down the underlying connection.
    ///
    /// See [`AdsDevice::shutdown`] for more details.
    pub async fn shutdown(&self) {
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

    // Fetches the metadata for a specific Symbol by its instance path (e.g. `"MAIN.nCount"`).
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

    /// Fetches multiple TwinCAT symbol information by their instance paths.
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

    /// Downloads the entire Symbol dictionary from the PLC in a single transaction.
    ///
    /// This returns an iterator for lazily parsing the heap-allocated memory blob.
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

    /// Fetches the metadata describing the symbol and data type blobs on the runtime.
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
}
