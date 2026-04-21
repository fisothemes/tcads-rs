use super::{IndexGroup, IndexOffset, SumUpError};

/// A request to read a variable as part of a batch Sum Command.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SumReadReq {
    index_group: IndexGroup,
    index_offset: IndexOffset,
    length: u32,
}

impl SumReadReq {
    /// The fixed byte length of this request in the ADS payload.
    pub const LENGTH: usize = 12;

    /// Creates a new [`SumReadReq`] with the given parameters.
    pub fn new(index_group: IndexGroup, index_offset: IndexOffset, length: u32) -> Self {
        Self {
            index_group,
            index_offset,
            length,
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

    /// The expected maximum length of the data to read in bytes.
    pub fn length(&self) -> u32 {
        self.length
    }

    /// Writes this request to a byte buffer.
    pub fn write_to(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.index_group.to_le_bytes());
        buf.extend_from_slice(&self.index_offset.to_le_bytes());
        buf.extend_from_slice(&self.length.to_le_bytes());
    }

    /// Reads a [`SumReadReq`] from a byte buffer.
    pub fn from_bytes(bytes: [u8; Self::LENGTH]) -> Self {
        Self {
            index_group: IndexGroup::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            index_offset: IndexOffset::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            length: u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
        }
    }

    /// Converts this request to a byte buffer.
    pub fn to_bytes(&self) -> [u8; Self::LENGTH] {
        let mut buf = [0; Self::LENGTH];
        buf[0..4].copy_from_slice(&self.index_group.to_le_bytes());
        buf[4..8].copy_from_slice(&self.index_offset.to_le_bytes());
        buf[8..12].copy_from_slice(&self.length.to_le_bytes());
        buf
    }

    /// Parses a slice of bytes into a [`SumReadReq`].
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

impl From<SumReadReq> for [u8; SumReadReq::LENGTH] {
    fn from(req: SumReadReq) -> Self {
        req.to_bytes()
    }
}

impl From<[u8; SumReadReq::LENGTH]> for SumReadReq {
    fn from(bytes: [u8; SumReadReq::LENGTH]) -> Self {
        SumReadReq::from_bytes(bytes)
    }
}

impl TryFrom<&[u8]> for SumReadReq {
    type Error = SumUpError;
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        SumReadReq::try_from_slice(bytes)
    }
}
