use std::io::{self, IoSlice, Write};

/// An internal helper trait to emulate the currently unstable [`write_all_vectored`](Write::write_all_vectored).
///
/// Unlike `write_vectored`, which may write only a portion of the data, this method
/// loops until every byte in every slice has been written or an error occurs.
pub trait WriteAllVectored {
    /// Writes all data from the provided iterator of buffers.
    fn write_all_vectored(&mut self, bufs: &mut [IoSlice]) -> io::Result<()>;
}

impl<W: Write> WriteAllVectored for W {
    fn write_all_vectored(&mut self, mut bufs: &mut [IoSlice]) -> io::Result<()> {
        // Guarantee that bufs is empty if it contains no data,
        // to avoid calling write_vectored if there is no data to be written.
        IoSlice::advance_slices(&mut bufs, 0);
        while !bufs.is_empty() {
            match self.write_vectored(bufs) {
                Ok(0) => {
                    return Err(io::Error::new(
                        io::ErrorKind::WriteZero,
                        "failed to write whole buffer",
                    ));
                }
                Ok(n) => IoSlice::advance_slices(&mut bufs, n),
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}
