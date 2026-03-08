use super::{AmsRequestDispatcher, DispatchKey};
use std::io::Read;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use tcads_core::io::blocking::AmsReader;
use tcads_core::protocol::RouterNotification;
use tcads_core::{AdsCommand, AdsHeader, AmsCommand};

/// Spawns a dedicated reader thread for deserializing incoming [AMS frames](tcads_core::AmsFrame)
/// and routing them to their waiting callers via the [`AmsRequestDispatcher`].
pub struct AmsResponseReader;

impl AmsResponseReader {
    /// Spawns the reader thread.
    ///
    /// The thread runs until the underlying stream reaches EOF or a [`AmsCommand::PortClose`]
    /// frame is received. On exit, [`AmsRequestDispatcher::clear`] is called unconditionally,
    /// waking all pending callers with [`Error::Disconnected`](crate::Error::Disconnected).
    ///
    /// The returned [`JoinHandle`] carries a [`Result`](crate::Result) that callers can join to
    /// surface any error that caused the reader to exit unexpectedly.
    pub fn spawn<R: Read + Send + 'static>(
        reader: AmsReader<R>,
        requests: Arc<AmsRequestDispatcher>,
    ) -> JoinHandle<crate::Result<()>> {
        thread::spawn(move || {
            let result = handle(reader, &requests);
            requests.clear()?;
            result
        })
    }
}

fn handle<R: Read>(reader: AmsReader<R>, requests: &AmsRequestDispatcher) -> crate::Result<()> {
    for result in reader.incoming() {
        let frame = match result {
            Ok(frame) => frame,
            Err(_) => continue,
        };

        match frame.header().command() {
            AmsCommand::PortConnect => requests.complete(DispatchKey::PortConnect, frame)?,
            AmsCommand::GetLocalNetId => requests.complete(DispatchKey::GetLocalNetId, frame)?,
            AmsCommand::AdsCommand => {
                if let Ok((header, _)) = AdsHeader::parse_prefix(frame.payload()) {
                    match header.command_id() {
                        AdsCommand::AdsDeviceNotification => {
                            todo!(
                                "Create an ADS notification dispatcher to handle ADS device notifications."
                            )
                        }
                        _ => {
                            requests.complete(DispatchKey::AdsCommand(header.invoke_id()), frame)?
                        }
                    }
                }
            }
            AmsCommand::RouterNotification => {
                if let Ok(_) = RouterNotification::try_from(frame) {
                    todo!("Create a router notification dispatcher to handle router notifications.")
                }
            }
            AmsCommand::PortClose => break,
            _ => {}
        }
    }
    Ok(())
}
