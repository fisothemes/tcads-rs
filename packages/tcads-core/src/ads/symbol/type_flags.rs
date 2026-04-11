use super::error::AdsTypeInfoError;
use bitflags::bitflags;
use std::fmt;

bitflags! {
    /// Flags describing the properties and optional wire sections of an [`AdsDataTypeInfo`](super::AdsDataTypeInfo) entry.
    ///
    /// This is a bitmask, so multiple flags can be set simultaneously. The flags serve two purposes:
    /// describing the nature of the type (e.g. [`PERSISTENT`](Self::PERSISTENT), [`STATIC`](Self::STATIC))
    /// and indicating which optional sections are present in the wire format
    /// (e.g. [`TYPE_GUID`](Self::TYPE_GUID), [`ATTRIBUTES`](Self::ATTRIBUTES)).
    ///
    /// # Wire Format
    ///
    /// | Offset | Size | Field   | Description      |
    /// |--------|------|---------|------------------|
    /// | 0      | 4    | `flags` | Bitmask (LE u32) |
    #[derive(
        serde::Serialize, serde::Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default,
    )]
    #[repr(transparent)]
    pub struct AdsTypeFlags: u32 {
        /// Entry is a root data type.
        const DATA_TYPE = 0x00000001;
        /// Entry is a data item (struct field / sub-item).
        const DATA_ITEM = 0x00000002;
        /// Entry contains a reference to another type.
        const REFERENCE_TO = 0x00000004;
        /// Entry is a method dereference.
        const METHOD_DEREF = 0x00000008;
        /// Entry uses oversampling.
        const OVERSAMPLE = 0x00000010;
        /// Size and offset are in bits rather than bytes.
        const BIT_VALUES = 0x00000020;
        /// Entry contains a reference to a function block property.
        const PROP_ITEM = 0x00000040;
        /// A 16-byte GUID follows the sub-items in the wire format.
        /// Confirmed present on every observed entry.
        const TYPE_GUID = 0x00000080;
        /// Entry or one of its fields is persistent across power cycles.
        const PERSISTENT = 0x00000100;
        /// A copy mask follows in the wire format (legacy).
        const COPY_MASK = 0x00000200;
        /// Entry contains or is a TcCOM interface pointer.
        const TCOM_INTERFACE_PTR = 0x00000400;
        /// An RPC method section follows in the wire format.
        const METHOD_INFOS = 0x00000800;
        /// An attribute section follows in the wire format.
        const ATTRIBUTES = 0x00001000;
        /// An enum info section follows in the wire format.
        const ENUM_INFOS = 0x00002000;
        /// Entry is aligned.
        const ALIGNED = 0x00010000;
        /// Data item is static — do not use offsets when accessing.
        const STATIC = 0x00020000;
        /// Entry has software protection levels.
        const SOFTWARE_PROTECTION_LEVELS = 0x00040000;
        /// Persistent data is not restored after a cold reset.
        const IGNORE_PERSIST = 0x00080000;
        /// Any-size array.
        const ANY_SIZE_ARRAY = 0x00100000;
        /// Entry is used for persistent variables.
        const PERSISTENT_DATATYPE = 0x00200000;
        /// Persistent data is not restored after reset (cold).
        const INIT_ON_RESET = 0x00400000;
        /// Entry contains or is a PLC pointer type.
        const PLC_POINTER_TYPE = 0x00800000;
        /// Refactoring information section follows in the wire format.
        const REFACTOR_INFO = 0x01000000;
        /// Sub-items are hidden and will not be evaluated.
        const HIDE_SUB_ITEMS = 0x02000000;
        /// Type description is incomplete.
        const INCOMPLETE = 0x04000000;
        /// Entry contains or is an online change pointer reference.
        const CONTAINS_ONLINE_CHANGE_PTR_REF = 0x08000000;
        /// Entry contains or is a variant / deref type. A deref section follows.
        const VARIANT = 0x10000000;
        /// Extended enum info section follows in the wire format.
        const EXTENDED_ENUM_INFOS = 0x20000000;
        /// Extended flags `u32` follows in the wire format.
        const EXTENDED_FLAGS = 0x80000000;
    }
}

impl AdsTypeFlags {
    /// Wire size in bytes.
    pub const LENGTH: usize = 4;

    /// Creates a new `AdsTypeFlags` from a raw `u32`.
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

    /// Returns the raw `u32` value.
    pub const fn as_raw(self) -> u32 {
        self.bits()
    }

    /// Returns `true` if the [`DATA_TYPE`](Self::DATA_TYPE) flag is set.
    pub const fn is_data_type(self) -> bool {
        self.contains(Self::DATA_TYPE)
    }
    /// Returns `true` if the [`DATA_ITEM`](Self::DATA_ITEM) flag is set.
    pub const fn is_data_item(self) -> bool {
        self.contains(Self::DATA_ITEM)
    }
    /// Returns `true` if the [`REFERENCE_TO`](Self::REFERENCE_TO) flag is set.
    pub const fn is_reference_to(self) -> bool {
        self.contains(Self::REFERENCE_TO)
    }
    /// Returns `true` if the [`BIT_VALUES`](Self::BIT_VALUES) flag is set.
    /// When true, `size` and `offset` are in bits, not bytes.
    pub const fn is_bit_values(self) -> bool {
        self.contains(Self::BIT_VALUES)
    }
    /// Returns `true` if the [`PROP_ITEM`](Self::PROP_ITEM) flag is set.
    pub const fn is_prop_item(self) -> bool {
        self.contains(Self::PROP_ITEM)
    }
    /// Returns `true` if a 16-byte GUID section is present in the wire format.
    pub const fn has_type_guid(self) -> bool {
        self.contains(Self::TYPE_GUID)
    }
    /// Returns `true` if the [`PERSISTENT`](Self::PERSISTENT) flag is set.
    pub const fn is_persistent(self) -> bool {
        self.contains(Self::PERSISTENT)
    }
    /// Returns `true` if a copy mask section is present (legacy).
    pub const fn has_copy_mask(self) -> bool {
        self.contains(Self::COPY_MASK)
    }
    /// Returns `true` if an RPC method section is present in the wire format.
    pub const fn has_method_infos(self) -> bool {
        self.contains(Self::METHOD_INFOS)
    }
    /// Returns `true` if an attribute section is present in the wire format.
    pub const fn has_attributes(self) -> bool {
        self.contains(Self::ATTRIBUTES)
    }
    /// Returns `true` if an enum info section is present in the wire format.
    pub const fn has_enum_infos(self) -> bool {
        self.contains(Self::ENUM_INFOS)
    }
    /// Returns `true` if the [`STATIC`](Self::STATIC) flag is set.
    pub const fn is_static(self) -> bool {
        self.contains(Self::STATIC)
    }
    /// Returns `true` if the [`ANY_SIZE_ARRAY`](Self::ANY_SIZE_ARRAY) flag is set.
    pub const fn is_any_size_array(self) -> bool {
        self.contains(Self::ANY_SIZE_ARRAY)
    }
    /// Returns `true` if the [`PLC_POINTER_TYPE`](Self::PLC_POINTER_TYPE) flag is set.
    pub const fn is_plc_pointer_type(self) -> bool {
        self.contains(Self::PLC_POINTER_TYPE)
    }
    /// Returns `true` if a refactor info section is present in the wire format.
    pub const fn has_refactor_info(self) -> bool {
        self.contains(Self::REFACTOR_INFO)
    }
    /// Returns `true` if sub-items are hidden.
    pub const fn hide_sub_items(self) -> bool {
        self.contains(Self::HIDE_SUB_ITEMS)
    }
    /// Returns `true` if a deref section is present in the wire format.
    pub const fn is_variant(self) -> bool {
        self.contains(Self::VARIANT)
    }
    /// Returns `true` if an extended enum info section is present in the wire format.
    pub const fn has_extended_enum_infos(self) -> bool {
        self.contains(Self::EXTENDED_ENUM_INFOS)
    }
    /// Returns `true` if an extended flags `u32` is present in the wire format.
    pub const fn has_extended_flags(self) -> bool {
        self.contains(Self::EXTENDED_FLAGS)
    }
    /// Returns `true` if a software protection levels section is present.
    pub const fn has_software_protection_levels(self) -> bool {
        self.contains(Self::SOFTWARE_PROTECTION_LEVELS)
    }
}

impl From<u32> for AdsTypeFlags {
    fn from(raw: u32) -> Self {
        Self::from_bits_retain(raw)
    }
}

impl From<AdsTypeFlags> for u32 {
    fn from(flags: AdsTypeFlags) -> Self {
        flags.bits()
    }
}

impl From<[u8; AdsTypeFlags::LENGTH]> for AdsTypeFlags {
    fn from(bytes: [u8; AdsTypeFlags::LENGTH]) -> Self {
        Self::from_bytes(bytes)
    }
}

impl From<AdsTypeFlags> for [u8; AdsTypeFlags::LENGTH] {
    fn from(flags: AdsTypeFlags) -> Self {
        flags.to_bytes()
    }
}

impl TryFrom<&[u8]> for AdsTypeFlags {
    type Error = AdsTypeInfoError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() != Self::LENGTH {
            return Err(AdsTypeInfoError::UnexpectedLength {
                expected: Self::LENGTH,
                got: value.len(),
            });
        }
        Ok(Self::from_bytes([value[0], value[1], value[2], value[3]]))
    }
}

impl fmt::Display for AdsTypeFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            f.write_str("None")
        } else {
            bitflags::parser::to_writer(self, f)
        }
    }
}

impl fmt::Debug for AdsTypeFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple(stringify!(AdsDataTypeFlags))
            .field(&format_args!("{:#010X}", self.bits()))
            .field(&format_args!("{}", self))
            .finish()
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
        let flags = AdsTypeFlags::new(DATA_TYPE_WITH_GUID);
        assert!(flags.is_data_type());
        assert!(flags.has_type_guid());
        assert!(!flags.is_data_item());
        assert!(!flags.has_attributes());
    }

    #[test]
    fn parses_data_type_with_guid_and_attrs() {
        let flags = AdsTypeFlags::new(DATA_TYPE_WITH_GUID_ATTRS);
        assert!(flags.is_data_type());
        assert!(flags.has_type_guid());
        assert!(flags.has_attributes());
    }

    #[test]
    fn parses_data_item_with_guid() {
        let flags = AdsTypeFlags::new(DATA_ITEM_WITH_GUID);
        assert!(flags.is_data_item());
        assert!(flags.has_type_guid());
        assert!(!flags.is_data_type());
    }

    #[test]
    fn roundtrip_bytes() {
        let flags = AdsTypeFlags::new(DATA_TYPE_WITH_GUID_ATTRS);
        assert_eq!(AdsTypeFlags::from_bytes(flags.to_bytes()), flags);
    }

    #[test]
    fn display_shows_active_flags() {
        let flags = AdsTypeFlags::new(DATA_TYPE_WITH_GUID_ATTRS);
        let s = flags.to_string();
        assert!(s.contains("DATA_TYPE"));
        assert!(s.contains("TYPE_GUID"));
        assert!(s.contains("ATTRIBUTES"));
        assert!(!s.contains("DATA_ITEM"));
    }

    #[test]
    fn zero_displays_none() {
        assert_eq!(AdsTypeFlags::default().to_string(), "None");
    }

    #[test]
    fn bitor_combines() {
        let a = AdsTypeFlags::DATA_TYPE;
        let b = AdsTypeFlags::TYPE_GUID;
        assert_eq!((a | b).as_raw(), DATA_TYPE_WITH_GUID);
    }
}
