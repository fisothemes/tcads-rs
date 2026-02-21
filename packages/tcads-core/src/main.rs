use tcads_core::ads::{AdsCommand, AdsHeader, AdsReturnCode, AdsState};
use tcads_core::ams::{AmsAddr, AmsCommand};
use tcads_core::io::blocking::AmsStream;
use tcads_core::protocol::{
    AdsReadDeviceInfoRequest, AdsReadDeviceInfoResponse, AdsReadRequest, AdsReadResponse,
    AdsReadStateRequest, AdsReadStateResponse, AdsReadWriteRequestOwned,
    AdsWriteControlRequestOwned, AdsWriteControlResponse, AdsWriteRequestOwned, AdsWriteResponse,
    GetLocalNetIdRequest, GetLocalNetIdResponse, PortConnectRequest, PortConnectResponse,
    RouterNotification,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stream = AmsStream::connect("127.0.0.1:48898")?;

    let (reader, mut writer) = stream.try_split()?;

    writer.write_frame(&PortConnectRequest::default().into_frame())?;

    let mut source = AmsAddr::default();
    let mut target = AmsAddr::default();
    let mut var_handle;

    for result in reader.incoming() {
        let frame = result?;

        print!("{:?}:\t", frame.header().command());

        match frame.header().command() {
            AmsCommand::PortConnect => {
                let resp = PortConnectResponse::try_from(frame)?;
                source = *resp.addr();
                println!("AMS Router has assigned us the address {}!", source);

                writer.write_frame(&GetLocalNetIdRequest::into_frame())?;
            }
            AmsCommand::GetLocalNetId => {
                let resp = GetLocalNetIdResponse::try_from(frame)?;
                println!("Local Net ID is {}", resp.net_id());

                target = AmsAddr::new(resp.net_id(), 851);

                // Kick off: device info and read state first
                writer.write_frame(
                    &AdsReadDeviceInfoRequest::new(target, source, 0x606060).into_frame(),
                )?;
                writer.write_frame(
                    &AdsReadStateRequest::new(target, source, 0x606060).into_frame(),
                )?;
            }
            AmsCommand::RouterNotification => {
                let notif = RouterNotification::try_from(frame)?;
                println!("{:?}", notif.state());
            }
            AmsCommand::AdsCommand => {
                let (header, payload) = AdsHeader::parse_prefix(frame.payload())?;

                print!("{:?} -> ", header.command_id());

                match header.command_id() {
                    AdsCommand::AdsReadDeviceInfo => {
                        let resp = AdsReadDeviceInfoResponse::parse_payload(payload)?;
                        println!("Device info: {:?}", resp);
                    }
                    AdsCommand::AdsReadState => {
                        let (code, state, _) = AdsReadStateResponse::parse_payload(payload)?;
                        println!("PLC state: {:?}", state);

                        match (header.invoke_id(), code, state) {
                            (0x606060, AdsReturnCode::Ok, AdsState::Run) => {
                                // Stop the PLC on start
                                writer.write_frame(
                                    &AdsWriteControlRequestOwned::new(
                                        target,
                                        source,
                                        0xBADF00D,
                                        AdsState::Stop,
                                        0,
                                    )
                                    .into_frame(),
                                )?;
                            }
                            (0x600DF00D, AdsReturnCode::Ok, AdsState::Stop) => {
                                // Start the PLC
                                writer.write_frame(
                                    &AdsWriteControlRequestOwned::new(
                                        target,
                                        source,
                                        0xBEEF,
                                        AdsState::Run,
                                        0,
                                    )
                                    .into_frame(),
                                )?;
                            }
                            (0xBEE2, AdsReturnCode::Ok, AdsState::Run) => {
                                // Get a handle for a variable
                                writer.write_frame(
                                    &AdsReadWriteRequestOwned::new(
                                        target,
                                        source,
                                        0xCAFE,
                                        0xF003, // Get Handle by name index group
                                        0x0000,
                                        4, // handle is always 4 bytes
                                        b"MAIN.nCount\0",
                                    )
                                    .into_frame(),
                                )?;
                            }
                            _ => panic!(
                                "Unexpected match! Got {:?}",
                                (header.invoke_id(), code, state)
                            ),
                        }
                    }
                    AdsCommand::AdsWriteControl => {
                        let resp = AdsWriteControlResponse::parse_payload(payload)?;

                        match (header.invoke_id(), resp) {
                            (0xBADF00D, AdsReturnCode::Ok) => {
                                println!("PLC stopped successfully!");

                                std::thread::sleep(std::time::Duration::from_secs(2));

                                writer.write_frame(
                                    &AdsReadStateRequest::new(target, source, 0x600DF00D)
                                        .into_frame(),
                                )?;
                            }
                            (0xBEEF, AdsReturnCode::Ok) => {
                                println!("PLC started successfully!");

                                writer.write_frame(
                                    &AdsReadStateRequest::new(target, source, 0xBEE2).into_frame(),
                                )?;
                            }
                            _ => panic!("Unexpected match! Got {:?}", (header.invoke_id(), resp)),
                        }
                    }
                    AdsCommand::AdsReadWrite => {
                        let (code, data) = AdsReadResponse::parse_payload(payload)?;

                        match (header.invoke_id(), code) {
                            (0xCAFE, AdsReturnCode::Ok) => {
                                var_handle = u32::from_le_bytes(data.try_into()?);
                                println!("Variable handle is for MAIN.nCount is {}", var_handle);

                                writer.write_frame(
                                    &AdsWriteRequestOwned::new(
                                        target,
                                        source,
                                        0,
                                        0xF005, // write by handle
                                        var_handle,
                                        42u32.to_le_bytes(),
                                    )
                                    .into_frame(),
                                )?;

                                writer.write_frame(
                                    &AdsReadRequest::new(
                                        target,
                                        source,
                                        0,
                                        0xF005,
                                        var_handle,
                                        size_of::<u32>() as u32,
                                    )
                                    .into_frame(),
                                )?;
                            }
                            _ => panic!("Unexpected match! Got {:?}", (header.invoke_id(), code)),
                        }
                    }
                    AdsCommand::AdsWrite => {
                        let resp = AdsWriteResponse::parse_payload(payload)?;
                        match resp {
                            AdsReturnCode::Ok => println!("Write successful!"),
                            _ => panic!("Write failed! {:?}", resp),
                        }
                    }
                    AdsCommand::AdsRead => {
                        let (code, data) = AdsReadResponse::parse_payload(payload)?;
                        match code {
                            AdsReturnCode::Ok => println!(
                                "Read successful! {:?}",
                                u32::from_le_bytes(data.try_into()?)
                            ),
                            _ => panic!("Read failed! ({:?}, {:?})", code, data),
                        }
                    }
                    _ => todo!(),
                }
            }
            _ => {
                println!("Unknown frame: {:?}", frame);
            }
        }
    }

    Ok(())
}
