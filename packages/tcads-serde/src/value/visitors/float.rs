use super::Float;
use serde::de::{Error, Visitor};
use std::fmt;

pub struct FloatVisitor;

impl<'de> Visitor<'de> for FloatVisitor {
    type Value = Float;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a floating point number (REAL or LREAL)")
    }

    fn visit_i64<E: Error>(self, v: i64) -> Result<Self::Value, E> {
        Ok(Float::LReal(v as f64))
    }

    fn visit_u64<E: Error>(self, v: u64) -> Result<Self::Value, E> {
        Ok(Float::LReal(v as f64))
    }

    fn visit_f32<E: Error>(self, v: f32) -> Result<Self::Value, E> {
        Ok(Float::Real(v))
    }

    fn visit_f64<E: Error>(self, v: f64) -> Result<Self::Value, E> {
        Ok(Float::LReal(v))
    }
}
