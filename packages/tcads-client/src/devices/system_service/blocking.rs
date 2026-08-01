use super::shared;
use crate::devices::blocking::{AdsDevice, AdsSubsystem};
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
    pub fn connect(net_id: AmsNetId, timeout: impl Into<Option<Duration>>) -> crate::Result<Self> {
        let device = AdsDevice::connect(timeout)?;
        Ok(Self::new(device, net_id))
    }

    /// Connects to the TwinCAT System Service (Port 10000) of an ADS device using the local
    /// AMS router at `127.0.0.1:48898` and the Net ID obtained using [`AdsDevice::get_local_net_id`].
    ///
    /// See [`AdsDevice::connect`] for more details.
    pub fn connect_local(timeout: impl Into<Option<Duration>>) -> crate::Result<Self> {
        let device = AdsDevice::connect(timeout)?;
        let net_id = device.get_local_net_id()?;
        Ok(Self::new(device, net_id))
    }

    /// Connects to the TwinCAT System Service (Port 10000) of an ADS device on a remote AMS router.
    ///
    /// The `source` address must be pre-configured as a static route on the remote router.
    /// The `net_id` is the Net ID of the remote target.
    ///
    /// The [`PortConnect`](tcads_core::protocol::PortConnectRequest) handshake
    /// is **not** performed. See [`AdsDevice::connect_remote`] for details.
    pub fn connect_remote(
        addr: impl std::net::ToSocketAddrs,
        source: AmsAddr,
        net_id: AmsNetId,
        timeout: impl Into<Option<Duration>>,
    ) -> crate::Result<Self> {
        let device = AdsDevice::connect_remote(addr, source, timeout)?;
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
            .write_control(self.target, AdsState::Shutdown, 0, timeout.to_le_bytes())
    }

    /// Instructs the ADS device's host operating system to restart.
    pub fn restart_host_os(&self, timeout: Duration) -> crate::Result<()> {
        let timeout: u32 = timeout.as_secs() as u32;
        self.device
            .write_control(self.target, AdsState::Shutdown, 1, timeout.to_le_bytes())
    }

    /// Instructs the ADS device's host operating system to abort the shutdown process.
    pub fn abort_host_os_shutdown(&self) -> crate::Result<()> {
        self.device
            .write_control(self.target, AdsState::PowerGood, 0, [])
    }

    /// Configures the number of shared CPU cores on the target device's host machine.
    pub fn set_host_shared_cores(&self, shared_cores: u32) -> crate::Result<()> {
        self.device.read_write(
            self.target,
            IndexGroup::SYSTEM_SERVICE_SET_NUM_PROC,
            IndexOffset::ZERO,
            0,
            shared_cores.to_le_bytes(),
        )?;
        Ok(())
    }

    /// Starts a new process on the host machine of the ADS device.
    ///
    /// `dir` is the working directory of the process. `args` are the command-line arguments passed
    /// to it. `is_hidden` indicates whether the process starts without a user interface.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tcads_client::devices::blocking::AdsSystemService;
    /// # fn main() -> tcads_client::Result<()> {
    /// # let device = AdsSystemService::connect_local(None)?;
    /// device.start_host_process(
    ///     "C:/Windows/Notepad.exe",
    ///     "C:/TwinCAT/3.1/Target",
    ///     "StaticRoutes.xml",
    ///     false
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn start_host_process(
        &self,
        app: impl AsRef<str>,
        dir: impl AsRef<str>,
        args: impl AsRef<str>,
        is_hidden: bool,
    ) -> crate::Result<()> {
        let app = app.as_ref();
        let dir = dir.as_ref();
        let args = args.as_ref();

        let data = shared::build_start_host_process_request(app, dir, args)?;

        let offset = if is_hidden {
            IndexOffset::SYSTEM_SERVICE_START_PROCESS_HIDDEN
        } else {
            IndexOffset::ZERO
        };

        self.device.write(
            self.target,
            IndexGroup::SYSTEM_SERVICE_START_PROCESS,
            offset,
            data,
        )
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
