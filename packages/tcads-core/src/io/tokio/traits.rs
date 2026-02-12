use std::io::IoSlice;
use tokio::io::{self, AsyncWrite, AsyncWriteExt};

pub trait WriteAllVectored {
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
