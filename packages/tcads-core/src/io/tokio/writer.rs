use super::traits::WriteAllVectored;
use crate::io::frame::AmsFrame;
use std::io::IoSlice;
use std::net::SocketAddr;
use tokio::io::{self, AsyncWrite, AsyncWriteExt, BufWriter};
use tokio::net::TcpStream;

/// A buffered writer specialised for serializing AMS frames to an asynchronous byte stream.
///
/// This struct wraps an underlying writer in a [`BufWriter`] to coalesce the header
/// and payload writes, but automatically flushes after every frame to ensure low latency.
pub struct AmsWriter<W: AsyncWrite + Unpin = TcpStream> {
    writer: BufWriter<W>,
}

impl<W: AsyncWrite + Unpin> AmsWriter<W> {
    /// Creates a new AmsWriter with [default buffering](BufWriter::new).
    pub fn new(writer: W) -> Self {
        Self {
            writer: BufWriter::new(writer),
        }
    }

    /// Creates a new AmsWriter with a specific buffer capacity.
    pub fn with_capacity(writer: W, capacity: usize) -> Self {
        Self {
            writer: BufWriter::with_capacity(capacity, writer),
        }
    }

    /// Writes a frame and immediately flushes the buffer.
    ///
    /// 1. Queues the header and payload into the internal buffer using vectored writes.
    /// 2. Calls [`flush`](AsyncWriteExt::flush) to send the packet immediately.
    pub async fn write_frame(&mut self, frame: &AmsFrame) -> io::Result<()> {
        let header_bytes = frame.header().to_bytes();
        let mut bufs = [IoSlice::new(&header_bytes), IoSlice::new(frame.payload())];

        WriteAllVectored::write_all_vectored(&mut self.writer, &mut bufs).await?;
        self.writer.flush().await
    }

    /// Consumes this BufWriter, returning the underlying writer.
    ///
    /// # Note
    ///
    /// Any leftover data in the internal buffer is lost.
    pub fn into_inner(self) -> W {
        self.writer.into_inner()
    }

    /// Returns a reference to the underlying writer.
    pub fn get_ref(&self) -> &W {
        self.writer.get_ref()
    }

    /// Returns a mutable reference to the underlying writer.
    ///
    /// # Note
    ///
    /// It is inadvisable to directly write to the underlying writer.
    pub fn get_mut(&mut self) -> &mut W {
        self.writer.get_mut()
    }
}

impl AmsWriter<TcpStream> {
    /// Returns the socket address of the remote peer of this TCP connection.
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.writer.get_ref().peer_addr()
    }

    /// Returns the socket address of the local half of this TCP connection
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.writer.get_ref().local_addr()
    }

    /// Shuts down the output stream, ensuring that the value can be dropped cleanly.
    ///
    /// See [`TcpStream::shutdown`] for more details.
    pub async fn shutdown(&mut self) -> io::Result<()> {
        self.writer.get_mut().shutdown().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ams::AmsCommand;
    use tokio::io::AsyncReadExt;

    #[tokio::test]
    async fn test_write_frame_simple() {
        let (client, mut server) = io::duplex(1024);
        let mut writer = AmsWriter::new(client);

        let frame = AmsFrame::new(AmsCommand::AdsCommand, [0xAA, 0xBB]);

        writer.write_frame(&frame).await.expect("Write failed");

        let mut buffer = [0u8; 8]; // 6 byte header + 2 byte payload
        server.read_exact(&mut buffer).await.expect("Read failed");

        let expected = [
            0x00, 0x00, // Command: AdsCommand
            0x02, 0x00, 0x00, 0x00, // Length: 2
            0xAA, 0xBB, // Payload
        ];
        assert_eq!(buffer, expected);
    }

    #[tokio::test]
    async fn test_write_large_frame() {
        let (client, mut server) = io::duplex(65536);
        let mut writer = AmsWriter::new(client);

        let payload = vec![0x11; 10_000];
        let frame = AmsFrame::new(AmsCommand::PortConnect, payload.clone());

        writer.write_frame(&frame).await.expect("Write failed");

        let mut buffer = vec![0u8; 6 + 10_000];
        server.read_exact(&mut buffer).await.expect("Read failed");

        assert_eq!(&buffer[6..], &payload[..]);
    }
}
