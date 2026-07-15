use super::resolved_field::{ResolvedField, resolve_fields};
use crate::TypeProvider;
use crate::de::AdsDeserializer;
use crate::resolvers::resolve_alias;
use serde::de::{DeserializeSeed, Deserializer, SeqAccess, Visitor};
use std::rc::Rc;
use tcads_core::{AdsArrayInfo, AdsTypeCategory, AdsTypeInfo};

/// Yields elements from a fixed-stride array slot of the input buffer.
pub struct AdsArrayAccess<'de, P: TypeProvider> {
    remaining_dims: &'de [AdsArrayInfo],
    element_type_info: &'de AdsTypeInfo,
    resolved_fields: Option<Rc<[ResolvedField<'de>]>>,
    input: &'de [u8],
    provider: &'de P,
    index: usize,
    count: usize,
    stride: usize,
}

impl<'de, P: TypeProvider> AdsArrayAccess<'de, P> {
    /// Creates a new instance of an [`AdsArrayAccess`].
    pub fn new(
        dims: &'de [AdsArrayInfo],
        element_type_name: &'de str,
        input: &'de [u8],
        provider: &'de P,
    ) -> Result<Self, crate::Error> {
        let raw_type_info = provider
            .get_type_info(element_type_name)
            .ok_or_else(|| crate::Error::TypeNotFound(element_type_name.to_string()))?;
        let element_type_info =
            resolve_alias(raw_type_info, provider, provider.get_platform_ptr_size())?;

        Self::with_element_type(dims, element_type_info, input, provider)
    }

    fn with_element_type(
        dims: &'de [AdsArrayInfo],
        element_type_info: &'de AdsTypeInfo,
        input: &'de [u8],
        provider: &'de P,
    ) -> Result<Self, crate::Error> {
        let (dim, remaining_dims) = dims
            .split_first()
            .expect("array type must have at least one dimension");
        let count = dim.element_count() as usize;

        let inner: usize = remaining_dims
            .iter()
            .map(|d| d.element_count() as usize)
            .product();
        let stride = element_type_info.size() as usize * inner;

        if stride * count != input.len() {
            return Err(crate::Error::SizeMismatch {
                expected: stride * count,
                got: input.len(),
            });
        }

        let resolved_fields = if remaining_dims.is_empty() {
            match AdsTypeCategory::determine(element_type_info, provider.get_platform_ptr_size()) {
                AdsTypeCategory::Struct
                | AdsTypeCategory::FunctionBlock
                | AdsTypeCategory::Union => Some(Rc::from(resolve_fields(
                    element_type_info.field_infos(),
                    provider,
                )?)),
                _ => None,
            }
        } else {
            None
        };

        Ok(Self {
            remaining_dims,
            element_type_info,
            resolved_fields,
            input,
            provider,
            index: 0,
            count,
            stride,
        })
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
        let elem_bytes = &self.input[start..start + self.stride];
        self.index += 1;

        if self.remaining_dims.is_empty() {
            let deserializer = match &self.resolved_fields {
                // Cheap: a refcount bump, not a re-resolve.
                Some(fields) => AdsDeserializer::with_resolved_fields(
                    elem_bytes,
                    self.element_type_info,
                    self.provider,
                    fields.clone(),
                ),
                None => AdsDeserializer::new(elem_bytes, self.element_type_info, self.provider),
            };
            seed.deserialize(deserializer).map(Some)
        } else {
            let sub_array = AdsArrayAccess::with_element_type(
                self.remaining_dims,
                self.element_type_info,
                elem_bytes,
                self.provider,
            )?;
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
