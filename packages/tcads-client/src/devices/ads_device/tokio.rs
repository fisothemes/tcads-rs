use crate::tasks::tokio::{
    AdsNotificationDispatcher, AmsRequestDispatchKey, AmsRequestDispatcher, AmsRequestWriter,
    AmsResponseReader, RouterNotificationDispatcher,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use tcads_core::io::tokio::AmsStream;
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
use tokio::net::ToSocketAddrs;
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
/// `AdsDevice` is [`Clone`], so all clones share the same underlying connection.
/// It is also [`Send`] + [`Sync`], so multiple tasks can issue ADS commands
/// concurrently. Responses are matched to their callers via Invoke ID with no
/// global lock on the connection.
///
/// # Example
///
/// ```no_run
/// use std::time::Duration;
/// use tcads_client::devices::tokio::AdsDevice;
/// use tcads_client::{AdsState, AdsTransMode, AmsAddr};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let device = AdsDevice::connect(Some(Duration::from_secs(5))).await?;
///     let target = AmsAddr::new(device.get_local_net_id().await?, 851);
///
///     let (ads_state, _) = device.read_state(target).await?;
///     println!("PLC state: {ads_state:?}");
///
///     let (mut rx, handle) = device.add_notification(
///         target, 0xF005, 0, 4, AdsTransMode::ServerOnChange, 0, 10,
///     ).await?;
///
///     if let Some(sample) = rx.recv().await {
///         println!("Sample: {:?}", sample.data());
///     }
///
///     device.delete_notification(target, handle).await?;
///     device.shutdown().await;
///     Ok(())
/// }
/// ```
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
    /// # Example
    ///
    /// ```no_run
    /// use tcads_client::devices::tokio::AdsDevice;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let device = AdsDevice::connect(None).await?;
    ///
    /// println!("Source: {}", device.source().await);
    /// println!("Local Net ID: {}", device.get_local_net_id().await?);
    ///
    /// device.shutdown().await;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect(timeout: Option<Duration>) -> crate::Result<Self> {
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
    /// let device = AdsDevice::connect_to("192.168.1.100:48898", None).await?;
    ///
    /// println!("Source: {}", device.source().await);
    /// println!("Local Net ID: {}", device.get_local_net_id().await?);
    ///
    /// device.shutdown().await;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect_to(
        addr: impl ToSocketAddrs,
        timeout: Option<Duration>,
    ) -> crate::Result<Self> {
        let stream = AmsStream::connect(addr).await?;
        let device = Self::new(stream, AmsAddr::default(), timeout);
        let source = device.port_connect().await?;
        *device.inner.source.write().await = source;
        Ok(device)
    }

    /// Connects to an AMS router at `addr` using an explicitly provided source
    /// address, skipping the [`PortConnect`](PortConnectRequest) handshake.
    ///
    /// Use this when a static route is configured on the PLC and the source
    /// address must exactly match the configured route.
    pub async fn connect_with_source(
        addr: impl ToSocketAddrs,
        source: AmsAddr,
        timeout: Option<Duration>,
    ) -> crate::Result<Self> {
        let stream = AmsStream::connect(addr).await?;
        Ok(Self::new(stream, source, timeout))
    }

    /// Creates an [`AdsDevice`] from an existing [`AmsStream`].
    ///
    /// Unlike [`connect`](Self::connect) and [`connect_to`](Self::connect_to), this
    /// constructor does **not** perform a [`PortConnect`] handshake. The caller is
    /// responsible for providing a valid `source` address.
    ///
    /// This is intended for power users who need control over the underlying stream,
    /// for example, to use a custom transport, inject test streams, or reuse an
    /// existing connection.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use tcads_core::io::tokio::AmsStream;
    /// use tcads_client::devices::tokio::AdsDevice;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let stream = AmsStream::connect("192.168.1.100:48898").await?;
    /// let source = "192.168.1.100.1.1:34000".parse()?;
    /// let device = AdsDevice::new(stream, source, None);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(stream: AmsStream, source: AmsAddr, timeout: Option<Duration>) -> Self {
        let (reader, writer) = stream.into_split();
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
                timeout,
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
    /// If the send fails (already disconnected) this returns `Ok(())` meaning the
    /// connection is already gone.
    pub async fn shutdown(&self) {
        let port = self.source().await.port();
        let frame = PortCloseRequest::new(port).into_frame();
        let _ = self.inner.ams_requests.send_only(frame).await;
    }

    /// Returns the source [`AmsAddr`] currently assigned to this connection.
    pub async fn source(&self) -> AmsAddr {
        *self.inner.source.read().await
    }

    /// Queries the router's local AMS Net ID.
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
    #[allow(clippy::too_many_arguments)]
    pub async fn add_notification(
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

        let rx = self.inner.ads_notifs.pre_register(invoke_id).await;

        let frame = AdsAddDeviceNotificationRequest::new(
            target,
            self.source().await,
            invoke_id,
            index_group,
            index_offset,
            length,
            trans_mode,
            max_delay,
            cycle_time,
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
