pub use crate::ams::{AmsCommand, RouterState};
use crate::io::frame::AmsFrame;
use crate::protocol::ProtocolError;

/// Represents an AMS Router Notification (Command `0x1001`).
///
/// The router sends this to all connected clients when its state changes.
///
/// # Usage
/// * **Server/Router:** Broadcasts this to all registered clients
///   (used [`PortConnect`](AmsCommand::PortConnect) command) when the state changes
///   (e.g., started, stopped, route removed).
/// * **Client:** Receives this to monitor router health and connection status.
///
/// # Protocol Details
/// * **Command ID:** `0x1001`
/// * **Payload Length:** 4 bytes
/// * **Payload:** 32-bit integer (Little Endian) representing [`RouterState`]
///
/// # Example
/// ```no_run
/// use tcads_core::protocol::{RouterNotification, PortConnectRequest, PortConnectResponse};
/// use tcads_core::io::blocking::AmsStream;
/// use tcads_core::ams::AmsCommand;
/// use tcads_core::protocol::ProtocolError;
/// use std::net::TcpStream;
///
/// fn example(mut stream: AmsStream<TcpStream>) -> Result<(), ProtocolError> {
///     let (reader, mut writer) = stream.try_split()?;
///     // Send Port Connect request
///     writer.write_frame(&PortConnectRequest::default().to_frame())?;
///     // Listen for notifications
///     for result in reader.incoming() {
///         let frame = result?;
///         match frame.header().command() {
///             AmsCommand::PortConnect => {
///                 let resp = PortConnectResponse::try_from(frame)?;
///                 println!("Router assigned us address: {}", resp.addr());
///             },
///             AmsCommand::RouterNotification => {
///                 let notif = RouterNotification::try_from(frame)?;
///                 println!("Router state changed: {}", notif.state())
///             },
///             cmd => {
///                 println!("Unexpected Router command: {cmd:?}");
///             }
///         }
///     }
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RouterNotification {
    state: RouterState,
}

impl RouterNotification {
    /// Creates a new Router Notification with the given state.
    pub fn new(state: RouterState) -> Self {
        Self { state }
    }

    /// Attempts to parse an [`AmsFrame`] into a [`RouterNotification`].
    pub fn try_from_frame(frame: AmsFrame) -> Result<Self, ProtocolError> {
        Self::try_from(frame)
    }

    /// Returns the router state from this notification.
    pub fn state(&self) -> RouterState {
        self.state
    }

    /// Consumes the notification and converts it into a raw [`AmsFrame`].
    pub fn into_frame(self) -> AmsFrame {
        self.into()
    }

    /// Creates a raw [`AmsFrame`] from the notification.
    pub fn to_frame(&self) -> AmsFrame {
        self.into()
    }
}

impl From<RouterNotification> for AmsFrame {
    fn from(value: RouterNotification) -> Self {
        Self::new(AmsCommand::RouterNotification, value.state.to_bytes())
    }
}

impl From<&RouterNotification> for AmsFrame {
    fn from(value: &RouterNotification) -> Self {
        (*value).into()
    }
}

impl TryFrom<AmsFrame> for RouterNotification {
    type Error = ProtocolError;

    fn try_from(value: AmsFrame) -> Result<Self, Self::Error> {
        let header = value.header();

        if header.command() != AmsCommand::RouterNotification {
            return Err(ProtocolError::UnexpectedAmsCommand {
                expected: AmsCommand::RouterNotification,
                got: header.command(),
            });
        }

        if header.length() != 4 {
            return Err(ProtocolError::UnexpectedLength {
                expected: 4,
                got: header.length() as usize,
            });
        }

        let payload = value.payload();
        let state = RouterState::from(u32::from_le_bytes([
            payload[0], payload[1], payload[2], payload[3],
        ]));

        Ok(Self { state })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_frame_from_notification() {
        let notification = RouterNotification::new(RouterState::Start);
        let frame = notification.to_frame();

        assert_eq!(frame.header().command(), AmsCommand::RouterNotification);
        assert_eq!(frame.header().length(), 4);
        assert_eq!(frame.payload(), &[1, 0, 0, 0]);
    }

    #[test]
    fn create_notification_from_frame_stop() {
        let frame = AmsFrame::new(AmsCommand::RouterNotification, [0, 0, 0, 0]);

        let notification = RouterNotification::try_from(frame).expect("Should parse");
        assert_eq!(notification.state(), RouterState::Stop);
    }

    #[test]
    fn create_notification_from_frame_start() {
        let frame = AmsFrame::new(AmsCommand::RouterNotification, [1, 0, 0, 0]);

        let notification = RouterNotification::try_from(frame).expect("Should parse");
        assert_eq!(notification.state(), RouterState::Start);
    }

    #[test]
    fn create_notification_from_frame_removed() {
        let frame = AmsFrame::new(AmsCommand::RouterNotification, [2, 0, 0, 0]);

        let notification = RouterNotification::try_from(frame).expect("Should parse");
        assert_eq!(notification.state(), RouterState::Removed);
    }

    #[test]
    fn create_notification_from_frame_unknown() {
        let frame = AmsFrame::new(AmsCommand::RouterNotification, [3, 0, 0, 0]);

        let notification = RouterNotification::try_from(frame).expect("Should parse");
        assert_eq!(notification.state(), RouterState::Unknown(3));
    }

    #[test]
    fn creating_notification_from_frame_fails_on_wrong_length() {
        let frame = AmsFrame::new(AmsCommand::RouterNotification, [0, 0]);

        let err = RouterNotification::try_from(frame).unwrap_err();

        assert!(matches!(
            err,
            ProtocolError::UnexpectedLength {
                expected: 4,
                got: 2
            }
        ));
    }

    #[test]
    fn creating_notification_from_frame_fails_on_wrong_command() {
        let frame = AmsFrame::new(AmsCommand::PortConnect, [0, 0, 0, 0]);

        let err = RouterNotification::try_from(frame).unwrap_err();

        assert!(matches!(
            err,
            ProtocolError::UnexpectedAmsCommand {
                expected: AmsCommand::RouterNotification,
                got: AmsCommand::PortConnect
            }
        ));
    }
}
