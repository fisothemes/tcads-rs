use super::commands::CommandId;
use super::state_flags::StateFlag;
use crate::constants::{AMS_HEADER_LEN, AMS_TCP_HEADER_LEN};
use crate::errors::{AdsError, AdsReturnCode};
use crate::types::addr::{AmsAddr, AmsPort};
use crate::types::netid::AmsNetId;
use std::io::{self, Write};

/// The 6-byte prefix for TCP communication.
///
/// Contains the total length of the AMS packet (AMS Header + ADS Data).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct AmsTcpHeader {
    reserved: u16,
    length: u32,
}

impl AmsTcpHeader {
    /// Creates a new TCP header.
    ///
    /// `length` is the size of the AMS Packet (AMS Header + ADS Data).
    pub const fn new(length: u32) -> Self {
        Self {
            reserved: 0,
            length,
        }
    }

    /// Creates a new TCP header with the given reserved and length fields.
    pub const fn with_reserved(reserved: u16, length: u32) -> Self {
        Self { reserved, length }
    }

    /// The reserved field is currently unused and must be set to zero.
    pub fn reserved(&self) -> u16 {
        self.reserved
    }

    /// The size of the AMS Packet (AMS Header + ADS Data).
    pub fn length(&self) -> u32 {
        self.length
    }

    /// Writes the 6 bytes to a writer (Little Endian).
    pub fn write_to<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(&self.reserved.to_le_bytes())?;
        w.write_all(&self.length.to_le_bytes())
    }
}

impl TryFrom<&[u8]> for AmsTcpHeader {
    type Error = AdsError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() < AMS_TCP_HEADER_LEN {
            return Err(AdsError::MalformedPacket("TCP Header data too short"));
        }

        Ok(
            // Safely unwraps because length was checked.
            Self {
                reserved: u16::from_le_bytes(value[0..2].try_into().unwrap()),
                length: u32::from_le_bytes(value[2..6].try_into().unwrap()),
            },
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AmsHeader {
    target: AmsAddr,
    source: AmsAddr,
    command_id: CommandId,
    state_flags: StateFlag,
    length: u32,
    error_code: AdsReturnCode,
    invoke_id: u32,
}

impl AmsHeader {
    pub fn new(
        target: AmsAddr,
        source: AmsAddr,
        command_id: CommandId,
        state_flags: StateFlag,
        length: u32,
        error_code: AdsReturnCode,
        invoke_id: u32,
    ) -> Self {
        Self {
            target,
            source,
            command_id,
            state_flags,
            length,
            error_code,
            invoke_id,
        }
    }

    /// Writes the 32 bytes to a writer using Little Endian.
    pub fn write_to<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(&self.target.net_id().0)?;
        w.write_all(&self.target.port().to_le_bytes())?;

        w.write_all(&self.source.net_id().0)?;
        w.write_all(&self.source.port().to_le_bytes())?;

        w.write_all(&u16::from(self.command_id).to_le_bytes())?;

        w.write_all(&u16::from(self.state_flags).to_le_bytes())?;

        w.write_all(&self.length.to_le_bytes())?;

        w.write_all(&u32::from(self.error_code).to_le_bytes())?;

        w.write_all(&self.invoke_id.to_le_bytes())?;

        Ok(())
    }

    /// The AMSNetId of the station, for which the packet is intended.
    pub fn target(&self) -> &AmsAddr {
        &self.target
    }

    /// the AMSNetId of the station, from which the packet was sent.
    pub fn source(&self) -> &AmsAddr {
        &self.source
    }

    /// The Command ID identifies the type of request/response.
    pub fn command_id(&self) -> CommandId {
        self.command_id
    }

    /// State flags (Request/Response, TCP/UDP).
    pub fn state_flags(&self) -> StateFlag {
        self.state_flags
    }

    /// Size of the data range in bytes.
    pub fn length(&self) -> u32 {
        self.length
    }

    /// AMS error number. See ADS Return Codes.
    pub fn error_code(&self) -> AdsReturnCode {
        self.error_code
    }

    /// Free usable 32-bit array. Usually this array serves to send an ID.
    /// This ID makes it possible to assign a received response to a request.
    pub fn invoke_id(&self) -> u32 {
        self.invoke_id
    }
}

impl TryFrom<&[u8]> for AmsHeader {
    type Error = AdsError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() < AMS_HEADER_LEN {
            return Err(AdsError::MalformedPacket(
                "AMS Header data too short (expected 32 bytes)",
            ));
        }

        Ok(Self {
            target: AmsAddr::new(
                AmsNetId::try_from(&value[0..6])?,
                AmsPort::from_le_bytes(value[6..8].try_into().unwrap()),
            ),
            source: AmsAddr::new(
                AmsNetId::try_from(&value[8..14])?,
                AmsPort::from_le_bytes(value[14..16].try_into().unwrap()),
            ),
            command_id: CommandId::from(u16::from_le_bytes(value[16..18].try_into().unwrap())),
            state_flags: StateFlag::from(u16::from_le_bytes(value[18..20].try_into().unwrap())),
            length: u32::from_le_bytes(value[20..24].try_into().unwrap()),
            error_code: AdsReturnCode::from(u32::from_le_bytes(value[24..28].try_into().unwrap())),
            invoke_id: u32::from_le_bytes(value[28..32].try_into().unwrap()),
        })
    }
}
