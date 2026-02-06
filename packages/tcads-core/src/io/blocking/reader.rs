use std::io::{Read, Write};

pub struct AmsReader<R: Read> {
    reader: R,
}

impl<R: Read> AmsReader<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    pub fn into_inner(self) -> R {
        self.reader
    }
}
