use crate::ads::{NotificationHandle, WindowsFileTime};

/// A zero-copy view of a single ADS notification sample.
///
/// Each sample represents the value of a watched variable at a point in time,
/// identified by the [`NotificationHandle`] that was assigned when the subscription
/// was created via `AdsAddDeviceNotification`.
///
/// Samples are always parsed as part of an [`AdsStampHeader`](super::stamp::AdsStampHeader),
/// which groups samples that share the same timestamp. The `data` field borrows
/// directly from the [`AmsFrame`](crate::AmsFrame) that was parsed, thus no copy is made.
///
/// This view intentionally does not carry its stamp's timestamp: it represents only
/// the wire sub-record for the sample itself. Use
/// [`AdsDeviceNotification::iter_samples`](super::AdsDeviceNotification::iter_samples) to get
/// `(timestamp, sample)` pairs, or see [`AdsNotificationSampleOwned::timestamp`] for the owned
/// equivalent used by client notification channels.
///
/// For storage or use after the frame is dropped, convert to [`AdsNotificationSampleOwned`]
/// via [`into_owned`](Self::into_owned) or [`to_owned`](Self::to_owned).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsNotificationSample<'a> {
    handle: NotificationHandle,
    data: &'a [u8],
}

impl<'a> AdsNotificationSample<'a> {
    /// The minimum number of bytes a sample occupies on the wire: Handle (4) + Sample Size (4).
    pub const MIN_SAMPLE_SIZE: usize = NotificationHandle::LENGTH + 4;

    /// Creates a new `AdsNotificationSample` from a handle and a data slice.
    ///
    /// The data slice must borrow from the frame that contains this sample.
    pub fn new(handle: NotificationHandle, data: &'a [u8]) -> Self {
        Self { handle, data }
    }

    /// Returns the [`NotificationHandle`] identifying the subscription this sample
    /// belongs to.
    ///
    /// Use this to dispatch the sample to the correct handler, e.g. as a key into
    /// a `HashMap<NotificationHandle, _>`.
    pub fn handle(&self) -> NotificationHandle {
        self.handle
    }

    /// Returns the size of the sample data.
    pub fn sample_size(&self) -> usize {
        self.data.len()
    }

    /// Returns a zero-copy slice of the sample data.
    ///
    /// The slice borrows from the originating [`AmsFrame`](crate::AmsFrame) —
    /// interpret it according to the data type of the watched variable.
    pub fn data(&self) -> &[u8] {
        self.data
    }

    /// Converts this view into an owned [`AdsNotificationSampleOwned`], copying the data.
    ///
    /// The result has no [`timestamp`](AdsNotificationSampleOwned::timestamp): this view does
    /// not carry one. Attach one with
    /// [`with_timestamp`](AdsNotificationSampleOwned::with_timestamp) if the caller has one from
    /// the enclosing stamp header (see [`AdsDeviceNotification::iter_samples`](super::AdsDeviceNotification::iter_samples)).
    pub fn into_owned(self) -> AdsNotificationSampleOwned {
        AdsNotificationSampleOwned {
            handle: self.handle,
            data: self.data.to_vec(),
            timestamp: None,
        }
    }

    /// Clones this view into an owned [`AdsNotificationSampleOwned`], copying the data.
    ///
    /// See [`into_owned`](Self::into_owned) for timestamp handling.
    pub fn to_owned(&self) -> AdsNotificationSampleOwned {
        AdsNotificationSampleOwned {
            handle: self.handle,
            data: self.data.to_vec(),
            timestamp: None,
        }
    }
}

/// A fully owned ADS notification sample.
///
/// Owns its data buffer, making it suitable for storage, sending across channels,
/// or constructing outgoing [`AdsDeviceNotification`](super::AdsDeviceNotification)
/// frames on a server.
///
/// Obtain one by:
/// * Calling [`AdsNotificationSampleOwned::new`] to construct a sample to send.
/// * Calling [`AdsNotificationSample::into_owned`] or [`AdsNotificationSample::to_owned`]
///   after parsing.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsNotificationSampleOwned {
    handle: NotificationHandle,
    data: Vec<u8>,
    timestamp: Option<WindowsFileTime>,
}

impl AdsNotificationSampleOwned {
    /// The minimum number of bytes a sample occupies on the wire: Handle (4) + Sample Size (4).
    pub const MIN_SAMPLE_SIZE: usize = AdsNotificationSample::MIN_SAMPLE_SIZE;

    /// Creates a new owned notification sample with no timestamp attached.
    ///
    /// Use this on a **server** to construct notification samples to send to a client — the
    /// timestamp for an outgoing sample belongs to its enclosing
    /// [`AdsStampHeaderOwned`](super::stamp::AdsStampHeaderOwned), not the sample itself.
    ///
    /// A **client** receiving samples from a live notification subscription gets a timestamp
    /// via [`timestamp`](Self::timestamp); see [`with_timestamp`](Self::with_timestamp).
    pub fn new(handle: NotificationHandle, data: impl Into<Vec<u8>>) -> Self {
        Self {
            handle,
            data: data.into(),
            timestamp: None,
        }
    }

    /// Returns the [`NotificationHandle`] identifying the subscription this sample
    /// belongs to.
    pub fn handle(&self) -> NotificationHandle {
        self.handle
    }

    /// Returns the PLC-side generation time of this sample.
    ///
    /// This is the timestamp of the [`AdsStampHeader`](super::stamp::AdsStampHeader) that
    /// contained this sample when an incoming [`AdsDeviceNotification`](super::AdsDeviceNotification)
    /// was parsed and dispatched to a notification channel — see
    /// [`with_timestamp`](Self::with_timestamp).
    ///
    /// `None` for samples that were never attached to a parsed stamp, such as those
    /// constructed directly with [`new`](Self::new) for an outgoing server frame, or test
    /// fixtures. Every sample delivered through a live client subscription has `Some` value,
    /// including the (unlikely) case where the device itself reports a raw tick count of zero.
    pub fn timestamp(&self) -> Option<WindowsFileTime> {
        self.timestamp
    }

    /// Returns a copy of this sample with `timestamp` attached.
    ///
    /// Intended for the client reader to call immediately after
    /// [`AdsDeviceNotification::iter_samples`](super::AdsDeviceNotification::iter_samples) yields
    /// a `(timestamp, sample)` pair, so the sample's PLC-side generation time survives the hop
    /// through a notification channel. Most callers will not need this directly.
    pub fn with_timestamp(mut self, timestamp: WindowsFileTime) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    /// Returns the size of the sample data.
    pub fn sample_size(&self) -> usize {
        self.data.len()
    }

    /// Returns the number of bytes this sample occupies on the wire.
    ///
    /// Wire layout: Handle (4) + Sample Size (4) + Data (n).
    pub fn wire_size(&self) -> usize {
        AdsNotificationSample::MIN_SAMPLE_SIZE + self.sample_size()
    }

    /// Returns the sample data.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Splits the notification sample into a [`NotificationHandle`] and its [`data`](Vec<u8>) part.
    ///
    /// Does not include [`timestamp`](Self::timestamp): that is metadata attached after
    /// parsing, not part of the sample's own wire sub-record.
    pub fn into_parts(self) -> (NotificationHandle, Vec<u8>) {
        (self.handle, self.data)
    }

    /// Serializes this sample into `buf`.
    ///
    /// Does not serialize [`timestamp`](Self::timestamp): on the wire the timestamp belongs to
    /// the enclosing [`AdsStampHeaderOwned`](super::stamp::AdsStampHeaderOwned), which writes it
    /// once for every sample in the group.
    pub fn write_into(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.handle.to_bytes());
        buf.extend_from_slice(&(self.data.len() as u32).to_le_bytes());
        buf.extend_from_slice(&self.data);
    }

    /// Borrows this sample as a zero-copy [`AdsNotificationSample`].
    pub fn as_view(&self) -> AdsNotificationSample<'_> {
        AdsNotificationSample {
            handle: self.handle,
            data: &self.data,
        }
    }
}

impl<'a> From<AdsNotificationSample<'a>> for AdsNotificationSampleOwned {
    fn from(value: AdsNotificationSample<'a>) -> Self {
        value.into_owned()
    }
}

impl<'a> From<&'a AdsNotificationSampleOwned> for AdsNotificationSample<'a> {
    fn from(value: &'a AdsNotificationSampleOwned) -> Self {
        value.as_view()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_handle(val: u32) -> NotificationHandle {
        NotificationHandle::from(val)
    }

    #[test]
    fn test_borrowed_accessors() {
        let handle = make_handle(42);
        let data = [0x01u8, 0x02, 0x03, 0x04];
        let sample = AdsNotificationSample::new(handle, &data);

        assert_eq!(sample.handle(), handle);
        assert_eq!(sample.data(), &data);
    }

    #[test]
    fn test_into_owned() {
        let handle = make_handle(42);
        let data = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let sample = AdsNotificationSample::new(handle, &data);

        let owned = sample.into_owned();
        assert_eq!(owned.handle(), handle);
        assert_eq!(owned.data(), data.as_slice());
    }

    #[test]
    fn test_to_owned() {
        let handle = make_handle(7);
        let data = vec![0x01, 0x02];
        let sample = AdsNotificationSample::new(handle, &data);

        let owned = sample.to_owned();
        assert_eq!(owned.handle(), handle);
        assert_eq!(owned.data(), data.as_slice());

        // Original still usable after to_owned
        assert_eq!(sample.handle(), handle);
    }

    #[test]
    fn test_owned_as_view() {
        let handle = make_handle(99);
        let data = vec![0xAA, 0xBB, 0xCC];

        let owned = AdsNotificationSampleOwned::new(handle, data.clone());
        let view = owned.as_view();

        assert_eq!(view.handle(), handle);
        assert_eq!(view.data(), data.as_slice());
    }

    #[test]
    fn test_from_impls() {
        let handle = make_handle(1);
        let data = vec![1, 2, 3];

        let owned = AdsNotificationSampleOwned::new(handle, data.clone());

        // &Owned -> Borrowed
        let view: AdsNotificationSample<'_> = AdsNotificationSample::from(&owned);
        assert_eq!(view.data(), data.as_slice());

        // Borrowed -> Owned
        let back: AdsNotificationSampleOwned = AdsNotificationSampleOwned::from(view);
        assert_eq!(back.data(), data.as_slice());
    }

    #[test]
    fn test_owned_wire_size() {
        let handle = make_handle(1);
        let data = vec![0u8; 8];

        let owned = AdsNotificationSampleOwned::new(handle, data);
        // Handle (4) + SampleSize (4) + Data (8) = 16
        assert_eq!(owned.wire_size(), 16);
    }

    #[test]
    fn test_owned_write_into() {
        let handle = make_handle(0x0000_001A);
        let data = vec![0x01u8, 0x02, 0x03, 0x04];

        let owned = AdsNotificationSampleOwned::new(handle, data.clone());
        let mut buf = Vec::new();
        owned.write_into(&mut buf);

        // Handle LE bytes
        assert_eq!(&buf[0..4], &[0x1A, 0x00, 0x00, 0x00]);
        // SampleSize LE = 4
        assert_eq!(&buf[4..8], &[0x04, 0x00, 0x00, 0x00]);
        // Data
        assert_eq!(&buf[8..12], data.as_slice());
    }

    #[test]
    fn test_zero_copy_data_pointer() {
        // The borrowed sample's data should point into the original slice, not a copy
        let data = vec![0x01u8, 0x02, 0x03, 0x04];
        let handle = make_handle(1);
        let sample = AdsNotificationSample::new(handle, &data);

        assert_eq!(sample.data().as_ptr(), data.as_ptr());
    }

    #[test]
    fn test_large_sample_no_copy() {
        let handle = make_handle(1);
        let large_data = vec![0xFFu8; 16_384]; // 16KB struct array
        let sample = AdsNotificationSample::new(handle, &large_data);

        // Pointer equality proves no copy
        assert_eq!(sample.data().as_ptr(), large_data.as_ptr());
        assert_eq!(sample.data().len(), 16_384);
    }

    #[test]
    fn test_handle_as_hashmap_key() {
        use std::collections::HashMap;

        let h1 = make_handle(1);
        let h2 = make_handle(2);
        let data = vec![0u8; 4];

        let s1 = AdsNotificationSampleOwned::new(h1, data.clone());
        let s2 = AdsNotificationSampleOwned::new(h2, data.clone());

        let mut handlers: HashMap<NotificationHandle, &str> = HashMap::new();
        handlers.insert(h1, "nCount handler");
        handlers.insert(h2, "bIncrement handler");

        assert_eq!(handlers[&s1.handle()], "nCount handler");
        assert_eq!(handlers[&s2.handle()], "bIncrement handler");
    }

    #[test]
    fn test_new_has_no_timestamp() {
        let owned = AdsNotificationSampleOwned::new(make_handle(1), vec![0u8; 4]);
        assert_eq!(owned.timestamp(), None);
    }

    #[test]
    fn test_with_timestamp_sets_value() {
        let ts = WindowsFileTime::from_raw(133_503_504_000_000_000);
        let owned =
            AdsNotificationSampleOwned::new(make_handle(1), vec![0u8; 4]).with_timestamp(ts);
        assert_eq!(owned.timestamp(), Some(ts));
    }

    #[test]
    fn test_zero_timestamp_is_some_not_none() {
        // A raw tick count of zero is a real (if implausible) timestamp, not "missing".
        let ts = WindowsFileTime::from_raw(0);
        let owned =
            AdsNotificationSampleOwned::new(make_handle(1), vec![0u8; 4]).with_timestamp(ts);
        assert_eq!(owned.timestamp(), Some(ts));
    }

    #[test]
    fn test_view_conversions_do_not_invent_timestamp() {
        let data = vec![0x01u8, 0x02, 0x03, 0x04];
        let handle = make_handle(1);

        let view = AdsNotificationSample::new(handle, &data);
        assert_eq!(view.clone().into_owned().timestamp(), None);
        assert_eq!(view.to_owned().timestamp(), None);
    }

    #[test]
    fn test_as_view_does_not_carry_timestamp() {
        let ts = WindowsFileTime::from_raw(133_503_504_000_000_000);
        let owned =
            AdsNotificationSampleOwned::new(make_handle(1), vec![0u8; 4]).with_timestamp(ts);

        // The view type has no timestamp field at all; it represents only the wire
        // sub-record for the sample itself.
        let view = owned.as_view();
        assert_eq!(view.data(), owned.data());
    }

    #[test]
    fn test_write_into_is_unaffected_by_timestamp() {
        let handle = make_handle(0x0000_001A);
        let data = vec![0x01u8, 0x02, 0x03, 0x04];
        let ts = WindowsFileTime::from_raw(133_503_504_000_000_000);

        let without_ts = AdsNotificationSampleOwned::new(handle, data.clone());
        let with_ts = without_ts.clone().with_timestamp(ts);

        let mut buf_without = Vec::new();
        without_ts.write_into(&mut buf_without);
        let mut buf_with = Vec::new();
        with_ts.write_into(&mut buf_with);

        assert_eq!(buf_without, buf_with);
    }

    #[test]
    fn test_into_parts_drops_timestamp() {
        let handle = make_handle(1);
        let data = vec![0x01u8, 0x02, 0x03, 0x04];
        let ts = WindowsFileTime::from_raw(133_503_504_000_000_000);

        let owned = AdsNotificationSampleOwned::new(handle, data.clone()).with_timestamp(ts);
        let (parts_handle, parts_data) = owned.into_parts();

        assert_eq!(parts_handle, handle);
        assert_eq!(parts_data, data);
    }
}
