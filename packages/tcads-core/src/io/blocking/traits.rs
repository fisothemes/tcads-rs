use std::io;
use std::io::{IoSlice, Write};

/// An internal helper trait to emulate the currently unstable [`write_all_vectored`](Write::write_all_vectored).
///
/// Unlike `write_vectored`, which may write only a portion of the data, this method
/// loops until every byte in every slice has been written or an error occurs.
pub trait WriteAllVectored {
    /// Writes all data from the provided iterator of buffers.
    fn write_all_vectored(&mut self, bufs: &mut dyn Iterator<Item = IoSlice>) -> io::Result<()>;
}

impl<W: Write> WriteAllVectored for W {
    fn write_all_vectored(&mut self, bufs: &mut dyn Iterator<Item = IoSlice>) -> io::Result<()> {
        let bufs: Vec<_> = bufs.collect();
        let total: usize = bufs.iter().map(|b| b.len()).sum();
        let mut written = 0;

        while written < total {
            let n = self.write_vectored(&bufs)?;
            if n == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "failed to write whole buffer",
                ));
            }
            written += n;
        }
        Ok(())
    }
}
