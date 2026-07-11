use crate::TypeProvider;
use crate::resolvers::resolve_alias;
use crate::ser::AdsSerializer;
use tcads_core::AdsFieldInfo;

pub(super) fn write_field_bytes<T, P>(
    output: &mut [u8],
    field: &AdsFieldInfo,
    provider: &P,
    value: &T,
) -> Result<(), crate::Error>
where
    T: ?Sized + serde::Serialize,
    P: TypeProvider,
{
    let start = field.offset() as usize;
    let end = start + field.size() as usize;
    let len = output.len();
    let field_output = output
        .get_mut(start..end)
        .ok_or(crate::Error::SizeMismatch {
            expected: end,
            got: len,
        })?;

    let field_type_name = field.type_name();
    let ptr_size = provider.get_platform_ptr_size();
    let raw_type_info = provider
        .get_type_info(field_type_name)
        .ok_or_else(|| crate::Error::TypeNotFound(field_type_name.to_string()))?;
    let field_type_info = resolve_alias(raw_type_info, provider, ptr_size)?;

    value.serialize(AdsSerializer::new(field_output, field_type_info, provider))
}
