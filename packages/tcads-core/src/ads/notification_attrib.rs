use super::AdsTransMode;
use super::error::AdsNotificationAttribError;
use std::time::Duration;

/// Attributes for an ADS device notification.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq, Default, Hash)]
pub struct AdsNotificationAttrib {
    length: u32,
    trans_mode: AdsTransMode,
    max_delay: Duration,
    cycle_time: Duration,
}

impl AdsNotificationAttrib {
    /// Wire size of the notification attributes in bytes.
    pub const LENGTH: usize = 16;

    /// Creates a new notification attribute.
    pub fn new(
        length: u32,
        trans_mode: AdsTransMode,
        max_delay: Duration,
        cycle_time: Duration,
    ) -> Self {
        Self {
            length,
            trans_mode,
            max_delay,
            cycle_time,
        }
    }

    /// Returns the length of bytes which should be sent every notification.
    pub fn length(&self) -> u32 {
        self.length
    }

    /// Returns how and when the notification should be sent.
    pub fn trans_mode(&self) -> AdsTransMode {
        self.trans_mode
    }

    /// Returns the maximum time the server/client may buffer a notification before sending it.
    pub fn max_delay(&self) -> Duration {
        self.max_delay
    }

    /// Returns how often the server/client checks the variable for changes.
    pub fn cycle_time(&self) -> Duration {
        self.cycle_time
    }

    /// Helper to convert a [`Duration`] into TwinCAT's 100ns wire format (u32).
    /// Safely clamps values that exceed ~429 seconds to [`u32::MAX`].
    pub fn duration_to_ticks(duration: Duration) -> u32 {
        let ticks = duration.as_nanos() / 100;
        u32::try_from(ticks).unwrap_or(u32::MAX)
    }

    /// Helper to convert TwinCAT's 100ns wire format (u32) into a Duration.
    pub fn ticks_to_duration(ticks: u32) -> Duration {
        Duration::from_nanos(ticks as u64 * 100)
    }

    /// Serializes the attribute into exactly 16 bytes for the ADS wire protocol.
    pub fn to_bytes(&self) -> [u8; Self::LENGTH] {
        let mut buf = [0u8; Self::LENGTH];
        buf[0..4].copy_from_slice(&self.length.to_le_bytes());
        buf[4..8].copy_from_slice(&self.trans_mode.to_bytes());

        let max_delay_ticks = Self::duration_to_ticks(self.max_delay);
        buf[8..12].copy_from_slice(&max_delay_ticks.to_le_bytes());

        let cycle_time_ticks = Self::duration_to_ticks(self.cycle_time);
        buf[12..16].copy_from_slice(&cycle_time_ticks.to_le_bytes());

        buf
    }

    /// Create a new notification attribute from an array of bytes.
    pub fn from_bytes(bytes: [u8; Self::LENGTH]) -> Self {
        let length = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let trans_mode = AdsTransMode::from_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);

        let max_delay_ticks = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        let max_delay = Self::ticks_to_duration(max_delay_ticks);

        let cycle_time_ticks = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
        let cycle_time = Self::ticks_to_duration(cycle_time_ticks);

        Self {
            length,
            trans_mode,
            max_delay,
            cycle_time,
        }
    }

    pub fn try_from_slice(data: &[u8]) -> Result<Self, AdsNotificationAttribError> {
        if data.len() != Self::LENGTH {
            return Err(AdsNotificationAttribError::UnexpectedLength {
                expected: Self::LENGTH,
                got: data.len(),
            });
        }
        Ok(Self::from_bytes(data.try_into().unwrap()))
    }
}

impl From<AdsNotificationAttrib> for [u8; AdsNotificationAttrib::LENGTH] {
    fn from(attr: AdsNotificationAttrib) -> Self {
        attr.to_bytes()
    }
}

impl From<[u8; Self::LENGTH]> for AdsNotificationAttrib {
    fn from(bytes: [u8; Self::LENGTH]) -> Self {
        Self::from_bytes(bytes)
    }
}

impl TryFrom<&[u8]> for AdsNotificationAttrib {
    type Error = AdsNotificationAttribError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        Self::try_from_slice(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normal_duration_conversion() {
        let attr = AdsNotificationAttrib::new(
            4,
            AdsTransMode::ServerOnChange,
            Duration::from_millis(0),
            Duration::from_millis(100),
        );

        let bytes = attr.to_bytes();

        // 0 ms -> 0 ticks
        assert_eq!(&bytes[8..12], &0u32.to_le_bytes());
        // 100 ms -> 100_000 microseconds -> 1_000_000 ticks of 100ns
        assert_eq!(&bytes[12..16], &1_000_000u32.to_le_bytes());

        let parsed = AdsNotificationAttrib::from_bytes(bytes);
        assert_eq!(parsed.max_delay, Duration::ZERO);
        assert_eq!(parsed.cycle_time, Duration::from_millis(100));
    }

    #[test]
    fn test_overflow_clamping() {
        let attr = AdsNotificationAttrib::new(
            4,
            AdsTransMode::ServerCycle,
            Duration::from_secs(3600), // 1 hour (exceeds ~429 seconds)
            Duration::from_secs(500),  // Also exceeds 429.49 seconds
        );

        let bytes = attr.to_bytes();

        // Both should clamp to u32::MAX
        assert_eq!(&bytes[8..12], &u32::MAX.to_le_bytes());
        assert_eq!(&bytes[12..16], &u32::MAX.to_le_bytes());

        // Parsing them back should yield the maximum representable duration (~429 seconds)
        let parsed = AdsNotificationAttrib::from_bytes(bytes);
        let max_duration = Duration::from_nanos(u32::MAX as u64 * 100);

        assert_eq!(parsed.max_delay, max_duration);
        assert_eq!(parsed.cycle_time, max_duration);
    }
}
