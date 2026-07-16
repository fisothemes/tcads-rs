use crate::TypeProvider;
use crate::resolvers::ResolvedField;
use crate::ser::AdsSerializer;
use serde::ser::{SerializeStruct, SerializeTuple, SerializeTupleStruct};
use std::rc::Rc;

/// Writes struct fields in declaration order rather than by name.
///
/// Takes pre-[`resolved`](ResolvedField) fields rather than raw `AdsFieldInfo`
/// plus a [`TypeProvider`] to look them up: resolution already happened once,
/// either fresh for a one-off struct encode or hoisted out of the loop by
/// [`AdsArraySerializer`](super::array_serializer::AdsArraySerializer) for a
/// repeated element type. This type never touches the `TypeProvider` itself.
///
/// Held as `Rc<[ResolvedField]>` rather than a borrowed slice for the same
/// reason as [`AdsStructAccess`](crate::de::access::AdsStructAccess) on the
/// deserialize side: when reused across array elements, the fields are owned
/// by `AdsArraySerializer`, which doesn't live as long as the output buffer's
/// own lifetime, an `Rc` clone (a refcount bump) sidesteps that.
pub struct AdsStructSerializer<'ser, P: TypeProvider> {
    fields: Rc<[ResolvedField<'ser>]>,
    index: usize,
    output: &'ser mut [u8],
    provider: &'ser P,
}

impl<'ser, P: TypeProvider> AdsStructSerializer<'ser, P> {
    /// Creates a new instance of the [`AdsStructSerializer`] over already-resolved fields.
    pub fn new(
        fields: Rc<[ResolvedField<'ser>]>,
        output: &'ser mut [u8],
        provider: &'ser P,
    ) -> Self {
        Self {
            fields,
            index: 0,
            output,
            provider,
        }
    }

    fn serialize_next<T>(&mut self, value: &T) -> Result<(), crate::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        let field = self
            .fields
            .get(self.index)
            .cloned()
            .ok_or_else(|| crate::Error::Custom("too many fields for this PLC type".into()))?;
        self.index += 1;

        let start = field.offset() as usize;
        let end = start + field.size() as usize;
        let len = self.output.len();
        let field_output = self
            .output
            .get_mut(start..end)
            .ok_or(crate::Error::SizeMismatch {
                expected: end,
                got: len,
            })?;

        value.serialize(AdsSerializer::new(
            field_output,
            field.type_info(),
            self.provider,
        ))
    }
}

impl<'ser, P: TypeProvider> SerializeTuple for AdsStructSerializer<'ser, P> {
    type Ok = ();
    type Error = crate::Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        self.serialize_next(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'ser, P: TypeProvider> SerializeTupleStruct for AdsStructSerializer<'ser, P> {
    type Ok = ();
    type Error = crate::Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        self.serialize_next(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'ser, P: TypeProvider> SerializeStruct for AdsStructSerializer<'ser, P> {
    type Ok = ();
    type Error = crate::Error;

    fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        self.serialize_next(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}
