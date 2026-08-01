use super::device_state::{AdsState, DeviceState};
use super::device_version::AdsDeviceVersion;
use super::error::AdsSystemStateError;
use super::system_state_flags::AdsSystemStateFlags;

/// Overall runtime status returned by a
/// [`SYSTEM_SERVICE_STATE`](super::IndexGroup::SYSTEM_SERVICE_STATE) read.
///
/// # Wire Format
///
/// | Offset | Size | Field          | Description                                  |
/// |--------|------|----------------|-----------------------------------------------|
/// | 0      | 2    | `ads_state`    | [`AdsState`]                                  |
/// | 2      | 2    | `device_state` | [`DeviceState`]                               |
/// | 4      | 2    | `restart_index`| Number of times the runtime restarted         |
/// | 6      | 1    | `version`      | Major version (part of [`AdsDeviceVersion`])  |
/// | 7      | 1    | `version`      | Revision (part of [`AdsDeviceVersion`])       |
/// | 8      | 2    | `version`      | Build (part of [`AdsDeviceVersion`])          |
/// | 10     | 1    | `platform`     | [`AdsPlatform`]                               |
/// | 11     | 1    | `os_type`      | [`AdsOsType`]                                 |
/// | 12     | 2    | `flags`        | [`AdsSystemStateFlags`]                       |
/// | 14     | 2    | `reserved`     | Reserved, currently unused                    |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AdsSystemState {
    ads_state: AdsState,
    device_state: DeviceState,
    restart_index: u16,
    version: AdsDeviceVersion,
    platform: AdsPlatform,
    os_type: AdsOsType,
    flags: AdsSystemStateFlags,
    reserved: u16,
}

impl AdsSystemState {
    /// Wire length of an [`AdsSystemState`] in bytes.
    pub const LENGTH: usize = 16;

    /// Returns the ADS state (e.g. `Run`, `Stop`, `Config`).
    pub fn ads_state(&self) -> AdsState {
        self.ads_state
    }

    /// Returns the device-specific state.
    pub fn device_state(&self) -> DeviceState {
        self.device_state
    }

    /// Returns the number of times the runtime has restarted.
    pub fn restart_index(&self) -> u16 {
        self.restart_index
    }

    /// Returns the target system's version (major, revision, build).
    pub fn version(&self) -> AdsDeviceVersion {
        self.version
    }

    /// Returns the hardware/CPU platform.
    pub fn platform(&self) -> AdsPlatform {
        self.platform
    }

    /// Returns the operating system category.
    pub fn os_type(&self) -> AdsOsType {
        self.os_type
    }

    /// Returns the System Service flags.
    pub fn flags(&self) -> AdsSystemStateFlags {
        self.flags
    }

    /// Returns the raw reserved field. Currently always `0`; exposed for
    /// forward-compatibility/debugging only.
    pub fn reserved(&self) -> u16 {
        self.reserved
    }

    /// Tries to parse an [`AdsSystemState`] from a byte slice.
    pub fn try_from_slice(data: &[u8]) -> Result<Self, AdsSystemStateError> {
        if data.len() < Self::LENGTH {
            return Err(AdsSystemStateError::TooShort {
                expected: Self::LENGTH,
                got: data.len(),
            });
        }

        Ok(Self {
            ads_state: AdsState::from_bytes([data[0], data[1]]),
            device_state: DeviceState::from_le_bytes([data[2], data[3]]),
            restart_index: u16::from_le_bytes([data[4], data[5]]),
            version: AdsDeviceVersion::from_bytes([data[6], data[7], data[8], data[9]]),
            platform: AdsPlatform::from(data[10]),
            os_type: AdsOsType::from(data[11]),
            flags: AdsSystemStateFlags::from_bytes([data[12], data[13]]),
            reserved: u16::from_le_bytes([data[14], data[15]]),
        })
    }
}

impl TryFrom<&[u8]> for AdsSystemState {
    type Error = AdsSystemStateError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        Self::try_from_slice(data)
    }
}

/// Hardware/CPU platform identifier reported by [`AdsSystemState::platform`].
#[derive(
    serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub enum AdsPlatform {
    X86,
    X64,
    ArmV7,
    ArmT2,
    /// A platform ID not defined in the library.
    Unknown(u8),
}

impl From<u8> for AdsPlatform {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::X86,
            1 => Self::X64,
            2 => Self::ArmV7,
            3 => Self::ArmT2,
            n => Self::Unknown(n),
        }
    }
}

impl From<AdsPlatform> for u8 {
    fn from(value: AdsPlatform) -> Self {
        match value {
            AdsPlatform::X86 => 0,
            AdsPlatform::X64 => 1,
            AdsPlatform::ArmV7 => 2,
            AdsPlatform::ArmT2 => 3,
            AdsPlatform::Unknown(n) => n,
        }
    }
}

/// Operating system category reported by [`AdsSystemState::os_type`].
#[derive(
    serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub enum AdsOsType {
    /// Windows.
    Windows,
    /// Windows CE.
    WindowsCe,
    /// A user-mode runtime (UmRT).
    UserMode,
    /// TwinCAT/BSD or another TwinCAT-managed OS image.
    TwinCat,
    /// An OS type not defined in the library.
    Unknown(u8),
}

impl From<u8> for AdsOsType {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Windows,
            1 => Self::WindowsCe,
            2 => Self::UserMode,
            3 => Self::TwinCat,
            n => Self::Unknown(n),
        }
    }
}

impl From<AdsOsType> for u8 {
    fn from(value: AdsOsType) -> Self {
        match value {
            AdsOsType::Windows => 0,
            AdsOsType::WindowsCe => 1,
            AdsOsType::UserMode => 2,
            AdsOsType::TwinCat => 3,
            AdsOsType::Unknown(n) => n,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_all_fields() {
        let mut data = Vec::new();
        data.extend_from_slice(&5u16.to_le_bytes()); // ads_state = Run
        data.extend_from_slice(&0u16.to_le_bytes()); // device_state
        data.extend_from_slice(&3u16.to_le_bytes()); // restart_index
        data.extend_from_slice(&[3, 1, 0x39, 0x05]); // version 3.1.1337
        data.push(1); // platform = X64
        data.push(2); // os_type = UserMode
        data.extend_from_slice(&0x0011u16.to_le_bytes()); // flags: ROUTER_MODE_ONLY | REDUNDANCY_ACTIVE
        data.extend_from_slice(&0u16.to_le_bytes()); // reserved

        let state = AdsSystemState::try_from_slice(&data).unwrap();

        assert_eq!(state.ads_state(), AdsState::Run);
        assert_eq!(state.device_state(), 0);
        assert_eq!(state.restart_index(), 3);
        assert_eq!(state.version(), AdsDeviceVersion::new(3, 1, 1337));
        assert_eq!(state.platform(), AdsPlatform::X64);
        assert_eq!(state.os_type(), AdsOsType::UserMode);
        assert!(state.flags().is_router_mode_only());
        assert!(state.flags().is_redundancy_active());
        assert_eq!(state.reserved(), 0);
    }

    #[test]
    fn rejects_short_payload() {
        let err = AdsSystemState::try_from_slice(&[0u8; 10]).unwrap_err();
        assert!(matches!(
            err,
            AdsSystemStateError::TooShort {
                expected: 16,
                got: 10
            }
        ));
    }
}
