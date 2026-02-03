use std::io::{self, Read, Write};
use std::sync::Arc;

use crate::constants::{
    AMS_HEADER_COMMAND_ID_RANGE, AMS_HEADER_ERROR_CODE_RANGE, AMS_HEADER_INVOKE_ID_RANGE,
    AMS_HEADER_LEN, AMS_HEADER_LENGTH_RANGE, AMS_HEADER_SOURCE_NETID_RANGE,
    AMS_HEADER_SOURCE_PORT_RANGE, AMS_HEADER_STATE_FLAGS_RANGE, AMS_HEADER_TARGET_NETID_RANGE,
    AMS_HEADER_TARGET_PORT_RANGE, AMS_TCP_HEADER_LEN, AMS_TCP_HEADER_LENGTH_RANGE,
    AMS_TCP_HEADER_RESERVED_RANGE,
};
use crate::errors::{AdsError, AdsReturnCode};
use crate::types::addr::{AmsAddr, AmsPort};
use crate::types::netid::AmsNetId;

use super::commands::CommandId;
use super::state_flags::StateFlag;

/// The 6-byte prefix for TCP communication.
///
/// Contains the total length of the AMS packet (AMS Header + ADS Data).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct AmsTcpHeader {
    reserved: u16,
    length: u32,
}

impl AmsTcpHeader {
    /// Creates a new TCP header.
    ///
    /// `length` is the size of the AMS Packet (AMS Header + ADS Data).
    pub const fn new(length: u32) -> Self {
        Self {
            reserved: 0,
            length,
        }
    }

    /// Creates a new TCP header with the given reserved and length fields.
    pub const fn with_reserved(reserved: u16, length: u32) -> Self {
        Self { reserved, length }
    }

    /// The reserved field is currently unused and must be set to zero.
    pub fn reserved(&self) -> u16 {
        self.reserved
    }

    /// The size of the AMS Packet (AMS Header + ADS Data).
    pub fn length(&self) -> u32 {
        self.length
    }

    /// Sets the size of the AMS Packet (AMS Header + ADS Data).
    pub fn set_length(&mut self, length: u32) {
        self.length = length;
    }

    /// Writes the 6 bytes to a writer (Little Endian).
    pub fn write_to<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(&self.reserved.to_le_bytes())?;
        w.write_all(&self.length.to_le_bytes())
    }

    /// Reads the 6 bytes from a reader (Little Endian).
    pub fn read_from<R: Read>(r: &mut R) -> io::Result<Self> {
        let mut buf = [0u8; AMS_TCP_HEADER_LEN];
        r.read_exact(&mut buf)?;
        Ok(Self::from(&buf))
    }
}

impl TryFrom<&[u8]> for AmsTcpHeader {
    type Error = AdsError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() < AMS_TCP_HEADER_LEN {
            return Err(AdsError::MalformedPacket(Arc::from(format!(
                "TCP Header data too short. Received {} bytes, expected {AMS_TCP_HEADER_LEN} bytes.",
                value.len()
            ))));
        }

        Ok(
            // Safely unwraps because length was checked.
            Self {
                reserved: u16::from_le_bytes(
                    value[AMS_TCP_HEADER_RESERVED_RANGE].try_into().unwrap(),
                ),
                length: u32::from_le_bytes(value[AMS_TCP_HEADER_LENGTH_RANGE].try_into().unwrap()),
            },
        )
    }
}

impl From<&[u8; AMS_TCP_HEADER_LEN]> for AmsTcpHeader {
    fn from(value: &[u8; AMS_TCP_HEADER_LEN]) -> Self {
        Self {
            reserved: u16::from_le_bytes(value[AMS_TCP_HEADER_RESERVED_RANGE].try_into().unwrap()),
            length: u32::from_le_bytes(value[AMS_TCP_HEADER_LENGTH_RANGE].try_into().unwrap()),
        }
    }
}

/// The AMS Packet Header structure (32 bytes).
///
/// Contains routing information, command IDs, flags, and error codes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AmsHeader {
    target: AmsAddr,
    source: AmsAddr,
    command_id: CommandId,
    state_flags: StateFlag,
    length: u32,
    error_code: AdsReturnCode,
    invoke_id: u32,
}

impl AmsHeader {
    pub fn new(
        target: AmsAddr,
        source: AmsAddr,
        command_id: CommandId,
        state_flags: StateFlag,
        length: u32,
        error_code: AdsReturnCode,
        invoke_id: u32,
    ) -> Self {
        Self {
            target,
            source,
            command_id,
            state_flags,
            length,
            error_code,
            invoke_id,
        }
    }

    /// Writes the 32 bytes to a writer using Little Endian.
    pub fn write_to<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(&self.target.net_id().0)?;
        w.write_all(&self.target.port().to_le_bytes())?;

        w.write_all(&self.source.net_id().0)?;
        w.write_all(&self.source.port().to_le_bytes())?;

        w.write_all(&u16::from(self.command_id).to_le_bytes())?;

        w.write_all(&u16::from(self.state_flags).to_le_bytes())?;

        w.write_all(&self.length.to_le_bytes())?;

        w.write_all(&u32::from(self.error_code).to_le_bytes())?;

        w.write_all(&self.invoke_id.to_le_bytes())
    }

    /// Reads the 32 bytes from a reader using Little Endian.
    pub fn read_from<R: Read>(r: &mut R) -> io::Result<Self> {
        let mut buf = [0u8; AMS_HEADER_LEN];
        r.read_exact(&mut buf)?;
        Self::try_from(&buf[..]).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// The AMSNetId of the station, for which the packet is intended.
    pub fn target(&self) -> &AmsAddr {
        &self.target
    }

    /// the AMSNetId of the station, from which the packet was sent.
    pub fn source(&self) -> &AmsAddr {
        &self.source
    }

    /// The Command ID identifies the type of request/response.
    pub fn command_id(&self) -> CommandId {
        self.command_id
    }

    /// State flags (Request/Response, TCP/UDP).
    pub fn state_flags(&self) -> StateFlag {
        self.state_flags
    }

    /// Size of the data range in bytes.
    pub fn length(&self) -> u32 {
        self.length
    }

    /// Sets the size of the data range in bytes.
    pub fn set_length(&mut self, length: u32) {
        self.length = length;
    }

    /// AMS error number. See ADS Return Codes.
    pub fn error_code(&self) -> AdsReturnCode {
        self.error_code
    }

    /// Free usable 32-bit array. Usually this array serves to send an ID.
    /// This ID makes it possible to assign a received response to a request.
    pub fn invoke_id(&self) -> u32 {
        self.invoke_id
    }
}

impl TryFrom<&[u8]> for AmsHeader {
    type Error = AdsError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() < AMS_HEADER_LEN {
            return Err(AdsError::MalformedPacket(Arc::from(
                "AMS Header data too short (expected {AMS_HEADER_LEN} bytes)",
            )));
        }

        Ok(Self {
            target: AmsAddr::new(
                AmsNetId::try_from(&value[AMS_HEADER_TARGET_NETID_RANGE])?,
                AmsPort::from_le_bytes(value[AMS_HEADER_TARGET_PORT_RANGE].try_into().unwrap()),
            ),
            source: AmsAddr::new(
                AmsNetId::try_from(&value[AMS_HEADER_SOURCE_NETID_RANGE])?,
                AmsPort::from_le_bytes(value[AMS_HEADER_SOURCE_PORT_RANGE].try_into().unwrap()),
            ),
            command_id: CommandId::from(u16::from_le_bytes(
                value[AMS_HEADER_COMMAND_ID_RANGE].try_into().unwrap(),
            )),
            state_flags: StateFlag::from(u16::from_le_bytes(
                value[AMS_HEADER_STATE_FLAGS_RANGE].try_into().unwrap(),
            )),
            length: u32::from_le_bytes(value[AMS_HEADER_LENGTH_RANGE].try_into().unwrap()),
            error_code: AdsReturnCode::from(u32::from_le_bytes(
                value[AMS_HEADER_ERROR_CODE_RANGE].try_into().unwrap(),
            )),
            invoke_id: u32::from_le_bytes(value[AMS_HEADER_INVOKE_ID_RANGE].try_into().unwrap()),
        })
    }
}

impl From<&AmsHeader> for [u8; AMS_HEADER_LEN] {
    fn from(value: &AmsHeader) -> Self {
        let mut buf = [0u8; AMS_HEADER_LEN];

        buf[AMS_HEADER_TARGET_NETID_RANGE].copy_from_slice(&value.target.net_id().0);
        buf[AMS_HEADER_TARGET_PORT_RANGE].copy_from_slice(&value.target.port().to_le_bytes());
        buf[AMS_HEADER_SOURCE_NETID_RANGE].copy_from_slice(&value.source.net_id().0);
        buf[AMS_HEADER_SOURCE_PORT_RANGE].copy_from_slice(&value.source.port().to_le_bytes());
        buf[AMS_HEADER_COMMAND_ID_RANGE]
            .copy_from_slice(&u16::from(value.command_id).to_le_bytes());
        buf[AMS_HEADER_STATE_FLAGS_RANGE]
            .copy_from_slice(&u16::from(value.state_flags).to_le_bytes());
        buf[AMS_HEADER_LENGTH_RANGE].copy_from_slice(&value.length.to_le_bytes());
        buf[AMS_HEADER_ERROR_CODE_RANGE]
            .copy_from_slice(&u32::from(value.error_code).to_le_bytes());
        buf[AMS_HEADER_INVOKE_ID_RANGE].copy_from_slice(&value.invoke_id.to_le_bytes());

        buf
    }
}

impl From<AmsHeader> for [u8; AMS_HEADER_LEN] {
    fn from(value: AmsHeader) -> Self {
        (&value).into()
    }
}

impl From<&AmsHeader> for Vec<u8> {
    fn from(value: &AmsHeader) -> Self {
        let bytes: [u8; AMS_HEADER_LEN] = value.into();
        bytes.to_vec()
    }
}

impl From<AmsHeader> for Vec<u8> {
    fn from(value: AmsHeader) -> Self {
        (&value).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_header() -> AmsHeader {
        AmsHeader::new(
            "127.0.0.1.1.1:851".parse().unwrap(),
            "192.168.0.2.1.1:40000".parse().unwrap(),
            CommandId::AdsRead,
            StateFlag::tcp_ads_response(),
            4,
            AdsReturnCode::Ok,
            12_345,
        )
    }

    #[test]
    fn test_tcp_header_too_short() {
        let bytes = vec![0x00, 0x00, 0x64];
        let result = AmsTcpHeader::try_from(bytes.as_slice());
        assert!(matches!(result, Err(AdsError::MalformedPacket(_))));
    }

    #[test]
    fn test_tcp_header_roundtrip() {
        let header = AmsTcpHeader::new(100);
        let mut buffer = Vec::new();

        header.write_to(&mut buffer).expect("Write failed");

        assert_eq!(buffer.len(), AMS_TCP_HEADER_LEN);
        // Reserved (0) + Length (100 = 0x64)
        assert_eq!(buffer, vec![0x00, 0x00, 0x64, 0x00, 0x00, 0x00]);

        let parsed = AmsTcpHeader::try_from(buffer.as_slice()).expect("Parse failed");
        assert_eq!(parsed, header);
        assert_eq!(parsed.length(), 100);
    }

    #[test]
    fn test_header_serialization_endianness() {
        let header = create_test_header();

        let bytes: [u8; AMS_HEADER_LEN] = (&header).into();

        // 1. Target NetId (6 bytes)
        assert_eq!(&bytes[AMS_HEADER_TARGET_NETID_RANGE], &[127, 0, 0, 1, 1, 1]);
        // 2. Target Port (2 bytes LE) - 851 = 0x0353
        assert_eq!(&bytes[AMS_HEADER_TARGET_PORT_RANGE], &[0x53, 0x03]);

        // 3. Source NetId (6 bytes)
        assert_eq!(
            &bytes[AMS_HEADER_SOURCE_NETID_RANGE],
            &[192, 168, 0, 2, 1, 1]
        );
        // 4. Source Port (2 bytes LE) - 40,000 = 0x9C40
        assert_eq!(&bytes[AMS_HEADER_SOURCE_PORT_RANGE], &[0x40, 0x9C]);

        // 5. Command ID (2 bytes LE) - AdsRead (2)
        assert_eq!(&bytes[AMS_HEADER_COMMAND_ID_RANGE], &[0x02, 0x00]);

        // 6. State Flags (2 bytes LE) - 1
        assert_eq!(&bytes[AMS_HEADER_STATE_FLAGS_RANGE], &[0x05, 0x00]);

        // 7. Length (4 bytes LE) - 4
        assert_eq!(&bytes[AMS_HEADER_LENGTH_RANGE], &[0x04, 0x00, 0x00, 0x00]);

        // 8. Error Code (4 bytes LE) - 0
        assert_eq!(
            &bytes[AMS_HEADER_ERROR_CODE_RANGE],
            &[0x00, 0x00, 0x00, 0x00]
        );

        // 9. Invoke ID (4 bytes LE) - 12,345 = 0x00003039
        assert_eq!(
            &bytes[AMS_HEADER_INVOKE_ID_RANGE],
            &[0x39, 0x30, 0x00, 0x00]
        );
    }

    #[test]
    fn test_header_roundtrip_parsing() {
        let original = create_test_header();

        let bytes: Vec<u8> = (&original).into();

        let parsed = AmsHeader::try_from(bytes.as_slice()).expect("Failed to parse valid header");

        assert_eq!(parsed.target(), original.target());
        assert_eq!(parsed.source(), original.source());
        assert_eq!(parsed.invoke_id(), original.invoke_id());
        assert_eq!(parsed.command_id(), original.command_id());

        assert_eq!(parsed, original);
    }

    #[test]
    fn test_write_to_matches_into_array() {
        let header = create_test_header();
        let mut writer_buf = Vec::new();
        header.write_to(&mut writer_buf).unwrap();
        let array_buf: [u8; AMS_HEADER_LEN] = (&header).into();
        assert_eq!(writer_buf, array_buf.to_vec());
    }
}
