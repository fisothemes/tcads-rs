//! Example 3: Using Protocol Structs
//! Run with: `cargo run --bin 03_protocol_structs`

use tcads::core::io::blocking::AmsStream;
use tcads::core::protocol::{PortConnectRequest, PortConnectResponse};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    let mut stream = AmsStream::connect("127.0.0.1:48898")?;

    println!("Successfully connected!");

    // 1. Use the typed request builder
    let request = PortConnectRequest::default();
    stream.write_frame(&request.into_frame())?;

    let response_frame = stream.read_frame()?;

    // 2. Parse the raw frame into a strongly typed response
    let response = PortConnectResponse::try_from(&response_frame)?;

    println!("Router assigned us AMS Address: {}", response.addr());

    Ok(())
}
