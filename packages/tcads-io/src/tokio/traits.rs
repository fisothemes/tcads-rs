use std::io::IoSlice;
use tokio::io::{self, AsyncWrite, AsyncWriteExt};

/// An internal helper trait to write multiple buffers (scatter/gather) fully.
///
/// Unlike [`AsyncWriteExt::write_vectored`], which may write only a portion of the data
/// (returning the number of bytes written), this method loops until every byte in
/// every slice has been written or an error occurs.
pub trait WriteAllVectored {
    /// Writes all data from the provided slice of buffers.
    async fn write_all_vectored(&mut self, bufs: &mut [IoSlice<'_>]) -> io::Result<()>;
}

impl<W: AsyncWrite + Unpin> WriteAllVectored for W {
    async fn write_all_vectored(&mut self, mut bufs: &mut [IoSlice<'_>]) -> io::Result<()> {
        IoSlice::advance_slices(&mut bufs, 0);

        while !bufs.is_empty() {
            match self.write_vectored(bufs).await {
                Ok(0) => {
                    return Err(io::Error::new(
                        io::ErrorKind::WriteZero,
                        "failed to write whole buffer",
                    ));
                }
                Ok(n) => {
                    IoSlice::advance_slices(&mut bufs, n);
                }
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}
