use super::error::AdsSymbolUploadInfoError;

/// Metadata describing the symbols and data types available on a PLC runtime.
///
/// Contains counts and total byte sizes for the symbol and data type blobs,
/// which must be fetched before downloading them to know how much to allocate.
///
/// # Wire Format
///
/// | Offset | Size | Field                    | Description                                    |
/// |--------|------|--------------------------|------------------------------------------------|
/// | 0      | 4    | `symbol_count`           | Number of symbols                              |
/// | 4      | 4    | `symbol_byte_size`       | Total byte size of the symbol blob             |
/// | 8      | 4    | `data_type_count`        | Number of data types                           |
/// | 12     | 4    | `data_type_byte_size`    | Total byte size of the data type blob          |
/// | 16     | 4    | `dyn_symbol_capacity`    | Maximum number of dynamic symbols              |
/// | 20     | 4    | `dyn_symbol_count`       | Number of dynamic symbols currently in use     |
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsSymbolUploadInfo {
    symbol_count: u32,
    symbol_byte_size: u32,
    data_type_count: u32,
    data_type_byte_size: u32,
    dyn_symbol_capacity: u32,
    dyn_symbol_count: u32,
}

impl AdsSymbolUploadInfo {
    /// Wire size in bytes.
    pub const LENGTH: usize = 24;

    /// Creates a new instance of [`AdsSymbolUploadInfo`].
    pub fn new(
        symbol_count: u32,
        symbol_byte_size: u32,
        data_type_count: u32,
        data_type_byte_size: u32,
        dyn_symbol_capacity: u32,
        dyn_symbol_count: u32,
    ) -> Self {
        Self {
            symbol_count,
            symbol_byte_size,
            data_type_count,
            data_type_byte_size,
            dyn_symbol_capacity,
            dyn_symbol_count,
        }
    }

    /// Number of symbols in the symbol blob.
    pub fn symbol_count(&self) -> u32 {
        self.symbol_count
    }

    /// Total byte size of the symbol blob (`ADSIGRP_SYM_UPLOAD`, `0xF00B`).
    ///
    /// Use this as the read length when fetching the full symbol blob.
    pub fn symbol_byte_size(&self) -> u32 {
        self.symbol_byte_size
    }

    /// Number of data types in the data type blob.
    pub fn data_type_count(&self) -> u32 {
        self.data_type_count
    }

    /// Total byte size of the data type blob (`ADSIGRP_SYM_DT_UPLOAD`, `0xF00E`).
    ///
    /// Use this as the read length when fetching the full data type blob.
    pub fn data_type_byte_size(&self) -> u32 {
        self.data_type_byte_size
    }

    /// Maximum number of dynamic symbols supported by this runtime.
    pub fn dyn_symbol_capacity(&self) -> u32 {
        self.dyn_symbol_capacity
    }

    /// Number of dynamic symbols currently in use.
    pub fn dyn_symbol_count(&self) -> u32 {
        self.dyn_symbol_count
    }

    /// Serialises to a 24-byte little-endian array.
    pub fn to_bytes(&self) -> [u8; Self::LENGTH] {
        let mut buf = [0u8; Self::LENGTH];
        buf[0..4].copy_from_slice(&self.symbol_count.to_le_bytes());
        buf[4..8].copy_from_slice(&self.symbol_byte_size.to_le_bytes());
        buf[8..12].copy_from_slice(&self.data_type_count.to_le_bytes());
        buf[12..16].copy_from_slice(&self.data_type_byte_size.to_le_bytes());
        buf[16..20].copy_from_slice(&self.dyn_symbol_capacity.to_le_bytes());
        buf[20..24].copy_from_slice(&self.dyn_symbol_count.to_le_bytes());
        buf
    }

    /// Parses an `AdsSymbolUploadInfo` from a byte slice.
    pub fn try_from_slice(data: &[u8]) -> Result<Self, AdsSymbolUploadInfoError> {
        if data.len() < Self::LENGTH {
            return Err(AdsSymbolUploadInfoError::UnexpectedLength {
                expected: Self::LENGTH,
                got: data.len(),
            });
        }
        Ok(Self {
            symbol_count: u32::from_le_bytes(data[0..4].try_into().unwrap()),
            symbol_byte_size: u32::from_le_bytes(data[4..8].try_into().unwrap()),
            data_type_count: u32::from_le_bytes(data[8..12].try_into().unwrap()),
            data_type_byte_size: u32::from_le_bytes(data[12..16].try_into().unwrap()),
            dyn_symbol_capacity: u32::from_le_bytes(data[16..20].try_into().unwrap()),
            dyn_symbol_count: u32::from_le_bytes(data[20..24].try_into().unwrap()),
        })
    }
}

impl From<&AdsSymbolUploadInfo> for [u8; AdsSymbolUploadInfo::LENGTH] {
    fn from(value: &AdsSymbolUploadInfo) -> Self {
        value.to_bytes()
    }
}

impl From<AdsSymbolUploadInfo> for [u8; AdsSymbolUploadInfo::LENGTH] {
    fn from(value: AdsSymbolUploadInfo) -> Self {
        value.to_bytes()
    }
}

impl TryFrom<&[u8]> for AdsSymbolUploadInfo {
    type Error = AdsSymbolUploadInfoError;
    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        Self::try_from_slice(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Real captured response from ADSIGRP_SYM_UPLOADINFO2
    fn real_bytes() -> [u8; 24] {
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
    fn parses_real_capture() {
        let info = AdsSymbolUploadInfo::try_from_slice(&real_bytes()).unwrap();
        assert_eq!(info.symbol_count(), 145);
        assert_eq!(info.symbol_byte_size(), 15712);
        assert_eq!(info.data_type_count(), 57);
        assert_eq!(info.data_type_byte_size(), 11296);
        assert_eq!(info.dyn_symbol_capacity(), 2000);
        assert_eq!(info.dyn_symbol_count(), 0);
    }

    #[test]
    fn roundtrip_bytes() {
        let original = AdsSymbolUploadInfo::try_from_slice(&real_bytes()).unwrap();
        let bytes = original.to_bytes();
        let parsed = AdsSymbolUploadInfo::try_from_slice(&bytes).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn to_bytes_length_is_correct() {
        let info = AdsSymbolUploadInfo::try_from_slice(&real_bytes()).unwrap();
        assert_eq!(info.to_bytes().len(), AdsSymbolUploadInfo::LENGTH);
    }

    #[test]
    fn too_short_returns_err() {
        let err = AdsSymbolUploadInfo::try_from_slice(&[0u8; 23]).unwrap_err();
        assert!(matches!(
            err,
            AdsSymbolUploadInfoError::UnexpectedLength {
                expected: 24,
                got: 23
            }
        ));
    }

    #[test]
    fn empty_returns_err() {
        assert!(AdsSymbolUploadInfo::try_from_slice(&[]).is_err());
    }
}
