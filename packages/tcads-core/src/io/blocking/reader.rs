use crate::ams::{AMS_TCP_HEADER_LEN, AmsTcpHeader};
use crate::io::frame::{AMS_FRAME_MAX_LEN, AmsFrame};
use std::io::{self, BufReader, Read};

/// A buffered reader specialised for parsing AMS frames from a byte stream.
///
/// This struct wraps an underlying reader in a [`BufReader`] to minimise system calls
/// when reading the [AMS/TCP header](AmsTcpHeader) (6 bytes) and variable-length payload.
pub struct AmsReader<R: Read> {
    reader: BufReader<R>,
}

impl<R: Read> AmsReader<R> {
    /// Creates a new AmsReader with [default buffering](BufReader::new).
    pub fn new(reader: R) -> Self {
        Self {
            reader: BufReader::new(reader),
        }
    }

    /// Creates a new AmsReader with a specific buffer capacity.
    pub fn with_capacity(reader: R, capacity: usize) -> Self {
        Self {
            reader: BufReader::with_capacity(capacity, reader),
        }
    }

    /// Reads a single AMS frame from the underlying stream.
    pub fn read_frame(&mut self) -> io::Result<AmsFrame> {
        let mut header_buf = [0u8; AMS_TCP_HEADER_LEN];
        self.reader.read_exact(&mut header_buf)?;
        let header = AmsTcpHeader::from(header_buf);

        let payload_len = header.length() as usize;
        if payload_len > AMS_FRAME_MAX_LEN {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Payload too large: {} bytes (max {})",
                    payload_len, AMS_FRAME_MAX_LEN
                ),
            ));
        }

        let mut payload = vec![0u8; payload_len];
        self.reader.read_exact(&mut payload)?;

        Ok(AmsFrame::from_parts(header, payload))
    }

    /// Returns an iterator over incoming frames.
    ///
    /// This is analogous to [`TcpListener::incoming()`](std::net::TcpListener::incoming), but for AMS frames.
    ///
    /// # Example
    /// ```no_run
    /// use tcads_core::io::blocking::AmsStream;
    /// use tcads_core::ams::AmsCommand;
    /// use tcads_core::error::Error;
    ///
    /// fn run_client(tcp: std::net::TcpStream) -> Result<(), Error> {
    ///     let stream = AmsStream::new(tcp);
    ///     let (reader, mut writer) = stream.try_split()?;
    ///
    ///     // "TcpListener" style loop
    ///     for frame_res in reader.incoming() {
    ///         // 1. Check for transport errors
    ///         let frame = frame_res?;
    ///
    ///         // 2. User decides how to deal with the frame
    ///         match frame.header().command() {
    ///             AmsCommand::AdsCommand => {
    ///                 println!("Got ADS packet, invoking dispatcher...");
    ///             },
    ///             AmsCommand::RouterNotification => {
    ///                 println!("Router state changed!");
    ///             },
    ///             AmsCommand::PortConnect => {
    ///                 println!("Received Port Connect response");
    ///             },
    ///             cmd => {
    ///                 println!("Received unknown command: {:?}", cmd);
    ///             }
    ///         }
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn incoming(self) -> AmsIncoming<R> {
        AmsIncoming { reader: self }
    }

    /// Consumes the AmsReader, returning the underlying reader.
    ///
    /// # Note
    ///
    /// Any leftover data in the internal buffer is lost.
    /// Therefore, a following read from the underlying reader may lead to data loss
    pub fn into_inner(self) -> R {
        self.reader.into_inner()
    }
}

/// An iterator that yields `std::io::Result<AmsFrame>` from the underlying stream.
pub struct AmsIncoming<R: Read> {
    reader: AmsReader<R>,
}
impl<R: Read> Iterator for AmsIncoming<R> {
    type Item = io::Result<AmsFrame>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.reader.read_frame() {
            Ok(frame) => Some(Ok(frame)),
            // EOF is expected when the connection is closed
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => None,
            Err(e) => Some(Err(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ams::AmsCommand;
    use std::io::Cursor;

    #[test]
    fn read_frame_reads_one_frame() {
        let data = AmsFrame::new(AmsCommand::PortConnect, [0x01, 0x02]).to_vec();
        let mut cursor = Cursor::new(data);
        let mut reader = AmsReader::new(&mut cursor);

        let frame = reader.read_frame().expect("Should read valid frame");

        assert_eq!(frame.header().command(), AmsCommand::PortConnect);
        assert_eq!(frame.header().length(), 2);
        assert_eq!(frame.payload(), &[0x01, 0x02]);
    }

    #[test]
    fn test_incoming_iterator() {
        let mut data = Vec::new();
        data.extend_from_slice(&AmsFrame::new(AmsCommand::AdsCommand, [0xAA]).to_vec());
        data.extend_from_slice(&AmsFrame::new(AmsCommand::GetLocalNetId, [0xBB, 0xCC]).to_vec());

        let cursor = Cursor::new(data);
        let reader = AmsReader::new(cursor);
        let mut iter = reader.incoming();

        let f1 = iter
            .next()
            .expect("Should have frame 1")
            .expect("Should be Ok");
        assert_eq!(f1.header().command(), AmsCommand::AdsCommand);
        assert_eq!(f1.payload(), &[0xAA]);

        let f2 = iter
            .next()
            .expect("Should have frame 2")
            .expect("Should be Ok");
        assert_eq!(f2.header().command(), AmsCommand::GetLocalNetId);
        assert_eq!(f2.payload(), &[0xBB, 0xCC]);

        assert!(iter.next().is_none(), "Iterator should end on EOF");
    }

    #[test]
    fn test_unexpected_eof() {
        let mut data = AmsFrame::new(AmsCommand::AdsCommand, [0x01, 0x02, 0x03, 0x04]).to_vec();
        data.pop();
        data.pop();

        let cursor = Cursor::new(data);
        let mut reader = AmsReader::new(cursor);

        let result = reader.read_frame();

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::UnexpectedEof);
    }
}
