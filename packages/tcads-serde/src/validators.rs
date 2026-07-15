use tcads_core::{AdsTypeCategory, AdsTypeId, AdsTypeInfo};

/// Validates that a PLC type strictly matches an expected primitive Type ID.
///
/// # Errors
///
/// - Returns [`crate::Error::TypeMismatch`] if the PLC's reported [`AdsTypeId`]
///   does not match the `expected` ID.
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

/// Validates an integer type, allowing `Pointer`, `Reference`, and `Interface` types
/// to be seamlessly mapped to integers of the target platform's pointer size.
///
/// # Errors
///
/// - Returns [`crate::Error::SizeMismatch`] if a pointer/interface type is evaluated,
///   but its platform size does not match the requested integer byte width `N`.
///
/// - Returns [`crate::Error::TypeMismatch`] if the type is a standard primitive
///   but does not match the `expected` ID.
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

/// Validates that a PLC type falls into one of the expected overarching categories.
///
/// This is used to route dynamic Serde parsing (e.g. ensuring a sequence is actually
/// backed by an `Array` or a `Struct`).
///
/// # Errors
///
/// - Returns [`crate::Error::TypeMismatch`] if the evaluated `AdsTypeCategory`
///   is not contained within the `expected` array.
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

/// Validates that a provided byte slice exactly matches the expected memory footprint.
///
/// This strict 1:1 boundary check prevents malformed network packets or shifting
/// struct alignments from silently causing out-of-bounds memory panics.
///
/// # Errors
///
/// - Returns [`crate::Error::SizeMismatch`] if `data.len()` does not equal `expected`.
pub fn validate_exact_size(data: &[u8], expected: usize) -> Result<(), crate::Error> {
    if data.len() != expected {
        return Err(crate::Error::SizeMismatch {
            expected,
            got: data.len(),
        });
    }
    Ok(())
}
