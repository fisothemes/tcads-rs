pub mod access;
pub mod deserializer;

pub use deserializer::AdsDeserializer;

use crate::TypeProvider;
use serde::de::DeserializeOwned;
use tcads_core::AdsTypeInfo;

/// Deserializes a byte buffer into a Rust value, using the given PLC type metadata.
pub fn from_bytes<T, P>(data: &[u8], type_info: &AdsTypeInfo, provider: &P) -> crate::Result<T>
where
    T: DeserializeOwned,
    P: TypeProvider,
{
    let deserializer = AdsDeserializer::new(data, type_info, provider);
    T::deserialize(deserializer)
}
