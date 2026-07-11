use crate::TypeProvider;
use crate::de::AdsDeserializer;
use crate::resolvers::resolve_alias;
use tcads_core::AdsFieldInfo;

pub(super) fn field_deserializer<'de, P>(
    input: &'de [u8],
    field: &AdsFieldInfo,
    provider: &'de P,
) -> Result<AdsDeserializer<'de, P>, crate::Error>
where
    P: TypeProvider,
{
    let start = field.offset() as usize;
    let end = start + field.size() as usize;
    let field_bytes = input.get(start..end).ok_or(crate::Error::SizeMismatch {
        expected: end,
        got: input.len(),
    })?;

    let field_type_name = field.type_name();
    let raw_type_info = provider
        .get_type_info(field_type_name)
        .ok_or_else(|| crate::Error::TypeNotFound(field_type_name.to_string()))?;
    let field_type_info = resolve_alias(raw_type_info, provider, provider.get_platform_ptr_size())?;

    Ok(AdsDeserializer::new(field_bytes, field_type_info, provider))
}
