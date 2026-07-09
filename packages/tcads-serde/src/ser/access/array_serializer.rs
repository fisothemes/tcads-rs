use crate::TypeProvider;
use crate::resolvers::resolve_alias;
use crate::ser::AdsSerializer;
use serde::ser::{SerializeSeq, SerializeTuple, SerializeTupleStruct};
use tcads_core::AdsArrayInfo;

pub struct AdsArraySerializer<'a, P: TypeProvider> {
    remaining_dims: &'a [AdsArrayInfo],
    element_type_name: &'a str,
    output: &'a mut [u8],
    provider: &'a P,
    index: usize,
    count: usize,
    stride: usize,
}

impl<'a, P: TypeProvider> AdsArraySerializer<'a, P> {
    pub fn new(
        dims: &'a [AdsArrayInfo],
        element_type_name: &'a str,
        output: &'a mut [u8],
        provider: &'a P,
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

        let stride = output.len() / count.max(1);
        Ok(Self {
            remaining_dims,
            element_type_name,
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
        let end = start + self.stride;
        let len = self.output.len();
        let elem_output = self
            .output
            .get_mut(start..end)
            .ok_or(crate::Error::SizeMismatch {
                expected: end,
                got: len,
            })?;
        self.index += 1;

        if self.remaining_dims.is_empty() {
            let ptr_size = self.provider.get_platform_ptr_size();
            let raw_type_info = self
                .provider
                .get_type_info(self.element_type_name)
                .ok_or_else(|| crate::Error::TypeNotFound(self.element_type_name.to_string()))?;
            let elem_type_info = resolve_alias(raw_type_info, self.provider, ptr_size)?;
            value.serialize(AdsSerializer::new(
                elem_output,
                elem_type_info,
                self.provider,
            ))
        } else {
            let sub_array = AdsArraySerializer::new(
                self.remaining_dims,
                self.element_type_name,
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

impl<'a, P: TypeProvider> SerializeSeq for AdsArraySerializer<'a, P> {
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

impl<'a, P: TypeProvider> SerializeTuple for AdsArraySerializer<'a, P> {
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

impl<'a, P: TypeProvider> SerializeTupleStruct for AdsArraySerializer<'a, P> {
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

impl<'a, P: TypeProvider> serde::Serializer for AdsArraySerializer<'a, P> {
    type Ok = ();
    type Error = crate::Error;

    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = serde::ser::Impossible<(), crate::Error>;
    type SerializeMap = serde::ser::Impossible<(), crate::Error>;
    type SerializeStruct = serde::ser::Impossible<(), crate::Error>;
    type SerializeStructVariant = serde::ser::Impossible<(), crate::Error>;

    fn serialize_bool(self, _v: bool) -> Result<Self::Ok, Self::Error> {
        Err(Self::not_a_seq())
    }
    fn serialize_i8(self, _v: i8) -> Result<Self::Ok, Self::Error> {
        Err(Self::not_a_seq())
    }
    fn serialize_i16(self, _v: i16) -> Result<Self::Ok, Self::Error> {
        Err(Self::not_a_seq())
    }

    fn serialize_i32(self, _v: i32) -> Result<Self::Ok, Self::Error> {
        Err(Self::not_a_seq())
    }
    fn serialize_i64(self, _v: i64) -> Result<Self::Ok, Self::Error> {
        Err(Self::not_a_seq())
    }
    fn serialize_u8(self, _v: u8) -> Result<Self::Ok, Self::Error> {
        Err(Self::not_a_seq())
    }
    fn serialize_u16(self, _v: u16) -> Result<Self::Ok, Self::Error> {
        Err(Self::not_a_seq())
    }
    fn serialize_u32(self, _v: u32) -> Result<Self::Ok, Self::Error> {
        Err(Self::not_a_seq())
    }
    fn serialize_u64(self, _v: u64) -> Result<Self::Ok, Self::Error> {
        Err(Self::not_a_seq())
    }
    fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Self::Error> {
        Err(Self::not_a_seq())
    }
    fn serialize_f64(self, _v: f64) -> Result<Self::Ok, Self::Error> {
        Err(Self::not_a_seq())
    }
    fn serialize_char(self, _v: char) -> Result<Self::Ok, Self::Error> {
        Err(Self::not_a_seq())
    }
    fn serialize_str(self, _v: &str) -> Result<Self::Ok, Self::Error> {
        Err(Self::not_a_seq())
    }
    fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> {
        Err(Self::not_a_seq())
    }
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Err(Self::not_a_seq())
    }
    fn serialize_some<T>(self, _value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        Err(Self::not_a_seq())
    }
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Err(Self::not_a_seq())
    }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Err(Self::not_a_seq())
    }
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Err(Self::not_a_seq())
    }
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
    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        Err(Self::not_a_seq())
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
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(Self::not_a_seq())
    }
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Err(Self::not_a_seq())
    }
    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Err(Self::not_a_seq())
    }
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(Self::not_a_seq())
    }
}
