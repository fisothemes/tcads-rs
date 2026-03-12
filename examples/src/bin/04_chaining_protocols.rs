//! Example 4: Chaining Protocols - Router Handshake & Device Info
//! Run with: `cargo run --bin 04_chaining_protocols`
//!
//! Demonstrates chaining multiple core protocol requests to perform a
//! full router handshake and query a PLC for its device information.

use tcads::core::io::blocking::AmsStream;
use tcads::core::protocol::{
    AdsReadDeviceInfoRequest, AdsReadDeviceInfoResponse, GetLocalNetIdRequest,
    GetLocalNetIdResponse, PortConnectRequest, PortConnectResponse,
};
use tcads::core::{AmsAddr, InvokeId};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    let mut stream = AmsStream::connect("127.0.0.1:48898")?;

    // 1. Request a source AMS Address from the router
    stream.write_frame(&PortConnectRequest::default().into_frame())?;
    let port_resp = PortConnectResponse::try_from(&stream.read_frame()?)?;
    let source = *port_resp.addr();

    // 2. Ask the router for its Local Net ID
    stream.write_frame(&GetLocalNetIdRequest.into())?;
    let netid_resp = GetLocalNetIdResponse::try_from(&stream.read_frame()?)?;
    let local_net_id = netid_resp.net_id();

    // 3. Read Device Info from the TcEventLogger (Port 110)
    //    If you want to read from the runtime PLC project, use port 851
    let target = AmsAddr::new(local_net_id, 110);
    let device_info_req = AdsReadDeviceInfoRequest::new(target, source, InvokeId::default());

    stream.write_frame(&device_info_req.into_frame())?;
    let info_resp = AdsReadDeviceInfoResponse::try_from(&stream.read_frame()?)?;

    println!("Target PLC Name: {}", info_resp.device_name());
    println!("Target PLC Version: {:?}", info_resp.version());

    Ok(())
}
