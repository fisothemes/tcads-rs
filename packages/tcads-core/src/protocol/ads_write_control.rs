use super::ProtocolError;
use crate::ads::{
    AdsCommand, AdsError, AdsHeader, AdsReturnCode, AdsState, DeviceState, StateFlag,
    StateFlagError,
};
use crate::ams::{AmsAddr, AmsCommand};
use crate::io::AmsFrame;

// Represents an ADS Write Control Request (Command `0x0005`).
///
/// This command is sent to an ADS device to change its ADS state (e.g., Run, Stop) and/or device state.
///
/// # Usage
/// * **Client:** Sends this to control the state of the target device (e.g. starting/stopping a PLC).
/// * **Server:** Receives this, attempts the state transition, and responds with [`AdsWriteControlResponse`].
///
/// # Protocol Details
/// * **AMS Command:** [`AdsCommand`](AmsCommand::AdsCommand) (`0x0000`)
/// * **ADS Command:** [`AdsWriteControl`](AdsCommand::AdsWriteControl) (`0x0005`)
/// * **ADS Payload Length:** 8 + n bytes (ADS State + Device State + Length + Data)
/// * **ADS Payload Layout:**
///   * **ADS State:** 2 bytes ([`AdsState`]) - The new target state (e.g., [`AdsState::Run`]).
///   * **Device State:** 2 bytes (u16) - The new device-specific state (usually 0).
///   * **Length:** 4 bytes (u32) - Length of the additional data.
///   * **Data:** n bytes - Additional data to transfer further information. Optional for current
///     ADS Devices (PLC, NC, ...).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsWriteControlRequest {
    header: AdsHeader,
    ads_state: AdsState,
    device_state: DeviceState,
    data: Vec<u8>,
}

impl AdsWriteControlRequest {
    /// The minimum size of the ADS payload (State + Device State + Length).
    pub const MIN_PAYLOAD_SIZE: usize = 8;

    /// Creates a new Write Control Request without additional data.
    pub fn new(
        target: AmsAddr,
        source: AmsAddr,
        invoke_id: u32,
        ads_state: AdsState,
        device_state: DeviceState,
    ) -> Self {
        Self::with_data(
            target,
            source,
            invoke_id,
            ads_state,
            device_state,
            Vec::new(),
        )
    }

    /// Creates a new Write Control Request with additional data.
    ///
    /// Additional data to transfer further information. At this moment in time, it is not known
    /// what that data is 🙁.
    pub fn with_data(
        target: AmsAddr,
        source: AmsAddr,
        invoke_id: u32,
        ads_state: AdsState,
        device_state: DeviceState,
        data: impl Into<Vec<u8>>,
    ) -> Self {
        let data = data.into();

        let header = AdsHeader::new(
            target,
            source,
            AdsCommand::AdsWriteControl,
            StateFlag::tcp_ads_request(),
            (Self::MIN_PAYLOAD_SIZE + data.len()) as u32,
            AdsReturnCode::Ok,
            invoke_id,
        );

        Self {
            header,
            ads_state,
            device_state,
            data,
        }
    }

    /// Tries to parse a request from an AMS Frame.
    pub fn try_from_frame(frame: &AmsFrame) -> Result<Self, ProtocolError> {
        Self::try_from(frame)
    }

    /// Consumes the request and converts it into an AMS Frame.
    pub fn into_frame(self) -> AmsFrame {
        AmsFrame::from(&self)
    }

    /// Serializes the request into an AMS Frame.
    pub fn to_frame(&self) -> AmsFrame {
        AmsFrame::from(self)
    }

    pub fn header(&self) -> &AdsHeader {
        &self.header
    }

    /// Returns the ADS state.
    pub fn ads_state(&self) -> AdsState {
        self.ads_state
    }

    /// Returns the device state.
    pub fn device_state(&self) -> DeviceState {
        self.device_state
    }

    /// Returns the additional data.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Parses only the ADS payload portion.
    ///
    /// Returns the [ADS State](AdsState), [Device State](DeviceState), and additional data.
    pub fn parse_payload(payload: &[u8]) -> Result<(AdsState, DeviceState, &[u8]), ProtocolError> {
        if payload.len() < Self::MIN_PAYLOAD_SIZE {
            return Err(AdsError::UnexpectedDataLength {
                expected: Self::MIN_PAYLOAD_SIZE,
                got: payload.len(),
            })?;
        }

        let ads_state = AdsState::try_from(&payload[0..2]).map_err(AdsError::from)?;
        let device_state = DeviceState::from_le_bytes(payload[2..4].try_into().unwrap());
        let data_len = u32::from_le_bytes(payload[4..8].try_into().unwrap()) as usize;

        let data = if data_len > 0 {
            if payload.len() < Self::MIN_PAYLOAD_SIZE + data_len {
                return Err(AdsError::UnexpectedDataLength {
                    expected: Self::MIN_PAYLOAD_SIZE + data_len,
                    got: payload.len(),
                })?;
            }
            &payload[Self::MIN_PAYLOAD_SIZE..Self::MIN_PAYLOAD_SIZE + data_len]
        } else {
            &[]
        };

        Ok((ads_state, device_state, data))
    }
}

impl From<&AdsWriteControlRequest> for AmsFrame {
    fn from(value: &AdsWriteControlRequest) -> Self {
        let mut payload = Vec::with_capacity(
            AdsHeader::LENGTH + AdsWriteControlRequest::MIN_PAYLOAD_SIZE + value.data.len(),
        );

        payload.extend_from_slice(&value.header.to_bytes());
        payload.extend_from_slice(&value.ads_state.to_bytes());
        payload.extend_from_slice(&value.device_state.to_le_bytes());
        payload.extend_from_slice(&(value.data.len() as u32).to_le_bytes());
        payload.extend_from_slice(&value.data);

        AmsFrame::new(AmsCommand::AdsCommand, payload)
    }
}

impl From<AdsWriteControlRequest> for AmsFrame {
    fn from(value: AdsWriteControlRequest) -> Self {
        AmsFrame::from(&value)
    }
}

impl TryFrom<&AmsFrame> for AdsWriteControlRequest {
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

        if header.command_id() != AdsCommand::AdsWriteControl {
            return Err(ProtocolError::UnexpectedAdsCommand {
                expected: AdsCommand::AdsWriteControl,
                got: header.command_id(),
            });
        }

        if !header.state_flags().is_request() {
            return Err(AdsError::from(StateFlagError::UnexpectedStateFlag {
                expected: vec![StateFlag::tcp_ads_request(), StateFlag::udp_ads_request()],
                got: header.state_flags(),
            }))?;
        }

        let (ads_state, device_state, data) = Self::parse_payload(data)?;

        Ok(Self {
            header,
            ads_state,
            device_state,
            data: data.to_vec(),
        })
    }
}

impl TryFrom<AmsFrame> for AdsWriteControlRequest {
    type Error = ProtocolError;

    fn try_from(value: AmsFrame) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

/// Represents an ADS Write Control Response (Command `0x0005`).
///
/// This is the reply sent by the ADS device indicating the success or failure of the state
/// change operation.
///
/// # Usage
/// * **Server:** Sends this to acknowledge a state change request.
/// * **Client:** Receives this to confirm the operation was successful.
///
/// # Protocol Details
/// * **AMS Command:** [`AdsCommand`](AmsCommand::AdsCommand) (`0x0000`)
/// * **ADS Command:** [`AdsWriteControl`](AdsCommand::AdsWriteControl) (`0x0005`)
/// * **ADS Payload Length:** 4 bytes
/// * **ADS Payload Layout:**
///   * **Result Code:** 4 bytes ([`AdsReturnCode`])
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsWriteControlResponse {
    header: AdsHeader,
    result: AdsReturnCode,
}

impl AdsWriteControlResponse {
    /// Size of the ADS Write Control Response body.
    pub const PAYLOAD_SIZE: usize = 4;

    /// Creates a new Write Control Response.
    pub fn new(target: AmsAddr, source: AmsAddr, invoke_id: u32, result: AdsReturnCode) -> Self {
        Self {
            header: AdsHeader::new(
                target,
                source,
                AdsCommand::AdsWriteControl,
                StateFlag::tcp_ads_response(),
                Self::PAYLOAD_SIZE as u32,
                result,
                invoke_id,
            ),
            result,
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

    /// Returns the ADS header.
    pub fn header(&self) -> &AdsHeader {
        &self.header
    }

    /// Returns the [ADS Return Code](AdsReturnCode).
    pub fn result(&self) -> AdsReturnCode {
        self.result
    }

    /// Parses only the ADS payload portion.
    ///
    /// Returns the [ADS Return Code](AdsReturnCode).
    pub fn parse_payload(payload: &[u8]) -> Result<AdsReturnCode, ProtocolError> {
        if payload.len() != Self::PAYLOAD_SIZE {
            return Err(AdsError::UnexpectedDataLength {
                expected: Self::PAYLOAD_SIZE,
                got: payload.len(),
            })?;
        }

        Ok(AdsReturnCode::try_from_slice(payload).map_err(AdsError::from)?)
    }
}

impl From<&AdsWriteControlResponse> for AmsFrame {
    fn from(value: &AdsWriteControlResponse) -> Self {
        let mut payload =
            Vec::with_capacity(AdsHeader::LENGTH + AdsWriteControlResponse::PAYLOAD_SIZE);

        payload.extend_from_slice(&value.header.to_bytes());
        payload.extend_from_slice(&value.result.to_bytes());

        AmsFrame::new(AmsCommand::AdsCommand, payload)
    }
}

impl From<AdsWriteControlResponse> for AmsFrame {
    fn from(value: AdsWriteControlResponse) -> Self {
        AmsFrame::from(&value)
    }
}

impl TryFrom<&AmsFrame> for AdsWriteControlResponse {
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

        if header.command_id() != AdsCommand::AdsWriteControl {
            return Err(ProtocolError::UnexpectedAdsCommand {
                expected: AdsCommand::AdsWriteControl,
                got: header.command_id(),
            });
        }

        if !header.state_flags().is_response() {
            return Err(AdsError::from(StateFlagError::UnexpectedStateFlag {
                expected: vec![StateFlag::tcp_ads_response(), StateFlag::udp_ads_response()],
                got: header.state_flags(),
            }))?;
        }

        Ok(Self {
            header,
            result: Self::parse_payload(data)?,
        })
    }
}
