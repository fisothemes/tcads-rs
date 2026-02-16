use super::ProtocolError;
use crate::ads::{AdsCommand, AdsError, AdsHeader, AdsReturnCode, StateFlag, StateFlagError};
use crate::ams::{AmsAddr, AmsCommand};
use crate::io::AmsFrame;

/// Represents an ADS Read Device Info Request (Command `0x0001`).
///
/// This command is sent to an ADS device to query its name and version information.
///
/// # Usage
/// * **Client:** Sends this to identify the target device (e.g. "Plc30 App").
/// * **Server:** Receives this and responds with [`AdsReadDeviceInfoResponse`].
///
/// # Protocol Details
/// * **AMS Command:** [`AdsCommand`](AmsCommand::AdsCommand) (`0x0000`)
/// * **ADS Command:** [`AdsReadDeviceInfo`](AdsCommand::AdsReadDeviceInfo) (`0x0001`)
/// * **ADS Payload Length:** 0 bytes (Body is empty)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsReadDeviceInfoRequest {
    header: AdsHeader,
}

impl AdsReadDeviceInfoRequest {
    /// Creates a new Read Device Info Request over TCP.
    pub fn new(target: AmsAddr, source: AmsAddr, invoke_id: u32) -> Self {
        let header = AdsHeader::new(
            target,
            source,
            AdsCommand::AdsReadDeviceInfo,
            StateFlag::tcp_ads_request(),
            0,
            AdsReturnCode::Ok,
            invoke_id,
        );

        Self { header }
    }

    /// Creates a new Read Device Info Request over UDP.
    pub fn new_udp(target: AmsAddr, source: AmsAddr, invoke_id: u32) -> Self {
        let header = AdsHeader::new(
            target,
            source,
            AdsCommand::AdsReadDeviceInfo,
            StateFlag::udp_ads_request(),
            0,
            AdsReturnCode::Ok,
            invoke_id,
        );

        Self { header }
    }

    /// Returns the ADS header.
    pub fn header(&self) -> &AdsHeader {
        &self.header
    }

    /// Tries to parse a request from an AMS Frame.
    pub fn try_from_frame(frame: &AmsFrame) -> Result<Self, ProtocolError> {
        Self::try_from(frame)
    }

    /// Consumes the request and converts it into an AMS Frame.
    pub fn into_frame(self) -> AmsFrame {
        AmsFrame::from(self)
    }

    /// Serializes the request into an AMS Frame.
    pub fn to_frame(&self) -> AmsFrame {
        AmsFrame::from(self)
    }
}

impl From<&AdsReadDeviceInfoRequest> for AmsFrame {
    fn from(request: &AdsReadDeviceInfoRequest) -> Self {
        AmsFrame::new(AmsCommand::AdsCommand, request.header.to_bytes())
    }
}

impl From<AdsReadDeviceInfoRequest> for AmsFrame {
    fn from(request: AdsReadDeviceInfoRequest) -> Self {
        AmsFrame::from(&request)
    }
}

impl TryFrom<&AmsFrame> for AdsReadDeviceInfoRequest {
    type Error = ProtocolError;

    fn try_from(value: &AmsFrame) -> Result<Self, Self::Error> {
        let header = value.header();

        if header.command() != AmsCommand::AdsCommand {
            return Err(ProtocolError::UnexpectedAmsCommand {
                expected: AmsCommand::AdsCommand,
                got: header.command(),
            });
        }

        let (header, data) = AdsHeader::parse_prefix(value.payload()).map_err(AdsError::from)?;

        if header.command_id() != AdsCommand::AdsReadDeviceInfo {
            return Err(ProtocolError::UnexpectedAdsCommand {
                expected: AdsCommand::AdsReadDeviceInfo,
                got: header.command_id(),
            });
        }

        if !header.state_flags().is_request() {
            return Err(StateFlagError::UnexpectedStateFlag {
                expected: vec![StateFlag::tcp_ads_request(), StateFlag::udp_ads_request()],
                got: header.state_flags(),
            })
            .map_err(AdsError::from)?;
        }

        if !data.is_empty() {
            return Err(AdsError::UnexpectedDataLength {
                expected: 0,
                got: data.len(),
            })?;
        }

        Ok(Self { header })
    }
}
