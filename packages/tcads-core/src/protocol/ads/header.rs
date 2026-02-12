use super::command::AdsCommand;
use super::return_codes::AdsReturnCode;
use super::state_flag::StateFlag;
use crate::ams::AmsAddr;

/// Length of the ADS Header (32 bytes)
pub const ADS_HEADER_LEN: usize = 32;

/// The ADS Packet Header structure (32 bytes).
///
/// This header follows the [AMS/TCP Header](crate::ams::AmsTcpHeader) in an ADS frame and contains
/// routing information, command IDs, flags, and error codes.
///
/// # Terminology
///
/// [Beckhoff documentation refers to this structure as the **AMS Header**](https://infosys.beckhoff.com/content/1033/tc3_ads_intro/115847307.html?id=7738940192708835096).
/// However, this library uses the term **ADS Header** to clearly distinguish it from the
/// TCP-level header and to emphasise its role in the ADS protocol layer.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsHeader {
    target: AmsAddr,
    source: AmsAddr,
    command_id: AdsCommand,
    state_flags: StateFlag,
    length: u32,
    error_code: AdsReturnCode,
    invoke_id: u32,
}

impl AdsHeader {
    /// Creates a new ADS Header.
    pub fn new(
        target: AmsAddr,
        source: AmsAddr,
        command_id: AdsCommand,
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

    /// The AMS address of the station, for which the packet is intended.
    pub fn target(&self) -> &AmsAddr {
        &self.target
    }

    /// the AMS address of the station, from which the packet was sent.
    pub fn source(&self) -> &AmsAddr {
        &self.source
    }

    /// The Command ID identifies the type of request/response.
    pub fn command_id(&self) -> AdsCommand {
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

    /// AMS error number. See [ADS Return Codes](AdsReturnCode).
    pub fn error_code(&self) -> AdsReturnCode {
        self.error_code
    }

    /// Free usable 32-bit array. Usually this array serves to send an ID.
    /// This ID makes it possible to assign a received response to a request.
    pub fn invoke_id(&self) -> u32 {
        self.invoke_id
    }
}
