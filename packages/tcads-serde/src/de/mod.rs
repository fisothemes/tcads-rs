pub mod access;
pub mod deserializer;

pub use deserializer::{AdsDeserializer, AdsRpcDeserializer};

use crate::TypeProvider;
use crate::resolvers::ResolvedField;
use serde::de::DeserializeOwned;
use std::rc::Rc;
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

/// Deserializes a byte buffer into a plain tuple (or `()`), positionally,
/// against a caller-supplied field list rather than a named PLC type.
pub fn from_rpc_fields<'de, T, P>(
    data: &'de [u8],
    fields: Rc<[ResolvedField<'de>]>,
    provider: &'de P,
) -> crate::Result<T>
where
    T: DeserializeOwned,
    P: TypeProvider,
{
    let deserializer = AdsRpcDeserializer::new(fields, data, provider);
    T::deserialize(deserializer)
}
