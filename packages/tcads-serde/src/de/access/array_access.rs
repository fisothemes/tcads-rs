use crate::TypeProvider;
use crate::de::AdsDeserializer;
use serde::de::{DeserializeSeed, Deserializer, SeqAccess, Visitor};
use tcads_core::AdsArrayInfo;

pub struct AdsArrayAccess<'de, P: TypeProvider> {
    remaining_dims: &'de [AdsArrayInfo],
    element_type_name: &'de str,
    input: &'de [u8],
    provider: &'de P,
    index: usize,
    count: usize,
    stride: usize,
}

impl<'de, P: TypeProvider> AdsArrayAccess<'de, P> {
    pub fn new(
        dims: &'de [AdsArrayInfo],
        element_type_name: &'de str,
        input: &'de [u8],
        provider: &'de P,
    ) -> Self {
        let (dim, remaining_dims) = dims
            .split_first()
            .expect("array type must have at least one dimension");
        let count = dim.element_count() as usize;
        let stride = input.len() / count.max(1);
        Self {
            remaining_dims,
            element_type_name,
            input,
            provider,
            index: 0,
            count,
            stride,
        }
    }
}

impl<'de, P: TypeProvider> SeqAccess<'de> for AdsArrayAccess<'de, P> {
    type Error = crate::Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        if self.index >= self.count {
            return Ok(None);
        }
        let start = self.index * self.stride;
        let end = start + self.stride;
        let elem_bytes = self
            .input
            .get(start..end)
            .ok_or(crate::Error::SizeMismatch {
                expected: end,
                got: self.input.len(),
            })?;
        self.index += 1;

        if self.remaining_dims.is_empty() {
            let ptr_size = self.provider.get_platform_ptr_size();
            let raw_type_info = self
                .provider
                .get_type_info(self.element_type_name)
                .ok_or_else(|| crate::Error::TypeNotFound(self.element_type_name.to_string()))?;
            let elem_type_info =
                AdsDeserializer::resolve_alias(raw_type_info, self.provider, ptr_size)?;
            seed.deserialize(AdsDeserializer::new(
                elem_bytes,
                elem_type_info,
                self.provider,
            ))
            .map(Some)
        } else {
            let sub_array = AdsArrayAccess::new(
                self.remaining_dims,
                self.element_type_name,
                elem_bytes,
                self.provider,
            );
            seed.deserialize(sub_array).map(Some)
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.count - self.index)
    }
}

impl<'de, P: TypeProvider> Deserializer<'de> for AdsArrayAccess<'de, P> {
    type Error = crate::Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_seq(self)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes byte_buf
        option unit unit_struct newtype_struct seq tuple tuple_struct map struct
        enum identifier ignored_any
    }
}
