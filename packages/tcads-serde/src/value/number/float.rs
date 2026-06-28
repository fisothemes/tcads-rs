use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

#[derive(Clone, Copy)]
pub enum Float {
    Real(f32),
    LReal(f64),
}

impl Float {
    pub const fn is_zero(&self) -> bool {
        match *self {
            Float::Real(n) => n == 0.0,
            Float::LReal(n) => n == 0.0,
        }
    }

    pub const fn is_negative(&self) -> bool {
        match *self {
            Float::Real(n) => n < 0.0,
            Float::LReal(n) => n < 0.0,
        }
    }

    pub const fn is_positive(&self) -> bool {
        !self.is_negative()
    }

    pub const fn is_real(&self) -> bool {
        matches!(self, Float::Real(_))
    }

    pub const fn is_lreal(&self) -> bool {
        matches!(self, Float::LReal(_))
    }
}

impl Serialize for Float {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Float::Real(n) => n.serialize(serializer),
            Float::LReal(n) => n.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for Float {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let n = f64::deserialize(deserializer)?;
        Ok(Float::from(n))
    }
}

impl fmt::Debug for Float {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Float::Real(n) => write!(f, "Real({n})"),
            Float::LReal(n) => write!(f, "LReal({n})"),
        }
    }
}

impl fmt::Display for Float {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Float::Real(n) => write!(f, "{n}"),
            Float::LReal(n) => write!(f, "{n}"),
        }
    }
}

impl From<f32> for Float {
    fn from(n: f32) -> Self {
        Float::Real(n)
    }
}

impl From<f64> for Float {
    fn from(n: f64) -> Self {
        Float::LReal(n)
    }
}

impl From<Float> for f64 {
    fn from(val: Float) -> f64 {
        match val {
            Float::Real(n) => n as f64,
            Float::LReal(n) => n,
        }
    }
}

impl From<Float> for f32 {
    fn from(val: Float) -> f32 {
        match val {
            Float::Real(n) => n,
            Float::LReal(n) => n as f32,
        }
    }
}

impl PartialEq for Float {
    fn eq(&self, other: &Self) -> bool {
        f64::from(*self) == f64::from(*other)
    }
}

impl PartialOrd for Float {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        f64::from(*self).partial_cmp(&f64::from(*other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_less_than_ordering() {
        assert!(Float::from(1.0f32) < Float::from(2.0f64));
        assert!(Float::from(1.0f64) < Float::from(2.0f32));
    }

    #[test]
    fn test_greater_than_ordering() {
        assert!(Float::from(2.0f32) > Float::from(1.0f64));
        assert!(Float::from(2.0f64) > Float::from(1.0f32));
    }

    #[test]
    fn test_equality() {
        assert_eq!(Float::from(1.0f32), Float::from(1.0f64));
        assert_eq!(Float::from(1.0f64), Float::from(1.0f32));
    }
}
