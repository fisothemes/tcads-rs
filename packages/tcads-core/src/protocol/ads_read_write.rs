use super::{ProtocolError, parse_ads_frame};
use crate::ads::{
    AdsCommand, AdsError, AdsHeader, AdsReturnCode, IndexGroup, IndexOffset, StateFlag,
};
use crate::ams::{AmsAddr, AmsCommand};
use crate::io::AmsFrame;

/// A zero-copy view of an ADS Read/Write Request (Command `0x0009`).
///
/// Borrows the write data directly from the [`AmsFrame`] that was parsed, avoiding
/// a copy. This is the preferred type for servers that process incoming read/write
/// requests without needing to store the write data beyond the current frame.
///
/// For cases where the request must be stored, sent across a channel, or used after
/// the frame is dropped, convert to [`AdsReadWriteRequestOwned`] via
/// [`into_owned`](Self::into_owned) or [`to_owned`](Self::to_owned).
///
/// # Usage
/// * **Client:** Sends this when a read result depends on first writing a value — e.g. querying
///   a symbol handle by name, or triggering a PLC function block and reading its output.
/// * **Server:** Receives this, performs the write, then the read, and responds with
///   [`AdsReadWriteResponse`].
///
/// # Protocol Details
/// * **AMS Command:** [`AdsCommand`](AmsCommand::AdsCommand) (`0x0000`)
/// * **ADS Command:** [`AdsReadWrite`](AdsCommand::AdsReadWrite) (`0x0009`)
/// * **ADS Payload Length:** 16 + n bytes
/// * **ADS Payload Layout:**
///   * **Index Group:** 4 bytes (u32) - Index Group in which the data should be written.
///   * **Index Offset:** 4 bytes (u32) - Index Offset in which the data should be written
///   * **Read Length:** 4 bytes (u32) - Number of bytes expected in the response.
///   * **Write Length:** 4 bytes (u32) - Number of bytes to write.
///   * **Data:** n bytes - The data to write (length == Write Length).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsReadWriteRequest<'a> {
    header: AdsHeader,
    index_group: IndexGroup,
    index_offset: IndexOffset,
    read_length: u32,
    data: &'a [u8],
}

impl<'a> AdsReadWriteRequest<'a> {
    /// The minimum size of the ADS payload (Index Group + Index Offset + Read Length + Write Length).
    pub const MIN_PAYLOAD_SIZE: usize = 16;

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

    /// Returns the number of bytes expected back from the device.
    pub fn read_length(&self) -> u32 {
        self.read_length
    }

    /// Returns the number of bytes to write to the device.
    pub fn write_length(&self) -> u32 {
        self.data.len() as u32
    }

    /// Returns a zero-copy slice of the data to be written.
    pub fn data(&self) -> &[u8] {
        self.data
    }

    /// Converts this view into an owned [`AdsReadWriteRequestOwned`], copying the write data.
    pub fn into_owned(self) -> AdsReadWriteRequestOwned {
        AdsReadWriteRequestOwned {
            header: self.header,
            index_group: self.index_group,
            index_offset: self.index_offset,
            read_length: self.read_length,
            data: self.data.to_vec(),
        }
    }

    /// Clones this view into an owned [`AdsReadWriteRequestOwned`], copying the write data.
    pub fn to_owned(&self) -> AdsReadWriteRequestOwned {
        AdsReadWriteRequestOwned {
            header: self.header.clone(),
            index_group: self.index_group,
            index_offset: self.index_offset,
            read_length: self.read_length,
            data: self.data.to_vec(),
        }
    }

    /// Parses only the ADS payload portion (16 + n bytes).
    ///
    /// Returns the [Index Group](IndexGroup), [Index Offset](IndexOffset),
    /// read length and a zero-copy slice of the write data.
    pub fn parse_payload(
        payload: &[u8],
    ) -> Result<(IndexGroup, IndexOffset, u32, &[u8]), ProtocolError> {
        if payload.len() < Self::MIN_PAYLOAD_SIZE {
            return Err(AdsError::UnexpectedDataLength {
                expected: Self::MIN_PAYLOAD_SIZE,
                got: payload.len(),
            })?;
        }

        let index_group = IndexGroup::from_le_bytes(payload[0..4].try_into().unwrap());
        let index_offset = IndexOffset::from_le_bytes(payload[4..8].try_into().unwrap());
        let read_length = u32::from_le_bytes(payload[8..12].try_into().unwrap());
        let write_length = u32::from_le_bytes(payload[12..16].try_into().unwrap()) as usize;

        if payload.len() < Self::MIN_PAYLOAD_SIZE + write_length {
            return Err(AdsError::UnexpectedDataLength {
                expected: Self::MIN_PAYLOAD_SIZE + write_length,
                got: payload.len(),
            })?;
        }

        Ok((
            index_group,
            index_offset,
            read_length,
            &payload[Self::MIN_PAYLOAD_SIZE..Self::MIN_PAYLOAD_SIZE + write_length],
        ))
    }
}

impl<'a> TryFrom<&'a AmsFrame> for AdsReadWriteRequest<'a> {
    type Error = ProtocolError;

    fn try_from(value: &'a AmsFrame) -> Result<Self, Self::Error> {
        let (header, data) = parse_ads_frame(value, AdsCommand::AdsReadWrite, true)?;

        let (index_group, index_offset, read_length, data) = Self::parse_payload(data)?;

        Ok(Self {
            header,
            index_group,
            index_offset,
            read_length,
            data,
        })
    }
}

/// A fully owned ADS Read/Write Request (Command `0x0009`).
///
/// Owns its write data buffer, making it suitable for storage, sending across channels,
/// or constructing requests on a client to send to a device.
///
/// # Usage
/// * **Client:** Sends this when a read result depends on first writing a value — e.g. querying
///   a symbol handle by name, or triggering a PLC function block and reading its output.
/// * **Server:** Receives this, performs the write, then the read, and responds with
///   [`AdsReadWriteResponse`].
///
/// Obtain one by:
/// * Calling [`AdsReadWriteRequestOwned::new`] to construct a request to send.
/// * Calling [`AdsReadWriteRequest::into_owned`] or [`AdsReadWriteRequest::to_owned`]
///   after parsing.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsReadWriteRequestOwned {
    header: AdsHeader,
    index_group: IndexGroup,
    index_offset: IndexOffset,
    read_length: u32,
    data: Vec<u8>,
}

impl AdsReadWriteRequestOwned {
    /// The minimum size of the ADS payload (Index Group + Index Offset + Read Length + Write Length).
    pub const MIN_PAYLOAD_SIZE: usize = AdsReadWriteRequest::MIN_PAYLOAD_SIZE;

    /// Creates a new Read/Write Request.
    ///
    /// * `read_length` - Number of bytes expected back in the response.
    /// * `data` - The data to write to the device.
    pub fn new(
        target: AmsAddr,
        source: AmsAddr,
        invoke_id: u32,
        index_group: IndexGroup,
        index_offset: IndexOffset,
        read_length: u32,
        data: impl Into<Vec<u8>>,
    ) -> Self {
        let data = data.into();

        let header = AdsHeader::new(
            target,
            source,
            AdsCommand::AdsReadWrite,
            StateFlag::tcp_ads_request(),
            (Self::MIN_PAYLOAD_SIZE + data.len()) as u32,
            AdsReturnCode::Ok,
            invoke_id,
        );

        Self {
            header,
            index_group,
            index_offset,
            read_length,
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

    /// Returns the number of bytes expected back from the device.
    pub fn read_length(&self) -> u32 {
        self.read_length
    }

    /// Returns the number of bytes to write to the device.
    pub fn write_length(&self) -> u32 {
        self.data.len() as u32
    }

    /// Returns the data to be written to the device.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Borrows this request as a zero-copy [`AdsReadWriteRequest`].
    pub fn as_view(&self) -> AdsReadWriteRequest<'_> {
        AdsReadWriteRequest {
            header: self.header.clone(),
            index_group: self.index_group,
            index_offset: self.index_offset,
            read_length: self.read_length,
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

impl From<&AdsReadWriteRequestOwned> for AmsFrame {
    fn from(value: &AdsReadWriteRequestOwned) -> Self {
        let mut payload = Vec::with_capacity(
            AdsHeader::LENGTH + AdsReadWriteRequestOwned::MIN_PAYLOAD_SIZE + value.data.len(),
        );

        payload.extend_from_slice(&value.header.to_bytes());
        payload.extend_from_slice(&value.index_group.to_le_bytes());
        payload.extend_from_slice(&value.index_offset.to_le_bytes());
        payload.extend_from_slice(&value.read_length.to_le_bytes());
        payload.extend_from_slice(&(value.data.len() as u32).to_le_bytes());
        payload.extend_from_slice(&value.data);

        AmsFrame::new(AmsCommand::AdsCommand, payload)
    }
}

impl From<AdsReadWriteRequestOwned> for AmsFrame {
    fn from(value: AdsReadWriteRequestOwned) -> Self {
        AmsFrame::from(&value)
    }
}

impl<'a> From<AdsReadWriteRequest<'a>> for AdsReadWriteRequestOwned {
    fn from(value: AdsReadWriteRequest<'a>) -> Self {
        value.into_owned()
    }
}

impl<'a> From<&'a AdsReadWriteRequestOwned> for AdsReadWriteRequest<'a> {
    fn from(value: &'a AdsReadWriteRequestOwned) -> Self {
        value.as_view()
    }
}

/// A zero-copy view of an ADS Read/Write Response (Command `0x0009`).
///
/// Borrows the read data directly from the [`AmsFrame`] that was parsed, avoiding
/// a copy. This is the preferred type for clients that decode the data immediately,
/// and for servers that route or inspect responses without storing them.
///
/// For cases where the response must be stored, sent across a channel, or used after
/// the frame is dropped, convert to [`AdsReadWriteResponseOwned`] via
/// [`into_owned`](Self::into_owned) or [`to_owned`](Self::to_owned).
///
/// # Protocol Details
/// * **AMS Command:** [`AdsCommand`](AmsCommand::AdsCommand) (`0x0000`)
/// * **ADS Command:** [`AdsReadWrite`](AdsCommand::AdsReadWrite) (`0x0009`)
/// * **ADS Payload Length:** 8 + n bytes
/// * **ADS Payload Layout:**
///   * **Result Code:** 4 bytes ([`AdsReturnCode`]) - The result of the operation.
///   * **Length:** 4 bytes (u32) - The length of the read data that follows.
///   * **Data:** n bytes - The data read from the device.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsReadWriteResponse<'a> {
    header: AdsHeader,
    result: AdsReturnCode,
    data: &'a [u8],
}

impl<'a> AdsReadWriteResponse<'a> {
    /// The minimum size of the ADS payload (Result Code + Length).
    pub const MIN_PAYLOAD_SIZE: usize = 8;

    /// Tries to parse a response from an AMS Frame.
    pub fn try_from_frame(frame: &'a AmsFrame) -> Result<Self, ProtocolError> {
        Self::try_from(frame)
    }

    /// Returns the ADS header.
    pub fn header(&self) -> &AdsHeader {
        &self.header
    }

    /// Returns the [ADS Return Code](AdsReturnCode).
    pub fn result(&self) -> AdsReturnCode {
        self.result
    }

    /// Returns the number of bytes read from the device.
    pub fn length(&self) -> u32 {
        self.data.len() as u32
    }

    /// Returns a zero-copy slice of the data read from the device.
    pub fn data(&self) -> &[u8] {
        self.data
    }

    /// Converts this view into an owned [`AdsReadWriteResponseOwned`], copying the data.
    pub fn into_owned(self) -> AdsReadWriteResponseOwned {
        AdsReadWriteResponseOwned {
            header: self.header,
            result: self.result,
            data: self.data.to_vec(),
        }
    }

    /// Clones this view into an owned [`AdsReadWriteResponseOwned`], copying the data.
    pub fn to_owned(&self) -> AdsReadWriteResponseOwned {
        AdsReadWriteResponseOwned {
            header: self.header.clone(),
            result: self.result,
            data: self.data.to_vec(),
        }
    }

    /// Parses only the ADS payload portion (8 + n bytes).
    ///
    /// Returns the [ADS Return Code](AdsReturnCode) and a zero-copy slice of the data read.
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

        Ok((
            result,
            &payload[Self::MIN_PAYLOAD_SIZE..Self::MIN_PAYLOAD_SIZE + length],
        ))
    }
}

impl<'a> TryFrom<&'a AmsFrame> for AdsReadWriteResponse<'a> {
    type Error = ProtocolError;

    fn try_from(value: &'a AmsFrame) -> Result<Self, Self::Error> {
        let (header, data) = parse_ads_frame(value, AdsCommand::AdsReadWrite, false)?;

        let (result, data) = Self::parse_payload(data)?;

        Ok(Self {
            header,
            result,
            data,
        })
    }
}

/// A fully owned ADS Read/Write Response (Command `0x0009`).
///
/// Owns its data buffer, making it suitable for storage, sending across channels,
/// or constructing responses on a server to send to a client.
///
/// Obtain one by:
/// * Calling [`AdsReadWriteResponseOwned::new`] to construct a response to send.
/// * Calling [`AdsReadWriteResponse::into_owned`] or [`AdsReadWriteResponse::to_owned`]
///   after parsing.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsReadWriteResponseOwned {
    header: AdsHeader,
    result: AdsReturnCode,
    data: Vec<u8>,
}

impl AdsReadWriteResponseOwned {
    // The minimum size of the ADS payload (Result Code + Length).
    pub const MIN_PAYLOAD_SIZE: usize = AdsReadWriteResponse::MIN_PAYLOAD_SIZE;

    /// Creates a new Read/Write Response.
    ///
    /// Use this on a **server** to construct a response to send back to a client.
    pub fn new(
        target: AmsAddr,
        source: AmsAddr,
        invoke_id: u32,
        result: AdsReturnCode,
        data: impl Into<Vec<u8>>,
    ) -> Self {
        let data = data.into();

        let header = AdsHeader::new(
            target,
            source,
            AdsCommand::AdsReadWrite,
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

    /// Returns the number of bytes read from the device.
    pub fn length(&self) -> u32 {
        self.data.len() as u32
    }

    /// Returns the data read from the device.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Borrows this response as a zero-copy [`AdsReadWriteResponse`].
    pub fn as_view(&self) -> AdsReadWriteResponse<'_> {
        AdsReadWriteResponse {
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

impl From<&AdsReadWriteResponseOwned> for AmsFrame {
    fn from(value: &AdsReadWriteResponseOwned) -> Self {
        let mut payload = Vec::with_capacity(
            AdsHeader::LENGTH + AdsReadWriteResponseOwned::MIN_PAYLOAD_SIZE + value.data.len(),
        );

        payload.extend_from_slice(&value.header.to_bytes());
        payload.extend_from_slice(&value.result.to_bytes());
        payload.extend_from_slice(&(value.data.len() as u32).to_le_bytes());
        payload.extend_from_slice(&value.data);

        AmsFrame::new(AmsCommand::AdsCommand, payload)
    }
}

impl From<AdsReadWriteResponseOwned> for AmsFrame {
    fn from(value: AdsReadWriteResponseOwned) -> Self {
        AmsFrame::from(&value)
    }
}

impl<'a> From<AdsReadWriteResponse<'a>> for AdsReadWriteResponseOwned {
    fn from(value: AdsReadWriteResponse<'a>) -> Self {
        value.into_owned()
    }
}

impl<'a> From<&'a AdsReadWriteResponseOwned> for AdsReadWriteResponse<'a> {
    fn from(value: &'a AdsReadWriteResponseOwned) -> Self {
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
    fn test_request_zero_copy() {
        let (target, source) = make_addrs();
        let write_data = vec![0x01, 0x02, 0x03, 0x04];

        let owned = AdsReadWriteRequestOwned::new(
            target,
            source,
            99,
            0xF003,
            0x0000,
            4,
            write_data.clone(),
        );
        let frame = owned.to_frame();

        let view = AdsReadWriteRequest::try_from(&frame).expect("Should parse");

        assert_eq!(view.index_group(), 0xF003);
        assert_eq!(view.index_offset(), 0x0000);
        assert_eq!(view.read_length(), 4);
        assert_eq!(view.data(), write_data.as_slice());
        assert_eq!(view.header().invoke_id(), 99);
        assert!(view.header().state_flags().is_request());
    }

    #[test]
    fn test_request_write_data_points_into_frame() {
        let (target, source) = make_addrs();
        let write_data = vec![0xDE, 0xAD, 0xBE, 0xEF];

        let owned =
            AdsReadWriteRequestOwned::new(target, source, 1, 0x1, 0x2, 4, write_data.clone());
        let frame = owned.to_frame();

        let view = AdsReadWriteRequest::try_from(&frame).expect("Should parse");

        // write_data starts at: AdsHeader (32) + IndexGroup (4) + IndexOffset (4)
        //                       + ReadLength (4) + WriteLength (4) = 48
        let frame_payload_ptr = frame.payload().as_ptr();
        let view_data_ptr = view.data().as_ptr();
        assert_eq!(view_data_ptr, unsafe { frame_payload_ptr.add(48) });
    }

    #[test]
    fn test_request_empty_write_data() {
        let (target, source) = make_addrs();

        let owned = AdsReadWriteRequestOwned::new(target, source, 1, 0x1, 0x0, 16, []);
        let frame = owned.to_frame();

        let view = AdsReadWriteRequest::try_from(&frame).expect("Should parse");
        assert!(view.data().is_empty());
        assert_eq!(view.read_length(), 16);
    }

    #[test]
    fn test_request_view_into_owned() {
        let (target, source) = make_addrs();
        let write_data = b"MAIN.counter\0".to_vec();

        let owned =
            AdsReadWriteRequestOwned::new(target, source, 1, 0xF003, 0x0, 4, write_data.clone());
        let frame = owned.to_frame();

        let view = AdsReadWriteRequest::try_from(&frame).expect("Should parse");
        let converted = view.into_owned();

        assert_eq!(converted.data(), write_data.as_slice());
        assert_eq!(converted.read_length(), 4);
    }

    #[test]
    fn test_request_owned_as_view() {
        let (target, source) = make_addrs();
        let write_data = vec![1, 2, 3];

        let owned =
            AdsReadWriteRequestOwned::new(target, source, 1, 0xF003, 0x0, 4, write_data.clone());
        let view = owned.as_view();

        assert_eq!(view.data(), write_data.as_slice());
        assert_eq!(view.index_group(), 0xF003);
    }

    #[test]
    fn test_response_zero_copy() {
        let (target, source) = make_addrs();
        let data = vec![0xDE, 0xAD, 0xBE, 0xEF];

        let owned =
            AdsReadWriteResponseOwned::new(target, source, 99, AdsReturnCode::Ok, data.clone());
        let frame = owned.to_frame();

        let view = AdsReadWriteResponse::try_from(&frame).expect("Should parse");

        assert_eq!(view.result(), AdsReturnCode::Ok);
        assert_eq!(view.data(), data.as_slice());
        assert_eq!(view.header().invoke_id(), 99);
        assert!(view.header().state_flags().is_response());
    }

    #[test]
    fn test_response_data_points_into_frame() {
        let (target, source) = make_addrs();
        let data = vec![0x01, 0x02, 0x03, 0x04];

        let owned =
            AdsReadWriteResponseOwned::new(target, source, 1, AdsReturnCode::Ok, data.clone());
        let frame = owned.to_frame();

        let view = AdsReadWriteResponse::try_from(&frame).expect("Should parse");

        // data starts at: AdsHeader (32) + Result (4) + Length (4) = 40
        let frame_payload_ptr = frame.payload().as_ptr();
        let view_data_ptr = view.data().as_ptr();
        assert_eq!(view_data_ptr, unsafe { frame_payload_ptr.add(40) });
    }

    #[test]
    fn test_response_view_into_owned() {
        let (target, source) = make_addrs();
        let data = vec![0xAA, 0xBB, 0xCC, 0xDD];

        let owned =
            AdsReadWriteResponseOwned::new(target, source, 1, AdsReturnCode::Ok, data.clone());
        let frame = owned.to_frame();

        let view = AdsReadWriteResponse::try_from(&frame).expect("Should parse");
        let converted = view.into_owned();

        assert_eq!(converted.data(), data.as_slice());
        assert_eq!(converted.result(), AdsReturnCode::Ok);
    }

    #[test]
    fn test_response_owned_as_view() {
        let (target, source) = make_addrs();
        let data = vec![1, 2, 3];

        let owned =
            AdsReadWriteResponseOwned::new(target, source, 1, AdsReturnCode::Ok, data.clone());
        let view = owned.as_view();

        assert_eq!(view.data(), data.as_slice());
        assert_eq!(view.result(), AdsReturnCode::Ok);
    }

    #[test]
    fn test_large_write_and_response_no_copy() {
        let (target, source) = make_addrs();
        let large_write = vec![0xAAu8; 32_768];
        let large_read = vec![0xBBu8; 32_768];

        let req_owned = AdsReadWriteRequestOwned::new(
            target,
            source,
            1,
            0xF080,
            0x0,
            32_768,
            large_write.clone(),
        );
        let req_frame = req_owned.to_frame();
        let req_view = AdsReadWriteRequest::try_from(&req_frame).expect("Should parse");
        assert_eq!(req_view.data().len(), 32_768);

        let resp_owned = AdsReadWriteResponseOwned::new(
            target,
            source,
            1,
            AdsReturnCode::Ok,
            large_read.clone(),
        );
        let resp_frame = resp_owned.to_frame();
        let resp_view = AdsReadWriteResponse::try_from(&resp_frame).expect("Should parse");
        assert_eq!(resp_view.data().len(), 32_768);
    }

    #[test]
    fn test_symbol_handle_lookup_pattern() {
        // Common real-world usage: AdsReadWrite to get a symbol handle by name
        let (target, source) = make_addrs();
        let symbol_name = b"MAIN.counter\0";

        let owned = AdsReadWriteRequestOwned::new(
            target,
            source,
            1,
            0xF003, // ADSIGRP_SYM_HNDBYNAME
            0x0000,
            4, // handle is 4 bytes
            symbol_name.to_vec(),
        );
        let frame = owned.to_frame();
        let view = AdsReadWriteRequest::try_from(&frame).expect("Should parse");

        assert_eq!(view.data(), symbol_name);
        assert_eq!(view.read_length(), 4);
        assert_eq!(view.index_group(), 0xF003);

        // Simulate the response: server sends back a 4-byte handle
        let handle = 0x0000_001Au32.to_le_bytes();
        let resp_owned =
            AdsReadWriteResponseOwned::new(source, target, 1, AdsReturnCode::Ok, handle);
        let resp_frame = resp_owned.to_frame();
        let resp_view = AdsReadWriteResponse::try_from(&resp_frame).expect("Should parse");

        assert_eq!(resp_view.data(), &handle);
    }

    #[test]
    fn test_request_from_impls() {
        let (target, source) = make_addrs();
        let write_data = vec![1, 2, 3];

        let owned =
            AdsReadWriteRequestOwned::new(target, source, 1, 0x1, 0x2, 8, write_data.clone());

        let view: AdsReadWriteRequest<'_> = AdsReadWriteRequest::from(&owned);
        assert_eq!(view.data(), write_data.as_slice());

        let back: AdsReadWriteRequestOwned = AdsReadWriteRequestOwned::from(view);
        assert_eq!(back.data(), write_data.as_slice());
    }

    #[test]
    fn test_response_from_impls() {
        let (target, source) = make_addrs();
        let data = vec![0xDE, 0xAD];

        let owned =
            AdsReadWriteResponseOwned::new(target, source, 1, AdsReturnCode::Ok, data.clone());

        let view: AdsReadWriteResponse<'_> = AdsReadWriteResponse::from(&owned);
        assert_eq!(view.data(), data.as_slice());

        let back: AdsReadWriteResponseOwned = AdsReadWriteResponseOwned::from(view);
        assert_eq!(back.data(), data.as_slice());
    }
}
