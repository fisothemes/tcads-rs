use super::commands::CommandId;
use super::state_flags::StateFlag;
use crate::constants::{AMS_HEADER_LEN, AMS_TCP_HEADER_LEN};
use crate::errors::{AdsError, AdsReturnCode};
use crate::types::addr::AmsAddr;
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

    /// Writes the 6 bytes to a writer.
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

    pub fn write_to<W: Write>(&self, w: &mut W) -> io::Result<()> {
        todo!()
    }

    pub fn target(&self) -> &AmsAddr {
        &self.target
    }

    pub fn source(&self) -> &AmsAddr {
        &self.source
    }

    pub fn command_id(&self) -> CommandId {
        self.command_id
    }

    pub fn state_flags(&self) -> StateFlag {
        self.state_flags
    }

    pub fn length(&self) -> u32 {
        self.length
    }
    pub fn error_code(&self) -> AdsReturnCode {
        self.error_code
    }

    pub fn invoke_id(&self) -> u32 {
        self.invoke_id
    }
}

impl TryFrom<&[u8]> for AmsHeader {
    type Error = AdsError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        todo!()
    }
}
