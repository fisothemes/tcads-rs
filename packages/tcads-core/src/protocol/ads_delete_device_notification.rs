use super::{ProtocolError, parse_ads_frame};
use crate::ads::{AdsCommand, AdsError, AdsHeader, AdsReturnCode, NotificationHandle, StateFlag};
use crate::ams::{AmsAddr, AmsCommand};
use crate::io::AmsFrame;

/// Represents an ADS Delete Device Notification Request (Command `0x0007`).
///
/// Cancels an active notification subscription identified by a [`NotificationHandle`]
/// previously obtained from [`AdsAddDeviceNotificationResponse`](super::AdsAddDeviceNotificationResponse).
///
/// # Usage
/// * **Client:** Sends this to cancel a subscription when it is no longer necessary.
/// * **Server:** Receives this, removes the subscription, and responds with
///   [`AdsDeleteDeviceNotificationResponse`].
///
/// # Protocol Details
/// * **AMS Command:** [`AdsCommand`](AmsCommand::AdsCommand) (`0x0000`)
/// * **ADS Command:** [`AdsDeleteDeviceNotification`](AdsCommand::AdsDeleteDeviceNotification) (`0x0007`)
/// * **ADS Payload Length:** 4 bytes
/// * **ADS Payload Layout:**
///   * **Notification Handle:** 4 bytes ([`NotificationHandle`])
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsDeleteDeviceNotificationRequest {
    header: AdsHeader,
    handle: NotificationHandle,
}

impl AdsDeleteDeviceNotificationRequest {
    /// Size of the ADS payload.
    pub const PAYLOAD_SIZE: usize = 4;

    /// Creates a new Delete Device Notification Request.
    pub fn new(
        target: AmsAddr,
        source: AmsAddr,
        invoke_id: u32,
        handle: NotificationHandle,
    ) -> Self {
        let header = AdsHeader::new(
            target,
            source,
            AdsCommand::AdsDeleteDeviceNotification,
            StateFlag::tcp_ads_request(),
            Self::PAYLOAD_SIZE as u32,
            AdsReturnCode::Ok,
            invoke_id,
        );

        Self { header, handle }
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

    /// Returns the [`NotificationHandle`] to cancel.
    pub fn handle(&self) -> NotificationHandle {
        self.handle
    }

    /// Parses only the ADS payload portion (4 bytes).
    pub fn parse_payload(payload: &[u8]) -> Result<NotificationHandle, ProtocolError> {
        if payload.len() != Self::PAYLOAD_SIZE {
            return Err(AdsError::UnexpectedDataLength {
                expected: Self::PAYLOAD_SIZE,
                got: payload.len(),
            })?;
        }

        Ok(NotificationHandle::try_from_slice(payload).map_err(AdsError::from)?)
    }
}

impl From<&AdsDeleteDeviceNotificationRequest> for AmsFrame {
    fn from(value: &AdsDeleteDeviceNotificationRequest) -> Self {
        let mut payload = Vec::with_capacity(
            AdsHeader::LENGTH + AdsDeleteDeviceNotificationRequest::PAYLOAD_SIZE,
        );

        payload.extend_from_slice(&value.header.to_bytes());
        payload.extend_from_slice(&value.handle.to_bytes());

        AmsFrame::new(AmsCommand::AdsCommand, payload)
    }
}

impl From<AdsDeleteDeviceNotificationRequest> for AmsFrame {
    fn from(value: AdsDeleteDeviceNotificationRequest) -> Self {
        AmsFrame::from(&value)
    }
}

impl TryFrom<&AmsFrame> for AdsDeleteDeviceNotificationRequest {
    type Error = ProtocolError;

    fn try_from(value: &AmsFrame) -> Result<Self, Self::Error> {
        let (header, data) = parse_ads_frame(value, AdsCommand::AdsDeleteDeviceNotification, true)?;

        let handle = Self::parse_payload(data)?;

        Ok(Self { header, handle })
    }
}

impl TryFrom<AmsFrame> for AdsDeleteDeviceNotificationRequest {
    type Error = ProtocolError;

    fn try_from(value: AmsFrame) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

/// Represents an ADS Delete Device Notification Response (Command `0x0007`).
///
/// Sent by the server to confirm the subscription has been cancelled.
///
/// # Usage
/// * **Server:** Sends this to acknowledge a delete request.
/// * **Client:** Receives this to confirm the subscription is cancelled.
///
/// # Protocol Details
/// * **AMS Command:** [`AdsCommand`](AmsCommand::AdsCommand) (`0x0000`)
/// * **ADS Command:** [`AdsDeleteDeviceNotification`](AdsCommand::AdsDeleteDeviceNotification) (`0x0007`)
/// * **ADS Payload Length:** 4 bytes
/// * **ADS Payload Layout:**
///   * **Result Code:** 4 bytes ([`AdsReturnCode`])
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsDeleteDeviceNotificationResponse {
    header: AdsHeader,
    result: AdsReturnCode,
}

impl AdsDeleteDeviceNotificationResponse {
    /// Size of the ADS payload.
    pub const PAYLOAD_SIZE: usize = 4;

    /// Creates a new Delete Device Notification Response.
    pub fn new(target: AmsAddr, source: AmsAddr, invoke_id: u32, result: AdsReturnCode) -> Self {
        let header = AdsHeader::new(
            target,
            source,
            AdsCommand::AdsDeleteDeviceNotification,
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

impl From<&AdsDeleteDeviceNotificationResponse> for AmsFrame {
    fn from(value: &AdsDeleteDeviceNotificationResponse) -> Self {
        let mut payload = Vec::with_capacity(
            AdsHeader::LENGTH + AdsDeleteDeviceNotificationResponse::PAYLOAD_SIZE,
        );

        payload.extend_from_slice(&value.header.to_bytes());
        payload.extend_from_slice(&value.result.to_bytes());

        AmsFrame::new(AmsCommand::AdsCommand, payload)
    }
}

impl From<AdsDeleteDeviceNotificationResponse> for AmsFrame {
    fn from(value: AdsDeleteDeviceNotificationResponse) -> Self {
        AmsFrame::from(&value)
    }
}

impl TryFrom<&AmsFrame> for AdsDeleteDeviceNotificationResponse {
    type Error = ProtocolError;

    fn try_from(value: &AmsFrame) -> Result<Self, Self::Error> {
        let (header, data) =
            parse_ads_frame(value, AdsCommand::AdsDeleteDeviceNotification, false)?;

        Ok(Self {
            header,
            result: Self::parse_payload(data)?,
        })
    }
}

impl TryFrom<AmsFrame> for AdsDeleteDeviceNotificationResponse {
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
        let handle = NotificationHandle::from(42_u32);

        let request = AdsDeleteDeviceNotificationRequest::new(target, source, 0xDEAD, handle);
        let frame = request.to_frame();
        let decoded = AdsDeleteDeviceNotificationRequest::try_from(&frame).expect("Should parse");

        assert_eq!(decoded.handle(), handle);
        assert_eq!(decoded.header().invoke_id(), 0xDEAD);
        assert!(decoded.header().state_flags().is_request());
    }

    #[test]
    fn test_request_payload_size() {
        let (target, source) = make_addrs();
        let handle = NotificationHandle::from(1_u32);

        let request = AdsDeleteDeviceNotificationRequest::new(target, source, 1, handle);
        let frame = request.to_frame();

        // AMS payload = AdsHeader (32) + handle (4)
        assert_eq!(
            frame.header().length() as usize,
            AdsHeader::LENGTH + AdsDeleteDeviceNotificationRequest::PAYLOAD_SIZE
        );
    }

    #[test]
    fn test_response_roundtrip() {
        let (target, source) = make_addrs();

        let response =
            AdsDeleteDeviceNotificationResponse::new(target, source, 0xDEAD, AdsReturnCode::Ok);
        let frame = response.to_frame();
        let decoded = AdsDeleteDeviceNotificationResponse::try_from(&frame).expect("Should parse");

        assert_eq!(decoded.result(), AdsReturnCode::Ok);
        assert_eq!(decoded.header().invoke_id(), 0xDEAD);
        assert!(decoded.header().state_flags().is_response());
    }

    #[test]
    fn test_full_delete_exchange() {
        // Simulate the complete delete notification exchange
        let (target, source) = make_addrs();
        let handle = NotificationHandle::from(0x0000_001A_u32);
        let invoke_id = 0xDEAD;

        let request = AdsDeleteDeviceNotificationRequest::new(target, source, invoke_id, handle);
        let response =
            AdsDeleteDeviceNotificationResponse::new(source, target, invoke_id, AdsReturnCode::Ok);

        let req_frame = request.to_frame();
        let resp_frame = response.to_frame();

        let decoded_req =
            AdsDeleteDeviceNotificationRequest::try_from(&req_frame).expect("Should parse");
        let decoded_resp =
            AdsDeleteDeviceNotificationResponse::try_from(&resp_frame).expect("Should parse");

        assert_eq!(decoded_req.handle(), handle);
        assert_eq!(
            decoded_req.header().invoke_id(),
            decoded_resp.header().invoke_id()
        );
        assert_eq!(decoded_resp.result(), AdsReturnCode::Ok);
    }

    #[test]
    fn test_wrong_direction_rejected() {
        let (target, source) = make_addrs();
        let handle = NotificationHandle::from(1_u32);

        let request = AdsDeleteDeviceNotificationRequest::new(target, source, 1, handle);
        let frame = request.to_frame();

        let err = AdsDeleteDeviceNotificationResponse::try_from(&frame).unwrap_err();
        assert!(matches!(err, ProtocolError::Ads(_)));
    }

    #[test]
    fn test_wrong_command_rejected() {
        let (target, source) = make_addrs();

        // Build an AdsReadState request and try to parse it as a delete notification request
        let read_state = crate::protocol::AdsReadStateRequest::new(target, source, 1);
        let frame = read_state.to_frame();

        let err = AdsDeleteDeviceNotificationRequest::try_from(&frame).unwrap_err();
        assert!(matches!(
            err,
            ProtocolError::UnexpectedAdsCommand {
                expected: AdsCommand::AdsDeleteDeviceNotification,
                ..
            }
        ));
    }
}
