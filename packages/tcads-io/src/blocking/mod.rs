use std::net::TcpStream;
#[cfg(unix)]
use std::os::unix::net::UnixStream;

pub mod reader;
pub mod stream;
mod traits;
pub mod writer;

pub use reader::{AmsIncoming, AmsReader};
pub use stream::AmsStream;
pub use writer::AmsWriter;

pub type TcpAmsStream = AmsStream<TcpStream>;
#[cfg(unix)]
pub type UnixAmsStream = AmsStream<UnixStream>;
