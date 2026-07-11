use super::field::field_deserializer;
use crate::TypeProvider;
use serde::de::value::StrDeserializer;
use serde::de::{DeserializeSeed, MapAccess};
use tcads_core::AdsFieldInfo;

/// Yields struct fields as name/value pairs, for dynamically-keyed targets (`Value`,
/// `HashMap<String, _>`) that need to know what each field is called.
pub struct AdsMapAccess<'de, P: TypeProvider> {
    fields: std::slice::Iter<'de, AdsFieldInfo>,
    current: Option<&'de AdsFieldInfo>,
    input: &'de [u8],
    provider: &'de P,
}

impl<'de, P: TypeProvider> AdsMapAccess<'de, P> {
    pub fn new(fields: &'de [AdsFieldInfo], input: &'de [u8], provider: &'de P) -> Self {
        Self {
            fields: fields.iter(),
            current: None,
            input,
            provider,
        }
    }
}

impl<'de, P: TypeProvider> MapAccess<'de> for AdsMapAccess<'de, P> {
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

        seed.deserialize(field_deserializer(self.input, field, self.provider)?)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.fields.len())
    }
}
