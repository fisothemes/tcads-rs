use std::collections::HashMap;
use std::net::ToSocketAddrs;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::{io, thread};
use tcads_core::ads::{DeviceState, NotificationHandle};
use tcads_core::io::blocking::{AmsStream, AmsWriter};
use tcads_core::protocol::{PortConnectRequest, PortConnectResponse, ProtocolError};
use tcads_core::{
    AdsHeader, AdsState, AmsAddr, AmsCommand, AmsFrame, IndexGroup, IndexOffset, InvokeId,
};

/// A map of pending requests awaiting a response from the PLC.
/// Keyed by invoke ID; value is a one-shot sender.
type PendingMap = HashMap<InvokeId, Sender<AmsFrame>>;

pub trait FrameHandler {
    fn handle(&self, frame: AmsFrame, pending: &Mutex<PendingMap>);
}

#[derive(Clone)]
pub struct AdsDevice {
    inner: Arc<AdsDeviceInner>,
}

impl AdsDevice {
    /// Connects to the local TwinCAT AMS Router (`127.0.0.1:48898`)
    /// and automatically requests an [AMS address](AmsAddr).
    pub fn connect() -> Result<Self, ProtocolError> {
        Self::connect_to("127.0.0.1:48898")
    }

    /// Connects to a custom AMS Router and automatically requests an [AMS address](AmsAddr).
    ///
    /// Useful if you are connecting to a remote PLC router but still want
    /// the router to assign your client an address.
    pub fn connect_to<A: ToSocketAddrs>(addr: A) -> Result<Self, ProtocolError> {
        let mut stream = AmsStream::connect(addr)?;

        let (mut reader, mut writer) = stream.try_split()?;

        writer.write_frame(&PortConnectRequest::default().into())?;

        let source = *PortConnectResponse::try_from(reader.read_frame()?)?.addr();

        let (write_tx, write_rx) = channel::<WriteRequest>();

        let pending = Mutex::new(HashMap::new());

        let inner = Arc::new(AdsDeviceInner {
            write_tx,
            pending,
            invoke_id: AtomicU32::new(1),
            source,
        });

        // Spawn writer thread that will serialise frames from a channel onto the TCP socket.
        // This should guarantee fairness because callers submit requests via a channel, FIFO ordering.
        thread::spawn(move || {
            for req in write_rx {
                if writer.write_frame(&req.frame).is_err() {
                    break;
                }
            }
        });

        // Spawn reader thread for handling incoming frames.
        let inner_reading = inner.clone();
        thread::spawn(move || {
            for result in reader.incoming() {
                let frame = match result {
                    Ok(f) => f,
                    Err(_) => break,
                };

                if frame.header().command() != AmsCommand::AdsCommand {
                    break;
                }

                let invoke_id = match AdsHeader::parse_prefix(frame.payload()) {
                    Ok((header, _)) => header.invoke_id(),
                    Err(_) => break,
                };

                let mut map = inner_reading.pending.lock().unwrap();
                if let Some(tx) = map.remove(&invoke_id) {
                    let _ = tx.send(frame);
                }
            }
            inner_reading.pending.lock().unwrap().clear();
        });

        Ok(Self { inner })
    }

    /// Connects to a custom AMS Router using an explicitly provided Source Address.
    ///
    /// This bypasses the handshake entirely. Necessary for clients where you must explicitly
    /// match the "Static Route" configured on the target PLC.
    pub fn connect_with_source<A: ToSocketAddrs>(
        addr: A,
        source: AmsAddr,
    ) -> Result<Self, ProtocolError> {
        todo!()
    }

    /// Sends a generic Read request to the target.
    pub fn read(
        &self,
        target: AmsAddr,
        index_group: IndexGroup,
        index_offset: IndexOffset,
        len: u32,
    ) -> io::Result<Vec<u8>> {
        todo!()
    }

    /// Sends a generic Write request to the target.
    pub fn write(
        &self,
        target: AmsAddr,
        index_group: IndexGroup,
        index_offset: IndexOffset,
        data: &[u8],
    ) -> io::Result<()> {
        todo!()
    }

    /// Sends an atomic ReadWrite request to the target.
    pub fn read_write(
        &self,
        target: AmsAddr,
        index_group: IndexGroup,
        index_offset: IndexOffset,
        read_len: u32,
        write_data: &[u8],
    ) -> io::Result<Vec<u8>> {
        todo!()
    }

    /// Reads the name and version of the target ADS device.
    pub fn read_device_info(&self, target: AmsAddr) -> io::Result<()> {
        todo!()
    }

    /// Reads the ADS State and Device State of the target.
    pub fn read_state(&self, target: AmsAddr) -> io::Result<(AdsState, DeviceState)> {
        todo!()
    }

    /// Changes the ADS State and Device State of the target.
    pub fn write_control(
        &self,
        target: AmsAddr,
        ads_state: AdsState,
        device_state: DeviceState,
        data: &[u8],
    ) -> io::Result<()> {
        todo!()
    }

    /// Subscribes to changes on a specific IndexGroup/IndexOffset.
    pub fn add_notification(
        &self,
        target: AmsAddr,
        index_group: IndexGroup,
        index_offset: IndexOffset,
        // Define a `NotificationAttributes` struct later probably...
    ) -> io::Result<NotificationHandle> {
        todo!()
    }

    /// Deletes an active notification.
    pub fn delete_notification(
        &self,
        target: AmsAddr,
        handle: NotificationHandle,
    ) -> io::Result<()> {
        todo!()
    }

    /// Returns the Source Address of this device.
    pub fn source(&self) -> AmsAddr {
        self.inner.source
    }

    fn next_invoke_id(&self) -> u32 {
        let id = self.inner.invoke_id.fetch_add(1, Ordering::Relaxed);
        if id == 0 {
            self.inner.invoke_id.fetch_add(1, Ordering::Relaxed)
        } else {
            id
        }
    }
}

struct WriteRequest {
    frame: AmsFrame,
    invoke_id: InvokeId,
}

struct AdsDeviceInner {
    write_tx: Sender<WriteRequest>,
    pending: Mutex<PendingMap>,
    invoke_id: AtomicU32,
    source: AmsAddr,
}
