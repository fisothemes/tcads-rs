use futures::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, BufReader};
use std::io;
use tcads_core::{AMS_FRAME_MAX_LEN, AmsFrame, AmsTcpHeader};

/// A buffered reader specialised for parsing AMS frames from an asynchronous byte stream.
///
/// This struct wraps an underlying async reader in a [`BufReader`] to minimise system calls
/// when reading the 6-byte [AMS/TCP header](AmsTcpHeader) and the variable-length payload.
pub struct AmsReader<R: AsyncRead + Unpin> {
    reader: BufReader<R>,
}

impl<R: AsyncRead + Unpin> AmsReader<R> {
    /// Creates a new AmsReader with [default buffering](BufReader::new).
    pub fn new(reader: R) -> Self {
        Self {
            reader: BufReader::new(reader),
        }
    }

    /// Creates a new [`AmsReader`] with a specific buffer capacity.
    pub fn with_capacity(reader: R, capacity: usize) -> Self {
        Self {
            reader: BufReader::with_capacity(capacity, reader),
        }
    }

    /// Reads a single AMS frame from the underlying stream.
    ///
    /// Steps:
    /// 1. Detect clean EOF (returns `UnexpectedEof`).
    /// 2. Read the 6-byte AMS/TCP header.
    /// 3. Validate payload length against [`AMS_FRAME_MAX_LEN`].
    /// 4. Read the exact payload.
    pub async fn read_frame(&mut self) -> io::Result<AmsFrame> {
        if self.reader.fill_buf().await?.is_empty() {
            return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
        }

        let mut header_buf = [0u8; AmsTcpHeader::LENGTH];
        self.reader.read_exact(&mut header_buf).await?;
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
        self.reader.read_exact(&mut payload).await?;

        Ok(AmsFrame::from_parts(header, payload))
    }

    /// Consumes the reader, returning the underlying stream.
    ///
    /// # Note
    ///
    /// Any leftover data in the internal buffer is discarded.
    pub fn into_inner(self) -> R {
        self.reader.into_inner()
    }

    /// Returns a reference to the underlying reader.
    ///
    /// # Note
    ///
    /// It is not advisable to directly read from the underlying reader
    pub fn get_ref(&self) -> &R {
        self.reader.get_ref()
    }

    /// Returns a mutable reference to the underlying reader.
    ///
    /// # Note
    ///
    /// It is inadvisable to directly read from the underlying reader.
    pub fn get_mut(&mut self) -> &mut R {
        self.reader.get_mut()
    }
}
