//! Definition of ADS Response Payloads

use std::io::{self, Read, Write};

use crate::errors::AdsReturnCode;

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
