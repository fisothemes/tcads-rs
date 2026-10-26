use super::traits::WriteAllVectored;
use super::{AmsReader, AmsWriter};
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

    /// Splits the stream into a buffered [`AmsReader`] and an [`AmsWriter`], so frames can be
    /// read and written from different tasks.
    ///
    /// This uses [`AsyncReadExt::split`], which shares the transport between the two halves
    /// behind a lock. To get the transport back, take each half out with
    /// [`AmsReader::into_inner`] and [`AmsWriter::into_inner`], then call
    /// [`ReadHalf::reunite`](io::ReadHalf::reunite).
    pub fn split(self) -> (AmsReader<io::ReadHalf<S>>, AmsWriter<io::WriteHalf<S>>) {
        let (reader, writer) = self.stream.split();
        (AmsReader::new(reader), AmsWriter::new(writer))
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

    #[test]
    fn split_halves_read_and_write_independently() {
        let (client, server) = futures_duplex();
        let (mut reader, mut writer) = AmsStream::new(client).split();
        let mut server = AmsStream::new(server);

        block_on(async {
            let request = AmsFrame::new(AmsCommand::PortConnect, [0x00, 0x00]);
            writer.write_frame(&request).await.unwrap();
            assert_eq!(
                server.read_frame().await.unwrap().payload(),
                request.payload()
            );

            let response = AmsFrame::new(AmsCommand::PortConnect, [0x01, 0x02]);
            server.write_frame(&response).await.unwrap();
            assert_eq!(
                reader.read_frame().await.unwrap().payload(),
                response.payload()
            );
        });
    }

    /// An in-memory full-duplex pipe built from two `futures` channels.
    fn futures_duplex() -> (Duplex, Duplex) {
        let (a_tx, a_rx) = futures::channel::mpsc::unbounded();
        let (b_tx, b_rx) = futures::channel::mpsc::unbounded();
        (Duplex::new(a_tx, b_rx), Duplex::new(b_tx, a_rx))
    }

    struct Duplex {
        tx: futures::channel::mpsc::UnboundedSender<std::io::Result<Vec<u8>>>,
        rx: futures::stream::IntoAsyncRead<
            futures::channel::mpsc::UnboundedReceiver<std::io::Result<Vec<u8>>>,
        >,
    }

    impl Duplex {
        fn new(
            tx: futures::channel::mpsc::UnboundedSender<std::io::Result<Vec<u8>>>,
            rx: futures::channel::mpsc::UnboundedReceiver<std::io::Result<Vec<u8>>>,
        ) -> Self {
            use futures::TryStreamExt;
            Self {
                tx,
                rx: rx.into_async_read(),
            }
        }
    }

    impl AsyncRead for Duplex {
        fn poll_read(
            mut self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
            buf: &mut [u8],
        ) -> std::task::Poll<std::io::Result<usize>> {
            std::pin::Pin::new(&mut self.rx).poll_read(cx, buf)
        }
    }

    impl AsyncWrite for Duplex {
        fn poll_write(
            self: std::pin::Pin<&mut Self>,
            _: &mut std::task::Context<'_>,
            buf: &[u8],
        ) -> std::task::Poll<std::io::Result<usize>> {
            let _ = self.tx.unbounded_send(Ok(buf.to_vec()));
            std::task::Poll::Ready(Ok(buf.len()))
        }

        fn poll_flush(
            self: std::pin::Pin<&mut Self>,
            _: &mut std::task::Context<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            std::task::Poll::Ready(Ok(()))
        }

        fn poll_close(
            self: std::pin::Pin<&mut Self>,
            _: &mut std::task::Context<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            self.tx.close_channel();
            std::task::Poll::Ready(Ok(()))
        }
    }
}
