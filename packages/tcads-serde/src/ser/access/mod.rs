pub mod array_serializer;
pub mod map_serializer;
pub mod struct_serializer;
pub mod tuple_serializer;

pub use array_serializer::AdsArraySerializer;
pub use map_serializer::AdsMapSerializer;
pub use struct_serializer::AdsStructSerializer;
pub use tuple_serializer::AdsTupleSerializer;

macro_rules! unsupported_serialize_methods {
    ($err:path => $($method:ident)+) => {
        $(unsupported_serialize_methods!(@one $err, $method);)+
    };
    (@one $err:path, bool) => {
        fn serialize_bool(self, _v: bool) -> Result<Self::Ok, Self::Error> { Err($err()) }
    };
    (@one $err:path, i8) => {
        fn serialize_i8(self, _v: i8) -> Result<Self::Ok, Self::Error> { Err($err()) }
    };
    (@one $err:path, i16) => {
        fn serialize_i16(self, _v: i16) -> Result<Self::Ok, Self::Error> { Err($err()) }
    };
    (@one $err:path, i32) => {
        fn serialize_i32(self, _v: i32) -> Result<Self::Ok, Self::Error> { Err($err()) }
    };
    (@one $err:path, i64) => {
        fn serialize_i64(self, _v: i64) -> Result<Self::Ok, Self::Error> { Err($err()) }
    };
    (@one $err:path, u8) => {
        fn serialize_u8(self, _v: u8) -> Result<Self::Ok, Self::Error> { Err($err()) }
    };
    (@one $err:path, u16) => {
        fn serialize_u16(self, _v: u16) -> Result<Self::Ok, Self::Error> { Err($err()) }
    };
    (@one $err:path, u32) => {
        fn serialize_u32(self, _v: u32) -> Result<Self::Ok, Self::Error> { Err($err()) }
    };
    (@one $err:path, u64) => {
        fn serialize_u64(self, _v: u64) -> Result<Self::Ok, Self::Error> { Err($err()) }
    };
    (@one $err:path, f32) => {
        fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Self::Error> { Err($err()) }
    };
    (@one $err:path, f64) => {
        fn serialize_f64(self, _v: f64) -> Result<Self::Ok, Self::Error> { Err($err()) }
    };
    (@one $err:path, char) => {
        fn serialize_char(self, _v: char) -> Result<Self::Ok, Self::Error> { Err($err()) }
    };
    (@one $err:path, str) => {
        fn serialize_str(self, _v: &str) -> Result<Self::Ok, Self::Error> { Err($err()) }
    };
    (@one $err:path, bytes) => {
        fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> { Err($err()) }
    };
    (@one $err:path, none) => {
        fn serialize_none(self) -> Result<Self::Ok, Self::Error> { Err($err()) }
    };
    (@one $err:path, some) => {
        fn serialize_some<T>(self, _value: &T) -> Result<Self::Ok, Self::Error>
        where T: ?Sized + serde::Serialize { Err($err()) }
    };
    (@one $err:path, unit) => {
        fn serialize_unit(self) -> Result<Self::Ok, Self::Error> { Err($err()) }
    };
    (@one $err:path, unit_struct) => {
        fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
            Err($err())
        }
    };
    (@one $err:path, unit_variant) => {
        fn serialize_unit_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            _variant: &'static str,
        ) -> Result<Self::Ok, Self::Error> { Err($err()) }
    };
    (@one $err:path, newtype_variant) => {
        fn serialize_newtype_variant<T>(
            self,
            _name: &'static str,
            _variant_index: u32,
            _variant: &'static str,
            _value: &T,
        ) -> Result<Self::Ok, Self::Error>
        where T: ?Sized + serde::Serialize { Err($err()) }
    };
    (@one $err:path, seq) => {
        fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
            Err($err())
        }
    };
    (@one $err:path, tuple) => {
        fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
            Err($err())
        }
    };
    (@one $err:path, tuple_struct) => {
        fn serialize_tuple_struct(
            self,
            _name: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeTupleStruct, Self::Error> { Err($err()) }
    };
    (@one $err:path, tuple_variant) => {
        fn serialize_tuple_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            _variant: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeTupleVariant, Self::Error> { Err($err()) }
    };
    (@one $err:path, map) => {
        fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
            Err($err())
        }
    };
    (@one $err:path, r#struct) => {
        fn serialize_struct(
            self,
            _name: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeStruct, Self::Error> { Err($err()) }
    };
    (@one $err:path, struct_variant) => {
        fn serialize_struct_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            _variant: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeStructVariant, Self::Error> { Err($err()) }
    };
}

pub(crate) use unsupported_serialize_methods;
