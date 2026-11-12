use super::AdsRequest;
use std::io::Read;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use tcads_core::ads::StateFlag;
use tcads_core::protocol::RouterNotification;
use tcads_core::{AdsCommand, AdsHeader, AdsReturnCode, AmsCommand, AmsFrame};
use tcads_io::blocking::AmsReader;
pub struct AmsRequestReader;

impl AmsRequestReader {
    pub fn spawn<R: Read + Send + 'static>(
        write_tx: Sender<AmsFrame>,
        reader: AmsReader<R>,
    ) -> (Receiver<AdsRequest>, JoinHandle<crate::Result<()>>) {
        let (tx, rx) = mpsc::channel();
        let handle = thread::spawn(move || {
            for result in reader.incoming() {
                let Ok(frame) = result else { continue };

                match frame.header().command() {
                    AmsCommand::RouterNotification => {
                        if let Ok(notif) = RouterNotification::try_from_frame(&frame) {
                            tx.send(AdsRequest::RouterState(notif.state()))?;
                        }
                    }
                    AmsCommand::AdsCommand => {
                        let Ok((header, data)) = AdsHeader::parse_prefix(frame.payload()) else {
                            continue;
                        };

                        match header.command_id() {
                            AdsCommand::AdsReadDeviceInfo => tx.send(AdsRequest::ReadDeviceInfo)?,
                            AdsCommand::AdsRead => tx.send(AdsRequest::Read)?,
                            AdsCommand::AdsWrite => tx.send(AdsRequest::Write)?,
                            AdsCommand::AdsReadState => tx.send(AdsRequest::ReadState)?,
                            AdsCommand::AdsWriteControl => tx.send(AdsRequest::WriteControl)?,
                            AdsCommand::AdsAddDeviceNotification => {
                                tx.send(AdsRequest::AddNotification)?
                            }
                            AdsCommand::AdsDeleteDeviceNotification => {
                                tx.send(AdsRequest::DeleteNotification)?
                            }
                            AdsCommand::AdsReadWrite => tx.send(AdsRequest::ReadWrite)?,
                            _ => {
                                let header = AdsHeader::new(
                                    *header.source(),
                                    *header.target(),
                                    header.command_id(),
                                    StateFlag::tcp_ads_response(),
                                    0,
                                    AdsReturnCode::AdsErrDeviceSrvNotSupp,
                                    header.invoke_id(),
                                );

                                let frame =
                                    AmsFrame::new(AmsCommand::AdsCommand, header.to_bytes());
                                write_tx.send(frame)?;
                            }
                        }
                    }
                    AmsCommand::PortClose => break,
                    _ => {}
                }
            }

            Ok(())
        });

        (rx, handle)
    }
}
