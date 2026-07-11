use tcads_core::{AdsTypeCategory, AdsTypeId, AdsTypeInfo};

pub fn validate_type_id(type_info: &AdsTypeInfo, expected: AdsTypeId) -> Result<(), crate::Error> {
    if type_info.type_id() != expected {
        return Err(crate::Error::TypeMismatch {
            expected: format!(
                "{expected:?}, but PLC type is '{}' ({:?})",
                type_info.name(),
                type_info.type_id(),
            ),
        });
    }
    Ok(())
}

pub fn validate_integer_type_id<const N: usize>(
    type_info: &AdsTypeInfo,
    expected: AdsTypeId,
    platform_ptr_size: u8,
) -> Result<(), crate::Error> {
    if matches!(
        AdsTypeCategory::determine(type_info, platform_ptr_size),
        AdsTypeCategory::Pointer | AdsTypeCategory::Reference | AdsTypeCategory::Interface
    ) {
        return if platform_ptr_size as usize == N {
            Ok(())
        } else {
            Err(crate::Error::SizeMismatch {
                expected: platform_ptr_size as usize,
                got: N,
            })
        };
    }
    validate_type_id(type_info, expected)
}

pub fn validate_type_category(
    type_info: &AdsTypeInfo,
    expected: &[AdsTypeCategory],
    platform_ptr_size: u8,
) -> Result<(), crate::Error> {
    let category = AdsTypeCategory::determine(type_info, platform_ptr_size);
    if !expected.contains(&category) {
        return Err(crate::Error::TypeMismatch {
            expected: format!(
                "one of {expected:?}, but PLC type is '{}' ({category:?})",
                type_info.name(),
            ),
        });
    }
    Ok(())
}

pub fn validate_exact_size(data: &[u8], expected: usize) -> Result<(), crate::Error> {
    if data.len() != expected {
        return Err(crate::Error::SizeMismatch {
            expected,
            got: data.len(),
        });
    }
    Ok(())
}
