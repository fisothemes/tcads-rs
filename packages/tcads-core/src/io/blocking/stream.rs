use super::reader::AmsReader;
use super::writer::AmsWriter;
use crate::io::frame::AmsFrame;
use std::io::{self, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::time::Duration;

pub struct AmsStream<S: Read + Write = TcpStream> {
    stream: S,
}

impl<S: Read + Write> AmsStream<S> {
    /// Creates a new instance of the AmsStream given a stream.
    pub fn new(stream: S) -> Self {
        Self { stream }
    }

    /// Reads a frame directly from the stream.
    ///
    /// Note: If you are doing this in a loop, prefer [`split`](AmsStream::split)
    /// or [`try_split`](AmsStream::try_split) to get a buffered [`AmsReader`].
    pub fn read_frame(&mut self) -> io::Result<AmsFrame> {
        AmsFrame::read_from(&mut self.stream)
    }

    /// Writes a frame directly to the stream.
    pub fn write_frame(&mut self, frame: &AmsFrame) -> io::Result<()> {
        frame.write_to(&mut self.stream)
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
    /// Splits the stream into a buffered Reader and buffered Writer.
    ///
    /// This is the preferred way to handle the main connection loop.
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
    /// (see the documentation of [Shutdown]).
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
