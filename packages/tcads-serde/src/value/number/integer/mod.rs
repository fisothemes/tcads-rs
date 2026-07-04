use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use tcads_core::AdsTypeId;

pub mod signed;
pub mod unsigned;

pub use signed::SignedInteger;
pub use unsigned::UnsignedInteger;

#[derive(Clone, Copy, Hash)]
pub enum Integer {
    Signed(SignedInteger),
    Unsigned(UnsignedInteger),
}

impl Integer {
    pub const fn is_signed(&self) -> bool {
        matches!(self, Integer::Signed(_))
    }

    pub const fn is_unsigned(&self) -> bool {
        matches!(self, Integer::Unsigned(_))
    }

    pub const fn is_zero(&self) -> bool {
        match self {
            Integer::Signed(s) => s.is_zero(),
            Integer::Unsigned(u) => u.is_zero(),
        }
    }

    pub fn try_from_le_slice(slice: &[u8], type_id: AdsTypeId) -> crate::Result<Self> {
        fn read<const N: usize>(bytes: &[u8]) -> Result<[u8; N], crate::Error> {
            bytes.try_into().map_err(|_| crate::Error::SizeMismatch {
                expected: N,
                got: bytes.len(),
            })
        }

        match type_id {
            AdsTypeId::Int8 => {
                let bytes = read::<1>(slice)?;
                Ok(Integer::from(i8::from_le_bytes(bytes)))
            }
            AdsTypeId::Int16 => {
                let bytes = read::<2>(slice)?;
                Ok(Integer::from(i16::from_le_bytes(bytes)))
            }
            AdsTypeId::Int32 => {
                let bytes = read::<4>(slice)?;
                Ok(Integer::from(i32::from_le_bytes(bytes)))
            }
            AdsTypeId::Int64 => {
                let bytes = read::<8>(slice)?;
                Ok(Integer::from(i64::from_le_bytes(bytes)))
            }
            AdsTypeId::UInt8 => {
                let bytes = read::<1>(slice)?;
                Ok(Integer::from(u8::from_le_bytes(bytes)))
            }
            AdsTypeId::UInt16 => {
                let bytes = read::<2>(slice)?;
                Ok(Integer::from(u16::from_le_bytes(bytes)))
            }
            AdsTypeId::UInt32 => {
                let bytes = read::<4>(slice)?;
                Ok(Integer::from(u32::from_le_bytes(bytes)))
            }
            AdsTypeId::UInt64 => {
                let bytes = read::<8>(slice)?;
                Ok(Integer::from(u64::from_le_bytes(bytes)))
            }
            other => Err(crate::Error::UnsupportedType(other)),
        }
    }

    pub const fn is_negative(&self) -> bool {
        match self {
            Integer::Signed(s) => s.is_negative(),
            Integer::Unsigned(_) => false,
        }
    }

    pub const fn is_positive(&self) -> bool {
        !self.is_negative()
    }

    pub const fn is_sint(&self) -> bool {
        match self {
            Integer::Signed(s) => s.is_sint(),
            Integer::Unsigned(_) => false,
        }
    }

    pub const fn is_int(&self) -> bool {
        match self {
            Integer::Signed(s) => s.is_int(),
            Integer::Unsigned(_) => false,
        }
    }

    pub const fn is_dint(&self) -> bool {
        match self {
            Integer::Signed(s) => s.is_dint(),
            Integer::Unsigned(_) => false,
        }
    }

    pub const fn is_lint(&self) -> bool {
        match self {
            Integer::Signed(s) => s.is_lint(),
            Integer::Unsigned(_) => false,
        }
    }

    pub const fn is_byte(&self) -> bool {
        match self {
            Integer::Signed(_) => false,
            Integer::Unsigned(u) => u.is_byte(),
        }
    }

    pub const fn is_uint(&self) -> bool {
        match self {
            Integer::Signed(_) => false,
            Integer::Unsigned(u) => u.is_uint(),
        }
    }

    pub const fn is_udint(&self) -> bool {
        match self {
            Integer::Signed(_) => false,
            Integer::Unsigned(u) => u.is_udint(),
        }
    }

    pub const fn is_ulint(&self) -> bool {
        match self {
            Integer::Signed(_) => false,
            Integer::Unsigned(u) => u.is_ulint(),
        }
    }
}

impl Serialize for Integer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Integer::Signed(s) => s.serialize(serializer),
            Integer::Unsigned(u) => u.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for Integer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let n = i128::deserialize(deserializer)?;

        if n >= 0 {
            Ok(Integer::Unsigned(UnsignedInteger::from(n as u64)))
        } else {
            Ok(Integer::Signed(SignedInteger::from(n as i64)))
        }
    }
}

impl fmt::Debug for Integer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Integer::Signed(s) => write!(f, "{s:?})"),
            Integer::Unsigned(u) => write!(f, "{u:?})"),
        }
    }
}

impl fmt::Display for Integer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Integer::Signed(s) => write!(f, "{s}"),
            Integer::Unsigned(u) => write!(f, "{u}"),
        }
    }
}

impl From<SignedInteger> for Integer {
    fn from(s: SignedInteger) -> Self {
        Integer::Signed(s)
    }
}

impl From<i8> for Integer {
    fn from(n: i8) -> Self {
        Integer::Signed(n.into())
    }
}
impl From<i16> for Integer {
    fn from(n: i16) -> Self {
        Integer::Signed(n.into())
    }
}
impl From<i32> for Integer {
    fn from(n: i32) -> Self {
        Integer::Signed(n.into())
    }
}
impl From<i64> for Integer {
    fn from(n: i64) -> Self {
        Integer::Signed(n.into())
    }
}

impl From<UnsignedInteger> for Integer {
    fn from(u: UnsignedInteger) -> Self {
        Integer::Unsigned(u)
    }
}

impl From<u8> for Integer {
    fn from(n: u8) -> Self {
        Integer::Unsigned(n.into())
    }
}
impl From<u16> for Integer {
    fn from(n: u16) -> Self {
        Integer::Unsigned(n.into())
    }
}
impl From<u32> for Integer {
    fn from(n: u32) -> Self {
        Integer::Unsigned(n.into())
    }
}
impl From<u64> for Integer {
    fn from(n: u64) -> Self {
        Integer::Unsigned(n.into())
    }
}

impl PartialEq for Integer {
    fn eq(&self, other: &Self) -> bool {
        match (*self, *other) {
            (Integer::Signed(a), Integer::Signed(b)) => a == b,
            (Integer::Unsigned(a), Integer::Unsigned(b)) => a == b,
            (Integer::Signed(s), Integer::Unsigned(u)) => {
                let s: i64 = s.into();
                let u: u64 = u.into();
                if s.is_negative() {
                    false
                } else {
                    u == s as u64
                }
            }
            (Integer::Unsigned(u), Integer::Signed(s)) => {
                let s: i64 = s.into();
                let u: u64 = u.into();
                if s.is_negative() {
                    false
                } else {
                    u == s as u64
                }
            }
        }
    }
}

impl Eq for Integer {}

impl PartialOrd for Integer {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Integer {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Integer::Signed(a), Integer::Signed(b)) => a.cmp(b),
            (Integer::Unsigned(a), Integer::Unsigned(b)) => a.cmp(b),
            (Integer::Signed(s), Integer::Unsigned(u)) => {
                let s: i64 = (*s).into();
                let u: u64 = (*u).into();

                if s.is_negative() {
                    std::cmp::Ordering::Less
                } else {
                    (s as u64).cmp(&u)
                }
            }
            (Integer::Unsigned(u), Integer::Signed(s)) => {
                let u: u64 = (*u).into();
                let s: i64 = (*s).into();

                if s.is_negative() {
                    std::cmp::Ordering::Greater
                } else {
                    u.cmp(&(s as u64))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_less_than_ordering() {
        assert!(Integer::from(1u8) < Integer::from(2i8));
        assert!(Integer::from(1i8) < Integer::from(2u8));
    }

    #[test]
    fn test_greater_than_ordering() {
        assert!(Integer::from(2u8) > Integer::from(1i8));
        assert!(Integer::from(2i8) > Integer::from(1u8));
    }

    #[test]
    fn test_equality() {
        assert_eq!(Integer::from(1u8), Integer::from(1i8));
        assert_eq!(Integer::from(1i8), Integer::from(1u8));
    }
}
