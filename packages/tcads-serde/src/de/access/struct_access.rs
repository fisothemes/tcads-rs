use crate::TypeProvider;
use crate::de::AdsDeserializer;
use crate::resolvers::ResolvedField;
use serde::de::{DeserializeSeed, SeqAccess};
use std::rc::Rc;

/// Yields struct fields sequentially based on their declaration order in memory.
///
/// Takes pre-[`resolved`](ResolvedField) fields rather than raw [`AdsFieldInfo`](tcads_core::AdsFieldInfo)
/// plus a [`TypeProvider`] to look them up: resolution already happened once,
/// either fresh for a one-off struct decode or hoisted out of the loop by
/// [`AdsArrayAccess`](super::array_access::AdsArrayAccess) for a repeated
/// element type. This type never touches the [`TypeProvider`] itself.
///
/// Held as `Rc<[ResolvedField]>` rather than a borrowed slice: when reused
/// across array elements, the fields are owned by [`AdsArrayAccess`](super::array_access::AdsArrayAccess),
/// which doesn't live as long as the input buffer's own lifetime, an `Rc` clone (a refcount bump)
/// sidesteps that without needing an extra lifetime parameter threaded through [`AdsDeserializer`].
pub struct AdsStructAccess<'de, P: TypeProvider> {
    fields: Rc<[ResolvedField<'de>]>,
    index: usize,
    input: &'de [u8],
    provider: &'de P,
}

impl<'de, P: TypeProvider> AdsStructAccess<'de, P> {
    /// Creates a new instance of an [`AdsStructAccess`] over already-resolved fields.
    pub fn new(fields: Rc<[ResolvedField<'de>]>, input: &'de [u8], provider: &'de P) -> Self {
        Self {
            fields,
            index: 0,
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
        let Some(field) = self.fields.get(self.index).cloned() else {
            return Ok(None);
        };
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
        .map(Some)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.fields.len() - self.index)
    }
}
