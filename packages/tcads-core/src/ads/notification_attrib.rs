use super::AdsTransMode;
use super::error::AdsNotificationAttribError;

/// Attributes for an ADS device notification.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq, Default, Hash)]
pub struct AdsNotificationAttrib {
    /// Length of bytes which should be sent every notification.
    pub length: u32,
    /// How and when the notification should be sent.
    pub trans_mode: AdsTransMode,
    /// Maximum time the server/client may buffer a notification before sending it 100ns steps
    /// (`1ms = 10_000`). `0` means send it immediately.
    pub max_delay: u32,
    /// How often the server/client checks the variable for changes 100ns steps (`1ms = 10_000`).
    pub cycle_time: u32,
}

impl AdsNotificationAttrib {
    /// Wire size of the notification attributes in bytes.
    pub const LENGTH: usize = 16;

    /// Creates a new notification attribute.
    ///
    /// `max_delay` and `cycle_time` must be provided in 100-nanosecond intervals.
    /// For example, 1 millisecond = `10_000`.
    pub fn new(length: u32, trans_mode: AdsTransMode, max_delay: u32, cycle_time: u32) -> Self {
        Self {
            length,
            trans_mode,
            max_delay,
            cycle_time,
        }
    }

    /// Serializes the attribute into exactly 16 bytes for the ADS wire protocol.
    pub fn to_bytes(&self) -> [u8; Self::LENGTH] {
        let mut buf = [0u8; Self::LENGTH];
        buf[0..4].copy_from_slice(&self.length.to_le_bytes());
        buf[4..8].copy_from_slice(&self.trans_mode.to_bytes());
        buf[8..12].copy_from_slice(&self.max_delay.to_le_bytes());
        buf[12..16].copy_from_slice(&self.cycle_time.to_le_bytes());
        buf
    }

    /// Create a new notification attribute from an array of bytes.
    pub fn from_bytes(bytes: [u8; Self::LENGTH]) -> Self {
        let length = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let trans_mode = AdsTransMode::from_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let max_delay = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        let cycle_time = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
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
