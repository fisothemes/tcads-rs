use tokio::net::TcpStream;
#[cfg(unix)]
use tokio::net::UnixStream;

pub mod reader;
pub mod stream;
mod traits;
pub mod writer;

pub use reader::AmsReader;
pub use stream::AmsStream;
pub use writer::AmsWriter;

pub type TcpAmsStream = AmsStream<TcpStream>;
#[cfg(unix)]
pub type UnixAmsStream = AmsStream<UnixStream>;
