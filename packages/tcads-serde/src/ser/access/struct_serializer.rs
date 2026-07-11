use super::field::write_field_bytes;
use crate::TypeProvider;
use serde::ser::{SerializeStruct, SerializeTuple, SerializeTupleStruct};
use tcads_core::AdsFieldInfo;

/// Writes struct fields in declaration order rather than by name.
pub struct AdsStructSerializer<'ser, P: TypeProvider> {
    fields: std::slice::Iter<'ser, AdsFieldInfo>,
    output: &'ser mut [u8],
    provider: &'ser P,
}

impl<'ser, P: TypeProvider> AdsStructSerializer<'ser, P> {
    /// Creates a new instance of the [`AdsStructSerializer`].
    pub fn new(fields: &'ser [AdsFieldInfo], output: &'ser mut [u8], provider: &'ser P) -> Self {
        Self {
            fields: fields.iter(),
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
            .next()
            .ok_or_else(|| crate::Error::Custom("too many fields for this PLC type".into()))?;

        write_field_bytes(self.output, field, self.provider, value)
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
