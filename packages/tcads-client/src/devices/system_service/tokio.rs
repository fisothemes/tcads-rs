use super::shared;
use crate::devices::tokio::{AdsDevice, AdsSubsystem};
use chrono::{DateTime, NaiveDateTime, Utc};
use std::time::Duration;
use tcads_core::{
    AdsError, AdsProductVersion, AdsState, AdsSystemState, AdsTargetType, AmsAddr, AmsNetId,
    AmsPort, Guid, IndexGroup, IndexOffset, WindowsFileTime,
};

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

    /// Starts a new process on the host machine of the ADS device.
    ///
    /// `dir` is the working directory of the process. `args` are the command-line arguments passed
    /// to it. `is_hidden` indicates whether the process starts without a user interface.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tcads_client::devices::tokio::AdsSystemService;
    /// # fn main() -> tcads_client::Result<()> {
    /// # let device = AdsSystemService::connect_local(None)?;
    /// device.start_host_process(
    ///     "C:/Windows/Notepad.exe",
    ///     "C:/TwinCAT/3.1/Target",
    ///     "StaticRoutes.xml",
    ///     false
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn start_process_on_host(
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

        self.device
            .write(
                self.target,
                IndexGroup::SYSTEM_SERVICE_START_PROCESS,
                offset,
                data,
            )
            .await
    }

    /// Reads the System Service's product version.
    pub async fn get_product_version(&self) -> crate::Result<AdsProductVersion> {
        let data = self
            .device
            .read(
                self.target,
                IndexGroup::SYSTEM_SERVICE_PRODUCT_VERSION,
                IndexOffset::ZERO,
                AdsProductVersion::MIN_LENGTH as u32,
            )
            .await?;

        Ok(AdsProductVersion::try_from_slice(&data).map_err(AdsError::from)?)
    }

    /// Reads the ADS device's current UTC system time.
    pub async fn get_time_utc(&self) -> crate::Result<DateTime<Utc>> {
        let data = self
            .device
            .read(
                self.target,
                IndexGroup::SYSTEM_SERVICE_TIME_SERVICES,
                IndexOffset::SYSTEM_SERVICE_TIME_UTC,
                WindowsFileTime::LENGTH as u32,
            )
            .await?;

        let ft = WindowsFileTime::try_from_slice(&data).map_err(AdsError::from)?;
        Ok(ft.to_datetime())
    }

    /// Sets the ADS device's UTC system time.
    pub async fn set_time_utc(&self, time: DateTime<Utc>) -> crate::Result<()> {
        let ft = WindowsFileTime::from_datetime(time);
        self.device
            .write(
                self.target,
                IndexGroup::SYSTEM_SERVICE_TIME_SERVICES,
                IndexOffset::SYSTEM_SERVICE_TIME_UTC,
                ft.to_bytes(),
            )
            .await
    }

    /// Reads the ADS device's current local system time.
    pub async fn get_time_local(&self) -> crate::Result<NaiveDateTime> {
        let data = self
            .device
            .read(
                self.target,
                IndexGroup::SYSTEM_SERVICE_TIME_SERVICES,
                IndexOffset::SYSTEM_SERVICE_TIME_LOCAL,
                WindowsFileTime::LENGTH as u32,
            )
            .await?;

        let ft = WindowsFileTime::try_from_slice(&data).map_err(AdsError::from)?;
        Ok(ft.to_naive_datetime())
    }

    /// Sets the ADS device's local system time.
    pub async fn set_time_local(&self, time: NaiveDateTime) -> crate::Result<()> {
        let ft = WindowsFileTime::from_naive_datetime(time);
        self.device
            .write(
                self.target,
                IndexGroup::SYSTEM_SERVICE_TIME_SERVICES,
                IndexOffset::SYSTEM_SERVICE_TIME_LOCAL,
                ft.to_bytes(),
            )
            .await
    }

    /// Reads the System Service's overall runtime status (ADS state, device state,
    /// restart count, version, platform, OS type, flags, and timeout).
    pub async fn get_system_state(&self) -> crate::Result<AdsSystemState> {
        let data = self
            .device
            .read(
                self.target,
                IndexGroup::SYSTEM_SERVICE_STATE,
                IndexOffset::ZERO,
                AdsSystemState::LENGTH as u32,
            )
            .await?;

        Ok(AdsSystemState::try_from_slice(&data).map_err(AdsError::from)?)
    }

    /// Retrieves information about the target device's host hardware, operating system, and
    /// TwinCAT installation as an XML string.
    pub async fn get_target_info(&self) -> crate::Result<String> {
        let data = self
            .device
            .read(
                self.target,
                IndexGroup::SYSTEM_SERVICE_TARGET_INFO,
                IndexOffset::SYSTEM_SERVICE_TARGET_INFO_XML,
                4,
            )
            .await?;

        let len = shared::decode_u32_le(&data)?;

        let data = self
            .device
            .read(
                self.target,
                IndexGroup::SYSTEM_SERVICE_TARGET_INFO,
                IndexOffset::SYSTEM_SERVICE_TARGET_INFO_XML,
                len,
            )
            .await?;

        Ok(String::from_utf8_lossy(&data).trim_matches('\0').into())
    }

    /// Reads the target device category (PC, CX, BC, or BX).
    pub async fn get_target_type(&self) -> crate::Result<AdsTargetType> {
        let data = self
            .device
            .read(
                self.target,
                IndexGroup::SYSTEM_SERVICE_TARGET_INFO,
                IndexOffset::SYSTEM_SERVICE_TARGET_INFO_TYPE,
                AdsTargetType::LENGTH as u32,
            )
            .await?;

        Ok(AdsTargetType::from(shared::decode_u32_le(&data)?))
    }

    /// Reads the target platform name.
    pub async fn get_target_platform(&self) -> crate::Result<String> {
        let data = self
            .device
            .read(
                self.target,
                IndexGroup::SYSTEM_SERVICE_TARGET_INFO,
                IndexOffset::SYSTEM_SERVICE_TARGET_INFO_PLATFORM,
                64,
            )
            .await?;

        Ok(String::from_utf8_lossy(&data).trim_matches('\0').into())
    }

    /// Reads the currently loaded project's GUID.
    pub async fn get_target_project_guid(&self) -> crate::Result<Guid> {
        let data = self
            .device
            .read(
                self.target,
                IndexGroup::SYSTEM_SERVICE_TARGET_INFO,
                IndexOffset::SYSTEM_SERVICE_TARGET_INFO_PROJECT_GUID,
                Guid::LENGTH as u32,
            )
            .await?;

        Ok(Guid::try_from_slice(&data).map_err(AdsError::from)?)
    }

    /// Reads the currently loaded project's version GUID.
    pub async fn get_target_project_version_guid(&self) -> crate::Result<Guid> {
        let data = self
            .device
            .read(
                self.target,
                IndexGroup::SYSTEM_SERVICE_TARGET_INFO,
                IndexOffset::SYSTEM_SERVICE_TARGET_INFO_PROJECT_VERSION_GUID,
                Guid::LENGTH as u32,
            )
            .await?;

        Ok(Guid::try_from_slice(&data).map_err(AdsError::from)?)
    }

    /// Reads the currently loaded project's name.
    pub async fn get_target_project_name(&self) -> crate::Result<String> {
        let data = &self
            .device
            .read(
                self.target,
                IndexGroup::SYSTEM_SERVICE_TARGET_INFO,
                IndexOffset::SYSTEM_SERVICE_TARGET_INFO_PROJECT_NAME,
                4,
            )
            .await?;

        let len = shared::decode_u32_le(data)?;

        let data = self
            .device
            .read(
                self.target,
                IndexGroup::SYSTEM_SERVICE_TARGET_INFO,
                IndexOffset::SYSTEM_SERVICE_TARGET_INFO_PROJECT_NAME,
                len,
            )
            .await?;

        Ok(String::from_utf8_lossy(&data).trim_matches('\0').into())
    }

    /// Reads the target's self-signed certificate fingerprint.
    pub async fn get_target_fingerprint(&self) -> crate::Result<String> {
        let data = self
            .device
            .read(
                self.target,
                IndexGroup::SYSTEM_SERVICE_TARGET_INFO,
                IndexOffset::SYSTEM_SERVICE_TARGET_INFO_CERT_FINGERPRINT,
                129,
            )
            .await?;

        Ok(String::from_utf8_lossy(&data).trim_matches('\0').into())
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
