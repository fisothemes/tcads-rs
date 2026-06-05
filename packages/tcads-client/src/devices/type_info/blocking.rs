use super::{
    DATATYPE_INFO_BY_NAME_INDEX_GROUP, DATATYPE_UPLOAD_INDEX_GROUP, SYMBOL_UPLOAD_INFO_INDEX_GROUP,
};
use crate::devices::blocking::{AdsDevice, SumDevice};
use std::net::ToSocketAddrs;
use std::time::Duration;
use tcads_core::{
    AdsError, AdsSymbolUploadInfo, AdsSymbolUploadInfoV3, AdsTypeInfo, AdsTypeInfoIteratorOwned,
    AmsAddr, AmsPort, SumReadWriteRequest,
};

/// An ADS device client for fetching data type info from a PLC runtime.
///
/// # Ports
///
/// Any PLC runtime port that exposes the symbol/type interface:
/// - Ports 801–899: PLC runtimes (851 is usually the default first runtime for TC3)
/// - Ports 301–399: FreeTasks
#[derive(Clone)]
pub struct TypeInfoDevice {
    device: AdsDevice,
    target: AmsAddr,
}

impl TypeInfoDevice {
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
        Self { device, target }
    }

    /// Shuts down the underlying connection.
    ///
    /// See [`AdsDevice::shutdown`] for more details.
    pub fn shutdown(&self) -> crate::Result<()> {
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

    /// Fetches the TwinCAT type information for a specific type by name.
    pub fn get_type_info(&self, name: impl AsRef<str>) -> crate::Result<AdsTypeInfo> {
        let name = name.as_ref();
        let length_bytes =
            self.device
                .read_write(self.target, DATATYPE_INFO_BY_NAME_INDEX_GROUP, 0, 4, name)?;

        let entry_length = u32::from_le_bytes(
            length_bytes
                .try_into()
                .map_err(|_| crate::Error::InvalidPayload)?,
        );

        let bytes = self.device.read_write(
            self.target,
            DATATYPE_INFO_BY_NAME_INDEX_GROUP,
            0,
            entry_length,
            name,
        )?;

        Ok(AdsTypeInfo::try_from(bytes.as_ref()).map_err(AdsError::from)?)
    }

    pub fn get_multi_type_infos<S: AsRef<str>>(
        &self,
        names: &[S],
    ) -> crate::Result<impl Iterator<Item = crate::Result<AdsTypeInfo>>> {
        let sum_device = SumDevice::new(self.device.clone());
        let expected_read_len = 65_536;

        let reqs: Vec<_> = names
            .iter()
            .map(|name| {
                SumReadWriteRequest::new(
                    DATATYPE_INFO_BY_NAME_INDEX_GROUP,
                    0,
                    expected_read_len,
                    name.as_ref().as_bytes(),
                )
            })
            .collect();

        let resp = sum_device.read_write(self.target, &reqs)?;

        let iter =
            resp.iter()
                .map(|res| match res {
                    Ok(bytes) => AdsTypeInfo::try_from(bytes)
                        .map_err(|e| crate::Error::from(AdsError::from(e))),
                    Err(e) => Err(crate::Error::from(e)),
                })
                .collect::<Vec<_>>()
                .into_iter();

        Ok(iter)
    }

    /// Fetches all TwinCAT type information from the PLC and returns a lazy iterator.
    ///
    /// The network request is made immediately, but the parsing happens lazily
    /// as you consume the iterator.
    pub fn get_all_type_infos(
        &self,
    ) -> crate::Result<impl Iterator<Item = crate::Result<AdsTypeInfo>>> {
        // There is no data type blob size for V1, so we just use a huge number.
        // This is safe because we know the size of the upload info is 8 bytes
        // and the data type blob size is 4 bytes
        let size = self
            .get_upload_info()?
            .data_type_blob_size()
            .unwrap_or(1_048_576);

        let raw_blob = self
            .device
            .read(self.target, DATATYPE_UPLOAD_INDEX_GROUP, 0, size)?;

        Ok(AdsTypeInfoIteratorOwned::new(raw_blob)
            .map(|res| res.map_err(|e| crate::Error::from(AdsError::from(e)))))
    }

    /// Fetches and caches the symbol upload metadata from the PLC.
    pub fn get_upload_info(&self) -> crate::Result<AdsSymbolUploadInfo> {
        let bytes = self.device.read(
            self.target,
            SYMBOL_UPLOAD_INFO_INDEX_GROUP,
            0,
            // Using the largest version because server will return the largest version it supports.
            AdsSymbolUploadInfoV3::LENGTH as u32,
        )?;

        let info = AdsSymbolUploadInfo::try_from_slice(&bytes).map_err(AdsError::from)?;

        Ok(info)
    }
}
