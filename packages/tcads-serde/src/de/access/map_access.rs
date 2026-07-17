use crate::TypeProvider;
use crate::de::AdsDeserializer;
use crate::resolvers::ResolvedField;
use serde::de::value::StrDeserializer;
use serde::de::{DeserializeSeed, MapAccess};
use std::rc::Rc;

/// Yields struct fields as name/value pairs, for dynamically-keyed targets (`Value`,
/// `HashMap<String, _>`) that need to know what each field is called.
pub struct AdsMapAccess<'de, P: TypeProvider> {
    fields: Rc<[ResolvedField<'de>]>,
    index: usize,
    input: &'de [u8],
    provider: &'de P,
}

impl<'de, P: TypeProvider> AdsMapAccess<'de, P> {
    pub(crate) fn new(
        fields: Rc<[ResolvedField<'de>]>,
        input: &'de [u8],
        provider: &'de P,
    ) -> Self {
        Self {
            fields,
            index: 0,
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
        match self.fields.get(self.index) {
            Some(field) => seed
                .deserialize(StrDeserializer::<crate::Error>::new(field.name()))
                .map(Some),
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let field = self.fields[self.index].clone();
        self.index += 1;

        let start = field.offset() as usize;
        let end = start + field.size() as usize;
        let field_bytes = self
            .input
            .get(start..end)
            .ok_or(crate::Error::SizeMismatch {
                expected: end,
                got: self.input.len(),
            })?;

        seed.deserialize(AdsDeserializer::new(
            field_bytes,
            field.type_info(),
            self.provider,
        ))
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.fields.len() - self.index)
    }
}
