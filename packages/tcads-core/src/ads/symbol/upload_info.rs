use super::AdsSymbolUploadFlags;
use super::error::AdsSymbolUploadInfoError;

/// Metadata describing the symbols and data types available on a PLC runtime.
///
/// Contains counts and total byte sizes for the symbol and data type blobs,
/// which must be fetched before downloading them to know how much to allocate.
///
/// Three versions exist depending on the TwinCAT runtime version. The version
/// is determined by the number of bytes returned by the server:
///
/// - **Version 1** (8 bytes): symbol count and blob size only.
/// - **Version 2** (24 bytes): adds data type count/size and dynamic symbol fields (`AdsSymbolUploadInfo2`).
/// - **Version 3** (64 bytes): adds invalid dynamic symbols, encoding code page, flags, and reserved fields.
///
/// # Wire Format
///
/// | Offset | Size | Field                      | Version | Description                                    |
/// |--------|------|----------------------------|---------|------------------------------------------------|
/// | 0      | 4    | `symbol_count`             | 1+      | Number of symbols                              |
/// | 4      | 4    | `symbol_blob_size`         | 1+      | Total byte size of the symbol blob             |
/// | 8      | 4    | `data_type_count`          | 2+      | Number of data types                           |
/// | 12     | 4    | `data_type_blob_size`      | 2+      | Total byte size of the data type blob          |
/// | 16     | 4    | `dyn_symbol_capacity`      | 2+      | Maximum number of dynamic symbols              |
/// | 20     | 4    | `dyn_symbol_count`         | 2+      | Dynamic symbols currently in use               |
/// | 24     | 4    | `invalid_dyn_symbol_count` | 3+      | Invalid dynamic symbols                        |
/// | 28     | 4    | `encoding_code_page`       | 3+      | String encoding (e.g. `1252` = Windows-1252)   |
/// | 32     | 4    | `flags`                    | 3+      | [`AdsSymbolUploadFlags`]                       |
/// | 36     | 28   | reserved                   | 3+      | Reserved for future use (7 x u32)              |
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsSymbolUploadInfo {
    symbol_count: u32,
    symbol_blob_size: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    data_type_count: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    data_type_blob_size: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    dyn_symbol_capacity: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    dyn_symbol_count: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    invalid_dyn_symbol_count: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    encoding_code_page: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    flags: Option<AdsSymbolUploadFlags>,
}

impl AdsSymbolUploadInfo {
    /// Wire size of a version 1 response.
    pub const LENGTH_V1: usize = 8;
    /// Wire size of a version 2 response.
    pub const LENGTH_V2: usize = 24;
    /// Default wire size in bytes (version 3).
    pub const LENGTH: usize = 64;

    /// Creates a new instance of [`AdsSymbolUploadInfo`].
    pub fn new(symbol_count: u32, symbol_blob_size: u32) -> Self {
        Self {
            symbol_count,
            symbol_blob_size,
            data_type_count: None,
            data_type_blob_size: None,
            dyn_symbol_capacity: None,
            dyn_symbol_count: None,
            invalid_dyn_symbol_count: None,
            encoding_code_page: None,
            flags: None,
        }
    }

    /// Creates a new instance of [`AdsSymbolUploadInfo`] with the given data type count and blob size.
    pub fn with_data_type_count(mut self, count: u32) -> Self {
        self.data_type_count = Some(count);
        self
    }

    pub fn with_data_type_blob_size(mut self, size: u32) -> Self {
        self.data_type_blob_size = Some(size);
        self
    }

    pub fn with_dyn_symbol_capacity(mut self, capacity: u32) -> Self {
        self.dyn_symbol_capacity = Some(capacity);
        self
    }

    pub fn with_dyn_symbol_count(mut self, count: u32) -> Self {
        self.dyn_symbol_count = Some(count);
        self
    }

    /// Creates a new instance of [`AdsSymbolUploadInfo`] with the given invalid dynamic symbol count.
    pub fn with_invalid_dyn_symbol_count(mut self, count: u32) -> Self {
        self.invalid_dyn_symbol_count = Some(count);
        self
    }

    /// Creates a new instance of [`AdsSymbolUploadInfo`] with the given encoding code page.
    pub fn with_encoding_code_page(mut self, code_page: u32) -> Self {
        self.encoding_code_page = Some(code_page);
        self
    }

    /// Creates a new instance of [`AdsSymbolUploadInfo`] with the given flags.
    pub fn with_flags(mut self, flags: AdsSymbolUploadFlags) -> Self {
        self.flags = Some(flags);
        self
    }

    /// Number of symbols in the symbol blob.
    pub fn symbol_count(&self) -> u32 {
        self.symbol_count
    }

    /// Total byte size of the symbol blob (`ADSIGRP_SYM_UPLOAD`, `0xF00B`).
    ///
    /// Use this as the read length when fetching the full symbol blob.
    pub fn symbol_blob_size(&self) -> u32 {
        self.symbol_blob_size
    }

    /// Number of data types in the data type blob.
    pub fn data_type_count(&self) -> Option<u32> {
        self.data_type_count
    }

    /// Total byte size of the data type blob (`ADSIGRP_SYM_DT_UPLOAD`, `0xF00E`).
    ///
    /// Use this as the read length when fetching the full data type blob.
    pub fn data_type_blob_size(&self) -> Option<u32> {
        self.data_type_blob_size
    }

    /// Maximum number of dynamic symbols supported by this runtime.
    pub fn dyn_symbol_capacity(&self) -> Option<u32> {
        self.dyn_symbol_capacity
    }

    /// Number of dynamic symbols currently in use.
    pub fn dyn_symbol_count(&self) -> Option<u32> {
        self.dyn_symbol_count
    }

    /// Number of invalid dynamic symbols. Only present in version 3 responses.
    pub fn invalid_dyn_symbol_count(&self) -> Option<u32> {
        self.invalid_dyn_symbol_count
    }

    /// String encoding code page for symbol and type names. Only present in version 3 responses.
    ///
    /// `1252` = Windows-1252 (Western European), the default for most TwinCAT runtimes.
    pub fn encoding_code_page(&self) -> Option<u32> {
        self.encoding_code_page
    }

    /// Runtime flags. Only present in version 3 responses.
    pub fn flags(&self) -> Option<AdsSymbolUploadFlags> {
        self.flags
    }

    /// Serialize to bytes (always writes full Version 3 layout)
    pub fn to_bytes(&self) -> ([u8; Self::LENGTH], usize) {
        let mut buf = [0u8; Self::LENGTH];

        buf[0..4].copy_from_slice(&self.symbol_count.to_le_bytes());
        buf[4..8].copy_from_slice(&self.symbol_blob_size.to_le_bytes());

        let mut size = Self::LENGTH_V1;

        if let Some(v) = self.data_type_count {
            buf[8..12].copy_from_slice(&v.to_le_bytes());
            size = Self::LENGTH_V2;
        }
        if let Some(v) = self.data_type_blob_size {
            buf[12..16].copy_from_slice(&v.to_le_bytes());
            size = Self::LENGTH_V2;
        }
        if let Some(v) = self.dyn_symbol_capacity {
            buf[16..20].copy_from_slice(&v.to_le_bytes());
            size = Self::LENGTH_V2;
        }
        if let Some(v) = self.dyn_symbol_count {
            buf[20..24].copy_from_slice(&v.to_le_bytes());
            size = Self::LENGTH_V2;
        }
        if let Some(v) = self.invalid_dyn_symbol_count {
            buf[24..28].copy_from_slice(&v.to_le_bytes());
            size = Self::LENGTH;
        }
        if let Some(v) = self.encoding_code_page {
            buf[28..32].copy_from_slice(&v.to_le_bytes());
            size = Self::LENGTH;
        }
        if let Some(flags) = self.flags {
            buf[32..36].copy_from_slice(&flags.to_bytes());
            size = Self::LENGTH;
        }
        (buf, size)
    }

    /// Parse from slice, supports v1, v2, and v3
    ///
    /// Returns a tuple containing the struct with bytes written.
    pub fn try_from_slice(data: &[u8]) -> Result<(Self, usize), AdsSymbolUploadInfoError> {
        if data.len() < Self::LENGTH_V1 {
            return Err(AdsSymbolUploadInfoError::TooShort {
                expected: Self::LENGTH_V1,
                got: data.len(),
            });
        }

        let mut info = Self::new(
            u32::from_le_bytes(data[0..4].try_into().unwrap()),
            u32::from_le_bytes(data[4..8].try_into().unwrap()),
        );

        let mut size = Self::LENGTH_V1;

        if data.len() >= Self::LENGTH_V2 {
            info = info
                .with_data_type_count(u32::from_le_bytes(data[8..12].try_into().unwrap()))
                .with_data_type_blob_size(u32::from_le_bytes(data[12..16].try_into().unwrap()))
                .with_dyn_symbol_capacity(u32::from_le_bytes(data[16..20].try_into().unwrap()))
                .with_dyn_symbol_count(u32::from_le_bytes(data[20..24].try_into().unwrap()));
            size = Self::LENGTH_V2;
        }

        if data.len() >= Self::LENGTH {
            info = info
                .with_invalid_dyn_symbol_count(u32::from_le_bytes(data[24..28].try_into().unwrap()))
                .with_encoding_code_page(u32::from_le_bytes(data[28..32].try_into().unwrap()))
                .with_flags(AdsSymbolUploadFlags::from_bytes(
                    data[32..36].try_into().unwrap(),
                ));

            size = Self::LENGTH;
        }

        Ok((info, size))
    }
}

impl From<&AdsSymbolUploadInfo> for [u8; AdsSymbolUploadInfo::LENGTH] {
    fn from(value: &AdsSymbolUploadInfo) -> Self {
        let (info, _) = value.to_bytes();
        info
    }
}

impl From<AdsSymbolUploadInfo> for [u8; AdsSymbolUploadInfo::LENGTH] {
    fn from(value: AdsSymbolUploadInfo) -> Self {
        let (info, _) = value.to_bytes();
        info
    }
}

impl TryFrom<&[u8]> for AdsSymbolUploadInfo {
    type Error = AdsSymbolUploadInfoError;
    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        let (info, _) = Self::try_from_slice(data)?;
        Ok(info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn real_v1_bytes() -> [u8; AdsSymbolUploadInfo::LENGTH_V1] {
        [
            0x91, 0x00, 0x00, 0x00, // symbol_count        = 145
            0x60, 0x3d, 0x00, 0x00, // symbol_byte_size    = 15712
        ]
    }

    fn real_v2_bytes() -> [u8; AdsSymbolUploadInfo::LENGTH_V2] {
        [
            0x91, 0x00, 0x00, 0x00, // symbol_count       = 145
            0x60, 0x3d, 0x00, 0x00, // symbol_byte_size    = 15712
            0x39, 0x00, 0x00, 0x00, // data_type_count     = 57
            0x20, 0x2c, 0x00, 0x00, // data_type_byte_size = 11296
            0xd0, 0x07, 0x00, 0x00, // dyn_symbol_capacity = 2000
            0x00, 0x00, 0x00, 0x00, // dyn_symbol_count    = 0
        ]
    }

    #[test]
    fn parses_real_capture_v2() {
        let info = AdsSymbolUploadInfo::try_from(real_v2_bytes().as_ref()).unwrap();
        assert_eq!(info.symbol_count(), 145);
        assert_eq!(info.symbol_blob_size(), 15712);
        assert_eq!(info.data_type_count().unwrap(), 57);
        assert_eq!(info.data_type_blob_size().unwrap(), 11296);
        assert_eq!(info.dyn_symbol_capacity().unwrap(), 2000);
        assert_eq!(info.dyn_symbol_count().unwrap(), 0);
    }

    #[test]
    fn roundtrip_bytes() {
        let original = AdsSymbolUploadInfo::try_from(real_v2_bytes().as_ref()).unwrap();
        let (bytes, written) = original.to_bytes();
        let parsed = AdsSymbolUploadInfo::try_from(&bytes[0..written]).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn to_bytes_length_is_correct() {
        let (_, written) = AdsSymbolUploadInfo::try_from(real_v1_bytes().as_ref())
            .unwrap()
            .to_bytes();
        assert_eq!(written, AdsSymbolUploadInfo::LENGTH_V1);

        let (_, written) = AdsSymbolUploadInfo::try_from(real_v2_bytes().as_ref())
            .unwrap()
            .to_bytes();
        assert_eq!(written, AdsSymbolUploadInfo::LENGTH_V2);
    }

    #[test]
    fn too_short_returns_err() {
        let err = AdsSymbolUploadInfo::try_from_slice(&[0u8; 1]).unwrap_err();
        assert!(matches!(
            err,
            AdsSymbolUploadInfoError::TooShort {
                expected: 8,
                got: 1
            }
        ));
    }

    #[test]
    fn empty_returns_err() {
        assert!(AdsSymbolUploadInfo::try_from_slice(&[]).is_err());
    }
}
