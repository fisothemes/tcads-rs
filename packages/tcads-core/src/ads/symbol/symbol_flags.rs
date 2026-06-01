use super::error::AdsSymbolInfoError;
use bitflags::bitflags;
use std::fmt;

bitflags! {
    /// Flags that describe the memory and access properties of a TwinCAT ADS Symbol.
    ///
    /// # Wire Format
    ///
    /// | Offset | Size | Field   | Description      |
    /// |--------|------|---------|------------------|
    /// | 0      | 2    | `flags` | Bitmask (LE u16) |
    #[derive(
        serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord,
        Hash, Default,
    )]
    #[repr(transparent)]
    pub struct AdsSymbolFlags: u16 {
        /// The symbol's value is persistent and retained across runtime restarts.
        const PERSISTENT = 0x0001;
        /// The symbol represents a single bit value (its byte size is 0).
        const BIT_VALUE = 0x0002;
        /// The symbol is a reference to another variable or memory location.
        const REFERENCE_TO = 0x0004;
        /// The symbol metadata includes a 16-byte Type GUID.
        const TYPE_GUID = 0x0008;
        /// The symbol is a TcCOM interface pointer.
        const TCOM_INTERFACE_PTR = 0x0010;
        /// The symbol is read-only and cannot be written to by an ADS client.
        const READ_ONLY = 0x0020;
        /// The symbol provides interface method access.
        const ITF_METHOD_ACCESS = 0x0040;
        /// The symbol requires method dereferencing.
        const METHOD_DEREF = 0x0080;
        /// A 4-bit mask (bits 8-11) indicating the context of the symbol.
        const CONTEXT_MASK = 0x0F00;
        /// The symbol metadata includes key-value attributes/pragmas.
        const ATTRIBUTES = 0x1000;
        /// The symbol is statically allocated.
        const STATIC = 0x2000;
        /// Persistent data will not be restored after a cold reset.
        const INIT_ON_RESET = 0x4000;
        /// The symbol metadata includes a 4-byte extended flags section ([`AdsSymbol2Flags`]).
        const EXTENDED_FLAGS = 0x8000;
    }
}

impl AdsSymbolFlags {
    /// Wire size in bytes.
    pub const LENGTH: usize = 2;

    /// Creates a new [`AdsSymbolFlags`] from a raw `u16`.
    pub const fn new(raw: u16) -> Self {
        Self::from_bits_retain(raw)
    }

    /// Parses the flags from a 2-byte little-endian array, safely retaining any unknown bits.
    pub const fn from_bytes(bytes: [u8; Self::LENGTH]) -> Self {
        Self::from_bits_retain(u16::from_le_bytes(bytes))
    }

    /// Serializes the flags into a 2-byte little-endian array.
    pub const fn to_bytes(self) -> [u8; Self::LENGTH] {
        self.bits().to_le_bytes()
    }

    /// Returns the raw `u16` representation of the flags.
    pub const fn as_raw(&self) -> u16 {
        self.bits()
    }

    /// Returns `true` if the symbol's value is persistent.
    pub const fn is_persistent(&self) -> bool {
        self.contains(Self::PERSISTENT)
    }

    /// Returns `true` if the symbol represents a bit value (byte size is 0).
    pub const fn is_bit_value(&self) -> bool {
        self.contains(Self::BIT_VALUE)
    }

    /// Returns `true` if the symbol is a reference to another variable.
    pub const fn is_reference_to(&self) -> bool {
        self.contains(Self::REFERENCE_TO)
    }

    /// Returns `true` if the symbol metadata contains a Type GUID.
    pub const fn has_type_guid(&self) -> bool {
        self.contains(Self::TYPE_GUID)
    }

    /// Returns `true` if the symbol is a TcCOM interface pointer.
    pub const fn is_tcom_interface_ptr(&self) -> bool {
        self.contains(Self::TCOM_INTERFACE_PTR)
    }

    /// Returns `true` if the symbol is strictly read-only.
    pub const fn is_read_only(&self) -> bool {
        self.contains(Self::READ_ONLY)
    }

    /// Returns `true` if the symbol provides interface method access.
    pub const fn is_itf_method_access(&self) -> bool {
        self.contains(Self::ITF_METHOD_ACCESS)
    }

    /// Returns `true` if the symbol requires method dereferencing.
    pub const fn is_method_deref(&self) -> bool {
        self.contains(Self::METHOD_DEREF)
    }

    /// Returns `true` if the symbol metadata contains attributes/pragmas.
    pub const fn has_attributes(&self) -> bool {
        self.contains(Self::ATTRIBUTES)
    }

    /// Returns `true` if the symbol is statically allocated.
    pub const fn is_static(&self) -> bool {
        self.contains(Self::STATIC)
    }

    /// Returns `true` if persistent data will not be restored on a cold reset.
    pub const fn is_init_on_reset(&self) -> bool {
        self.contains(Self::INIT_ON_RESET)
    }

    /// Returns `true` if the symbol metadata includes extended flags.
    pub const fn has_extended_flags(&self) -> bool {
        self.contains(Self::EXTENDED_FLAGS)
    }

    /// Extracts the 4-bit context mask (bits 8-11).
    pub const fn context_mask(&self) -> u8 {
        ((self.bits() & Self::CONTEXT_MASK.bits()) >> 8) as u8
    }
}

impl From<u16> for AdsSymbolFlags {
    fn from(raw: u16) -> Self {
        Self::from_bits_retain(raw)
    }
}

impl From<AdsSymbolFlags> for u16 {
    fn from(flags: AdsSymbolFlags) -> Self {
        flags.bits()
    }
}

impl From<[u8; AdsSymbolFlags::LENGTH]> for AdsSymbolFlags {
    fn from(bytes: [u8; AdsSymbolFlags::LENGTH]) -> Self {
        Self::from_bytes(bytes)
    }
}

impl From<AdsSymbolFlags> for [u8; AdsSymbolFlags::LENGTH] {
    fn from(flags: AdsSymbolFlags) -> Self {
        flags.to_bytes()
    }
}

impl TryFrom<&[u8]> for AdsSymbolFlags {
    type Error = AdsSymbolInfoError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() != Self::LENGTH {
            return Err(AdsSymbolInfoError::UnexpectedLength {
                expected: Self::LENGTH,
                got: value.len(),
            });
        }

        Ok(Self::from_bytes([value[0], value[1]]))
    }
}

impl fmt::Display for AdsSymbolFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            f.write_str("None")
        } else {
            bitflags::parser::to_writer(self, f)
        }
    }
}

bitflags! {
    /// Extended flags for a TwinCAT ADS Symbol.
    ///
    /// Present in the wire format only if [`AdsSymbolFlags::EXTENDED_FLAGS`] is set.
    ///
    /// # Wire Format
    ///
    /// | Offset | Size | Field   | Description      |
    /// |--------|------|---------|------------------|
    /// | 0      | 4    | `flags` | Bitmask (LE u32) |
    #[derive(
        serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord,
        Hash, Default,
    )]
    #[repr(transparent)]
    pub struct AdsSymbol2Flags: u32 {
        /// A PLC pointer type that requires derelocation/relocation while equalizing redundancy projects.
        const PLC_POINTER_TYPE = 0x00000001;
        /// Ignore this symbol while equalizing redundancy projects.
        const REDUNDANCY_IGNORE = 0x00000002;
        /// The symbol contains refactoring information.
        const REFACTOR_INFO = 0x00000004;
        /// A PLC pointer or reference that the runtime attempts to resolve after an online change.
        const ONLINE_CHANGE_PTR_REF_TYPE = 0x00000008;
        /// The symbol is a Variant Type.
        const VARIANT_TYPE = 0x00000010;
    }
}

impl AdsSymbol2Flags {
    /// Wire size in bytes.
    pub const LENGTH: usize = 4;

    /// Creates a new [`AdsSymbol2Flags`] from a raw `u32`.
    pub const fn new(raw: u32) -> Self {
        Self::from_bits_retain(raw)
    }

    /// Returns the raw `u32` representation of the flags.
    pub const fn as_raw(&self) -> u32 {
        self.bits()
    }

    /// Parses the flags from a 4-byte little-endian array, safely retaining any unknown bits.
    pub const fn from_bytes(bytes: [u8; Self::LENGTH]) -> Self {
        Self::from_bits_retain(u32::from_le_bytes(bytes))
    }

    /// Serializes the flags into a 4-byte little-endian array.
    pub const fn to_bytes(self) -> [u8; Self::LENGTH] {
        self.bits().to_le_bytes()
    }

    /// Returns `true` if the symbol is a PLC pointer type.
    pub const fn is_plc_pointer_type(&self) -> bool {
        self.contains(Self::PLC_POINTER_TYPE)
    }

    /// Returns `true` if the symbol is ignored during redundancy project equalization.
    pub const fn is_redundancy_ignore(&self) -> bool {
        self.contains(Self::REDUNDANCY_IGNORE)
    }

    /// Returns `true` if the symbol metadata contains refactoring history.
    pub const fn has_refactor_info(&self) -> bool {
        self.contains(Self::REFACTOR_INFO)
    }

    /// Returns `true` if the pointer/reference is resolved after an online change.
    pub const fn is_online_change_ptr_ref_type(&self) -> bool {
        self.contains(Self::ONLINE_CHANGE_PTR_REF_TYPE)
    }

    /// Returns `true` if the symbol is a Variant Type.
    pub const fn is_variant_type(&self) -> bool {
        self.contains(Self::VARIANT_TYPE)
    }
}

impl From<u32> for AdsSymbol2Flags {
    fn from(raw: u32) -> Self {
        Self::from_bits_retain(raw)
    }
}

impl From<AdsSymbol2Flags> for u32 {
    fn from(flags: AdsSymbol2Flags) -> Self {
        flags.bits()
    }
}

impl From<[u8; AdsSymbol2Flags::LENGTH]> for AdsSymbol2Flags {
    fn from(bytes: [u8; AdsSymbol2Flags::LENGTH]) -> Self {
        Self::from_bytes(bytes)
    }
}

impl From<AdsSymbol2Flags> for [u8; AdsSymbol2Flags::LENGTH] {
    fn from(flags: AdsSymbol2Flags) -> Self {
        flags.to_bytes()
    }
}

impl TryFrom<&[u8]> for AdsSymbol2Flags {
    type Error = AdsSymbolInfoError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() != Self::LENGTH {
            return Err(AdsSymbolInfoError::UnexpectedLength {
                expected: Self::LENGTH,
                got: value.len(),
            });
        }

        Ok(Self::from_bytes([value[0], value[1], value[2], value[3]]))
    }
}

impl fmt::Display for AdsSymbol2Flags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            f.write_str("None")
        } else {
            bitflags::parser::to_writer(self, f)
        }
    }
}
