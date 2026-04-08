use super::error::AdsSymbolUploadInfoError;
use bitflags::bitflags;

bitflags! {
    /// Flags describing properties of a TwinCAT PLC runtime returned in
    /// [`AdsSymbolUploadInfo`](super::AdsSymbolUploadInfo).
    ///
    /// # Wire Format
    ///
    /// | Offset | Size | Field   | Description      |
    /// |--------|------|---------|------------------|
    /// | 0      | 4    | `flags` | Bitmask (LE u32) |
    ///
    /// Only present in version 3 (64-byte) upload info responses.
    #[derive(
        serde::Serialize, serde::Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default,
    )]
    #[repr(transparent)]
    pub struct AdsSymbolUploadFlags: u32 {
        /// The target runtime is a 64-bit platform. Pointer and reference sizes are 8 bytes.
        const IS_64BIT_PLATFORM = 0x00000001;
        /// The data type blob includes built-in base types (e.g. `UDINT`, `LINT`).
        const INCLUDES_BASE_TYPES = 0x00000002;
        /// All PLC `STRING` data is UTF-8 encoded. Set from TwinCAT 4026 onwards.
        const UTF8_ENCODED_STRING_DATA = 0x00000004;
    }
}

impl AdsSymbolUploadFlags {
    /// Wire size in bytes.
    pub const LENGTH: usize = 4;

    /// Creates a new `AdsSymbolUploadFlags` from a raw `u32`.
    pub const fn new(raw: u32) -> Self {
        Self::from_bits_retain(raw)
    }

    /// Creates from a 4-byte little-endian array.
    pub const fn from_bytes(bytes: [u8; Self::LENGTH]) -> Self {
        Self::from_bits_retain(u32::from_le_bytes(bytes))
    }

    /// Converts to a 4-byte little-endian array.
    pub const fn to_bytes(self) -> [u8; Self::LENGTH] {
        self.bits().to_le_bytes()
    }

    /// Parses a slice of bytes and create a new instance.
    pub fn try_from_slice(bytes: &[u8]) -> Result<Self, AdsSymbolUploadInfoError> {
        if bytes.len() != Self::LENGTH {
            return Err(AdsSymbolUploadInfoError::UnexpectedLength {
                expected: Self::LENGTH,
                got: bytes.len(),
            });
        }

        Ok(Self::from_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// Returns the raw `u32` value.
    pub const fn as_raw(self) -> u32 {
        self.bits()
    }

    /// Returns `true` if the target runtime is a 64-bit platform.
    /// When true, pointer and reference sizes are 8 bytes instead of 4.
    pub const fn is_64bit_platform(self) -> bool {
        self.contains(Self::IS_64BIT_PLATFORM)
    }

    /// Returns `true` if the data type blob includes built-in base types.
    pub const fn includes_base_types(self) -> bool {
        self.contains(Self::INCLUDES_BASE_TYPES)
    }

    /// Returns `true` if PLC `STRING` values are UTF-8 encoded.
    /// When `false`, strings use the code page indicated by
    /// [`AdsSymbolUploadInfo::encoding_code_page`](super::AdsSymbolUploadInfo::encoding_code_page).
    pub const fn is_utf8_encoded(self) -> bool {
        self.contains(Self::UTF8_ENCODED_STRING_DATA)
    }
}

impl From<u32> for AdsSymbolUploadFlags {
    fn from(raw: u32) -> Self {
        Self::from_bits_retain(raw)
    }
}

impl From<AdsSymbolUploadFlags> for u32 {
    fn from(flags: AdsSymbolUploadFlags) -> Self {
        flags.bits()
    }
}

impl TryFrom<&[u8]> for AdsSymbolUploadFlags {
    type Error = AdsSymbolUploadInfoError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Self::try_from_slice(value)
    }
}

impl std::fmt::Display for AdsSymbolUploadFlags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_empty() {
            f.write_str("None")
        } else {
            bitflags::parser::to_writer(self, f)
        }
    }
}

impl std::fmt::Debug for AdsSymbolUploadFlags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("AdsSymbolUploadFlags")
            .field(&format_args!("{:#010X}", self.bits()))
            .field(&format_args!("{}", self))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const UTF8_64_BIT_PLATFORM_DATA: u32 = 0x00000005;
    const INCLUDES_BASE_TYPES_DATA_AND_64_BIT: u32 = 0x00000003;

    #[test]
    fn create_flags_from_raw() {
        assert_eq!(AdsSymbolUploadFlags::new(0), AdsSymbolUploadFlags::empty());
        assert_eq!(
            AdsSymbolUploadFlags::new(1),
            AdsSymbolUploadFlags::IS_64BIT_PLATFORM
        );
        assert_eq!(
            AdsSymbolUploadFlags::new(2),
            AdsSymbolUploadFlags::INCLUDES_BASE_TYPES
        );
        assert_eq!(
            AdsSymbolUploadFlags::new(3),
            AdsSymbolUploadFlags::IS_64BIT_PLATFORM | AdsSymbolUploadFlags::INCLUDES_BASE_TYPES
        );
        assert_eq!(
            AdsSymbolUploadFlags::new(5),
            AdsSymbolUploadFlags::IS_64BIT_PLATFORM
                | AdsSymbolUploadFlags::UTF8_ENCODED_STRING_DATA
        );
        assert_eq!(
            AdsSymbolUploadFlags::new(7),
            AdsSymbolUploadFlags::IS_64BIT_PLATFORM
                | AdsSymbolUploadFlags::UTF8_ENCODED_STRING_DATA
                | AdsSymbolUploadFlags::INCLUDES_BASE_TYPES
        );
    }

    #[test]
    fn parses_flags_from_raw() {
        let flag = AdsSymbolUploadFlags::new(UTF8_64_BIT_PLATFORM_DATA);
        assert!(flag.is_64bit_platform());
        assert!(!flag.includes_base_types());
        assert!(flag.is_utf8_encoded());
    }

    #[test]
    fn parses_flags_from_bytes() {
        let flag = AdsSymbolUploadFlags::from_bytes([0x05, 0x00, 0x00, 0x00]);
        assert!(flag.is_64bit_platform());
    }

    #[test]
    fn roundtrip_bytes() {
        let flags = AdsSymbolUploadFlags::new(INCLUDES_BASE_TYPES_DATA_AND_64_BIT);
        assert_eq!(AdsSymbolUploadFlags::from_bytes(flags.to_bytes()), flags);
    }

    #[test]
    fn display_shows_active_flags() {
        let flags = AdsSymbolUploadFlags::new(INCLUDES_BASE_TYPES_DATA_AND_64_BIT);
        let s = flags.to_string();
        assert!(s.contains("IS_64BIT_PLATFORM"));
        assert!(s.contains("INCLUDES_BASE_TYPES"));
    }

    #[test]
    fn zero_displays_none() {
        let flags = AdsSymbolUploadFlags::new(0);
        assert_eq!(flags.to_string(), "None");
    }
}
