use super::error::WindowsFileTimeError;
use chrono::{DateTime, TimeZone, Utc};

/// A timestamp encoded in the Windows FILETIME format.
///
/// Represents the number of 100-nanosecond intervals since `1601-01-01 00:00:00 UTC`.
/// Used in [`AdsStampHeader`](crate::protocol::AdsStampHeader) to timestamp incoming
/// device notification samples, and required when constructing outgoing
/// [`AdsDeviceNotification`](crate::protocol::AdsDeviceNotification) frames on a server.
///
/// # Conversions
/// - [`WindowsFileTime::now`] - construct from the current UTC time.
/// - [`WindowsFileTime::to_datetime`] - convert to [`DateTime<Utc>`] for display or arithmetic.
/// - [`WindowsFileTime::from_datetime`] - convert from [`DateTime<Utc>`].
/// - [`WindowsFileTime::as_raw`] - access the raw tick count as an escape hatch.
///
/// # Precision
/// FILETIME has 100-nanosecond resolution. [`DateTime<Utc>`] has microsecond resolution,
/// so one decimal place of sub-microsecond precision is lost on conversion. This is
/// inconsequential for ADS notification timestamps in practice.
///
/// # Wire Format
/// 8 bytes, little-endian `u64`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WindowsFileTime(u64);

impl WindowsFileTime {
    /// The length of a `WindowsFileTime` on the wire.
    pub const LENGTH: usize = 8;

    /// The number of `100ns` ticks between `1601-01-01` and `1970-01-01` (the Unix epoch).
    ///
    /// Computed as: 369 years × 365.2425 days/year × 86,400 s/day × 10,000,000 ticks/s
    pub const FILETIME_TO_UNIX_EPOCH_TICKS: u64 = 116_444_736_000_000_000;

    /// Number of 100ns ticks per second.
    pub const TICKS_PER_SEC: u64 = 10_000_000;

    /// Number of 100ns ticks per microsecond.
    pub const TICKS_PER_MICROS: u64 = 10;

    /// Creates a `WindowsFileTime` from a raw tick count.
    ///
    /// The value must be the number of `100ns` intervals since `1601-01-01 UTC`.
    pub const fn from_raw(ticks: u64) -> Self {
        Self(ticks)
    }

    /// Returns the raw tick count (100ns intervals since 1601-01-01 UTC).
    pub const fn as_raw(self) -> u64 {
        self.0
    }

    /// Tries to parse a `WindowsFileTime` from a byte slice.
    ///
    /// Returns an error if the slice is shorter than 8 bytes.
    pub fn try_from_slice(bytes: &[u8]) -> Result<Self, WindowsFileTimeError> {
        bytes.try_into()
    }

    /// Creates a `WindowsFileTime` from an 8-byte little-endian array.
    pub fn from_bytes(bytes: [u8; Self::LENGTH]) -> Self {
        Self(u64::from_le_bytes(bytes))
    }

    /// Returns the 8-byte little-endian representation.
    pub fn to_bytes(self) -> [u8; Self::LENGTH] {
        self.0.to_le_bytes()
    }

    /// Returns a `WindowsFileTime` representing the current UTC time.
    pub fn now() -> Self {
        Self::from_datetime(Utc::now())
    }

    /// Converts to a [`DateTime<Utc>`].
    ///
    /// Saturates to the Unix epoch (`1970-01-01 00:00:00 UTC`) for FILETIME values
    /// before 1970, which should never occur in practice for ADS notification timestamps.
    pub fn to_datetime(self) -> DateTime<Utc> {
        let ticks_since_unix = self.0.saturating_sub(Self::FILETIME_TO_UNIX_EPOCH_TICKS);

        let micros = (ticks_since_unix / Self::TICKS_PER_MICROS) as i64;

        Utc.timestamp_micros(micros)
            .single()
            .unwrap_or(DateTime::<Utc>::UNIX_EPOCH)
    }

    /// Converts from a [`DateTime<Utc>`].
    ///
    /// Saturates to the FILETIME epoch (`1601-01-01`) for datetimes before the Unix
    /// epoch, which should never occur in practice.
    pub fn from_datetime(dt: DateTime<Utc>) -> Self {
        let micros = dt.timestamp_micros().max(0) as u64;
        let ticks_since_unix = micros * Self::TICKS_PER_MICROS;

        Self(Self::FILETIME_TO_UNIX_EPOCH_TICKS + ticks_since_unix)
    }
}

impl From<u64> for WindowsFileTime {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<WindowsFileTime> for u64 {
    fn from(value: WindowsFileTime) -> Self {
        value.0
    }
}

impl From<[u8; WindowsFileTime::LENGTH]> for WindowsFileTime {
    fn from(value: [u8; WindowsFileTime::LENGTH]) -> Self {
        Self::from_bytes(value)
    }
}

impl From<WindowsFileTime> for [u8; WindowsFileTime::LENGTH] {
    fn from(value: WindowsFileTime) -> Self {
        value.to_bytes()
    }
}

impl From<DateTime<Utc>> for WindowsFileTime {
    fn from(value: DateTime<Utc>) -> Self {
        Self::from_datetime(value)
    }
}

impl From<WindowsFileTime> for DateTime<Utc> {
    fn from(value: WindowsFileTime) -> Self {
        value.to_datetime()
    }
}

impl TryFrom<&[u8]> for WindowsFileTime {
    type Error = WindowsFileTimeError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() != Self::LENGTH {
            return Err(WindowsFileTimeError::UnexpectedLength {
                expected: Self::LENGTH,
                got: value.len(),
            });
        }
        Ok(Self(u64::from_le_bytes(value[..8].try_into().unwrap())))
    }
}

impl std::fmt::Display for WindowsFileTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.to_datetime().format("%Y-%m-%d %H:%M:%S%.6f UTC")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2026-02-21 12:00:00 UTC as a Windows FILETIME tick count.
    ///
    /// Computed as: (2026-02-21 12:00:00 UTC - 1601-01-01 00:00:00 UTC) in 100ns ticks.
    const KNOWN_TICKS: u64 = 134_161_488_000_000_000;

    fn known_datetime() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 2, 21, 12, 0, 0).unwrap()
    }

    #[test]
    fn test_from_raw_roundtrip() {
        let ft = WindowsFileTime::from_raw(KNOWN_TICKS);
        assert_eq!(ft.as_raw(), KNOWN_TICKS);
    }

    #[test]
    fn test_from_bytes_roundtrip() {
        let ft = WindowsFileTime::from_raw(KNOWN_TICKS);
        let bytes = ft.to_bytes();
        let ft2 = WindowsFileTime::from_bytes(bytes);
        assert_eq!(ft, ft2);
    }

    #[test]
    fn test_try_from_slice_valid() {
        let ft = WindowsFileTime::from_raw(KNOWN_TICKS);
        let bytes = ft.to_bytes();
        let ft2 = WindowsFileTime::try_from_slice(&bytes).unwrap();
        assert_eq!(ft, ft2);
    }

    #[test]
    fn test_try_from_slice_too_long() {
        let ft = WindowsFileTime::from_raw(KNOWN_TICKS);
        let mut bytes = ft.to_bytes().to_vec();
        bytes.extend_from_slice(&[0xFF, 0xFF]);
        let err = WindowsFileTime::try_from_slice(&bytes).unwrap_err();
        assert!(matches!(
            err,
            WindowsFileTimeError::UnexpectedLength {
                expected: 8,
                got: 10
            }
        ));
    }

    #[test]
    fn test_try_from_slice_too_short() {
        let err = WindowsFileTime::try_from_slice(&[0x01, 0x02, 0x03, 0x04]).unwrap_err();
        assert!(matches!(
            err,
            WindowsFileTimeError::UnexpectedLength {
                expected: 8,
                got: 4
            }
        ));
    }

    #[test]
    fn test_to_datetime_known_value() {
        let ft = WindowsFileTime::from_raw(KNOWN_TICKS);
        let dt = ft.to_datetime();
        assert_eq!(dt, known_datetime());
    }

    #[test]
    fn test_from_datetime_known_value() {
        let ft = WindowsFileTime::from_datetime(known_datetime());
        assert_eq!(ft.as_raw(), KNOWN_TICKS);
    }

    #[test]
    fn test_from_datetime_roundtrip() {
        // Round-trip via a FILETIME with microsecond-aligned ticks (no sub-microsecond loss)
        let original = WindowsFileTime::from_raw(KNOWN_TICKS);
        let dt = original.to_datetime();
        let roundtripped = WindowsFileTime::from_datetime(dt);
        assert_eq!(original, roundtripped);
    }

    #[test]
    fn test_from_impl_roundtrip() {
        let original = WindowsFileTime::from_raw(KNOWN_TICKS);
        let dt: DateTime<Utc> = original.into();
        let back: WindowsFileTime = dt.into();
        assert_eq!(original, back);
    }

    #[test]
    fn test_now_is_after_known_value() {
        let known = WindowsFileTime::from_raw(KNOWN_TICKS);
        let now = WindowsFileTime::now();
        assert!(now > known);
    }

    #[test]
    fn test_pre_unix_epoch_saturates() {
        // A FILETIME before the Unix epoch (raw value smaller than the offset constant)
        let pre_epoch = WindowsFileTime::from_raw(100);
        let dt = pre_epoch.to_datetime();
        assert_eq!(dt, DateTime::<Utc>::UNIX_EPOCH);
    }

    #[test]
    fn test_ordering() {
        let earlier = WindowsFileTime::from_raw(KNOWN_TICKS);
        let later = WindowsFileTime::from_raw(KNOWN_TICKS + 10_000_000); // 1 second later
        assert!(later > earlier);
    }

    #[test]
    fn test_le_encoding() {
        // Raw value 1 should encode as [1, 0, 0, 0, 0, 0, 0, 0]
        let ft = WindowsFileTime::from_raw(1);
        assert_eq!(ft.to_bytes(), [1, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_from_u64_into_u64() {
        let ft: WindowsFileTime = KNOWN_TICKS.into();
        let back: u64 = ft.into();
        assert_eq!(back, KNOWN_TICKS);
    }

    #[test]
    fn test_display() {
        let ft = WindowsFileTime::from_raw(KNOWN_TICKS);
        assert_eq!(format!("{ft}"), "2026-02-21 12:00:00.000000 UTC");
    }

    #[test]
    fn test_display_with_subseconds() {
        // 500ms = 500_000 microseconds = 5_000_000 ticks
        let ft = WindowsFileTime::from_raw(KNOWN_TICKS + 5_000_000);
        assert_eq!(format!("{ft}"), "2026-02-21 12:00:00.500000 UTC");
    }
}
