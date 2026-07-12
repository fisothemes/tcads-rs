use super::UnsignedInteger;
use serde::de::{Error, Visitor};
use std::fmt;

pub struct UnsignedIntegerVisitor;

impl<'de> Visitor<'de> for UnsignedIntegerVisitor {
    type Value = UnsignedInteger;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("an unsigned integer (BYTE/USINT, UINT, UDINT, or ULINT)")
    }

    fn visit_i64<E: Error>(self, v: i64) -> Result<Self::Value, E> {
        u64::try_from(v)
            .map(UnsignedInteger::from)
            .map_err(|_| E::custom(format!("{v} does not fit in an unsigned integer")))
    }

    fn visit_u8<E: Error>(self, v: u8) -> Result<Self::Value, E> {
        Ok(UnsignedInteger::Byte(v))
    }

    fn visit_u16<E: Error>(self, v: u16) -> Result<Self::Value, E> {
        Ok(UnsignedInteger::UInt(v))
    }

    fn visit_u32<E: Error>(self, v: u32) -> Result<Self::Value, E> {
        Ok(UnsignedInteger::UDInt(v))
    }

    fn visit_u64<E: Error>(self, v: u64) -> Result<Self::Value, E> {
        Ok(UnsignedInteger::ULInt(v))
    }
}
