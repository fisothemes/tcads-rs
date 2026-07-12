use super::SignedInteger;
use serde::de::{Error, Visitor};
use std::fmt;

pub struct SignedIntegerVisitor;

impl<'de> Visitor<'de> for SignedIntegerVisitor {
    type Value = SignedInteger;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a signed integer (SINT, INT, DINT, or LINT)")
    }

    fn visit_i8<E: Error>(self, v: i8) -> Result<Self::Value, E> {
        Ok(SignedInteger::SInt(v))
    }

    fn visit_i16<E: Error>(self, v: i16) -> Result<Self::Value, E> {
        Ok(SignedInteger::Int(v))
    }

    fn visit_i32<E: Error>(self, v: i32) -> Result<Self::Value, E> {
        Ok(SignedInteger::DInt(v))
    }

    fn visit_i64<E: Error>(self, v: i64) -> Result<Self::Value, E> {
        Ok(SignedInteger::LInt(v))
    }

    fn visit_u64<E: Error>(self, v: u64) -> Result<Self::Value, E> {
        i64::try_from(v)
            .map(SignedInteger::from)
            .map_err(|_| E::custom(format!("{v} does not fit in a signed integer")))
    }
}
