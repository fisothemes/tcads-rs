use crate::TypeProvider;
use tcads_core::{AdsTypeCategory, AdsTypeInfo};

pub fn resolve_alias<'a>(
    type_info: &'a AdsTypeInfo,
    provider: &'a impl TypeProvider,
    platform_ptr_size: u8,
) -> Result<&'a AdsTypeInfo, crate::Error> {
    let mut type_info = type_info;
    while AdsTypeCategory::determine(type_info, platform_ptr_size) == AdsTypeCategory::Alias {
        type_info = provider
            .get_type_info(type_info.type_name())
            .ok_or_else(|| crate::Error::TypeNotFound(type_info.type_name().to_string()))?;
    }
    Ok(type_info)
}
