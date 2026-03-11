use super::{ProtocolError, parse_ads_frame};
use crate::ads::{
    AdsCommand, AdsError, AdsHeader, AdsReturnCode, AdsTransMode, IndexGroup, IndexOffset,
    InvokeId, NotificationHandle, StateFlag,
};
use crate::ams::{AmsAddr, AmsCommand};
use crate::io::AmsFrame;

/// Represents an ADS Add Device Notification Request (Command `0x0006`).
///
/// Registers a subscription on the ADS server. The server will send an
/// [`AdsDeviceNotification`](super::AdsDeviceNotification) to the client
/// whenever the watched variable meets the transmission criteria defined by
/// [`trans_mode`](Self::trans_mode), [`max_delay`](Self::max_delay), and
/// [`cycle_time`](Self::cycle_time).
///
/// The server responds with [`AdsAddDeviceNotificationResponse`], which contains the
/// [`NotificationHandle`] needed to identify incoming samples and to cancel the
/// subscription via [`AdsDeleteDeviceNotification`](super::AdsDeleteDeviceNotificationRequest).
///
/// # Usage
/// * **Client:** Sends this to subscribe to changes on a variable.
/// * **Server:** Receives this, registers the subscription, and responds with a handle.
///
/// # Protocol Details
/// * **AMS Command:** [`AdsCommand`](AmsCommand::AdsCommand) (`0x0000`)
/// * **ADS Command:** [`AdsAddDeviceNotification`](AdsCommand::AdsAddDeviceNotification) (`0x0006`)
/// * **ADS Payload Length:** 40 bytes
/// * **ADS Payload Layout:**
///   * **Index Group:** 4 bytes ([`IndexGroup`])
///   * **Index Offset:** 4 bytes ([`IndexOffset`])
///   * **Length:** 4 bytes (u32) - the length of bytes which should be sent every notification.
///   * **Trans Mode:** 4 bytes ([`AdsTransMode`]) - when to send notifications.
///   * **Max Delay:** 4 bytes (u32, milliseconds) - maximum time the server may buffer
///     a notification before sending it. `0` means send it immediately.
///   * **Cycle Time:** 4 bytes (u32, milliseconds) - how often the server checks the
///     variable for changes. Only meaningful for cyclic trans modes.
///   * **Reserved:** 16 bytes - always zero.
///
/// # Note
/// The [TE1000 manual](https://infosys.beckhoff.com/content/1033/tc3_ads_intro/115880971.html?id=7388557527878561663)
/// recommends not registering more than 550 notifications per device.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsAddDeviceNotificationRequest {
    header: AdsHeader,
    index_group: IndexGroup,
    index_offset: IndexOffset,
    length: u32,
    trans_mode: AdsTransMode,
    max_delay: u32,
    cycle_time: u32,
    reserved: [u8; AdsAddDeviceNotificationRequest::RESERVED_SIZE],
}

impl AdsAddDeviceNotificationRequest {
    /// Size of the ADS payload (including the 16-byte reserved block).
    pub const PAYLOAD_SIZE: usize = 40;

    /// Size of the reserved block at the end of the payload.
    pub const RESERVED_SIZE: usize = 16;

    /// Creates a new Add Device Notification Request with zeroed reserved bytes.
    ///
    /// * `length` - the length of bytes which should be sent every notification.
    /// * `max_delay` - maximum buffering delay in milliseconds (`0` = send it immediately).
    /// * `cycle_time` - check interval in milliseconds (relevant for cyclic trans modes).
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        target: AmsAddr,
        source: AmsAddr,
        invoke_id: InvokeId,
        index_group: IndexGroup,
        index_offset: IndexOffset,
        length: u32,
        trans_mode: AdsTransMode,
        max_delay: u32,
        cycle_time: u32,
    ) -> Self {
        Self::with_reserved(
            target,
            source,
            invoke_id,
            index_group,
            index_offset,
            length,
            trans_mode,
            max_delay,
            cycle_time,
            [0; Self::RESERVED_SIZE],
        )
    }

    /// Creates a new Add Device Notification Request with reserved bytes.
    ///
    /// * `length` - the length of bytes which should be sent every notification.
    /// * `max_delay` - maximum buffering delay in milliseconds (`0` = send it immediately).
    /// * `cycle_time` - check interval in milliseconds (relevant for cyclic trans modes).
    #[allow(clippy::too_many_arguments)]
    pub fn with_reserved(
        target: AmsAddr,
        source: AmsAddr,
        invoke_id: InvokeId,
        index_group: IndexGroup,
        index_offset: IndexOffset,
        length: u32,
        trans_mode: AdsTransMode,
        max_delay: u32,
        cycle_time: u32,
        reserved: [u8; Self::RESERVED_SIZE],
    ) -> Self {
        let header = AdsHeader::new(
            target,
            source,
            AdsCommand::AdsAddDeviceNotification,
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
            trans_mode,
            max_delay,
            cycle_time,
            reserved,
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

    /// Returns the index group of the variable to watch.
    pub fn index_group(&self) -> IndexGroup {
        self.index_group
    }

    /// Returns the index offset of the variable to watch.
    pub fn index_offset(&self) -> IndexOffset {
        self.index_offset
    }

    /// Returns the length of bytes which should be sent every notification.
    pub fn length(&self) -> u32 {
        self.length
    }

    /// Returns the transmission mode.
    pub fn trans_mode(&self) -> AdsTransMode {
        self.trans_mode
    }

    /// Returns the maximum buffering delay in milliseconds.
    pub fn max_delay(&self) -> u32 {
        self.max_delay
    }

    /// Returns the cyclic check interval in milliseconds.
    pub fn cycle_time(&self) -> u32 {
        self.cycle_time
    }

    /// Returns the reserved bytes at the end of the payload.
    pub fn reserved(&self) -> &[u8] {
        &self.reserved
    }

    /// Parses only the ADS payload portion (40 bytes).
    ///
    /// Returns the [Index Group](IndexGroup), [Index Offset](IndexOffset), length of the bytes sent
    /// every notification, [Transmission Mode](AdsTransMode), maximum buffering delay in
    /// milliseconds, cyclic check interval in milliseconds, and the reserved bytes at
    /// the end of the payload.
    #[allow(clippy::type_complexity)]
    pub fn parse_payload(
        payload: &[u8],
    ) -> Result<(IndexGroup, IndexOffset, u32, AdsTransMode, u32, u32, &[u8]), ProtocolError> {
        if payload.len() != Self::PAYLOAD_SIZE {
            return Err(AdsError::UnexpectedDataLength {
                expected: Self::PAYLOAD_SIZE,
                got: payload.len(),
            })?;
        }

        let index_group = IndexGroup::from_le_bytes(payload[0..4].try_into().unwrap());
        let index_offset = IndexOffset::from_le_bytes(payload[4..8].try_into().unwrap());
        let length = u32::from_le_bytes(payload[8..12].try_into().unwrap());
        let trans_mode = AdsTransMode::try_from_slice(&payload[12..16]).map_err(AdsError::from)?;
        let max_delay = u32::from_le_bytes(payload[16..20].try_into().unwrap());
        let cycle_time = u32::from_le_bytes(payload[20..24].try_into().unwrap());
        let reserved = &payload[24..40];

        Ok((
            index_group,
            index_offset,
            length,
            trans_mode,
            max_delay,
            cycle_time,
            reserved,
        ))
    }
}

impl From<&AdsAddDeviceNotificationRequest> for AmsFrame {
    fn from(value: &AdsAddDeviceNotificationRequest) -> Self {
        let mut payload =
            Vec::with_capacity(AdsHeader::LENGTH + AdsAddDeviceNotificationRequest::PAYLOAD_SIZE);

        payload.extend_from_slice(&value.header.to_bytes());
        payload.extend_from_slice(&value.index_group.to_le_bytes());
        payload.extend_from_slice(&value.index_offset.to_le_bytes());
        payload.extend_from_slice(&value.length.to_le_bytes());
        payload.extend_from_slice(&value.trans_mode.to_bytes());
        payload.extend_from_slice(&value.max_delay.to_le_bytes());
        payload.extend_from_slice(&value.cycle_time.to_le_bytes());
        payload.extend_from_slice(&value.reserved);

        AmsFrame::new(AmsCommand::AdsCommand, payload)
    }
}

impl From<AdsAddDeviceNotificationRequest> for AmsFrame {
    fn from(value: AdsAddDeviceNotificationRequest) -> Self {
        AmsFrame::from(&value)
    }
}

impl TryFrom<&AmsFrame> for AdsAddDeviceNotificationRequest {
    type Error = ProtocolError;

    fn try_from(value: &AmsFrame) -> Result<Self, Self::Error> {
        let (header, data) = parse_ads_frame(value, AdsCommand::AdsAddDeviceNotification, true)?;

        let (index_group, index_offset, length, trans_mode, max_delay, cycle_time, reserved) =
            Self::parse_payload(data)?;

        Ok(Self {
            header,
            index_group,
            index_offset,
            length,
            trans_mode,
            max_delay,
            cycle_time,
            reserved: reserved.try_into().unwrap(),
        })
    }
}

impl TryFrom<AmsFrame> for AdsAddDeviceNotificationRequest {
    type Error = ProtocolError;

    fn try_from(value: AmsFrame) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

/// Represents an ADS Add Device Notification Response (Command `0x0006`).
///
/// Sent by the server in response to an [`AdsAddDeviceNotificationRequest`].
/// On success, contains the [`NotificationHandle`] that identifies this subscription
/// in all subsequent [`AdsDeviceNotification`](super::AdsDeviceNotification)
/// samples and in the matching
/// [`AdsDeleteDeviceNotification`](super::AdsDeleteDeviceNotificationRequest) request.
///
/// # Usage
/// * **Server:** Sends this to confirm the subscription and deliver the handle.
/// * **Client:** Receives this and stores the handle for later dispatch and clean-up.
///
/// # Protocol Details
/// * **AMS Command:** [`AdsCommand`](AmsCommand::AdsCommand) (`0x0000`)
/// * **ADS Command:** [`AdsAddDeviceNotification`](AdsCommand::AdsAddDeviceNotification) (`0x0006`)
/// * **ADS Payload Length:** 8 bytes
/// * **ADS Payload Layout:**
///   * **Result Code:** 4 bytes ([`AdsReturnCode`])
///   * **Notification Handle:** 4 bytes ([`NotificationHandle`])
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsAddDeviceNotificationResponse {
    header: AdsHeader,
    result: AdsReturnCode,
    handle: NotificationHandle,
}

impl AdsAddDeviceNotificationResponse {
    // Size of the ADS payload.
    pub const PAYLOAD_SIZE: usize = 8;

    /// Creates a new Add Device Notification Response.
    ///
    /// Use this on a **server** to confirm a subscription and deliver the handle.
    pub fn new(
        target: AmsAddr,
        source: AmsAddr,
        invoke_id: u32,
        result: AdsReturnCode,
        handle: NotificationHandle,
    ) -> Self {
        let header = AdsHeader::new(
            target,
            source,
            AdsCommand::AdsAddDeviceNotification,
            StateFlag::tcp_ads_response(),
            Self::PAYLOAD_SIZE as u32,
            result,
            invoke_id,
        );

        Self {
            header,
            result,
            handle,
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

    /// Returns the [`NotificationHandle`] assigned by the server.
    ///
    /// Store this handle to correlate incoming notification samples and to
    /// cancel the subscription via [`AdsDeleteDeviceNotification`](super::AdsDeleteDeviceNotificationRequest).
    pub fn handle(&self) -> NotificationHandle {
        self.handle
    }

    /// Parses only the ADS payload portion (8 bytes).
    ///
    /// Returns the [ADS Return Code](AdsReturnCode) and [Notification Handle](NotificationHandle).
    pub fn parse_payload(
        payload: &[u8],
    ) -> Result<(AdsReturnCode, NotificationHandle), ProtocolError> {
        if payload.len() != Self::PAYLOAD_SIZE {
            return Err(AdsError::UnexpectedDataLength {
                expected: Self::PAYLOAD_SIZE,
                got: payload.len(),
            })?;
        }

        let result = AdsReturnCode::try_from_slice(&payload[0..4]).map_err(AdsError::from)?;
        let handle = NotificationHandle::try_from_slice(&payload[4..8]).map_err(AdsError::from)?;

        Ok((result, handle))
    }
}

impl From<&AdsAddDeviceNotificationResponse> for AmsFrame {
    fn from(value: &AdsAddDeviceNotificationResponse) -> Self {
        let mut payload =
            Vec::with_capacity(AdsHeader::LENGTH + AdsAddDeviceNotificationResponse::PAYLOAD_SIZE);

        payload.extend_from_slice(&value.header.to_bytes());
        payload.extend_from_slice(&value.result.to_bytes());
        payload.extend_from_slice(&value.handle.to_bytes());

        AmsFrame::new(AmsCommand::AdsCommand, payload)
    }
}

impl From<AdsAddDeviceNotificationResponse> for AmsFrame {
    fn from(value: AdsAddDeviceNotificationResponse) -> Self {
        AmsFrame::from(&value)
    }
}

impl TryFrom<&AmsFrame> for AdsAddDeviceNotificationResponse {
    type Error = ProtocolError;

    fn try_from(value: &AmsFrame) -> Result<Self, Self::Error> {
        let (header, data) = parse_ads_frame(value, AdsCommand::AdsAddDeviceNotification, false)?;

        let (result, handle) = Self::parse_payload(data)?;

        Ok(Self {
            header,
            result,
            handle,
        })
    }
}

impl TryFrom<AmsFrame> for AdsAddDeviceNotificationResponse {
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
    fn test_request_roundtrip() {
        let (target, source) = make_addrs();

        let request = AdsAddDeviceNotificationRequest::new(
            target,
            source,
            0xCAFE,
            0xF005,
            0x0001_0203,
            4,
            AdsTransMode::ClientOnChange,
            0,   // max_delay: send it immediately
            100, // cycle_time: 100ms
        );

        let frame = request.to_frame();
        let decoded = AdsAddDeviceNotificationRequest::try_from(&frame).expect("Should parse");

        assert_eq!(decoded.index_group(), 0xF005);
        assert_eq!(decoded.index_offset(), 0x0001_0203);
        assert_eq!(decoded.length(), 4);
        assert_eq!(decoded.trans_mode(), AdsTransMode::ClientOnChange);
        assert_eq!(decoded.max_delay(), 0);
        assert_eq!(decoded.cycle_time(), 100);
        assert_eq!(decoded.header().invoke_id(), 0xCAFE);
        assert!(decoded.header().state_flags().is_request());
    }

    #[test]
    fn test_request_reserved_bytes_are_zero() {
        let (target, source) = make_addrs();

        let request = AdsAddDeviceNotificationRequest::new(
            target,
            source,
            1,
            0x4020,
            0x0,
            4,
            AdsTransMode::ClientCycle,
            10,
            10,
        );

        let frame = request.to_frame();
        let payload = frame.payload();

        // Reserved block starts at AdsHeader (32) + 24 bytes of fields = offset 56
        let reserved = &payload[AdsHeader::LENGTH + 24..AdsHeader::LENGTH + 40];
        assert_eq!(reserved, &[0u8; 16]);
    }

    #[test]
    fn test_request_payload_size() {
        let (target, source) = make_addrs();

        let request = AdsAddDeviceNotificationRequest::new(
            target,
            source,
            1,
            0x1,
            0x0,
            4,
            AdsTransMode::None,
            0,
            0,
        );

        let frame = request.to_frame();
        // AMS payload = AdsHeader (32) + fixed fields (40)
        assert_eq!(
            frame.header().length() as usize,
            AdsHeader::LENGTH + AdsAddDeviceNotificationRequest::PAYLOAD_SIZE
        );
    }

    #[test]
    fn test_response_roundtrip() {
        let (target, source) = make_addrs();
        let handle = NotificationHandle::from(0x0000_001A_u32);

        let response = AdsAddDeviceNotificationResponse::new(
            target,
            source,
            0xCAFE,
            AdsReturnCode::Ok,
            handle,
        );

        let frame = response.to_frame();
        let decoded = AdsAddDeviceNotificationResponse::try_from(&frame).expect("Should parse");

        assert_eq!(decoded.result(), AdsReturnCode::Ok);
        assert_eq!(decoded.handle(), handle);
        assert_eq!(decoded.header().invoke_id(), 0xCAFE);
        assert!(decoded.header().state_flags().is_response());
    }

    #[test]
    fn test_response_handle_correlates_with_request() {
        // Simulate the full add notification exchange
        let (target, source) = make_addrs();
        let invoke_id = 0xCAFE;

        let request = AdsAddDeviceNotificationRequest::new(
            target,
            source,
            invoke_id,
            0xF005,
            0x1234,
            4,
            AdsTransMode::ClientOnChange,
            0,
            100,
        );

        let assigned_handle = NotificationHandle::from(42_u32);

        let response = AdsAddDeviceNotificationResponse::new(
            source, // the server replies from its own address
            target,
            invoke_id,
            AdsReturnCode::Ok,
            assigned_handle,
        );

        let req_frame = request.to_frame();
        let resp_frame = response.to_frame();

        let decoded_req =
            AdsAddDeviceNotificationRequest::try_from(&req_frame).expect("Should parse");
        let decoded_resp =
            AdsAddDeviceNotificationResponse::try_from(&resp_frame).expect("Should parse");

        // Invoke IDs should match. Client can correlate the response to its request
        assert_eq!(
            decoded_req.header().invoke_id(),
            decoded_resp.header().invoke_id()
        );
        assert_eq!(decoded_resp.handle(), assigned_handle);
    }

    #[test]
    fn test_wrong_direction_rejected() {
        let (target, source) = make_addrs();

        // Build a request frame and try to parse it as a response
        let request = AdsAddDeviceNotificationRequest::new(
            target,
            source,
            1,
            0x1,
            0x0,
            4,
            AdsTransMode::None,
            0,
            0,
        );
        let frame = request.to_frame();

        let err = AdsAddDeviceNotificationResponse::try_from(&frame).unwrap_err();
        assert!(matches!(err, ProtocolError::Ads(_)));
    }
}
