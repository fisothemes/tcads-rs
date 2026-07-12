use super::field::field_deserializer;
use crate::TypeProvider;
use serde::de::{DeserializeSeed, SeqAccess};
use tcads_core::AdsFieldInfo;

/// Yields struct fields sequentially based on their declaration order in memory.
pub struct AdsStructAccess<'de, P: TypeProvider> {
    fields: std::slice::Iter<'de, AdsFieldInfo>,
    input: &'de [u8],
    provider: &'de P,
}

impl<'de, P: TypeProvider> AdsStructAccess<'de, P> {
    /// Creates a new instance of an [`AdsStructAccess`].
    pub fn new(fields: &'de [AdsFieldInfo], input: &'de [u8], provider: &'de P) -> Self {
        Self {
            fields: fields.iter(),
            input,
            provider,
        }
    }
}

impl<'de, P: TypeProvider> SeqAccess<'de> for AdsStructAccess<'de, P> {
    type Error = crate::Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        let Some(field) = self.fields.next() else {
            return Ok(None);
        };

        seed.deserialize(field_deserializer(self.input, field, self.provider)?)
            .map(Some)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.fields.len())
    }
}
