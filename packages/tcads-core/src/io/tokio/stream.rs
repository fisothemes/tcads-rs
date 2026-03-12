use super::reader::AmsReader;
use super::traits::WriteAllVectored;
use super::writer::AmsWriter;
use crate::ams::AmsTcpHeader;
use crate::io::frame::{AMS_FRAME_MAX_LEN, AmsFrame};
use std::io::IoSlice;
use std::net::SocketAddr;
use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{self, TcpStream};
use tokio::time::{self, Duration, timeout};

/// A stream wrapper for communicating with an AMS Router asynchronously.
///
/// This struct serves as the main entry point for an ADS connection. It wraps a raw byte stream
/// (typically a [`TcpStream`]) and provides methods to read and write [`AmsFrame`]s.
pub struct AmsStream<S: AsyncRead + AsyncWrite + Unpin = TcpStream> {
    stream: S,
    read_timeout: Option<Duration>,
    write_timeout: Option<Duration>,
}

impl<S: AsyncRead + AsyncWrite + Unpin> AmsStream<S> {
    /// Creates a new instance of the AmsStream given a stream.
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            read_timeout: None,
            write_timeout: None,
        }
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
        match self.read_timeout {
            Some(dur) => timeout(dur, self.read_frame_inner())
                .await
                .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "timeout"))?,
            None => self.read_frame_inner().await,
        }
    }

    /// Writes a frame directly to the stream using vectored I/O.
    ///
    /// This method attempts to send the header and payload in a single system call
    /// (if supported by the OS) and **flushes immediately** to avoid TCP fragmentation
    /// or Nagle's algorithm delays.
    pub async fn write_frame(&mut self, frame: &AmsFrame) -> io::Result<()> {
        match self.write_timeout {
            Some(dur) => timeout(dur, self.write_frame_inner(frame))
                .await
                .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "timeout"))?,
            None => self.write_frame_inner(frame).await,
        }
    }

    /// Splits the stream into a buffered Reader and buffered Writer.
    ///
    /// This uses [`tokio::io::split`] internally, which wraps the stream in a `Mutex` / `Arc`
    /// to allow concurrent access. For `TcpStream`, prefer using [`into_split`](AmsStream::into_split)
    /// (if available on the specific impl) for zero-overhead splitting.
    pub fn split(self) -> (AmsReader<io::ReadHalf<S>>, AmsWriter<io::WriteHalf<S>>) {
        let (reader, writer) = io::split(self.stream);
        (AmsReader::new(reader), AmsWriter::new(writer))
    }

    /// Consumes the AmsStream and returns the underlying stream.
    pub fn into_inner(self) -> S {
        self.stream
    }

    /// Returns a reference to the underlying stream.
    pub fn get_ref(&self) -> &S {
        &self.stream
    }

    /// Returns a mutable reference to the underlying stream
    pub fn get_mut(&mut self) -> &mut S {
        &mut self.stream
    }

    /// Sets the read timeout for the underlying stream.
    pub fn set_read_timeout(&mut self, dur: Option<Duration>) -> io::Result<()> {
        if let Some(dur) = dur
            && dur.is_zero()
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "cannot set a 0 duration timeout",
            ));
        }
        self.read_timeout = dur;
        Ok(())
    }

    /// Returns the read timeout of the underlying stream.
    ///
    /// If the timeout is [`None`], then [`read_frame`](AmsStream::read_frame) calls will block indefinitely.
    pub fn read_timeout(&self) -> Option<Duration> {
        self.read_timeout
    }

    /// Sets the write timeout for the underlying stream.
    pub fn set_write_timeout(&mut self, dur: Option<Duration>) -> io::Result<()> {
        if let Some(dur) = dur
            && dur.is_zero()
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "cannot set a 0 duration timeout",
            ));
        }
        self.write_timeout = dur;
        Ok(())
    }

    /// Returns the write timeout of the underlying stream.
    ///
    /// If the timeout is [`None`], then [`write_frame`](AmsStream::write_frame) calls will block indefinitely.
    pub fn write_timeout(&self) -> Option<Duration> {
        self.write_timeout
    }

    async fn read_frame_inner(&mut self) -> io::Result<AmsFrame> {
        let mut header_buf = [0u8; AmsTcpHeader::LENGTH];
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

    async fn write_frame_inner(&mut self, frame: &AmsFrame) -> io::Result<()> {
        let header_bytes = frame.header().to_bytes();
        let mut bufs = [IoSlice::new(&header_bytes), IoSlice::new(frame.payload())];

        WriteAllVectored::write_all_vectored(&mut self.stream, &mut bufs).await?;
        self.stream.flush().await
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
    ///
    /// # Example
    ///
    /// ```no_run
    /// use tcads_core::io::tokio::AmsStream;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let stream = AmsStream::connect("127.0.0.1:851").await.unwrap();
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect<A: net::ToSocketAddrs>(addr: A) -> io::Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        stream.set_nodelay(true)?;
        Ok(Self::new(stream))
    }

    /// Connects to an AMS router at a specified address with a timeout.
    ///
    /// Unlike [`connect`](Self::connect), this method allows specifying a maximum
    /// duration to wait for the connection attempt to complete.
    ///
    /// It is an error to pass a zero [`Duration`] to this function.
    ///
    /// This method wraps [`connect`](Self::connect) in [`tokio::time::timeout`].
    /// If the connection is not established before the timeout expires, the
    /// connection future is cancelled and an [`io::ErrorKind::TimedOut`] error
    /// is returned.
    ///
    /// # Note
    ///
    /// This timeout applies to the entire asynchronous connect operation.
    /// If the timeout expires, the connect future is dropped and the underlying
    /// socket is closed.
    pub async fn connect_timeout(addr: &SocketAddr, timeout: Duration) -> io::Result<Self> {
        if timeout.is_zero() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "cannot set a 0 duration timeout",
            ));
        }

        time::timeout(timeout, Self::connect(addr))
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "timeout"))?
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

    /// Shuts down the output stream, ensuring that the value can be dropped cleanly.
    ///
    /// See [`TcpStream::shutdown`] for more details.
    pub async fn shutdown(&mut self) -> io::Result<()> {
        self.stream.shutdown().await
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
