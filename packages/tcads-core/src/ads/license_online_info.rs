use super::Guid;
use super::error::AdsLicenseOnlineInfoError;
use super::filetime::WindowsFileTime;

/// One entry from a [`LICENSE_ONLINE_INFO`](super::IndexGroup::LICENSE_ONLINE_INFO) read on the
/// license server ads device (port 30).
///
/// The license count is read first as a plain `u32`, then this many entries are read back
/// as one contiguous array from the same index group.
///
/// `license_id` holds the 16 bytes exactly as they arrive on the wire. Pass
/// [`license_id().as_bytes()`](Guid::as_bytes) as the write payload for
/// [`LICENSE_NAME`](super::IndexGroup::LICENSE_NAME) or
/// [`LICENSE_ORDER_NUMBER`](super::IndexGroup::LICENSE_ORDER_NUMBER) ReadWrite requests to
/// look up this license's display name or order number.
///
/// # Wire Format
///
/// | Offset | Size | Field         |
/// |--------|------|---------------|
/// | 0      | 16   | `license_id`  |
/// | 16     | 8    | `expire_time` |
/// | 24     | 4    | `count`       |
/// | 28     | 4    | `used`        |
/// | 32     | 4    | `status`      |
/// | 36     | 4    | `volume_no`   |
/// | 40     | 8    | reserved      |
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsLicenseOnlineInfo {
    license_id: Guid,
    expire_time: Option<WindowsFileTime>,
    count: u32,
    used: u32,
    status: AdsLicenseStatus,
    volume_no: u32,
}

impl AdsLicenseOnlineInfo {
    /// Wire length of a single [`AdsLicenseOnlineInfo`] entry in bytes.
    pub const LENGTH: usize = 48;

    /// Returns the license identifier, in wire byte order.
    ///
    /// This is the lookup key for the license's name and order number. Its string form
    /// prints the bytes in wire order, so it may not match a GUID shown for the same
    /// license elsewhere in TwinCAT, which typically prints Windows GUIDs with the first
    /// three fields byte-swapped.
    pub fn license_id(&self) -> &Guid {
        &self.license_id
    }

    /// Returns the license's expiry time, or `None` if it does not expire.
    ///
    /// On the wire the expiry is a signed tick count; a negative value (commonly `-1`)
    /// conventionally marks a license that does not expire, and is read as `None`. That
    /// convention isn't confirmed by the source this type was derived from.
    pub fn expire_time(&self) -> Option<WindowsFileTime> {
        self.expire_time
    }

    /// Returns the license count. `0` conventionally means a per-CPU license rather than a
    /// fixed instance count.
    pub fn count(&self) -> u32 {
        self.count
    }

    /// Returns the number of instances of this license currently in use.
    pub fn used(&self) -> u32 {
        self.used
    }

    /// Returns the license's validity status.
    pub fn status(&self) -> AdsLicenseStatus {
        self.status
    }

    /// Returns the volume number this license was issued for.
    pub fn volume_no(&self) -> u32 {
        self.volume_no
    }

    /// Tries to parse a single [`AdsLicenseOnlineInfo`] entry from a byte slice.
    ///
    /// The slice must be exactly [`LENGTH`](Self::LENGTH) bytes; to parse a full array
    /// response, chunk the payload into [`LENGTH`](Self::LENGTH)-sized slices and call this
    /// once per chunk.
    pub fn try_from_slice(data: &[u8]) -> Result<Self, AdsLicenseOnlineInfoError> {
        let data: &[u8; Self::LENGTH] =
            data.try_into()
                .map_err(|_| AdsLicenseOnlineInfoError::UnexpectedLength {
                    expected: Self::LENGTH,
                    got: data.len(),
                })?;

        let u32_at =
            |i: usize| u32::from_le_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]);
        let raw_expire = i64::from_le_bytes(data[16..24].try_into().unwrap());

        Ok(Self {
            license_id: Guid::new(data[0..16].try_into().unwrap()),
            expire_time: u64::try_from(raw_expire)
                .ok()
                .map(WindowsFileTime::from_raw),
            count: u32_at(24),
            used: u32_at(28),
            status: u32_at(32).into(),
            volume_no: u32_at(36),
            // Bytes 40..48 are reserved on the wire and intentionally not stored.
        })
    }
}

impl TryFrom<&[u8]> for AdsLicenseOnlineInfo {
    type Error = AdsLicenseOnlineInfoError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        Self::try_from_slice(data)
    }
}

/// The validity state of an installed license, as reported by the `nResult` field of an
/// [`AdsLicenseOnlineInfo`] entry.
///
/// The known values are undocumented in the public ADS specification; they are taken from
/// the `ShowStatus` mapping in
/// [Beckhoff/ADS `LicenseAccess.cpp`](https://github.com/Beckhoff/ADS/blob/master/AdsLib/LicenseAccess.cpp).
#[derive(
    serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub enum AdsLicenseStatus {
    /// The license is valid. (`0`)
    Valid,
    /// The license is valid and pending (not yet active). (`515`)
    ValidPending,
    /// The license is valid as a trial. (`596`)
    ValidTrial,
    /// The license is valid as an OEM license. (`597`)
    ValidOem,
    /// A status code not recognized by this library.
    Unknown(u32),
}

impl AdsLicenseStatus {
    /// Wire size in bytes.
    pub const LENGTH: usize = 4;
}

impl From<u32> for AdsLicenseStatus {
    fn from(value: u32) -> Self {
        match value {
            0 => Self::Valid,
            515 => Self::ValidPending,
            596 => Self::ValidTrial,
            597 => Self::ValidOem,
            n => Self::Unknown(n),
        }
    }
}

impl From<AdsLicenseStatus> for u32 {
    fn from(value: AdsLicenseStatus) -> Self {
        match value {
            AdsLicenseStatus::Valid => 0,
            AdsLicenseStatus::ValidPending => 515,
            AdsLicenseStatus::ValidTrial => 596,
            AdsLicenseStatus::ValidOem => 597,
            AdsLicenseStatus::Unknown(n) => n,
        }
    }
}

impl std::fmt::Display for AdsLicenseStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Valid => write!(f, "Valid"),
            Self::ValidPending => write!(f, "Valid (Pending)"),
            Self::ValidTrial => write!(f, "Valid (Trial)"),
            Self::ValidOem => write!(f, "Valid (OEM)"),
            Self::Unknown(n) => write!(f, "0x{n:X}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_bytes() -> Vec<u8> {
        let mut data = vec![0u8; AdsLicenseOnlineInfo::LENGTH];
        data[0..16].copy_from_slice(&[0xAB; 16]);
        data[16..24].copy_from_slice(&(-1i64).to_le_bytes()); // never expires
        data[24..28].copy_from_slice(&5u32.to_le_bytes());
        data[28..32].copy_from_slice(&2u32.to_le_bytes());
        data[32..36].copy_from_slice(&596u32.to_le_bytes()); // Trial
        data[36..40].copy_from_slice(&42u32.to_le_bytes());
        data
    }

    #[test]
    fn decodes_all_fields() {
        let info = AdsLicenseOnlineInfo::try_from_slice(&sample_bytes()).unwrap();

        assert_eq!(info.license_id().as_bytes(), &[0xAB; 16]);
        assert_eq!(info.expire_time(), None);
        assert_eq!(info.count(), 5);
        assert_eq!(info.used(), 2);
        assert_eq!(info.status(), AdsLicenseStatus::ValidTrial);
        assert_eq!(info.volume_no(), 42);
    }

    #[test]
    fn positive_expiry_converts() {
        let mut data = sample_bytes();
        data[16..24].copy_from_slice(&1_000i64.to_le_bytes());

        let info = AdsLicenseOnlineInfo::try_from_slice(&data).unwrap();

        assert_eq!(info.expire_time(), Some(WindowsFileTime::from_raw(1_000)));
    }

    #[test]
    fn rejects_wrong_length() {
        let err = AdsLicenseOnlineInfo::try_from_slice(&[0u8; 10]).unwrap_err();
        assert!(matches!(
            err,
            AdsLicenseOnlineInfoError::UnexpectedLength {
                expected: 48,
                got: 10
            }
        ));
    }

    #[test]
    fn status_unknown_roundtrips() {
        let status = AdsLicenseStatus::from(1234);
        assert_eq!(status, AdsLicenseStatus::Unknown(1234));
        assert_eq!(u32::from(status), 1234);
        assert_eq!(status.to_string(), "0x4D2");
    }

    #[test]
    fn status_display_known_values() {
        assert_eq!(AdsLicenseStatus::Valid.to_string(), "Valid");
        assert_eq!(
            AdsLicenseStatus::ValidPending.to_string(),
            "Valid (Pending)"
        );
        assert_eq!(AdsLicenseStatus::ValidTrial.to_string(), "Valid (Trial)");
        assert_eq!(AdsLicenseStatus::ValidOem.to_string(), "Valid (OEM)");
    }
}
