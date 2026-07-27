pub mod access;
pub mod serializer;

pub use serializer::{AdsRpcSerializer, AdsSerializer};

use crate::TypeProvider;
use crate::resolvers::ResolvedField;
use serde::Serialize;
use std::rc::Rc;
use tcads_core::AdsTypeInfo;

/// Serializes a Rust value into a byte buffer, using the given PLC type metadata.
///
/// # Note
///
/// `buf` must be exactly [`type_info.size()`](AdsTypeInfo::size) bytes.
pub fn to_bytes<T, P>(
    value: &T,
    buf: &mut [u8],
    type_info: &AdsTypeInfo,
    provider: &P,
) -> crate::Result<()>
where
    T: Serialize + ?Sized,
    P: TypeProvider,
{
    let serializer = AdsSerializer::new(buf, type_info, provider);
    value.serialize(serializer)?;
    Ok(())
}

/// Serializes a Rust value into a byte vector, using the given PLC type metadata.
pub fn to_vec<T, P>(value: &T, type_info: &AdsTypeInfo, provider: &P) -> crate::Result<Vec<u8>>
where
    T: Serialize + ?Sized,
    P: TypeProvider,
{
    let mut buf = vec![0u8; type_info.size() as usize];
    to_bytes(value, &mut buf, type_info, provider)?;
    Ok(buf)
}

/// Serializes a plain tuple (or `()`) into a byte buffer, positionally,
/// against a caller-supplied field list rather than a named PLC type.
///
/// # Note
///
/// `buf` must be exactly the sum of every field's size.
pub fn to_rpc_fields<'ser, T, P>(
    value: &T,
    buf: &'ser mut [u8],
    fields: Rc<[ResolvedField<'ser>]>,
    provider: &'ser P,
) -> crate::Result<()>
where
    T: Serialize + ?Sized,
    P: TypeProvider,
{
    let serializer = AdsRpcSerializer::new(fields, buf, provider);
    value.serialize(serializer)?;
    Ok(())
}
