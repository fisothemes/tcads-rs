use super::traits::WriteAllVectored;
use futures::io::{AsyncWrite, AsyncWriteExt, BufWriter};
use std::io::{self, IoSlice};
use tcads_core::AmsFrame;

/// A buffered writer specialised for serializing AMS frames to an asynchronous byte stream.
///
/// This struct wraps an underlying writer in a [`BufWriter`] to coalesce the header
/// and payload writes, but automatically flushes after every frame to ensure low latency.
pub struct AmsWriter<W: AsyncWrite + Unpin> {
    writer: BufWriter<W>,
}

impl<W: AsyncWrite + Unpin> AmsWriter<W> {
    /// Creates a new [`AmsWriter`] with [default buffering](BufWriter::new).
    pub fn new(writer: W) -> Self {
        Self {
            writer: BufWriter::new(writer),
        }
    }

    /// Creates a new [`AmsWriter`] with a specific buffer capacity.
    pub fn with_capacity(writer: W, capacity: usize) -> Self {
        Self {
            writer: BufWriter::with_capacity(capacity, writer),
        }
    }

    /// Writes a frame and immediately flushes the buffer.
    pub async fn write_frame(&mut self, frame: &AmsFrame) -> io::Result<()> {
        let header_bytes = frame.header().to_bytes();
        let mut bufs = [IoSlice::new(&header_bytes), IoSlice::new(frame.payload())];

        WriteAllVectored::write_all_vectored(&mut self.writer, &mut bufs).await?;
        self.writer.flush().await
    }

    /// Consumes the writer, returning the underlying stream.
    ///
    /// # Note
    ///
    /// Any leftover data in the internal buffer is discarded.
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

#[cfg(test)]
mod tests {
    use super::*;
    use futures::executor::block_on;
    use futures::io::Cursor;
    use tcads_core::AmsCommand;

    #[test]
    fn test_write_frame_simple() {
        let mut buffer = Vec::new();
        {
            let mut writer = AmsWriter::new(Cursor::new(&mut buffer));
            let frame = AmsFrame::new(AmsCommand::AdsCommand, [0xAA, 0xBB]);
            block_on(writer.write_frame(&frame)).expect("Write failed");
        }

        let expected = [
            0x00, 0x00, // AdsCommand
            0x02, 0x00, 0x00, 0x00, // length 2
            0xAA, 0xBB,
        ];
        assert_eq!(buffer, expected);
    }

    #[test]
    fn test_write_large_frame() {
        let mut buffer = Vec::new();
        {
            let mut writer = AmsWriter::new(Cursor::new(&mut buffer));
            let payload = vec![0x11; 10_000];
            let frame = AmsFrame::new(AmsCommand::PortConnect, payload.clone());
            block_on(writer.write_frame(&frame)).expect("Write failed");
        }

        assert_eq!(&buffer[6..], &vec![0x11; 10_000][..]);
    }
}
