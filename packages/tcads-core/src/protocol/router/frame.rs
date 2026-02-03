use std::io;
use std::io::{Read, Write};
use std::sync::Arc;

use crate::constants::{AMS_PACKET_MAX_LEN, AMS_TCP_HEADER_LEN};
use crate::prelude::{AdsError, AmsTcpHeader};

use super::AmsRouterCommand;

/// A raw AMS Router frame (AMS/TCP header + router payload).
///
/// The TCP header's `reserved` field acts as the router command/flag, and the
/// TCP header's `length` is the router payload length in bytes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AmsRouterFrame<B = Vec<u8>> {
    command: AmsRouterCommand,
    payload: B,
}

impl<B> AmsRouterFrame<B> {
    /// Creates a new router frame.
    pub const fn new(command: AmsRouterCommand, payload: B) -> Self {
        Self { command, payload }
    }

    /// Returns the router command.
    pub const fn command(&self) -> AmsRouterCommand {
        self.command
    }

    /// Returns a reference to the payload.
    pub fn payload(&self) -> &B {
        &self.payload
    }

    /// Returns the command and payload, consuming the frame.
    pub fn into_parts(self) -> (AmsRouterCommand, B) {
        (self.command, self.payload)
    }
}

impl<B: AsRef<[u8]>> AmsRouterFrame<B> {
    /// Returns the AMS/TCP header for this router frame.
    pub fn tcp_header(&self) -> AmsTcpHeader {
        AmsTcpHeader::with_reserved(u16::from(self.command), self.payload.as_ref().len() as u32)
    }

    /// Writes the full wire format: AMS/TCP header + router payload.
    pub fn write_to<W: Write>(&self, w: &mut W) -> io::Result<usize> {
        let payload = self.payload.as_ref();
        self.tcp_header().write_to(w)?;
        w.write_all(payload)?;
        Ok(AMS_TCP_HEADER_LEN + payload.len())
    }
}

impl<B: From<Vec<u8>>> AmsRouterFrame<B> {
    /// Reads a router frame from a stream, allocating a new payload buffer.
    pub fn read_from<R: Read>(r: &mut R) -> Result<Self, AdsError> {
        let mut tcp_buf = [0u8; AMS_TCP_HEADER_LEN];
        r.read_exact(&mut tcp_buf)?;
        let tcp = AmsTcpHeader::try_from(&tcp_buf[..])?;

        let command = AmsRouterCommand::from(tcp.reserved());
        let len = tcp.length() as usize;

        if len > AMS_PACKET_MAX_LEN {
            return Err(AdsError::MalformedPacket(Arc::from(format!(
                "Router payload larger than maximum allowed packet size of {AMS_PACKET_MAX_LEN} bytes ({len} bytes received)"
            ))));
        }

        let mut payload = vec![0u8; len];
        r.read_exact(&mut payload)?;

        Ok(Self::new(command, B::from(payload)))
    }
}

impl<B: AsMut<[u8]> + AsRef<[u8]>> AmsRouterFrame<B> {
    /// Reads a router frame into the existing payload buffer.
    ///
    /// Returns the number of bytes written into the payload buffer.
    pub fn read_into<R: Read>(&mut self, r: &mut R) -> Result<usize, AdsError> {
        let mut tcp_buf = [0u8; AMS_TCP_HEADER_LEN];
        r.read_exact(&mut tcp_buf)?;
        let tcp = AmsTcpHeader::try_from(&tcp_buf[..])?;

        let command = AmsRouterCommand::from(tcp.reserved());
        let len = tcp.length() as usize;

        if len > AMS_PACKET_MAX_LEN {
            return Err(AdsError::MalformedPacket(Arc::from(format!(
                "Router payload larger than maximum allowed packet size of {AMS_PACKET_MAX_LEN} bytes ({len} bytes received)"
            ))));
        }

        if len > self.payload.as_ref().len() {
            return Err(AdsError::MalformedPacket(Arc::from(
                "Router frame too large for buffer",
            )));
        }

        self.command = command;
        let dest = &mut self.payload.as_mut()[0..len];
        r.read_exact(dest)?;
        Ok(len)
    }
}
