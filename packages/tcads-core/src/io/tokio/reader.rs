use crate::ams::{AMS_TCP_HEADER_LEN, AmsTcpHeader};
use crate::io::frame::{AMS_FRAME_MAX_LEN, AmsFrame};
use tokio::io::{self, AsyncBufReadExt, AsyncRead, AsyncReadExt, BufReader};

/// A buffered reader specialised for parsing AMS frames from an asynchronous byte stream.
///
/// This struct wraps an underlying async reader in a [`BufReader`] to minimise system calls
/// when reading the 6-byte [AMS/TCP header](AmsTcpHeader) and the variable-length payload.
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
    ///
    /// This method performs the following steps:
    /// 1. Checks for EOF (returns `UnexpectedEof` if the stream is closed cleanly at the start).
    /// 2. Reads the 6-byte AMS/TCP header.
    /// 3. Validates the payload length against [`AMS_FRAME_MAX_LEN`].
    /// 4. Reads the exact payload size into a vector.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ams::AmsCommand;
    use std::time::Duration;
    use tokio_test::io::Builder;

    #[tokio::test]
    async fn read_fragmented_frame() {
        let header_part1 = [0x00, 0x10, 0x02]; // Command: 0x1000 (PortConnect), Length partial
        let header_part2 = [0x00, 0x00, 0x00]; // Length: 2 bytes
        let payload = [0xCA, 0xFE];

        let mut mock = Builder::new()
            .read(&header_part1)
            .wait(Duration::from_millis(10)) // Simulate network lag
            .read(&header_part2)
            .read(&payload)
            .build();

        let mut reader = AmsReader::new(&mut mock);
        let frame = reader
            .read_frame()
            .await
            .expect("Should assemble fragmented frame");

        assert_eq!(frame.header().command(), AmsCommand::PortConnect);
        assert_eq!(frame.payload(), &payload);
    }

    #[tokio::test]
    async fn test_clean_eof() {
        let mut mock = Builder::new().build(); // Empty stream
        let mut reader = AmsReader::new(&mut mock);

        let err = reader.read_frame().await.unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
    }

    #[tokio::test]
    async fn test_dirty_eof_in_header() {
        // Scenario: Connection drops while reading header.
        let partial_header = [0x00, 0x10]; // Only 2 bytes
        let mut mock = Builder::new().read(&partial_header).build();
        let mut reader = AmsReader::new(&mut mock);

        let err = reader.read_frame().await.unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
    }

    #[tokio::test]
    async fn test_payload_too_large() {
        let mut header = [0u8; AMS_TCP_HEADER_LEN];
        let bad_len = (AMS_FRAME_MAX_LEN as u32 + 1).to_le_bytes();
        header[2..6].copy_from_slice(&bad_len);

        let mut mock = Builder::new().read(&header).build();
        let mut reader = AmsReader::new(&mut mock);

        let err = reader.read_frame().await.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("Payload too large"));
    }
}
