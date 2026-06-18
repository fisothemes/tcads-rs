use crate::tasks::tokio::{
    AdsNotificationDispatcher, AmsRequestDispatchKey, AmsRequestDispatcher, AmsRequestWriter,
    AmsResponseReader, RouterNotificationDispatcher,
};
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use tcads_core::io::tokio::{AmsReader, AmsStream, AmsWriter};
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
    AdsDeviceVersion, AdsError, AdsHeader, AdsNotificationAttrib, AdsReturnCode, AdsState, AmsAddr,
    AmsCommand, AmsFrame, AmsNetId, DeviceState, IndexGroup, IndexOffset, InvokeId,
    NotificationHandle, RouterState, SumAddNotificationRequest, SumAddNotificationResponse,
    SumDeleteNotificationResponse, SumReadRequest, SumReadResponseOwned, SumReadWriteRequest,
    SumReadWriteResponseOwned, SumWriteRequest, SumWriteResponse,
};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::RwLock;
use tokio::sync::mpsc::UnboundedReceiver as Receiver;

/// Shared state for an [`AdsDevice`] connection.
///
/// Held behind an [`Arc`] so all [`AdsDevice`] clones share the same connection.
/// Exposed as `pub` for power users who need direct access to the underlying
/// dispatchers to build custom device abstractions on top of the
/// same connection without going through the [`AdsDevice`] API.
///
/// # Lifetime
///
/// The reader and writer tasks are tied to the lifetime of this struct.
/// When the last [`AdsDevice`] clone is dropped, `AdsDeviceInner` drops,
/// which drops [`AmsRequestDispatcher`] and its `write_tx`. The writer task
/// exits when `write_tx` is dropped, the TCP stream closes, and the reader
/// task exits on the next read returning EOF.
pub struct AdsDeviceInner {
    pub ams_requests: Arc<AmsRequestDispatcher>,
    pub ads_notifs: Arc<AdsNotificationDispatcher>,
    pub router_notifs: Arc<RouterNotificationDispatcher>,
    pub source: RwLock<AmsAddr>,
    pub invoke_id: AtomicU32,
    pub timeout: Option<Duration>,
}

/// An async ADS client for communicating with TwinCAT devices.
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
/// local router performs a `PortConnect` handshake and dynamically assigns
/// a source address to the client.
///
/// ```no_run
/// use tcads_client::devices::tokio::AdsDevice;
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Connect to the local router at 127.0.0.1:48898
/// let device = AdsDevice::connect(Duration::from_secs(5)).await?;
///
/// // Connect to a router at a specific address
/// let device = AdsDevice::connect_to("192.168.1.50:48898", Duration::from_secs(5)).await?;
/// # Ok(())
/// # }
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
/// use tcads_client::devices::tokio::AdsDevice;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let source = "192.168.1.10.1.1:32750".parse()?;
/// let device = AdsDevice::connect_remote("192.168.1.120:48898", source, None).await?;
/// # Ok(())
/// # }
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
/// The writer task exits when its sender is dropped, the reader task exits
/// when the TCP stream closes, and all pending callers receive
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
    /// use tcads_client::devices::tokio::AdsDevice;
    /// use std::time::Duration;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let device = AdsDevice::connect(Duration::from_secs(5)).await?;
    /// println!("Source: {}", device.source().await);
    /// device.shutdown().await;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect(timeout: impl Into<Option<Duration>>) -> crate::Result<Self> {
        Self::connect_to("127.0.0.1:48898", timeout).await
    }

    /// Connects to an AMS router at `addr`.
    ///
    /// Performs a [`PortConnect`](PortConnectRequest) handshake to obtain a
    /// dynamically assigned source address.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use tcads_client::devices::tokio::AdsDevice;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let device = AdsDevice::connect_to("192.168.1.50:48898", None).await?;
    /// println!("Source: {}", device.source().await);
    /// device.shutdown().await;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect_to(
        addr: impl ToSocketAddrs,
        timeout: impl Into<Option<Duration>>,
    ) -> crate::Result<Self> {
        let timeout = timeout.into();
        let addr: SocketAddr = addr.to_socket_addrs()?.next().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "could not resolve address",
            )
        })?;
        let (reader, writer) = match timeout {
            Some(duration) => AmsStream::connect_timeout(&addr, duration)
                .await?
                .into_split(),
            None => AmsStream::connect(addr).await?.into_split(),
        };
        let device = Self::new(reader, writer, AmsAddr::default(), timeout);
        let source = device.port_connect().await?;
        *device.inner.source.write().await = source;
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
    /// use tcads_client::devices::tokio::AdsDevice;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// // Source must match the static route entry on the remote router
    /// let source = "192.168.1.10.1.1:32750".parse()?;
    /// let device = AdsDevice::connect_remote("192.168.1.120:48898", source, None).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect_remote(
        addr: impl ToSocketAddrs,
        source: AmsAddr,
        timeout: impl Into<Option<Duration>>,
    ) -> crate::Result<Self> {
        let timeout = timeout.into();
        let addr: SocketAddr = addr.to_socket_addrs()?.next().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "could not resolve address",
            )
        })?;
        let (reader, writer) = match timeout {
            Some(duration) => AmsStream::connect_timeout(&addr, duration)
                .await?
                .into_split(),
            None => AmsStream::connect(addr).await?.into_split(),
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
    /// use tcads_core::io::tokio::{AmsReader, AmsWriter};
    /// use tcads_client::devices::tokio::AdsDevice;
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
        R: AsyncRead + Unpin + Send + 'static,
        W: AsyncWrite + Unpin + Send + 'static,
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
    /// task writes it and exits, closing the TCP write half. The router closes
    /// the connection, causing the reader task to hit EOF and clear all pending
    /// callers and notification subscribers.
    ///
    /// If the send fails (already disconnected) this returns silently meaning the
    /// connection is already gone.
    pub async fn shutdown(&self) {
        let port = self.source().await.port();
        let frame = PortCloseRequest::new(port).into_frame();
        let _ = self.inner.ams_requests.send_only(frame);
    }

    /// Returns the source [`AmsAddr`] currently assigned to this connection.
    pub async fn source(&self) -> AmsAddr {
        *self.inner.source.read().await
    }

    /// Queries the local AMS router's Net ID.
    ///
    /// # Warning
    ///
    /// This command is only supported by a local AMS router. Do not call this
    /// on a device created with [`connect_remote`](Self::connect_remote). The
    /// remote router does not support this command and will close the TCP
    /// connection, causing subsequent calls to return
    /// [`Error::Disconnected`](crate::Error::Disconnected).
    pub async fn get_local_net_id(&self) -> crate::Result<AmsNetId> {
        let frame = GetLocalNetIdRequest::into_frame();
        let mut rx = self
            .inner
            .ams_requests
            .dispatch(AmsRequestDispatchKey::GetLocalNetId, frame)
            .await?;
        let resp = GetLocalNetIdResponse::try_from(self.wait(&mut rx).await?)?;
        Ok(resp.net_id())
    }

    async fn port_connect(&self) -> crate::Result<AmsAddr> {
        let frame = PortConnectRequest::default().into_frame();
        let mut rx = self
            .inner
            .ams_requests
            .dispatch(AmsRequestDispatchKey::PortConnect, frame)
            .await?;
        let resp = PortConnectResponse::try_from(self.wait(&mut rx).await?)?;
        Ok(*resp.addr())
    }

    /// Subscribes to router state changes.
    ///
    /// Returns a [`Receiver`] that yields each [`RouterState`] transition.
    /// The receiver returns [`None`] when the connection is lost or all
    /// [`AdsDevice`] clones are dropped.
    pub async fn subscribe_router(&self) -> Receiver<RouterState> {
        self.inner.router_notifs.subscribe().await
    }

    /// Reads the device name and version from `target`.
    pub async fn read_device_info(
        &self,
        target: AmsAddr,
    ) -> crate::Result<(AdsDeviceVersion, String)> {
        let invoke_id = self.next_invoke_id();

        let frame =
            AdsReadDeviceInfoRequest::new(target, self.source().await, invoke_id).into_frame();
        let resp =
            AdsReadDeviceInfoResponse::try_from(self.send_and_wait(frame, invoke_id).await?)?;

        Self::check_result(resp.result())?;

        Ok((resp.version(), resp.device_name().into_owned()))
    }

    /// Reads the ADS and device state of `target`.
    pub async fn read_state(&self, target: AmsAddr) -> crate::Result<(AdsState, DeviceState)> {
        let invoke_id = self.next_invoke_id();

        let frame = AdsReadStateRequest::new(target, self.source().await, invoke_id).into_frame();
        let resp = AdsReadStateResponse::try_from(self.send_and_wait(frame, invoke_id).await?)?;

        Self::check_result(resp.result())?;

        Ok((resp.ads_state(), resp.device_state()))
    }

    /// Changes the ADS and device state of `target`.
    pub async fn write_control(
        &self,
        target: AmsAddr,
        ads_state: AdsState,
        device_state: DeviceState,
        data: impl Into<Vec<u8>>,
    ) -> crate::Result<()> {
        let invoke_id = self.next_invoke_id();

        let frame = AdsWriteControlRequestOwned::with_data(
            target,
            self.source().await,
            invoke_id,
            ads_state,
            device_state,
            data,
        )
        .into_frame();
        let resp = AdsWriteControlResponse::try_from(&self.send_and_wait(frame, invoke_id).await?)?;

        Self::check_result(resp.result())?;

        Ok(())
    }

    /// Reads `length` bytes from `target` at `index_group` and `index_offset`.
    pub async fn read(
        &self,
        target: AmsAddr,
        index_group: IndexGroup,
        index_offset: IndexOffset,
        length: u32,
    ) -> crate::Result<Vec<u8>> {
        let invoke_id = self.next_invoke_id();

        let frame = AdsReadRequest::new(
            target,
            self.source().await,
            invoke_id,
            index_group,
            index_offset,
            length,
        )
        .into_frame();
        let frame = self.send_and_wait(frame, invoke_id).await?;
        let resp = AdsReadResponse::try_from_frame(&frame)?;

        Self::check_result(resp.result())?;

        Ok(resp.data().to_vec())
    }

    /// Writes `data` to `target` at `index_group` and `index_offset`.
    pub async fn write(
        &self,
        target: AmsAddr,
        index_group: IndexGroup,
        index_offset: IndexOffset,
        data: impl Into<Vec<u8>>,
    ) -> crate::Result<()> {
        let invoke_id = self.next_invoke_id();

        let frame = AdsWriteRequestOwned::new(
            target,
            self.source().await,
            invoke_id,
            index_group,
            index_offset,
            data,
        )
        .into_frame();
        let resp = AdsWriteResponse::try_from(self.send_and_wait(frame, invoke_id).await?)?;

        Self::check_result(resp.result())?;

        Ok(())
    }

    /// Sends a combined read/write to `target` in a single round trip.
    ///
    /// Writes `write_data` then reads `read_length` bytes back.
    pub async fn read_write(
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
            self.source().await,
            invoke_id,
            index_group,
            index_offset,
            read_length,
            write_data,
        )
        .into_frame();
        let frame = self.send_and_wait(frame, invoke_id).await?;
        let resp = AdsReadWriteResponse::try_from_frame(&frame)?;

        Self::check_result(resp.result())?;

        Ok(resp.data().to_vec())
    }

    /// Registers a device notification on `target`.
    ///
    /// Returns a [`Receiver`] for incoming samples and the [`NotificationHandle`]
    /// assigned by the PLC.
    ///
    /// The receiver returns [`None`] after [`delete_notification`](Self::delete_notification)
    /// is called, or when the router transitions to [`RouterState::Stop`] or [`RouterState::Removed`].
    ///
    /// # Note
    ///
    /// The target device may fire an initial sample immediately upon registration.
    pub async fn add_notification(
        &self,
        target: AmsAddr,
        index_group: IndexGroup,
        index_offset: IndexOffset,
        notif_attr: AdsNotificationAttrib,
    ) -> crate::Result<(Receiver<AdsNotificationSampleOwned>, NotificationHandle)> {
        let invoke_id = self.next_invoke_id();

        let rx = self.inner.ads_notifs.pre_register(invoke_id).await;

        let frame = AdsAddDeviceNotificationRequest::new(
            target,
            self.source().await,
            invoke_id,
            index_group,
            index_offset,
            notif_attr,
        )
        .into_frame();
        let resp = AdsAddDeviceNotificationResponse::try_from(
            self.send_and_wait(frame, invoke_id).await?,
        )?;

        Self::check_result(resp.result())?;

        let handle = resp.handle();
        self.inner.ads_notifs.promote(invoke_id, handle).await;

        Ok((rx, handle))
    }

    /// Deletes a device notification on `target`.
    ///
    /// The receiver obtained from [`add_notification`](Self::add_notification)
    /// will return [`None`] on its next [`recv`](Receiver::recv) call.
    pub async fn delete_notification(
        &self,
        target: AmsAddr,
        handle: NotificationHandle,
    ) -> crate::Result<()> {
        let invoke_id = self.next_invoke_id();
        let frame =
            AdsDeleteDeviceNotificationRequest::new(target, self.source().await, invoke_id, handle)
                .into_frame();
        let resp = AdsDeleteDeviceNotificationResponse::try_from(
            self.send_and_wait(frame, invoke_id).await?,
        )?;
        Self::check_result(resp.result())?;
        self.inner.ads_notifs.remove(handle).await;
        Ok(())
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
    /// corruption, or unexpected state transitions that affect the entire PLC task
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
    pub async unsafe fn write_frame(&self, frame: AmsFrame) -> crate::Result<Receiver<AmsFrame>> {
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

        self.inner.ams_requests.dispatch(key, frame).await
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

    /// Generates the next invoke ID used for an ADS request.
    ///
    /// This method acts as an escape hatch for power users and library authors
    /// who need to build custom device abstractions
    pub fn next_invoke_id(&self) -> InvokeId {
        self.inner.invoke_id.fetch_add(1, Ordering::Relaxed)
    }

    async fn send_and_wait(&self, frame: AmsFrame, invoke_id: InvokeId) -> crate::Result<AmsFrame> {
        let mut rx = self
            .inner
            .ams_requests
            .dispatch(AmsRequestDispatchKey::AdsCommand(invoke_id), frame)
            .await?;
        self.wait(&mut rx).await
    }

    async fn wait(&self, rx: &mut Receiver<AmsFrame>) -> crate::Result<AmsFrame> {
        match self.inner.timeout {
            Some(duration) => tokio::time::timeout(duration, rx.recv())
                .await
                .map_err(|_| crate::Error::Timeout)?
                .ok_or(crate::Error::Disconnected),
            None => rx.recv().await.ok_or(crate::Error::Disconnected),
        }
    }

    fn check_result(code: AdsReturnCode) -> crate::Result<()> {
        match code {
            AdsReturnCode::Ok => Ok(()),
            code => Err(code.into()),
        }
    }

    /// Sends multiple Read ADS requests to the PLC in a single network transaction.
    ///
    /// Returns a [`SumReadResponseOwned`] which lazily parses the network buffer. Iterating over
    /// the response yields a `Result<&[u8], AdsReturnCode>` for each requested variable,
    /// guaranteeing zero-copy data extraction and safe alignment even if individual variables fail.
    pub async fn read_multi(
        &self,
        target: AmsAddr,
        requests: &[SumReadRequest],
    ) -> crate::Result<SumReadResponseOwned> {
        let n = requests.len() as u32;

        if n == 0 {
            return Ok(SumReadResponseOwned::new(vec![], requests));
        }

        let mut expected_data_len = 0;
        let mut buf = Vec::with_capacity(n as usize * SumReadRequest::LENGTH);

        for req in requests {
            req.write_to(&mut buf);
            expected_data_len += req.length();
        }

        let read_len = (n * 8) + expected_data_len;
        let resp = self
            .read_write(
                target,
                IndexGroup::SUM_READ_EX,
                IndexOffset::new(n),
                read_len,
                buf,
            )
            .await?;

        Ok(SumReadResponseOwned::new(resp, requests))
    }

    /// Sends multiple Write ADS requests to the PLC in a single network transaction.
    ///
    /// Iterating over the returned [`SumWriteResponse`] yields a `Result<(), AdsReturnCode>`
    /// for each variable, indicating whether the PLC successfully accepted the write payload.
    pub async fn write_multi(
        &self,
        target: AmsAddr,
        requests: &[SumWriteRequest<'_>],
    ) -> crate::Result<SumWriteResponse> {
        let n = requests.len();
        if n == 0 {
            return Ok(SumWriteResponse::empty());
        }

        let total_header_len = n * SumWriteRequest::HEADER_LENGTH;
        let total_data_len: usize = requests.iter().map(|r| r.data().len()).sum();

        let mut buf = Vec::with_capacity(total_header_len + total_data_len);
        buf.resize(total_header_len, 0);

        for (i, req) in requests.iter().enumerate() {
            let header = &mut buf
                [i * SumWriteRequest::HEADER_LENGTH..(i + 1) * SumWriteRequest::HEADER_LENGTH];
            header.copy_from_slice(&req.header_to_bytes());
            buf.extend_from_slice(req.data());
        }

        let resp = self
            .read_write(
                target,
                IndexGroup::SUM_WRITE,
                IndexOffset::new(n as u32),
                (n * AdsReturnCode::LENGTH) as u32,
                buf,
            )
            .await?;

        Ok(SumWriteResponse::new(resp).map_err(AdsError::from)?)
    }

    /// Sends an ADS read-write batch request to the PLC in a single network transaction.
    ///
    /// This is most commonly used to dynamically resolve multiple symbol names into
    /// handle integers using Index Group `0xF003` in a single round-trip.
    pub async fn read_write_multi(
        &self,
        target: AmsAddr,
        requests: &[SumReadWriteRequest<'_>],
    ) -> crate::Result<SumReadWriteResponseOwned> {
        let n = requests.len();
        if n == 0 {
            return Ok(SumReadWriteResponseOwned::new(vec![], requests));
        }

        let total_header_len = n * SumReadWriteRequest::HEADER_LENGTH;
        let mut expected_read_data_len = 0;
        let mut total_write_data_len = 0;

        for req in requests {
            expected_read_data_len += req.read_length() as usize;
            total_write_data_len += req.write_data().len();
        }

        let mut buf = Vec::with_capacity(total_header_len + total_write_data_len);
        buf.resize(total_header_len, 0);

        for (i, req) in requests.iter().enumerate() {
            let header = &mut buf[i * SumReadWriteRequest::HEADER_LENGTH
                ..(i + 1) * SumReadWriteRequest::HEADER_LENGTH];
            header.copy_from_slice(&req.header_to_bytes());
            buf.extend_from_slice(req.write_data());
        }

        let read_len = (n * 8) + expected_read_data_len;

        let resp = self
            .read_write(
                target,
                IndexGroup::SUM_READ_WRITE,
                IndexOffset::new(n as u32),
                read_len as u32,
                buf,
            )
            .await?;

        if resp.len() < n * 8 {
            return Err(crate::Error::InvalidPayload);
        }

        Ok(SumReadWriteResponseOwned::new(resp, requests))
    }

    /// Registers a batch of variable notifications with the PLC simultaneously.
    ///
    /// This method is highly optimized for concurrency. It synchronizes directly with the
    /// background network thread to guarantee that no data samples are lost, even if the PLC
    /// begins streaming data before the response is fully processed.
    ///
    /// # Returns
    ///
    /// A vector containing a `Result` for every request.
    /// * **Success:** Yields the assigned `NotificationHandle` and a dedicated `Receiver` channel for that specific variable's data stream.
    /// * **Failure:** Yields an `AdsReturnCode`. The internal channel is automatically dropped, preventing memory leaks.
    pub async fn add_multi_notification(
        &self,
        target: AmsAddr,
        requests: &[SumAddNotificationRequest],
    ) -> crate::Result<
        Vec<Result<(NotificationHandle, Receiver<AdsNotificationSampleOwned>), AdsReturnCode>>,
    > {
        let n = requests.len();
        if n == 0 {
            return Ok(vec![]);
        }

        let invoke_id = self.next_invoke_id();

        let receivers = self.inner.ads_notifs.pre_register_batch(invoke_id, n).await;

        let mut write_buf = Vec::with_capacity(n * SumAddNotificationRequest::LENGTH);
        for req in requests {
            req.write_to(&mut write_buf);
        }

        let expected_read_len = (n * 8) as u32;
        let frame = tcads_core::protocol::AdsReadWriteRequestOwned::new(
            target,
            self.source().await,
            invoke_id,
            IndexGroup::SUM_ADD_NOTIFICATION,
            IndexOffset::new(n as u32),
            expected_read_len,
            write_buf,
        )
        .into_frame();

        let mut rx = self
            .inner
            .ams_requests
            .dispatch(AmsRequestDispatchKey::AdsCommand(invoke_id), frame)
            .await?;

        let response_frame = match self.inner.timeout {
            Some(duration) => {
                let maybe_msg = tokio::time::timeout(duration, rx.recv())
                    .await
                    .map_err(|_| crate::Error::Timeout)?;
                maybe_msg.ok_or(crate::Error::Disconnected)?
            }
            None => rx.recv().await.ok_or(crate::Error::Disconnected)?,
        };

        let read_write_resp = AdsReadWriteResponse::try_from_frame(&response_frame)?;

        if read_write_resp.result() != AdsReturnCode::Ok {
            return Err(crate::Error::from(read_write_resp.result()));
        }

        let response = SumAddNotificationResponse::new(read_write_resp.data())
            .map_err(|e| crate::Error::from(AdsError::from(e)))?;

        let parsed_results: Vec<Result<NotificationHandle, AdsReturnCode>> =
            response.iter().collect();

        self.inner
            .ads_notifs
            .promote_batch(invoke_id, &parsed_results)
            .await?;

        let final_output = receivers
            .into_iter()
            .zip(parsed_results)
            .map(|(rx, res)| res.map(|handle| (handle, rx)))
            .collect();

        Ok(final_output)
    }

    /// Deletes a batch of variable notifications from the PLC simultaneously.
    ///
    /// This method safely synchronizes with the background network thread. If the PLC
    /// successfully deletes a handle, the local routing channel is immediately closed,
    /// allowing any listening threads to safely terminate.
    pub async fn delete_multi_notifications(
        &self,
        target: AmsAddr,
        handles: &[NotificationHandle],
    ) -> crate::Result<SumDeleteNotificationResponse> {
        let n = handles.len();
        if n == 0 {
            return Ok(SumDeleteNotificationResponse::empty());
        }

        let mut buf = Vec::with_capacity(n * 4);
        for handle in handles {
            buf.extend_from_slice(&handle.to_bytes());
        }

        let resp_bytes = self
            .read_write(
                target,
                IndexGroup::SUM_DELETE_NOTIFICATION,
                IndexOffset::new(n as u32),
                (n * 4) as u32,
                buf,
            )
            .await?;

        let resp = SumDeleteNotificationResponse::new(resp_bytes)
            .map_err(|e| crate::Error::from(AdsError::from(e)))?;

        for (i, result) in resp.iter().enumerate() {
            if result.is_ok() {
                self.inner.ads_notifs.remove(handles[i]).await;
            }
        }

        Ok(resp)
    }
}
