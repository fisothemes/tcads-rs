use super::{AdsTypeId, AdsTypeInfo};

/// Category of a TwinCAT Data Type or Instance.
///
/// This provides a high-level classification of the memory layout, simplifying
/// how consumers interpret the nested fields and properties of a PLC variable.
#[derive(
    serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, Default,
)]
pub enum AdsTypeCategory {
    /// Uninitialized or unknown data type.
    #[default]
    None,
    /// Simple base data type (e.g., `BOOL`, `INT`, `REAL`).
    Primitive,
    /// Type alias pointing to a base type.
    Alias,
    /// Enumeration type.
    Enum,
    /// Array data type.
    Array,
    /// Structure data type.
    Struct,
    /// Function block (POU) instance.
    FunctionBlock,
    /// Program (POU) instance.
    Program,
    /// Function (POU) instance.
    Function,
    /// Sub-range type.
    SubRange,
    /// String type (e.g., `STRING`, `WSTRING`).
    String,
    /// Bitset type.
    Bitset,
    /// Pointer type (`POINTER TO ...`, `PVOID`).
    Pointer,
    /// Union type (overlapping memory fields).
    Union,
    /// Reference type (`REFERENCE TO ...`).
    Reference,
    /// Interface pointer.
    Interface,
}

impl AdsTypeCategory {
    /// Determines the category of a TwinCAT data type from its [`AdsTypeInfo`].
    ///
    /// `platform_ptr_size` is the pointer size in bytes of the target runtime (4 or 8),
    /// used to distinguish interfaces from function blocks when both have methods.
    /// Pass `None` if the platform pointer size is unknown, this may cause some
    /// interfaces to be classified as [`FunctionBlock`](Self::FunctionBlock) instead.
    ///
    /// # Important
    ///
    /// Empty function blocks with no interface will be classified as an interface. Inheritance
    /// plays no part unless the parent has fields or implements an interface.
    pub fn determine(info: &AdsTypeInfo, platform_ptr_size: impl Into<Option<u8>>) -> Self {
        let platform_ptr_size = platform_ptr_size.into();

        // Pointers
        if info.flags().is_plc_pointer_type()
            || info.name() == "PVOID"
            || info.name() == "PCCH"
            || info.name().starts_with("POINTER TO")
        {
            return Self::Pointer;
        }
        // References
        if info.flags().is_reference_to() || info.name().starts_with("REFERENCE TO") {
            return Self::Reference;
        }
        // Arrays
        if !info.array_infos().is_empty() {
            return Self::Array;
        }
        // Enums
        if info.flags().has_enum_infos() {
            return Self::Enum;
        }
        // Sub-ranges
        if info.name().contains("..") && info.name().contains('(') && info.name().contains(')') {
            return Self::SubRange;
        }
        // Alias
        if !info.type_name().is_empty()
            && info.field_infos().is_empty()
            && info.method_infos().is_empty()
        {
            if info.name() == "BOOL" {
                return Self::Primitive;
            }
            return Self::Alias;
        }

        match info.type_id() {
            // Primitives
            AdsTypeId::Int8
            | AdsTypeId::UInt8
            | AdsTypeId::Int16
            | AdsTypeId::UInt16
            | AdsTypeId::Int32
            | AdsTypeId::UInt32
            | AdsTypeId::Int64
            | AdsTypeId::UInt64
            | AdsTypeId::Real32
            | AdsTypeId::Real64
            | AdsTypeId::Real80 => return Self::Primitive,
            AdsTypeId::Bit => return Self::Bitset,
            // Strings
            AdsTypeId::String | AdsTypeId::WString => return Self::String,
            _ => {
                if matches!(
                    info.name(),
                    "TOD"
                        | "DT"
                        | "TIME"
                        | "DATE"
                        | "LTIME"
                        | "TIME_OF_DAY"
                        | "DATE_AND_TIME"
                        | "UXINT"
                        | "XINT"
                        | "XWORD"
                        | "__UXINT"
                        | "__XINT"
                        | "__XWORD"
                ) {
                    return Self::Primitive;
                }
            }
        }

        // Unions (Check for memory offset overlap among field items)
        if !info.field_infos().is_empty()
            && info.type_name().is_empty()
            && info.method_infos().is_empty()
        {
            let fields = info.field_infos();
            if fields.len() > 1 && fields[0].offset() == fields[1].offset() {
                return Self::Union;
            }
        }

        // Complex Types: Interfaces, Function Blocks, and Structs
        if !info.field_infos().is_empty() || !info.method_infos().is_empty() {
            // FBs and Interfaces often expose methods
            if !info.method_infos().is_empty() {
                // Check for Interface implementations
                if info
                    .attributes()
                    .iter()
                    .any(|attr| attr.name() == "TcImplements")
                {
                    return Self::FunctionBlock;
                }
                if info.field_infos().is_empty() {
                    return Self::Interface;
                }
                if platform_ptr_size.is_some_and(|size| info.size() == size as u32) {
                    return Self::Interface;
                }
                return Self::FunctionBlock;
            }

            // If the first user-defined sub-item does NOT start at offset 0, it has
            // hidden state (FB)
            if info
                .field_infos()
                .first()
                .is_some_and(|child| child.offset() > 0)
            {
                return Self::FunctionBlock;
            }

            return Self::Struct;
        }

        // Things are getting funky now...
        if info.field_infos().is_empty() {
            if !info.name().is_empty() && info.size() == 0 {
                return AdsTypeCategory::Struct;
            }

            // Check for Interface implementations
            if info
                .attributes()
                .iter()
                .any(|attr| attr.name() == "TcImplements")
            {
                return AdsTypeCategory::FunctionBlock;
            }

            if info.size().is_multiple_of(4) && info.size() > 8 {
                return AdsTypeCategory::FunctionBlock;
            }

            if info.size() == 4 || info.size() == 8 {
                return AdsTypeCategory::Interface;
            }
        }
        Self::None
    }

    /// Returns `true` if the type is a primitive (e.g. `BOOL`, `INT`, `REAL`).
    pub fn is_primitive(info: &AdsTypeInfo) -> bool {
        Self::determine(info, None) == Self::Primitive
    }

    /// Returns `true` if the type is an alias.
    pub fn is_alias(info: &AdsTypeInfo) -> bool {
        Self::determine(info, None) == Self::Alias
    }

    /// Returns `true` if the type is an enum.
    pub fn is_enum(info: &AdsTypeInfo) -> bool {
        Self::determine(info, None) == Self::Enum
    }

    // Returns `true` if the type is an array.
    pub fn is_array(info: &AdsTypeInfo) -> bool {
        Self::determine(info, None) == Self::Array
    }

    /// Returns `true` if the type is a struct.
    pub fn is_struct(info: &AdsTypeInfo) -> bool {
        Self::determine(info, None) == Self::Struct
    }

    /// Returns `true` if the type is a function block.
    pub fn is_function_block(info: &AdsTypeInfo) -> bool {
        Self::determine(info, None) == Self::FunctionBlock
    }

    /// Returns `true` if the type is a sub-range (e.g. `INT(0..100)`).
    pub fn is_sub_range(info: &AdsTypeInfo) -> bool {
        Self::determine(info, None) == Self::SubRange
    }

    /// Returns `true` if the type is a string (`STRING` or `WSTRING`).
    pub fn is_string(info: &AdsTypeInfo) -> bool {
        Self::determine(info, None) == Self::String
    }

    /// Returns `true` if the type is a bitset (single-bit type)
    pub fn is_bitset(info: &AdsTypeInfo) -> bool {
        Self::determine(info, None) == Self::Bitset
    }

    /// Returns `true` if the type is a pointer (`POINTER TO ...` or `PVOID`).
    pub fn is_pointer(info: &AdsTypeInfo) -> bool {
        Self::determine(info, None) == Self::Pointer
    }

    /// Returns `true` if the type is a union (fields share the same memory offset).
    pub fn is_union(info: &AdsTypeInfo) -> bool {
        Self::determine(info, None) == Self::Union
    }

    /// Returns `true` if the type is a reference (`REFERENCE TO ...`).
    pub fn is_reference(info: &AdsTypeInfo) -> bool {
        Self::determine(info, None) == Self::Reference
    }

    /// Returns `true` if the type is an interface pointer.
    ///
    /// `platform_ptr_size` is required to distinguish interfaces from function blocks,
    /// pass the pointer size in bytes (4 or 8) from
    /// [`AdsSymbolUploadInfo`](super::AdsSymbolUploadInfo).
    ///
    /// # Important
    ///
    /// Empty function blocks with no interface will be classified as an interface. Inheritance
    /// plays no part unless the parent has fields or implements an interface.
    pub fn is_interface(info: &AdsTypeInfo, platform_ptr_size: impl Into<Option<u8>>) -> bool {
        Self::determine(info, platform_ptr_size) == Self::Interface
    }
}
