use super::SignedIntegerVisitor;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

#[derive(Clone, Copy, Hash)]
pub enum SignedInteger {
    SInt(i8),
    Int(i16),
    DInt(i32),
    LInt(i64),
}

impl SignedInteger {
    pub const fn is_zero(&self) -> bool {
        match *self {
            SignedInteger::SInt(n) => n == 0,
            SignedInteger::Int(n) => n == 0,
            SignedInteger::DInt(n) => n == 0,
            SignedInteger::LInt(n) => n == 0,
        }
    }

    pub const fn is_negative(&self) -> bool {
        match *self {
            SignedInteger::SInt(n) => n < 0,
            SignedInteger::Int(n) => n < 0,
            SignedInteger::DInt(n) => n < 0,
            SignedInteger::LInt(n) => n < 0,
        }
    }

    pub const fn is_positive(&self) -> bool {
        !self.is_negative()
    }

    pub const fn is_sint(&self) -> bool {
        matches!(self, SignedInteger::SInt(_))
    }

    pub const fn is_int(&self) -> bool {
        matches!(self, SignedInteger::Int(_))
    }

    pub const fn is_dint(&self) -> bool {
        matches!(self, SignedInteger::DInt(_))
    }

    pub const fn is_lint(&self) -> bool {
        matches!(self, SignedInteger::LInt(_))
    }
}

impl Serialize for SignedInteger {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            SignedInteger::SInt(n) => n.serialize(serializer),
            SignedInteger::Int(n) => n.serialize(serializer),
            SignedInteger::DInt(n) => n.serialize(serializer),
            SignedInteger::LInt(n) => n.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for SignedInteger {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(SignedIntegerVisitor)
    }
}

impl fmt::Debug for SignedInteger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SignedInteger::SInt(n) => write!(f, "SInt({n})"),
            SignedInteger::Int(n) => write!(f, "Int({n})"),
            SignedInteger::DInt(n) => write!(f, "DInt({n})"),
            SignedInteger::LInt(n) => write!(f, "LInt({n})"),
        }
    }
}

impl fmt::Display for SignedInteger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SignedInteger::SInt(n) => write!(f, "{n}"),
            SignedInteger::Int(n) => write!(f, "{n}"),
            SignedInteger::DInt(n) => write!(f, "{n}"),
            SignedInteger::LInt(n) => write!(f, "{n}"),
        }
    }
}

impl From<i8> for SignedInteger {
    fn from(n: i8) -> Self {
        SignedInteger::SInt(n)
    }
}
impl From<i16> for SignedInteger {
    fn from(n: i16) -> Self {
        SignedInteger::Int(n)
    }
}
impl From<i32> for SignedInteger {
    fn from(n: i32) -> Self {
        SignedInteger::DInt(n)
    }
}
impl From<i64> for SignedInteger {
    fn from(n: i64) -> Self {
        SignedInteger::LInt(n)
    }
}

impl From<SignedInteger> for i64 {
    fn from(val: SignedInteger) -> i64 {
        match val {
            SignedInteger::SInt(n) => n as i64,
            SignedInteger::Int(n) => n as i64,
            SignedInteger::DInt(n) => n as i64,
            SignedInteger::LInt(n) => n,
        }
    }
}

impl From<SignedInteger> for i32 {
    fn from(val: SignedInteger) -> i32 {
        match val {
            SignedInteger::SInt(n) => n as i32,
            SignedInteger::Int(n) => n as i32,
            SignedInteger::DInt(n) => n,
            SignedInteger::LInt(n) => n as i32,
        }
    }
}

impl From<SignedInteger> for i16 {
    fn from(val: SignedInteger) -> i16 {
        match val {
            SignedInteger::SInt(n) => n as i16,
            SignedInteger::Int(n) => n,
            SignedInteger::DInt(n) => n as i16,
            SignedInteger::LInt(n) => n as i16,
        }
    }
}

impl From<SignedInteger> for i8 {
    fn from(val: SignedInteger) -> i8 {
        match val {
            SignedInteger::SInt(n) => n,
            SignedInteger::Int(n) => n as i8,
            SignedInteger::DInt(n) => n as i8,
            SignedInteger::LInt(n) => n as i8,
        }
    }
}

impl PartialEq for SignedInteger {
    fn eq(&self, other: &Self) -> bool {
        i64::from(*self) == i64::from(*other)
    }
}

impl Eq for SignedInteger {}

impl PartialOrd for SignedInteger {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SignedInteger {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        i64::from(*self).cmp(&i64::from(*other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_less_than_ordering() {
        assert!(SignedInteger::SInt(1) < SignedInteger::SInt(2));
        assert!(SignedInteger::SInt(1) < SignedInteger::Int(2));
        assert!(SignedInteger::SInt(1) < SignedInteger::DInt(2));
        assert!(SignedInteger::SInt(1) < SignedInteger::LInt(2));
        assert!(SignedInteger::Int(1) < SignedInteger::SInt(2));
        assert!(SignedInteger::Int(1) < SignedInteger::Int(2));
        assert!(SignedInteger::Int(1) < SignedInteger::DInt(2));
        assert!(SignedInteger::Int(1) < SignedInteger::LInt(2));
        assert!(SignedInteger::DInt(1) < SignedInteger::SInt(2));
        assert!(SignedInteger::DInt(1) < SignedInteger::Int(2));
        assert!(SignedInteger::DInt(1) < SignedInteger::DInt(2));
        assert!(SignedInteger::DInt(1) < SignedInteger::LInt(2));
        assert!(SignedInteger::LInt(1) < SignedInteger::SInt(2));
        assert!(SignedInteger::LInt(1) < SignedInteger::Int(2));
        assert!(SignedInteger::LInt(1) < SignedInteger::DInt(2));
    }

    #[test]
    fn test_greater_than_ordering() {
        assert!(SignedInteger::SInt(2) > SignedInteger::SInt(1));
        assert!(SignedInteger::SInt(2) > SignedInteger::Int(1));
        assert!(SignedInteger::SInt(2) > SignedInteger::DInt(1));
        assert!(SignedInteger::SInt(2) > SignedInteger::LInt(1));
        assert!(SignedInteger::Int(2) > SignedInteger::SInt(1));
        assert!(SignedInteger::Int(2) > SignedInteger::Int(1));
        assert!(SignedInteger::Int(2) > SignedInteger::DInt(1));
        assert!(SignedInteger::Int(2) > SignedInteger::LInt(1));
        assert!(SignedInteger::DInt(2) > SignedInteger::SInt(1));
        assert!(SignedInteger::DInt(2) > SignedInteger::Int(1));
        assert!(SignedInteger::DInt(2) > SignedInteger::DInt(1));
        assert!(SignedInteger::DInt(2) > SignedInteger::LInt(1));
        assert!(SignedInteger::LInt(2) > SignedInteger::SInt(1));
        assert!(SignedInteger::LInt(2) > SignedInteger::Int(1));
        assert!(SignedInteger::LInt(2) > SignedInteger::DInt(1));
    }

    #[test]
    fn test_equality() {
        assert_eq!(SignedInteger::SInt(1), SignedInteger::SInt(1));
        assert_eq!(SignedInteger::Int(1), SignedInteger::Int(1));
        assert_eq!(SignedInteger::DInt(1), SignedInteger::DInt(1));
        assert_eq!(SignedInteger::LInt(1), SignedInteger::LInt(1));
    }
}
