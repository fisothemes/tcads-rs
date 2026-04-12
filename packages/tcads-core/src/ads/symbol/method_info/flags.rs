use super::error::AdsTypeInfoError;
use bitflags::bitflags;
use std::fmt;

bitflags! {
    /// Flags describing the properties of an ADS RPC Method.
    ///
    /// # Wire Format
    /// | Offset | Size | Field   | Description      |
    /// |--------|------|---------|------------------|
    /// | 0      | 4    | `flags` | Bitmask (LE u32) |
    #[derive(
        serde::Serialize, serde::Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default,
    )]
    #[repr(transparent)]
    pub struct AdsMethodFlags: u32 {
        const PLC_CALLING_CONVENTION = 0x0000_0001;
        const CALL_UNLOCKED = 0x00000002;
        const NOT_CALLABLE = 0x00000004;
        const ATTRIBUTES = 0x00000008;
    }
}

impl AdsMethodFlags {
    /// Wire size in bytes.
    pub const LENGTH: usize = 4;

    /// Creates a new [`AdsMethodFlags`] from a raw u32 value.
    pub const fn new(val: u32) -> Self {
        Self::from_bits_retain(val)
    }

    /// Creates [`AdsMethodFlags`] from little-endian bytes.
    pub const fn from_bytes(bytes: [u8; Self::LENGTH]) -> Self {
        Self::from_bits_retain(u32::from_le_bytes(bytes))
    }

    /// Converts [`AdsMethodFlags`] to little-endian bytes.
    pub const fn to_bytes(&self) -> [u8; Self::LENGTH] {
        self.bits().to_le_bytes()
    }

    /// Returns the raw underlying u32 value.
    pub const fn as_raw(self) -> u32 {
        self.bits()
    }

    pub const fn is_plc_calling_convention(self) -> bool {
        self.contains(Self::PLC_CALLING_CONVENTION)
    }
    pub const fn is_call_unlocked(self) -> bool {
        self.contains(Self::CALL_UNLOCKED)
    }
    pub const fn is_not_callable(self) -> bool {
        self.contains(Self::NOT_CALLABLE)
    }
    pub const fn has_attributes(self) -> bool {
        self.contains(Self::ATTRIBUTES)
    }
}

impl From<u32> for AdsMethodFlags {
    fn from(val: u32) -> Self {
        Self::new(val)
    }
}

impl From<AdsMethodFlags> for u32 {
    fn from(val: AdsMethodFlags) -> Self {
        val.bits()
    }
}

impl From<[u8; Self::LENGTH]> for AdsMethodFlags {
    fn from(value: [u8; Self::LENGTH]) -> Self {
        Self::from_bytes(value)
    }
}

impl From<AdsMethodFlags> for [u8; AdsMethodFlags::LENGTH] {
    fn from(value: AdsMethodFlags) -> Self {
        value.to_bytes()
    }
}

impl TryFrom<&[u8]> for AdsMethodFlags {
    type Error = AdsTypeInfoError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() != Self::LENGTH {
            return Err(AdsTypeInfoError::EntryLengthMismatch {
                expected: Self::LENGTH,
                got: value.len(),
            });
        }
        Ok(Self::from_bytes([value[0], value[1], value[2], value[3]]))
    }
}

impl fmt::Display for AdsMethodFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            f.write_str("None")
        } else {
            bitflags::parser::to_writer(self, f)
        }
    }
}

impl fmt::Debug for AdsMethodFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("AdsMethodFlags")
            .field(&format_args!("{:#010x}", self.bits()))
            .field(&format_args!("{}", self))
            .finish()
    }
}

bitflags! {
    /// Flags describing the properties of an ADS RPC Method Parameter.
    ///
    /// # Wire Format
    /// | Offset | Size | Field   | Description      |
    /// |--------|------|---------|------------------|
    /// | 0      | 4    | `flags` | Bitmask (LE u32) |
    #[derive(
        serde::Serialize, serde::Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default,
    )]
    #[repr(transparent)]
    pub struct AdsMethodParamFlags: u32 {
        const IN = 0x00000001;
        const OUT = 0x00000002;
        const BY_REF = 0x00000004;
        const ATTRIBUTES = 0x00000008;
    }
}

impl AdsMethodParamFlags {
    /// Wire size in bytes.
    pub const LENGTH: usize = 4;

    /// Creates a new [`AdsMethodParamFlags`] from a raw u32 value.
    pub const fn new(val: u32) -> Self {
        Self::from_bits_retain(val)
    }

    /// Creates [`AdsMethodParamFlags`] from little-endian bytes.
    pub const fn from_bytes(bytes: [u8; Self::LENGTH]) -> Self {
        Self::from_bits_retain(u32::from_le_bytes(bytes))
    }

    /// Converts [`AdsMethodParamFlags`] to little-endian bytes.
    pub const fn to_bytes(&self) -> [u8; Self::LENGTH] {
        self.bits().to_le_bytes()
    }

    /// Returns the raw underlying u32 value.
    pub const fn as_raw(self) -> u32 {
        self.bits()
    }

    /// Returns `true` if the method parameter is an input parameter.
    pub const fn is_input(self) -> bool {
        self.contains(Self::IN)
    }

    /// Returns `true` if the method parameter is an output parameter.
    pub const fn is_output(self) -> bool {
        self.contains(Self::OUT)
    }

    /// Returns `true` if the method parameter is a reference.
    pub const fn is_reference(self) -> bool {
        self.contains(Self::BY_REF)
    }

    /// Returns `true` if the method parameter has attributes.
    pub const fn has_attributes(self) -> bool {
        self.contains(Self::ATTRIBUTES)
    }
}

impl From<u32> for AdsMethodParamFlags {
    fn from(value: u32) -> Self {
        Self::from_bits_retain(value)
    }
}

impl From<AdsMethodParamFlags> for u32 {
    fn from(value: AdsMethodParamFlags) -> Self {
        value.bits()
    }
}

impl From<[u8; AdsMethodParamFlags::LENGTH]> for AdsMethodParamFlags {
    fn from(bytes: [u8; AdsMethodParamFlags::LENGTH]) -> Self {
        Self::from_bytes(bytes)
    }
}

impl From<AdsMethodParamFlags> for [u8; AdsMethodParamFlags::LENGTH] {
    fn from(value: AdsMethodParamFlags) -> Self {
        value.to_bytes()
    }
}

impl TryFrom<&[u8]> for AdsMethodParamFlags {
    type Error = AdsTypeInfoError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() != Self::LENGTH {
            return Err(AdsTypeInfoError::EntryLengthMismatch {
                expected: Self::LENGTH,
                got: value.len(),
            });
        }
        Ok(Self::from_bytes([value[0], value[1], value[2], value[3]]))
    }
}

impl fmt::Display for AdsMethodParamFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            f.write_str("None")
        } else {
            bitflags::parser::to_writer(self, f)
        }
    }
}

impl fmt::Debug for AdsMethodParamFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("AdsMethodParamFlags")
            .field(&format_args!("{:#010x}", self.bits()))
            .field(&format_args!("{}", self))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ads_method_flags_basics() {
        let flags = AdsMethodFlags::new(0x0000_0005); // PLC_CALLING_CONVENTION | NOT_CALLABLE

        assert!(flags.is_plc_calling_convention());
        assert!(flags.is_not_callable());
        assert!(!flags.is_call_unlocked());
        assert!(!flags.has_attributes());
    }

    #[test]
    fn ads_method_flags_multiple_and_unknown() {
        let flags = AdsMethodFlags::new(0x0000_001B); // PLC + CALL_UNLOCKED + ATTRIBUTES + unknown bit 0x10

        assert!(flags.is_plc_calling_convention());
        assert!(flags.is_call_unlocked());
        assert!(flags.has_attributes());
        assert!(!flags.is_not_callable());
    }

    #[test]
    fn ads_method_flags_roundtrip_bytes() {
        let original = AdsMethodFlags::new(0x0000_000F); // all known flags
        let roundtripped = AdsMethodFlags::from_bytes(original.to_bytes());
        assert_eq!(roundtripped, original);
    }

    #[test]
    fn ads_method_flags_display() {
        let flags = AdsMethodFlags::new(0x0000_0003); // PLC_CALLING_CONVENTION | CALL_UNLOCKED

        let s = flags.to_string();
        assert!(s.contains("PLC_CALLING_CONVENTION"));
        assert!(s.contains("CALL_UNLOCKED"));
        assert!(!s.contains("NOT_CALLABLE"));
        assert!(!s.contains("ATTRIBUTES"));
    }

    #[test]
    fn ads_method_flags_zero_is_none() {
        assert_eq!(AdsMethodFlags::default().to_string(), "None");
    }

    #[test]
    fn ads_method_flags_debug_format() {
        let flags = AdsMethodFlags::new(0x0000_0005);
        let debug_str = format!("{:?}", flags);

        assert!(debug_str.contains("AdsMethodFlags"));
        assert!(debug_str.contains("0x00000005"));
        assert!(debug_str.contains("PLC_CALLING_CONVENTION"));
        assert!(debug_str.contains("NOT_CALLABLE"));
    }

    #[test]
    fn ads_method_param_flags_basics() {
        let flags = AdsMethodParamFlags::new(0x0000_0005); // IN | BY_REF

        assert!(flags.is_input());
        assert!(flags.is_reference());
        assert!(!flags.is_output());
        assert!(!flags.has_attributes());
    }

    #[test]
    fn ads_method_param_flags_roundtrip_bytes() {
        let original = AdsMethodParamFlags::new(0x0000_000F); // all known flags
        assert_eq!(
            AdsMethodParamFlags::from_bytes(original.to_bytes()),
            original
        );
    }

    #[test]
    fn ads_method_param_flags_display() {
        let flags = AdsMethodParamFlags::new(0x0000_0003); // IN | OUT

        let s = flags.to_string();
        assert!(s.contains("IN"));
        assert!(s.contains("OUT"));
        assert!(!s.contains("BY_REF"));
    }

    #[test]
    fn ads_method_param_flags_zero_is_none() {
        assert_eq!(AdsMethodParamFlags::default().to_string(), "None");
    }

    #[test]
    fn ads_method_param_flags_bitor_works() {
        let combined = AdsMethodParamFlags::IN | AdsMethodParamFlags::OUT;

        assert!(combined.is_input());
        assert!(combined.is_output());
        assert_eq!(combined.as_raw(), 0x0000_0003);
    }
}
