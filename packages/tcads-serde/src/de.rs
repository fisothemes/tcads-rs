use super::TypeProvider;
use serde::de::Visitor;
use tcads_core::{AdsTypeId, AdsTypeInfo};

pub fn from_bytes<'a, T, P>(
    data: &'a [u8],
    type_info: &AdsTypeInfo,
    provider: &'a P,
) -> crate::Result<T>
where
    T: serde::Deserialize<'a>,
    P: TypeProvider,
{
    if data.len() < type_info.size() as usize {
        return Err(crate::Error::SizeMismatch {
            expected: type_info.size() as usize,
            got: data.len(),
        });
    }

    let mut de = AdsDeserializer::new(data, type_info, provider);
    T::deserialize(&mut de)
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum AdsValue {
    BOOL(bool),
    SINT(i8),
    INT(i16),
    DINT(i32),
    LINT(i64),
    BYTE(u8),
    UINT(u16),
    UDINT(u32),
    ULINT(i64),
    REAL(f32),
    LREAL(f64),
    WSTRING(String),
    STRING(String),
    STRUCT(Vec<(String, AdsValue)>),
    ARRAY(Vec<AdsValue>),
}

pub struct AdsDeserializer<'a, 'de, P: TypeProvider> {
    input: &'de [u8],
    type_info: &'a AdsTypeInfo,
    provider: &'a P,
}

impl<'a, 'de, P: TypeProvider> AdsDeserializer<'a, 'de, P> {
    pub fn new(input: &'de [u8], type_info: &'a AdsTypeInfo, provider: &'a P) -> Self {
        Self {
            input,
            type_info,
            provider,
        }
    }

    // Helper method to safely extract a specific number of bytes
    fn read_bytes<const N: usize>(&self) -> Result<[u8; N], crate::Error> {
        let bytes: [u8; N] = self
            .input
            .get(..N)
            .ok_or_else(|| crate::Error::Custom("Unexpected end of data".into()))?
            .try_into()
            .unwrap(); // Unwrap is safe due to the get(..N) check
        Ok(bytes)
    }
}

impl<'a, 'de, P: TypeProvider> serde::de::Deserializer<'de> for &mut AdsDeserializer<'a, 'de, P> {
    type Error = crate::Error;

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.type_info.type_id() {
            AdsTypeId::Bit => self.deserialize_bool(_visitor),
            AdsTypeId::Int8 => self.deserialize_i8(_visitor),
            AdsTypeId::Int16 => self.deserialize_i16(_visitor),
            AdsTypeId::Int32 => self.deserialize_i32(_visitor),
            AdsTypeId::Int64 => self.deserialize_i64(_visitor),
            AdsTypeId::UInt8 => self.deserialize_u8(_visitor),
            AdsTypeId::UInt16 => self.deserialize_u16(_visitor),
            AdsTypeId::UInt32 => self.deserialize_u32(_visitor),
            AdsTypeId::UInt64 => self.deserialize_u64(_visitor),
            AdsTypeId::Real32 => self.deserialize_f32(_visitor),
            AdsTypeId::Real64 => self.deserialize_f64(_visitor),

            _ => Err(crate::Error::Custom(format!(
                "{:?} is an unsupported type",
                self.type_info.name()
            ))),
        }
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
        todo!("Implement AdsMapAccess using self.type_info.field_infos()")
    }

    serde::forward_to_deserialize_any! {
        i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 bool char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map enum identifier ignored_any
    }
}
