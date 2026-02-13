use super::command::AmsCommand;
use super::error::AmsTcpHeaderError;

/// The 6-byte prefix for TCP communication.
///
/// See [Beckhoff ADS Specification (TE1000)](https://infosys.beckhoff.com/content/1033/tc3_ads_intro/115846283.html?id=5591912318145837195).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AmsTcpHeader {
    command: AmsCommand,
    length: u32,
}

impl AmsTcpHeader {
    /// Length of the AMS/TCP header in bytes.
    pub const LENGTH: usize = 6;

    /// Constructs a new AmsTcpHeader.
    pub fn new(command: AmsCommand, length: u32) -> Self {
        Self { command, length }
    }

    /// Returns the AmsCommand.
    pub fn command(&self) -> AmsCommand {
        self.command
    }

    /// Returns the length of the payload (excluding the 6-byte header).
    pub fn length(&self) -> u32 {
        self.length
    }

    /// Creates a new AmsTcpHeader from a byte array.
    pub fn from_bytes(bytes: [u8; AmsTcpHeader::LENGTH]) -> Self {
        Self::from(bytes)
    }

    /// Converts the current instance into a byte array.
    pub fn to_bytes(&self) -> [u8; AmsTcpHeader::LENGTH] {
        self.into()
    }

    pub fn try_from_slice(bytes: &[u8]) -> Result<Self, AmsTcpHeaderError> {
        Self::try_from(bytes)
    }
}

impl From<&AmsTcpHeader> for [u8; AmsTcpHeader::LENGTH] {
    fn from(value: &AmsTcpHeader) -> Self {
        let mut buf = [0u8; AmsTcpHeader::LENGTH];
        buf[..2].copy_from_slice(&u16::from(value.command).to_le_bytes());
        buf[2..AmsTcpHeader::LENGTH].copy_from_slice(&value.length.to_le_bytes());
        buf
    }
}

impl From<[u8; AmsTcpHeader::LENGTH]> for AmsTcpHeader {
    fn from(value: [u8; AmsTcpHeader::LENGTH]) -> Self {
        Self {
            command: AmsCommand::from(u16::from_le_bytes(value[0..2].try_into().unwrap())),
            length: u32::from_le_bytes(value[2..AmsTcpHeader::LENGTH].try_into().unwrap()),
        }
    }
}

impl TryFrom<&[u8]> for AmsTcpHeader {
    type Error = AmsTcpHeaderError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() < AmsTcpHeader::LENGTH {
            return Err(AmsTcpHeaderError::BufferTooSmall {
                expected: AmsTcpHeader::LENGTH,
                found: value.len(),
            });
        }

        let value: [u8; AmsTcpHeader::LENGTH] = value[..AmsTcpHeader::LENGTH].try_into().unwrap();

        Ok(Self::from(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_new_and_accessors() {
        let header = AmsTcpHeader::new(AmsCommand::PortConnect, 0x1234_5678);
        assert_eq!(header.command(), AmsCommand::PortConnect);
        assert_eq!(header.length(), 0x1234_5678);
    }

    #[test]
    fn test_to_bytes_and_from_bytes_roundtrip() {
        let header = AmsTcpHeader::new(AmsCommand::RouterNotification, 0xA1B2_C3D4);
        let bytes = header.to_bytes();
        assert_eq!(bytes, [0x01, 0x10, 0xD4, 0xC3, 0xB2, 0xA1]);

        let parsed = AmsTcpHeader::from_bytes(bytes);
        assert_eq!(parsed, header);
    }

    #[test]
    fn test_try_from_slice_too_small() {
        let err = AmsTcpHeader::try_from(&[0u8; AmsTcpHeader::LENGTH - 1][..]).unwrap_err();

        assert_eq! {
            err,
            AmsTcpHeaderError::BufferTooSmall {
                expected: AmsTcpHeader::LENGTH,
                found: AmsTcpHeader::LENGTH - 1,
            }
        }
    }
}
