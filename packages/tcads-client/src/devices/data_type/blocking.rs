use super::{
    DATATYPE_INFO_BY_NAME_INDEX_GROUP, DATATYPE_UPLOAD_INDEX_GROUP, SYMBOL_UPLOAD_INFO_INDEX_GROUP,
};
use crate::devices::blocking::AdsDevice;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use std::time::Duration;
use tcads_core::ads::error::AdsTypeInfoError;
use tcads_core::{
    AdsError, AdsSymbolUploadInfo, AdsSymbolUploadInfoV3, AdsTypeInfo, AdsTypeInfoIteratorOwned,
    AmsAddr, AmsPort,
};

pub struct DataTypeDeviceInner {
    pub device: AdsDevice,
    pub target: AmsAddr,
}

/// An ADS device client for fetching data type info from a PLC runtime.
///
/// # Ports
///
/// Any PLC runtime port that exposes the symbol/type interface:
/// - Ports 801–899: PLC runtimes (851 is usually the default first runtime for TC3)
/// - Ports 301–399: FreeTasks
#[derive(Clone)]
pub struct DataTypeDevice {
    inner: Arc<DataTypeDeviceInner>,
}

impl DataTypeDevice {
    /// Connects to a specific `port` on the local AMS router at `127.0.0.1:48898`.
    ///
    /// Automatically discovers the local Net ID and targets `local_net_id:port`.
    /// The `port` is the [`AmsPort`] of the target PLC runtime (e.g. 851).
    ///
    /// # Note
    ///
    /// On Windows, connecting via `127.0.0.1` requires the `EnableAmsTcpLoopback`
    /// registry key to be set. This is enabled by default in TwinCAT 4024.5 and newer.
    pub fn connect(
        port: impl Into<AmsPort>,
        timeout: impl Into<Option<Duration>>,
    ) -> crate::Result<Self> {
        let device = AdsDevice::connect(timeout)?;
        let target = AmsAddr::new(device.get_local_net_id()?, port);
        Ok(Self::new(device, target))
    }

    /// Connects to the explicit `target` address via the local AMS router.
    ///
    /// Use this when the target ADS device has a different AMS Net ID than the
    /// local router's primary Net ID (e.g. targeting a specific UmRT).
    pub fn connect_to(
        target: AmsAddr,
        timeout: impl Into<Option<Duration>>,
    ) -> crate::Result<Self> {
        let device = AdsDevice::connect(timeout)?;
        Ok(Self::new(device, target))
    }

    /// Connects directly to a remote AMS router on a system without a local router.
    ///
    /// The `source` address must be pre-configured as a static route on the
    /// remote router. The `target` address is the full address of the PLC runtime.
    ///
    /// See [`AdsDevice::connect_remote`] for more details.
    pub fn connect_remote(
        addr: impl ToSocketAddrs,
        source: AmsAddr,
        target: AmsAddr,
        timeout: impl Into<Option<Duration>>,
    ) -> crate::Result<Self> {
        let device = AdsDevice::connect_remote(addr, source, timeout)?;
        Ok(Self::new(device, target))
    }

    /// Creates a `DataTypeDevice` from an existing [`AdsDevice`] and target address.
    ///
    /// Use this when sharing a connection with other device clients.
    pub fn new(device: AdsDevice, target: AmsAddr) -> Self {
        Self {
            inner: Arc::new(DataTypeDeviceInner { device, target }),
        }
    }

    /// Shuts down the underlying connection.
    ///
    /// See [`AdsDevice::shutdown`] for more details.
    pub fn shutdown(&self) -> crate::Result<()> {
        self.inner.device.shutdown()
    }

    /// Returns the target AMS Address.
    pub fn target(&self) -> AmsAddr {
        self.inner.target
    }

    /// Returns a reference to the underlying [`AdsDevice`].
    pub fn get_ref(&self) -> &AdsDevice {
        &self.inner.device
    }

    /// Fetches the data type information for a specific symbol name.
    pub fn get_data_type_info(&self, name: impl AsRef<str>) -> crate::Result<AdsTypeInfo> {
        let name = name.as_ref();
        let length_bytes = self.inner.device.read_write(
            self.inner.target,
            DATATYPE_INFO_BY_NAME_INDEX_GROUP,
            0,
            4,
            name,
        )?;

        let entry_length = u32::from_le_bytes(
            length_bytes
                .try_into()
                .map_err(|_| crate::Error::InvalidPayload)?,
        );

        let bytes = self.inner.device.read_write(
            self.inner.target,
            DATATYPE_INFO_BY_NAME_INDEX_GROUP,
            0,
            entry_length,
            name,
        )?;

        Ok(AdsTypeInfo::try_from(bytes.as_ref()).map_err(AdsError::from)?)
    }

    /// Fetches all data types from the PLC and returns a lazy iterator.
    ///
    /// The network request is made immediately, but the parsing happens lazily
    /// as you consume the iterator.
    pub fn get_all_data_type_info(
        &self,
    ) -> crate::Result<impl Iterator<Item = Result<AdsTypeInfo, AdsTypeInfoError>>> {
        // There is no data type blob size for V1, so we just use a huge number.
        // This is safe because we know the size of the upload info is 8 bytes
        // and the data type blob size is 4 bytes
        let size = self
            .get_upload_info()?
            .data_type_blob_size()
            .unwrap_or(1_048_576);

        let raw_blob =
            self.inner
                .device
                .read(self.inner.target, DATATYPE_UPLOAD_INDEX_GROUP, 0, size)?;

        Ok(AdsTypeInfoIteratorOwned::new(raw_blob))
    }

    /// Fetches and caches the symbol upload metadata from the PLC.
    pub fn get_upload_info(&self) -> crate::Result<AdsSymbolUploadInfo> {
        let bytes = self.inner.device.read(
            self.inner.target,
            SYMBOL_UPLOAD_INFO_INDEX_GROUP,
            0,
            // Using the largest version because server will return the largest version it supports.
            AdsSymbolUploadInfoV3::LENGTH as u32,
        )?;

        let info = AdsSymbolUploadInfo::try_from_slice(&bytes).map_err(AdsError::from)?;

        Ok(info)
    }
}
