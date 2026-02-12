use super::traits::WriteAllVectored;
use crate::io::frame::AmsFrame;
use std::io::{self, BufWriter, IntoInnerError, IoSlice, Write};

/// A buffered writer specialised for serializing AMS frames to a byte stream.
///
/// This struct wraps an underlying writer in a [`BufWriter`].
///
/// # Latency Note
///
/// To ensure real-time responsiveness for ADS commands, [`write_frame`](Self::write_frame)
/// **automatically flushes** the buffer after writing each frame.
///
/// This prevents commands from sitting in the buffer waiting for 8KB of data to accumulate.
pub struct AmsWriter<W: Write> {
    writer: BufWriter<W>,
}

impl<W: Write> AmsWriter<W> {
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
    /// 2. Calls [`flush`](Write::flush) to send the packet immediately.
    pub fn write_frame(&mut self, frame: &AmsFrame) -> io::Result<()> {
        let header_bytes = frame.header().to_bytes();
        let mut bufs = [IoSlice::new(&header_bytes), IoSlice::new(frame.payload())];

        WriteAllVectored::write_all_vectored(&mut self.writer, &mut bufs)?;
        self.writer.flush()
    }

    /// Consumes the AmsWriter, returning the writer
    pub fn into_inner(self) -> Result<W, IntoInnerError<BufWriter<W>>> {
        self.writer.into_inner()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ams::AmsCommand;

    #[test]
    fn test_write_frame_flushes_correctly() {
        let frame = AmsFrame::new(AmsCommand::PortConnect, [0xCA, 0xFE]);

        let mut buffer = Vec::new();
        let mut writer = AmsWriter::new(&mut buffer);

        writer.write_frame(&frame).expect("Write should succeed");

        let expected = [
            0x00, 0x10, // Command: Port Connect (0x1000)
            0x02, 0x00, 0x00, 0x00, // Length: 2 bytes
            0xCA, 0xFE, // Payload: CA FE
        ];

        let buffer = writer.into_inner().expect("Should return borrowed buffer");

        assert_eq!(
            buffer, &expected,
            "Buffer should contain flushed frame data"
        );
    }
}
