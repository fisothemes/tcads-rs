use crate::ams::{AMS_TCP_HEADER_LEN, AmsCommand, AmsTcpHeader};

/// Maximum allowed AMS frame/packet size (64KB) to prevent allocation attacks.
pub const AMS_FRAME_MAX_LEN: usize = 65535 - AMS_TCP_HEADER_LEN;

/// A single AMS frame/packet consisting of a header and a payload.
///
/// This struct is I/O-agnostic and simply holds the frame data.
/// Reading and writing frames is handled by the I/O layer
/// ([`blocking`](crate::io::blocking) or [`tokio`](crate::io::tokio)).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AmsFrame {
    header: AmsTcpHeader,
    payload: Vec<u8>,
}

impl AmsFrame {
    /// Creates a new frame with the given command and payload.
    ///
    /// # Panics
    ///
    /// Panics if the payload exceeds [`AMS_FRAME_MAX_LEN`].
    /// Use [`AmsFrame::try_new`] for fallible construction.
    pub fn new(command: AmsCommand, payload: impl Into<Vec<u8>>) -> Self {
        let payload = payload.into();
        assert!(
            payload.len() <= AMS_FRAME_MAX_LEN,
            "Payload too large: {} bytes (max {})",
            payload.len(),
            AMS_FRAME_MAX_LEN
        );
        Self {
            header: AmsTcpHeader::new(command, payload.len() as u32),
            payload,
        }
    }

    /// Creates a new frame with the given command and payload.
    ///
    /// Returns `None` if the payload exceeds [`AMS_FRAME_MAX_LEN`].
    pub fn try_new(command: AmsCommand, payload: impl Into<Vec<u8>>) -> Option<Self> {
        let payload = payload.into();
        if payload.len() > AMS_FRAME_MAX_LEN {
            return None;
        }

        Some(Self {
            header: AmsTcpHeader::new(command, payload.len() as u32),
            payload,
        })
    }

    /// Creates a new frame with the given command and empty payload.
    pub fn empty(command: AmsCommand) -> Self {
        Self::new(command, Vec::new())
    }

    /// Constructs a frame directly from a header and payload.
    ///
    /// # Safety
    ///
    /// This does NOT validate that `payload.len()` matches `header.length()`.
    /// It's the caller's responsibility to ensure consistency.
    ///
    /// This is primarily intended for use by I/O readers that have already
    /// read the exact payload length specified in the header.
    pub fn from_parts(header: AmsTcpHeader, payload: impl Into<Vec<u8>>) -> Self {
        Self {
            header,
            payload: payload.into(),
        }
    }

    /// Constructs a frame from a header and payload, validating consistency.
    ///
    /// Returns `None` if the payload length doesn't match the header length.
    pub fn try_from_parts(header: AmsTcpHeader, payload: impl Into<Vec<u8>>) -> Option<Self> {
        let payload = payload.into();
        if payload.len() != header.length() as usize {
            return None;
        }
        Some(Self { header, payload })
    }

    /// Returns the frame's header.
    pub fn header(&self) -> AmsTcpHeader {
        self.header
    }

    /// Returns the frame's payload.
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    /// Splits the frame into its header and payload.
    pub fn into_parts(self) -> (AmsTcpHeader, Vec<u8>) {
        (self.header, self.payload)
    }

    /// Returns the total size of this frame in bytes (header + payload).
    pub fn total_size(&self) -> usize {
        AMS_TCP_HEADER_LEN + self.payload.len()
    }

    /// Serializes the frame into a byte vector.
    ///
    /// This is useful for testing or when you need the raw bytes.
    /// For I/O, prefer using an `AmsWriter` which can use vectored I/O
    /// depending on implementation.
    pub fn to_vec(&self) -> Vec<u8> {
        let mut vec = Vec::with_capacity(AMS_TCP_HEADER_LEN + self.payload.len());
        vec.extend_from_slice(&self.header.to_bytes());
        vec.extend_from_slice(&self.payload);
        vec
    }
}

impl From<(AmsCommand, Vec<u8>)> for AmsFrame {
    fn from((command, payload): (AmsCommand, Vec<u8>)) -> Self {
        Self::new(command, payload)
    }
}

impl From<AmsFrame> for (AmsCommand, Vec<u8>) {
    fn from(frame: AmsFrame) -> Self {
        (frame.header.command(), frame.payload)
    }
}

impl From<AmsFrame> for Vec<u8> {
    fn from(frame: AmsFrame) -> Self {
        frame.to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_frame_with_correct_header() {
        let frame = AmsFrame::new(AmsCommand::PortConnect, vec![1, 2, 3, 4]);

        assert_eq!(frame.header().command(), AmsCommand::PortConnect);
        assert_eq!(frame.header().length(), 4);
        assert_eq!(frame.payload(), &[1, 2, 3, 4]);
    }

    #[test]
    #[should_panic(expected = "Payload too large")]
    fn new_panics_on_oversized_payload() {
        AmsFrame::new(AmsCommand::AdsCommand, vec![0u8; AMS_FRAME_MAX_LEN + 1]);
    }

    #[test]
    fn try_new_returns_none_on_oversized_payload() {
        let result = AmsFrame::try_new(AmsCommand::AdsCommand, vec![0u8; AMS_FRAME_MAX_LEN + 1]);
        assert!(result.is_none());
    }

    #[test]
    fn try_new_succeeds_on_valid_payload() {
        let result = AmsFrame::try_new(AmsCommand::PortConnect, vec![1, 2, 3]);
        assert!(result.is_some());
    }

    #[test]
    fn empty_creates_frame_with_no_payload() {
        let frame = AmsFrame::empty(AmsCommand::GetLocalNetId);

        assert_eq!(frame.header().command(), AmsCommand::GetLocalNetId);
        assert_eq!(frame.header().length(), 0);
        assert_eq!(frame.payload(), &[]);
    }

    #[test]
    fn from_parts_creates_frame() {
        let header = AmsTcpHeader::new(AmsCommand::PortClose, 2);
        let payload = vec![0xAA, 0xBB];

        let frame = AmsFrame::from_parts(header, payload.clone());

        assert_eq!(frame.header().command(), AmsCommand::PortClose);
        assert_eq!(frame.payload(), payload.as_slice());
    }

    #[test]
    fn try_from_parts_validates_length() {
        let header = AmsTcpHeader::new(AmsCommand::PortClose, 2);

        // Correct length
        let frame = AmsFrame::try_from_parts(header, vec![1, 2]);
        assert!(frame.is_some());

        // Wrong length
        let frame = AmsFrame::try_from_parts(header, vec![1, 2, 3]);
        assert!(frame.is_none());
    }

    #[test]
    fn into_parts_returns_components() {
        let frame = AmsFrame::new(AmsCommand::RouterNotification, [1, 0, 0, 0]);
        let (header, payload) = frame.into_parts();

        assert_eq!(header.command(), AmsCommand::RouterNotification);
        assert_eq!(header.length(), 4);
        assert_eq!(payload, vec![1, 0, 0, 0]);
    }

    #[test]
    fn to_bytes_serializes_correctly() {
        let frame = AmsFrame::new(AmsCommand::PortConnect, [0x12, 0x34]);
        let bytes = frame.to_vec();

        // Header: [0x00, 0x10] (PortConnect) + [0x02, 0x00, 0x00, 0x00] (length=2)
        // Payload: [0x12, 0x34]
        assert_eq!(bytes, vec![0x00, 0x10, 0x02, 0x00, 0x00, 0x00, 0x12, 0x34]);
    }

    #[test]
    fn total_size_is_correct() {
        let frame = AmsFrame::new(AmsCommand::GetLocalNetId, [1, 2, 3, 4]);
        assert_eq!(frame.total_size(), AMS_TCP_HEADER_LEN + 4);
    }
}
