pub mod access;
pub mod serializer;

pub use serializer::AdsSerializer;

use crate::TypeProvider;
use serde::Serialize;
use tcads_core::AdsTypeInfo;

/// Serializes a Rust value into a byte buffer, using the given PLC type metadata.
///
/// `buf` must be exactly [`type_info.size()`](AdsTypeInfo::size) bytes.
pub fn to_bytes<T, P>(
    value: &T,
    buf: &mut [u8],
    type_info: &AdsTypeInfo,
    provider: &P,
) -> crate::Result<()>
where
    T: Serialize,
    P: TypeProvider,
{
    let serializer = AdsSerializer::new(buf, type_info, provider);
    value.serialize(serializer)?;
    Ok(())
}

/// Serializes a Rust value into a byte vector, using the given PLC type metadata.
pub fn to_vec<T, P>(value: &T, type_info: &AdsTypeInfo, provider: &P) -> crate::Result<Vec<u8>>
where
    T: Serialize,
    P: TypeProvider,
{
    let mut buf = vec![0u8; type_info.size() as usize];
    to_bytes(value, &mut buf, type_info, provider)?;
    Ok(buf)
}
