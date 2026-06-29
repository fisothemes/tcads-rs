pub mod deserializer;

use crate::TypeProvider;
pub use deserializer::AdsDeserializer;
use serde::de::DeserializeOwned;
use tcads_core::AdsTypeInfo;

/// Deserializes a byte buffer into a Rust type, using the given PLC type metadata.
pub fn from_bytes<T, P>(data: &[u8], type_info: &AdsTypeInfo, provider: &P) -> crate::Result<T>
where
    T: DeserializeOwned,
    P: TypeProvider,
{
    let deserializer = AdsDeserializer::new(data, type_info, provider);
    T::deserialize(deserializer)
}
