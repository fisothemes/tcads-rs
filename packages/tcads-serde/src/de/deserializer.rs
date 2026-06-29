use crate::TypeProvider;
use serde::de::{Deserializer, Visitor};
use tcads_core::{AdsTypeCategory, AdsTypeId, AdsTypeInfo};

pub struct AdsDeserializer<'de, P: TypeProvider> {
    input: &'de [u8],
    type_info: &'de AdsTypeInfo,
    provider: &'de P,
}

impl<'de, P: TypeProvider> AdsDeserializer<'de, P> {
    pub fn new(input: &'de [u8], type_info: &'de AdsTypeInfo, provider: &'de P) -> Self {
        Self {
            input,
            type_info,
            provider,
        }
    }

    pub fn input(&self) -> &[u8] {
        self.input
    }

    pub fn type_info(&self) -> &AdsTypeInfo {
        self.type_info
    }

    pub fn provider(&self) -> &P {
        self.provider
    }

    pub fn validate_type_id(
        type_info: &AdsTypeInfo,
        expected: AdsTypeId,
    ) -> Result<(), crate::Error> {
        if type_info.type_id() != expected {
            return Err(crate::Error::TypeMismatch {
                expected: type_info.name().into(),
            });
        }
        Ok(())
    }

    pub fn extract_bytes<const N: usize>(data: impl AsRef<[u8]>) -> Result<[u8; N], crate::Error> {
        data.as_ref()
            .try_into()
            .map_err(|_| crate::Error::SizeMismatch {
                expected: N,
                got: data.as_ref().len(),
            })
    }
}

impl<'de, P: TypeProvider> Deserializer<'de> for AdsDeserializer<'de, P> {
    type Error = crate::Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.type_info.type_id() {
            AdsTypeId::Bit => self.deserialize_bool(visitor),
            AdsTypeId::Int8 => self.deserialize_i8(visitor),
            AdsTypeId::Int16 => self.deserialize_i16(visitor),
            AdsTypeId::Int32 => self.deserialize_i32(visitor),
            AdsTypeId::Int64 => self.deserialize_i64(visitor),
            AdsTypeId::UInt8 => self.deserialize_u8(visitor),
            AdsTypeId::UInt16 => self.deserialize_u16(visitor),
            AdsTypeId::UInt32 => self.deserialize_u32(visitor),
            AdsTypeId::UInt64 => self.deserialize_u64(visitor),
            AdsTypeId::Real32 => self.deserialize_f32(visitor),
            AdsTypeId::Real64 => self.deserialize_f64(visitor),
            AdsTypeId::String => self.deserialize_string(visitor),
            AdsTypeId::WString => self.deserialize_string(visitor),
            _ => {
                match AdsTypeCategory::determine(
                    &self.type_info.clone(),
                    self.provider.get_platform_ptr_size(),
                ) {
                    AdsTypeCategory::Enum => todo!(),
                    AdsTypeCategory::Struct => todo!(),
                    AdsTypeCategory::Union => todo!(),
                    AdsTypeCategory::FunctionBlock => todo!(),
                    AdsTypeCategory::Interface | AdsTypeCategory::Pointer => todo!(),
                    AdsTypeCategory::Array => todo!(),
                    AdsTypeCategory::Alias => todo!(),
                    _ => todo!(),
                }
            }
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Self::validate_type_id(self.type_info, AdsTypeId::Bit)?;
        let byte = Self::extract_bytes::<{ size_of::<bool>() }>(self.input)?[0];

        visitor.visit_bool(byte != 0)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Self::validate_type_id(self.type_info, AdsTypeId::Int8)?;
        let bytes = Self::extract_bytes::<{ size_of::<i8>() }>(self.input)?;

        visitor.visit_i8(i8::from_le_bytes(bytes))
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Self::validate_type_id(self.type_info, AdsTypeId::Int16)?;
        let bytes = Self::extract_bytes::<{ size_of::<i16>() }>(self.input)?;

        visitor.visit_i16(i16::from_le_bytes(bytes))
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Self::validate_type_id(self.type_info, AdsTypeId::Int32)?;
        let bytes = Self::extract_bytes::<{ size_of::<i32>() }>(self.input)?;

        visitor.visit_i32(i32::from_le_bytes(bytes))
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Self::validate_type_id(self.type_info, AdsTypeId::Int64)?;
        let bytes = Self::extract_bytes::<{ size_of::<i64>() }>(self.input)?;

        visitor.visit_i64(i64::from_le_bytes(bytes))
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Self::validate_type_id(self.type_info, AdsTypeId::UInt8)?;
        let bytes = Self::extract_bytes::<{ size_of::<u8>() }>(self.input)?;

        visitor.visit_u8(u8::from_le_bytes(bytes))
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Self::validate_type_id(self.type_info, AdsTypeId::UInt16)?;
        let bytes = Self::extract_bytes::<{ size_of::<u16>() }>(self.input)?;

        visitor.visit_u16(u16::from_le_bytes(bytes))
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Self::validate_type_id(self.type_info, AdsTypeId::UInt32)?;
        let bytes = Self::extract_bytes::<{ size_of::<u32>() }>(self.input)?;

        visitor.visit_u32(u32::from_le_bytes(bytes))
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Self::validate_type_id(self.type_info, AdsTypeId::UInt64)?;
        let bytes = Self::extract_bytes::<{ size_of::<u64>() }>(self.input)?;

        visitor.visit_u64(u64::from_le_bytes(bytes))
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Self::validate_type_id(self.type_info, AdsTypeId::Real32)?;
        let bytes = Self::extract_bytes::<{ size_of::<f32>() }>(self.input)?;

        visitor.visit_f32(f32::from_le_bytes(bytes))
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Self::validate_type_id(self.type_info, AdsTypeId::Real64)?;
        let bytes = Self::extract_bytes::<{ size_of::<f64>() }>(self.input)?;

        visitor.visit_f64(f64::from_le_bytes(bytes))
    }

    fn deserialize_char<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_str<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_string<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_option<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_unit<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_seq<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_tuple<V>(self, _len: usize, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_map<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_identifier<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_ignored_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }
}
