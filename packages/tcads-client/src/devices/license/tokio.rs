use super::shared;
use crate::devices::tokio::{AdsDevice, AdsDeviceExt};
use std::net::ToSocketAddrs;
use std::time::Duration;
use tcads_core::{
    AdsError, AdsLicenseOnlineInfo, AmsAddr, AmsNetId, AmsPort, Guid, IndexGroup, IndexOffset,
};

/// An [`AdsDevice`] client for the TwinCAT license server ADS device on ADS port `30`.
///
/// Covers reading the installed licenses, resolving a license's display name and order
/// number, and reading the target's platform ID, system ID, and volume number.
///
/// None of this is part of the public ADS specification; it is derived from
/// [Beckhoff/ADS `LicenseAccess.cpp`](https://github.com/Beckhoff/ADS/blob/master/AdsLib/LicenseAccess.cpp).
///
/// # Thread Safety
///
/// [`AdsLicense`] is [`Clone`], so all clones share the same underlying connection. It is
/// also [`Send`] + [`Sync`]. Clean-up only happens when the last clone is dropped.
#[derive(Debug, Clone)]
pub struct AdsLicense {
    device: AdsDevice,
    target: AmsAddr,
}

impl AdsLicense {
    /// Connects to the license server ADS device (Port 30) of a specific `net_id` using the local
    /// AMS router.
    ///
    /// Performs a [`PortConnect`](tcads_core::protocol::PortConnectRequest) handshake
    /// to obtain a dynamically assigned source address from the local router.
    pub async fn connect(
        net_id: impl Into<AmsNetId>,
        timeout: impl Into<Option<Duration>>,
    ) -> crate::Result<Self> {
        let device = AdsDevice::connect(timeout).await?;
        let device = Self::new(device, net_id.into());
        let _ = device.read_state().await?;
        Ok(device)
    }

    /// Connects to the license server ADS device (Port 30) of the local AMS router at
    /// `127.0.0.1:48898`.
    ///
    /// Automatically discovers the local Net ID and targets `local_net_id:30`.
    ///
    /// # Note
    ///
    /// On Windows, connecting via `127.0.0.1` requires the `EnableAmsTcpLoopback`
    /// registry key to be set. This is enabled by default in TwinCAT 4024.5 and newer.
    pub async fn connect_local(timeout: impl Into<Option<Duration>>) -> crate::Result<Self> {
        let device = AdsDevice::connect(timeout).await?;
        let net_id = device.get_local_net_id().await?;
        let device = Self::new(device, net_id);
        let _ = device.read_state().await?;
        Ok(device)
    }

    /// Connects directly to the license server ADS device (Port 30) of a remote AMS router without a
    /// local router.
    ///
    /// The `source` address must be pre-configured as a static route on the remote router.
    /// The `net_id` is the Net ID of the remote target.
    ///
    /// The [`PortConnect`](tcads_core::protocol::PortConnectRequest) handshake is **not**
    /// performed. See [`AdsDevice::connect_remote`] for details.
    pub async fn connect_remote(
        addr: impl ToSocketAddrs,
        source: impl Into<AmsAddr>,
        net_id: impl Into<AmsNetId>,
        timeout: impl Into<Option<Duration>>,
    ) -> crate::Result<Self> {
        let device = AdsDevice::connect_remote(addr, source, timeout).await?;
        let device = Self::new(device, net_id.into());
        let _ = device.read_state().await?;
        Ok(device)
    }

    /// Creates an [`AdsLicense`] from an existing [`AdsDevice`] and router Net ID.
    ///
    /// Use this when sharing an existing connection with other device clients.
    pub fn new(device: AdsDevice, net_id: AmsNetId) -> Self {
        Self {
            device,
            target: AmsAddr::new(net_id, AmsPort::LICENCE_SERVER),
        }
    }

    /// Shuts down the underlying connection.
    ///
    /// See [`AdsDevice::shutdown`] for more details.
    pub async fn shutdown(&self) {
        self.device.shutdown().await
    }

    /// Reads every license currently installed on the target.
    ///
    /// Two round trips: one to read the license count, one to read the resulting array.
    /// Returns an empty `Vec` if no license is installed.
    pub async fn get_online_info(&self) -> crate::Result<Vec<AdsLicenseOnlineInfo>> {
        let count = self
            .device
            .read(
                self.target,
                IndexGroup::LICENSE_ONLINE_INFO,
                IndexOffset::ZERO,
                4,
            )
            .await?;
        let count = shared::decode_u32_le(&count)?;

        if count == 0 {
            return Ok(Vec::new());
        }

        let len = count * (AdsLicenseOnlineInfo::LENGTH as u32);
        let data = self
            .device
            .read(
                self.target,
                IndexGroup::LICENSE_ONLINE_INFO,
                IndexOffset::ZERO,
                len,
            )
            .await?;

        shared::parse_online_info_entries(&data)
    }

    /// Resolves a license's display name from its [`license_id`](AdsLicenseOnlineInfo::license_id).
    pub async fn get_license_name(&self, license_id: &Guid) -> crate::Result<String> {
        let data = self
            .device
            .read_write(
                self.target,
                IndexGroup::LICENSE_NAME,
                IndexOffset::ZERO,
                64,
                license_id.as_bytes().as_slice(),
            )
            .await?;

        Ok(shared::c_string(&data))
    }

    /// Resolves a license's order number from its
    /// [`license_id`](AdsLicenseOnlineInfo::license_id).
    pub async fn get_order_number(&self, license_id: &Guid) -> crate::Result<String> {
        let data = self
            .device
            .read_write(
                self.target,
                IndexGroup::LICENSE_ORDER_NUMBER,
                IndexOffset::ZERO,
                16,
                license_id.as_bytes().as_slice(),
            )
            .await?;

        Ok(shared::c_string(&data))
    }

    /// Reads the target's 16-bit platform ID.
    pub async fn get_platform_id(&self) -> crate::Result<u16> {
        let data = self
            .device
            .read(
                self.target,
                IndexGroup::LICENSE_SYSTEM_INFO,
                IndexOffset::LICENSE_PLATFORM_ID,
                2,
            )
            .await?;

        if data.len() != 2 {
            return Err(AdsError::UnexpectedDataLength {
                expected: 2,
                got: data.len(),
            }
            .into());
        }

        Ok(u16::from_le_bytes([data[0], data[1]]))
    }

    /// Reads the target's system ID as a [`Guid`].
    ///
    /// The wire layout is the standard COM GUID struct: `Data1` (`u32`) and `Data2`/`Data3`
    /// (`u16` each) are little-endian, `Data4`'s 8 bytes are as-is. The bytes are reordered
    /// into display order before being wrapped in a [`Guid`], since [`Guid`] itself has no
    /// awareness of this field layout, it only stores and prints bytes as given.
    pub async fn get_system_id(&self) -> crate::Result<Guid> {
        let raw = self
            .device
            .read(
                self.target,
                IndexGroup::LICENSE_SYSTEM_INFO,
                IndexOffset::LICENSE_SYSTEM_ID,
                Guid::LENGTH as u32,
            )
            .await?;

        if raw.len() != Guid::LENGTH {
            return Err(AdsError::UnexpectedDataLength {
                expected: Guid::LENGTH,
                got: raw.len(),
            }
            .into());
        }

        let reordered = [
            raw[3], raw[2], raw[1], raw[0], raw[5], raw[4], raw[7], raw[6], raw[8], raw[9],
            raw[10], raw[11], raw[12], raw[13], raw[14], raw[15],
        ];

        Ok(Guid::from(reordered))
    }

    /// Reads the target's volume number.
    pub async fn get_volume_no(&self) -> crate::Result<u32> {
        let data = self
            .device
            .read(
                self.target,
                IndexGroup::LICENSE_SYSTEM_INFO,
                IndexOffset::LICENSE_VOLUME_NO,
                4,
            )
            .await?;

        shared::decode_u32_le(&data)
    }
}

impl AdsDeviceExt for AdsLicense {
    fn device(&self) -> &AdsDevice {
        &self.device
    }

    fn target(&self) -> AmsAddr {
        self.target
    }
}
