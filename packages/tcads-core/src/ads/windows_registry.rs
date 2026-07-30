use std::array::TryFromSliceError;

/// Windows Registry Value Types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WinRegistryValueType {
    /// No defined value type (REG_NONE) (0)
    None,
    /// A null-terminated string (REG_SZ) (1)
    String,
    /// A null-terminated string that contains unexpanded references to environment variables (REG_EXPAND_SZ) (2)
    ExpandString,
    /// Binary data in any form (REG_BINARY) (3)
    Binary,
    /// A 32-bit number in little-endian format (REG_DWORD) (4)
    DWord,
    /// A 32-bit number in big-endian format (REG_DWORD_BIG_ENDIAN) (5)
    DWordBigEndian,
    /// A null-terminated string that contains the target path of a symbolic link (REG_LINK) (6)
    Link,
    /// A sequence of null-terminated strings, terminated by an empty string (REG_MULTI_SZ) (7)
    MultiString,
    /// A series of nested arrays designed to store a resource list (REG_RESOURCE_LIST) (8)
    ResourceList,
    /// A series of nested arrays designed to store a resource descriptor (REG_FULL_RESOURCE_DESCRIPTOR) (9)
    FullResourceDescriptor,
    /// A series of nested arrays designed to store a resource requirements list (REG_RESOURCE_REQUIREMENTS_LIST) (10)
    ResourceRequirementsList,
    /// A 64-bit number in little-endian format (REG_QWORD) (11)
    QWord,
    /// A registry value type not natively modeled by this library.
    Other(u32),
}

impl WinRegistryValueType {
    /// The length of the Registry Value Type ID in bytes.
    pub const LENGTH: usize = 4;

    /// Creates a new [`WinRegistryValueType`] from a 4-byte array (Little Endian).
    pub fn from_bytes(bytes: [u8; Self::LENGTH]) -> Self {
        Self::from(bytes)
    }

    /// Converts the registry value type to a 4-byte array (Little Endian).
    pub fn to_bytes(&self) -> [u8; Self::LENGTH] {
        (*self).into()
    }

    /// Tries to parse a `RegistryValueType` from a byte slice.
    pub fn try_from_slice(bytes: &[u8]) -> Result<Self, TryFromSliceError> {
        bytes.try_into()
    }
}

impl From<u32> for WinRegistryValueType {
    fn from(value: u32) -> Self {
        match value {
            0 => Self::None,
            1 => Self::String,
            2 => Self::ExpandString,
            3 => Self::Binary,
            4 => Self::DWord,
            5 => Self::DWordBigEndian,
            6 => Self::Link,
            7 => Self::MultiString,
            8 => Self::ResourceList,
            9 => Self::FullResourceDescriptor,
            10 => Self::ResourceRequirementsList,
            11 => Self::QWord,
            n => Self::Other(n),
        }
    }
}

impl From<WinRegistryValueType> for u32 {
    fn from(value: WinRegistryValueType) -> Self {
        match value {
            WinRegistryValueType::None => 0,
            WinRegistryValueType::String => 1,
            WinRegistryValueType::ExpandString => 2,
            WinRegistryValueType::Binary => 3,
            WinRegistryValueType::DWord => 4,
            WinRegistryValueType::DWordBigEndian => 5,
            WinRegistryValueType::Link => 6,
            WinRegistryValueType::MultiString => 7,
            WinRegistryValueType::ResourceList => 8,
            WinRegistryValueType::FullResourceDescriptor => 9,
            WinRegistryValueType::ResourceRequirementsList => 10,
            WinRegistryValueType::QWord => 11,
            WinRegistryValueType::Other(n) => n,
        }
    }
}

impl From<[u8; Self::LENGTH]> for WinRegistryValueType {
    fn from(bytes: [u8; Self::LENGTH]) -> Self {
        u32::from_le_bytes(bytes).into()
    }
}

impl From<WinRegistryValueType> for [u8; WinRegistryValueType::LENGTH] {
    fn from(reg_type: WinRegistryValueType) -> Self {
        u32::from(reg_type).to_le_bytes()
    }
}

impl TryFrom<&[u8]> for WinRegistryValueType {
    type Error = TryFromSliceError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let bytes: [u8; WinRegistryValueType::LENGTH] = bytes.try_into()?;
        Ok(Self::from_bytes(bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_type_conversion() {
        assert_eq!(WinRegistryValueType::from(1), WinRegistryValueType::String);
        assert_eq!(WinRegistryValueType::from(11), WinRegistryValueType::QWord);
        assert_eq!(
            WinRegistryValueType::from(0xFF),
            WinRegistryValueType::Other(0xFF)
        );
        assert_eq!(WinRegistryValueType::from(0), WinRegistryValueType::None);
    }

    #[test]
    fn test_registry_type_from_u32() {
        assert_eq!(u32::from(WinRegistryValueType::String), 1);
        assert_eq!(u32::from(WinRegistryValueType::QWord), 11);
        assert_eq!(u32::from(WinRegistryValueType::Other(12345)), 12345);
    }

    #[test]
    fn test_registry_type_ord() {
        assert!(WinRegistryValueType::String < WinRegistryValueType::DWord);
    }

    #[test]
    fn test_registry_type_bytes() {
        // 4 bytes for u32 representation of REG_DWORD (4)
        assert_eq!(
            WinRegistryValueType::DWord.to_bytes(),
            [0x04, 0x00, 0x00, 0x00]
        );
    }

    #[test]
    fn test_registry_type_from_bytes() {
        assert_eq!(
            WinRegistryValueType::from_bytes([0x04, 0x00, 0x00, 0x00]),
            WinRegistryValueType::DWord
        );
    }

    #[test]
    fn test_registry_type_try_from_slice() {
        assert_eq!(
            WinRegistryValueType::try_from_slice(&[0x04, 0x00, 0x00, 0x00]).unwrap(),
            WinRegistryValueType::DWord
        );
    }
}
