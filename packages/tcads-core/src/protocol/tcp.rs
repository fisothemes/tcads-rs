use crate::constants::{
    AMS_TCP_HEADER_LEN, AMS_TCP_HEADER_LENGTH_RANGE, AMS_TCP_HEADER_RESERVED_RANGE,
};
use crate::errors::AdsError;
use crate::protocol::router::AmsRouterCommand;
use std::io;
use std::io::{Read, Write};
use std::sync::Arc;

/// The 6-byte prefix for TCP communication.
///
/// Contains the total length of the AMS packet (AMS Header + ADS Data).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct AmsTcpHeader {
    pub command: AmsRouterCommand,
    pub length: u32,
}

impl AmsTcpHeader {
    pub fn new(command: AmsRouterCommand, length: u32) -> Self {
        Self { command, length }
    }

    /// Writes the 6-byte header to the writer.
    pub fn write_to<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(&u16::from(self.command).to_le_bytes())?;
        w.write_all(&self.length.to_le_bytes())
    }

    /// Reads exactly 6 bytes from the reader and parses the header.
    pub fn read_from<R: Read>(r: &mut R) -> io::Result<Self> {
        let mut buf = [0u8; AMS_TCP_HEADER_LEN];
        r.read_exact(&mut buf)?;
        Ok(Self::from(&buf))
    }
}

impl TryFrom<&[u8]> for AmsTcpHeader {
    type Error = AdsError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() < AMS_TCP_HEADER_LEN {
            return Err(AdsError::MalformedPacket(Arc::from(format!(
                "TCP Header data too short. Received {} bytes, expected {AMS_TCP_HEADER_LEN} bytes.",
                value.len()
            ))));
        }

        Ok(
            // Safely unwraps because length was checked.
            Self {
                command: AmsRouterCommand::from(u16::from_le_bytes(
                    value[AMS_TCP_HEADER_RESERVED_RANGE].try_into().unwrap(),
                )),
                length: u32::from_le_bytes(value[AMS_TCP_HEADER_LENGTH_RANGE].try_into().unwrap()),
            },
        )
    }
}

impl From<&[u8; AMS_TCP_HEADER_LEN]> for AmsTcpHeader {
    fn from(value: &[u8; AMS_TCP_HEADER_LEN]) -> Self {
        Self {
            command: AmsRouterCommand::from(u16::from_le_bytes(
                value[AMS_TCP_HEADER_RESERVED_RANGE].try_into().unwrap(),
            )),
            length: u32::from_le_bytes(value[AMS_TCP_HEADER_LENGTH_RANGE].try_into().unwrap()),
        }
    }
}
