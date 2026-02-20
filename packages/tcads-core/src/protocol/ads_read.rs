use super::{ProtocolError, parse_ads_frame};
use crate::ads::{
    AdsCommand, AdsError, AdsHeader, AdsReturnCode, IndexGroup, IndexOffset, StateFlag,
    StateFlagError,
};
use crate::ams::{AmsAddr, AmsCommand};
use crate::io::AmsFrame;

/// Represents an ADS Read Request (Command `0x0002`).
///
/// This command is sent to an ADS device to read data. The data is addressed by the Index Group and the
/// Index Offset
///
/// # Usage
/// * **Client:** Sends this request to read a variable or memory area from the target.
/// * **Server:** Receives this request, reads the requested data, and replies with an [`AdsReadResponse`].
///
/// # Protocol Details
/// * **AMS Command:** [`AdsCommand`](AmsCommand::AdsCommand) (`0x0000`)
/// * **ADS Command:** [`AdsRead`](AdsCommand::AdsRead) (`0x0002`)
/// * **ADS Payload Length:** 12 bytes
/// * **ADS Payload Layout:**
///   * **Index Group:** 4 bytes (u32) - Specifies the Index Group of the data which should be read.
///   * **Index Offset:** 4 bytes (u32) - Specifies the Index Offset of the data which should be read.
///   * **Length:** 4 bytes (u32) - The length of the data (in bytes) which should be read.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsReadRequest {
    header: AdsHeader,
    index_group: IndexGroup,
    index_offset: IndexOffset,
    length: u32,
}

impl AdsReadRequest {
    /// Size of the ADS Read Request body.
    pub const PAYLOAD_SIZE: usize = 12;

    /// Creates a new Read Request.
    pub fn new(
        target: AmsAddr,
        source: AmsAddr,
        invoke_id: u32,
        index_group: u32,
        index_offset: u32,
        length: u32,
    ) -> Self {
        let header = AdsHeader::new(
            target,
            source,
            AdsCommand::AdsRead,
            StateFlag::tcp_ads_request(),
            Self::PAYLOAD_SIZE as u32,
            AdsReturnCode::Ok,
            invoke_id,
        );

        Self {
            header,
            index_group,
            index_offset,
            length,
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

    /// Returns the ADS header.
    pub fn header(&self) -> &AdsHeader {
        &self.header
    }

    /// Returns the index group.
    pub fn index_group(&self) -> IndexGroup {
        self.index_group
    }

    /// Returns the index offset.
    pub fn index_offset(&self) -> IndexOffset {
        self.index_offset
    }

    /// Returns the length of the data to be read.
    pub fn length(&self) -> u32 {
        self.length
    }

    /// Parse only the ADS payload portion. (12 bytes)
    ///
    /// Returns the [Index Group](IndexGroup), [Index Offset](IndexOffset), and
    /// length of the data to be read.
    pub fn parse_payload(payload: &[u8]) -> Result<(IndexGroup, IndexOffset, u32), ProtocolError> {
        if payload.len() != Self::PAYLOAD_SIZE {
            return Err(AdsError::UnexpectedDataLength {
                expected: Self::PAYLOAD_SIZE,
                got: payload.len(),
            })?;
        }

        let index_group = IndexGroup::from_le_bytes(payload[0..4].try_into().unwrap());
        let index_offset = IndexOffset::from_le_bytes(payload[4..8].try_into().unwrap());
        let length = u32::from_le_bytes(payload[8..12].try_into().unwrap());

        Ok((index_group, index_offset, length))
    }
}

impl From<&AdsReadRequest> for AmsFrame {
    fn from(value: &AdsReadRequest) -> Self {
        let mut payload = Vec::with_capacity(AdsHeader::LENGTH + AdsReadRequest::PAYLOAD_SIZE);

        payload.extend_from_slice(&value.header.to_bytes());
        payload.extend_from_slice(&value.index_group.to_le_bytes());
        payload.extend_from_slice(&value.index_offset.to_le_bytes());
        payload.extend_from_slice(&value.length.to_le_bytes());

        AmsFrame::new(AmsCommand::AdsCommand, payload)
    }
}

impl From<AdsReadRequest> for AmsFrame {
    fn from(value: AdsReadRequest) -> Self {
        AmsFrame::from(&value)
    }
}

impl TryFrom<&AmsFrame> for AdsReadRequest {
    type Error = ProtocolError;

    fn try_from(value: &AmsFrame) -> Result<Self, Self::Error> {
        let (header, data) = parse_ads_frame(value, AdsCommand::AdsRead, true)?;

        let (index_group, index_offset, length) = Self::parse_payload(data)?;

        Ok(Self {
            header,
            index_group,
            index_offset,
            length,
        })
    }
}

impl TryFrom<AmsFrame> for AdsReadRequest {
    type Error = ProtocolError;

    fn try_from(value: AmsFrame) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

/// Represents an ADS Read Response (Command `0x0002`).
///
/// This is the reply containing the data that was requested by an [`AdsReadRequest`].
///
/// # Usage
/// * **Server:** Sends this back to the client, attaching the read data (or an error code if the read failed).
/// * **Client:** Receives this and decodes the `data` payload into the expected data type.
///
/// # Protocol Details
/// * **AMS Command:** [`AdsCommand`](AmsCommand::AdsCommand) (`0x0000`)
/// * **ADS Command:** [`AdsRead`](AdsCommand::AdsRead) (`0x0002`)
/// * **ADS Payload Length:** 8 + n bytes (ADS Return Code + Length + Data)
/// * **ADS Payload Layout:**
///   * **Result Code:** 4 bytes ([`AdsReturnCode`]) - The result of the read operation.
///   * **Length:** 4 bytes (u32) - The length of the data that follows.
///   * **Data:** n bytes - The actual data read from the device.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsReadResponse {
    header: AdsHeader,
    result: AdsReturnCode,
    data: Vec<u8>,
}

impl AdsReadResponse {
    /// The minimum size of the ADS payload (ADS Return Code + Length).
    pub const MIN_PAYLOAD_SIZE: usize = 8;

    /// Creates a new ADS Read Response.
    pub fn new(
        target: AmsAddr,
        source: AmsAddr,
        invoke_id: u32,
        result: AdsReturnCode,
        data: impl Into<Vec<u8>>,
    ) -> Self {
        let data = data.into();

        let length = Self::MIN_PAYLOAD_SIZE as u32 + data.len() as u32;

        let header = AdsHeader::new(
            target,
            source,
            AdsCommand::AdsRead,
            StateFlag::tcp_ads_response(),
            length,
            result,
            invoke_id,
        );

        Self {
            header,
            result,
            data,
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

    /// Returns the ADS return code.
    pub fn result(&self) -> AdsReturnCode {
        self.result
    }

    /// Returns the data read.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Parses only the ADS payload portion (8 + n bytes)
    ///
    /// Returns the [ADS Return Code](AdsReturnCode) and the data read.
    pub fn parse_payload(payload: &[u8]) -> Result<(AdsReturnCode, &[u8]), ProtocolError> {
        if payload.len() < Self::MIN_PAYLOAD_SIZE {
            return Err(AdsError::UnexpectedDataLength {
                expected: Self::MIN_PAYLOAD_SIZE,
                got: payload.len(),
            })?;
        }

        let result = AdsReturnCode::try_from_slice(&payload[0..4]).map_err(AdsError::from)?;
        let length = u32::from_le_bytes(payload[4..8].try_into().unwrap()) as usize;

        if payload.len() < Self::MIN_PAYLOAD_SIZE + length {
            return Err(AdsError::UnexpectedDataLength {
                expected: Self::MIN_PAYLOAD_SIZE + length,
                got: payload.len(),
            })?;
        }

        let data = &payload[Self::MIN_PAYLOAD_SIZE..Self::MIN_PAYLOAD_SIZE + length];

        Ok((result, data))
    }
}

impl From<&AdsReadResponse> for AmsFrame {
    fn from(value: &AdsReadResponse) -> Self {
        let mut payload = Vec::with_capacity(
            AdsHeader::LENGTH + AdsReadResponse::MIN_PAYLOAD_SIZE + value.data.len(),
        );

        payload.extend_from_slice(&value.header.to_bytes());
        payload.extend_from_slice(&value.result().to_bytes());
        payload.extend_from_slice(&(value.data.len() as u32).to_le_bytes());
        payload.extend_from_slice(&value.data);

        AmsFrame::new(AmsCommand::AdsCommand, payload)
    }
}

impl From<AdsReadResponse> for AmsFrame {
    fn from(value: AdsReadResponse) -> Self {
        AmsFrame::from(&value)
    }
}

impl TryFrom<&AmsFrame> for AdsReadResponse {
    type Error = ProtocolError;

    fn try_from(value: &AmsFrame) -> Result<Self, Self::Error> {
        let (header, data) = parse_ads_frame(value, AdsCommand::AdsRead, false)?;

        let (result, data) = Self::parse_payload(data)?;

        Ok(Self {
            header,
            result,
            data: data.to_vec(),
        })
    }
}

impl TryFrom<AmsFrame> for AdsReadResponse {
    type Error = ProtocolError;

    fn try_from(value: AmsFrame) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}
