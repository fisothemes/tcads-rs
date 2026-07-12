use super::visitors::{
    FloatVisitor, IntegerVisitor, NumberVisitor, SignedIntegerVisitor, UnsignedIntegerVisitor,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

pub mod float;
pub mod integer;

pub use float::Float;
pub use integer::{Integer, SignedInteger, UnsignedInteger};

#[derive(Clone, Copy)]
pub enum Number {
    Integer(Integer),
    Float(Float),
}

impl Number {
    pub fn is_float(&self) -> bool {
        matches!(self, Number::Float(_))
    }

    pub fn is_integer(&self) -> bool {
        matches!(self, Number::Integer(_))
    }
}

impl Serialize for Number {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Number::Integer(i) => i.serialize(serializer),
            Number::Float(f) => f.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for Number {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(NumberVisitor)
    }
}

impl fmt::Debug for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Number::Integer(n) => write!(f, "{n:?}"),
            Number::Float(n) => write!(f, "{n:?}"),
        }
    }
}

impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Number::Integer(n) => write!(f, "{n}"),
            Number::Float(n) => write!(f, "{n}"),
        }
    }
}

impl From<Integer> for Number {
    fn from(i: Integer) -> Self {
        Number::Integer(i)
    }
}

impl From<Float> for Number {
    fn from(f: Float) -> Self {
        Number::Float(f)
    }
}

impl From<SignedInteger> for Number {
    fn from(s: SignedInteger) -> Self {
        Number::Integer(s.into())
    }
}

impl From<i8> for Number {
    fn from(n: i8) -> Self {
        Number::Integer(n.into())
    }
}
impl From<i16> for Number {
    fn from(n: i16) -> Self {
        Number::Integer(n.into())
    }
}
impl From<i32> for Number {
    fn from(n: i32) -> Self {
        Number::Integer(n.into())
    }
}
impl From<i64> for Number {
    fn from(n: i64) -> Self {
        Number::Integer(n.into())
    }
}

impl From<UnsignedInteger> for Number {
    fn from(u: UnsignedInteger) -> Self {
        Number::Integer(u.into())
    }
}

impl From<u8> for Number {
    fn from(n: u8) -> Self {
        Number::Integer(n.into())
    }
}
impl From<u16> for Number {
    fn from(n: u16) -> Self {
        Number::Integer(n.into())
    }
}
impl From<u32> for Number {
    fn from(n: u32) -> Self {
        Number::Integer(n.into())
    }
}
impl From<u64> for Number {
    fn from(n: u64) -> Self {
        Number::Integer(n.into())
    }
}

impl From<f32> for Number {
    fn from(n: f32) -> Self {
        Number::Float(n.into())
    }
}
impl From<f64> for Number {
    fn from(n: f64) -> Self {
        Number::Float(n.into())
    }
}

impl PartialEq for Number {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Number::Integer(a), Number::Integer(b)) => a == b,
            (Number::Float(a), Number::Float(b)) => a == b,
            (Number::Integer(i), Number::Float(f)) => match *i {
                Integer::Signed(i) => i64::from(i) as f64 == f64::from(*f),
                Integer::Unsigned(u) => u64::from(u) as f64 == f64::from(*f),
            },
            (Number::Float(f), Number::Integer(i)) => match *i {
                Integer::Signed(i) => i64::from(i) as f64 == f64::from(*f),
                Integer::Unsigned(u) => u64::from(u) as f64 == f64::from(*f),
            },
        }
    }
}

impl PartialOrd for Number {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Number::Integer(a), Number::Integer(b)) => Some(a.cmp(b)),
            (Number::Float(a), Number::Float(b)) => a.partial_cmp(b),
            (Number::Integer(i), Number::Float(f)) => match *i {
                Integer::Signed(i) => (i64::from(i) as f64).partial_cmp(&(f64::from(*f))),
                Integer::Unsigned(u) => (u64::from(u) as f64).partial_cmp(&(f64::from(*f))),
            },
            (Number::Float(f), Number::Integer(i)) => match *i {
                Integer::Signed(i) => f64::from(*f).partial_cmp(&(i64::from(i) as f64)),
                Integer::Unsigned(u) => f64::from(*f).partial_cmp(&(u64::from(u) as f64)),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_less_than_ordering() {
        assert!(Number::from(1i8) < Number::from(2f32));
        assert!(Number::from(1f64) < Number::from(2i16));
    }

    #[test]
    fn test_greater_than_ordering() {
        assert!(Number::from(2i16) > Number::from(1f64));
        assert!(Number::from(2f32) > Number::from(1i8));
    }
    #[test]
    fn test_equality() {
        assert_eq!(Number::from(1i8), Number::from(1f64));
        assert_eq!(Number::from(1f64), Number::from(1i8));
    }
}
