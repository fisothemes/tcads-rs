use crate::devices::blocking::{AdsDevice, AdsSubsystem};
use std::time::Duration;
use tcads_core::{AdsState, AmsAddr, AmsNetId, AmsPort};

/// Interacts with the TwinCAT System Service (Port 10000).
///
/// Provides host OS control, CPU core isolation, remote file access,
/// process execution, and Windows Registry management.
pub struct AdsSystemService {
    device: AdsDevice,
    target: AmsAddr,
}

impl AdsSystemService {
    /// Connects to the TwinCAT System Service (Port 10000) of an ADS device with the specified
    /// `net_id` using the local AMS router.
    ///
    /// Use this when targeting a specific device on the same router that has a different AMS Net ID
    /// (e.g. a UMRT or specific PLC/IPC instance connected to the local AMS router).
    ///
    /// Performs a [`PortConnect`](tcads_core::protocol::PortConnectRequest) handshake
    /// to obtain a dynamically assigned source address from the local router.
    /// See [`AdsDevice::connect`] for more details.
    pub fn connect(net_id: AmsNetId, timeout: impl Into<Option<Duration>>) -> crate::Result<Self> {
        let device = AdsDevice::connect(timeout)?;
        Ok(Self::new(device, net_id))
    }

    /// Shuts down the underlying connection.
    ///
    /// See [`AdsDevice::shutdown`] for more details.
    pub fn shutdown(&self) -> crate::Result<()> {
        self.device.shutdown()
    }

    /// Creates a TwinCAT System Service (Port 10000) ADS device given a Net ID.
    ///
    /// Use this when sharing an existing connection with other device clients.
    pub fn new(device: AdsDevice, net_id: AmsNetId) -> Self {
        Self {
            device,
            target: AmsAddr::new(net_id, AmsPort::SYSTEM_SERVICE),
        }
    }

    /// Sets the TwinCAT system to Config Mode.
    pub fn set_config_mode(&self) -> crate::Result<()> {
        self.write_control(AdsState::Reconfig, 0)
    }

    /// Sets the TwinCAT system to Run Mode.
    pub fn set_run_mode(&self) -> crate::Result<()> {
        self.write_control(AdsState::Reset, 0)
    }

    /// Instructs the ADS device's host operating system to shut down.
    pub fn shutdown_host_os(&self, timeout: Duration) -> crate::Result<()> {
        let timeout: u32 = timeout.as_secs() as u32;
        self.device
            .write_control(self.target, AdsState::Shutdown, 0, &timeout.to_le_bytes())
    }

    /// Instructs the ADS device's host operating system to restart.
    pub fn restart_host_os(&self, timeout: Duration) -> crate::Result<()> {
        let timeout: u32 = timeout.as_secs() as u32;
        self.device
            .write_control(self.target, AdsState::Shutdown, 1, &timeout.to_le_bytes())
    }
}

impl AdsSubsystem for AdsSystemService {
    fn device(&self) -> &AdsDevice {
        &self.device
    }

    fn target(&self) -> AmsAddr {
        self.target
    }
}
