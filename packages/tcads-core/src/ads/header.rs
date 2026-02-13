use super::command::AdsCommand;
use super::error::AdsHeaderError;
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

    /// Converts the current instance into a byte array.
    pub fn to_bytes(&self) -> [u8; ADS_HEADER_LEN] {
        self.into()
    }

    /// Creates a new AdsHeader from a byte array.
    pub fn from_bytes(bytes: [u8; ADS_HEADER_LEN]) -> Self {
        Self::from(bytes)
    }

    /// Tries to parse an `AdsHeader` from a byte slice.
    pub fn try_from_slice(bytes: &[u8]) -> Result<Self, AdsHeaderError> {
        bytes.try_into()
    }
}

impl From<&AdsHeader> for [u8; ADS_HEADER_LEN] {
    fn from(value: &AdsHeader) -> Self {
        let mut buf = [0u8; ADS_HEADER_LEN];

        buf[0..8].copy_from_slice(&value.target.to_bytes());
        buf[8..16].copy_from_slice(&value.source.to_bytes());
        buf[16..18].copy_from_slice(&value.command_id.to_bytes());
        buf[18..20].copy_from_slice(&value.state_flags.to_bytes());
        buf[20..24].copy_from_slice(&value.length.to_le_bytes());
        buf[24..28].copy_from_slice(&value.error_code.to_bytes());
        buf[28..32].copy_from_slice(&value.invoke_id.to_le_bytes());

        buf
    }
}

impl From<AdsHeader> for [u8; ADS_HEADER_LEN] {
    fn from(value: AdsHeader) -> Self {
        (&value).into()
    }
}

impl From<&[u8; ADS_HEADER_LEN]> for AdsHeader {
    fn from(value: &[u8; ADS_HEADER_LEN]) -> Self {
        let target = AmsAddr::from_bytes(value[0..8].try_into().unwrap());
        let source = AmsAddr::from_bytes(value[8..16].try_into().unwrap());
        let command_id = AdsCommand::from_bytes(value[16..18].try_into().unwrap());
        let state_flags = StateFlag::from_bytes(value[18..20].try_into().unwrap());
        let length = u32::from_le_bytes(value[20..24].try_into().unwrap());
        let error_code = AdsReturnCode::from_bytes(value[24..28].try_into().unwrap());
        let invoke_id = u32::from_le_bytes(value[28..32].try_into().unwrap());

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
}

impl From<[u8; ADS_HEADER_LEN]> for AdsHeader {
    fn from(value: [u8; ADS_HEADER_LEN]) -> Self {
        (&value).into()
    }
}

impl TryFrom<&[u8]> for AdsHeader {
    type Error = AdsHeaderError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() != ADS_HEADER_LEN {
            return Err(AdsHeaderError::UnexpectedLength {
                expected: ADS_HEADER_LEN,
                got: value.len(),
            });
        }
        Ok(Self::from(&value[0..ADS_HEADER_LEN].try_into().unwrap()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ams::AmsNetId;

    #[test]
    fn test_roundtrip_serialization() {
        let target = AmsAddr::new(AmsNetId::new(192, 168, 0, 1, 1, 1), 851);
        let source = AmsAddr::new(AmsNetId::new(10, 10, 10, 10, 1, 1), 30000);

        let header = AdsHeader::new(
            target,
            source,
            AdsCommand::AdsRead,
            StateFlag::tcp_ads_request(),
            4,
            AdsReturnCode::Ok,
            12345,
        );

        let bytes = header.to_bytes();
        let parsed = AdsHeader::from_bytes(bytes);
        let parsed_slice = AdsHeader::try_from_slice(&bytes[..ADS_HEADER_LEN]).unwrap();

        assert_eq!(header, parsed);
        assert_eq!(parsed.command_id(), AdsCommand::AdsRead);
        assert_eq!(parsed.invoke_id(), 12345);
        assert_eq!(parsed_slice, parsed);
    }
}
