use super::access::{AdsArrayAccess, AdsEnumAccess, AdsMapAccess, AdsStructAccess};
use crate::resolvers::{ResolvedField, resolve_alias, resolve_fields};
use crate::validators::{
    validate_exact_size, validate_integer_type_id, validate_type_category, validate_type_id,
};
use crate::{Integer, TypeProvider};
use serde::de::{Deserializer, Visitor};
use std::borrow::Cow;
use std::rc::Rc;
use tcads_core::{AdsTypeCategory, AdsTypeId, AdsTypeInfo};

/// Deserializes a PLC memory layout into a Rust type using the [`AdsTypeInfo`].
pub struct AdsDeserializer<'de, P: TypeProvider> {
    input: &'de [u8],
    type_info: &'de AdsTypeInfo,
    provider: &'de P,
    resolved_fields: Option<Rc<[ResolvedField<'de>]>>,
}

impl<'de, P: TypeProvider> AdsDeserializer<'de, P> {
    /// Creates a new instance of the [`AdsDeserializer`].
    pub fn new(input: &'de [u8], type_info: &'de AdsTypeInfo, provider: &'de P) -> Self {
        Self {
            input,
            type_info,
            provider,
            resolved_fields: None,
        }
    }

    /// Same as [`new`](Self::new), but carrying fields the caller already
    /// resolved (see [`AdsArrayAccess`]'s doc comment for why this exists).
    pub fn with_resolved_fields(
        input: &'de [u8],
        type_info: &'de AdsTypeInfo,
        provider: &'de P,
        resolved_fields: Rc<[ResolvedField<'de>]>,
    ) -> Self {
        Self {
            input,
            type_info,
            provider,
            resolved_fields: Some(resolved_fields),
        }
    }

    /// The buffer that was passed to the deserializer, i.e. the memory layout that was obtained
    /// from the PLC.
    pub fn input(&self) -> &[u8] {
        self.input
    }

    /// The [`AdsTypeInfo`] that was passed to the deserializer.
    pub fn type_info(&self) -> &AdsTypeInfo {
        self.type_info
    }

    /// The [`TypeProvider`] that is used to resolve the type further if necessary, e.g. for
    /// aliases, structs, arrays, etc.
    pub fn provider(&self) -> &P {
        self.provider
    }

    /// Reads the exact number of bytes from the input buffer.
    ///
    /// # Type Parameters
    ///
    /// - `N`: The size of the byte array, determined at compile time.
    pub fn read_bytes<const N: usize>(data: impl AsRef<[u8]>) -> Result<[u8; N], crate::Error> {
        data.as_ref()
            .try_into()
            .map_err(|_| crate::Error::SizeMismatch {
                expected: N,
                got: data.as_ref().len(),
            })
    }

    /// Decodes a `STRING`'s raw bytes (null-terminated Windows-1252) to UTF-8.
    fn read_string(input: &'de [u8]) -> Cow<'de, str> {
        let null_pos = input.iter().position(|&b| b == 0).unwrap_or(input.len());
        encoding_rs::WINDOWS_1252.decode(&input[..null_pos]).0
    }

    /// Decodes a `WSTRING`'s raw bytes (null-terminated UTF-16LE) to UTF-8.
    fn read_wstring(input: &'de [u8]) -> Cow<'de, str> {
        let mut null_pos = input.len();
        for (i, chunk) in input.chunks_exact(2).enumerate() {
            if chunk[0] == 0 && chunk[1] == 0 {
                null_pos = i * 2;
                break;
            }
        }
        encoding_rs::UTF_16LE.decode(&input[..null_pos]).0
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
            AdsTypeCategory::Primitive | AdsTypeCategory::SubRange | AdsTypeCategory::Bitset => {
                match type_info.type_id() {
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
                    other => Err(crate::Error::UnsupportedType(other)),
                }
            }
            AdsTypeCategory::String => self.deserialize_string(visitor),
            AdsTypeCategory::Enum => self.deserialize_enum("", &[], visitor),
            AdsTypeCategory::Pointer | AdsTypeCategory::Reference | AdsTypeCategory::Interface => {
                match pointer_size {
                    2 => self.deserialize_u16(visitor),
                    4 => self.deserialize_u32(visitor),
                    8 => self.deserialize_u64(visitor),
                    other => Err(crate::Error::InvalidByteLength(other as usize)),
                }
            }
            AdsTypeCategory::Alias => {
                let target = resolve_alias(type_info, self.provider, pointer_size)?;
                Self::new(self.input, target, self.provider).deserialize_any(visitor)
            }
            AdsTypeCategory::Struct | AdsTypeCategory::FunctionBlock | AdsTypeCategory::Union => {
                self.deserialize_map(visitor)
            }
            AdsTypeCategory::Array => self.deserialize_seq(visitor),
            AdsTypeCategory::None => self.deserialize_unit(visitor),
            AdsTypeCategory::Program | AdsTypeCategory::Function => {
                unreachable!("AdsTypeCategory::determine never returns Program/Function")
            }
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        validate_type_id(self.type_info, AdsTypeId::Bit)?;
        let byte = Self::read_bytes::<1>(self.input)?[0];

        visitor.visit_bool(byte != 0)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        validate_type_id(self.type_info, AdsTypeId::Int8)?;
        let bytes = Self::read_bytes::<1>(self.input)?;

        visitor.visit_i8(i8::from_le_bytes(bytes))
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        validate_type_id(self.type_info, AdsTypeId::Int16)?;
        let bytes = Self::read_bytes::<2>(self.input)?;

        visitor.visit_i16(i16::from_le_bytes(bytes))
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        validate_type_id(self.type_info, AdsTypeId::Int32)?;
        let bytes = Self::read_bytes::<4>(self.input)?;

        visitor.visit_i32(i32::from_le_bytes(bytes))
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        validate_type_id(self.type_info, AdsTypeId::Int64)?;
        let bytes = Self::read_bytes::<8>(self.input)?;

        visitor.visit_i64(i64::from_le_bytes(bytes))
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        validate_type_id(self.type_info, AdsTypeId::UInt8)?;
        let bytes = Self::read_bytes::<1>(self.input)?;

        visitor.visit_u8(u8::from_le_bytes(bytes))
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        validate_integer_type_id::<2>(
            self.type_info,
            AdsTypeId::UInt16,
            self.provider.get_platform_ptr_size(),
        )?;
        let bytes = Self::read_bytes::<2>(self.input)?;

        visitor.visit_u16(u16::from_le_bytes(bytes))
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        validate_integer_type_id::<4>(
            self.type_info,
            AdsTypeId::UInt32,
            self.provider.get_platform_ptr_size(),
        )?;
        let bytes = Self::read_bytes::<4>(self.input)?;

        visitor.visit_u32(u32::from_le_bytes(bytes))
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        validate_integer_type_id::<8>(
            self.type_info,
            AdsTypeId::UInt64,
            self.provider.get_platform_ptr_size(),
        )?;
        let bytes = Self::read_bytes::<8>(self.input)?;

        visitor.visit_u64(u64::from_le_bytes(bytes))
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        validate_type_id(self.type_info, AdsTypeId::Real32)?;
        let bytes = Self::read_bytes::<{ size_of::<f32>() }>(self.input)?;

        visitor.visit_f32(f32::from_le_bytes(bytes))
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        validate_type_id(self.type_info, AdsTypeId::Real64)?;
        let bytes = Self::read_bytes::<{ size_of::<f64>() }>(self.input)?;

        visitor.visit_f64(f64::from_le_bytes(bytes))
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        validate_exact_size(self.input, self.type_info.size() as usize)?;

        match self.type_info.type_id() {
            AdsTypeId::String => match Self::read_string(self.input) {
                Cow::Borrowed(s) => visitor.visit_borrowed_str(s),
                Cow::Owned(s) => visitor.visit_string(s),
            },
            AdsTypeId::WString => match Self::read_wstring(self.input) {
                Cow::Borrowed(s) => visitor.visit_borrowed_str(s),
                Cow::Owned(s) => visitor.visit_string(s),
            },
            other => Err(crate::Error::TypeMismatch {
                expected: format!(
                    "STRING/WSTRING, but PLC type is '{}' ({other:?})",
                    self.type_info.name(),
                ),
            }),
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        validate_exact_size(self.input, self.type_info.size() as usize)?;

        let decoded_string = match self.type_info.type_id() {
            AdsTypeId::String => Self::read_string(self.input).into_owned(),
            AdsTypeId::WString => Self::read_wstring(self.input).into_owned(),
            other => {
                return Err(crate::Error::TypeMismatch {
                    expected: format!(
                        "STRING/WSTRING, but PLC type is '{}' ({other:?})",
                        self.type_info.name(),
                    ),
                });
            }
        };

        visitor.visit_string(decoded_string)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_borrowed_bytes(self.input)
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
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let ptr_size = self.provider.get_platform_ptr_size();
        let type_info = resolve_alias(self.type_info, self.provider, ptr_size)?;

        validate_type_category(type_info, &[AdsTypeCategory::Array], ptr_size)?;
        validate_exact_size(self.input, type_info.size() as usize)?;

        let array = AdsArrayAccess::new(
            type_info.array_infos(),
            type_info.type_name(),
            self.input,
            self.provider,
        )?;
        visitor.visit_seq(array)
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let ptr_size = self.provider.get_platform_ptr_size();
        let type_info = resolve_alias(self.type_info, self.provider, ptr_size)?;

        validate_type_category(
            type_info,
            &[
                AdsTypeCategory::Struct,
                AdsTypeCategory::FunctionBlock,
                AdsTypeCategory::Union,
                AdsTypeCategory::Array,
            ],
            ptr_size,
        )?;
        validate_exact_size(self.input, type_info.size() as usize)?;

        match AdsTypeCategory::determine(type_info, ptr_size) {
            AdsTypeCategory::Array => {
                let count: usize = type_info
                    .array_infos()
                    .iter()
                    .map(|d| d.element_count() as usize)
                    .product();
                if count != len {
                    return Err(crate::Error::ShapeMismatch {
                        expected: len,
                        got: count,
                    });
                }
                let array = AdsArrayAccess::new(
                    type_info.array_infos(),
                    type_info.type_name(),
                    self.input,
                    self.provider,
                )?;
                visitor.visit_seq(array)
            }
            AdsTypeCategory::Struct | AdsTypeCategory::FunctionBlock | AdsTypeCategory::Union => {
                if let Some(resolved) = &self.resolved_fields {
                    if resolved.len() != len {
                        return Err(crate::Error::ShapeMismatch {
                            expected: len,
                            got: resolved.len(),
                        });
                    }
                    return visitor.visit_seq(AdsStructAccess::new(
                        resolved.clone(),
                        self.input,
                        self.provider,
                    ));
                }
                let fields = type_info.field_infos();
                if fields.len() != len {
                    return Err(crate::Error::ShapeMismatch {
                        expected: len,
                        got: fields.len(),
                    });
                }
                let resolved: Rc<[ResolvedField<'de>]> =
                    Rc::from(resolve_fields(fields, self.provider)?);
                visitor.visit_seq(AdsStructAccess::new(resolved, self.input, self.provider))
            }
            _ => Err(crate::Error::TypeMismatch {
                expected: "array or struct (for tuple deserialization)".into(),
            }),
        }
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let ptr_size = self.provider.get_platform_ptr_size();
        let type_info = resolve_alias(self.type_info, self.provider, ptr_size)?;

        validate_type_category(
            type_info,
            &[
                AdsTypeCategory::Struct,
                AdsTypeCategory::FunctionBlock,
                AdsTypeCategory::Union,
            ],
            ptr_size,
        )?;
        validate_exact_size(self.input, type_info.size() as usize)?;

        visitor.visit_map(AdsMapAccess::new(
            type_info.field_infos(),
            self.input,
            self.provider,
        ))
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let ptr_size = self.provider.get_platform_ptr_size();
        let type_info = resolve_alias(self.type_info, self.provider, ptr_size)?;

        validate_type_category(
            type_info,
            &[
                AdsTypeCategory::Struct,
                AdsTypeCategory::FunctionBlock,
                AdsTypeCategory::Union,
            ],
            ptr_size,
        )?;
        validate_exact_size(self.input, type_info.size() as usize)?;

        if let Some(resolved) = &self.resolved_fields {
            if resolved.len() != fields.len() {
                return Err(crate::Error::ShapeMismatch {
                    expected: fields.len(),
                    got: resolved.len(),
                });
            }
            return visitor.visit_seq(AdsStructAccess::new(
                resolved.clone(),
                self.input,
                self.provider,
            ));
        }

        let plc_fields = type_info.field_infos();
        if plc_fields.len() != fields.len() {
            return Err(crate::Error::ShapeMismatch {
                expected: fields.len(),
                got: plc_fields.len(),
            });
        }

        let resolved: Rc<[ResolvedField<'de>]> =
            Rc::from(resolve_fields(plc_fields, self.provider)?);
        visitor.visit_seq(AdsStructAccess::new(resolved, self.input, self.provider))
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
        let type_info = resolve_alias(self.type_info, self.provider, ptr_size)?;

        validate_type_category(type_info, &[AdsTypeCategory::Enum], ptr_size)?;
        validate_exact_size(self.input, type_info.size() as usize)?;

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

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }
}
