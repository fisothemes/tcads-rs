pub mod guard;

use tcads_core::WindowsFileTime;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct AdsNotification<T = Vec<u8>> {
    timestamp: WindowsFileTime,
    value: T,
}

impl<T> AdsNotification<T> {
    pub const fn new(timestamp: WindowsFileTime, value: T) -> Self {
        Self { timestamp, value }
    }

    pub const fn timestamp(&self) -> WindowsFileTime {
        self.timestamp
    }

    pub const fn value(&self) -> &T {
        &self.value
    }

    pub fn into_value(self) -> T {
        self.value
    }

    pub fn into_parts(self) -> (WindowsFileTime, T) {
        (self.timestamp, self.value)
    }

    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> AdsNotification<U> {
        AdsNotification {
            timestamp: self.timestamp,
            value: f(self.value),
        }
    }

    pub fn try_map<U, E>(self, f: impl FnOnce(T) -> Result<U, E>) -> Result<AdsNotification<U>, E> {
        Ok(AdsNotification {
            timestamp: self.timestamp,
            value: f(self.value)?,
        })
    }
}

impl<T> AsRef<T> for AdsNotification<T> {
    fn as_ref(&self) -> &T {
        &self.value
    }
}

impl<T> From<(WindowsFileTime, T)> for AdsNotification<T> {
    fn from((timestamp, value): (WindowsFileTime, T)) -> Self {
        Self { timestamp, value }
    }
}

impl<T> From<AdsNotification<T>> for (WindowsFileTime, T) {
    fn from(n: AdsNotification<T>) -> Self {
        (n.timestamp, n.value)
    }
}
