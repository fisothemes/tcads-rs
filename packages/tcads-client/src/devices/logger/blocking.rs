use super::log_entry::entry_to_frame;
use super::{
    LOGGER_DATA_LEN, LOGGER_INDEX_GROUP, LOGGER_INDEX_OFFSET, LOGGER_PORT, LogEntry, MessageType,
};
use crate::devices::blocking::AdsDevice;
use crate::notif_guard::blocking::NotificationGuard;
use std::collections::HashSet;
use std::net::ToSocketAddrs;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tcads_core::{
    AdsNotificationSampleOwned, AdsTransMode, AmsAddr, AmsNetId, NotificationHandle,
    WindowsFileTime,
};

/// Shared state of an [`AdsDevice`] client for the TwinCAT system logger.
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
        for handle in handles.drain() {
            let _ = self.device.delete_notification(self.target, handle);
        }
    }
}

/// An [`AdsDevice`] client for the TwinCAT system logger on ADS port `100`.
///
/// # Thread Safety
///
/// The `Logger` device is [`Clone`], so all clones share the same underlying connection.
/// It is also [`Send`] + [`Sync`], so multiple tasks can receive log entries
/// concurrently. Clean-up only happens when the last clone is dropped.
#[derive(Clone)]
pub struct Logger {
    inner: Arc<LoggerInner>,
}

impl Logger {
    /// Connects to the local AMS router at `127.0.0.1:48898`.
    ///
    /// # Note
    ///
    /// On Windows, connecting via `127.0.0.1` requires the
    /// `EnableAmsTcpLoopback` registry key to be set. This is enabled by
    /// default in TwinCAT 4024.5 and newer.
    pub fn connect(timeout: impl Into<Option<Duration>>) -> crate::Result<Self> {
        Self::connect_to("127.0.0.1:48898", timeout)
    }

    /// Connects to an AMS router at `addr`.
    ///
    /// Performs a [`PortConnect`](tcads_core::protocol::PortConnectRequest) handshake
    /// to obtain a dynamically assigned source address.
    pub fn connect_to(
        addr: impl ToSocketAddrs,
        timeout: impl Into<Option<Duration>>,
    ) -> crate::Result<Self> {
        let device = AdsDevice::connect_to(addr, timeout)?;
        let net_id = device.get_local_net_id()?;
        Ok(Self::new(device, net_id))
    }

    /// Connects directly to a remote AMS router without a local router.
    ///
    /// The `source` address must be pre-configured as a static route on the
    /// remote router. The `net_id` should be the Net ID of the remote router
    /// and is used to address the logger target (`net_id:100`).
    ///
    /// The [`PortConnect`](tcads_core::protocol::PortConnectRequest) handshake
    /// is **not** performed. See [`AdsDevice::connect_remote`] for details.
    pub fn connect_remote(
        addr: impl ToSocketAddrs,
        source: AmsAddr,
        net_id: AmsNetId,
        timeout: impl Into<Option<Duration>>,
    ) -> crate::Result<Self> {
        let device = AdsDevice::connect_remote(addr, source, timeout)?;
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
                target: AmsAddr::new(net_id, LOGGER_PORT),
                handles: Mutex::new(HashSet::new()),
            }),
        }
    }

    /// Shuts down the underlying connection.
    ///
    /// See [`AdsDevice::shutdown`] for more details.
    pub fn shutdown(&self) -> crate::Result<()> {
        self.inner.device.shutdown()
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
    pub fn write_log<A: AsRef<str>>(
        &self,
        message_type: MessageType,
        task_name: A,
        message: A,
    ) -> crate::Result<()> {
        self.write_entry(LogEntry::new(
            WindowsFileTime::now(),
            message_type,
            self.get_ref().source()?.port(),
            task_name.as_ref().to_owned(),
            message.as_ref().to_owned(),
        ))
    }

    /// Writes a log message to the TwinCAT logger.
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
    pub fn write_entry(&self, entry: LogEntry) -> crate::Result<()> {
        let frame = entry_to_frame(self.target(), self.get_ref().source()?, entry);
        unsafe { self.get_ref().write_frame_only(frame) }
    }

    // Subscribes to logger notifications.
    ///
    /// Returns a [`LogEntryReceiver`] that yields decoded [`LogEntry`] values.
    /// The subscription is cancelled automatically when the [`LogEntryReceiver`]
    /// is dropped, or explicitly via [`unsubscribe`](Self::unsubscribe).
    ///
    /// Multiple subscriptions can be active simultaneously, and each returns an
    /// independent [`LogEntryReceiver`].
    pub fn subscribe(&self) -> crate::Result<(LogEntryReceiver, NotificationHandle)> {
        let (rx, handle) = self.inner.device.add_notification(
            self.inner.target,
            LOGGER_INDEX_GROUP,
            LOGGER_INDEX_OFFSET,
            LOGGER_DATA_LEN,
            AdsTransMode::ServerCycle,
            0,
            0,
        )?;
        match self.inner.handles.lock() {
            Ok(mut handles) => handles.insert(handle),
            Err(e) => {
                let _ = self
                    .inner
                    .device
                    .delete_notification(self.inner.target, handle);
                return Err(e.into());
            }
        };

        let guard = NotificationGuard::new(handle, self.inner.target, self.inner.device.clone());

        Ok((
            LogEntryReceiver::new(rx, guard, Arc::clone(&self.inner)),
            handle,
        ))
    }

    /// Explicitly cancels a subscription by handle.
    ///
    /// The [`LogEntryReceiver`] associated with this handle will return
    /// [`Err(Error::Disconnected)`](Error::Disconnected) on its next call.
    ///
    /// Dropping the [`LogEntryReceiver`] has the same effect and is preferred
    /// in most cases.
    pub fn unsubscribe(&self, handle: NotificationHandle) -> crate::Result<()> {
        self.inner.handles.lock()?.remove(&handle);
        self.inner
            .device
            .delete_notification(self.inner.target, handle)
    }
}

/// A receiver for decoded [`LogEntry`] values.
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

    /// Blocks until the next log entry arrives.
    ///
    /// Returns [`Err`] when the subscription is cancelled or the connection
    /// is lost.
    pub fn recv(&self) -> crate::Result<LogEntry> {
        let sample = self.rx.recv()?;
        LogEntry::try_from(sample.data())
    }

    /// Blocks until the next log entry arrives or `timeout` elapses.
    ///
    /// Returns [`Err(Error::Timeout)`](crate::Error::Timeout) if the timeout expires,
    /// or [`Err(Error::Disconnected)`](crate::Error::Disconnected) if the subscription
    /// is cancelled or the connection is lost.
    pub fn recv_timeout(&self, timeout: Duration) -> crate::Result<LogEntry> {
        let sample = self.rx.recv_timeout(timeout)?;
        LogEntry::try_from(sample.data())
    }

    /// Returns the next log entry if one is immediately available, without blocking.
    ///
    /// Returns [`Ok(None)`] if no sample is currently available,
    /// or [`Err(Error::Disconnected)`](crate::Error::Disconnected) if the
    /// subscription is cancelled or the connection is lost.
    pub fn try_recv(&self) -> crate::Result<Option<LogEntry>> {
        match self.rx.try_recv() {
            Ok(sample) => Ok(Some(LogEntry::try_from(sample.data())?)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(crate::Error::Disconnected),
        }
    }

    /// Returns an iterator that blocks on each call, yielding log entries
    /// until the subscription is cancelled or the connection is lost.
    pub fn iter(&self) -> impl Iterator<Item = crate::Result<LogEntry>> + '_ {
        std::iter::from_fn(move || match self.recv() {
            Err(crate::Error::Disconnected) => None,
            result => Some(result),
        })
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
