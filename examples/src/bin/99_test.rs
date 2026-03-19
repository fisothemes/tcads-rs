//! Scratch pad for testing.

use std::io::Read;
use tcads::core::WindowsFileTime;
use tcads::core::ads::{AdsCommand, AdsHeader, AdsReturnCode, StateFlag};
use tcads::core::ams::{AmsAddr, AmsCommand};
use tcads::core::io::AmsFrame;
use tcads::core::io::blocking::AmsStream;
use tcads::core::protocol::{
    AdsReadWriteRequestOwned, AdsReadWriteResponse, GetLocalNetIdRequest, GetLocalNetIdResponse,
    PortConnectRequest, PortConnectResponse,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn subscribe_to_logger(stream: &mut AmsStream, target: AmsAddr, source: AmsAddr) -> Result<()> {
    let frame = AdsReadWriteRequestOwned::new(target, source, 1, 0xF090, 0, 4, source.to_bytes())
        .into_frame();

    stream.write_frame(&frame)?;
    let frame = stream.read_frame()?;
    let resp = AdsReadWriteResponse::try_from(&frame)?;
    println!("{resp:?}");

    Ok(())
}

fn send(stream: &mut AmsStream, target: AmsAddr, source: AmsAddr) -> Result<()> {
    let header = AdsHeader::new(
        target,
        source,
        AdsCommand::AdsDeviceNotification,
        StateFlag::new(StateFlag::ADS_COMMAND),
        63,
        AdsReturnCode::Ok,
        0x0,
    );

    let bytes: [u8; 64] = [
        59, 0, 0, 0, // Notif Size
        0, 0, 0, 0, // Count of Stamps
        16, 87, 133, 69, 142, 182, 220, 1, // Windows file time
        0, 0, 0, 0, 0, 0, 0, 0, // Reserved
        10, 0, 0, 0, // Length of format + arg?
        2, 0, 0, 0, // Mask
        80, 108, 99, 84, 97, 115, 107, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // Task Name
        65, 65, 40, 37, 115, 41, 0, // Format String
        57, 57, 0, 0, // Argument
    ];

    let mut payload = Vec::from(header.to_bytes());
    payload.extend_from_slice(&bytes[0..8]); // header
    payload.extend_from_slice(&WindowsFileTime::now().to_bytes()); // file time
    payload.extend_from_slice(&bytes[16..]);

    let frame = AmsFrame::new(AmsCommand::AdsCommand, payload);

    println!("{:?}", &frame.to_vec());

    stream.write_frame(&frame)?;

    Ok(())
}

fn main() -> Result<()> {
    let mut stream = AmsStream::connect("127.0.0.1:48898")?;

    stream.write_frame(&PortConnectRequest::default().into())?;

    let source = *PortConnectResponse::try_from(stream.read_frame()?)?.addr();

    stream.write_frame(&GetLocalNetIdRequest::default().into())?;

    let target = AmsAddr::new(
        GetLocalNetIdResponse::try_from(stream.read_frame()?)?.net_id(),
        100,
    );

    subscribe_to_logger(&mut stream, target, source)?;

    send(&mut stream, target, source)?;

    let mut buf = [0; 1024];

    let n = stream.get_mut().read(&mut buf)?;

    println!("written: {:?}", n);

    Ok(())
}
