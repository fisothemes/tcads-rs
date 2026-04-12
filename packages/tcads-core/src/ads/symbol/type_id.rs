use super::error::AdsTypeInfoError;

/// ADS data type identifier.
///
/// Identifies the machine type of a symbol or the base/element type of a composite.
/// For structs, pointer and references this is typically [`BigType`](Self::BigType); for arrays
/// it identifies the element type; for aliases and enums it identifies the underlying primitive.
///
/// # Wire Format
///
/// | Offset | Size | Field     | Description               |
/// |--------|------|-----------|---------------------------|
/// | 0      | 4    | `type_id` | Type identifier (LE u32)  |
#[derive(
    serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub enum AdsTypeId {
    /// Void / empty (`0`).
    Void,
    /// `INT` - signed 16-bit integer (`2`).
    Int16,
    /// `DINT`- signed 32-bit integer (`3`).
    Int32,
    /// `REAL` - 32-bit IEEE 754 float (`4`).
    Real32,
    /// `LREAL` - 64-bit IEEE 754 float (`5`).
    Real64,
    /// `SINT` - signed 8-bit integer (`16`).
    Int8,
    /// `USINT` / `BYTE` - unsigned 8-bit integer (`17`).
    UInt8,
    /// `UINT` / `WORD` - unsigned 16-bit integer (`18`).
    UInt16,
    /// `UDINT` / `DWORD` - unsigned 32-bit integer (`19`).
    UInt32,
    /// `LINT` - signed 64-bit integer (`20`).
    Int64,
    /// `ULINT` / `LWORD` - unsigned 64-bit integer (`21`).
    UInt64,
    /// `STRING(n)` - fixed-length string (`30`).
    String,
    /// `WSTRING(n)` - fixed-length wide string (`31`).
    WString,
    /// 80-bit extended precision float (`32`)?
    Real80,
    /// `BIT` - single bit (`33`). Also used for `BOOL`.
    Bit,
    /// Internal use only - max. available type (`34`).
    MaxTypes,
    /// Large / composite type - used for structs, references, and aliases (`65`).
    BigType,
    /// An unknown or vendor-specific type ID.
    Unknown(u32),
}

impl AdsTypeId {
    /// Wire size in bytes.
    pub const LENGTH: usize = 4;

    /// Tries to parse a slice of bytes and construct an [`AdsTypeId`] struct.
    pub fn try_from_slice(data: &[u8]) -> Result<Self, AdsTypeInfoError> {
        Self::try_from(data)
    }

    /// Creates from a 4-byte little-endian array.
    pub fn from_bytes(bytes: [u8; Self::LENGTH]) -> Self {
        u32::from_le_bytes(bytes).into()
    }

    /// Converts to a 4-byte little-endian array.
    pub fn to_bytes(self) -> [u8; Self::LENGTH] {
        u32::from(self).to_le_bytes()
    }
}

impl From<u32> for AdsTypeId {
    fn from(value: u32) -> Self {
        match value {
            0 => Self::Void,
            2 => Self::Int16,
            3 => Self::Int32,
            4 => Self::Real32,
            5 => Self::Real64,
            16 => Self::Int8,
            17 => Self::UInt8,
            18 => Self::UInt16,
            19 => Self::UInt32,
            20 => Self::Int64,
            21 => Self::UInt64,
            30 => Self::String,
            31 => Self::WString,
            32 => Self::Real80,
            33 => Self::Bit,
            34 => Self::MaxTypes,
            65 => Self::BigType,
            _ => Self::Unknown(value),
        }
    }
}

impl From<AdsTypeId> for u32 {
    fn from(value: AdsTypeId) -> Self {
        match value {
            AdsTypeId::Void => 0,
            AdsTypeId::Int16 => 2,
            AdsTypeId::Int32 => 3,
            AdsTypeId::Real32 => 4,
            AdsTypeId::Real64 => 5,
            AdsTypeId::Int8 => 16,
            AdsTypeId::UInt8 => 17,
            AdsTypeId::UInt16 => 18,
            AdsTypeId::UInt32 => 19,
            AdsTypeId::Int64 => 20,
            AdsTypeId::UInt64 => 21,
            AdsTypeId::String => 30,
            AdsTypeId::WString => 31,
            AdsTypeId::Real80 => 32,
            AdsTypeId::Bit => 33,
            AdsTypeId::MaxTypes => 34,
            AdsTypeId::BigType => 65,
            AdsTypeId::Unknown(value) => value,
        }
    }
}

impl From<[u8; Self::LENGTH]> for AdsTypeId {
    fn from(bytes: [u8; Self::LENGTH]) -> Self {
        Self::from_bytes(bytes)
    }
}

impl From<AdsTypeId> for [u8; AdsTypeId::LENGTH] {
    fn from(value: AdsTypeId) -> Self {
        value.to_bytes()
    }
}

impl TryFrom<&[u8]> for AdsTypeId {
    type Error = AdsTypeInfoError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() != Self::LENGTH {
            return Err(AdsTypeInfoError::EntryLengthMismatch {
                expected: Self::LENGTH,
                got: value.len(),
            });
        }

        let id = u32::from_le_bytes([value[0], value[1], value[2], value[3]]);
        Ok(id.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_ids_from_captures() {
        assert_eq!(AdsTypeId::from(19u32), AdsTypeId::UInt32); // UDINT
        assert_eq!(AdsTypeId::from(20u32), AdsTypeId::Int64); // LINT
        assert_eq!(AdsTypeId::from(18u32), AdsTypeId::UInt16); // UINT
        assert_eq!(AdsTypeId::from(30u32), AdsTypeId::String); // STRING
        assert_eq!(AdsTypeId::from(33u32), AdsTypeId::Bit); // BOOL
        assert_eq!(AdsTypeId::from(65u32), AdsTypeId::BigType); // struct/reference
        assert_eq!(AdsTypeId::from(17u32), AdsTypeId::UInt8); // BYTE
    }

    #[test]
    fn unknown_preserves_value() {
        let id = AdsTypeId::from(0xDEAD_BEEFu32);
        assert_eq!(id, AdsTypeId::Unknown(0xDEAD_BEEF));
        assert_eq!(u32::from(id), 0xDEAD_BEEF);
    }

    #[test]
    fn bytes_roundtrip() {
        for id in [AdsTypeId::UInt32, AdsTypeId::Int64, AdsTypeId::BigType] {
            assert_eq!(AdsTypeId::from_bytes(id.to_bytes()), id);
        }
    }
}
