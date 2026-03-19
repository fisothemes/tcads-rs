use super::LOGGER_PORT;
use crate::devices::blocking::AdsDevice;
use std::net::ToSocketAddrs;
use std::time::Duration;
use tcads_core::{AmsAddr, AmsNetId, NotificationHandle};

/// A client for the TwinCAT system logger on ADS port `100`.
pub struct Logger {
    device: AdsDevice,
    target: AmsAddr,
    handle: Option<NotificationHandle>,
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
            device,
            target: AmsAddr::new(net_id, LOGGER_PORT),
            handle: None,
        }
    }
}

impl Drop for Logger {
    fn drop(&mut self) {
        todo!()
    }
}
