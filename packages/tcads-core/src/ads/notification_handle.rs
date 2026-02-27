use super::error::AdsNotificationHandleError;
use std::fmt;

/// A handle identifying an active ADS device notification subscription.
///
/// Assigned by the ADS server in response to an
/// [`AdsAddDeviceNotification`](crate::protocol::AdsAddDeviceNotificationRequest) request
/// and returned in every [`AdsDeviceNotification`](crate::protocol::AdsDeviceNotification)
/// sample. Pass it to
/// [`AdsDeleteDeviceNotification`](crate::protocol::AdsDeleteDeviceNotificationRequest)
/// to cancel the subscription.
///
/// The value is opaque, thus it has no meaning beyond identity. Equality and hashing
/// are well-defined, making it suitable as a `HashMap` key for dispatching incoming
/// notification samples to the correct handler.
///
/// # Wire Format
/// 4 bytes, little-endian `u32`.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct NotificationHandle([u8; Self::LENGTH]);

impl NotificationHandle {
    /// The length of a `NotificationHandle` on the wire.
    pub const LENGTH: usize = 4;

    pub fn new(handle: u32) -> Self {
        Self(handle.to_le_bytes())
    }

    /// Creates a [`NotificationHandle`] from a 4-byte array (little-endian).
    pub fn from_bytes(bytes: [u8; Self::LENGTH]) -> Self {
        Self(bytes)
    }

    /// Tries to parse a `NotificationHandle` from a byte slice.
    ///
    /// Returns an error if the slice is shorter than 4 bytes.
    pub fn try_from_slice(bytes: &[u8]) -> Result<Self, AdsNotificationHandleError> {
        bytes.try_into()
    }

    /// Returns the raw 4-byte little-endian representation.
    pub fn to_bytes(&self) -> [u8; Self::LENGTH] {
        self.0
    }

    /// Returns the handle value as a `u32`.
    pub fn as_u32(&self) -> u32 {
        u32::from_le_bytes(self.0)
    }
}

impl From<u32> for NotificationHandle {
    fn from(value: u32) -> Self {
        Self(value.to_le_bytes())
    }
}

impl From<NotificationHandle> for u32 {
    fn from(value: NotificationHandle) -> Self {
        u32::from_le_bytes(value.0)
    }
}

impl From<[u8; NotificationHandle::LENGTH]> for NotificationHandle {
    fn from(bytes: [u8; NotificationHandle::LENGTH]) -> Self {
        Self(bytes)
    }
}

impl From<NotificationHandle> for [u8; NotificationHandle::LENGTH] {
    fn from(handle: NotificationHandle) -> Self {
        handle.0
    }
}

impl TryFrom<&[u8]> for NotificationHandle {
    type Error = AdsNotificationHandleError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() != Self::LENGTH {
            return Err(AdsNotificationHandleError::UnexpectedLength {
                expected: Self::LENGTH,
                got: bytes.len(),
            });
        }
        Ok(Self([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }
}

impl fmt::Debug for NotificationHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NotificationHandle(0x{:08X})", self.as_u32())
    }
}

impl serde::Serialize for NotificationHandle {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u32(self.as_u32())
    }
}

impl<'de> serde::Deserialize<'de> for NotificationHandle {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(NotificationHandle::from(u32::deserialize(d)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_u32_roundtrip() {
        let handle = NotificationHandle::from(0x0000_001A_u32);
        assert_eq!(handle.as_u32(), 0x0000_001A);
        assert_eq!(u32::from(handle), 0x0000_001A);
    }

    #[test]
    fn test_from_bytes_roundtrip() {
        let bytes = [0x1A, 0x00, 0x00, 0x00];
        let handle = NotificationHandle::from_bytes(bytes);
        assert_eq!(handle.to_bytes(), bytes);
        assert_eq!(handle.as_u32(), 0x0000_001A);
    }

    #[test]
    fn test_try_from_slice_valid() {
        let bytes = [0x01, 0x02, 0x03, 0x04];
        let handle = NotificationHandle::try_from_slice(&bytes).unwrap();
        assert_eq!(handle.as_u32(), 0x04030201);
    }

    #[test]
    fn test_try_from_slice_too_long() {
        let bytes = [0x01, 0x02, 0x03, 0x04, 0xFF, 0xFF];
        let err = NotificationHandle::try_from_slice(&bytes).unwrap_err();
        assert!(matches!(
            err,
            AdsNotificationHandleError::UnexpectedLength {
                expected: 4,
                got: 6
            }
        ));
    }

    #[test]
    fn test_try_from_slice_too_short() {
        let err = NotificationHandle::try_from_slice(&[0x01, 0x02, 0x03]).unwrap_err();
        assert!(matches!(
            err,
            AdsNotificationHandleError::UnexpectedLength {
                expected: 4,
                got: 3
            }
        ));
    }

    #[test]
    fn test_equality_and_hash() {
        use std::collections::HashMap;

        let h1 = NotificationHandle::from(42_u32);
        let h2 = NotificationHandle::from(42_u32);
        let h3 = NotificationHandle::from(99_u32);

        assert_eq!(h1, h2);
        assert_ne!(h1, h3);

        let mut map = HashMap::new();
        map.insert(h1, "handler_a");
        assert_eq!(map[&h2], "handler_a");
        assert!(!map.contains_key(&h3));
    }

    #[test]
    fn test_serde_notification_handle_serialize() {
        let handle = NotificationHandle::from(42_u32);
        let s = serde_json::to_string(&handle).unwrap();
        assert_eq!(s, "42");
    }

    #[test]
    fn test_serde_notification_handle_deserialize() {
        let handle: NotificationHandle = serde_json::from_str("42").unwrap();
        assert_eq!(handle, NotificationHandle::from(42_u32));
    }

    #[test]
    fn test_serde_notification_handle_roundtrip() {
        let original = NotificationHandle::from(0x0000_001A_u32);
        let s = serde_json::to_string(&original).unwrap();
        let back: NotificationHandle = serde_json::from_str(&s).unwrap();
        assert_eq!(original, back);
    }
}
