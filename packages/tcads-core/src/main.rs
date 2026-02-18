use tcads_core::ads::{AdsCommand, AdsHeader, AdsState};
use tcads_core::ams::{AmsAddr, AmsCommand};
use tcads_core::io::blocking::AmsStream;
use tcads_core::protocol::{
    AdsReadDeviceInfoRequest, AdsReadDeviceInfoResponse, AdsReadStateRequest, AdsReadStateResponse,
    AdsWriteControlRequest, AdsWriteControlResponse, GetLocalNetIdRequest, GetLocalNetIdResponse,
    PortConnectRequest, PortConnectResponse, RouterNotification,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stream = AmsStream::connect("127.0.0.1:48898")?;

    let (reader, mut writer) = stream.try_split()?;

    writer.write_frame(&PortConnectRequest::default().into_frame())?;

    let mut source = AmsAddr::default();

    for result in reader.incoming() {
        let frame = result?;

        match frame.header().command() {
            AmsCommand::PortConnect => {
                let resp = PortConnectResponse::try_from(frame)?;
                source = *resp.addr();

                println!("AMS Router has assigned us the address {}!", source);

                writer.write_frame(&GetLocalNetIdRequest::into_frame())?;
            }
            AmsCommand::GetLocalNetId => {
                let resp = GetLocalNetIdResponse::try_from(frame)?;
                println!("Local Net ID: {}", resp.net_id());

                let target = AmsAddr::new(resp.net_id(), 851);

                writer.write_frame(&AdsReadStateRequest::new(target, source, 22).into_frame())?;
                writer
                    .write_frame(&AdsReadDeviceInfoRequest::new(target, source, 23).into_frame())?;
                writer.write_frame(
                    &AdsWriteControlRequest::new(target, source, 24, AdsState::Stop, 0)
                        .into_frame(),
                )?;
            }
            AmsCommand::AdsCommand => {
                let (header, payload) = AdsHeader::parse_prefix(frame.payload())?;

                match header.command_id() {
                    AdsCommand::AdsReadState => {
                        let resp = AdsReadStateResponse::parse_payload(payload)?;
                        println!("AdsReadState: {:?}", resp);
                    }
                    AdsCommand::AdsReadDeviceInfo => {
                        let resp = AdsReadDeviceInfoResponse::parse_payload(payload)?;
                        println!("AdsReadDeviceInfo: {:?}", resp);
                    }
                    AdsCommand::AdsWriteControl => {
                        let resp = AdsWriteControlResponse::parse_payload(payload)?;
                        println!("AdsWriteControl: {:?}", resp);
                    }
                    _ => todo!(),
                }
            }
            AmsCommand::RouterNotification => {
                let notif = RouterNotification::try_from(frame)?;
                println!("Router Notification: {:?}", notif);
            }
            _ => {
                println!("Unknown frame: {:?}", frame);
            }
        }
    }

    Ok(())
}
