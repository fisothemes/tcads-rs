use super::shared;
use crate::devices::blocking::{AdsDevice, AdsSubsystem};
use chrono::{DateTime, NaiveDateTime, Utc};
use std::time::Duration;
use tcads_core::{
    AdsError, AdsFileFlags, AdsFileHandle, AdsFilePathType, AdsFileSeekOrigin, AdsFileStatus,
    AdsProductVersion, AdsState, AdsSystemState, AdsTargetType, AmsAddr, AmsNetId, AmsPort, Guid,
    IndexGroup, IndexOffset, WindowsFileTime,
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
    pub fn connect(
        net_id: impl Into<AmsNetId>,
        timeout: impl Into<Option<Duration>>,
    ) -> crate::Result<Self> {
        let device = AdsDevice::connect(timeout)?;
        let device = Self::new(device, net_id.into());
        let _ = device.read_state()?;
        Ok(device)
    }

    /// Connects to the TwinCAT System Service (Port 10000) of an ADS device using the local
    /// AMS router at `127.0.0.1:48898` and the Net ID obtained using [`AdsDevice::get_local_net_id`].
    ///
    /// See [`AdsDevice::connect`] for more details.
    pub fn connect_local(timeout: impl Into<Option<Duration>>) -> crate::Result<Self> {
        let device = AdsDevice::connect(timeout)?;
        let net_id = device.get_local_net_id()?;
        let device = Self::new(device, net_id);
        let _ = device.read_state()?;
        Ok(device)
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
        source: impl Into<AmsAddr>,
        net_id: impl Into<AmsNetId>,
        timeout: impl Into<Option<Duration>>,
    ) -> crate::Result<Self> {
        let device = AdsDevice::connect_remote(addr, source, timeout)?;
        let device = Self::new(device, net_id.into());
        let _ = device.read_state()?;
        Ok(device)
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
    /// device.start_process_on_host(
    ///     "C:/Windows/Notepad.exe",
    ///     "C:/TwinCAT/3.1/Target",
    ///     "StaticRoutes.xml",
    ///     false
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn start_process_on_host(
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

    /// Reads the System Service's product version.
    pub fn get_product_version(&self) -> crate::Result<AdsProductVersion> {
        let data = self.device.read(
            self.target,
            IndexGroup::SYSTEM_SERVICE_PRODUCT_VERSION,
            IndexOffset::ZERO,
            AdsProductVersion::MIN_LENGTH as u32,
        )?;

        Ok(AdsProductVersion::try_from_slice(&data).map_err(AdsError::from)?)
    }

    /// Reads the ADS device's current UTC system time.
    pub fn get_time_utc(&self) -> crate::Result<DateTime<Utc>> {
        let data = self.device.read(
            self.target,
            IndexGroup::SYSTEM_SERVICE_TIME_SERVICES,
            IndexOffset::SYSTEM_SERVICE_TIME_UTC,
            WindowsFileTime::LENGTH as u32,
        )?;

        let ft = WindowsFileTime::try_from_slice(&data).map_err(AdsError::from)?;
        Ok(ft.to_datetime())
    }

    /// Sets the ADS device's UTC system time.
    pub fn set_time_utc(&self, time: DateTime<Utc>) -> crate::Result<()> {
        let ft = WindowsFileTime::from_datetime(time);
        self.device.write(
            self.target,
            IndexGroup::SYSTEM_SERVICE_TIME_SERVICES,
            IndexOffset::SYSTEM_SERVICE_TIME_UTC,
            ft.to_bytes(),
        )
    }

    /// Reads the ADS device's current local system time.
    pub fn get_time_local(&self) -> crate::Result<NaiveDateTime> {
        let data = self.device.read(
            self.target,
            IndexGroup::SYSTEM_SERVICE_TIME_SERVICES,
            IndexOffset::SYSTEM_SERVICE_TIME_LOCAL,
            WindowsFileTime::LENGTH as u32,
        )?;

        let ft = WindowsFileTime::try_from_slice(&data).map_err(AdsError::from)?;
        Ok(ft.to_naive_datetime())
    }

    /// Sets the ADS device's local system time.
    pub fn set_time_local(&self, time: NaiveDateTime) -> crate::Result<()> {
        let ft = WindowsFileTime::from_naive_datetime(time);
        self.device.write(
            self.target,
            IndexGroup::SYSTEM_SERVICE_TIME_SERVICES,
            IndexOffset::SYSTEM_SERVICE_TIME_LOCAL,
            ft.to_bytes(),
        )
    }

    /// Reads the System Service's overall runtime status (ADS state, device state,
    /// restart count, version, platform, OS type, flags, and timeout).
    pub fn get_system_state(&self) -> crate::Result<AdsSystemState> {
        let data = self.device.read(
            self.target,
            IndexGroup::SYSTEM_SERVICE_STATE,
            IndexOffset::ZERO,
            AdsSystemState::LENGTH as u32,
        )?;

        Ok(AdsSystemState::try_from_slice(&data).map_err(AdsError::from)?)
    }

    /// Retrieves information about the target device's host hardware, operating system, and
    /// TwinCAT installation as an XML string.
    pub fn get_target_info(&self) -> crate::Result<String> {
        let data = self.device.read(
            self.target,
            IndexGroup::SYSTEM_SERVICE_TARGET_INFO,
            IndexOffset::SYSTEM_SERVICE_TARGET_INFO_XML,
            4,
        )?;

        let len = shared::decode_u32_le(&data)?;

        let data = self.device.read(
            self.target,
            IndexGroup::SYSTEM_SERVICE_TARGET_INFO,
            IndexOffset::SYSTEM_SERVICE_TARGET_INFO_XML,
            len,
        )?;

        Ok(String::from_utf8_lossy(&data).trim_matches('\0').into())
    }

    /// Reads the target device category (PC, CX, BC, or BX).
    pub fn get_target_type(&self) -> crate::Result<AdsTargetType> {
        let data = self.device.read(
            self.target,
            IndexGroup::SYSTEM_SERVICE_TARGET_INFO,
            IndexOffset::SYSTEM_SERVICE_TARGET_INFO_TYPE,
            AdsTargetType::LENGTH as u32,
        )?;

        Ok(AdsTargetType::from(shared::decode_u32_le(&data)?))
    }

    /// Reads the target platform name.
    pub fn get_target_platform(&self) -> crate::Result<String> {
        let data = self.device.read(
            self.target,
            IndexGroup::SYSTEM_SERVICE_TARGET_INFO,
            IndexOffset::SYSTEM_SERVICE_TARGET_INFO_PLATFORM,
            64,
        )?;

        Ok(String::from_utf8_lossy(&data).trim_matches('\0').into())
    }

    /// Reads the currently loaded project's GUID.
    pub fn get_target_project_guid(&self) -> crate::Result<Guid> {
        let data = self.device.read(
            self.target,
            IndexGroup::SYSTEM_SERVICE_TARGET_INFO,
            IndexOffset::SYSTEM_SERVICE_TARGET_INFO_PROJECT_GUID,
            Guid::LENGTH as u32,
        )?;

        Ok(Guid::try_from_slice(&data).map_err(AdsError::from)?)
    }

    /// Reads the currently loaded project's version GUID.
    pub fn get_target_project_version_guid(&self) -> crate::Result<Guid> {
        let data = self.device.read(
            self.target,
            IndexGroup::SYSTEM_SERVICE_TARGET_INFO,
            IndexOffset::SYSTEM_SERVICE_TARGET_INFO_PROJECT_VERSION_GUID,
            Guid::LENGTH as u32,
        )?;

        Ok(Guid::try_from_slice(&data).map_err(AdsError::from)?)
    }

    /// Reads the currently loaded project's name.
    pub fn get_target_project_name(&self) -> crate::Result<String> {
        let data = &self.device.read(
            self.target,
            IndexGroup::SYSTEM_SERVICE_TARGET_INFO,
            IndexOffset::SYSTEM_SERVICE_TARGET_INFO_PROJECT_NAME,
            4,
        )?;

        let len = shared::decode_u32_le(data)?;

        let data = self.device.read(
            self.target,
            IndexGroup::SYSTEM_SERVICE_TARGET_INFO,
            IndexOffset::SYSTEM_SERVICE_TARGET_INFO_PROJECT_NAME,
            len,
        )?;

        Ok(String::from_utf8_lossy(&data).trim_matches('\0').into())
    }

    /// Reads the target's self-signed certificate fingerprint.
    pub fn get_target_fingerprint(&self) -> crate::Result<String> {
        let data = self.device.read(
            self.target,
            IndexGroup::SYSTEM_SERVICE_TARGET_INFO,
            IndexOffset::SYSTEM_SERVICE_TARGET_INFO_CERT_FINGERPRINT,
            129,
        )?;

        Ok(String::from_utf8_lossy(&data).trim_matches('\0').into())
    }

    /// Opens a file on the ADS device's host machine and returns a file handle.
    pub fn open_file(
        &self,
        path: impl AsRef<str>,
        path_type: AdsFilePathType,
        flags: AdsFileFlags,
    ) -> crate::Result<AdsFileHandle> {
        let offset = IndexOffset::new(((path_type.as_u16() as u32) << 16) | flags.as_raw());

        let data = self.device.read_write(
            self.target,
            IndexGroup::SYSTEM_SERVICE_FOPEN,
            offset,
            AdsFileHandle::LENGTH as u32,
            path.as_ref().as_bytes(),
        )?;

        Ok(AdsFileHandle::try_from_slice(&data).map_err(AdsError::from)?)
    }

    /// Closes a file handle created a [`open_file`](Self::open_file) call.
    pub fn close_file(&self, handle: AdsFileHandle) -> crate::Result<()> {
        self.device.read_write(
            self.target,
            IndexGroup::SYSTEM_SERVICE_FCLOSE,
            IndexOffset::new(handle.into()),
            0,
            [],
        )?;

        Ok(())
    }

    /// Reads the contents of a file on the ADS device's host machine into a buffer and returns the
    /// number of bytes read.
    pub fn read_file(&self, handle: AdsFileHandle, buf: &mut [u8]) -> crate::Result<usize> {
        let data = self.device.read_write(
            self.target,
            IndexGroup::SYSTEM_SERVICE_FREAD,
            IndexOffset::new(handle.as_u32()),
            buf.len() as u32,
            [],
        )?;

        let n = data.len().min(buf.len());
        buf[..n].copy_from_slice(&data[..n]);
        Ok(n)
    }

    /// Writes data to a file on the ADS device's host machine. Returns the number of bytes written.
    pub fn write_file(&self, handle: AdsFileHandle, data: &[u8]) -> crate::Result<usize> {
        let resp = self.device.read_write(
            self.target,
            IndexGroup::SYSTEM_SERVICE_FWRITE,
            IndexOffset::new(handle.as_u32()),
            4,
            data,
        )?;

        Ok(shared::decode_u32_le(&resp)? as usize)
    }

    /// Sets an open file's position, relative to `origin`.
    pub fn seek_file(
        &self,
        handle: AdsFileHandle,
        pos: i32,
        origin: AdsFileSeekOrigin,
    ) -> crate::Result<()> {
        let mut data = Vec::with_capacity(8);
        data.extend_from_slice(&pos.to_le_bytes());
        data.extend_from_slice(&origin.as_u32().to_le_bytes());

        self.device.read_write(
            self.target,
            IndexGroup::SYSTEM_SERVICE_FSEEK,
            IndexOffset::new(handle.as_u32()),
            0,
            data,
        )?;
        Ok(())
    }

    /// Reads an open file's current position.
    pub fn tell_file(&self, handle: AdsFileHandle) -> crate::Result<u32> {
        let data = self.device.read_write(
            self.target,
            IndexGroup::SYSTEM_SERVICE_FTELL,
            IndexOffset::new(handle.as_u32()),
            4,
            [],
        )?;

        shared::decode_u32_le(&data)
    }

    /// Returns `true` if an open file's position is at end-of-file.
    pub fn is_file_eof(&self, handle: AdsFileHandle) -> crate::Result<bool> {
        let data = self.device.read_write(
            self.target,
            IndexGroup::SYSTEM_SERVICE_FEOF,
            IndexOffset::new(handle.as_u32()),
            4,
            [],
        )?;

        Ok(shared::decode_u32_le(&data)? != 0)
    }

    /// Deletes a file on the ADS device's host machine.
    pub fn delete_file(
        &self,
        path: impl AsRef<str>,
        path_type: AdsFilePathType,
    ) -> crate::Result<()> {
        let offset = IndexOffset::new(((path_type.as_u16() as u32) << 16) | 0);

        self.device.read_write(
            self.target,
            IndexGroup::SYSTEM_SERVICE_FDELETE,
            offset,
            0,
            path.as_ref().as_bytes(),
        )?;
        Ok(())
    }

    /// Renames or moves a file on the ADS device's host machine.
    pub fn rename_file(
        &self,
        from: impl AsRef<str>,
        to: impl AsRef<str>,
        path_type: AdsFilePathType,
    ) -> crate::Result<()> {
        let data = shared::build_rename_or_copy_request(from.as_ref(), to.as_ref())?;

        self.device.read_write(
            self.target,
            IndexGroup::SYSTEM_SERVICE_FRENAME,
            IndexOffset::new(path_type.as_u16() as u32),
            0,
            data,
        )?;
        Ok(())
    }

    /// Copies a file on the ADS device's host machine.
    pub fn copy_file(
        &self,
        from: impl AsRef<str>,
        to: impl AsRef<str>,
        path_type: AdsFilePathType,
    ) -> crate::Result<()> {
        let data = shared::build_rename_or_copy_request(from.as_ref(), to.as_ref())?;

        self.device.read_write(
            self.target,
            IndexGroup::SYSTEM_SERVICE_FCOPY,
            IndexOffset::new(path_type.as_u16() as u32),
            0,
            data,
        )?;
        Ok(())
    }

    /// Reads a file's status (size, timestamps, attributes).
    pub fn get_file_status(
        &self,
        path: impl AsRef<str>,
        path_type: AdsFilePathType,
    ) -> crate::Result<AdsFileStatus> {
        let data = self.device.read_write(
            self.target,
            IndexGroup::SYSTEM_SERVICE_FGETSTATUS,
            IndexOffset::new(path_type.as_u16() as u32),
            AdsFileStatus::LENGTH as u32,
            path.as_ref().as_bytes(),
        )?;

        Ok(AdsFileStatus::try_from_slice(&data).map_err(AdsError::from)?)
    }

    /// Creates a directory on the ADS device's host machine.
    pub fn create_dir(
        &self,
        path: impl AsRef<str>,
        path_type: AdsFilePathType,
    ) -> crate::Result<()> {
        self.device.read_write(
            self.target,
            IndexGroup::SYSTEM_SERVICE_MKDIR,
            IndexOffset::new(path_type.as_u16() as u32),
            0,
            path.as_ref().as_bytes(),
        )?;
        Ok(())
    }

    /// Removes a directory on the ADS device's host machine.
    pub fn remove_dir(
        &self,
        path: impl AsRef<str>,
        path_type: AdsFilePathType,
    ) -> crate::Result<()> {
        self.device.read_write(
            self.target,
            IndexGroup::SYSTEM_SERVICE_RMDIR,
            IndexOffset::new(path_type.as_u16() as u32),
            0,
            path.as_ref().as_bytes(),
        )?;
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
