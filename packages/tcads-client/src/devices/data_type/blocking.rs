use super::ADSIGRP_SYM_DT_INFOBYNAME;
use crate::devices::blocking::AdsDevice;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use std::time::Duration;
use tcads_core::{AmsAddr, AmsPort};

pub struct DataTypeDeviceInner {
    pub device: AdsDevice,
    pub target: AmsAddr,
}

/// An ADS device client for fetching data type info from a PLC runtime.
///
/// # Ports
///
/// Any PLC runtime port that exposes the symbol/type interface:
/// - Ports 801–899: PLC runtimes (851 is usually the default first runtime for TC3)
/// - Ports 301–399: FreeTasks
#[derive(Clone)]
pub struct DataTypeDevice {
    inner: Arc<DataTypeDeviceInner>,
}

impl DataTypeDevice {
    /// Connects to the local AMS router at `127.0.0.1:48898`.
    ///
    /// The `port` is the [`AmsPort`] of the run-time/free-task ADS Device you wish to interact with.
    ///
    /// See [`AdsDevice::connect`] for more details.
    pub fn connect(port: AmsPort, timeout: impl Into<Option<Duration>>) -> crate::Result<Self> {
        Self::connect_to("127.0.0.1:48898", port, timeout)
    }

    /// Connects to an AMS router at `addr`.
    ///
    /// Performs a [`PortConnect`](tcads_core::protocol::PortConnectRequest) handshake
    /// to obtain a dynamically assigned source address.
    ///
    /// The `port` is the [`AmsPort`] of the run-time/free-task ADS Device you wish to interact with.
    pub fn connect_to(
        addr: impl ToSocketAddrs,
        port: AmsPort,
        timeout: impl Into<Option<Duration>>,
    ) -> crate::Result<Self> {
        let device = AdsDevice::connect_to(addr, timeout)?;
        let net_id = device.get_local_net_id()?;
        Ok(Self::new(device, (net_id, port).into()))
    }

    /// Connects directly to a remote AMS router without a local router.
    ///
    /// The `source` address must be pre-configured as a static route on the
    /// remote router. The `target` address is the address of the run-time/free-task ADS
    /// Device you wish to interact with.
    ///
    /// See [`AdsDevice::connect_remote`] for more details.
    pub fn connect_remote(
        addr: impl ToSocketAddrs,
        source: AmsAddr,
        target: AmsAddr,
        timeout: impl Into<Option<Duration>>,
    ) -> crate::Result<Self> {
        let device = AdsDevice::connect_remote(addr, source, timeout)?;
        Ok(Self::new(device, target))
    }

    /// Creates a `DataTypeDevice` from an existing [`AdsDevice`] and target address.
    ///
    /// Use this when sharing a connection with other device clients.
    pub fn new(device: AdsDevice, target: AmsAddr) -> Self {
        Self {
            inner: Arc::new(DataTypeDeviceInner { device, target }),
        }
    }

    /// Shuts down the underlying connection.
    ///
    /// See [`AdsDevice::shutdown`] for more details.
    pub fn shutdown(&self) -> crate::Result<()> {
        self.inner.device.shutdown()
    }

    /// Returns the target AMS Address.
    pub fn target(&self) -> AmsAddr {
        self.inner.target
    }

    /// Returns a reference to the underlying [`AdsDevice`].
    pub fn get_ref(&self) -> &AdsDevice {
        &self.inner.device
    }

    /// Returns data type info (current raw bytes until I work out the format)
    pub fn get_data_type_info(&self, name: &str) -> crate::Result<Vec<u8>> {
        self.inner.device.read_write(
            self.inner.target,
            ADSIGRP_SYM_DT_INFOBYNAME,
            0,
            1024,
            name.as_bytes(),
        )
    }
}
