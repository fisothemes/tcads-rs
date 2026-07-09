use crate::TypeProvider;
use crate::resolvers::resolve_alias;
use crate::validators::{validate_exact_size, validate_integer_type_id, validate_type_id};
use serde::Serialize;
use serde::ser::{Impossible, Serializer};
use tcads_core::{AdsTypeId, AdsTypeInfo};

pub struct AdsSerializer<'ser, P: TypeProvider> {
    output: &'ser mut [u8],
    type_info: &'ser AdsTypeInfo,
    provider: &'ser P,
}

impl<'ser, P: TypeProvider> AdsSerializer<'ser, P> {
    pub fn new(output: &'ser mut [u8], type_info: &'ser AdsTypeInfo, provider: &'ser P) -> Self {
        Self {
            output,
            type_info,
            provider,
        }
    }

    pub fn output(&self) -> &[u8] {
        self.output
    }

    pub fn type_info(&self) -> &AdsTypeInfo {
        self.type_info
    }

    pub fn provider(&self) -> &P {
        self.provider
    }

    pub fn write_bytes<const N: usize>(
        data: &mut [u8],
        bytes: [u8; N],
    ) -> Result<(), crate::Error> {
        validate_exact_size(data, N)?;
        data.copy_from_slice(&bytes);
        Ok(())
    }

    pub fn write_string(output: &mut [u8], value: &str) -> Result<(), crate::Error> {
        let (encoded, _, _) = encoding_rs::WINDOWS_1252.encode(value);
        if encoded.len() + 1 > output.len() {
            return Err(crate::Error::SizeMismatch {
                expected: output.len(),
                got: encoded.len() + 1,
            });
        }
        output.fill(0);
        output[..encoded.len()].copy_from_slice(&encoded);
        Ok(())
    }

    pub fn write_wstring(output: &mut [u8], value: &str) -> Result<(), crate::Error> {
        let encoded: Vec<u8> = value.encode_utf16().flat_map(u16::to_le_bytes).collect();
        if encoded.len() + 2 > output.len() {
            return Err(crate::Error::SizeMismatch {
                expected: output.len(),
                got: encoded.len() + 2,
            });
        }
        output.fill(0);
        output[..encoded.len()].copy_from_slice(&encoded);
        Ok(())
    }
}

impl<'ser, P: TypeProvider> Serializer for AdsSerializer<'ser, P> {
    type Ok = ();
    type Error = crate::Error;

    type SerializeSeq = Impossible<(), crate::Error>;
    type SerializeTuple = Impossible<(), crate::Error>;
    type SerializeTupleStruct = Impossible<(), crate::Error>;
    type SerializeTupleVariant = Impossible<(), crate::Error>;
    type SerializeMap = Impossible<(), crate::Error>;
    type SerializeStruct = Impossible<(), crate::Error>;
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
        match self.type_info.type_id() {
            AdsTypeId::String => Self::write_string(self.output, v),
            AdsTypeId::WString => Self::write_wstring(self.output, v),
            _ => Err(crate::Error::TypeMismatch {
                expected: "STRING/WSTRING based type.".into(),
            }),
        }
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        validate_exact_size(self.output, v.len())?;
        self.output.copy_from_slice(v);
        Ok(())
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Err(crate::Error::Custom(
            "Cannot serialize `None` because PLC memory has no null representation.".into(),
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
        let ptr_size = self.provider.get_platform_ptr_size();
        let type_info = resolve_alias(self.type_info, self.provider, ptr_size)?;

        let value = type_info
            .enum_infos()
            .iter()
            .find(|e| e.name() == variant)
            .map(|e| e.value())
            .ok_or_else(|| {
                crate::Error::UnknownUnknownEnumVariant(
                    variant.to_string(),
                    type_info.name().to_string(),
                )
            })?;

        validate_exact_size(self.output, value.len())?;
        self.output.copy_from_slice(value);
        Ok(())
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

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        todo!()
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        todo!()
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
        todo!()
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.serialize_map(Some(len))
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
