use super::reader::AmsReader;
use super::traits::WriteAllVectored;
use super::writer::AmsWriter;
use crate::ams::{AMS_TCP_HEADER_LEN, AmsTcpHeader};
use crate::io::frame::{AMS_FRAME_MAX_LEN, AmsFrame};
use std::io::{self, IoSlice, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::time::Duration;

/// A stream wrapper for communicating with an AMS Router.
///
/// This struct serves as the main entry point for an ADS connection. It wraps a raw byte stream
/// (typically a [`TcpStream`]) and provides methods to read and write [`AmsFrame`]s.
pub struct AmsStream<S: Read + Write = TcpStream> {
    stream: S,
}

impl<S: Read + Write> AmsStream<S> {
    /// Creates a new instance of the AmsStream given a stream.
    pub fn new(stream: S) -> Self {
        Self { stream }
    }

    /// Reads a frame directly from the stream without internal buffering.
    ///
    /// # Note
    ///
    /// This function performs two read calls (one for the header, one for the payload).
    /// If you are reading frames in a tight loop, prefer using [`split`](AmsStream::split)
    /// or [`try_split`](AmsStream::try_split) to get an [`AmsReader`], which buffers reads
    /// to minimise system calls.
    pub fn read_frame(&mut self) -> io::Result<AmsFrame> {
        let mut header_buf = [0u8; AMS_TCP_HEADER_LEN];
        self.stream.read_exact(&mut header_buf)?;
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
        self.stream.read_exact(&mut payload)?;

        Ok(AmsFrame::from_parts(header, payload))
    }

    /// Writes a frame directly to the stream using vectored I/O.
    ///
    /// This method attempts to send the header and payload in a single system call
    /// (if supported by the OS) to avoid TCP fragmentation or Nagle's algorithm delays.
    pub fn write_frame(&mut self, frame: &AmsFrame) -> io::Result<()> {
        let header_bytes = frame.header().to_bytes();
        let mut bufs = [IoSlice::new(&header_bytes), IoSlice::new(frame.payload())];

        WriteAllVectored::write_all_vectored(&mut self.stream, &mut bufs)?;
        self.stream.flush()
    }

    /// Consumes the AmsStream and returns the underlying stream.
    pub fn into_inner(self) -> S {
        self.stream
    }
}

impl<S: Read + Write + Clone> AmsStream<S> {
    /// Splits the stream into a buffered Reader and buffered Writer.
    pub fn split<R: Read, W: Write>(self) -> (AmsReader<S>, AmsWriter<S>) {
        (
            AmsReader::new(self.stream.clone()),
            AmsWriter::new(self.stream),
        )
    }
}

impl AmsStream<TcpStream> {
    /// Connects to an AMS router at the specified address.
    ///
    /// This convenience method that:
    ///
    /// 1. Establishes a [`TcpStream`] connection.
    /// 2. **Disables Nagle's algorithm** (`set_nodelay(true)`). This is critical for ADS
    ///    performance, preventing 200ms latency spikes on small Read/Write requests.
    /// 3. Wraps the stream in an [`AmsStream`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use tcads_core::io::blocking::AmsStream;
    ///
    /// let stream = AmsStream::connect("127.0.0.1:48898").unwrap();
    /// ```
    pub fn connect<A: std::net::ToSocketAddrs>(addr: A) -> io::Result<Self> {
        let stream = TcpStream::connect(addr)?;
        stream.set_nodelay(true)?;
        Ok(Self::new(stream))
    }

    /// Splits the `TcpStream` into a buffered Reader and buffered Writer.
    ///
    /// This allows reading and writing to occur on separate threads or logic paths.
    pub fn try_split(self) -> io::Result<(AmsReader<TcpStream>, AmsWriter<TcpStream>)> {
        Ok((
            AmsReader::new(self.stream.try_clone()?),
            AmsWriter::new(self.stream),
        ))
    }

    /// Disables Nagle's algorithm (TCP_NODELAY).
    ///
    /// **Recommendation:** Set this to `true` for ADS to avoid 200ms latency on small requests.
    pub fn set_nodelay(&self, nodelay: bool) -> io::Result<()> {
        self.stream.set_nodelay(nodelay)
    }

    /// Sets the read timeout for the underlying stream.
    pub fn set_read_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
        self.stream.set_read_timeout(dur)
    }

    /// Sets the write timeout for the underlying stream.
    pub fn set_write_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
        self.stream.set_write_timeout(dur)
    }
    /// Returns the socket address of the remote peer of this TCP connection.
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.stream.peer_addr()
    }

    // Returns the socket address of the local half of this TCP connection
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.stream.local_addr()
    }

    /// Shuts down the read, write, or both halves of this connection.
    /// This function will cause all pending and future I/O on the specified
    /// portions to return immediately with an appropriate value
    /// (see documentation for [`Shutdown`]).
    pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        self.stream.shutdown(how)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ams::AmsCommand;
    use std::io::Cursor;

    #[test]
    fn test_stream_generic_read_write() {
        let incoming_data = vec![
            0x00, 0x10, // Command: Port Connect (0x1000)
            0x02, 0x00, 0x00, 0x00, // Length: 2 bytes
            0x01, 0x01, // Payload: 01 01
        ];

        let mock_socket = Cursor::new(incoming_data);
        let mut stream = AmsStream::new(mock_socket);

        let received = stream.read_frame().expect("Read should succeed");
        assert_eq!(received.header().command(), AmsCommand::PortConnect);
        assert_eq!(received.payload(), &[0x01, 0x01]);

        let resp_frame = AmsFrame::new(AmsCommand::PortClose, vec![0xFF]);
        stream
            .write_frame(&resp_frame)
            .expect("Write should succeed");

        // 3. Verify underlying buffer content
        let underlying = stream.into_inner();
        let buffer = underlying.into_inner(); // Extract Vec from Cursor

        let expected_tail = [
            0x01, 0x00, // Command: Port Close (0x0001)
            0x01, 0x00, 0x00, 0x00, // Length: 1 byte
            0xFF, // Payload: FF
        ];

        assert_eq!(&buffer[8..], expected_tail);
    }
}
