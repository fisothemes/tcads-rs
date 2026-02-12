use crate::ams::{AMS_TCP_HEADER_LEN, AmsTcpHeader};
use crate::io::frame::{AMS_FRAME_MAX_LEN, AmsFrame};
use tokio::io::{self, AsyncBufReadExt, AsyncRead, AsyncReadExt, BufReader};

pub struct AmsReader<R: AsyncRead> {
    reader: BufReader<R>,
}

impl<R: AsyncRead + Unpin> AmsReader<R> {
    /// Creates a new AmsReader with [default buffering](BufReader::new).
    pub fn new(reader: R) -> Self {
        Self {
            reader: BufReader::new(reader),
        }
    }

    /// Creates a new AmsReader with a specific buffer capacity.
    pub fn with_capacity(reader: R, capacity: usize) -> Self {
        Self {
            reader: BufReader::with_capacity(capacity, reader),
        }
    }

    /// Reads a single AMS frame from the underlying stream.
    pub async fn read_frame(&mut self) -> io::Result<AmsFrame> {
        if self.reader.fill_buf().await?.is_empty() {
            return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
        }

        let mut header_buf = [0u8; AMS_TCP_HEADER_LEN];
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

    /// Consumes this AmsReader, returning the underlying reader.
    ///
    /// # Note
    ///
    /// Any leftover data in the internal buffer is lost.
    pub fn into_inner(self) -> R {
        self.reader.into_inner()
    }
}
