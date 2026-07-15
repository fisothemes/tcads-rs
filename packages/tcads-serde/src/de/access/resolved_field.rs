use crate::TypeProvider;
use crate::resolvers::resolve_alias;
use tcads_core::{AdsFieldInfo, AdsTypeInfo};

/// A struct/union field whose type has already been resolved (aliases unwound)
/// via the [`TypeProvider`], carrying its byte offset and size within the
/// struct.
///
/// Resolving a field is a `TypeProvider` lookup by name plus alias unwinding.
/// A struct's field types are fixed for the type, identical across every
/// element of an array of that struct, only the underlying bytes differ per
/// element. Without hoisting resolution out of the per-element decode loop,
/// an array of N structs with F fields each repeats that lookup N*F times
/// for data that resolves to the same answer every time. [`resolve_fields`]
/// does it once; [`AdsArrayAccess`](super::array_access::AdsArrayAccess) reuses
/// the result across every element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedField<'a> {
    offset: u32,
    size: u32,
    type_info: &'a AdsTypeInfo,
}

impl<'a> ResolvedField<'a> {
    /// Creates a new instance of [`ResolvedField`].
    pub fn new(offset: u32, size: u32, type_info: &'a AdsTypeInfo) -> Self {
        Self {
            offset,
            size,
            type_info,
        }
    }

    /// Returns the byte offset of this field within the struct.
    pub fn offset(&self) -> u32 {
        self.offset
    }

    /// Returns the size of this field in bytes.
    pub fn size(&self) -> u32 {
        self.size
    }

    /// Returns the resolved type info for this field.
    pub fn type_info(&self) -> &'a AdsTypeInfo {
        self.type_info
    }
}

/// Resolves every field of a struct/union once, for reuse across repeated
/// decodes of the same type.
pub fn resolve_fields<'a>(
    fields: &'a [AdsFieldInfo],
    provider: &'a impl TypeProvider,
) -> Result<Vec<ResolvedField<'a>>, crate::Error> {
    let ptr_size = provider.get_platform_ptr_size();
    fields
        .iter()
        .map(|field| {
            let type_name = field.type_name();
            let raw = provider
                .get_type_info(type_name)
                .ok_or_else(|| crate::Error::TypeNotFound(type_name.to_string()))?;
            let type_info = resolve_alias(raw, provider, ptr_size)?;
            Ok(ResolvedField::new(field.offset(), field.size(), type_info))
        })
        .collect()
}
