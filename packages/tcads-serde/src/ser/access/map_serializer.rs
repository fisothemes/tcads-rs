use super::field::write_field_bytes;
use super::unsupported_serialize_methods;
use crate::TypeProvider;
use serde::ser::{Impossible, SerializeMap};
use tcads_core::AdsFieldInfo;

/// Writes struct fields by name, for dynamically keyed values (`Value`, `HashMap<String, _>`)
/// where there's no compile-time field order to fall back on.
pub struct AdsMapSerializer<'ser, P: TypeProvider> {
    type_name: &'ser str,
    fields: &'ser [AdsFieldInfo],
    output: &'ser mut [u8],
    provider: &'ser P,
    pending_field: Option<&'ser AdsFieldInfo>,
}

impl<'ser, P: TypeProvider> AdsMapSerializer<'ser, P> {
    pub fn new(
        type_name: &'ser str,
        fields: &'ser [AdsFieldInfo],
        output: &'ser mut [u8],
        provider: &'ser P,
    ) -> Self {
        Self {
            type_name,
            fields,
            output,
            provider,
            pending_field: None,
        }
    }

    fn find_field(&self, name: &str) -> Result<&'ser AdsFieldInfo, crate::Error> {
        self.fields
            .iter()
            .find(|f| f.name() == name)
            .ok_or_else(|| crate::Error::UnknownField(name.to_string(), self.type_name.to_string()))
    }
}

impl<'ser, P: TypeProvider> SerializeMap for AdsMapSerializer<'ser, P> {
    type Ok = ();
    type Error = crate::Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        let name = key.serialize(FieldNameSerializer)?;
        self.pending_field = Some(self.find_field(&name)?);
        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        let field = self
            .pending_field
            .take()
            .expect("serialize_value called before serialize_key");
        write_field_bytes(self.output, field, self.provider, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

/// Renders a map key to a `String` so it can be matched against a field name.
///
/// Struct fields in PLC memory are always addressed by name, so only string-like keys
/// make sense here (e.g. `HashMap<String, Value>` or an enum used as a key).
struct FieldNameSerializer;

impl FieldNameSerializer {
    fn not_a_field_name() -> crate::Error {
        crate::Error::Custom("struct field names must be string-like".into())
    }
}

impl serde::Serializer for FieldNameSerializer {
    type Ok = String;
    type Error = crate::Error;

    type SerializeSeq = Impossible<String, crate::Error>;
    type SerializeTuple = Impossible<String, crate::Error>;
    type SerializeTupleStruct = Impossible<String, crate::Error>;
    type SerializeTupleVariant = Impossible<String, crate::Error>;
    type SerializeMap = Impossible<String, crate::Error>;
    type SerializeStruct = Impossible<String, crate::Error>;
    type SerializeStructVariant = Impossible<String, crate::Error>;

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        Ok(v.to_string())
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(v.to_string())
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Ok(variant.to_string())
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

    unsupported_serialize_methods! {
        Self::not_a_field_name =>
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 bytes none unit unit_struct
        newtype_variant seq tuple tuple_struct tuple_variant map r#struct struct_variant
    }
}
