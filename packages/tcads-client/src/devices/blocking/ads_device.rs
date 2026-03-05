use std::collections::HashMap;
use std::net::ToSocketAddrs;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use tcads_core::ads::{DeviceState, NotificationHandle};
use tcads_core::io::blocking::{AmsReader, AmsStream, AmsWriter};
use tcads_core::protocol::{
    AdsReadRequest, AdsReadResponse, AdsReadStateRequest, AdsReadStateResponse,
    AdsWriteControlRequestOwned, AdsWriteControlResponse, AdsWriteRequestOwned, AdsWriteResponse,
    GetLocalNetIdRequest, GetLocalNetIdResponse, PortCloseRequest, PortConnectRequest,
    PortConnectResponse, ProtocolError, RouterNotification,
};
use tcads_core::{
    AdsHeader, AdsReturnCode, AdsState, AmsAddr, AmsCommand, AmsFrame, AmsNetId, IndexGroup,
    IndexOffset, InvokeId, RouterState,
};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum PendingKey {
    AdsCommand(InvokeId),
    GetLocalNetId,
}

/// A map of pending requests awaiting a response from the PLC.
pub type PendingMap = HashMap<PendingKey, Sender<AmsFrame>>;

struct AdsDeviceInner {
    writer: Mutex<AmsWriter>,
    pending: Mutex<PendingMap>,
    invoke_id: AtomicU32,
    source: AmsAddr,
}

#[derive(Clone)]
pub struct AdsDevice {
    inner: Arc<AdsDeviceInner>,
}

impl AdsDevice {
    /// Connects to the local TwinCAT AMS Router (`127.0.0.1:48898`)
    /// and automatically requests an [AMS address](AmsAddr).
    pub fn connect() -> crate::Result<Self> {
        Self::connect_to("127.0.0.1:48898")
    }

    /// Connects to a custom AMS Router and automatically requests an [AMS address](AmsAddr).
    ///
    /// Useful if you are connecting to a remote PLC router but still want
    /// the router to assign your client an address.
    pub fn connect_to<A: ToSocketAddrs>(addr: A) -> crate::Result<Self> {
        let mut stream = AmsStream::connect(addr)?;

        let source = request_ams_addr(&mut stream)?;

        let (reader, writer) = stream.try_split()?;

        let pending = Mutex::new(HashMap::new());

        let inner = Arc::new(AdsDeviceInner {
            writer: Mutex::new(writer),
            pending,
            invoke_id: AtomicU32::new(1),
            source,
        });

        let _ = spawn_reader_thread(reader, inner.clone());

        Ok(Self { inner })
    }

    /// Connects to a custom AMS Router using an explicitly provided Source Address.
    ///
    /// This bypasses the handshake entirely. Necessary for clients where you must explicitly
    /// match the "Static Route" configured on the target PLC.
    pub fn connect_with_source<A: ToSocketAddrs>(addr: A, source: AmsAddr) -> crate::Result<Self> {
        todo!()
    }

    /// Sends a generic Read request to the target.
    pub fn read(
        &self,
        target: AmsAddr,
        index_group: IndexGroup,
        index_offset: IndexOffset,
        len: u32,
    ) -> crate::Result<Vec<u8>> {
        let invoke_id = self.next_invoke_id();
        let frame = AdsReadRequest::new(
            target,
            self.inner.source,
            invoke_id,
            index_group,
            index_offset,
            len,
        )
        .into_frame();
        let frame = self.send_and_wait(frame, invoke_id)?;
        let resp = AdsReadResponse::try_from(&frame)?;
        check_result(resp.result())?;
        Ok(resp.data().to_vec())
    }

    /// Sends a generic Write request to the target.
    pub fn write(
        &self,
        target: AmsAddr,
        index_group: IndexGroup,
        index_offset: IndexOffset,
        data: &[u8],
    ) -> crate::Result<()> {
        let invoke_id = self.next_invoke_id();
        let frame = AdsWriteRequestOwned::new(
            target,
            self.inner.source,
            invoke_id,
            index_group,
            index_offset,
            data,
        )
        .into_frame();
        let resp = AdsWriteResponse::try_from(&self.send_and_wait(frame, invoke_id)?)?;
        check_result(resp.result())
    }

    /// Sends an atomic ReadWrite request to the target.
    pub fn read_write(
        &self,
        target: AmsAddr,
        index_group: IndexGroup,
        index_offset: IndexOffset,
        read_len: u32,
        write_data: &[u8],
    ) -> crate::Result<Vec<u8>> {
        todo!()
    }

    /// Reads the name and version of the target ADS device.
    pub fn read_device_info(&self, target: AmsAddr) -> crate::Result<()> {
        todo!()
    }

    /// Reads the ADS State and Device State of the target.
    pub fn read_state(&self, target: AmsAddr) -> crate::Result<(AdsState, DeviceState)> {
        let invoke_id = self.next_invoke_id();
        let frame = AdsReadStateRequest::new(target, self.inner.source, invoke_id).into_frame();
        let resp = AdsReadStateResponse::try_from(&self.send_and_wait(frame, invoke_id)?)?;

        check_result(resp.result())?;
        Ok((resp.ads_state(), resp.device_state()))
    }

    /// Changes the ADS State and Device State of the target.
    pub fn write_control(
        &self,
        target: AmsAddr,
        ads_state: AdsState,
        device_state: DeviceState,
        data: &[u8],
    ) -> crate::Result<()> {
        let invoke_id = self.next_invoke_id();
        let frame = AdsWriteControlRequestOwned::with_data(
            target,
            self.inner.source,
            invoke_id,
            ads_state,
            device_state,
            data,
        )
        .into_frame();
        let resp = AdsWriteControlResponse::try_from(&self.send_and_wait(frame, invoke_id)?)?;
        check_result(resp.result())
    }

    /// Subscribes to changes on a specific IndexGroup/IndexOffset.
    pub fn add_notification(
        &self,
        target: AmsAddr,
        index_group: IndexGroup,
        index_offset: IndexOffset,
        // Define a `NotificationAttributes` struct later probably...
    ) -> crate::Result<NotificationHandle> {
        todo!()
    }

    /// Deletes an active notification.
    pub fn delete_notification(
        &self,
        target: AmsAddr,
        handle: NotificationHandle,
    ) -> crate::Result<()> {
        todo!()
    }

    /// Returns the Source Address of this device.
    pub fn source(&self) -> AmsAddr {
        self.inner.source
    }

    /// Fetches the AMS Router's Local [Net ID](AmsNetId).
    pub fn get_local_net_id(&self) -> crate::Result<AmsNetId> {
        let (tx, rx) = channel();

        {
            self.inner
                .writer
                .lock()
                .expect("Writer lock should be held")
                .write_frame(&GetLocalNetIdRequest::into_frame())?;

            self.inner
                .pending
                .lock()
                .expect("Pending lock should be held")
                .insert(PendingKey::GetLocalNetId, tx);
        }

        let frame = rx.recv().map_err(|_| crate::Error::Disconnected)?;
        let net_id = GetLocalNetIdResponse::try_from(frame)?.net_id();
        Ok(net_id)
    }

    /// Generates the next [Invoke ID](InvokeId)
    fn next_invoke_id(&self) -> InvokeId {
        let id = self.inner.invoke_id.fetch_add(1, Ordering::Relaxed);
        if id == 0 {
            self.inner.invoke_id.fetch_add(1, Ordering::Relaxed)
        } else {
            id
        }
    }

    fn send_and_wait(&self, frame: AmsFrame, invoke_id: InvokeId) -> crate::Result<AmsFrame> {
        let (tx, rx) = channel();

        {
            self.inner
                .pending
                .lock()
                .expect("Pending lock should be held")
                .insert(PendingKey::AdsCommand(invoke_id), tx);

            self.inner
                .writer
                .lock()
                .expect("Writer lock should be held")
                .write_frame(&frame)?;
        }

        rx.recv().map_err(|_| crate::Error::Disconnected)
    }
}

fn request_ams_addr(stream: &mut AmsStream) -> Result<AmsAddr, ProtocolError> {
    stream.write_frame(&PortConnectRequest::default().into())?;

    let addr = *PortConnectResponse::try_from(stream.read_frame()?)?.addr();

    Ok(addr)
}

fn check_result(code: AdsReturnCode) -> crate::Result<()> {
    match code {
        AdsReturnCode::Ok => Ok(()),
        code => Err(code.into()),
    }
}

fn spawn_reader_thread(
    reader: AmsReader,
    inner: Arc<AdsDeviceInner>,
) -> JoinHandle<crate::Result<()>> {
    let handle = thread::spawn(move || -> crate::Result<()> {
        for result in reader.incoming() {
            if let Ok(frame) = result {
                match frame.header().command() {
                    AmsCommand::PortConnect => {
                        panic!("Port connect should not be received on the reader thread!")
                    }
                    AmsCommand::GetLocalNetId => {
                        let mut map = inner.pending.lock().unwrap();
                        if let Some(tx) = map.remove(&PendingKey::GetLocalNetId) {
                            let _ = tx.send(frame);
                        }
                    }
                    AmsCommand::PortClose => break,
                    AmsCommand::RouterNotification => {
                        if let Ok(resp) = RouterNotification::try_from(frame) {
                            match resp.state() {
                                RouterState::Removed => {
                                    inner
                                        .writer
                                        .lock()
                                        .expect("Writer lock should be held")
                                        .write_frame(
                                            &PortCloseRequest::new(inner.source.port()).into(),
                                        )?;
                                }
                                _ => {} // TODO: Handle other states
                            }
                        }
                    }
                    AmsCommand::AdsCommand => {
                        if let Ok((header, _)) = AdsHeader::parse_prefix(frame.payload()) {
                            let mut map =
                                inner.pending.lock().expect("Pending lock should be held");
                            if let Some(tx) =
                                map.remove(&PendingKey::AdsCommand(header.invoke_id()))
                            {
                                let _ = tx.send(frame);
                            }
                        };
                    }
                    other => todo!("Handle Unknown command: {other:?}"),
                }
            }
        }

        inner
            .pending
            .lock()
            .expect("Pending lock should be held")
            .clear();

        Ok(())
    });

    handle
}
