use std::fmt;

/// A handle returned by the ADS server when registering a device notification.
///
/// Used to identify the notification when receiving data or when deleting it.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NotificationHandle(u32);

impl NotificationHandle {
    /// Creates a new handle from a raw u32.
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the raw u32 value.
    pub fn as_u32(&self) -> u32 {
        self.0
    }
}

impl From<u32> for NotificationHandle {
    fn from(val: u32) -> Self {
        Self(val)
    }
}

impl From<NotificationHandle> for u32 {
    fn from(val: NotificationHandle) -> Self {
        val.0
    }
}

impl fmt::Debug for NotificationHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NotificationHandle({})", self.0)
    }
}

impl fmt::Display for NotificationHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
