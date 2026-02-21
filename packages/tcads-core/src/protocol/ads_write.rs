use super::{ProtocolError, parse_ads_frame};
use crate::ads::{
    AdsCommand, AdsError, AdsHeader, AdsReturnCode, IndexGroup, IndexOffset, StateFlag,
};
use crate::ams::{AmsAddr, AmsCommand};
use crate::io::AmsFrame;

/// A zero-copy view of an ADS Write Request (Command `0x0003`).
///
/// Borrows the write data directly from the [`AmsFrame`] that was parsed, avoiding
/// a copy. This is the preferred type for servers that process incoming writes
/// without needing to store the data beyond the current frame.
///
/// For cases where the request must be stored, sent across a channel, or used after
/// the frame is dropped, convert to [`AdsWriteRequestOwned`] via
/// [`into_owned`](Self::into_owned) or [`to_owned`](Self::to_owned).
///
/// # Usage
/// * **Client:** Sends this request to write a variable or memory area on the target.
/// * **Server:** Receives this, writes the data, and replies with an [`AdsWriteResponse`].
///
/// # Protocol Details
/// * **AMS Command:** [`AdsCommand`](AmsCommand::AdsCommand) (`0x0000`)
/// * **ADS Command:** [`AdsWrite`](AdsCommand::AdsWrite) (`0x0003`)
/// * **ADS Payload Length:** 12 + n bytes
/// * **ADS Payload Layout:**
///   * **Index Group:** 4 bytes (u32) - Specifies the Index Group of the data to write.
///   * **Index Offset:** 4 bytes (u32) - Specifies the Index Offset of the data to write.
///   * **Length:** 4 bytes (u32) - The length of the data (in bytes) to write.
///   * **Data:** n bytes - The data to write.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsWriteRequest<'a> {
    header: AdsHeader,
    index_group: IndexGroup,
    index_offset: IndexOffset,
    data: &'a [u8],
}

impl<'a> AdsWriteRequest<'a> {
    /// The minimum size of the ADS payload (Index Group + Index Offset + Length).
    pub const MIN_PAYLOAD_SIZE: usize = 12;

    /// Tries to parse a request from an AMS Frame.
    pub fn try_from_frame(frame: &'a AmsFrame) -> Result<Self, ProtocolError> {
        Self::try_from(frame)
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

    /// Returns the length of the data to be written.
    pub fn length(&self) -> u32 {
        self.data.len() as u32
    }

    /// Returns a zero-copy slice of the data to be written.
    pub fn data(&self) -> &[u8] {
        self.data
    }

    /// Converts this view into an owned [`AdsWriteRequestOwned`], copying the data.
    pub fn into_owned(self) -> AdsWriteRequestOwned {
        AdsWriteRequestOwned {
            header: self.header,
            index_group: self.index_group,
            index_offset: self.index_offset,
            data: self.data.to_vec(),
        }
    }

    /// Clones this view into an owned [`AdsWriteRequestOwned`], copying the data.
    pub fn to_owned(&self) -> AdsWriteRequestOwned {
        AdsWriteRequestOwned {
            header: self.header.clone(),
            index_group: self.index_group,
            index_offset: self.index_offset,
            data: self.data.to_vec(),
        }
    }

    pub fn parse_payload(
        payload: &[u8],
    ) -> Result<(IndexGroup, IndexOffset, &[u8]), ProtocolError> {
        if payload.len() < Self::MIN_PAYLOAD_SIZE {
            return Err(AdsError::UnexpectedDataLength {
                expected: Self::MIN_PAYLOAD_SIZE,
                got: payload.len(),
            })?;
        }

        let index_group = IndexGroup::from_le_bytes(payload[0..4].try_into().unwrap());
        let index_offset = IndexOffset::from_le_bytes(payload[4..8].try_into().unwrap());
        let data_len = u32::from_le_bytes(payload[8..12].try_into().unwrap()) as usize;

        if payload.len() < Self::MIN_PAYLOAD_SIZE + data_len {
            return Err(AdsError::UnexpectedDataLength {
                expected: Self::MIN_PAYLOAD_SIZE + data_len,
                got: payload.len(),
            })?;
        }

        Ok((
            index_group,
            index_offset,
            &payload[Self::MIN_PAYLOAD_SIZE..Self::MIN_PAYLOAD_SIZE + data_len],
        ))
    }
}

impl<'a> TryFrom<&'a AmsFrame> for AdsWriteRequest<'a> {
    type Error = ProtocolError;

    fn try_from(value: &'a AmsFrame) -> Result<Self, Self::Error> {
        let (header, data) = parse_ads_frame(value, AdsCommand::AdsWrite, true)?;

        let (index_group, index_offset, data) = Self::parse_payload(data)?;

        Ok(Self {
            header,
            index_group,
            index_offset,
            data,
        })
    }
}

/// A fully owned ADS Write Request (Command `0x0003`).
///
/// Owns its data buffer, making it suitable for storage, sending across channels,
/// or constructing requests on a client to send to a device.
///
/// # Usage
/// * **Client:** Sends this request to write a variable or memory area on the target.
/// * **Server:** Receives this, writes the data, and replies with an [`AdsWriteResponse`].
///
/// Obtain one by:
/// * Calling [`AdsWriteRequestOwned::new`] to construct a request to send.
/// * Calling [`AdsWriteRequest::into_owned`] or [`AdsWriteRequest::to_owned`] after parsing.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsWriteRequestOwned {
    header: AdsHeader,
    index_group: IndexGroup,
    index_offset: IndexOffset,
    data: Vec<u8>,
}

impl AdsWriteRequestOwned {
    /// The minimum size of the ADS payload (Index Group + Index Offset + Length).
    pub const MIN_PAYLOAD_SIZE: usize = AdsWriteRequest::MIN_PAYLOAD_SIZE;

    /// Creates a new Write Request.
    pub fn new(
        target: AmsAddr,
        source: AmsAddr,
        invoke_id: u32,
        index_group: IndexGroup,
        index_offset: IndexOffset,
        data: impl Into<Vec<u8>>,
    ) -> Self {
        let data = data.into();

        let header = AdsHeader::new(
            target,
            source,
            AdsCommand::AdsWrite,
            StateFlag::tcp_ads_request(),
            (Self::MIN_PAYLOAD_SIZE + data.len()) as u32,
            AdsReturnCode::Ok,
            invoke_id,
        );

        Self {
            header,
            index_group,
            index_offset,
            data,
        }
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

    /// Returns the length of the data to be written.
    pub fn length(&self) -> u32 {
        self.data.len() as u32
    }

    /// Returns the data to be written.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Borrows this request as a zero-copy [`AdsWriteRequest`].
    pub fn as_view(&self) -> AdsWriteRequest<'_> {
        AdsWriteRequest {
            header: self.header.clone(),
            index_group: self.index_group,
            index_offset: self.index_offset,
            data: &self.data,
        }
    }

    /// Consumes the request and converts it into an AMS Frame.
    pub fn into_frame(self) -> AmsFrame {
        AmsFrame::from(&self)
    }

    /// Serializes the request into an AMS Frame.
    pub fn to_frame(&self) -> AmsFrame {
        AmsFrame::from(self)
    }
}

impl From<&AdsWriteRequestOwned> for AmsFrame {
    fn from(value: &AdsWriteRequestOwned) -> Self {
        let mut payload = Vec::with_capacity(
            AdsHeader::LENGTH + AdsWriteRequestOwned::MIN_PAYLOAD_SIZE + value.data.len(),
        );

        payload.extend_from_slice(&value.header.to_bytes());
        payload.extend_from_slice(&value.index_group.to_le_bytes());
        payload.extend_from_slice(&value.index_offset.to_le_bytes());
        payload.extend_from_slice(&(value.data.len() as u32).to_le_bytes());
        payload.extend_from_slice(&value.data);

        AmsFrame::new(AmsCommand::AdsCommand, payload)
    }
}

impl From<AdsWriteRequestOwned> for AmsFrame {
    fn from(value: AdsWriteRequestOwned) -> Self {
        AmsFrame::from(&value)
    }
}

impl<'a> From<AdsWriteRequest<'a>> for AdsWriteRequestOwned {
    fn from(value: AdsWriteRequest<'a>) -> Self {
        value.into_owned()
    }
}

impl<'a> From<&'a AdsWriteRequestOwned> for AdsWriteRequest<'a> {
    fn from(value: &'a AdsWriteRequestOwned) -> Self {
        value.as_view()
    }
}

/// Represents an ADS Write Response (Command `0x0003`).
///
/// This is the reply sent by the ADS device indicating the success or failure of the
/// write operation.
///
/// # Usage
/// * **Server:** Sends this to acknowledge a write request.
/// * **Client:** Receives this to confirm the write was successful.
///
/// # Protocol Details
/// * **AMS Command:** [`AdsCommand`](AmsCommand::AdsCommand) (`0x0000`)
/// * **ADS Command:** [`AdsWrite`](AdsCommand::AdsWrite) (`0x0003`)
/// * **ADS Payload Length:** 4 bytes
/// * **ADS Payload Layout:**
///   * **Result Code:** 4 bytes ([`AdsReturnCode`])
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsWriteResponse {
    header: AdsHeader,
    result: AdsReturnCode,
}

impl AdsWriteResponse {
    /// Size of the ADS Write Response body.
    pub const PAYLOAD_SIZE: usize = 4;

    /// Creates a new Write Response.
    pub fn new(target: AmsAddr, source: AmsAddr, invoke_id: u32, result: AdsReturnCode) -> Self {
        let header = AdsHeader::new(
            target,
            source,
            AdsCommand::AdsWrite,
            StateFlag::tcp_ads_response(),
            Self::PAYLOAD_SIZE as u32,
            result,
            invoke_id,
        );

        Self { header, result }
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

    /// Parses only the ADS payload portion (4 bytes).
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

impl From<&AdsWriteResponse> for AmsFrame {
    fn from(value: &AdsWriteResponse) -> Self {
        let mut payload = Vec::with_capacity(AdsHeader::LENGTH + AdsWriteResponse::PAYLOAD_SIZE);

        payload.extend_from_slice(&value.header.to_bytes());
        payload.extend_from_slice(&value.result.to_bytes());

        AmsFrame::new(AmsCommand::AdsCommand, payload)
    }
}

impl From<AdsWriteResponse> for AmsFrame {
    fn from(value: AdsWriteResponse) -> Self {
        AmsFrame::from(&value)
    }
}

impl TryFrom<&AmsFrame> for AdsWriteResponse {
    type Error = ProtocolError;

    fn try_from(value: &AmsFrame) -> Result<Self, Self::Error> {
        let (header, data) = parse_ads_frame(value, AdsCommand::AdsWrite, false)?;

        Ok(Self {
            header,
            result: Self::parse_payload(data)?,
        })
    }
}

impl TryFrom<AmsFrame> for AdsWriteResponse {
    type Error = ProtocolError;

    fn try_from(value: AmsFrame) -> Result<Self, Self::Error> {
        Self::try_from(&value)
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
    fn test_write_request_zero_copy() {
        let (target, source) = make_addrs();
        let data = vec![0x01, 0x02, 0x03, 0x04];

        let owned = AdsWriteRequestOwned::new(target, source, 42, 0x4020, 0x0000, data.clone());
        let frame = owned.to_frame();

        let view = AdsWriteRequest::try_from(&frame).expect("Should parse");

        assert_eq!(view.index_group(), 0x4020);
        assert_eq!(view.index_offset(), 0x0000);
        assert_eq!(view.data(), data.as_slice());
        assert_eq!(view.header().invoke_id(), 42);
        assert!(view.header().state_flags().is_request());
    }

    #[test]
    fn test_write_request_data_points_into_frame() {
        let (target, source) = make_addrs();
        let data = vec![0xDE, 0xAD, 0xBE, 0xEF];

        let owned = AdsWriteRequestOwned::new(target, source, 1, 0x1, 0x2, data.clone());
        let frame = owned.to_frame();

        let view = AdsWriteRequest::try_from(&frame).expect("Should parse");

        // data starts at: AdsHeader (32) + IndexGroup (4) + IndexOffset (4) + Length (4) = 44
        let frame_payload_ptr = frame.payload().as_ptr();
        let view_data_ptr = view.data().as_ptr();
        assert_eq!(view_data_ptr, unsafe { frame_payload_ptr.add(44) });
    }

    #[test]
    fn test_write_request_empty_data() {
        let (target, source) = make_addrs();

        let owned = AdsWriteRequestOwned::new(target, source, 1, 0x4020, 0x0010, []);
        let frame = owned.to_frame();

        let view = AdsWriteRequest::try_from(&frame).expect("Should parse");
        assert!(view.data().is_empty());
    }

    #[test]
    fn test_view_into_owned() {
        let (target, source) = make_addrs();
        let data = vec![0xAA, 0xBB];

        let owned = AdsWriteRequestOwned::new(target, source, 1, 0x1, 0x2, data.clone());
        let frame = owned.to_frame();

        let view = AdsWriteRequest::try_from(&frame).expect("Should parse");
        let converted = view.into_owned();

        assert_eq!(converted.index_group(), 0x1);
        assert_eq!(converted.data(), data.as_slice());
    }

    #[test]
    fn test_owned_as_view() {
        let (target, source) = make_addrs();
        let data = vec![1, 2, 3];

        let owned = AdsWriteRequestOwned::new(target, source, 1, 0x4020, 0x0, data.clone());
        let view = owned.as_view();

        assert_eq!(view.data(), data.as_slice());
        assert_eq!(view.index_group(), 0x4020);
    }

    #[test]
    fn test_owned_roundtrip_via_frame() {
        let (target, source) = make_addrs();
        let data = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];

        let original = AdsWriteRequestOwned::new(target, source, 99, 0x4020, 0x10, data.clone());
        let frame = original.to_frame();

        let view = AdsWriteRequest::try_from(&frame).expect("Should parse");
        let roundtripped = view.into_owned();

        assert_eq!(roundtripped.index_group(), original.index_group());
        assert_eq!(roundtripped.index_offset(), original.index_offset());
        assert_eq!(roundtripped.data(), original.data());
        assert_eq!(
            roundtripped.header().invoke_id(),
            original.header().invoke_id()
        );
    }

    #[test]
    fn test_large_write_no_copy() {
        let (target, source) = make_addrs();
        let large_data = vec![0xAAu8; 32_768]; // 32KB

        let owned = AdsWriteRequestOwned::new(target, source, 1, 0xF081, 0x0, large_data.clone());
        let frame = owned.to_frame();

        let view = AdsWriteRequest::try_from(&frame).expect("Should parse");
        assert_eq!(view.data().len(), 32_768);
        assert_eq!(view.data(), large_data.as_slice());
    }

    #[test]
    fn test_from_impls() {
        let (target, source) = make_addrs();
        let data = vec![1, 2, 3];

        let owned = AdsWriteRequestOwned::new(target, source, 1, 0x1, 0x2, data.clone());

        // &AdsWriteRequestOwned -> AdsWriteRequest<'_>
        let view: AdsWriteRequest<'_> = AdsWriteRequest::from(&owned);
        assert_eq!(view.data(), data.as_slice());

        // AdsWriteRequest<'_> -> AdsWriteRequestOwned
        let back: AdsWriteRequestOwned = AdsWriteRequestOwned::from(view);
        assert_eq!(back.data(), data.as_slice());
    }

    #[test]
    fn test_write_response_roundtrip() {
        let (target, source) = make_addrs();

        let response = AdsWriteResponse::new(target, source, 42, AdsReturnCode::Ok);
        let frame = response.to_frame();

        let decoded = AdsWriteResponse::try_from(&frame).expect("Should parse");
        assert_eq!(decoded.result(), AdsReturnCode::Ok);
        assert_eq!(decoded.header().invoke_id(), 42);
        assert!(decoded.header().state_flags().is_response());
    }

    #[test]
    fn test_payload_size_in_header() {
        let (target, source) = make_addrs();
        let data = vec![0xAA, 0xBB];

        let owned = AdsWriteRequestOwned::new(target, source, 1, 0x1, 0x2, data.clone());
        let frame = owned.to_frame();

        // AMS payload = AdsHeader (32) + IndexGroup (4) + IndexOffset (4) + Length (4) + Data (2)
        assert_eq!(frame.header().length() as usize, 32 + 12 + 2);
    }
}
