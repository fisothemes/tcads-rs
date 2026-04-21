use super::{AdsNotificationAttrib, IndexGroup, IndexOffset, SumUpError};

/// A request to subscribe to a variable's changes via a batch Sum Command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SumAddNotificationReq {
    index_group: IndexGroup,
    index_offset: IndexOffset,
    attributes: AdsNotificationAttrib,
}

impl SumAddNotificationReq {
    /// The fixed byte length of this request (4 + 4 + 16 bytes).
    pub const LENGTH: usize = 24;

    pub fn new(
        index_group: IndexGroup,
        index_offset: IndexOffset,
        attributes: AdsNotificationAttrib,
    ) -> Self {
        Self {
            index_group,
            index_offset,
            attributes,
        }
    }

    /// The Index Group of the target variable.
    pub fn index_group(&self) -> IndexGroup {
        self.index_group
    }

    /// The Index Offset of the target variable.
    pub fn index_offset(&self) -> IndexOffset {
        self.index_offset
    }

    /// The transmission mode and cycle times for the notification.
    pub fn attributes(&self) -> &AdsNotificationAttrib {
        &self.attributes
    }

    /// Writes this request to a byte buffer.
    pub fn write_to(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.to_bytes());
    }

    /// Reads a [`SumAddNotificationReq`] from a byte buffer.
    pub fn from_bytes(bytes: [u8; Self::LENGTH]) -> Self {
        Self {
            index_group: IndexGroup::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            index_offset: IndexOffset::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            attributes: AdsNotificationAttrib::from_bytes([
                bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14],
                bytes[15], bytes[16], bytes[17], bytes[18], bytes[19], bytes[20], bytes[21],
                bytes[22], bytes[23],
            ]),
        }
    }

    /// Converts this request to a byte buffer.
    pub fn to_bytes(&self) -> [u8; Self::LENGTH] {
        let mut buf = [0; Self::LENGTH];
        buf[0..4].copy_from_slice(&self.index_group.to_le_bytes());
        buf[4..8].copy_from_slice(&self.index_offset.to_le_bytes());
        buf[8..24].copy_from_slice(&self.attributes.to_bytes());
        buf
    }

    /// Parses a slice of bytes into a [`SumAddNotificationReq`].
    pub fn try_from_slice(bytes: &[u8]) -> Result<Self, SumUpError> {
        if bytes.len() != Self::LENGTH {
            return Err(SumUpError::UnexpectedLength {
                expected: Self::LENGTH,
                got: bytes.len(),
            });
        }
        let mut buf = [0u8; Self::LENGTH];
        buf.copy_from_slice(bytes);
        Ok(Self::from_bytes(buf))
    }
}

impl From<SumAddNotificationReq> for [u8; SumAddNotificationReq::LENGTH] {
    fn from(req: SumAddNotificationReq) -> Self {
        req.to_bytes()
    }
}

impl From<[u8; SumAddNotificationReq::LENGTH]> for SumAddNotificationReq {
    fn from(bytes: [u8; SumAddNotificationReq::LENGTH]) -> Self {
        SumAddNotificationReq::from_bytes(bytes)
    }
}

impl TryFrom<&[u8]> for SumAddNotificationReq {
    type Error = SumUpError;
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        SumAddNotificationReq::try_from_slice(bytes)
    }
}
