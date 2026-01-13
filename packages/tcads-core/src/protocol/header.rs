use super::commands::CommandId;
use super::state_flags::StateFlag;
use crate::errors::{AdsError, AdsReturnCode};
use crate::types::addr::AmsAddr;
use std::io::{self, Write};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct AmsTcpHeader {
    reserved: u16,
    length: u32,
}

impl AmsTcpHeader {
    pub const fn new(length: u32) -> Self {
        Self {
            reserved: 0,
            length,
        }
    }

    pub const fn with_reserved(reserved: u16, length: u32) -> Self {
        Self { reserved, length }
    }

    pub fn length(&self) -> u32 {
        self.length
    }
}

impl TryFrom<&[u8]> for AmsTcpHeader {
    type Error = AdsError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        todo!()
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
