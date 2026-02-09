use crate::ams::{self, AmsCommand, AmsPort};
use crate::io::frame::AmsFrame;
use crate::protocol::ProtocolError;

/// Represents an AMS Port Close Request (Command `0x0001`).
///
/// This command is sent to an AMS Router to unregister a previously open port.
///
/// # Usage
/// * **Client:** Sends this before closing the TCP connection to cleanly unregister from the router.
/// * **Server/Router:** Receives this to remove a route/client from its routing table.
///
/// # Protocol Details
/// * **Command ID:** `0x0001`
/// * **Payload Length:** 2 bytes
/// * **Payload:** 16-bit integer (Little Endian) representing the port to close.
///
/// # Note
/// This command does not receive an AMS-level response.
/// The router usually acknowledges it by closing the TCP connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct PortCloseRequest {
    port: AmsPort,
}

impl PortCloseRequest {
    /// Creates a new Port Close Request.
    pub fn new(port: AmsPort) -> Self {
        Self { port }
    }

    /// Attempts to parse an [`AmsFrame`] into a [`PortCloseRequest`].
    /// # Note
    ///
    /// Payload length must be exactly 2 bytes or a [`ProtocolError`] is returned.
    pub fn try_from_frame(frame: AmsFrame) -> Result<Self, ProtocolError> {
        Self::try_from(frame)
    }

    /// Returns the port number associated with this request.
    pub fn port(&self) -> AmsPort {
        self.port
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

impl From<PortCloseRequest> for AmsFrame {
    fn from(value: PortCloseRequest) -> Self {
        Self::new(AmsCommand::PortClose, value.port.to_le_bytes())
    }
}

impl From<&PortCloseRequest> for AmsFrame {
    fn from(value: &PortCloseRequest) -> Self {
        (*value).into_frame()
    }
}

impl TryFrom<AmsFrame> for PortCloseRequest {
    type Error = ProtocolError;

    fn try_from(value: AmsFrame) -> Result<Self, Self::Error> {
        let header = value.header();

        if header.command() != AmsCommand::PortClose {
            return Err(ProtocolError::UnexpectedCommand {
                expected: AmsCommand::PortClose,
                got: header.command(),
            });
        }

        if header.length() as usize != ams::AMS_PORT_LEN {
            return Err(ProtocolError::UnexpectedLength {
                expected: ams::AMS_PORT_LEN,
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
            port: AmsPort::from_le_bytes(payload.try_into().unwrap()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_frame_from_request() {
        let frame: AmsFrame = PortCloseRequest::new(12345).into();

        assert_eq!(frame.header().command(), AmsCommand::PortClose);
        assert_eq!(frame.header().length() as usize, ams::AMS_PORT_LEN);
        assert_eq!(frame.payload(), &12345u16.to_le_bytes());
    }

    #[test]
    fn create_request_from_frame() {
        let frame = AmsFrame::new(AmsCommand::PortClose, 30000u16.to_le_bytes());

        let req = PortCloseRequest::try_from(frame).expect("Should parse valid request");
        assert_eq!(req.port(), 30000);
    }

    #[test]
    fn creating_request_from_frame_fails_on_wrong_length() {
        let frame = AmsFrame::new(AmsCommand::PortClose, [0u8; 4]);

        let err = PortCloseRequest::try_from(frame).unwrap_err();

        assert!(matches!(
            err,
            ProtocolError::UnexpectedLength {
                expected: 2,
                got: 4
            }
        ));
    }

    #[test]
    fn creating_request_from_frame_fails_on_wrong_command() {
        let frame = AmsFrame::new(AmsCommand::PortConnect, 0u16.to_le_bytes());

        let err = PortCloseRequest::try_from(frame).unwrap_err();

        assert!(matches!(
            err,
            ProtocolError::UnexpectedCommand {
                expected: AmsCommand::PortClose,
                got: AmsCommand::PortConnect
            }
        ));
    }
}
