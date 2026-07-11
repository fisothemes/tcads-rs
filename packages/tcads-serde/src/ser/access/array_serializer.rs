use super::unsupported_serialize_methods;
use crate::TypeProvider;
use crate::resolvers::resolve_alias;
use crate::ser::AdsSerializer;
use serde::ser::{Impossible, SerializeSeq, SerializeTuple, SerializeTupleStruct};
use tcads_core::{AdsArrayInfo, AdsTypeInfo};

/// Writes elements into a fixed-stride array/tuple slot of the output buffer.
pub struct AdsArraySerializer<'ser, P: TypeProvider> {
    remaining_dims: &'ser [AdsArrayInfo],
    element_type_info: &'ser AdsTypeInfo,
    output: &'ser mut [u8],
    provider: &'ser P,
    index: usize,
    count: usize,
    stride: usize,
}

impl<'ser, P: TypeProvider> AdsArraySerializer<'ser, P> {
    /// Create a new instance of an [`AdsArraySerializer`].
    pub fn new(
        dims: &'ser [AdsArrayInfo],
        element_type_name: &'ser str,
        output: &'ser mut [u8],
        provider: &'ser P,
        len_hint: Option<usize>,
    ) -> Result<Self, crate::Error> {
        let raw_type_info = provider
            .get_type_info(element_type_name)
            .ok_or_else(|| crate::Error::TypeNotFound(element_type_name.to_string()))?;
        let element_type_info =
            resolve_alias(raw_type_info, provider, provider.get_platform_ptr_size())?;

        Self::with_element_type(dims, element_type_info, output, provider, len_hint)
    }

    fn with_element_type(
        dims: &'ser [AdsArrayInfo],
        element_type_info: &'ser AdsTypeInfo,
        output: &'ser mut [u8],
        provider: &'ser P,
        len_hint: Option<usize>,
    ) -> Result<Self, crate::Error> {
        let (dim, remaining_dims) = dims
            .split_first()
            .expect("array type must have at least one dimension");
        let count = dim.element_count() as usize;

        if let Some(len) = len_hint {
            let total: usize = dims.iter().map(|d| d.element_count() as usize).product();
            if len != total {
                return Err(crate::Error::ShapeMismatch {
                    expected: len,
                    got: total,
                });
            }
        }

        let inner: usize = remaining_dims
            .iter()
            .map(|d| d.element_count() as usize)
            .product();
        let stride = element_type_info.size() as usize * inner;

        if stride * count != output.len() {
            return Err(crate::Error::SizeMismatch {
                expected: stride * count,
                got: output.len(),
            });
        }

        Ok(Self {
            remaining_dims,
            element_type_info,
            output,
            provider,
            index: 0,
            count,
            stride,
        })
    }

    fn serialize_next<T>(&mut self, value: &T) -> Result<(), crate::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        if self.index >= self.count {
            return Err(crate::Error::ShapeMismatch {
                expected: self.count,
                got: self.index + 1,
            });
        }

        let start = self.index * self.stride;
        let elem_output = &mut self.output[start..start + self.stride];
        self.index += 1;

        if self.remaining_dims.is_empty() {
            value.serialize(AdsSerializer::new(
                elem_output,
                self.element_type_info,
                self.provider,
            ))
        } else {
            let sub_array = AdsArraySerializer::with_element_type(
                self.remaining_dims,
                self.element_type_info,
                elem_output,
                self.provider,
                None,
            )?;
            value.serialize(sub_array)
        }
    }

    fn finish(self) -> Result<(), crate::Error> {
        if self.index != self.count {
            return Err(crate::Error::ShapeMismatch {
                expected: self.count,
                got: self.index,
            });
        }
        Ok(())
    }

    fn not_a_seq() -> crate::Error {
        crate::Error::TypeMismatch {
            expected: "sequence (nested array dimension)".into(),
        }
    }
}

impl<'ser, P: TypeProvider> SerializeSeq for AdsArraySerializer<'ser, P> {
    type Ok = ();
    type Error = crate::Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        self.serialize_next(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.finish()
    }
}

impl<'ser, P: TypeProvider> SerializeTuple for AdsArraySerializer<'ser, P> {
    type Ok = ();
    type Error = crate::Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        self.serialize_next(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.finish()
    }
}

impl<'ser, P: TypeProvider> SerializeTupleStruct for AdsArraySerializer<'ser, P> {
    type Ok = ();
    type Error = crate::Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        self.serialize_next(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.finish()
    }
}

impl<'ser, P: TypeProvider> serde::Serializer for AdsArraySerializer<'ser, P> {
    type Ok = ();
    type Error = crate::Error;

    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Impossible<(), crate::Error>;
    type SerializeMap = Impossible<(), crate::Error>;
    type SerializeStruct = Impossible<(), crate::Error>;
    type SerializeStructVariant = Impossible<(), crate::Error>;

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(self)
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(self)
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(self)
    }

    unsupported_serialize_methods! {
        Self::not_a_seq =>
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str bytes none some
        unit unit_struct unit_variant newtype_variant tuple_variant map r#struct struct_variant
    }
}
