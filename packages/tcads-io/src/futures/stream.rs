use super::traits::WriteAllVectored;
use futures::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use std::io::IoSlice;
use tcads_core::{AMS_FRAME_MAX_LEN, AmsFrame, AmsTcpHeader};

/// A stream wrapper for communicating with an AMS Router over any asynchronous transport
/// that implements [`AsyncRead`] + [`AsyncWrite`].
///
/// This is the runtime-agnostic counterpart of the tokio `AmsStream`. Pass any type that
/// satisfies the bounds (a smol TCP stream, a custom WebSocket adapter, an in-memory duplex, etc.).
///
/// # Example
///
/// ```rust, no_run
/// use futures::io::Cursor;
/// use tcads_io::futures::AmsStream;
/// use tcads_core::{AmsCommand, AmsFrame};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut mock = Cursor::new(Vec::new());
/// let mut stream = AmsStream::new(mock);
///
/// let frame = AmsFrame::new(AmsCommand::PortConnect, [0x00, 0x00]);
/// stream.write_frame(&frame).await?;
/// # Ok(())
/// # }
/// ```
pub struct AmsStream<S: AsyncRead + AsyncWrite + Unpin> {
    stream: S,
}

impl<S: AsyncRead + AsyncWrite + Unpin> AmsStream<S> {
    /// Creates a new [`AmsStream`] from the given transport.
    pub fn new(stream: S) -> Self {
        Self { stream }
    }

    /// Reads a frame directly from the stream (no internal buffering).
    pub async fn read_frame(&mut self) -> io::Result<AmsFrame> {
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

    /// Writes a frame using vectored I/O and flushes immediately.
    pub async fn write_frame(&mut self, frame: &AmsFrame) -> io::Result<()> {
        let header_bytes = frame.header().to_bytes();
        let mut bufs = [IoSlice::new(&header_bytes), IoSlice::new(frame.payload())];

        WriteAllVectored::write_all_vectored(&mut self.stream, &mut bufs).await?;
        self.stream.flush().await
    }

    /// Consumes the `AmsStream`, returning the underlying transport.
    pub fn into_inner(self) -> S {
        self.stream
    }

    /// Returns a reference to the underlying transport.
    pub fn get_ref(&self) -> &S {
        &self.stream
    }

    /// Returns a mutable reference to the underlying transport.
    pub fn get_mut(&mut self) -> &mut S {
        &mut self.stream
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::executor::block_on;
    use futures::io::Cursor;
    use tcads_core::AmsCommand;

    #[test]
    fn test_stream_read_write_roundtrip() {
        let mut buffer = Vec::new();
        {
            let mut stream = AmsStream::new(Cursor::new(&mut buffer));
            let frame = AmsFrame::new(AmsCommand::PortConnect, [0x01, 0x01]);
            block_on(stream.write_frame(&frame)).unwrap();
        }

        let mut stream = AmsStream::new(Cursor::new(buffer));
        let received = block_on(stream.read_frame()).expect("Read should succeed");

        assert_eq!(received.header().command(), AmsCommand::PortConnect);
        assert_eq!(received.payload(), &[0x01, 0x01]);
    }
}
