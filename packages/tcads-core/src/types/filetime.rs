use std::fmt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Represents a Windows FILETIME (64-bit).
///
/// Contains the number of 100-nanosecond intervals since January 1, 1601 (UTC).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct WindowsFiletime(u64);

impl WindowsFiletime {
    // 1601-01-01 to 1970-01-01 is 11,644,473,600 seconds
    const WINDOWS_TO_UNIX_SECONDS: u64 = 11_644_473_600;
    const TICKS_PER_SECOND: u64 = 10_000_000;

    /// Creates a new Filetime from raw ticks.
    pub const fn new(ticks: u64) -> Self {
        Self(ticks)
    }

    /// Returns the raw 100ns ticks.
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl fmt::Debug for WindowsFiletime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WindowsFiletime({:?})", SystemTime::from(*self))
    }
}

impl From<WindowsFiletime> for SystemTime {
    fn from(val: WindowsFiletime) -> Self {
        let ticks = val.0;
        let seconds_since_windows_epoch = ticks / WindowsFiletime::TICKS_PER_SECOND;
        let nanos_remainder = (ticks % WindowsFiletime::TICKS_PER_SECOND) * 100;

        if seconds_since_windows_epoch >= WindowsFiletime::WINDOWS_TO_UNIX_SECONDS {
            // Date is after 1970
            let unix_seconds =
                seconds_since_windows_epoch - WindowsFiletime::WINDOWS_TO_UNIX_SECONDS;
            UNIX_EPOCH + Duration::new(unix_seconds, nanos_remainder as u32)
        } else {
            // Date is before 1970 (but after 1601)
            let unix_seconds =
                WindowsFiletime::WINDOWS_TO_UNIX_SECONDS - seconds_since_windows_epoch;
            UNIX_EPOCH - Duration::new(unix_seconds, nanos_remainder as u32)
        }
    }
}

impl From<SystemTime> for WindowsFiletime {
    fn from(val: SystemTime) -> Self {
        match val.duration_since(UNIX_EPOCH) {
            Ok(duration) => {
                // Post-1970
                let total_seconds = duration.as_secs() + WindowsFiletime::WINDOWS_TO_UNIX_SECONDS;
                let ticks = (total_seconds * WindowsFiletime::TICKS_PER_SECOND)
                    + (duration.subsec_nanos() as u64 / 100);
                Self(ticks)
            }
            Err(e) => {
                // Pre-1970
                let duration = e.duration();
                let unix_seconds = duration.as_secs();
                let windows_seconds =
                    WindowsFiletime::WINDOWS_TO_UNIX_SECONDS.saturating_sub(unix_seconds);
                let ticks = (windows_seconds * WindowsFiletime::TICKS_PER_SECOND)
                    - (duration.subsec_nanos() as u64 / 100);
                Self(ticks)
            }
        }
    }
}
