use super::{LOGGER_INDEX_GROUP, LOGGER_INDEX_OFFSET, LogEntry, LogMessageType};
use crate::devices::tokio::AdsDevice;
use crate::notif_guard::tokio::NotificationGuard;
use std::collections::HashSet;
use std::net::ToSocketAddrs;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tcads_core::protocol::AdsLoggerWriteRequestOwned;
use tcads_core::{
    AdsNotificationAttrib, AdsNotificationSampleOwned, AdsTransMode, AmsAddr, AmsNetId, AmsPort,
    NotificationHandle, WindowsFileTime,
};
use tokio::sync::mpsc::UnboundedReceiver as Receiver;
use tokio::sync::mpsc::error::TryRecvError;

/// Shared state of an async [`AdsDevice`] client for the TwinCAT system logger.
///
/// Held behind an [`Arc`] so all [`Logger`] clones share the same connection.
/// Exposed as `pub` for power users who need direct access to the underlying
/// dispatchers to build custom device abstractions on top of the
/// same connection without going through the [`Logger`] API.
///
/// # Lifetime
///
/// All [`LogEntryReceiver`]s hold an [`Arc`] clone of this inner state.
/// The underlying connection and handle tracking remain alive as long as
/// either a [`Logger`] clone or a [`LogEntryReceiver`] exists.
/// When the last reference is dropped, [`Drop`] fires and all active
/// notification handles are deleted from the router.
pub struct LoggerInner {
    pub device: AdsDevice,
    pub target: AmsAddr,
    pub handles: Mutex<HashSet<NotificationHandle>>,
}

impl Drop for LoggerInner {
    fn drop(&mut self) {
        let Ok(mut handles) = self.handles.lock() else {
            return;
        };

        if handles.is_empty() {
            return;
        }

        let device = self.device.clone();
        let target = self.target;
        let handles_to_delete: Vec<_> = handles.drain().collect();

        // Safely check for a Tokio runtime context before spawning the clean-up task
        if let Ok(rt) = tokio::runtime::Handle::try_current() {
            rt.spawn(async move {
                for handle in handles_to_delete {
                    let _ = device.delete_notification(target, handle).await;
                }
            });
        }
        // No runtime context, this is best-effort at best, router cleans up on connection close
        // anyway. 100% safe to ignore.
    }
}

/// An asynchronous [`AdsDevice`] client for the TwinCAT system logger on ADS port `100`.
///
/// # Thread Safety
///
/// `Logger` is [`Clone`], so all clones share the same underlying connection.
/// It is also [`Send`] + [`Sync`], so multiple tasks can write or receive log
/// entries concurrently. Clean-up only happens when the last clone is dropped.
#[derive(Clone)]
pub struct Logger {
    inner: Arc<LoggerInner>,
}

impl Logger {
    /// Connects to the Logger (Port 100) of the local AMS router at `127.0.0.1:48898`.
    ///
    /// Automatically discovers the local Net ID and targets `local_net_id:100`.
    ///
    /// Performs a [`PortConnect`](tcads_core::protocol::PortConnectRequest) handshake
    /// to obtain a dynamically assigned source address from the local router.
    ///
    /// # Note
    ///
    /// On Windows, connecting via `127.0.0.1` requires the `EnableAmsTcpLoopback`
    /// registry key to be set. This is enabled by default in TwinCAT 4024.5 and newer.
    pub async fn connect(timeout: impl Into<Option<Duration>>) -> crate::Result<Self> {
        let device = AdsDevice::connect(timeout).await?;
        let net_id = device.get_local_net_id().await?;
        Ok(Self::new(device, net_id))
    }

    /// Connects to the Logger (Port 100) of a specific `net_id` using the local AMS router.
    ///
    /// Use this when targeting a specific device on the same router that has a
    /// different AMS Net ID (e.g., a UMRT or a specific PLC instance connected to the
    /// local AMS Router).
    ///
    /// Performs a [`PortConnect`](tcads_core::protocol::PortConnectRequest) handshake
    /// to obtain a dynamically assigned source address from the local router
    pub async fn connect_to(
        net_id: AmsNetId,
        timeout: impl Into<Option<Duration>>,
    ) -> crate::Result<Self> {
        let device = AdsDevice::connect(timeout).await?;
        Ok(Self::new(device, net_id))
    }

    /// Connects directly to the Logger of a remote AMS router without a local router.
    ///
    /// The `source` address must be pre-configured as a static route on the
    /// remote router. The `net_id` is the Net ID of the remote target.
    ///
    /// The [`PortConnect`](tcads_core::protocol::PortConnectRequest) handshake
    /// is **not** performed. See [`AdsDevice::connect_remote`] for details.
    pub async fn connect_remote(
        addr: impl ToSocketAddrs,
        source: AmsAddr,
        net_id: AmsNetId,
        timeout: impl Into<Option<Duration>>,
    ) -> crate::Result<Self> {
        let device = AdsDevice::connect_remote(addr, source, timeout).await?;
        Ok(Self::new(device, net_id))
    }

    /// Creates a `Logger` from an existing [`AdsDevice`] and router Net ID.
    ///
    /// Use this when sharing an existing connection with other device clients.
    /// `net_id` is used to construct the logger target address (`net_id:100`).
    pub fn new(device: AdsDevice, net_id: AmsNetId) -> Self {
        Self {
            inner: Arc::new(LoggerInner {
                device,
                target: AmsAddr::new(net_id, AmsPort::LOGGER),
                handles: Mutex::new(HashSet::new()),
            }),
        }
    }

    /// Shuts down the underlying connection.
    ///
    /// See [`AdsDevice::shutdown`] for more details.
    pub async fn shutdown(&self) {
        self.inner.device.shutdown().await
    }

    /// Returns the target address of the logger.
    pub fn target(&self) -> AmsAddr {
        self.inner.target
    }

    /// Returns a reference to the underlying [`AdsDevice`].
    pub fn get_ref(&self) -> &AdsDevice {
        &self.inner.device
    }

    /// Writes a log message to the TwinCAT logger using the current system time.
    ///
    /// `task_name` is encoded as Windows-1252 and truncated to
    /// [`MAX_TASK_NAME_LEN`](super::MAX_TASK_NAME_LEN) - 1 characters if it exceeds
    /// that limit.
    pub async fn write_log<A: AsRef<str>>(
        &self,
        message_type: LogMessageType,
        task_name: A,
        message: A,
    ) -> crate::Result<()> {
        self.write_entry(LogEntry::new(
            WindowsFileTime::now(),
            message_type,
            self.get_ref().source().await.port(),
            task_name.as_ref().to_owned(),
            message.as_ref().to_owned(),
        ))
        .await
    }

    /// Writes a [`LogEntry`] to the TwinCAT logger.
    ///
    /// This replicates [`ADSLOGSTR`](https://infosys.beckhoff.com/content/1033/tcplclib_tc2_system/31033611.html)
    /// from Structured Text, sending an ADS Device Notification directly to port 100.
    ///
    /// Prefer [`write_log`](Self::write_log) for most use cases. Use this method
    /// when you need full control over the [`LogEntry`], such as providing an explicit
    /// timestamp via [`LogEntry::new`].
    ///
    /// `entry.sender` is encoded as Windows-1252 and truncated to
    /// [`MAX_TASK_NAME_LEN`](super::MAX_TASK_NAME_LEN) - 1 characters if it exceeds
    /// that limit.
    pub async fn write_entry(&self, entry: LogEntry) -> crate::Result<()> {
        let frame = AdsLoggerWriteRequestOwned::new(
            self.target(),
            self.get_ref().source().await,
            entry.timestamp(),
            entry.message_type(),
            entry.sender(),
            entry.message(),
            "",
        )
        .into();
        unsafe { self.get_ref().write_frame_only(frame) }
    }

    /// Subscribes asynchronously to TwinCAT logger notifications.
    ///
    /// Returns a [`LogEntryReceiver`] that yields decoded [`LogEntry`] values.
    /// The subscription is cancelled automatically when the [`LogEntryReceiver`] is dropped.
    ///
    /// Multiple subscriptions can be active simultaneously.
    pub async fn subscribe(&self) -> crate::Result<(LogEntryReceiver, NotificationHandle)> {
        let (rx, handle) = self
            .inner
            .device
            .add_notification(
                self.inner.target,
                LOGGER_INDEX_GROUP,
                LOGGER_INDEX_OFFSET,
                AdsNotificationAttrib::new(
                    LogEntry::MAX_PAYLOAD_LEN,
                    AdsTransMode::ServerCycle,
                    0,
                    0,
                ),
            )
            .await?;

        match self.inner.handles.lock() {
            Ok(mut handles) => {
                handles.insert(handle);
            }
            Err(e) => {
                let _ = self
                    .inner
                    .device
                    .delete_notification(self.inner.target, handle)
                    .await;
                return Err(e.into());
            }
        };

        let guard = NotificationGuard::new(handle, self.inner.target, self.inner.device.clone());

        Ok((
            LogEntryReceiver::new(rx, guard, Arc::clone(&self.inner)),
            handle,
        ))
    }

    /// Explicitly cancels a subscription by its handle.
    ///
    /// The [`LogEntryReceiver`] associated with this handle will return
    /// [`Err(Error::Disconnected)`](crate::Error::Disconnected) on its next call.
    ///
    /// Dropping the [`LogEntryReceiver`] has the same effect and is preferred
    /// in most cases.
    pub async fn unsubscribe(&self, handle: NotificationHandle) -> crate::Result<()> {
        if let Ok(mut handles) = self.inner.handles.lock() {
            handles.remove(&handle);
        }
        self.inner
            .device
            .delete_notification(self.inner.target, handle)
            .await
    }
}

/// An asynchronous receiver for decoded [`LogEntry`] values.
///
/// Wraps the raw ADS notification channel and decodes each sample on demand.
/// The subscription is cancelled automatically when this is dropped.
///
/// Obtain one by calling [`Logger::subscribe`].
pub struct LogEntryReceiver {
    rx: Receiver<AdsNotificationSampleOwned>,
    guard: NotificationGuard,
    inner: Arc<LoggerInner>,
}

impl LogEntryReceiver {
    /// Creates a new instance of the [`LogEntryReceiver`].
    pub fn new(
        rx: Receiver<AdsNotificationSampleOwned>,
        guard: NotificationGuard,
        inner: Arc<LoggerInner>,
    ) -> Self {
        Self { rx, guard, inner }
    }

    /// Returns the notification handle for this subscription.
    pub fn handle(&self) -> NotificationHandle {
        self.guard.handle()
    }

    /// Asynchronously awaits the next log entry.
    ///
    /// Returns [`Err(Error::Disconnected)`](crate::Error::Disconnected) when the
    /// subscription is cancelled or the connection is lost.
    ///
    /// # Note
    ///
    /// Unlike the blocking [`recv`](super::blocking::LogEntryReceiver::recv), this
    /// method requires `&mut self` because Tokio's [`Receiver`] does not support
    /// shared references.
    pub async fn recv(&mut self) -> crate::Result<LogEntry> {
        let sample = self.rx.recv().await.ok_or(crate::Error::Disconnected)?;
        Ok(LogEntry::try_from(sample.data())?)
    }

    /// Returns the next log entry if one is immediately available, without awaiting.
    ///
    /// Returns [`Ok(None)`] if no sample is currently available,
    /// or [`Err(Error::Disconnected)`](crate::Error::Disconnected) if the
    /// subscription is cancelled or the connection is lost.
    pub fn try_recv(&mut self) -> crate::Result<Option<LogEntry>> {
        match self.rx.try_recv() {
            Ok(sample) => Ok(Some(LogEntry::try_from(sample.data())?)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(crate::Error::Disconnected),
        }
    }
}

impl Drop for LogEntryReceiver {
    fn drop(&mut self) {
        let _ = self
            .inner
            .handles
            .lock()
            .map(|mut h| h.remove(&self.guard.handle()));
    }
}
