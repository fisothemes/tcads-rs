//! Definition of ADS Response Payloads

use std::io::{self, Read, Write};

use crate::errors::AdsReturnCode;
use crate::types::enums::AdsState;

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
