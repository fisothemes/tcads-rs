use super::traits::WriteAllVectored;
use crate::io::frame::AmsFrame;
use std::io::IoSlice;
use tokio::io::{self, AsyncWrite, AsyncWriteExt, BufWriter};

pub struct AmsWriter<W: AsyncWriteExt + Unpin> {
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

    pub fn into_inner(self) -> W {
        self.writer.into_inner()
    }
}
