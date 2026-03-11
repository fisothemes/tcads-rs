use crate::tasks::blocking::{
    AdsNotificationDispatcher, AmsRequestDispatchKey, AmsRequestDispatcher, AmsRequestWriter,
    AmsResponseReader, RouterNotificationDispatcher,
};
use std::net::ToSocketAddrs;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tcads_core::io::blocking::AmsStream;
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
    AdsDeviceVersion, AdsReturnCode, AdsState, AdsTransMode, AmsAddr, AmsFrame, AmsNetId,
    DeviceState, IndexGroup, IndexOffset, InvokeId, NotificationHandle, RouterState,
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
/// thread exits on the next read returning EO
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
/// The `AdsDevice` manages a single TCP connection to an AMS router and exposes all standard
/// ADS commands as blocking methods. It is cheap to clone, and all clones share the same
/// underlying connection and state.
///
/// # Connection
///
/// Use one of the `connect` constructors:
///
/// ```no_run
/// use std::time::Duration;
/// use tcads_client::devices::blocking::AdsDevice;
///
/// // Local router, auto-assigned source
/// let device = AdsDevice::connect(Some(Duration::from_secs(5)))?;
///
/// // Remote router, auto-assigned source
/// let device = AdsDevice::connect_to("192.168.1.100:48898", Some(Duration::from_secs(5)))?;
///
/// // Remote router, explicit source, skips PortConnect handshake
/// let source = "192.168.1.100.1.1:32838".parse()?;
/// let device = AdsDevice::connect_with_source("192.168.1.100:48898", source, None)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Thread Safety
///
/// `AdsDevice` is `Send + Sync`. Multiple threads can issue ADS commands concurrently.
/// Responses are matched to their callers by Invoke ID with no global lock on the
/// connection.
///
/// # Shutdown
///
/// Call [`shutdown`](AdsDevice::shutdown) for a clean disconnect. Dropping the last
/// `AdsDevice` clone also tears down the connection automatically. The writer thread
/// exits when its sender is dropped, the reader thread exits when the TCP stream closes,
/// and all pending callers receive [`Error::Disconnected`](crate::Error::Disconnected).
#[derive(Clone)]
pub struct AdsDevice {
    inner: Arc<AdsDeviceInner>,
}

impl AdsDevice {
    /// Connects to the local AMS router at `127.0.0.1:48898`.
    ///
    /// Performs a [`PortConnect`](PortConnectRequest) handshake to get a
    /// dynamically assigned source address.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use tcads_client::devices::blocking::AdsDevice;
    ///
    /// let device = AdsDevice::connect(None)?;
    ///
    /// println!("Source: {}", device.source()?);
    /// println!("Local Net ID: {}", device.get_local_net_id()?);
    ///
    /// device.shutdown()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn connect(timeout: Option<Duration>) -> crate::Result<Self> {
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
    ///
    /// println!("Source: {}", device.source()?);
    /// println!("Local Net ID: {}", device.get_local_net_id()?);
    ///
    /// device.shutdown()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn connect_to(addr: impl ToSocketAddrs, timeout: Option<Duration>) -> crate::Result<Self> {
        let stream = AmsStream::connect(addr)?;
        let device = Self::new(stream, AmsAddr::default(), timeout)?;
        let source = device.port_connect()?;
        *device.inner.source.write()? = source;
        Ok(device)
    }

    /// Connects to an AMS router at `addr` using an explicitly provided
    /// source address, skipping the [`PortConnect`](PortConnectRequest) handshake.
    ///
    /// Use this when a static route is configured on the PLC and the source address
    /// must exactly match the configured route.
    pub fn connect_with_source(
        addr: impl ToSocketAddrs,
        source: AmsAddr,
        timeout: Option<Duration>,
    ) -> crate::Result<Self> {
        let stream = AmsStream::connect(addr)?;
        Self::new(stream, source, timeout)
    }

    /// Creates an [`AdsDevice`] from an existing [`AmsStream`].
    ///
    /// Unlike [`connect`](Self::connect) and [`connect_to`](Self::connect_to), this
    /// constructor does **not** perform a [`PortConnect`] handshake. The caller is
    /// responsible for providing a valid `source` address.
    ///
    /// This is intended for power users who need control over the underlying stream,
    /// for example to use a custom transport, inject test streams, or reuse an
    /// existing connection.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use tcads_core::io::blocking::AmsStream;
    /// use tcads_client::devices::blocking::AdsDevice;
    ///
    /// let stream = AmsStream::connect("192.168.1.100:48898")?;
    /// let source = "192.168.1.100.1.1:851".parse()?;
    /// let device = AdsDevice::new(stream, source, None)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(
        stream: AmsStream,
        source: AmsAddr,
        timeout: Option<Duration>,
    ) -> crate::Result<Self> {
        let (reader, writer) = stream.try_split()?;
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

        Ok(Self {
            inner: Arc::new(AdsDeviceInner {
                ams_requests,
                ads_notifs,
                router_notifs,
                source: RwLock::new(source),
                invoke_id: AtomicU32::new(1),
                timeout,
            }),
        })
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

    /// Queries the router's local AMS Net ID.
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
        length: u32,
        trans_mode: AdsTransMode,
        max_delay: u32,
        cycle_time: u32,
    ) -> crate::Result<(Receiver<AdsNotificationSampleOwned>, NotificationHandle)> {
        let invoke_id = self.next_invoke_id();

        let rx = self.inner.ads_notifs.pre_register(invoke_id)?;

        let frame = AdsAddDeviceNotificationRequest::new(
            target,
            self.source()?,
            invoke_id,
            index_group,
            index_offset,
            length,
            trans_mode,
            max_delay,
            cycle_time,
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
