use crate::TypeProvider;
use crate::de::AdsDeserializer;
use crate::resolvers::resolve_alias;
use serde::de::value::StrDeserializer;
use serde::de::{DeserializeSeed, MapAccess};
use tcads_core::AdsFieldInfo;

pub struct AdsStructAccess<'de, P: TypeProvider> {
    fields: std::slice::Iter<'de, AdsFieldInfo>,
    current: Option<&'de AdsFieldInfo>,
    input: &'de [u8],
    provider: &'de P,
}

impl<'de, P: TypeProvider> AdsStructAccess<'de, P> {
    pub fn new(fields: &'de [AdsFieldInfo], input: &'de [u8], provider: &'de P) -> Self {
        Self {
            fields: fields.iter(),
            current: None,
            input,
            provider,
        }
    }
}

impl<'de, P: TypeProvider> MapAccess<'de> for AdsStructAccess<'de, P> {
    type Error = crate::Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        match self.fields.next() {
            Some(field) => {
                self.current = Some(field);
                seed.deserialize(StrDeserializer::<crate::Error>::new(field.name()))
                    .map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let field = self
            .current
            .take()
            .expect("next_value_seed called before next_key_seed");

        let start = field.offset() as usize;
        let end = start + field.size() as usize;
        let field_bytes = self
            .input
            .get(start..end)
            .ok_or(crate::Error::SizeMismatch {
                expected: end,
                got: self.input.len(),
            })?;

        let field_type_name = field.type_name();
        let ptr_size = self.provider.get_platform_ptr_size();
        let raw_type_info = self
            .provider
            .get_type_info(field_type_name)
            .ok_or_else(|| crate::Error::TypeNotFound(field_type_name.to_string()))?;
        let field_type_info = resolve_alias(raw_type_info, self.provider, ptr_size)?;

        seed.deserialize(AdsDeserializer::new(
            field_bytes,
            field_type_info,
            self.provider,
        ))
    }
}
