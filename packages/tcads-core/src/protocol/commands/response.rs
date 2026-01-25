//! Definition of ADS Response Payloads

use std::io::{self, Read, Write};

use crate::errors::{AdsError, AdsReturnCode};
use crate::types::AdsString;

use super::{AdsState, NotificationHandle};

/// Payload Header for [`CommandId::AdsRead`](super::CommandId::AdsRead) (Response).
///
/// Direction: Server -> Client
///
/// A response from an ADS device to a read request.
/// See [`AdsReadRequest`](super::AdsReadRequest) for more information.
///
/// # Layout
/// - **Result:** 4 bytes (ADS Return Code)
/// - **Length:** 4 bytes (Size of the data that follows)
///
/// # Usage
/// This struct parses the *fixed header* of the response.
/// The actual read data immediately follows this structure in the stream.
///
/// ```text
/// [ Result (4) ] [ Length (4) ] [ Data (n bytes...) ]
/// ^---------------------------^
///  AdsReadResponse parses this
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdsReadResponse {
    result: AdsReturnCode,
    length: u32,
}

impl AdsReadResponse {
    /// Size of the fixed header of the response.
    pub const SIZE: usize = 8;

    /// Creates a new AdsReadResponse.
    pub fn new(result: AdsReturnCode, length: u32) -> Self {
        Self { result, length }
    }

    /// Returns ADS error code.
    pub fn result(&self) -> AdsReturnCode {
        self.result
    }

    /// Returns length of data which are supplied back.
    pub fn length(&self) -> u32 {
        self.length
    }

    /// Writes the fixed header of the response.
    pub fn write_to<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(&u32::from(self.result).to_le_bytes())?;
        w.write_all(&self.length.to_le_bytes())?;
        Ok(())
    }

    /// Reads the fixed header of the response.
    pub fn read_from<R: Read>(r: &mut R) -> io::Result<Self> {
        let mut buf = [0u8; 8];
        r.read_exact(&mut buf)?;
        Ok(Self {
            result: AdsReturnCode::from(u32::from_le_bytes(buf[0..4].try_into().unwrap())),
            length: u32::from_le_bytes(buf[4..8].try_into().unwrap()),
        })
    }
}

/// Payload for [`CommandId::AdsWrite`](super::CommandId::AdsWrite) (Response).
///
/// Direction: Server -> Client
///
/// A response from an ADS device the write request was made to.
/// See [`AdsWriteRequest`](super::AdsWriteRequest) for more information.
///
/// # Layout
/// - **Result:** 4 bytes (ADS Return Code)
///
/// ```text
/// [ Result (4) ]
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdsWriteResponse {
    result: AdsReturnCode,
}

impl AdsWriteResponse {
    /// Size of the fixed header of the response.
    pub const SIZE: usize = 4;

    pub fn new(result: AdsReturnCode) -> Self {
        Self { result }
    }

    /// Returns ADS error code.
    pub fn result(&self) -> AdsReturnCode {
        self.result
    }

    /// Writes the fixed header of the response.
    pub fn write_to<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(&u32::from(self.result).to_le_bytes())?;
        Ok(())
    }

    /// Reads the fixed header of the response.
    pub fn read_from<R: Read>(r: &mut R) -> io::Result<Self> {
        let mut buf = [0u8; 4];
        r.read_exact(&mut buf)?;
        Ok(Self {
            result: AdsReturnCode::from(u32::from_le_bytes(buf)),
        })
    }
}

/// Payload Header for [`CommandId::AdsReadWrite`](super::CommandId::AdsReadWrite) (Response).
///
/// Direction: Server -> Client
///
/// A response from an ADS device to a read-write request.
/// See [`AdsReadWriteRequest`](super::AdsReadWriteRequest) for more information.
///
/// # Layout
/// - **Result:** 4 bytes (ADS Return Code)
/// - **Length:** 4 bytes (Size of the read data that follows)
///
/// # Usage
/// This struct parses the *fixed header* of the response.
/// The actual read data immediately follows this structure.
///
/// ```text
/// [ Result (4) ] [ Length (4) ] [ Read Data (n bytes...) ]
/// ^---------------------------^
/// AdsReadWriteResponse parses this
/// ```
pub type AdsReadWriteResponse = AdsReadResponse;

/// Payload for [`CommandId::AdsReadState`](super::CommandId::AdsReadState) (Response).
///
/// Direction: Server -> Client
///
/// # Layout
/// - **Result:** 4 bytes
/// - **ADS State:** 2 bytes
/// - **Device State:** 2 bytes
///
/// ```text
/// [ Result (4) ] [ AdsState (2) ] [ DevState (2) ]
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdsReadStateResponse {
    result: AdsReturnCode,
    ads_state: AdsState,
    device_state: u16,
}

impl AdsReadStateResponse {
    /// Size of the fixed header of the response.
    pub const SIZE: usize = 8;

    pub fn new(result: AdsReturnCode, ads_state: AdsState, device_state: u16) -> Self {
        Self {
            result,
            ads_state,
            device_state,
        }
    }

    /// Returns ADS error code.
    pub fn result(&self) -> AdsReturnCode {
        self.result
    }

    /// Returns the ADS status of the device.
    pub fn ads_state(&self) -> AdsState {
        self.ads_state
    }

    /// Returns the device status of the device.
    ///
    /// # Note
    ///
    /// The documentation is extremely unclear about the meaning of this value.
    ///
    /// - **For a TwinCAT PLC:** It is almost always `0`.
    /// - **For Custom ADS Servers:** If you write your own ADS Server,
    ///   you can put whatever status flags you want in there
    ///   (e.g. bitmask for "Overheating", "Door Open").
    pub fn device_state(&self) -> u16 {
        self.device_state
    }

    /// Writes the fixed header of the response.
    pub fn write_to<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(&u32::from(self.result).to_le_bytes())?;
        w.write_all(&u16::from(self.ads_state).to_le_bytes())?;
        w.write_all(&self.device_state.to_le_bytes())?;
        Ok(())
    }

    /// Reads the fixed header of the response.
    pub fn read_from<R: Read>(r: &mut R) -> io::Result<Self> {
        let mut buf = [0u8; 8];
        r.read_exact(&mut buf)?;
        Ok(Self {
            result: AdsReturnCode::from(u32::from_le_bytes(buf[0..4].try_into().unwrap())),
            ads_state: AdsState::from(u16::from_le_bytes(buf[4..6].try_into().unwrap())),
            device_state: u16::from_le_bytes(buf[6..8].try_into().unwrap()),
        })
    }
}

/// Payload for [`CommandId::AdsWriteControl`](super::CommandId::AdsWriteControl) (Response).
///
/// Direction: Server -> Client
///
/// A response from an ADS device to a write-control request.
/// See [`AdsWriteControlRequest`](super::AdsWriteControlRequest) for more information.
///
/// # Layout
/// - **Result:** 4 bytes (ADS Return Code)
///
/// ```text
/// [ Result (4) ]
/// ```
pub type AdsWriteControlResponse = AdsWriteResponse;

/// Response for [`CommandId::AdsReadDeviceInfo`](super::CommandId::AdsReadDeviceInfo).
///
/// Direction: Server -> Client
///
/// # Layout
/// - **Result:** 4 bytes
/// - **Major Version:** 1 byte
/// - **Minor Version:** 1 byte
/// - **Build:** 2 bytes
/// - **Device Name:** 16 bytes (Fixed char array)
///
/// ```text
/// [ Result (4) ] [ Maj (1) ] [ Min (1) ] [ Build (2) ] [ Device Name (16) ]
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdsDeviceInfoResponse {
    result: AdsReturnCode,
    major_version: u8,
    minor_version: u8,
    version_build: u16,
    device_name: AdsString<16>,
}

impl AdsDeviceInfoResponse {
    /// Size of the fixed header of the response.
    pub const SIZE: usize = 24;

    pub fn new(
        result: AdsReturnCode,
        major: u8,
        minor: u8,
        build: u16,
        name: AdsString<16>,
    ) -> Self {
        Self {
            result,
            major_version: major,
            minor_version: minor,
            version_build: build,
            device_name: name,
        }
    }

    /// Returns ADS error code.
    pub fn result(&self) -> AdsReturnCode {
        self.result
    }

    /// Returns the major version number of the ADS device.
    pub fn major_version(&self) -> u8 {
        self.major_version
    }

    /// Returns the minor version number of the ADS device.
    pub fn minor_version(&self) -> u8 {
        self.minor_version
    }

    /// Returns the build number of the ADS device.
    pub fn version_build(&self) -> u16 {
        self.version_build
    }

    /// Returns the device name of the ADS device.
    pub fn device_name(&self) -> &AdsString<16> {
        &self.device_name
    }

    /// Writes the fixed header of the response.
    pub fn write_to<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(&u32::from(self.result).to_le_bytes())?;
        w.write_all(&self.major_version.to_le_bytes())?;
        w.write_all(&self.minor_version.to_le_bytes())?;
        w.write_all(&self.version_build.to_le_bytes())?;
        w.write_all(self.device_name.as_bytes())?;
        Ok(())
    }

    /// Reads the fixed header of the response.
    pub fn read_from<R: Read>(r: &mut R) -> Result<Self, AdsError> {
        let mut buf = [0u8; 24];
        r.read_exact(&mut buf)?;

        let result = AdsReturnCode::from(u32::from_le_bytes(buf[0..4].try_into().unwrap()));
        let major = buf[4];
        let minor = buf[5];
        let build = u16::from_le_bytes(buf[6..8].try_into().unwrap());
        let name_bytes: [u8; 16] = buf[8..24].try_into().unwrap();

        Ok(Self {
            result,
            major_version: major,
            minor_version: minor,
            version_build: build,
            device_name: AdsString::from(name_bytes),
        })
    }
}

/// Payload for [`CommandId::AdsAddDeviceNotification`](super::CommandId::AdsAddDeviceNotification) (Response).
///
/// Direction: Server -> Client
///
/// A response from an ADS device to an added notification request.
/// See [`AdsAddDeviceNotificationRequest`](super::AdsAddDeviceNotificationRequest) for more information.
///
/// # Layout
/// - **Result:** 4 bytes
/// - **Notification Handle:** 4 bytes
///
/// ```text
/// [ Result (4) ] [ Handle (4) ]
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdsAddDeviceNotificationResponse {
    result: AdsReturnCode,
    handle: NotificationHandle,
}

impl AdsAddDeviceNotificationResponse {
    /// Size of the fixed header of the response.
    pub const SIZE: usize = 8;

    pub fn new(result: AdsReturnCode, handle: NotificationHandle) -> Self {
        Self { result, handle }
    }

    /// Returns ADS error code.
    pub fn result(&self) -> AdsReturnCode {
        self.result
    }

    /// Returns the notification handle of the device notification.
    pub fn handle(&self) -> NotificationHandle {
        self.handle
    }

    /// Writes the fixed header of the response.
    pub fn write_to<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(&u32::from(self.result).to_le_bytes())?;
        w.write_all(&self.handle.as_u32().to_le_bytes())?;
        Ok(())
    }

    /// Reads the fixed header of the response.
    pub fn read_from<R: Read>(r: &mut R) -> io::Result<Self> {
        let mut buf = [0u8; 8];
        r.read_exact(&mut buf)?;
        Ok(Self {
            result: AdsReturnCode::from(u32::from_le_bytes(buf[0..4].try_into().unwrap())),
            handle: NotificationHandle::new(u32::from_le_bytes(buf[4..8].try_into().unwrap())),
        })
    }
}

/// Payload for [`CommandId::AdsDeleteDeviceNotification`](super::CommandId::AdsDeleteDeviceNotification) (Response).
///
/// Direction: Server -> Client
///
/// A response from an ADS device to a deleted notification request.
/// See [`AdsDeleteDeviceNotificationRequest`](super::AdsDeleteDeviceNotificationRequest) for more information.
///
/// # Layout
/// - **Result:** 4 bytes
///
/// ```text
/// [ Result (4) ]
/// ```
pub type AdsDeleteDeviceNotificationResponse = AdsWriteResponse;
