use crate::tasks::blocking::{
    AdsNotificationDispatcher, AmsRequestDispatchKey, AmsRequestDispatcher, AmsRequestWriter,
    AmsResponseReader, RouterNotificationDispatcher,
};
use std::io::{Read, Write};
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tcads_core::io::blocking::{AmsReader, AmsStream, AmsWriter};
use tcads_core::protocol::{
    AdsAddDeviceNotificationRequest, AdsAddDeviceNotificationResponse,
    AdsDeleteDeviceNotificationRequest, AdsDeleteDeviceNotificationResponse,
    AdsNotificationSampleOwned, AdsReadDeviceInfoRequest, AdsReadDeviceInfoResponse,
    AdsReadRequest, AdsReadResponse, AdsReadStateRequest, AdsReadStateResponse,
    AdsReadWriteRequestOwned, AdsReadWriteResponse, AdsWriteControlRequestOwned,
    AdsWriteControlResponse, AdsWriteRequestOwned, AdsWriteResponse, GetLocalNetIdRequest,
    GetLocalNetIdResponse, PortCloseRequest, PortConnectRequest, PortConnectResponse,
};
use tcads_core::{
    AdsDeviceVersion, AdsHeader, AdsNotificationAttrib, AdsReturnCode, AdsState, AmsAddr,
    AmsCommand, AmsFrame, AmsNetId, DeviceState, IndexGroup, IndexOffset, InvokeId,
    NotificationHandle, RouterState,
};

/// Shared state for an [`AdsDevice`] connection.
///
/// Held behind an [`Arc`] so all [`AdsDevice`] clones share the same connection.
/// Exposed as `pub` for power users who need direct access to the underlying
/// dispatchers to build custom device abstractions on top of the
/// same connection without going through the [`AdsDevice`] API.
///
/// # Lifetime
///
/// The reader and writer threads are tied to the lifetime of this struct.
/// When the last [`AdsDevice`] clone is dropped, `AdsDeviceInner` drops,
/// which drops [`AmsRequestDispatcher`] and its `write_tx`. The writer thread
/// exits when `write_tx` is dropped, the TCP stream closes, and the reader
/// thread exits on the next read returning EOF.
pub struct AdsDeviceInner {
    pub ams_requests: Arc<AmsRequestDispatcher>,
    pub ads_notifs: Arc<AdsNotificationDispatcher>,
    pub router_notifs: Arc<RouterNotificationDispatcher>,
    pub source: RwLock<AmsAddr>,
    pub invoke_id: AtomicU32,
    pub timeout: Option<Duration>,
}

/// A blocking ADS device client.
///
/// `AdsDevice` manages a TCP connection to an AMS router and exposes all
/// standard ADS commands as async methods. It is designed to be used standalone
/// or as a building block for higher-level device abstractions.
///
/// # Connection
///
/// There are two ways to connect depending on whether an AMS router is
/// running on the client machine.
///
/// ### 1. Connecting via a local router
///
/// Use [`connect`](Self::connect) or [`connect_to`](Self::connect_to). The
/// local router performs a [`PortConnect`](cra) handshake and dynamically assigns
/// a source address to the client.
///
/// ```no_run
/// use tcads_client::devices::blocking::AdsDevice;
/// use std::time::Duration;
///
/// // Connect to the local router at 127.0.0.1:48898
/// let device = AdsDevice::connect(Duration::from_secs(5))?;
///
/// // Connect to a router at a specific address
/// let device = AdsDevice::connect_to("192.168.1.50:48898", Duration::from_secs(5))?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// ### 2. Connecting directly to a remote router
///
/// Use [`connect_remote`](Self::connect_remote) when no local router is
/// present. The source address must be pre-configured as a static route on
/// the remote router, see [`connect_remote`](Self::connect_remote) for
/// details.
///
/// ```no_run
/// use tcads_client::devices::blocking::AdsDevice;
///
/// let source = "192.168.1.10.1.1:32750".parse()?;
/// let device = AdsDevice::connect_remote("192.168.1.120:48898", source, None)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Thread Safety
///
/// `AdsDevice` is [`Clone`], so all clones share the same underlying connection.
/// It is also [`Send`] + [`Sync`], so multiple tasks can issue ADS commands
/// concurrently. Responses are matched to their callers via Invoke ID with no
/// global lock on the connection.
///
/// # Shutdown
///
/// Call [`shutdown`](AdsDevice::shutdown) for a clean disconnect. Dropping
/// the last `AdsDevice` clone also tears down the connection automatically.
/// The writer thread exits when its sender is dropped, the reader thread
/// exits when the TCP stream closes, and all pending callers receive
/// [`Error::Disconnected`](crate::Error::Disconnected).
#[derive(Clone)]
pub struct AdsDevice {
    inner: Arc<AdsDeviceInner>,
}

impl AdsDevice {
    /// Connects to the local AMS router at `127.0.0.1:48898`.
    ///
    /// Performs a [`PortConnect`](PortConnectRequest) handshake to obtain a
    /// dynamically assigned source address.
    ///
    /// # Note
    ///
    /// On Windows, connecting via `127.0.0.1` requires the
    /// `EnableAmsTcpLoopback` registry key to be set. This is enabled by
    /// default in TwinCAT 4024.5 and newer.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use tcads_client::devices::blocking::AdsDevice;
    /// use std::time::Duration;
    ///
    /// let device = AdsDevice::connect(Duration::from_secs(5))?;
    /// println!("Source: {}", device.source()?);
    /// device.shutdown()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn connect(timeout: impl Into<Option<Duration>>) -> crate::Result<Self> {
        Self::connect_to("127.0.0.1:48898", timeout)
    }

    /// Connects to an AMS router at `addr`.
    ///
    /// Performs a [`PortConnect`](PortConnectRequest) handshake to obtain a
    /// dynamically assigned source address.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use tcads_client::devices::blocking::AdsDevice;
    ///
    /// let device = AdsDevice::connect_to("192.168.1.100:48898", None)?;
    /// println!("Source: {}", device.source()?);
    /// device.shutdown()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn connect_to(
        addr: impl ToSocketAddrs,
        timeout: impl Into<Option<Duration>>,
    ) -> crate::Result<Self> {
        let timeout = timeout.into();
        let (reader, writer) = match timeout {
            Some(duration) => {
                let addr: SocketAddr = addr.to_socket_addrs()?.next().ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "could not resolve address",
                    )
                })?;
                AmsStream::connect_timeout(&addr, duration)?.try_split()?
            }
            None => AmsStream::connect(addr)?.try_split()?,
        };
        let device = Self::new(reader, writer, AmsAddr::default(), timeout);
        let source = device.port_connect()?;
        *device.inner.source.write()? = source;
        Ok(device)
    }

    /// Connects directly to a remote AMS router without a local router.
    ///
    /// Use this when no AMS router is running on the client machine.
    ///
    /// The `source` address must be pre-configured as a static route on the
    /// remote router. The remote router identifies the client by the source
    /// address carried in each ADS AMS frame.
    ///
    /// # Note
    ///
    /// Frames from unrecognized source addresses will cause the router to close the TCP connection.
    ///
    /// The [`PortConnect`](PortConnectRequest) handshake is **not** performed.
    /// Remote routers do not support this command and will also close the TCP
    /// connection if they receive one.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use tcads_client::devices::blocking::AdsDevice;
    ///
    /// // Source Net ID must match the static route entry on the remote router
    /// // The port can be any unused number.
    /// let source = "192.168.1.10.1.1:32750".parse()?;
    /// let device = AdsDevice::connect_remote("192.168.1.120:48898", source, None)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn connect_remote(
        addr: impl ToSocketAddrs,
        source: AmsAddr,
        timeout: impl Into<Option<Duration>>,
    ) -> crate::Result<Self> {
        let timeout = timeout.into();
        let (reader, writer) = match timeout {
            Some(duration) => {
                let addr: SocketAddr = addr.to_socket_addrs()?.next().ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "could not resolve address",
                    )
                })?;
                AmsStream::connect_timeout(&addr, duration)?.try_split()?
            }
            None => AmsStream::connect(addr)?.try_split()?,
        };
        Ok(Self::new(reader, writer, source, timeout))
    }

    /// Creates an [`AdsDevice`] from an already-split reader and writer.
    ///
    /// This is the low-level constructor intended for power users who need
    /// full control over the underlying transport to, for example, use a
    /// TLS stream, a Unix socket, or inject mock streams in tests.
    ///
    /// Unlike the `connect_*` constructors, `new` does **not** perform a
    /// `PortConnect` handshake. The caller is responsible for providing a
    /// valid `source` address.
    ///
    /// The `timeout` applies to all ADS command round-trips. Pass a
    /// [`Duration`] directly or `None` for no timeout.
    ///
    /// # Example
    ///
    /// ```no_run ignore
    /// use tcads_core::io::blocking::{AmsReader, AmsWriter};
    /// use tcads_client::devices::blocking::AdsDevice;
    /// use std::time::Duration;
    ///
    /// let reader = AmsReader::new(/* your reader */);
    /// let writer = AmsWriter::new(/* your writer */);
    /// let source = "192.168.1.10.1.1:32750".parse()?;
    /// let device = AdsDevice::new(reader, writer, source, Duration::from_secs(5));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new<R, W>(
        reader: AmsReader<R>,
        writer: AmsWriter<W>,
        source: AmsAddr,
        timeout: impl Into<Option<Duration>>,
    ) -> Self
    where
        R: Read + Send + 'static,
        W: Write + Send + 'static,
    {
        let (write_tx, _) = AmsRequestWriter::spawn(writer);

        let ams_requests = Arc::new(AmsRequestDispatcher::new(write_tx));
        let ads_notifs = Arc::new(AdsNotificationDispatcher::new());
        let router_notifs = Arc::new(RouterNotificationDispatcher::new());

        AmsResponseReader::spawn(
            reader,
            Arc::clone(&ams_requests),
            Arc::clone(&ads_notifs),
            Arc::clone(&router_notifs),
        );

        Self {
            inner: Arc::new(AdsDeviceInner {
                ams_requests,
                ads_notifs,
                router_notifs,
                source: RwLock::new(source),
                invoke_id: AtomicU32::new(1),
                timeout: timeout.into(),
            }),
        }
    }

    /// Gracefully shuts down the connection.
    ///
    /// Sends a [`PortClose`](PortCloseRequest) frame to the router. The writer
    /// thread writes it and exits, dropping the channel receiver which invalidates
    /// all senders. The router closes the TCP connection, causing the reader thread
    /// to hit EOF and exit, clearing all pending callers and notification subscribers.
    ///
    /// If the send fails (already disconnected) this returns `Ok(())` meaning the
    /// connection is already gone.
    pub fn shutdown(&self) -> crate::Result<()> {
        let frame = PortCloseRequest::new(self.source()?.port()).into_frame();
        let _ = self.inner.ams_requests.send_only(frame);
        Ok(())
    }

    /// Returns the source [`AmsAddr`] currently assigned to this connection.
    pub fn source(&self) -> crate::Result<AmsAddr> {
        Ok(*self.inner.source.read()?)
    }

    /// Queries the local AMS router's Net ID.
    ///
    /// # Warning
    ///
    /// This command is only supported by a local AMS router. Do not call this
    /// on a device created with [`connect_remote`](Self::connect_remote). The
    /// remote router does not support this command and will close the TCP
    /// connection, causing subsequent calls to return.
    /// [`Error::Disconnected`](crate::Error::Disconnected).
    pub fn get_local_net_id(&self) -> crate::Result<AmsNetId> {
        let frame = GetLocalNetIdRequest::into_frame();
        let rx = self
            .inner
            .ams_requests
            .dispatch(AmsRequestDispatchKey::GetLocalNetId, frame)?;
        let resp = GetLocalNetIdResponse::try_from(self.wait(rx)?)?;

        Ok(resp.net_id())
    }

    /// Subscribes to router state changes.
    ///
    /// Returns a [`Receiver`] that yields each [`RouterState`] transition.
    /// The receiver yields [`Err`] when the connection is lost or all
    /// `AdsDevice` clones are dropped.
    pub fn subscribe_router(&self) -> crate::Result<Receiver<RouterState>> {
        self.inner.router_notifs.subscribe()
    }

    /// Reads the device name and version from `target`.
    pub fn read_device_info(&self, target: AmsAddr) -> crate::Result<(AdsDeviceVersion, String)> {
        let invoke_id = self.next_invoke_id();

        let frame = AdsReadDeviceInfoRequest::new(target, self.source()?, invoke_id).into_frame();
        let resp = AdsReadDeviceInfoResponse::try_from(self.send_and_wait(frame, invoke_id)?)?;

        Self::check_result(resp.result())?;

        Ok((resp.version(), resp.device_name().into_owned()))
    }

    /// Reads the ADS and device state of `target`.
    pub fn read_state(&self, target: AmsAddr) -> crate::Result<(AdsState, DeviceState)> {
        let invoke_id = self.next_invoke_id();

        let frame = AdsReadStateRequest::new(target, self.source()?, invoke_id).into_frame();
        let resp = AdsReadStateResponse::try_from(self.send_and_wait(frame, invoke_id)?)?;

        Self::check_result(resp.result())?;

        Ok((resp.ads_state(), resp.device_state()))
    }

    /// Changes the ADS and device state of `target`.
    pub fn write_control(
        &self,
        target: AmsAddr,
        ads_state: AdsState,
        device_state: DeviceState,
        data: impl Into<Vec<u8>>,
    ) -> crate::Result<()> {
        let invoke_id = self.next_invoke_id();

        let frame = AdsWriteControlRequestOwned::with_data(
            target,
            self.source()?,
            invoke_id,
            ads_state,
            device_state,
            data,
        )
        .into_frame();
        let resp = AdsWriteControlResponse::try_from(&self.send_and_wait(frame, invoke_id)?)?;

        Self::check_result(resp.result())?;

        Ok(())
    }

    /// Reads `length` of bytes from `target` at specified a `index_group` and `index_offset`.
    pub fn read(
        &self,
        target: AmsAddr,
        index_group: IndexGroup,
        index_offset: IndexOffset,
        length: u32,
    ) -> crate::Result<Vec<u8>> {
        let invoke_id = self.next_invoke_id();

        let frame = AdsReadRequest::new(
            target,
            self.source()?,
            invoke_id,
            index_group,
            index_offset,
            length,
        )
        .into_frame();
        let frame = self.send_and_wait(frame, invoke_id)?;
        let resp = AdsReadResponse::try_from_frame(&frame)?;

        Self::check_result(resp.result())?;

        Ok(resp.data().to_vec())
    }

    /// Writes `data` to `target` at a specified `index_group` and `index_offset`.
    pub fn write(
        &self,
        target: AmsAddr,
        index_group: IndexGroup,
        index_offset: IndexOffset,
        data: impl Into<Vec<u8>>,
    ) -> crate::Result<()> {
        let invoke_id = self.next_invoke_id();
        let frame = AdsWriteRequestOwned::new(
            target,
            self.source()?,
            invoke_id,
            index_group,
            index_offset,
            data,
        )
        .into_frame();
        let resp = AdsWriteResponse::try_from(self.send_and_wait(frame, invoke_id)?)?;

        Self::check_result(resp.result())?;

        Ok(())
    }

    /// Sends a combined read/write to `target` in a single round trip.
    ///
    /// Writes `write_data` then reads `read_length` bytes back.
    pub fn read_write(
        &self,
        target: AmsAddr,
        index_group: IndexGroup,
        index_offset: IndexOffset,
        read_length: u32,
        write_data: impl Into<Vec<u8>>,
    ) -> crate::Result<Vec<u8>> {
        let invoke_id = self.next_invoke_id();
        let frame = AdsReadWriteRequestOwned::new(
            target,
            self.source()?,
            invoke_id,
            index_group,
            index_offset,
            read_length,
            write_data,
        )
        .into_frame();
        let frame = self.send_and_wait(frame, invoke_id)?;
        let resp = AdsReadWriteResponse::try_from_frame(&frame)?;

        Self::check_result(resp.result())?;

        Ok(resp.data().to_vec())
    }

    /// Registers a device notification on `target`.
    ///
    /// Returns a [`Receiver`] for incoming samples and the [`NotificationHandle`]
    /// assigned by the PLC.
    ///
    /// The receiver yields [`Err`] after [`delete_notification`](Self::delete_notification)
    /// is called, or when the router transitions to [`RouterState::Stop`] or [`RouterState::Removed`].
    ///
    /// # Note
    ///
    /// The target device may fire an initial sample upon registration.
    #[allow(clippy::too_many_arguments)]
    pub fn add_notification(
        &self,
        target: AmsAddr,
        index_group: IndexGroup,
        index_offset: IndexOffset,
        notif_attr: AdsNotificationAttrib,
    ) -> crate::Result<(Receiver<AdsNotificationSampleOwned>, NotificationHandle)> {
        let invoke_id = self.next_invoke_id();

        let rx = self.inner.ads_notifs.pre_register(invoke_id)?;

        let frame = AdsAddDeviceNotificationRequest::new(
            target,
            self.source()?,
            invoke_id,
            index_group,
            index_offset,
            notif_attr,
        )
        .into_frame();
        let resp =
            AdsAddDeviceNotificationResponse::try_from(self.send_and_wait(frame, invoke_id)?)?;

        Self::check_result(resp.result())?;

        let handle = resp.handle();
        self.inner.ads_notifs.promote(invoke_id, handle)?;

        Ok((rx, handle))
    }

    /// Deletes a device notification on `target`.
    ///
    /// The receiver obtained from [`add_notification`](Self::add_notification)
    /// will yield [`Err`] on its next [`recv`](Receiver::recv) call.
    pub fn delete_notification(
        &self,
        target: AmsAddr,
        handle: NotificationHandle,
    ) -> crate::Result<()> {
        let invoke_id = self.next_invoke_id();

        let frame =
            AdsDeleteDeviceNotificationRequest::new(target, self.source()?, invoke_id, handle)
                .into_frame();
        let resp =
            AdsDeleteDeviceNotificationResponse::try_from(self.send_and_wait(frame, invoke_id)?)?;

        Self::check_result(resp.result())?;

        self.inner.ads_notifs.remove(handle)
    }

    /// Sends a fire-and-forget frame to the router.
    ///
    /// This method exists for unconventional ADS commands that do not follow the
    /// standard request/response pattern, such as
    /// [`AdsDeviceNotification`](tcads_core::AdsCommand::AdsDeviceNotification) frames
    /// written directly to the TwinCAT logger (port 100). For standard commands
    /// that expect a response, use the high-level methods on [`AdsDevice`].
    ///
    /// # Warning
    ///
    /// The frame is queued on the writer channel and sent as-is. No `InvokeId`
    /// is registered with the dispatcher, so **any reply the router sends will be
    /// silently dropped**.
    ///
    /// # Safety
    ///
    /// The caller must ensure the [`AmsFrame`] is correctly formed for the intended
    /// unconventional command. An incorrect frame such as; a malformed ADS header,
    /// wrong target address, or an invalid payload layout, can cause undefined
    /// behaviour in the remote TwinCAT runtime. This including crashes, silent data
    /// corruption, or unexpected state transitions that affect the entire PLC task.
    pub unsafe fn write_frame_only(&self, frame: AmsFrame) -> crate::Result<()> {
        self.inner.ams_requests.send_only(frame)
    }

    /// Sends a frame and returns a [`Receiver`] for the matching response.
    ///
    /// This method exists for unconventional ADS commands that are not covered by
    /// the high-level API on [`AdsDevice`]. For standard commands, prefer the
    /// typed methods (e.g. [`read`](Self::read), [`write`](Self::write)) which
    /// handle `InvokeId` assignment, frame construction, and response parsing.
    /// For unconventional commands that never receive a response, use
    /// [`write_frame_only`](Self::write_frame_only) instead.
    ///
    /// # Errors
    ///
    /// Returns [`Err(Error::InvalidPayload)`](crate::Error::InvalidPayload) if the
    /// frame carries an [`AmsCommand`] variant that has no dispatch key (anything
    /// other than [`AdsCommand`](AmsCommand::AdsCommand), [`GetLocalNetId`](AmsCommand::GetLocalNetId),
    /// or [`PortConnect`](AmsCommand::PortConnect)).
    ///
    /// # Safety
    ///
    /// The caller must ensure the [`AmsFrame`] is correctly formed for the intended
    /// unconventional command. An incorrect frame such as; a malformed ADS header,
    /// wrong target address, or an invalid payload layout, can cause undefined
    /// behaviour in the remote TwinCAT runtime. This including crashes, silent data
    /// corruption, or unexpected state transitions that affect the entire PLC task.
    ///
    /// The caller is also responsible for ensuring the `InvokeId` embedded in the
    /// [`AdsHeader`] is unique among all in-flight requests on this connection.
    /// A duplicate `InvokeId` will cause the wrong caller to receive the response.
    pub unsafe fn write_frame(&self, frame: AmsFrame) -> crate::Result<Receiver<AmsFrame>> {
        let key = match frame.header().command() {
            AmsCommand::AdsCommand => {
                let (header, _) =
                    AdsHeader::parse_prefix(frame.payload()).map_err(tcads_core::AdsError::from)?;
                AmsRequestDispatchKey::AdsCommand(header.invoke_id())
            }
            AmsCommand::GetLocalNetId => AmsRequestDispatchKey::GetLocalNetId,
            AmsCommand::PortConnect => AmsRequestDispatchKey::PortConnect,
            _ => return Err(crate::Error::InvalidPayload),
        };
        self.inner.ams_requests.dispatch(key, frame)
    }

    /// Returns a reference to the shared internal state and dispatchers of the ADS device.
    ///
    /// This method acts as an escape hatch for power users and library authors
    /// who need to build custom device abstractions (such as high-performance batch wrappers
    /// or custom protocol extensions) that share the same underlying TCP connection.
    ///
    /// By accessing [`AdsDeviceInner`], you can interact directly with the
    /// network request queues, notification routing tables, and the `InvokeId`
    /// generator without being constrained by the high-level request/response API.
    ///
    /// # Thread Safety
    ///
    /// All fields within [`AdsDeviceInner`] are heavily protected by atomic operations
    /// and thread-safe primitives (`Arc`, `RwLock`, channels). It is entirely safe
    /// to access and mutate the inner routing state concurrently across multiple threads.
    pub fn inner(&self) -> &AdsDeviceInner {
        &self.inner
    }

    fn port_connect(&self) -> crate::Result<AmsAddr> {
        let frame = PortConnectRequest::default().into_frame();
        let rx = self
            .inner
            .ams_requests
            .dispatch(AmsRequestDispatchKey::PortConnect, frame)?;
        let resp = PortConnectResponse::try_from(self.wait(rx)?)?;

        Ok(*resp.addr())
    }

    fn send_and_wait(&self, frame: AmsFrame, invoke_id: InvokeId) -> crate::Result<AmsFrame> {
        let rx = self
            .inner
            .ams_requests
            .dispatch(AmsRequestDispatchKey::AdsCommand(invoke_id), frame)?;
        self.wait(rx)
    }

    fn wait(&self, rx: Receiver<AmsFrame>) -> crate::Result<AmsFrame> {
        match self.inner.timeout {
            Some(duration) => Ok(rx.recv_timeout(duration)?),
            None => Ok(rx.recv()?),
        }
    }

    fn next_invoke_id(&self) -> InvokeId {
        self.inner.invoke_id.fetch_add(1, Ordering::Relaxed)
    }

    fn check_result(code: AdsReturnCode) -> crate::Result<()> {
        match code {
            AdsReturnCode::Ok => Ok(()),
            code => Err(code.into()),
        }
    }
}
