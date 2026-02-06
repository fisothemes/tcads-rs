use std::io::Write;

pub struct AmsWriter<W: Write> {
    writer: W,
}

impl<W: Write> AmsWriter<W> {
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    pub fn into_inner(self) -> W {
        self.writer
    }
}
