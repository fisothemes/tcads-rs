use super::reader::AmsReader;
use super::traits::WriteAllVectored;
use super::writer::AmsWriter;
use crate::ams::{AMS_TCP_HEADER_LEN, AmsTcpHeader};
use crate::io::frame::{AMS_FRAME_MAX_LEN, AmsFrame};
use std::io::IoSlice;
use std::net::SocketAddr;
use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{self, TcpStream};

/// A stream wrapper for communicating with an AMS Router asynchronously.
///
/// This struct serves as the main entry point for an ADS connection. It wraps a raw byte stream
/// (typically a [`TcpStream`]) and provides methods to read and write [`AmsFrame`]s.
pub struct AmsStream<S: AsyncRead + AsyncWrite + Unpin = TcpStream> {
    stream: S,
}

impl<S: AsyncRead + AsyncWrite + Unpin> AmsStream<S> {
    /// Creates a new instance of the AmsStream given a stream.
    pub fn new(stream: S) -> Self {
        Self { stream }
    }

    /// Consumes the AmsStream and returns the underlying stream.
    pub fn into_inner(self) -> S {
        self.stream
    }

    /// Reads a frame directly from the stream without internal buffering.
    ///
    /// # Note
    ///
    /// This function performs two read calls (one for the header, one for the payload).
    /// If you are reading frames in a tight loop, prefer using [`split`](AmsStream::split) or
    /// [`into_split`](AmsStream::into_split) to get an [`AmsReader`], which buffers
    /// reads to minimise system calls and handles clean EOFs more gracefully.
    pub async fn read_frame(&mut self) -> io::Result<AmsFrame> {
        let mut header_buf = [0u8; AMS_TCP_HEADER_LEN];
        self.stream.read_exact(&mut header_buf).await?;
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
        self.stream.read_exact(&mut payload).await?;

        Ok(AmsFrame::from_parts(header, payload))
    }

    /// Writes a frame directly to the stream using vectored I/O.
    ///
    /// This method attempts to send the header and payload in a single system call
    /// (if supported by the OS) and **flushes immediately** to avoid TCP fragmentation
    /// or Nagle's algorithm delays.
    pub async fn write_frame(&mut self, frame: &AmsFrame) -> io::Result<()> {
        let header_bytes = frame.header().to_bytes();
        let mut bufs = [IoSlice::new(&header_bytes), IoSlice::new(frame.payload())];

        WriteAllVectored::write_all_vectored(&mut self.stream, &mut bufs).await?;
        self.stream.flush().await
    }

    /// Splits the stream into a buffered Reader and buffered Writer.
    ///
    /// This uses [`tokio::io::split`](tokio::io::split) internally, which wraps the stream in a `Mutex` / `Arc`
    /// to allow concurrent access. For `TcpStream`, prefer using [`into_split`](AmsStream::into_split)
    /// (if available on the specific impl) for zero-overhead splitting.
    pub fn split(self) -> (AmsReader<io::ReadHalf<S>>, AmsWriter<io::WriteHalf<S>>) {
        let (reader, writer) = io::split(self.stream);
        (AmsReader::new(reader), AmsWriter::new(writer))
    }
}

impl AmsStream<TcpStream> {
    /// Connects to an AMS router at the specified address.
    ///
    /// This convenience method:
    ///
    /// 1. Establishes a [`TcpStream`] connection.
    /// 2. **Disables Nagle's algorithm** (`set_nodelay(true)`). This is critical for ADS
    ///    performance to prevent latency spikes on small Read/Write requests.
    /// 3. Wraps the stream in an [`AmsStream`].
    pub async fn connect<A: net::ToSocketAddrs>(addr: A) -> io::Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        stream.set_nodelay(true)?;
        Ok(Self::new(stream))
    }

    /// Splits the `TcpStream` into a buffered Reader and buffered Writer.
    ///
    /// This uses [`TcpStream::into_split`] for zero-overhead splitting (unlike the generic `split` method).
    pub fn into_split(
        self,
    ) -> (
        AmsReader<net::tcp::OwnedReadHalf>,
        AmsWriter<net::tcp::OwnedWriteHalf>,
    ) {
        let (reader, writer) = self.stream.into_split();
        (AmsReader::new(reader), AmsWriter::new(writer))
    }

    /// Disables Nagle's algorithm (TCP_NODELAY).
    pub fn set_nodelay(&self, nodelay: bool) -> io::Result<()> {
        self.stream.set_nodelay(nodelay)
    }

    /// Returns the socket address of the remote peer of this TCP connection.
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.stream.peer_addr()
    }

    /// Returns the socket address of the local half of this TCP connection.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.stream.local_addr()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ams::AmsCommand;

    #[tokio::test]
    async fn test_stream_generic_read_write() {
        // Use duplex to simulate a TCP stream
        let (client, mut server) = io::duplex(1024);
        let mut stream = AmsStream::new(client);

        let incoming_data = [
            0x00, 0x10, // Command: Port Connect (0x1000)
            0x02, 0x00, 0x00, 0x00, // Length: 2 bytes
            0x01, 0x01, // Payload: 01 01
        ];
        server.write_all(&incoming_data).await.unwrap();

        let received = stream.read_frame().await.expect("Read should succeed");
        assert_eq!(received.header().command(), AmsCommand::PortConnect);
        assert_eq!(received.payload(), &[0x01, 0x01]);

        let resp_frame = AmsFrame::new(AmsCommand::PortClose, [0xFF]);
        stream
            .write_frame(&resp_frame)
            .await
            .expect("Write should succeed");

        let mut buffer = [0u8; 7]; // Header (6) + Payload (1)
        server.read_exact(&mut buffer).await.unwrap();

        let expected_tail = [
            0x01, 0x00, // Command: Port Close (0x0001)
            0x01, 0x00, 0x00, 0x00, // Length: 1 byte
            0xFF, // Payload: FF
        ];
        assert_eq!(buffer, expected_tail);
    }
}
