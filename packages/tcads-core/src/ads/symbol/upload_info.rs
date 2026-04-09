use super::AdsSymbolUploadFlags;
use super::error::AdsSymbolUploadInfoError;
use crate::AdsSymbolUploadInfo::V3;

/// Metadata describing the symbols and data types available on a PLC runtime.
///
/// Contains counts and total byte sizes for the symbol and data type blobs,
/// which must be fetched before downloading them to know how much to allocate.
///
/// Three versions exist depending on the TwinCAT runtime version. The version
/// is determined by the number of bytes returned by the server:
///
/// | Variant | Size     | Description                                             |
/// |---------|----------|---------------------------------------------------------|
/// | `V1`    | 8 bytes  | Symbol count and blob size only                         |
/// | `V2`    | 24 bytes | Adds data type count/size and dynamic symbol fields     |
/// | `V3`    | 64 bytes | Adds invalid dynamic symbols, encoding, flags, reserved |
///
/// # Full Wire Format
///
/// | Offset | Size | Field                      | Version | Description                                  |
/// |--------|------|----------------------------|---------|----------------------------------------------|
/// | 0      | 4    | `symbol_count`             | 1+      | Number of symbols                            |
/// | 4      | 4    | `symbol_blob_size`         | 1+      | Total byte size of the symbol blob           |
/// | 8      | 4    | `data_type_count`          | 2+      | Number of data types                         |
/// | 12     | 4    | `data_type_blob_size`      | 2+      | Total byte size of the data type blob        |
/// | 16     | 4    | `dyn_symbol_capacity`      | 2+      | Maximum number of dynamic symbols            |
/// | 20     | 4    | `dyn_symbol_count`         | 2+      | Dynamic symbols currently in use             |
/// | 24     | 4    | `invalid_dyn_symbol_count` | 3+      | Invalid dynamic symbols                      |
/// | 28     | 4    | `encoding_code_page`       | 3+      | String encoding (e.g. `1252` = Windows-1252) |
/// | 32     | 4    | `flags`                    | 3+      | [`AdsSymbolUploadFlags`]                     |
/// | 36     | 28   | reserved                   | 3+      | Reserved for future use (7 x u32)            |
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum AdsSymbolUploadInfo {
    /// Version 1 symbol upload info (8 bytes).
    /// Symbol count and blob size only.
    V1(AdsSymbolUploadInfoV1),
    /// Version 2 symbol upload info (24 bytes).
    /// Adds data type count/size and dynamic symbol fields to [`version 1`](Self::V1).
    V2(AdsSymbolUploadInfoV2),
    /// Version 3 symbol upload info (64 bytes).
    /// Adds invalid dynamic symbol count, string encoding, and runtime flags to
    /// [`version 2`](Self::V2).
    V3(AdsSymbolUploadInfoV3),
}

impl AdsSymbolUploadInfo {
    /// Number of symbols in the symbol blob.
    pub fn symbol_count(&self) -> u32 {
        match self {
            Self::V1(v) => v.symbol_count(),
            Self::V2(v) => v.symbol_count(),
            Self::V3(v) => v.symbol_count(),
        }
    }

    /// Total byte size of the symbol blob.
    ///
    /// Use this as the read length when fetching the full symbol blob.
    pub fn symbol_blob_size(&self) -> u32 {
        match self {
            Self::V1(v) => v.symbol_blob_size(),
            Self::V2(v) => v.symbol_blob_size(),
            Self::V3(v) => v.symbol_blob_size(),
        }
    }

    /// Number of data types in the data type blob.
    ///
    /// Return `None` if the runtime does not support data type blobs.
    pub fn data_type_count(&self) -> Option<u32> {
        match self {
            Self::V1(_) => None,
            Self::V2(v) => Some(v.data_type_count()),
            Self::V3(v) => Some(v.data_type_count()),
        }
    }

    /// Total byte size of the data type blob.
    ///
    /// Use this as the read length when fetching the full data type blob.
    ///
    /// Return `None` if the runtime does not support data type blobs.
    pub fn data_type_blob_size(&self) -> Option<u32> {
        match self {
            Self::V1(_) => None,
            Self::V2(v) => Some(v.data_type_blob_size()),
            Self::V3(v) => Some(v.data_type_blob_size()),
        }
    }

    /// Maximum number of dynamic symbols supported by this runtime.
    ///
    /// Returns `None` if the runtime does not support the retrieval of dynamic symbol capacity.
    pub fn dyn_symbol_capacity(&self) -> Option<u32> {
        match self {
            Self::V1(_) => None,
            Self::V2(v) => Some(v.dyn_symbol_capacity()),
            Self::V3(v) => Some(v.dyn_symbol_capacity()),
        }
    }

    /// Number of dynamic symbols currently in use.
    ///
    /// Returns `None` if the runtime does not support the retrieval of dynamic symbol count.
    pub fn dyn_symbol_count(&self) -> Option<u32> {
        match self {
            Self::V1(_) => None,
            Self::V2(v) => Some(v.dyn_symbol_count()),
            Self::V3(v) => Some(v.dyn_symbol_count()),
        }
    }

    /// String encoding code page for symbol and type names.
    ///
    /// `1252` = Windows-1252 (Western European), the default for most TwinCAT runtimes.
    ///
    /// Returns `None` if the runtime does not support retreival of the encoding code page.
    pub fn encoding_code_page(&self) -> Option<u32> {
        match self {
            V3(v) => Some(v.encoding_code_page),
            _ => None,
        }
    }

    /// Runtime flags.
    ///
    /// Returns `None` if the runtime does no support retrieval of runtime flags.
    pub fn flags(&self) -> Option<AdsSymbolUploadFlags> {
        match self {
            V3(v) => Some(v.flags),
            _ => None,
        }
    }

    /// Wire size in bytes.
    pub fn wire_size(&self) -> usize {
        match self {
            Self::V1(_) => AdsSymbolUploadInfoV1::LENGTH,
            Self::V2(_) => AdsSymbolUploadInfoV2::LENGTH,
            Self::V3(_) => AdsSymbolUploadInfoV3::LENGTH,
        }
    }

    /// Serializes to bytes.
    ///
    /// Returns a heap-allocated `Vec<u8>` sized to the exact wire length for
    /// this version (`8`, `24`, or `64` bytes).
    pub fn to_vec(&self) -> Vec<u8> {
        match self {
            Self::V1(v) => v.to_bytes().to_vec(),
            Self::V2(v) => v.to_bytes().to_vec(),
            Self::V3(v) => v.to_bytes().to_vec(),
        }
    }

    /// Parses from a byte slice, detecting the version from the slice length.
    ///
    /// - `>= 64 bytes` = [`V3`](Self::V3)
    /// - `>= 24 bytes` = [`V2`](Self::V2)
    /// - `>= 8 bytes`  = [`V1`](Self::V1)
    ///
    /// Returns [`AdsSymbolUploadInfoError::TooShort`] if fewer than 8 bytes
    /// are provided.
    pub fn try_from_slice(data: &[u8]) -> Result<Self, AdsSymbolUploadInfoError> {
        if data.len() >= AdsSymbolUploadInfoV3::LENGTH {
            Ok(Self::V3(AdsSymbolUploadInfoV3::try_from_slice(data)?))
        } else if data.len() >= AdsSymbolUploadInfoV2::LENGTH {
            Ok(Self::V2(AdsSymbolUploadInfoV2::try_from_slice(data)?))
        } else if data.len() >= AdsSymbolUploadInfoV1::LENGTH {
            Ok(Self::V1(AdsSymbolUploadInfoV1::try_from_slice(data)?))
        } else {
            Err(AdsSymbolUploadInfoError::TooShort {
                expected: AdsSymbolUploadInfoV1::LENGTH,
                got: data.len(),
            })
        }
    }
}

impl TryFrom<&[u8]> for AdsSymbolUploadInfo {
    type Error = AdsSymbolUploadInfoError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        Self::try_from_slice(data)
    }
}

impl From<AdsSymbolUploadInfoV1> for AdsSymbolUploadInfo {
    fn from(value: AdsSymbolUploadInfoV1) -> Self {
        Self::V1(value)
    }
}

impl From<AdsSymbolUploadInfoV2> for AdsSymbolUploadInfo {
    fn from(value: AdsSymbolUploadInfoV2) -> Self {
        Self::V2(value)
    }
}

impl From<AdsSymbolUploadInfoV3> for AdsSymbolUploadInfo {
    fn from(value: AdsSymbolUploadInfoV3) -> Self {
        Self::V3(value)
    }
}

/// Version 1 symbol upload info (8 bytes).
///
/// Contains only the symbol count and blob size.
///
/// # Wire Format
///
/// | Offset | Size | Field              | Description                        |
/// |--------|------|--------------------|------------------------------------|
/// | 0      | 4    | `symbol_count`     | Number of symbols                  |
/// | 4      | 4    | `symbol_blob_size` | Total byte size of the symbol blob |
#[derive(
    serde::Serialize, serde::Deserialize, Debug, Copy, Clone, PartialEq, Eq, Hash, Default,
)]
pub struct AdsSymbolUploadInfoV1 {
    symbol_count: u32,
    symbol_blob_size: u32,
}

impl AdsSymbolUploadInfoV1 {
    /// Wire size in bytes of a version 1 response.
    pub const LENGTH: usize = 8;

    /// Creates a new version 1 instance.
    pub fn new(symbol_count: u32, symbol_blob_size: u32) -> Self {
        Self {
            symbol_count,
            symbol_blob_size,
        }
    }

    /// Number of symbols in the symbol blob.
    pub fn symbol_count(&self) -> u32 {
        self.symbol_count
    }

    /// Total byte size of the symbol blob.
    ///
    /// Use this as the read length when fetching the full symbol blob.
    pub fn symbol_blob_size(&self) -> u32 {
        self.symbol_blob_size
    }

    /// Serializes to bytes.
    pub fn to_bytes(&self) -> [u8; Self::LENGTH] {
        let mut buf = [0u8; Self::LENGTH];
        buf[0..4].copy_from_slice(&self.symbol_count.to_le_bytes());
        buf[4..8].copy_from_slice(&self.symbol_blob_size.to_le_bytes());
        buf
    }

    /// Parses from a byte array.
    pub fn from_bytes(data: [u8; Self::LENGTH]) -> Self {
        Self::new(
            u32::from_le_bytes([data[0], data[1], data[2], data[3]]),
            u32::from_le_bytes([data[4], data[5], data[6], data[7]]),
        )
    }

    /// Parses from a byte slice.
    ///
    /// Returns [`AdsSymbolUploadInfoError::TooShort`] if fewer than
    /// [`LENGTH`](Self::LENGTH) bytes are provided.
    pub fn try_from_slice(data: &[u8]) -> Result<Self, AdsSymbolUploadInfoError> {
        if data.len() < Self::LENGTH {
            return Err(AdsSymbolUploadInfoError::TooShort {
                expected: Self::LENGTH,
                got: data.len(),
            });
        }
        Ok(Self::from_bytes([
            data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
        ]))
    }
}

impl TryFrom<&[u8]> for AdsSymbolUploadInfoV1 {
    type Error = AdsSymbolUploadInfoError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        Self::try_from_slice(data)
    }
}

/// Version 2 symbol upload info (24 bytes).
///
/// Extends version 1 with data type counts and dynamic symbol fields.
///
/// # Wire Format
///
/// | Offset | Size | Field                 | Description                           |
/// |--------|------|--------------------   |---------------------------------------|
/// | 0      | 4    | `symbol_count`        | Number of symbols                     |
/// | 4      | 4    | `symbol_blob_size`    | Total byte size of the symbol blob    |
/// | 8      | 4    | `data_type_count`     | Number of data types                  |
/// | 12     | 4    | `data_type_blob_size` | Total byte size of the data type blob |
/// | 16     | 4    | `dyn_symbol_capacity` | Maximum number of dynamic symbols     |
/// | 20     | 4    | `dyn_symbol_count`    | Dynamic symbols currently in use      |
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct AdsSymbolUploadInfoV2 {
    symbol_count: u32,
    symbol_blob_size: u32,
    data_type_count: u32,
    data_type_blob_size: u32,
    dyn_symbol_capacity: u32,
    dyn_symbol_count: u32,
}

impl AdsSymbolUploadInfoV2 {
    /// Wire size of a version 2 response.
    pub const LENGTH: usize = 24;

    /// Creates a new version 2 instance.
    pub fn new(
        symbol_count: u32,
        symbol_blob_size: u32,
        data_type_count: u32,
        data_type_blob_size: u32,
        dyn_symbol_capacity: u32,
        dyn_symbol_count: u32,
    ) -> Self {
        Self {
            symbol_count,
            symbol_blob_size,
            data_type_count,
            data_type_blob_size,
            dyn_symbol_capacity,
            dyn_symbol_count,
        }
    }

    /// Number of symbols in the symbol blob.
    pub fn symbol_count(&self) -> u32 {
        self.symbol_count
    }

    /// Total byte size of the symbol blob.
    ///
    /// Use this as the read length when fetching the full symbol blob.
    pub fn symbol_blob_size(&self) -> u32 {
        self.symbol_blob_size
    }

    /// Number of data types in the data type blob.
    pub fn data_type_count(&self) -> u32 {
        self.data_type_count
    }

    /// Total byte size of the data type blob.
    ///
    /// Use this as the read length when fetching the full data type blob.
    pub fn data_type_blob_size(&self) -> u32 {
        self.data_type_blob_size
    }

    /// Maximum number of dynamic symbols supported by this runtime.
    pub fn dyn_symbol_capacity(&self) -> u32 {
        self.dyn_symbol_capacity
    }

    /// Number of dynamic symbols currently in use.
    pub fn dyn_symbol_count(&self) -> u32 {
        self.dyn_symbol_count
    }

    /// Serializes to bytes.
    pub fn to_bytes(&self) -> [u8; Self::LENGTH] {
        let mut buf = [0u8; Self::LENGTH];
        buf[0..4].copy_from_slice(&self.symbol_count.to_le_bytes());
        buf[4..8].copy_from_slice(&self.symbol_blob_size.to_le_bytes());
        buf[8..12].copy_from_slice(&self.data_type_count.to_le_bytes());
        buf[12..16].copy_from_slice(&self.data_type_blob_size.to_le_bytes());
        buf[16..20].copy_from_slice(&self.dyn_symbol_capacity.to_le_bytes());
        buf[20..24].copy_from_slice(&self.dyn_symbol_count.to_le_bytes());
        buf
    }

    /// Parses from a byte array.
    pub fn from_bytes(data: [u8; Self::LENGTH]) -> Self {
        Self::new(
            u32::from_le_bytes([data[0], data[1], data[2], data[3]]),
            u32::from_le_bytes([data[4], data[5], data[6], data[7]]),
            u32::from_le_bytes([data[8], data[9], data[10], data[11]]),
            u32::from_le_bytes([data[12], data[13], data[14], data[15]]),
            u32::from_le_bytes([data[16], data[17], data[18], data[19]]),
            u32::from_le_bytes([data[20], data[21], data[22], data[23]]),
        )
    }

    /// Parses from a byte slice.
    ///
    /// Returns [`AdsSymbolUploadInfoError::TooShort`] if fewer than
    /// [`LENGTH`](Self::LENGTH) bytes are provided.
    pub fn try_from_slice(data: &[u8]) -> Result<Self, AdsSymbolUploadInfoError> {
        if data.len() < Self::LENGTH {
            return Err(AdsSymbolUploadInfoError::TooShort {
                expected: Self::LENGTH,
                got: data.len(),
            });
        }

        Ok(Self::from_bytes([
            data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7], data[8],
            data[9], data[10], data[11], data[12], data[13], data[14], data[15], data[16],
            data[17], data[18], data[19], data[20], data[21], data[22], data[23],
        ]))
    }
}

impl TryFrom<&[u8]> for AdsSymbolUploadInfoV2 {
    type Error = AdsSymbolUploadInfoError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        Self::try_from_slice(data)
    }
}

/// Version 3 symbol upload info (64 bytes).
///
/// Extends version 2 with invalid dynamic symbol count, string encoding, and
/// runtime flags. The remaining 28 bytes are reserved for future use.
///
/// # Wire Format
///
/// | Offset | Size | Field                      | Description                                  |
/// |--------|------|--------------------        |----------------------------------------------|
/// | 0      | 4    | `symbol_count`             | Number of symbols                            |
/// | 4      | 4    | `symbol_blob_size`         | Total byte size of the symbol blob           |
/// | 8      | 4    | `data_type_count`          | Number of data types                         |
/// | 12     | 4    | `data_type_blob_size`      | Total byte size of the data type blob        |
/// | 16     | 4    | `dyn_symbol_capacity`      | Maximum number of dynamic symbols            |
/// | 20     | 4    | `dyn_symbol_count`         | Dynamic symbols currently in use             |
/// | 24     | 4    | `invalid_dyn_symbol_count` | Invalid dynamic symbols                      |
/// | 28     | 4    | `encoding_code_page`       | String encoding (e.g. `1252` = Windows-1252) |
/// | 32     | 4    | `flags`                    | [`AdsSymbolUploadFlags`]                     |
/// | 36     | 28   | reserved                   | Reserved for future use (7 x u32)            |
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct AdsSymbolUploadInfoV3 {
    symbol_count: u32,
    symbol_blob_size: u32,
    data_type_count: u32,
    data_type_blob_size: u32,
    dyn_symbol_capacity: u32,
    dyn_symbol_count: u32,
    invalid_dyn_symbol_count: u32,
    encoding_code_page: u32,
    flags: AdsSymbolUploadFlags,
}

impl AdsSymbolUploadInfoV3 {
    /// Wire size of a version 3 response.
    pub const LENGTH: usize = 64;

    /// Creates a new version 3 instance.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        symbol_count: u32,
        symbol_blob_size: u32,
        data_type_count: u32,
        data_type_blob_size: u32,
        dyn_symbol_capacity: u32,
        dyn_symbol_count: u32,
        invalid_dyn_symbol_count: u32,
        encoding_code_page: u32,
        flags: AdsSymbolUploadFlags,
    ) -> Self {
        Self {
            symbol_count,
            symbol_blob_size,
            data_type_count,
            data_type_blob_size,
            dyn_symbol_capacity,
            dyn_symbol_count,
            invalid_dyn_symbol_count,
            encoding_code_page,
            flags,
        }
    }

    /// Number of symbols in the symbol blob.
    pub fn symbol_count(&self) -> u32 {
        self.symbol_count
    }

    /// Total byte size of the symbol blob .
    ///
    /// Use this as the read length when fetching the full symbol blob.
    pub fn symbol_blob_size(&self) -> u32 {
        self.symbol_blob_size
    }

    /// Number of data types in the data type blob.
    pub fn data_type_count(&self) -> u32 {
        self.data_type_count
    }

    /// Total byte size of the data type blob.
    ///
    /// Use this as the read length when fetching the full data type blob.
    pub fn data_type_blob_size(&self) -> u32 {
        self.data_type_blob_size
    }

    /// Maximum number of dynamic symbols supported by this runtime.
    pub fn dyn_symbol_capacity(&self) -> u32 {
        self.dyn_symbol_capacity
    }

    /// Number of dynamic symbols currently in use.
    pub fn dyn_symbol_count(&self) -> u32 {
        self.dyn_symbol_count
    }

    /// Number of invalid dynamic symbols.
    pub fn invalid_dyn_symbol_count(&self) -> u32 {
        self.invalid_dyn_symbol_count
    }

    /// String encoding code page for symbol and type names.
    ///
    /// `1252` = Windows-1252 (Western European), the default for most TwinCAT runtimes.
    pub fn encoding_code_page(&self) -> u32 {
        self.encoding_code_page
    }

    /// Runtime flags.
    pub fn flags(&self) -> AdsSymbolUploadFlags {
        self.flags
    }

    /// Serializes to bytes. Reserved bytes are written as zero.
    pub fn to_bytes(&self) -> [u8; Self::LENGTH] {
        let mut buf = [0u8; Self::LENGTH];
        buf[0..4].copy_from_slice(&self.symbol_count.to_le_bytes());
        buf[4..8].copy_from_slice(&self.symbol_blob_size.to_le_bytes());
        buf[8..12].copy_from_slice(&self.data_type_count.to_le_bytes());
        buf[12..16].copy_from_slice(&self.data_type_blob_size.to_le_bytes());
        buf[16..20].copy_from_slice(&self.dyn_symbol_capacity.to_le_bytes());
        buf[20..24].copy_from_slice(&self.dyn_symbol_count.to_le_bytes());
        buf[24..28].copy_from_slice(&self.invalid_dyn_symbol_count.to_le_bytes());
        buf[28..32].copy_from_slice(&self.encoding_code_page.to_le_bytes());
        buf[32..36].copy_from_slice(&self.flags.to_bytes());
        buf
    }

    /// Parses from a byte array.
    pub fn from_bytes(data: [u8; Self::LENGTH]) -> Self {
        Self::new(
            u32::from_le_bytes([data[0], data[1], data[2], data[3]]),
            u32::from_le_bytes([data[4], data[5], data[6], data[7]]),
            u32::from_le_bytes([data[8], data[9], data[10], data[11]]),
            u32::from_le_bytes([data[12], data[13], data[14], data[15]]),
            u32::from_le_bytes([data[16], data[17], data[18], data[19]]),
            u32::from_le_bytes([data[20], data[21], data[22], data[23]]),
            u32::from_le_bytes([data[24], data[25], data[26], data[27]]),
            u32::from_le_bytes([data[28], data[29], data[30], data[31]]),
            AdsSymbolUploadFlags::from_bytes([data[32], data[33], data[34], data[35]]),
        )
    }

    /// Parses from a byte slice.
    pub fn try_from_slice(data: &[u8]) -> Result<Self, AdsSymbolUploadInfoError> {
        if data.len() != Self::LENGTH {
            return Err(AdsSymbolUploadInfoError::UnexpectedLength {
                expected: Self::LENGTH,
                got: data.len(),
            });
        }
        let mut bytes = [0u8; Self::LENGTH];
        bytes.copy_from_slice(data);

        Ok(Self::from_bytes(bytes))
    }
}

impl TryFrom<&[u8]> for AdsSymbolUploadInfoV3 {
    type Error = AdsSymbolUploadInfoError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        Self::try_from_slice(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn real_v1_bytes() -> [u8; AdsSymbolUploadInfoV1::LENGTH] {
        [
            0x91, 0x00, 0x00, 0x00, // symbol_count        = 145
            0x60, 0x3d, 0x00, 0x00, // symbol_byte_size    = 15712
        ]
    }

    fn real_v2_bytes() -> [u8; AdsSymbolUploadInfoV2::LENGTH] {
        [
            0x91, 0x00, 0x00, 0x00, // symbol_count       = 145
            0x60, 0x3d, 0x00, 0x00, // symbol_byte_size    = 15712
            0x39, 0x00, 0x00, 0x00, // data_type_count     = 57
            0x20, 0x2c, 0x00, 0x00, // data_type_blob_size = 11296
            0xd0, 0x07, 0x00, 0x00, // dyn_symbol_capacity = 2000
            0x00, 0x00, 0x00, 0x00, // dyn_symbol_count    = 0
        ]
    }

    fn real_v3_bytes() -> [u8; AdsSymbolUploadInfoV3::LENGTH] {
        [
            0x91, 0x00, 0x00, 0x00, // symbol_count             = 145
            0x60, 0x3d, 0x00, 0x00, // symbol_byte_size         = 15712
            0x39, 0x00, 0x00, 0x00, // data_type_count          = 57
            0x20, 0x2c, 0x00, 0x00, // data_type_blob_size      = 11296
            0xd0, 0x07, 0x00, 0x00, // dyn_symbol_capacity      = 2000
            0x00, 0x00, 0x00, 0x00, // dyn_symbol_count         = 0
            0x00, 0x00, 0x00, 0x00, // invalid_dyn_symbol_count = 0
            0xe4, 0x04, 0x00, 0x00, // encoding_code_page       = 1252 (Windows-1252)
            0x03, 0x00, 0x00, 0x00, // flags                    = 0x03
            0x00, 0x00, 0x00, 0x00, // reserved
            0x00, 0x00, 0x00, 0x00, // reserved
            0x00, 0x00, 0x00, 0x00, // reserved
            0x00, 0x00, 0x00, 0x00, // reserved
            0x00, 0x00, 0x00, 0x00, // reserved
            0x00, 0x00, 0x00, 0x00, // reserved
            0x00, 0x00, 0x00, 0x00, // reserved
        ]
    }

    #[test]
    fn parses_v1() {
        let info = AdsSymbolUploadInfo::try_from(real_v1_bytes().as_ref()).unwrap();
        let AdsSymbolUploadInfo::V1(v1) = &info else {
            panic!("expected V1")
        };
        assert_eq!(v1.symbol_count(), 145);
        assert_eq!(v1.symbol_blob_size(), 15712);
    }

    #[test]
    fn parses_v2() {
        let info = AdsSymbolUploadInfo::try_from(real_v2_bytes().as_ref()).unwrap();
        let AdsSymbolUploadInfo::V2(v2) = &info else {
            panic!("expected V2")
        };
        assert_eq!(v2.symbol_count(), 145);
        assert_eq!(v2.symbol_blob_size(), 15712);
        assert_eq!(v2.data_type_count(), 57);
        assert_eq!(v2.data_type_blob_size(), 11296);
        assert_eq!(v2.dyn_symbol_capacity(), 2000);
        assert_eq!(v2.dyn_symbol_count(), 0);
    }

    #[test]
    fn parses_v3() {
        let info = AdsSymbolUploadInfo::try_from(real_v3_bytes().as_ref()).unwrap();
        let AdsSymbolUploadInfo::V3(v3) = &info else {
            panic!("expected V3")
        };
        assert_eq!(v3.symbol_count(), 145);
        assert_eq!(v3.symbol_blob_size(), 15712);
        assert_eq!(v3.data_type_count(), 57);
        assert_eq!(v3.data_type_blob_size(), 11296);
        assert_eq!(v3.dyn_symbol_capacity(), 2000);
        assert_eq!(v3.dyn_symbol_count(), 0);
        assert_eq!(v3.invalid_dyn_symbol_count(), 0);
        assert_eq!(v3.encoding_code_page(), 1252);
    }

    #[test]
    fn roundtrip_v1() {
        let original = AdsSymbolUploadInfo::try_from(real_v1_bytes().as_ref()).unwrap();
        let bytes = original.to_vec();
        assert_eq!(bytes.len(), AdsSymbolUploadInfoV1::LENGTH);
        let parsed = AdsSymbolUploadInfo::try_from(bytes.as_slice()).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn roundtrip_v2() {
        let original = AdsSymbolUploadInfo::try_from(real_v2_bytes().as_ref()).unwrap();
        let bytes = original.to_vec();
        assert_eq!(bytes.len(), AdsSymbolUploadInfoV2::LENGTH);
        let parsed = AdsSymbolUploadInfo::try_from(bytes.as_slice()).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn roundtrip_v3() {
        let original = AdsSymbolUploadInfo::try_from(real_v3_bytes().as_ref()).unwrap();
        let bytes = original.to_vec();
        assert_eq!(bytes.len(), AdsSymbolUploadInfoV3::LENGTH);
        let parsed = AdsSymbolUploadInfo::try_from(bytes.as_slice()).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn shared_accessors_work_across_variants() {
        let v1 = AdsSymbolUploadInfo::try_from(real_v1_bytes().as_ref()).unwrap();
        let v2 = AdsSymbolUploadInfo::try_from(real_v2_bytes().as_ref()).unwrap();
        let v3 = AdsSymbolUploadInfo::try_from(real_v3_bytes().as_ref()).unwrap();
        assert_eq!(v1.symbol_count(), 145);
        assert_eq!(v2.symbol_count(), 145);
        assert_eq!(v1.symbol_blob_size(), 15712);
        assert_eq!(v2.symbol_blob_size(), 15712);
        assert_eq!(v3.symbol_count(), 145);
        assert_eq!(v3.symbol_blob_size(), 15712);
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
