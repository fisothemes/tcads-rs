use super::{ProtocolError, parse_ads_frame};
use crate::ads::{
    AdsCommand, AdsError, AdsHeader, AdsReturnCode, IndexGroup, IndexOffset, InvokeId, StateFlag,
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
        invoke_id: InvokeId,
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

/// A zero-copy view of an ADS Read Response (Command `0x0002`).
///
/// Borrows directly from the [`AmsFrame`] that was parsed, avoiding a copy of the
/// response data. This is the preferred type for clients that decode the data
/// immediately, and for servers that route or inspect responses without storing them.
///
/// For cases where the response must be stored, sent across a channel, or used after
/// the frame is dropped, convert to [`AdsReadResponseOwned`] via [`into_owned`](Self::into_owned)
/// or [`to_owned`](Self::to_owned).
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
///   * **Data:** n bytes - The data read from the device.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsReadResponse<'a> {
    header: AdsHeader,
    result: AdsReturnCode,
    data: &'a [u8],
}

impl<'a> AdsReadResponse<'a> {
    /// The minimum size of the ADS payload (ADS Return Code + Length).
    pub const MIN_PAYLOAD_SIZE: usize = 8;

    /// Tries to parse a response from an AMS Frame.
    pub fn try_from_frame(frame: &'a AmsFrame) -> Result<Self, ProtocolError> {
        Self::try_from(frame)
    }

    /// Returns the ADS header.
    pub fn header(&self) -> &AdsHeader {
        &self.header
    }

    /// Returns the ADS return code.
    pub fn result(&self) -> AdsReturnCode {
        self.result
    }

    /// Returns the length of the data read.
    pub fn length(&self) -> u32 {
        self.data.len() as u32
    }

    /// Returns the data read.
    pub fn data(&self) -> &[u8] {
        self.data
    }

    /// Converts this view into an owned [`AdsReadResponseOwned`], copying the data.
    pub fn into_owned(self) -> AdsReadResponseOwned {
        AdsReadResponseOwned {
            header: self.header,
            result: self.result,
            data: self.data.to_vec(),
        }
    }

    /// Clones this view into an owned [`AdsReadResponseOwned`], copying the data.
    pub fn to_owned(&self) -> AdsReadResponseOwned {
        AdsReadResponseOwned {
            header: self.header.clone(),
            result: self.result,
            data: self.data.to_vec(),
        }
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

impl<'a> TryFrom<&'a AmsFrame> for AdsReadResponse<'a> {
    type Error = ProtocolError;

    fn try_from(value: &'a AmsFrame) -> Result<Self, Self::Error> {
        let (header, data) = parse_ads_frame(value, AdsCommand::AdsRead, false)?;

        let (result, data) = Self::parse_payload(data)?;

        Ok(Self {
            header,
            result,
            data,
        })
    }
}

/// A fully owned ADS Read Response (Command `0x0002`).
///
/// Owns its data buffer, making it suitable for storage, sending across channels,
/// or constructing responses on a server to send to a client.
///
/// # Usage
/// * **Server:** Sends this back to the client, attaching the read data (or an error code if the read failed).
/// * **Client:** Receives this and decodes the `data` payload into the expected data type.
///
/// Obtain one by:
/// * Calling [`AdsReadResponseOwned::new`] to construct a response to send.
/// * Calling [`AdsReadResponse::into_owned`] or [`AdsReadResponse::to_owned`] after parsing.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsReadResponseOwned {
    header: AdsHeader,
    result: AdsReturnCode,
    data: Vec<u8>,
}

impl AdsReadResponseOwned {
    /// The minimum size of the ADS payload (Result Code + Length).
    pub const MIN_PAYLOAD_SIZE: usize = AdsReadResponse::MIN_PAYLOAD_SIZE;

    /// Creates a new owned Read Response.
    ///
    /// Use this on a **server** to construct a response to send back to a client.
    pub fn new(
        target: AmsAddr,
        source: AmsAddr,
        invoke_id: InvokeId,
        result: AdsReturnCode,
        data: impl Into<Vec<u8>>,
    ) -> Self {
        let data = data.into();

        let header = AdsHeader::new(
            target,
            source,
            AdsCommand::AdsRead,
            StateFlag::tcp_ads_response(),
            (Self::MIN_PAYLOAD_SIZE + data.len()) as u32,
            result,
            invoke_id,
        );

        Self {
            header,
            result,
            data,
        }
    }

    /// Returns the ADS header.
    pub fn header(&self) -> &AdsHeader {
        &self.header
    }

    /// Returns the [ADS Return Code](AdsReturnCode).
    pub fn result(&self) -> AdsReturnCode {
        self.result
    }

    /// Returns the length of the data read.
    pub fn length(&self) -> u32 {
        self.data.len() as u32
    }

    /// Returns the response data.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Borrows this response as a zero-copy [`AdsReadResponse`].
    pub fn as_view(&self) -> AdsReadResponse<'_> {
        AdsReadResponse {
            header: self.header.clone(),
            result: self.result,
            data: &self.data,
        }
    }

    /// Consumes the response and converts it into an AMS Frame.
    pub fn into_frame(self) -> AmsFrame {
        AmsFrame::from(&self)
    }

    /// Serializes the response into an AMS Frame.
    pub fn to_frame(&self) -> AmsFrame {
        AmsFrame::from(self)
    }
}

impl From<&AdsReadResponseOwned> for AmsFrame {
    fn from(value: &AdsReadResponseOwned) -> Self {
        let mut payload = Vec::with_capacity(
            AdsHeader::LENGTH + AdsReadResponseOwned::MIN_PAYLOAD_SIZE + value.data.len(),
        );

        payload.extend_from_slice(&value.header.to_bytes());
        payload.extend_from_slice(&value.result.to_bytes());
        payload.extend_from_slice(&(value.data.len() as u32).to_le_bytes());
        payload.extend_from_slice(&value.data);

        AmsFrame::new(AmsCommand::AdsCommand, payload)
    }
}

impl From<AdsReadResponseOwned> for AmsFrame {
    fn from(value: AdsReadResponseOwned) -> Self {
        AmsFrame::from(&value)
    }
}

impl<'a> From<AdsReadResponse<'a>> for AdsReadResponseOwned {
    fn from(value: AdsReadResponse<'a>) -> Self {
        value.into_owned()
    }
}

impl<'a> From<&'a AdsReadResponseOwned> for AdsReadResponse<'a> {
    fn from(value: &'a AdsReadResponseOwned) -> Self {
        value.as_view()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ams::AmsNetId;

    fn make_addrs() -> (AmsAddr, AmsAddr) {
        let target = AmsAddr::new(AmsNetId::new(192, 168, 0, 1, 1, 1), 851);
        let source = AmsAddr::new(AmsNetId::new(172, 16, 0, 1, 1, 1), 30000);
        (target, source)
    }

    #[test]
    fn test_read_request_roundtrip() {
        let (target, source) = make_addrs();

        let request = AdsReadRequest::new(target, source, 42, 0x4020, 0x0000, 4);
        let frame = request.to_frame();
        let decoded = AdsReadRequest::try_from(&frame).expect("Should deserialize");

        assert_eq!(decoded.index_group(), 0x4020);
        assert_eq!(decoded.index_offset(), 0x0000);
        assert_eq!(decoded.length(), 4);
        assert_eq!(decoded.header().invoke_id(), 42);
        assert!(decoded.header().state_flags().is_request());
    }

    #[test]
    fn test_read_response_zero_copy() {
        let (target, source) = make_addrs();
        let data = vec![0xDE, 0xAD, 0xBE, 0xEF];

        let owned = AdsReadResponseOwned::new(target, source, 42, AdsReturnCode::Ok, data.clone());
        let frame = owned.to_frame();

        // Parse as zero-copy view, data pointer lives inside frame's payload bytes
        let view = AdsReadResponse::try_from(&frame).expect("Should deserialize");

        assert_eq!(view.result(), AdsReturnCode::Ok);
        assert_eq!(view.data(), data.as_slice());
        assert_eq!(view.header().invoke_id(), 42);
        assert!(view.header().state_flags().is_response());
    }

    #[test]
    fn test_view_data_points_into_frame() {
        let (target, source) = make_addrs();
        let data = vec![0x01, 0x02, 0x03, 0x04];

        let owned = AdsReadResponseOwned::new(target, source, 1, AdsReturnCode::Ok, data.clone());
        let frame = owned.to_frame();

        let view = AdsReadResponse::try_from(&frame).expect("Should parse");

        // The data slice should point directly into the frame payload, not a separate allocation
        let frame_payload_ptr = frame.payload().as_ptr();
        let view_data_ptr = view.data().as_ptr();

        // view data starts at offset: AdsHeader (32) + result (4) + length (4) = 40
        assert_eq!(view_data_ptr, unsafe { frame_payload_ptr.add(40) });
    }

    #[test]
    fn test_view_to_owned_conversion() {
        let (target, source) = make_addrs();
        let data = vec![0xAA, 0xBB];

        let original =
            AdsReadResponseOwned::new(target, source, 1, AdsReturnCode::Ok, data.clone());
        let frame = original.to_frame();

        let view = AdsReadResponse::try_from(&frame).expect("Should parse");
        let converted = view.into_owned();

        assert_eq!(converted.data(), data.as_slice());
        assert_eq!(converted.result(), AdsReturnCode::Ok);
    }

    #[test]
    fn test_owned_as_view_conversion() {
        let (target, source) = make_addrs();
        let data = vec![1, 2, 3];

        let owned = AdsReadResponseOwned::new(target, source, 1, AdsReturnCode::Ok, data.clone());
        let view = owned.as_view();

        assert_eq!(view.data(), data.as_slice());
        assert_eq!(view.result(), AdsReturnCode::Ok);
    }

    #[test]
    fn test_owned_roundtrip_via_frame() {
        let (target, source) = make_addrs();
        let data = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];

        let original =
            AdsReadResponseOwned::new(target, source, 99, AdsReturnCode::Ok, data.clone());
        let frame = original.to_frame();

        // Parse as view, convert to owned, compare
        let view = AdsReadResponse::try_from(&frame).expect("Should parse");
        let roundtripped = view.into_owned();

        assert_eq!(roundtripped.data(), original.data());
        assert_eq!(roundtripped.result(), original.result());
        assert_eq!(
            roundtripped.header().invoke_id(),
            original.header().invoke_id()
        );
    }

    #[test]
    fn test_large_response_no_copy() {
        // Simulates a sum-read response, data stays in the frame, no intermediate Vec
        let (target, source) = make_addrs();
        let large_data = vec![0xFFu8; 32_768]; // 32KB

        let owned =
            AdsReadResponseOwned::new(target, source, 1, AdsReturnCode::Ok, large_data.clone());
        let frame = owned.to_frame();

        let view = AdsReadResponse::try_from(&frame).expect("Should parse");
        assert_eq!(view.data().len(), 32_768);
        assert_eq!(view.data(), large_data.as_slice());
    }

    #[test]
    fn test_empty_response() {
        let (target, source) = make_addrs();

        let owned = AdsReadResponseOwned::new(target, source, 1, AdsReturnCode::Ok, vec![]);
        let frame = owned.to_frame();

        let view = AdsReadResponse::try_from(&frame).expect("Should parse");

        assert!(view.data().is_empty());
        assert_eq!(view.data().len(), 0);
    }
}
