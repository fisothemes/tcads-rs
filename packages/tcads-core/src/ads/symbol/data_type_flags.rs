use super::error::AdsError;
use core::ops::{BitAnd, BitOr, BitOrAssign, Not};
use std::fmt;

/// Flags describing the properties and optional wire sections of an [`AdsTypeInfo`](super::AdsTypeInfo) entry.
///
/// This is a bitmask, so multiple flags can be set simultaneously. The flags serve two purposes:
/// describing the nature of the type (e.g. [`PERSISTENT`](Self::PERSISTENT), [`STATIC`](Self::STATIC))
/// and indicating which optional sections are present in the wire format
/// (e.g. [`TYPE_GUID`](Self::TYPE_GUID), [`ATTRIBUTES`](Self::ATTRIBUTES)).
///
/// # Wire Format
/// 4 bytes, Little Endian `u32`.
#[derive(
    serde::Serialize, serde::Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default,
)]
pub struct AdsDataTypeFlags(u32);

impl AdsDataTypeFlags {
    /// Wire size in bytes.
    pub const LENGTH: usize = 4;

    /// Entry is a root data type.
    pub const DATA_TYPE: u32 = 0x00000001;
    /// Entry is a data item (struct field / sub-item).
    pub const DATA_ITEM: u32 = 0x00000002;
    /// Entry contains a reference to another type.
    pub const REFERENCE_TO: u32 = 0x00000004;
    /// Entry is a method dereference.
    pub const METHOD_DEREF: u32 = 0x00000008;
    /// Entry uses oversampling.
    pub const OVERSAMPLE: u32 = 0x00000010;
    /// Size and offset are in bits rather than bytes.
    pub const BIT_VALUES: u32 = 0x00000020;
    /// Entry contains a reference to a function block property.
    pub const PROP_ITEM: u32 = 0x00000040;
    /// A 16-byte GUID follows the sub-items in the wire format.
    /// Confirmed present on every observed entry.
    pub const TYPE_GUID: u32 = 0x00000080;
    /// Entry or one of its fields is persistent across power cycles.
    pub const PERSISTENT: u32 = 0x00000100;
    /// A copy mask follows in the wire format (legacy).
    pub const COPY_MASK: u32 = 0x00000200;
    /// Entry contains or is a TcCOM interface pointer.
    pub const TCOM_INTERFACE_PTR: u32 = 0x00000400;
    /// An RPC method section follows in the wire format.
    pub const METHOD_INFOS: u32 = 0x00000800;
    /// An attribute section follows in the wire format.
    pub const ATTRIBUTES: u32 = 0x00001000;
    /// An enum info section follows in the wire format.
    pub const ENUM_INFOS: u32 = 0x00002000;
    /// Entry is aligned.
    pub const ALIGNED: u32 = 0x00010000;
    /// Data item is static — do not use offsets when accessing.
    pub const STATIC: u32 = 0x00020000;
    /// Entry has software protection levels.
    pub const SOFTWARE_PROTECTION_LEVELS: u32 = 0x00040000;
    /// Persistent data is not restored after a cold reset.
    pub const IGNORE_PERSIST: u32 = 0x00080000;
    /// Any-size array.
    pub const ANY_SIZE_ARRAY: u32 = 0x00100000;
    /// Entry is used for persistent variables.
    pub const PERSISTENT_DATATYPE: u32 = 0x00200000;
    /// Persistent data is not restored after reset (cold).
    pub const INIT_ON_RESET: u32 = 0x00400000;
    /// Entry contains or is a PLC pointer type.
    pub const PLC_POINTER_TYPE: u32 = 0x00800000;
    /// Refactoring information section follows in the wire format.
    pub const REFACTOR_INFO: u32 = 0x01000000;
    /// Sub-items are hidden and will not be evaluated.
    pub const HIDE_SUB_ITEMS: u32 = 0x02000000;
    /// Type description is incomplete.
    pub const INCOMPLETE: u32 = 0x04000000;
    /// Entry contains or is an online change pointer reference.
    pub const CONTAINS_ONLINE_CHANGE_PTR_REF: u32 = 0x08000000;
    /// Entry contains or is a variant / deref type. A deref section follows.
    pub const VARIANT: u32 = 0x10000000;
    /// Extended enum info section follows in the wire format.
    pub const EXTENDED_ENUM_INFOS: u32 = 0x20000000;
    /// Extended flags `u32` follows in the wire format.
    pub const EXTENDED_FLAGS: u32 = 0x80000000;

    /// Creates a new `AdsTypeFlags` from a raw `u32`.
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// Creates from a 4-byte little-endian array.
    pub const fn from_bytes(bytes: [u8; Self::LENGTH]) -> Self {
        Self(u32::from_le_bytes(bytes))
    }

    /// Converts to a 4-byte little-endian array.
    pub fn to_bytes(self) -> [u8; Self::LENGTH] {
        self.0.to_le_bytes()
    }

    /// Returns the raw `u32` value.
    pub const fn as_raw(self) -> u32 {
        self.0
    }

    /// Returns `true` if the [`DATA_TYPE`](Self::DATA_TYPE) flag is set.
    pub fn is_data_type(self) -> bool {
        self.0 & Self::DATA_TYPE != 0
    }
    /// Returns `true` if the [`DATA_ITEM`](Self::DATA_ITEM) flag is set.
    pub fn is_data_item(self) -> bool {
        self.0 & Self::DATA_ITEM != 0
    }
    /// Returns `true` if the [`REFERENCE_TO`](Self::REFERENCE_TO) flag is set.
    pub fn is_reference_to(self) -> bool {
        self.0 & Self::REFERENCE_TO != 0
    }
    /// Returns `true` if the [`BIT_VALUES`](Self::BIT_VALUES) flag is set.
    /// When true, `size` and `offset` are in bits, not bytes.
    pub fn is_bit_values(self) -> bool {
        self.0 & Self::BIT_VALUES != 0
    }
    /// Returns `true` if the [`PROP_ITEM`](Self::PROP_ITEM) flag is set.
    pub fn is_prop_item(self) -> bool {
        self.0 & Self::PROP_ITEM != 0
    }
    /// Returns `true` if a 16-byte GUID section is present in the wire format.
    pub fn has_type_guid(self) -> bool {
        self.0 & Self::TYPE_GUID != 0
    }
    /// Returns `true` if the [`PERSISTENT`](Self::PERSISTENT) flag is set.
    pub fn is_persistent(self) -> bool {
        self.0 & Self::PERSISTENT != 0
    }
    /// Returns `true` if a copy mask section is present (legacy).
    pub fn has_copy_mask(self) -> bool {
        self.0 & Self::COPY_MASK != 0
    }
    /// Returns `true` if an RPC method section is present in the wire format.
    pub fn has_method_infos(self) -> bool {
        self.0 & Self::METHOD_INFOS != 0
    }
    /// Returns `true` if an attribute section is present in the wire format.
    pub fn has_attributes(self) -> bool {
        self.0 & Self::ATTRIBUTES != 0
    }
    /// Returns `true` if an enum info section is present in the wire format.
    pub fn has_enum_infos(self) -> bool {
        self.0 & Self::ENUM_INFOS != 0
    }
    /// Returns `true` if the [`STATIC`](Self::STATIC) flag is set.
    pub fn is_static(self) -> bool {
        self.0 & Self::STATIC != 0
    }
    /// Returns `true` if the [`ANY_SIZE_ARRAY`](Self::ANY_SIZE_ARRAY) flag is set.
    pub fn is_any_size_array(self) -> bool {
        self.0 & Self::ANY_SIZE_ARRAY != 0
    }
    /// Returns `true` if the [`PLC_POINTER_TYPE`](Self::PLC_POINTER_TYPE) flag is set.
    pub fn is_plc_pointer_type(self) -> bool {
        self.0 & Self::PLC_POINTER_TYPE != 0
    }
    /// Returns `true` if a refactor info section is present in the wire format.
    pub fn has_refactor_info(self) -> bool {
        self.0 & Self::REFACTOR_INFO != 0
    }
    /// Returns `true` if sub-items are hidden.
    pub fn hide_sub_items(self) -> bool {
        self.0 & Self::HIDE_SUB_ITEMS != 0
    }
    /// Returns `true` if a deref section is present in the wire format.
    pub fn is_variant(self) -> bool {
        self.0 & Self::VARIANT != 0
    }
    /// Returns `true` if an extended enum info section is present in the wire format.
    pub fn has_extended_enum_infos(self) -> bool {
        self.0 & Self::EXTENDED_ENUM_INFOS != 0
    }
    /// Returns `true` if an extended flags `u32` is present in the wire format.
    pub fn has_extended_flags(self) -> bool {
        self.0 & Self::EXTENDED_FLAGS != 0
    }
    /// Returns `true` if a software protection levels section is present.
    pub fn has_software_protection_levels(self) -> bool {
        self.0 & Self::SOFTWARE_PROTECTION_LEVELS != 0
    }
}

impl From<u32> for AdsDataTypeFlags {
    fn from(val: u32) -> Self {
        Self(val)
    }
}

impl From<AdsDataTypeFlags> for u32 {
    fn from(f: AdsDataTypeFlags) -> Self {
        f.0
    }
}

impl From<[u8; AdsDataTypeFlags::LENGTH]> for AdsDataTypeFlags {
    fn from(bytes: [u8; AdsDataTypeFlags::LENGTH]) -> Self {
        Self::from_bytes(bytes)
    }
}

impl From<AdsDataTypeFlags> for [u8; AdsDataTypeFlags::LENGTH] {
    fn from(f: AdsDataTypeFlags) -> Self {
        f.to_bytes()
    }
}

impl TryFrom<&[u8]> for AdsDataTypeFlags {
    type Error = AdsError;
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() < Self::LENGTH {
            return Err(AdsError::UnexpectedDataLength {
                expected: Self::LENGTH,
                got: value.len(),
            });
        }
        Ok(Self(u32::from_le_bytes(value[..4].try_into().unwrap())))
    }
}

impl BitOr for AdsDataTypeFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for AdsDataTypeFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for AdsDataTypeFlags {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

impl Not for AdsDataTypeFlags {
    type Output = Self;
    fn not(self) -> Self {
        Self(!self.0)
    }
}

impl fmt::Debug for AdsDataTypeFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AdsTypeFlags({:#010X}: {})", self.0, self)
    }
}

impl fmt::Display for AdsDataTypeFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 == 0 {
            return f.write_str("None");
        }
        let mut first = true;
        macro_rules! flag {
            ($check:expr, $name:literal) => {
                if $check {
                    if !first {
                        f.write_str(" | ")?;
                    }
                    f.write_str($name)?;
                    first = false;
                }
            };
        }
        flag!(self.is_data_type(), "DATA_TYPE");
        flag!(self.is_data_item(), "DATA_ITEM");
        flag!(self.is_reference_to(), "REFERENCE_TO");
        flag!(self.0 & Self::METHOD_DEREF != 0, "METHOD_DEREF");
        flag!(self.0 & Self::OVERSAMPLE != 0, "OVERSAMPLE");
        flag!(self.is_bit_values(), "BIT_VALUES");
        flag!(self.is_prop_item(), "PROP_ITEM");
        flag!(self.has_type_guid(), "TYPE_GUID");
        flag!(self.is_persistent(), "PERSISTENT");
        flag!(self.has_copy_mask(), "COPY_MASK");
        flag!(self.0 & Self::TCOM_INTERFACE_PTR != 0, "TCOM_INTERFACE_PTR");
        flag!(self.has_method_infos(), "METHOD_INFOS");
        flag!(self.has_attributes(), "ATTRIBUTES");
        flag!(self.has_enum_infos(), "ENUM_INFOS");
        flag!(self.0 & Self::ALIGNED != 0, "ALIGNED");
        flag!(self.is_static(), "STATIC");
        flag!(
            self.has_software_protection_levels(),
            "SOFTWARE_PROTECTION_LEVELS"
        );
        flag!(self.0 & Self::IGNORE_PERSIST != 0, "IGNORE_PERSIST");
        flag!(self.is_any_size_array(), "ANY_SIZE_ARRAY");
        flag!(
            self.0 & Self::PERSISTENT_DATATYPE != 0,
            "PERSISTENT_DATATYPE"
        );
        flag!(self.0 & Self::INIT_ON_RESET != 0, "INIT_ON_RESET");
        flag!(self.is_plc_pointer_type(), "PLC_POINTER_TYPE");
        flag!(self.has_refactor_info(), "REFACTOR_INFO");
        flag!(self.hide_sub_items(), "HIDE_SUB_ITEMS");
        flag!(self.0 & Self::INCOMPLETE != 0, "INCOMPLETE");
        flag!(
            self.0 & Self::CONTAINS_ONLINE_CHANGE_PTR_REF != 0,
            "CONTAINS_ONLINE_CHANGE_PTR_REF"
        );
        flag!(self.is_variant(), "VARIANT");
        flag!(self.has_extended_enum_infos(), "EXTENDED_ENUM_INFOS");
        flag!(self.has_extended_flags(), "EXTENDED_FLAGS");

        let known = Self::DATA_TYPE
            | Self::DATA_ITEM
            | Self::REFERENCE_TO
            | Self::METHOD_DEREF
            | Self::OVERSAMPLE
            | Self::BIT_VALUES
            | Self::PROP_ITEM
            | Self::TYPE_GUID
            | Self::PERSISTENT
            | Self::COPY_MASK
            | Self::TCOM_INTERFACE_PTR
            | Self::METHOD_INFOS
            | Self::ATTRIBUTES
            | Self::ENUM_INFOS
            | Self::ALIGNED
            | Self::STATIC
            | Self::SOFTWARE_PROTECTION_LEVELS
            | Self::IGNORE_PERSIST
            | Self::ANY_SIZE_ARRAY
            | Self::PERSISTENT_DATATYPE
            | Self::INIT_ON_RESET
            | Self::PLC_POINTER_TYPE
            | Self::REFACTOR_INFO
            | Self::HIDE_SUB_ITEMS
            | Self::INCOMPLETE
            | Self::CONTAINS_ONLINE_CHANGE_PTR_REF
            | Self::VARIANT
            | Self::EXTENDED_ENUM_INFOS
            | Self::EXTENDED_FLAGS;

        if self.0 & !known != 0 {
            if !first {
                f.write_str(" | ")?;
            }
            f.write_str("UNKNOWN")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Confirmed from real wire captures
    const DATA_TYPE_WITH_GUID: u32 = 0x00000081; // UDINT, STRING, ARRAY etc.
    const DATA_TYPE_WITH_GUID_ATTRS: u32 = 0x00001081; // UDINT, LINT (with DisplayMin/Max)
    const DATA_ITEM_WITH_GUID: u32 = 0x00000082; // All sub-items in PlcTaskSystemInfo

    #[test]
    fn parses_data_type_with_guid() {
        let flags = AdsDataTypeFlags::new(DATA_TYPE_WITH_GUID);
        assert!(flags.is_data_type());
        assert!(flags.has_type_guid());
        assert!(!flags.is_data_item());
        assert!(!flags.has_attributes());
    }

    #[test]
    fn parses_data_type_with_guid_and_attrs() {
        let flags = AdsDataTypeFlags::new(DATA_TYPE_WITH_GUID_ATTRS);
        assert!(flags.is_data_type());
        assert!(flags.has_type_guid());
        assert!(flags.has_attributes());
    }

    #[test]
    fn parses_data_item_with_guid() {
        let flags = AdsDataTypeFlags::new(DATA_ITEM_WITH_GUID);
        assert!(flags.is_data_item());
        assert!(flags.has_type_guid());
        assert!(!flags.is_data_type());
    }

    #[test]
    fn roundtrip_bytes() {
        let flags = AdsDataTypeFlags::new(DATA_TYPE_WITH_GUID_ATTRS);
        assert_eq!(AdsDataTypeFlags::from_bytes(flags.to_bytes()), flags);
    }

    #[test]
    fn display_shows_active_flags() {
        let flags = AdsDataTypeFlags::new(DATA_TYPE_WITH_GUID_ATTRS);
        let s = flags.to_string();
        assert!(s.contains("DATA_TYPE"));
        assert!(s.contains("TYPE_GUID"));
        assert!(s.contains("ATTRIBUTES"));
        assert!(!s.contains("DATA_ITEM"));
    }

    #[test]
    fn zero_displays_none() {
        assert_eq!(AdsDataTypeFlags::default().to_string(), "None");
    }

    #[test]
    fn bitor_combines() {
        let a = AdsDataTypeFlags::new(AdsDataTypeFlags::DATA_TYPE);
        let b = AdsDataTypeFlags::new(AdsDataTypeFlags::TYPE_GUID);
        assert_eq!((a | b).as_raw(), DATA_TYPE_WITH_GUID);
    }
}
