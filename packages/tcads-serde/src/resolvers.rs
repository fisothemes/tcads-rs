use crate::TypeProvider;
use tcads_core::{AdsTypeCategory, AdsTypeInfo};

const MAX_ALIAS_DEPTH: usize = 32;

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
