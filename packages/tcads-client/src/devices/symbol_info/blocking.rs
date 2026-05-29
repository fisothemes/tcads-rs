use super::{
    SYMBOL_INFO_BY_NAME_EX_INDEX_GROUP, SYMBOL_INFO_UPLOAD_INDEX_GROUP,
    SYMBOL_UPLOAD_INFO_INDEX_GROUP,
};
use crate::devices::blocking::AdsDevice;
use std::time::Duration;
use tcads_core::{AdsError, AdsSymbolUploadInfo, AdsSymbolUploadInfoV3, AmsAddr, AmsPort};

#[derive(Clone)]
pub struct SymbolInfoDevice {
    device: AdsDevice,
    target: AmsAddr,
}

impl SymbolInfoDevice {
    pub fn connect(
        port: impl Into<AmsPort>,
        timeout: impl Into<Option<Duration>>,
    ) -> crate::Result<Self> {
        let device = AdsDevice::connect(timeout)?;
        let target = AmsAddr::new(device.get_local_net_id()?, port);
        Ok(Self::new(device, target))
    }

    pub fn connect_to(
        target: AmsAddr,
        timeout: impl Into<Option<Duration>>,
    ) -> crate::Result<Self> {
        let device = AdsDevice::connect(timeout)?;
        Ok(Self::new(device, target))
    }

    pub fn new(device: AdsDevice, target: AmsAddr) -> Self {
        Self { device, target }
    }

    pub fn get_ref(&self) -> &AdsDevice {
        &self.device
    }

    pub fn get_symbol_info(&self, name: impl AsRef<str>) -> crate::Result<Vec<u8>> {
        let name = name.as_ref();
        let entry_len_bytes =
            self.device
                .read_write(self.target, SYMBOL_INFO_BY_NAME_EX_INDEX_GROUP, 0, 4, name)?;

        let entry_length = u32::from_le_bytes(
            entry_len_bytes
                .try_into()
                .map_err(|_| crate::Error::InvalidPayload)?,
        );

        self.device.read_write(
            self.target,
            SYMBOL_INFO_BY_NAME_EX_INDEX_GROUP,
            0,
            entry_length,
            name,
        )
    }

    pub fn get_all_symbol_info(&self) -> crate::Result<Vec<u8>> {
        let info = self.get_upload_info()?;

        let blob_size = info.symbol_blob_size();

        self.device
            .read(self.target, SYMBOL_INFO_UPLOAD_INDEX_GROUP, 0, blob_size)
    }

    pub fn get_upload_info(&self) -> crate::Result<AdsSymbolUploadInfo> {
        let bytes = self.device.read(
            self.target,
            SYMBOL_UPLOAD_INFO_INDEX_GROUP,
            0,
            AdsSymbolUploadInfoV3::LENGTH as u32,
        )?;
        Ok(AdsSymbolUploadInfo::try_from(bytes.as_ref()).map_err(AdsError::from)?)
    }
}
