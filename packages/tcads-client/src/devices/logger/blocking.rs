use crate::devices::blocking::{AdsDevice, AdsSubsystem};
use crate::notif_guard::blocking::NotificationGuard;
use std::net::ToSocketAddrs;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::Duration;
use tcads_core::protocol::AdsLoggerWriteRequestOwned;
use tcads_core::{
    AdsNotificationAttrib, AdsNotificationSampleOwned, AdsTransMode, AmsAddr, AmsNetId, AmsPort,
    IndexGroup, IndexOffset, LogEntry, LogMessageType, NotificationHandle, WindowsFileTime,
};

/// An [`AdsDevice`] client for the TwinCAT system logger on ADS port `100`.
///
/// # Thread Safety
///
/// The [`AdsLogger`] device is [`Clone`], so all clones share the same underlying connection.
/// It is also [`Send`] + [`Sync`], so multiple tasks can receive log entries
/// concurrently. Clean-up only happens when the last clone is dropped.
#[derive(Clone)]
pub struct AdsLogger {
    device: AdsDevice,
    target: AmsAddr,
}

impl AdsLogger {
    /// Connects to the ADS logger (Port 100) of a specific `net_id` using the local AMS router.
    ///
    /// Use this when targeting a specific device on the same router that has a
    /// different AMS Net ID (e.g., a UMRT or a specific PLC instance connected to the
    /// local AMS Router).
    ///
    /// Performs a [`PortConnect`](tcads_core::protocol::PortConnectRequest) handshake
    /// to obtain a dynamically assigned source address from the local router.
    pub fn connect(net_id: AmsNetId, timeout: impl Into<Option<Duration>>) -> crate::Result<Self> {
        let device = AdsDevice::connect(timeout)?;
        Ok(Self::new(device, net_id))
    }

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
    pub fn connect_local(timeout: impl Into<Option<Duration>>) -> crate::Result<Self> {
        let device = AdsDevice::connect(timeout)?;
        let net_id = device.get_local_net_id()?;
        Ok(Self::new(device, net_id))
    }

    /// Connects directly to the Ads logger (Port 100) of a remote AMS router without a local router.
    ///
    /// The `source` address must be pre-configured as a static route on the
    /// remote router. The `net_id` is the Net ID of the remote target.
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

    /// Creates an [`AdsLogger`] from an existing [`AdsDevice`] and router Net ID.
    ///
    /// Use this when sharing an existing connection with other device clients.
    /// `net_id` is used to construct the logger target address (`net_id:100`).
    pub fn new(device: AdsDevice, net_id: AmsNetId) -> Self {
        Self {
            device,
            target: AmsAddr::new(net_id, AmsPort::LOGGER),
        }
    }

    /// Shuts down the underlying connection.
    ///
    /// See [`AdsDevice::shutdown`] for more details.
    pub fn shutdown(&self) -> crate::Result<()> {
        self.device.shutdown()
    }

    /// Writes a log message to the TwinCAT logger using the current system time.
    ///
    /// `task_name` is encoded as Windows-1252 and truncated to
    /// [`MAX_TASK_NAME_LEN`](LogEntry::MAX_TASK_NAME_LEN) - 1 characters if it exceeds
    /// that limit.
    pub fn write_log<A: AsRef<str>>(
        &self,
        message_type: LogMessageType,
        task_name: A,
        message: A,
    ) -> crate::Result<()> {
        self.write_entry(LogEntry::new(
            WindowsFileTime::now(),
            message_type,
            self.device.source().port(),
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
    /// [`MAX_TASK_NAME_LEN`](LogEntry::MAX_TASK_NAME_LEN) - 1 characters if it exceeds
    /// that limit.
    pub fn write_entry(&self, entry: LogEntry) -> crate::Result<()> {
        let frame = AdsLoggerWriteRequestOwned::new(
            self.target(),
            self.device.source(),
            entry.timestamp(),
            entry.message_type(),
            entry.sender(),
            entry.message(),
            "",
        )
        .into();
        unsafe { self.device.write_frame_only(frame) }
    }

    // Subscribes to logger notifications.
    ///
    /// Returns a [`LogEntryReceiver`] that yields decoded [`LogEntry`] values.
    /// The subscription is cancelled automatically when the [`LogEntryReceiver`]
    /// is dropped, or explicitly via [`unsubscribe`](Self::unsubscribe).
    ///
    /// Multiple subscriptions can be active simultaneously.
    pub fn subscribe(&self) -> crate::Result<(LogEntryReceiver, NotificationHandle)> {
        let (rx, handle) = self.device.add_notification(
            self.target,
            IndexGroup::SYSTEM_LOGGER,
            IndexOffset::SYSTEM_LOGGER,
            AdsNotificationAttrib::new(
                LogEntry::MAX_PAYLOAD_LEN,
                AdsTransMode::ServerCycle,
                Duration::ZERO,
                Duration::ZERO,
            ),
        )?;

        let guard = NotificationGuard::new(handle, self.target, self.device.clone());

        Ok((LogEntryReceiver::new(rx, guard), handle))
    }

    /// Explicitly cancels a subscription by handle.
    ///
    /// The [`LogEntryReceiver`] associated with this handle will return
    /// [`Err(Error::Disconnected)`](crate::Error::Disconnected) on its next call.
    pub fn unsubscribe(&self, handle: NotificationHandle) -> crate::Result<()> {
        self.device.delete_notification(self.target, handle)
    }
}

/// A receiver for decoded [`LogEntry`] values.
///
/// Wraps the raw ADS notification channel and decodes each sample on demand.
/// The subscription is cancelled automatically when this is dropped.
///
/// Obtain one by calling [`AdsLogger::subscribe`].
pub struct LogEntryReceiver {
    rx: Receiver<AdsNotificationSampleOwned>,
    guard: NotificationGuard,
}

impl LogEntryReceiver {
    /// Creates a new instance of the [`LogEntryReceiver`].
    pub fn new(rx: Receiver<AdsNotificationSampleOwned>, guard: NotificationGuard) -> Self {
        Self { rx, guard }
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
        Ok(LogEntry::try_from(sample.data())?)
    }

    /// Blocks until the next log entry arrives or `timeout` elapses.
    ///
    /// Returns [`Err(Error::Timeout)`](crate::Error::Timeout) if the timeout expires,
    /// or [`Err(Error::Disconnected)`](crate::Error::Disconnected) if the subscription
    /// is cancelled or the connection is lost.
    pub fn recv_timeout(&self, timeout: Duration) -> crate::Result<LogEntry> {
        let sample = self.rx.recv_timeout(timeout)?;
        Ok(LogEntry::try_from(sample.data())?)
    }

    /// Returns the next log entry if one is immediately available, without blocking.
    ///
    /// Returns `Ok(None)` if no sample is currently available,
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

impl AdsSubsystem for AdsLogger {
    fn device(&self) -> &AdsDevice {
        &self.device
    }

    fn target(&self) -> AmsAddr {
        self.target
    }
}
