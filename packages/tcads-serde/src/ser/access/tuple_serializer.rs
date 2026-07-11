use super::{AdsArraySerializer, AdsStructSerializer};
use crate::TypeProvider;
use serde::ser::{SerializeTuple, SerializeTupleStruct};

/// A PLC `ARRAY [..] OF ..` and a PLC `STRUCT`/`FUNCTION_BLOCK` both look like a fixed-length
/// tuple from serde's point of view (both go through `Serializer::serialize_tuple`), but they
/// need different wire logic: array elements share one element type and a fixed stride,
/// struct fields each have their own type and offset. This wraps whichever one applies.
pub enum AdsTupleSerializer<'a, P: TypeProvider> {
    Array(AdsArraySerializer<'a, P>),
    Struct(AdsStructSerializer<'a, P>),
}

impl<'ser, P: TypeProvider> SerializeTuple for AdsTupleSerializer<'ser, P> {
    type Ok = ();
    type Error = crate::Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        match self {
            Self::Array(inner) => inner.serialize_element(value),
            Self::Struct(inner) => inner.serialize_element(value),
        }
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        match self {
            Self::Array(inner) => SerializeTuple::end(inner),
            Self::Struct(inner) => SerializeTuple::end(inner),
        }
    }
}

impl<'ser, P: TypeProvider> SerializeTupleStruct for AdsTupleSerializer<'ser, P> {
    type Ok = ();
    type Error = crate::Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        match self {
            Self::Array(inner) => SerializeTupleStruct::serialize_field(inner, value),
            Self::Struct(inner) => SerializeTupleStruct::serialize_field(inner, value),
        }
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        match self {
            Self::Array(inner) => SerializeTupleStruct::end(inner),
            Self::Struct(inner) => SerializeTupleStruct::end(inner),
        }
    }
}
