use crate::TypeProvider;
use crate::de::AdsDeserializer;
use crate::resolvers::resolve_alias;
use serde::de::{DeserializeSeed, SeqAccess};
use tcads_core::AdsFieldInfo;

pub struct AdsStructSeqAccess<'de, P: TypeProvider> {
    fields: std::slice::Iter<'de, AdsFieldInfo>,
    input: &'de [u8],
    provider: &'de P,
}

impl<'de, P: TypeProvider> AdsStructSeqAccess<'de, P> {
    pub fn new(fields: &'de [AdsFieldInfo], input: &'de [u8], provider: &'de P) -> Self {
        Self {
            fields: fields.iter(),
            input,
            provider,
        }
    }
}

impl<'de, P: TypeProvider> SeqAccess<'de> for AdsStructSeqAccess<'de, P> {
    type Error = crate::Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        let field = match self.fields.next() {
            Some(f) => f,
            None => return Ok(None),
        };

        let start = field.offset() as usize;
        let end = start + field.size() as usize;
        let field_bytes = self
            .input
            .get(start..end)
            .ok_or(crate::Error::SizeMismatch {
                expected: end,
                got: self.input.len(),
            })?;

        let ptr_size = self.provider.get_platform_ptr_size();
        let raw_type_info = self
            .provider
            .get_type_info(field.type_name())
            .ok_or_else(|| crate::Error::TypeNotFound(field.type_name().to_string()))?;
        let field_type_info = resolve_alias(raw_type_info, self.provider, ptr_size)?;

        seed.deserialize(AdsDeserializer::new(
            field_bytes,
            field_type_info,
            self.provider,
        ))
        .map(Some)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.fields.len())
    }
}
