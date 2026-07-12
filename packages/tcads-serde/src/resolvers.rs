use crate::TypeProvider;
use tcads_core::{AdsTypeCategory, AdsTypeInfo};

// The maximum allowed depth when recursively unwrapping `ALIAS` types.
/// Prevents cyclic alias declarations in the PLC from causing an infinite loop.
pub const MAX_ALIAS_DEPTH: usize = 32;

/// Recursively resolves an IEC 61131-3 `ALIAS` type down to its underlying base memory type.
///
/// If the provided `type_info` is not an alias, it is immediately returned.
/// If it is an alias, this function requests the target types sequentially from the
/// [`TypeProvider`] until a non-alias type (like a Struct, Enum, or Primitive) is reached.
///
/// # Errors
///
/// - Returns [`crate::Error::TypeNotFound`] if the `TypeProvider` is missing a type in the chain.
///
/// - Returns [`crate::Error::Custom`] if the resolution chain exceeds `MAX_ALIAS_DEPTH`,
/// which typically indicates a cyclic type definition in the PLC.
pub fn resolve_alias<'a>(
    type_info: &'a AdsTypeInfo,
    provider: &'a impl TypeProvider,
    platform_ptr_size: u8,
) -> Result<&'a AdsTypeInfo, crate::Error> {
    let mut type_info = type_info;
    for _ in 0..MAX_ALIAS_DEPTH {
        if AdsTypeCategory::determine(type_info, platform_ptr_size) != AdsTypeCategory::Alias {
            return Ok(type_info);
        }
        type_info = provider
            .get_type_info(type_info.type_name())
            .ok_or_else(|| crate::Error::TypeNotFound(type_info.type_name().to_string()))?;
    }
    Err(crate::Error::Custom(format!(
        "alias chain exceeds {MAX_ALIAS_DEPTH} levels resolving '{}': type table likely contains a cycle",
        type_info.name(),
    )))
}
