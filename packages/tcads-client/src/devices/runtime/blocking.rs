use crate::devices::blocking::AdsDevice;
use std::net::ToSocketAddrs;
use std::time::Duration;
use tcads_core::{
    AdsError, AdsSymbolInfo, AdsSymbolInfoIteratorOwned, AdsSymbolUploadInfo,
    AdsSymbolUploadInfoV3, AdsTypeInfo, AdsTypeInfoIteratorOwned, AmsAddr, AmsPort, IndexGroup,
    IndexOffset, SumReadWriteRequest,
};

/// A high-level client for interacting with a specific TwinCAT runtime.
///
/// `RuntimeDevice` provides specialized methods for querying a target's memory
/// layout, including Symbol (variable) metadata and Data Type definitions.
///
/// It is bound to a single target address ([`AmsAddr`]). Common target ports include:
/// - **851** (and **801–899**): PLC runtimes (851 is the default first TC3 PLC task).
/// - **301–399**: FreeTasks.
pub struct RuntimeDevice {
    device: AdsDevice,
    target: AmsAddr,
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
        Self { device, target }
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
    pub fn get_multi_symbol_infos<S: AsRef<str>>(
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
            .read_write_multi(self.target, &reqs)?
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
    pub fn get_multi_type_infos<S: AsRef<str>>(
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
            .read_write_multi(self.target, &reqs)?
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
}
