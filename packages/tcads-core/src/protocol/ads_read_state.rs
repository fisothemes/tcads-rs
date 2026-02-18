use super::ProtocolError;
use crate::ads::{
    AdsCommand, AdsError, AdsHeader, AdsReturnCode, AdsState, DeviceState, StateFlag,
    StateFlagError,
};
use crate::ams::{AmsAddr, AmsCommand};
use crate::io::AmsFrame;

/// Represents an ADS Read State Request (Command `0x0004`).
///
/// This command is sent to an ADS device to query its current ADS status and device state.
///
/// # Usage
/// * **Client:** Sends this to a target to check if it is in `Run`, `Stop`, `Config`, etc. mode.
/// * **Server:** Receives this and responds with its current [`AdsState`] and device-specific state.
///
/// # Protocol Details
/// * **AMS Command:** [`AdsCommand`](AmsCommand::AdsCommand) (`0x0000`)
/// * **ADS Command:** [`AdsReadState`](AdsCommand::AdsReadState) (`0x0004`)
/// * **ADS Payload Length:** 0 bytes (Body is empty)
/// * **AMS Payload:** The serialized [`AdsHeader`] (32 bytes).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsReadStateRequest {
    header: AdsHeader,
}

impl AdsReadStateRequest {
    /// Creates a new Read State Request over TCP.
    pub fn new(target: AmsAddr, source: AmsAddr, invoke_id: u32) -> Self {
        let header = AdsHeader::new(
            target,
            source,
            AdsCommand::AdsReadState,
            StateFlag::tcp_ads_request(),
            0,
            AdsReturnCode::Ok,
            invoke_id,
        );

        Self { header }
    }

    /// Creates a new Read State Request over UDP.
    pub fn new_udp(target: AmsAddr, source: AmsAddr, invoke_id: u32) -> Self {
        let header = AdsHeader::new(
            target,
            source,
            AdsCommand::AdsReadState,
            StateFlag::udp_ads_request(),
            0,
            AdsReturnCode::Ok,
            invoke_id,
        );

        Self { header }
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

    /// Returns the ADS header.
    pub fn header(&self) -> &AdsHeader {
        &self.header
    }
}

impl From<&AdsReadStateRequest> for AmsFrame {
    fn from(request: &AdsReadStateRequest) -> Self {
        AmsFrame::new(AmsCommand::AdsCommand, request.header.to_bytes())
    }
}

impl From<AdsReadStateRequest> for AmsFrame {
    fn from(request: AdsReadStateRequest) -> Self {
        AmsFrame::from(&request)
    }
}

impl TryFrom<&AmsFrame> for AdsReadStateRequest {
    type Error = ProtocolError;

    fn try_from(value: &AmsFrame) -> Result<Self, Self::Error> {
        let header = value.header();

        if header.command() != AmsCommand::AdsCommand {
            return Err(ProtocolError::UnexpectedAmsCommand {
                expected: AmsCommand::AdsCommand,
                got: header.command(),
            });
        };

        let (header, data) = AdsHeader::parse_prefix(value.payload()).map_err(AdsError::from)?;

        if header.command_id() != AdsCommand::AdsReadState {
            return Err(ProtocolError::UnexpectedAdsCommand {
                expected: AdsCommand::AdsReadState,
                got: header.command_id(),
            });
        }

        if !header.state_flags().is_request() {
            return Err(AdsError::from(StateFlagError::UnexpectedStateFlag {
                expected: vec![StateFlag::tcp_ads_request(), StateFlag::udp_ads_request()],
                got: header.state_flags(),
            }))?;
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

impl TryFrom<AmsFrame> for AdsReadStateRequest {
    type Error = ProtocolError;
    fn try_from(value: AmsFrame) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

/// Represents an ADS Read State Response (Command `0x0004`).
///
/// This is the reply sent by an ADS device containing its current state.
///
/// # Usage
/// * **Server:** Sends this in response to a [`AdsReadStateRequest`].
/// * **Client:** Receives this to update its knowledge of the target's state.
///
/// # Protocol Details
/// * **AMS Command:** [`AdsCommand`](AmsCommand::AdsCommand) (`0x0000`)
/// * **ADS Command:** [`AdsReadState`](AdsCommand::AdsReadState) (`0x0004`)
/// * **ADS Payload Length:** 8 bytes
/// * **ADS Payload Layout:**
///   * **Result Code:** 4 bytes ([`AdsReturnCode`])
///   * **ADS State:** 2 bytes ([`AdsState`])
///   * **Device State:** 2 bytes (u16)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsReadStateResponse {
    header: AdsHeader,
    result: AdsReturnCode,
    ads_state: AdsState,
    device_state: u16,
}

impl AdsReadStateResponse {
    /// The size of the ADS Read State Response body (Result + AdsState + DeviceState).
    pub const PAYLOAD_SIZE: usize = 8;

    /// Creates a new Read State Response over TCP.
    pub fn new(
        target: AmsAddr,
        source: AmsAddr,
        invoke_id: u32,
        result: AdsReturnCode,
        ads_state: AdsState,
        device_state: DeviceState,
    ) -> Self {
        let header = AdsHeader::new(
            target,
            source,
            AdsCommand::AdsReadState,
            StateFlag::tcp_ads_response(),
            Self::PAYLOAD_SIZE as u32,
            result,
            invoke_id,
        );

        Self {
            header,
            result,
            ads_state,
            device_state,
        }
    }

    /// Creates a new Read State Response over UDP.
    pub fn new_udp(
        target: AmsAddr,
        source: AmsAddr,
        invoke_id: u32,
        result: AdsReturnCode,
        ads_state: AdsState,
        device_state: DeviceState,
    ) -> Self {
        let header = AdsHeader::new(
            target,
            source,
            AdsCommand::AdsReadState,
            StateFlag::udp_ads_response(),
            Self::PAYLOAD_SIZE as u32,
            result,
            invoke_id,
        );

        Self {
            header,
            result,
            ads_state,
            device_state,
        }
    }

    /// Tries to parse a response from an AMS Frame.
    pub fn try_from_frame(frame: &AmsFrame) -> Result<Self, ProtocolError> {
        Self::try_from(frame)
    }

    /// Consumes the response and converts it into an AMS Frame.
    pub fn into_frame(self) -> AmsFrame {
        AmsFrame::from(&self)
    }

    /// Serializes the response into an AMS Frame.
    pub fn to_frame(&self) -> AmsFrame {
        AmsFrame::from(self)
    }

    /// Returns the ADS return code.
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
    pub fn device_state(&self) -> DeviceState {
        self.device_state
    }

    /// Returns the ADS header.
    pub fn header(&self) -> &AdsHeader {
        &self.header
    }

    /// Parses only the ADS payload portion (8 bytes).
    ///
    /// Returns the [ADS Return Code](AdsReturnCode), [ADS State](AdsState), and [Device State](DeviceState)
    pub fn parse_payload(
        payload: &[u8],
    ) -> Result<(AdsReturnCode, AdsState, DeviceState), ProtocolError> {
        if payload.len() != Self::PAYLOAD_SIZE {
            return Err(AdsError::UnexpectedDataLength {
                expected: Self::PAYLOAD_SIZE,
                got: payload.len(),
            })?;
        }

        let result = AdsReturnCode::try_from_slice(&payload[0..4]).map_err(AdsError::from)?;
        let ads_state = AdsState::try_from_slice(&payload[4..6]).map_err(AdsError::from)?;
        let device_state = u16::from_le_bytes(payload[6..8].try_into().unwrap());

        Ok((result, ads_state, device_state))
    }
}

impl From<&AdsReadStateResponse> for AmsFrame {
    fn from(value: &AdsReadStateResponse) -> Self {
        let mut payload =
            Vec::with_capacity(AdsHeader::LENGTH + AdsReadStateResponse::PAYLOAD_SIZE);

        payload.extend_from_slice(&value.header.to_bytes());
        payload.extend_from_slice(&value.result.to_bytes());
        payload.extend_from_slice(&value.ads_state.to_bytes());
        payload.extend_from_slice(&value.device_state.to_le_bytes());

        AmsFrame::new(AmsCommand::AdsCommand, payload)
    }
}

impl From<AdsReadStateResponse> for AmsFrame {
    fn from(value: AdsReadStateResponse) -> Self {
        AmsFrame::from(&value)
    }
}

impl TryFrom<&AmsFrame> for AdsReadStateResponse {
    type Error = ProtocolError;

    fn try_from(value: &AmsFrame) -> Result<Self, Self::Error> {
        let header = value.header();

        if header.command() != AmsCommand::AdsCommand {
            return Err(ProtocolError::UnexpectedAmsCommand {
                expected: AmsCommand::AdsCommand,
                got: header.command(),
            });
        };

        let (header, data) = AdsHeader::parse_prefix(value.payload()).map_err(AdsError::from)?;

        if header.command_id() != AdsCommand::AdsReadState {
            return Err(ProtocolError::UnexpectedAdsCommand {
                expected: AdsCommand::AdsReadState,
                got: header.command_id(),
            });
        }

        if !header.state_flags().is_response() {
            return Err(AdsError::from(StateFlagError::UnexpectedStateFlag {
                expected: vec![StateFlag::tcp_ads_response(), StateFlag::udp_ads_response()],
                got: header.state_flags(),
            }))?;
        }

        let (result, ads_state, device_state) = Self::parse_payload(data)?;

        Ok(Self {
            header,
            result,
            ads_state,
            device_state,
        })
    }
}

impl TryFrom<AmsFrame> for AdsReadStateResponse {
    type Error = ProtocolError;
    fn try_from(value: AmsFrame) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ams::AmsNetId;

    #[test]
    fn test_read_state_request_roundtrip() {
        let target = AmsAddr::new(AmsNetId::new(192, 168, 0, 1, 1, 1), 851);
        let source = AmsAddr::new(AmsNetId::new(172, 16, 0, 1, 1, 1), 30000);
        let invoke_id = 12345;

        let request = AdsReadStateRequest::new(target, source, invoke_id);

        let frame = request.to_frame();

        assert_eq!(frame.header().command(), AmsCommand::AdsCommand);
        assert_eq!(frame.header().length(), 32); // Header only, no body

        let decoded = AdsReadStateRequest::try_from(&frame).expect("Should deserialize");

        assert_eq!(decoded.header().target(), &target);
        assert_eq!(decoded.header().source(), &source);
        assert_eq!(decoded.header().invoke_id(), invoke_id);
        assert!(decoded.header().state_flags().is_request());
    }

    #[test]
    fn test_read_state_response_roundtrip() {
        let target = AmsAddr::new(AmsNetId::new(192, 168, 0, 1, 1, 1), 851);
        let source = AmsAddr::new(AmsNetId::new(172, 16, 0, 1, 1, 1), 30000);
        let invoke_id = 999;

        let response = AdsReadStateResponse::new(
            target,
            source,
            invoke_id,
            AdsReturnCode::Ok,
            AdsState::Run,
            0,
        );

        let frame = response.to_frame();

        assert_eq!(frame.header().command(), AmsCommand::AdsCommand);
        assert_eq!(frame.header().length(), 32 + 8); // Header (32) + Body (8)

        let decoded = AdsReadStateResponse::try_from(&frame).expect("Should deserialize");

        assert_eq!(decoded.result(), AdsReturnCode::Ok);
        assert_eq!(decoded.ads_state(), AdsState::Run);
        assert_eq!(decoded.device_state(), 0);
        assert!(decoded.header().state_flags().is_response());
    }
}
