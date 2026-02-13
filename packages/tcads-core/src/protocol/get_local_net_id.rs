use crate::ams::{self, AmsCommand, AmsNetId};
use crate::io::frame::AmsFrame;
use crate::protocol::ProtocolError;

/// Represents an AMS Get Local NetId Request (Command `0x1002`).
///
/// This command is sent to an AMS Router to query its local AMS Net ID.
///
/// # Usage
/// * **Client:** Sends this to discover the router's own Net ID.
/// * **Server/Router:** Receives this and responds with its configured Net ID.
///
/// # Protocol Details
/// * **Command ID:** `0x1002`
/// * **Payload Length:** 4 bytes (must be exactly 4, content is ignored)
/// * **Payload:** Any 4 bytes (typically zeros). The router ignores the content
///   and only validates the length.
///
/// # Implementation Note
/// Testing confirms the router responds with its Net ID regardless of payload content,
/// as long as the payload is exactly 4 bytes
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct GetLocalNetIdRequest;

impl GetLocalNetIdRequest {
    /// Standard 4-byte payload (zeros).
    pub const PAYLOAD: [u8; 4] = [0; 4];

    /// Creates a frame for this request.
    pub fn into_frame() -> AmsFrame {
        AmsFrame::new(AmsCommand::GetLocalNetId, Self::PAYLOAD)
    }
}

impl From<GetLocalNetIdRequest> for AmsFrame {
    fn from(_: GetLocalNetIdRequest) -> Self {
        Self::new(AmsCommand::GetLocalNetId, GetLocalNetIdRequest::PAYLOAD)
    }
}

impl From<&GetLocalNetIdRequest> for AmsFrame {
    fn from(_: &GetLocalNetIdRequest) -> Self {
        Self::new(AmsCommand::GetLocalNetId, GetLocalNetIdRequest::PAYLOAD)
    }
}

impl TryFrom<AmsFrame> for GetLocalNetIdRequest {
    type Error = ProtocolError;

    fn try_from(value: AmsFrame) -> Result<Self, Self::Error> {
        let header = value.header();

        if header.command() != AmsCommand::GetLocalNetId {
            return Err(ProtocolError::UnexpectedAmsCommand {
                expected: AmsCommand::GetLocalNetId,
                got: header.command(),
            });
        }

        if header.length() != 4 {
            return Err(ProtocolError::UnexpectedLength {
                expected: 4,
                got: header.length() as usize,
            });
        }

        // From what I have tested, router ignores payload content, so we don't validate it
        Ok(Self)
    }
}

/// Represents an AMS Get Local NetId Response (Command `0x1002`).
///
/// This is the reply sent by the AMS Router containing its local Net ID.
///
/// # Protocol Details
/// * **Command ID:** `0x1002`
/// * **Payload Length:** 6 bytes
/// * **Payload:** The router's [`AmsNetId`]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GetLocalNetIdResponse {
    net_id: AmsNetId,
}

impl GetLocalNetIdResponse {
    /// Creates a new Get Local Net ID Response with the given Net ID.
    pub fn new(net_id: AmsNetId) -> Self {
        Self { net_id }
    }

    /// Attempts to parse an [`AmsFrame`] into a [`GetLocalNetIdResponse`].
    pub fn try_from_frame(frame: AmsFrame) -> Result<Self, ProtocolError> {
        Self::try_from(frame)
    }

    /// Returns the Net ID from this response.
    pub fn net_id(&self) -> AmsNetId {
        self.net_id
    }

    /// Consumes the response and converts it into a raw [`AmsFrame`].
    pub fn into_frame(self) -> AmsFrame {
        self.into()
    }

    /// Creates a raw [`AmsFrame`] from the response.
    pub fn to_frame(&self) -> AmsFrame {
        self.into()
    }
}

impl From<GetLocalNetIdResponse> for AmsFrame {
    fn from(value: GetLocalNetIdResponse) -> Self {
        Self::new(AmsCommand::GetLocalNetId, value.net_id.to_bytes())
    }
}

impl From<&GetLocalNetIdResponse> for AmsFrame {
    fn from(value: &GetLocalNetIdResponse) -> Self {
        (*value).into()
    }
}

impl TryFrom<AmsFrame> for GetLocalNetIdResponse {
    type Error = ProtocolError;

    fn try_from(value: AmsFrame) -> Result<Self, Self::Error> {
        let header = value.header();

        if header.command() != AmsCommand::GetLocalNetId {
            return Err(ProtocolError::UnexpectedAmsCommand {
                expected: AmsCommand::GetLocalNetId,
                got: header.command(),
            });
        }

        if header.length() as usize != ams::NETID_LEN {
            return Err(ProtocolError::UnexpectedLength {
                expected: ams::NETID_LEN,
                got: header.length() as usize,
            });
        }

        let net_id = AmsNetId::try_from_slice(value.payload()).map_err(ams::AmsError::from)?;

        Ok(Self { net_id })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_frame_from_request() {
        let frame = GetLocalNetIdRequest::into_frame();

        assert_eq!(frame.header().command(), AmsCommand::GetLocalNetId);
        assert_eq!(frame.header().length(), 4);
        assert_eq!(frame.payload(), &[0u8; 4]);
    }

    #[test]
    fn create_request_from_frame_with_zeros() {
        let frame = AmsFrame::new(AmsCommand::GetLocalNetId, [0u8; 4]);

        let req = GetLocalNetIdRequest::try_from(frame).expect("Should parse valid request");
        assert_eq!(req, GetLocalNetIdRequest);
    }

    #[test]
    fn create_request_from_frame_with_any_bytes() {
        // Payload content doesn't matter, only length
        let frame = AmsFrame::new(AmsCommand::GetLocalNetId, [0xAA, 0xBB, 0xCC, 0xDD]);

        let req = GetLocalNetIdRequest::try_from(frame).expect("Should parse with any bytes");
        assert_eq!(req, GetLocalNetIdRequest);
    }

    #[test]
    fn creating_request_from_frame_fails_on_wrong_length() {
        let frame = AmsFrame::new(AmsCommand::GetLocalNetId, [0u8; 2]);

        let err = GetLocalNetIdRequest::try_from(frame).unwrap_err();

        assert!(matches!(
            err,
            ProtocolError::UnexpectedLength {
                expected: 4,
                got: 2
            }
        ));
    }

    #[test]
    fn creating_request_from_frame_fails_on_wrong_command() {
        let frame = AmsFrame::new(AmsCommand::PortClose, [0u8; 4]);

        let err = GetLocalNetIdRequest::try_from(frame).unwrap_err();

        assert!(matches!(
            err,
            ProtocolError::UnexpectedAmsCommand {
                expected: AmsCommand::GetLocalNetId,
                got: AmsCommand::PortClose
            }
        ));
    }

    #[test]
    fn create_frame_from_response() {
        let net_id: AmsNetId = "192.168.1.1.1.1".parse().unwrap();
        let resp = GetLocalNetIdResponse::new(net_id);

        let frame = resp.to_frame();

        assert_eq!(frame.header().command(), AmsCommand::GetLocalNetId);
        assert_eq!(frame.header().length() as usize, ams::NETID_LEN);
        assert_eq!(frame.payload(), &[192, 168, 1, 1, 1, 1]);
    }

    #[test]
    fn create_response_from_frame() {
        let frame = AmsFrame::new(AmsCommand::GetLocalNetId, [192, 168, 1, 1, 1, 1]);

        let resp = GetLocalNetIdResponse::try_from(frame).expect("Should parse valid response");

        assert_eq!(resp.net_id(), "192.168.1.1.1.1".parse().unwrap());
    }

    #[test]
    fn creating_response_from_frame_fails_on_wrong_length() {
        let frame = AmsFrame::new(AmsCommand::GetLocalNetId, [0u8; 8]);

        let err = GetLocalNetIdResponse::try_from(frame).unwrap_err();

        assert!(matches!(
            err,
            ProtocolError::UnexpectedLength {
                expected: 6,
                got: 8
            }
        ));
    }

    #[test]
    fn creating_response_from_frame_fails_on_wrong_command() {
        let frame = AmsFrame::new(AmsCommand::PortConnect, [0u8; 6]);

        let err = GetLocalNetIdResponse::try_from(frame).unwrap_err();

        assert!(matches!(
            err,
            ProtocolError::UnexpectedAmsCommand {
                expected: AmsCommand::GetLocalNetId,
                got: AmsCommand::PortConnect
            }
        ));
    }
}
