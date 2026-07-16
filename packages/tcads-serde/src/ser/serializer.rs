use super::access::{
    AdsArraySerializer, AdsMapSerializer, AdsStructSerializer, AdsTupleSerializer,
};
use crate::TypeProvider;
use crate::resolvers::{ResolvedField, resolve_alias, resolve_fields};
use crate::validators::{
    validate_exact_size, validate_integer_type_id, validate_type_category, validate_type_id,
};
use serde::Serialize;
use serde::ser::{Impossible, Serializer};
use std::rc::Rc;
use tcads_core::{AdsTypeCategory, AdsTypeId, AdsTypeInfo};

/// Serializes a Rust value into a PLC memory layout, driven by `AdsTypeInfo` metadata.
///
/// # Note
///
/// The size of the buffer must be the size of [`AdsTypeInfo::size()`]
pub struct AdsSerializer<'ser, P: TypeProvider> {
    output: &'ser mut [u8],
    type_info: &'ser AdsTypeInfo,
    provider: &'ser P,
    resolved_fields: Option<Rc<[ResolvedField<'ser>]>>,
}

impl<'ser, P: TypeProvider> AdsSerializer<'ser, P> {
    /// Creates a new instance of the [`AdsSerializer`].
    pub fn new(output: &'ser mut [u8], type_info: &'ser AdsTypeInfo, provider: &'ser P) -> Self {
        Self {
            output,
            type_info,
            provider,
            resolved_fields: None,
        }
    }

    /// Same as [`new`](Self::new), but carrying fields the caller already.
    pub fn with_resolved_fields(
        output: &'ser mut [u8],
        type_info: &'ser AdsTypeInfo,
        provider: &'ser P,
        resolved_fields: Rc<[ResolvedField<'ser>]>,
    ) -> Self {
        Self {
            output,
            type_info,
            provider,
            resolved_fields: Some(resolved_fields),
        }
    }

    /// The buffer that was passed to the serializer, i.e. the memory layout that will be written to
    /// the PLC.
    pub fn output(&self) -> &[u8] {
        self.output
    }

    /// The [`AdsTypeInfo`] that was used to resolve the type.
    pub fn type_info(&self) -> &AdsTypeInfo {
        self.type_info
    }

    /// The [`TypeProvider`] that is used to resolve the type if necessary, e.g. for aliases,
    /// structs, arrays, etc.
    pub fn provider(&self) -> &P {
        self.provider
    }

    /// Writes an exact number of bytes into a mutable output buffer.
    ///
    /// # Type Parameters
    ///
    /// - `N`: The size of the byte array, determined at compile time.
    pub fn write_bytes<const N: usize>(
        data: &mut [u8],
        bytes: [u8; N],
    ) -> Result<(), crate::Error> {
        validate_exact_size(data, N)?;
        data.copy_from_slice(&bytes);
        Ok(())
    }

    /// Encodes a `STRING` into a fixed-width, null-terminated Windows-1252 buffer.
    pub fn write_string(output: &mut [u8], value: &str) -> Result<(), crate::Error> {
        let (encoded, _, _) = encoding_rs::WINDOWS_1252.encode(value);

        if encoded.len() + 1 > output.len() {
            return Err(crate::Error::SizeMismatch {
                expected: output.len(),
                got: encoded.len() + 1,
            });
        }

        output[..encoded.len()].copy_from_slice(&encoded);
        output[encoded.len()..].fill(0);
        Ok(())
    }

    /// Encodes a `WSTRING` into a fixed-width, null-terminated UTF-16LE buffer.
    pub fn write_wstring(output: &mut [u8], value: &str) -> Result<(), crate::Error> {
        let encoded = value.encode_utf16();
        let needed = encoded.clone().count() * 2 + 2;
        if needed > output.len() {
            return Err(crate::Error::SizeMismatch {
                expected: output.len(),
                got: needed,
            });
        }

        let mut pos = 0;
        for unit in encoded {
            output[pos..pos + 2].copy_from_slice(&unit.to_le_bytes());
            pos += 2;
        }
        output[pos..].fill(0);
        Ok(())
    }

    fn write_enum_variant(self, variant: &str) -> Result<(), crate::Error> {
        let ptr_size = self.provider.get_platform_ptr_size();
        let type_info = resolve_alias(self.type_info, self.provider, ptr_size)?;

        validate_type_category(type_info, &[AdsTypeCategory::Enum], ptr_size)?;
        validate_exact_size(self.output, type_info.size() as usize)?;

        let value = type_info
            .enum_infos()
            .iter()
            .find(|e| e.name() == variant)
            .map(|e| e.value())
            .ok_or_else(|| {
                crate::Error::UnknownEnumVariant(variant.to_string(), type_info.name().to_string())
            })?;

        validate_exact_size(self.output, value.len())?;
        self.output.copy_from_slice(value);
        Ok(())
    }
}

impl<'ser, P: TypeProvider> Serializer for AdsSerializer<'ser, P> {
    type Ok = ();
    type Error = crate::Error;

    type SerializeSeq = AdsArraySerializer<'ser, P>;
    type SerializeTuple = AdsTupleSerializer<'ser, P>;
    type SerializeTupleStruct = AdsTupleSerializer<'ser, P>;
    type SerializeTupleVariant = Impossible<(), crate::Error>;
    type SerializeMap = AdsMapSerializer<'ser, P>;
    type SerializeStruct = AdsStructSerializer<'ser, P>;
    type SerializeStructVariant = Impossible<(), crate::Error>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        validate_type_id(self.type_info, AdsTypeId::Bit)?;
        Self::write_bytes(self.output, [v as u8])
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        validate_type_id(self.type_info, AdsTypeId::Int8)?;
        Self::write_bytes(self.output, v.to_le_bytes())
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        validate_type_id(self.type_info, AdsTypeId::Int16)?;
        Self::write_bytes(self.output, v.to_le_bytes())
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        validate_type_id(self.type_info, AdsTypeId::Int32)?;
        Self::write_bytes(self.output, v.to_le_bytes())
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        validate_type_id(self.type_info, AdsTypeId::Int64)?;
        Self::write_bytes(self.output, v.to_le_bytes())
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        validate_type_id(self.type_info, AdsTypeId::UInt8)?;
        Self::write_bytes(self.output, v.to_le_bytes())
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        validate_integer_type_id::<2>(
            self.type_info,
            AdsTypeId::UInt16,
            self.provider.get_platform_ptr_size(),
        )?;
        Self::write_bytes(self.output, v.to_le_bytes())
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        validate_integer_type_id::<4>(
            self.type_info,
            AdsTypeId::UInt32,
            self.provider.get_platform_ptr_size(),
        )?;
        Self::write_bytes(self.output, v.to_le_bytes())
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        validate_integer_type_id::<8>(
            self.type_info,
            AdsTypeId::UInt64,
            self.provider.get_platform_ptr_size(),
        )?;
        Self::write_bytes(self.output, v.to_le_bytes())
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        validate_type_id(self.type_info, AdsTypeId::Real32)?;
        Self::write_bytes(self.output, v.to_le_bytes())
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        validate_type_id(self.type_info, AdsTypeId::Real64)?;
        Self::write_bytes(self.output, v.to_le_bytes())
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        let mut buf = [0u8; 4];
        self.serialize_str(v.encode_utf8(&mut buf))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        let ptr_size = self.provider.get_platform_ptr_size();
        let type_info = resolve_alias(self.type_info, self.provider, ptr_size)?;

        if AdsTypeCategory::determine(type_info, ptr_size) == AdsTypeCategory::Enum {
            return self.write_enum_variant(v);
        }

        match type_info.type_id() {
            AdsTypeId::String => Self::write_string(self.output, v),
            AdsTypeId::WString => Self::write_wstring(self.output, v),
            other => Err(crate::Error::TypeMismatch {
                expected: format!(
                    "STRING/WSTRING, but PLC type is '{}' ({other:?})",
                    self.type_info.name(),
                ),
            }),
        }
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        validate_exact_size(self.output, v.len())?;
        self.output.copy_from_slice(v);
        Ok(())
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Err(crate::Error::NoneNotRepresentable(
            self.type_info.name().into(),
        ))
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.write_enum_variant(variant)
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
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
        T: ?Sized + Serialize,
    {
        Err(crate::Error::TypeMismatch {
            expected: "unit enum variant (PLC enums carry no payload)".into(),
        })
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        let ptr_size = self.provider.get_platform_ptr_size();
        let type_info = resolve_alias(self.type_info, self.provider, ptr_size)?;

        validate_type_category(type_info, &[AdsTypeCategory::Array], ptr_size)?;
        validate_exact_size(self.output, type_info.size() as usize)?;

        AdsArraySerializer::new(
            type_info.array_infos(),
            type_info.type_name(),
            self.output,
            self.provider,
            len,
        )
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
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
        validate_exact_size(self.output, type_info.size() as usize)?;

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
                Ok(AdsTupleSerializer::Array(AdsArraySerializer::new(
                    type_info.array_infos(),
                    type_info.type_name(),
                    self.output,
                    self.provider,
                    Some(len),
                )?))
            }
            AdsTypeCategory::Struct | AdsTypeCategory::FunctionBlock | AdsTypeCategory::Union => {
                if let Some(resolved) = &self.resolved_fields {
                    if resolved.len() != len {
                        return Err(crate::Error::ShapeMismatch {
                            expected: len,
                            got: resolved.len(),
                        });
                    }
                    return Ok(AdsTupleSerializer::Struct(AdsStructSerializer::new(
                        resolved.clone(),
                        self.output,
                        self.provider,
                    )));
                }
                let fields = type_info.field_infos();
                if fields.len() != len {
                    return Err(crate::Error::ShapeMismatch {
                        expected: len,
                        got: fields.len(),
                    });
                }
                let resolved: Rc<[ResolvedField<'ser>]> =
                    Rc::from(resolve_fields(fields, self.provider)?);
                Ok(AdsTupleSerializer::Struct(AdsStructSerializer::new(
                    resolved,
                    self.output,
                    self.provider,
                )))
            }
            _ => Err(crate::Error::TypeMismatch {
                expected: "array or struct (for tuple serialization)".into(),
            }),
        }
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_tuple(len)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(crate::Error::TypeMismatch {
            expected: "unit enum variant (PLC enums carry no payload)".into(),
        })
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
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
        validate_exact_size(self.output, type_info.size() as usize)?;

        Ok(AdsMapSerializer::new(
            type_info.name(),
            type_info.field_infos(),
            self.output,
            self.provider,
        ))
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
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
        validate_exact_size(self.output, type_info.size() as usize)?;

        let fields = type_info.field_infos();
        if fields.len() != len {
            return Err(crate::Error::ShapeMismatch {
                expected: len,
                got: fields.len(),
            });
        }

        if let Some(resolved) = &self.resolved_fields {
            if resolved.len() != len {
                return Err(crate::Error::ShapeMismatch {
                    expected: len,
                    got: resolved.len(),
                });
            }
            return Ok(AdsStructSerializer::new(
                resolved.clone(),
                self.output,
                self.provider,
            ));
        }

        let resolved: Rc<[ResolvedField<'ser>]> = Rc::from(resolve_fields(fields, self.provider)?);
        Ok(AdsStructSerializer::new(
            resolved,
            self.output,
            self.provider,
        ))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(crate::Error::TypeMismatch {
            expected: "unit enum variant (PLC enums carry no payload)".into(),
        })
    }
}
