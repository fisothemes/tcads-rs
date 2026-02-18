use super::ProtocolError;
use crate::ads::{
    AdsCommand, AdsError, AdsHeader, AdsReturnCode, AdsState, DeviceState, StateFlag,
    StateFlagError,
};
use crate::ams::{AmsAddr, AmsCommand};
use crate::io::AmsFrame;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsWriteControlRequest {
    header: AdsHeader,
    ads_state: AdsState,
    device_state: DeviceState,
    data: Vec<u8>,
}

impl AdsWriteControlRequest {
    pub const MIN_PAYLOAD_SIZE: usize = 8;

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

    pub fn try_from_frame(frame: &AmsFrame) -> Result<Self, ProtocolError> {
        Self::try_from(frame)
    }

    pub fn into_frame(self) -> AmsFrame {
        AmsFrame::from(&self)
    }

    pub fn to_frame(&self) -> AmsFrame {
        AmsFrame::from(self)
    }

    pub fn header(&self) -> &AdsHeader {
        &self.header
    }

    pub fn ads_state(&self) -> AdsState {
        self.ads_state
    }

    pub fn device_state(&self) -> DeviceState {
        self.device_state
    }

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
            return Err(StateFlagError::UnexpectedStateFlag {
                expected: vec![StateFlag::tcp_ads_request(), StateFlag::udp_ads_request()],
                got: header.state_flags(),
            })
            .map_err(AdsError::from)?;
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsWriteControlResponse {
    header: AdsHeader,
    result: AdsReturnCode,
}

impl AdsWriteControlResponse {
    const PAYLOAD_SIZE: usize = 4;

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

    pub fn try_from_frame(frame: &AmsFrame) -> Result<Self, ProtocolError> {
        Self::try_from(frame)
    }

    pub fn into_frame(self) -> AmsFrame {
        AmsFrame::from(&self)
    }

    pub fn to_frame(&self) -> AmsFrame {
        AmsFrame::from(self)
    }

    pub fn header(&self) -> &AdsHeader {
        &self.header
    }

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
        AmsFrame::new(AmsCommand::AdsCommand, value.result.to_bytes())
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
            return Err(StateFlagError::UnexpectedStateFlag {
                expected: vec![StateFlag::tcp_ads_response(), StateFlag::udp_ads_response()],
                got: header.state_flags(),
            })
            .map_err(AdsError::from)?;
        }

        Ok(Self {
            header,
            result: Self::parse_payload(data)?,
        })
    }
}
