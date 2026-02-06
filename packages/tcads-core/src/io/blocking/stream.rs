use super::reader::AmsReader;
use super::writer::AmsWriter;
use std::io::{self, Read, Write};
use std::net::TcpStream;

pub struct AmsStream<S: Read + Write = TcpStream> {
    stream: S,
}

impl<S: Read + Write> AmsStream<S> {
    pub fn new(stream: S) -> Self {
        Self { stream }
    }

    pub fn into_inner(self) -> S {
        self.stream
    }
}

impl<S: Read + Write + Clone> AmsStream<S> {
    pub fn split<R: Read, W: Write>(self) -> (AmsReader<S>, AmsWriter<S>) {
        (
            AmsReader::new(self.stream.clone()),
            AmsWriter::new(self.stream),
        )
    }
}

impl AmsStream<TcpStream> {
    pub fn try_split(self) -> io::Result<(AmsReader<TcpStream>, AmsWriter<TcpStream>)> {
        Ok((
            AmsReader::new(self.stream.try_clone()?),
            AmsWriter::new(self.stream),
        ))
    }
}
