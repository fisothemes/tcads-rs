use super::UnsignedIntegerVisitor;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// An exact-width representation of a TwinCAT unsigned integer.
#[derive(Clone, Copy, Hash)]
pub enum UnsignedInteger {
    /// An IEC 61131-3 `USINT` or `BYTE` (8-bit unsigned integer).
    Byte(u8),
    /// An IEC 61131-3 `UINT` or `WORD` (16-bit unsigned integer).
    UInt(u16),
    /// An IEC 61131-3 `UDINT` or `DWORD` (32-bit unsigned integer).
    UDInt(u32),
    /// An IEC 61131-3 `ULINT` or `LWORD` (64-bit unsigned integer).
    ULInt(u64),
}

impl UnsignedInteger {
    /// Returns `true` if the integer is 0.
    pub const fn is_zero(&self) -> bool {
        match self {
            UnsignedInteger::Byte(n) => *n == 0,
            UnsignedInteger::UInt(n) => *n == 0,
            UnsignedInteger::UDInt(n) => *n == 0,
            UnsignedInteger::ULInt(n) => *n == 0,
        }
    }

    /// Returns `true` if the value is an 8-bit `BYTE` / `USINT`.
    pub const fn is_byte(&self) -> bool {
        matches!(self, UnsignedInteger::Byte(_))
    }

    /// Returns `true` if the value is a 16-bit `UINT` / `WORD`.
    pub const fn is_uint(&self) -> bool {
        matches!(self, UnsignedInteger::UInt(_))
    }

    /// Returns `true` if the value is a 32-bit `UDINT` / `DWORD`.
    pub const fn is_udint(&self) -> bool {
        matches!(self, UnsignedInteger::UDInt(_))
    }

    /// Returns `true` if the value is a 64-bit `ULINT` / `LWORD`.
    pub const fn is_ulint(&self) -> bool {
        matches!(self, UnsignedInteger::ULInt(_))
    }
}

impl Serialize for UnsignedInteger {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            UnsignedInteger::Byte(n) => n.serialize(serializer),
            UnsignedInteger::UInt(n) => n.serialize(serializer),
            UnsignedInteger::UDInt(n) => n.serialize(serializer),
            UnsignedInteger::ULInt(n) => n.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for UnsignedInteger {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(UnsignedIntegerVisitor)
    }
}

impl fmt::Debug for UnsignedInteger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnsignedInteger::Byte(n) => write!(f, "Byte({n})"),
            UnsignedInteger::UInt(n) => write!(f, "UInt({n})"),
            UnsignedInteger::UDInt(n) => write!(f, "UDInt({n})"),
            UnsignedInteger::ULInt(n) => write!(f, "ULInt({n})"),
        }
    }
}

impl fmt::Display for UnsignedInteger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnsignedInteger::Byte(n) => write!(f, "{n}"),
            UnsignedInteger::UInt(n) => write!(f, "{n}"),
            UnsignedInteger::UDInt(n) => write!(f, "{n}"),
            UnsignedInteger::ULInt(n) => write!(f, "{n}"),
        }
    }
}

impl From<u8> for UnsignedInteger {
    fn from(n: u8) -> Self {
        UnsignedInteger::Byte(n)
    }
}

impl From<u16> for UnsignedInteger {
    fn from(n: u16) -> Self {
        UnsignedInteger::UInt(n)
    }
}

impl From<u32> for UnsignedInteger {
    fn from(n: u32) -> Self {
        UnsignedInteger::UDInt(n)
    }
}

impl From<u64> for UnsignedInteger {
    fn from(n: u64) -> Self {
        UnsignedInteger::ULInt(n)
    }
}

impl From<UnsignedInteger> for u64 {
    fn from(val: UnsignedInteger) -> u64 {
        match val {
            UnsignedInteger::Byte(n) => n as u64,
            UnsignedInteger::UInt(n) => n as u64,
            UnsignedInteger::UDInt(n) => n as u64,
            UnsignedInteger::ULInt(n) => n,
        }
    }
}

impl From<UnsignedInteger> for u32 {
    fn from(val: UnsignedInteger) -> u32 {
        match val {
            UnsignedInteger::Byte(n) => n as u32,
            UnsignedInteger::UInt(n) => n as u32,
            UnsignedInteger::UDInt(n) => n,
            UnsignedInteger::ULInt(n) => n as u32,
        }
    }
}

impl From<UnsignedInteger> for u16 {
    fn from(val: UnsignedInteger) -> u16 {
        match val {
            UnsignedInteger::Byte(n) => n as u16,
            UnsignedInteger::UInt(n) => n,
            UnsignedInteger::UDInt(n) => n as u16,
            UnsignedInteger::ULInt(n) => n as u16,
        }
    }
}

impl From<UnsignedInteger> for u8 {
    fn from(val: UnsignedInteger) -> u8 {
        match val {
            UnsignedInteger::Byte(n) => n,
            UnsignedInteger::UInt(n) => n as u8,
            UnsignedInteger::UDInt(n) => n as u8,
            UnsignedInteger::ULInt(n) => n as u8,
        }
    }
}

impl PartialEq for UnsignedInteger {
    fn eq(&self, other: &Self) -> bool {
        u64::from(*self) == u64::from(*other)
    }
}

impl Eq for UnsignedInteger {}

impl PartialOrd for UnsignedInteger {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for UnsignedInteger {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        u64::from(*self).cmp(&u64::from(*other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_less_than_ordering() {
        assert!(UnsignedInteger::Byte(1) < UnsignedInteger::Byte(2));
        assert!(UnsignedInteger::Byte(1) < UnsignedInteger::UInt(2));
        assert!(UnsignedInteger::Byte(1) < UnsignedInteger::UDInt(2));
        assert!(UnsignedInteger::Byte(1) < UnsignedInteger::ULInt(2));
        assert!(UnsignedInteger::UInt(1) < UnsignedInteger::Byte(2));
        assert!(UnsignedInteger::UInt(1) < UnsignedInteger::UInt(2));
        assert!(UnsignedInteger::UInt(1) < UnsignedInteger::UDInt(2));
        assert!(UnsignedInteger::UInt(1) < UnsignedInteger::ULInt(2));
        assert!(UnsignedInteger::UDInt(1) < UnsignedInteger::Byte(2));
        assert!(UnsignedInteger::UDInt(1) < UnsignedInteger::UInt(2));
        assert!(UnsignedInteger::UDInt(1) < UnsignedInteger::UDInt(2));
        assert!(UnsignedInteger::UDInt(1) < UnsignedInteger::ULInt(2));
        assert!(UnsignedInteger::ULInt(1) < UnsignedInteger::Byte(2));
        assert!(UnsignedInteger::ULInt(1) < UnsignedInteger::UInt(2));
        assert!(UnsignedInteger::ULInt(1) < UnsignedInteger::UDInt(2));
        assert!(UnsignedInteger::ULInt(1) < UnsignedInteger::ULInt(2));
    }

    #[test]
    fn test_greater_than_ordering() {
        assert!(UnsignedInteger::Byte(2) > UnsignedInteger::Byte(1));
        assert!(UnsignedInteger::Byte(2) > UnsignedInteger::UInt(1));
        assert!(UnsignedInteger::Byte(2) > UnsignedInteger::UDInt(1));
        assert!(UnsignedInteger::Byte(2) > UnsignedInteger::ULInt(1));
        assert!(UnsignedInteger::UInt(2) > UnsignedInteger::Byte(1));
        assert!(UnsignedInteger::UInt(2) > UnsignedInteger::UInt(1));
        assert!(UnsignedInteger::UInt(2) > UnsignedInteger::UDInt(1));
        assert!(UnsignedInteger::UInt(2) > UnsignedInteger::ULInt(1));
        assert!(UnsignedInteger::UDInt(2) > UnsignedInteger::Byte(1));
        assert!(UnsignedInteger::UDInt(2) > UnsignedInteger::UInt(1));
        assert!(UnsignedInteger::UDInt(2) > UnsignedInteger::UDInt(1));
        assert!(UnsignedInteger::UDInt(2) > UnsignedInteger::ULInt(1));
        assert!(UnsignedInteger::ULInt(2) > UnsignedInteger::Byte(1));
        assert!(UnsignedInteger::ULInt(2) > UnsignedInteger::UInt(1));
        assert!(UnsignedInteger::ULInt(2) > UnsignedInteger::UDInt(1));
        assert!(UnsignedInteger::ULInt(2) > UnsignedInteger::ULInt(1));
    }

    #[test]
    fn test_equality() {
        assert_eq!(UnsignedInteger::Byte(1), UnsignedInteger::Byte(1));
        assert_eq!(UnsignedInteger::UInt(1), UnsignedInteger::UInt(1));
        assert_eq!(UnsignedInteger::UDInt(1), UnsignedInteger::UDInt(1));
        assert_eq!(UnsignedInteger::ULInt(1), UnsignedInteger::ULInt(1));
    }
}
