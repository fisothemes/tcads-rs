use super::{DATATYPE_INFO_BY_NAME_INDEX_GROUP, SYMBOL_UPLOAD_INFO_INDEX_GROUP};
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
    /// Connects to the local target ADS device at a given `port` using the local
    /// AMS router at `127.0.0.1:48898`.
    ///
    /// The `port` is the [`AmsPort`] of the run-time/free-task ADS Device you wish to interact
    /// with based on the target's AMS address.
    ///
    /// See [`AdsDevice::connect`] for more details.
    pub fn connect(port: AmsPort, timeout: impl Into<Option<Duration>>) -> crate::Result<Self> {
        let device = AdsDevice::connect_to("127.0.0.1:48898", timeout)?;
        let target = AmsAddr::new(device.get_local_net_id()?, port);
        Ok(Self::new(device, target))
    }

    /// Connects to the `target` ADS device using the local AMS router at `127.0.0.1:48898`.
    ///
    /// The `target` is the [`AmsAddr`] of the run-time/free-task ADS Device you wish to interact with.
    pub fn connect_to(
        target: AmsAddr,
        timeout: impl Into<Option<Duration>>,
    ) -> crate::Result<Self> {
        let device = AdsDevice::connect_to("127.0.0.1:48898", timeout)?;
        Ok(Self::new(device, target))
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
        let length_bytes = self.inner.device.read_write(
            self.inner.target,
            DATATYPE_INFO_BY_NAME_INDEX_GROUP,
            0,
            4,
            name,
        )?;

        let entry_length = u32::from_le_bytes(
            length_bytes
                .try_into()
                .map_err(|_| crate::Error::InvalidPayload)?,
        );

        self.inner.device.read_write(
            self.inner.target,
            DATATYPE_INFO_BY_NAME_INDEX_GROUP,
            0,
            entry_length,
            name,
        )
    }
}
