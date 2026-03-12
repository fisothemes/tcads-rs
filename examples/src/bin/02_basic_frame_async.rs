//! Example 2: Basic Asynchronous Frame (Tokio)
//! Run with: `cargo run --bin 02_basic_frame_async`

use tcads::core::io::tokio::AmsStream;
use tcads::core::{AmsCommand, AmsFrame};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[tokio::main]
async fn main() -> Result<()> {
    // Connect to the local AMS Router
    let mut stream = AmsStream::connect("127.0.0.1:48898").await?;

    // Construct a raw Port Connect frame
    let port_connect_frame = AmsFrame::new(AmsCommand::PortConnect, [0x00, 0x00]);

    // Write and read directly on the stream
    stream.write_frame(&port_connect_frame).await?;
    let response_frame = stream.read_frame().await?;

    println!(
        "Received: {:?} -> {:?}",
        response_frame.header().command(),
        response_frame.payload()
    );

    Ok(())
}
