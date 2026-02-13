use crate::ams::{self, AmsAddr, AmsCommand, AmsPort};
use crate::io::frame::AmsFrame;
use crate::protocol::ProtocolError;

/// Represents an AMS Port Connect Request (Command `0x1000`).
///
/// This command is sent to an AMS Router to register a client and receive an assigned
/// AMS address (NetId + Port).
///
/// # Usage
/// * **Client:** Sends this to the router to announce its presence. Typically, `desired_port` is
///   set to `0` to request a dynamic port.
/// * **Server/Router:** Receives this to register a new route/client.
///
/// # Protocol Details
/// * **Command ID:** `0x1000`
/// * **Payload Length:** 2 bytes
/// * **Payload:** 16-bit integer (Little Endian) representing the desired port.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct PortConnectRequest {
    desired_port: AmsPort,
}

impl PortConnectRequest {
    /// Creates a new Port Connect Request.
    pub fn new(desired_port: AmsPort) -> Self {
        Self { desired_port }
    }

    /// Attempts to parse an [`AmsFrame`] into a [`PortConnectRequest`].
    ///
    /// # Note
    /// Payload length must be exactly 2 bytes or a [`ProtocolError`] is returned.
    pub fn try_from_frame(frame: AmsFrame) -> Result<Self, ProtocolError> {
        Self::try_from(frame)
    }

    /// Returns the desired port associated with this request.
    pub fn desired_port(&self) -> AmsPort {
        self.desired_port
    }

    /// Consumes the request and converts it into a raw [`AmsFrame`].
    pub fn into_frame(self) -> AmsFrame {
        self.into()
    }

    /// Creates a raw [`AmsFrame`] from the request (cloning).
    pub fn to_frame(&self) -> AmsFrame {
        self.into()
    }
}

impl From<PortConnectRequest> for AmsFrame {
    fn from(value: PortConnectRequest) -> Self {
        Self::new(AmsCommand::PortConnect, value.desired_port.to_le_bytes())
    }
}

impl From<&PortConnectRequest> for AmsFrame {
    fn from(value: &PortConnectRequest) -> Self {
        (*value).into_frame()
    }
}

impl TryFrom<AmsFrame> for PortConnectRequest {
    type Error = ProtocolError;

    fn try_from(value: AmsFrame) -> Result<Self, Self::Error> {
        let header = value.header();

        if header.command() != AmsCommand::PortConnect {
            return Err(ProtocolError::UnexpectedAmsCommand {
                expected: AmsCommand::PortConnect,
                got: header.command(),
            });
        }

        if header.length() != 2 {
            return Err(ProtocolError::UnexpectedLength {
                expected: 2,
                got: header.length() as usize,
            });
        }

        let payload = value.payload();

        if payload.len() != ams::AMS_PORT_LEN {
            return Err(ProtocolError::UnexpectedLength {
                expected: ams::AMS_PORT_LEN,
                got: payload.len(),
            });
        }

        Ok(Self {
            desired_port: AmsPort::from_le_bytes(payload.try_into().unwrap()),
        })
    }
}

/// Represents an AMS Port Connect Response (Command `0x1000`).
///
/// This is the reply sent by the AMS Router after a successful [`PortConnectRequest`].
/// It contains the full AMS Address (NetId + Port) assigned to the client.
///
/// # Protocol Details
/// * **Command ID:** `0x1000`
/// * **Payload Length:** 8 bytes (Standard) or more.
/// * **Payload:**
///     * Bytes 0-5: [`AmsNetId`](ams::AmsNetId)
///     * Bytes 6-7: [`AmsPort`] (Little Endian)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PortConnectResponse {
    addr: AmsAddr,
}

impl PortConnectResponse {
    /// Creates a new Port Connect Response with the assigned address.
    pub fn new(addr: AmsAddr) -> Self {
        Self { addr }
    }

    /// Attempts to parse an [`AmsFrame`] into a [`PortConnectResponse`].
    pub fn try_from_frame(frame: AmsFrame) -> Result<Self, ProtocolError> {
        Self::try_from(frame)
    }

    /// Returns the assigned AMS address.
    pub fn addr(&self) -> &AmsAddr {
        &self.addr
    }

    /// Consumes the response and converts it into a raw [`AmsFrame`].
    pub fn into_frame(self) -> AmsFrame {
        self.into()
    }

    /// Creates a raw [`AmsFrame`] from the response (cloning).
    pub fn to_frame(&self) -> AmsFrame {
        self.into()
    }
}

impl From<PortConnectResponse> for AmsFrame {
    fn from(value: PortConnectResponse) -> Self {
        Self::new(AmsCommand::PortConnect, value.addr.to_bytes())
    }
}

impl From<&PortConnectResponse> for AmsFrame {
    fn from(value: &PortConnectResponse) -> Self {
        (*value).into_frame()
    }
}

impl TryFrom<AmsFrame> for PortConnectResponse {
    type Error = ProtocolError;

    fn try_from(value: AmsFrame) -> Result<Self, Self::Error> {
        let header = value.header();

        if header.command() != AmsCommand::PortConnect {
            return Err(ProtocolError::UnexpectedAmsCommand {
                expected: AmsCommand::PortConnect,
                got: header.command(),
            });
        }

        if header.length() as usize != ams::AMS_ADDR_LEN {
            return Err(ProtocolError::UnexpectedLength {
                expected: ams::AMS_ADDR_LEN,
                got: header.length() as usize,
            });
        }

        let addr = AmsAddr::try_from_slice(value.payload()).map_err(ams::AmsError::from)?;

        Ok(Self { addr })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_frame_from_request() {
        let frame = PortConnectRequest::new(851).to_frame();

        assert_eq!(frame.header().command(), AmsCommand::PortConnect);
        assert_eq!(frame.header().length(), 2);
        assert_eq!(frame.payload(), 851u16.to_le_bytes());
    }

    #[test]
    fn create_request_from_frame() {
        let frame = AmsFrame::new(AmsCommand::PortConnect, 12345u16.to_le_bytes());

        let req = PortConnectRequest::try_from(frame).expect("Should parse valid request");
        assert_eq!(req.desired_port(), 12345);
    }

    #[test]
    fn creating_request_from_frame_fails_on_wrong_length() {
        let frame = AmsFrame::new(AmsCommand::PortConnect, [0u8; 8]);

        let err = PortConnectRequest::try_from(frame).unwrap_err();

        assert!(matches!(
            err,
            ProtocolError::UnexpectedLength {
                expected: 2,
                got: 8
            }
        ));
    }

    #[test]
    fn creating_request_from_frame_fails_on_wrong_command() {
        let frame = AmsFrame::new(AmsCommand::PortClose, [0u8; 2]);

        let err = PortConnectRequest::try_from(frame).unwrap_err();

        assert!(matches!(
            err,
            ProtocolError::UnexpectedAmsCommand {
                expected: AmsCommand::PortConnect,
                got: AmsCommand::PortClose
            }
        ));
    }

    #[test]
    fn create_frame_from_response() {
        let resp = PortConnectResponse::new("192.168.1.1.1.1:851".parse().unwrap());

        let frame = AmsFrame::from(resp);

        assert_eq!(frame.header().command(), AmsCommand::PortConnect);
        assert_eq!(frame.header().length() as usize, ams::AMS_ADDR_LEN);

        let payload = frame.payload();

        assert_eq!(&payload[0..6], [192, 168, 1, 1, 1, 1]);
        assert_eq!(&payload[6..8], 851u16.to_le_bytes());
    }

    #[test]
    fn create_response_from_frame() {
        let frame = AmsFrame::new(AmsCommand::PortConnect, [192, 168, 1, 1, 1, 1, 0x32, 0x80]);

        let resp = PortConnectResponse::try_from(frame).expect("Should parse valid response");

        assert_eq!(*resp.addr(), "192.168.1.1.1.1:32818".parse().unwrap());
    }

    #[test]
    fn creating_response_from_frame_fails_on_wrong_length() {
        let frame = AmsFrame::new(AmsCommand::PortConnect, [0u8; 10]);

        let err = PortConnectResponse::try_from(frame).unwrap_err();

        assert!(matches!(
            err,
            ProtocolError::UnexpectedLength {
                expected: 8,
                got: 10
            }
        ));
    }

    #[test]
    fn creating_response_from_frame_fails_on_wrong_command() {
        let frame = AmsFrame::new(AmsCommand::PortClose, [0u8; 2]);

        let err = PortConnectResponse::try_from(frame).unwrap_err();

        assert!(matches!(
            err,
            ProtocolError::UnexpectedAmsCommand {
                expected: AmsCommand::PortConnect,
                got: AmsCommand::PortClose
            }
        ));
    }
}
