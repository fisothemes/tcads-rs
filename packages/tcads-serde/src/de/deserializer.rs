use super::access::{AdsEnumAccess, AdsStructAccess};
use crate::{Integer, TypeProvider};
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

    pub fn validate_integer_type_id<const N: usize>(
        type_info: &AdsTypeInfo,
        expected: AdsTypeId,
        platform_ptr_size: u8,
    ) -> Result<(), crate::Error> {
        if matches!(
            AdsTypeCategory::determine(type_info, platform_ptr_size),
            AdsTypeCategory::Pointer | AdsTypeCategory::Reference
        ) {
            return if platform_ptr_size as usize == N {
                Ok(())
            } else {
                Err(crate::Error::SizeMismatch {
                    expected: platform_ptr_size as usize,
                    got: N,
                })
            };
        }
        Self::validate_type_id(type_info, expected)
    }

    pub fn resolve_alias<'a>(
        type_info: &'a AdsTypeInfo,
        provider: &'a P,
        platform_ptr_size: u8,
    ) -> Result<&'a AdsTypeInfo, crate::Error> {
        let mut type_info = type_info;
        while AdsTypeCategory::determine(type_info, platform_ptr_size) == AdsTypeCategory::Alias {
            type_info = provider
                .get_type_info(type_info.type_name())
                .ok_or_else(|| crate::Error::TypeNotFound(type_info.type_name().to_string()))?;
        }
        Ok(type_info)
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
        let type_info = self.type_info;
        let pointer_size = self.provider.get_platform_ptr_size();

        match AdsTypeCategory::determine(type_info, pointer_size) {
            AdsTypeCategory::Primitive => match type_info.type_id() {
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
                _ => todo!(),
            },
            AdsTypeCategory::Enum => self.deserialize_enum("", &[], visitor),
            AdsTypeCategory::Pointer | AdsTypeCategory::Reference => match pointer_size {
                2 => self.deserialize_u16(visitor),
                4 => self.deserialize_u32(visitor),
                8 => self.deserialize_u64(visitor),
                other => Err(crate::Error::InvalidByteLength(other as usize)),
            },
            AdsTypeCategory::Alias => {
                let target = Self::resolve_alias(type_info, self.provider, pointer_size)?;
                Self::new(self.input, target, self.provider).deserialize_any(visitor)
            }
            AdsTypeCategory::Struct | AdsTypeCategory::FunctionBlock => {
                self.deserialize_struct("", &[], visitor)
            }
            AdsTypeCategory::Union => todo!(),
            AdsTypeCategory::Interface => todo!(),
            AdsTypeCategory::Array => todo!(),
            _ => todo!(),
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
        let bytes = Self::extract_bytes::<1>(self.input)?;

        visitor.visit_i8(i8::from_le_bytes(bytes))
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Self::validate_type_id(self.type_info, AdsTypeId::Int16)?;
        let bytes = Self::extract_bytes::<2>(self.input)?;

        visitor.visit_i16(i16::from_le_bytes(bytes))
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Self::validate_type_id(self.type_info, AdsTypeId::Int32)?;
        let bytes = Self::extract_bytes::<4>(self.input)?;

        visitor.visit_i32(i32::from_le_bytes(bytes))
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Self::validate_type_id(self.type_info, AdsTypeId::Int64)?;
        let bytes = Self::extract_bytes::<8>(self.input)?;

        visitor.visit_i64(i64::from_le_bytes(bytes))
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Self::validate_type_id(self.type_info, AdsTypeId::UInt8)?;
        let bytes = Self::extract_bytes::<1>(self.input)?;

        visitor.visit_u8(u8::from_le_bytes(bytes))
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Self::validate_integer_type_id::<2>(
            self.type_info,
            AdsTypeId::UInt16,
            self.provider.get_platform_ptr_size(),
        )?;
        let bytes = Self::extract_bytes::<2>(self.input)?;

        visitor.visit_u16(u16::from_le_bytes(bytes))
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Self::validate_integer_type_id::<4>(
            self.type_info,
            AdsTypeId::UInt32,
            self.provider.get_platform_ptr_size(),
        )?;
        let bytes = Self::extract_bytes::<4>(self.input)?;

        visitor.visit_u32(u32::from_le_bytes(bytes))
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Self::validate_integer_type_id::<8>(
            self.type_info,
            AdsTypeId::UInt64,
            self.provider.get_platform_ptr_size(),
        )?;
        let bytes = Self::extract_bytes::<8>(self.input)?;

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

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if self.input.len() != self.type_info.size() as usize {
            return Err(crate::Error::SizeMismatch {
                expected: self.type_info.size() as usize,
                got: self.input.len(),
            });
        }

        let decoded_string = match self.type_info.type_id() {
            AdsTypeId::String => {
                let null_pos = self
                    .input
                    .iter()
                    .position(|&b| b == 0)
                    .unwrap_or(self.input.len());
                let str_bytes = &self.input[..null_pos];
                encoding_rs::WINDOWS_1252.decode(str_bytes).0.to_string()
            }
            AdsTypeId::WString => {
                let mut null_pos = self.input.len();

                for (i, chunk) in self.input.chunks_exact(2).enumerate() {
                    if chunk[0] == 0 && chunk[1] == 0 {
                        null_pos = i * 2;
                        break;
                    }
                }

                let str_bytes = &self.input[..null_pos];

                encoding_rs::UTF_16LE.decode(str_bytes).0.to_string()
            }
            _ => {
                return Err(crate::Error::TypeMismatch {
                    expected: "STRING/WSTRING based type.".into(),
                });
            }
        };

        visitor.visit_string(decoded_string)
    }

    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_some(self)
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let ptr_size = self.provider.get_platform_ptr_size();
        let type_info = Self::resolve_alias(self.type_info, self.provider, ptr_size)?;

        visitor.visit_map(AdsStructAccess::new(
            type_info.field_infos(),
            self.input,
            self.provider,
        ))
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
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let ptr_size = self.provider.get_platform_ptr_size();
        let type_info = Self::resolve_alias(self.type_info, self.provider, ptr_size)?;

        let variant_name = type_info
            .enum_infos()
            .iter()
            .find(|e| e.value() == self.input)
            .map(|e| e.name());

        match variant_name {
            Some(name) => visitor.visit_enum(AdsEnumAccess::new(name)),
            None => {
                let discriminant =
                    Integer::try_from_le_slice(self.input, self.type_info.type_id())?;
                Err(crate::Error::UnknownEnumDiscriminant(
                    discriminant,
                    self.type_info.name().to_string(),
                ))
            }
        }
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
