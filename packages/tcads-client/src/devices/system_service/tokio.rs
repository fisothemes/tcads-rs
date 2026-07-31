use crate::devices::tokio::{AdsDevice, AdsSubsystem};
use std::time::Duration;
use tcads_core::{AdsState, AmsAddr, AmsNetId, AmsPort, IndexGroup, IndexOffset};

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
    pub async fn connect(
        net_id: AmsNetId,
        timeout: impl Into<Option<Duration>>,
    ) -> crate::Result<Self> {
        let device = AdsDevice::connect(timeout).await?;
        Ok(Self::new(device, net_id))
    }

    /// Connects to the TwinCAT System Service (Port 10000) of an ADS device using the local
    /// AMS router at `127.0.0.1:48898` and the Net ID obtained using [`AdsDevice::get_local_net_id`].
    ///
    /// See [`AdsDevice::connect`] for more details.
    pub async fn connect_local(timeout: impl Into<Option<Duration>>) -> crate::Result<Self> {
        let device = AdsDevice::connect(timeout).await?;
        let net_id = device.get_local_net_id().await?;
        Ok(Self::new(device, net_id))
    }

    /// Connects to the TwinCAT System Service (Port 10000) of an ADS device on a remote AMS router.
    ///
    /// The `source` address must be pre-configured as a static route on the remote router.
    /// The `net_id` is the Net ID of the remote target.
    ///
    /// The [`PortConnect`](tcads_core::protocol::PortConnectRequest) handshake
    /// is **not** performed. See [`AdsDevice::connect_remote`] for details.
    pub async fn connect_remote(
        addr: impl std::net::ToSocketAddrs,
        source: AmsAddr,
        net_id: AmsNetId,
        timeout: impl Into<Option<Duration>>,
    ) -> crate::Result<Self> {
        let device = AdsDevice::connect_remote(addr, source, timeout).await?;
        Ok(Self::new(device, net_id))
    }

    /// Shuts down the underlying connection.
    ///
    /// See [`AdsDevice::shutdown`] for more details.
    pub async fn shutdown(&self) {
        self.device.shutdown().await
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
    pub async fn set_config_mode(&self) -> crate::Result<()> {
        self.write_control(AdsState::Reconfig, 0).await
    }

    /// Sets the TwinCAT system to Run Mode.
    pub async fn set_run_mode(&self) -> crate::Result<()> {
        self.write_control(AdsState::Reset, 0).await
    }

    /// Instructs the ADS device's host operating system to shut down.
    pub async fn shutdown_host_os(&self, timeout: Duration) -> crate::Result<()> {
        let timeout: u32 = timeout.as_secs() as u32;
        self.device
            .write_control(self.target, AdsState::Shutdown, 0, timeout.to_le_bytes())
            .await
    }

    /// Instructs the ADS device's host operating system to restart.
    pub async fn restart_host_os(&self, timeout: Duration) -> crate::Result<()> {
        let timeout: u32 = timeout.as_secs() as u32;
        self.device
            .write_control(self.target, AdsState::Shutdown, 1, timeout.to_le_bytes())
            .await
    }

    /// Instructs the ADS device's host operating system to abort the shutdown process.
    pub async fn abort_host_os_shutdown(&self) -> crate::Result<()> {
        self.device
            .write_control(self.target, AdsState::PowerGood, 0, [])
            .await
    }

    /// Configures the number of shared CPU cores on the target device's host machine.
    pub async fn set_host_shared_cores(&self, shared_cores: u32) -> crate::Result<()> {
        self.device
            .read_write(
                self.target,
                IndexGroup::SYSTEM_SERVICE_SET_NUM_PROC,
                IndexOffset::ZERO,
                0,
                shared_cores.to_le_bytes(),
            )
            .await?;
        Ok(())
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
