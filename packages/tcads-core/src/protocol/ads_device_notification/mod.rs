mod sample;
mod stamp;

pub use sample::{AdsNotificationSample, AdsNotificationSampleOwned};
pub use stamp::{AdsStampHeader, AdsStampHeaderOwned};

use super::{ProtocolError, parse_ads_frame};
use crate::ads::{
    AdsCommand, AdsError, AdsHeader, AdsReturnCode, InvokeId, StateFlag, WindowsFileTime,
};
use crate::ams::{AmsAddr, AmsCommand};
use crate::io::AmsFrame;

/// A zero-copy view of an ADS Device Notification (Command `0x0008`).
///
/// The server sends this whenever a watched variable meets the transmission
/// criteria of an active subscription registered via `AdsAddDeviceNotification`.
///
/// The stamp and sample structs borrow directly from the [`AmsFrame`] that was
/// parsed. The `Vec<AdsStampHeader>` and nested `Vec<AdsNotificationSample>` are
/// allocated during parsing (their counts are known from the wire), but all
/// variable-length sample data stays in the frame buffer insuring no copy is made.
///
/// For storage, channels, or server construction, convert to
/// [`AdsDeviceNotificationOwned`] via [`into_owned`](Self::into_owned) or
/// [`to_owned`](Self::to_owned).
///
/// # Usage
/// * **Server:** Sends this when a watched variable meets its notification criteria.
/// * **Client:** Receives this and dispatches samples by [`NotificationHandle`](crate::ads::NotificationHandle).
///
/// # Protocol Details
/// * **AMS Command:** [`AdsCommand`](AmsCommand::AdsCommand) (`0x0000`)
/// * **ADS Command:** [`AdsDeviceNotification`](AdsCommand::AdsDeviceNotification) (`0x0008`)
/// * **ADS Payload Layout:**
///   * **Length:** 4 bytes (u32) - total byte count of stamps data including stamps count (4 bytes).
///   * **Stamps:** 4 bytes (u32) - number of [`AdsStampHeader`] entries.
///   * Per stamp:
///     * **Timestamp:** 8 bytes ([`WindowsFileTime`])
///     * **Samples:** 4 bytes (u32) - number of [`AdsNotificationSample`] entries.
///     * Per sample:
///       * **Handle:** 4 bytes ([`NotificationHandle`](crate::ads::NotificationHandle))
///       * **Sample Size:** 4 bytes (u32)
///       * **Data:** n bytes
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsDeviceNotification<'a> {
    header: AdsHeader,
    stamps: Vec<AdsStampHeader<'a>>,
}

impl<'a> AdsDeviceNotification<'a> {
    /// Minimum size of the ADS payload (Length field + Stamps count).
    pub const MIN_PAYLOAD_SIZE: usize = 8;

    /// Tries to parse a notification from an AMS Frame.
    pub fn try_from_frame(frame: &'a AmsFrame) -> Result<Self, ProtocolError> {
        Self::try_from(frame)
    }

    /// Returns the ADS header.
    pub fn header(&self) -> &AdsHeader {
        &self.header
    }

    /// Returns the stamp groups in this notification.
    ///
    /// Each stamp groups samples that share the same server-side timestamp.
    /// Use [`iter_samples`](Self::iter_samples) if you don't care about stamp
    /// grouping and just want all samples with their timestamps.
    pub fn stamps(&self) -> &[AdsStampHeader<'a>] {
        &self.stamps
    }

    /// Returns a flattened iterator over all samples in this notification.
    ///
    /// Yields `(timestamp, sample)` pairs across all stamp groups, in order.
    /// This is the most convenient API for the common case of dispatching samples
    /// by handle regardless of which stamp group they belong to.
    ///
    /// # Example
    /// ```no_run
    /// # use tcads_core::protocol::{AdsDeviceNotification, AdsStampHeader, AdsNotificationSample};
    /// # use tcads_core::ams::AmsCommand;
    /// # use tcads_core::ads::NotificationHandle;
    /// # use tcads_core::io::AmsFrame;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let ncount_notif_handle = NotificationHandle::from(1);
    /// # let frame = AmsFrame::new(AmsCommand::AdsCommand, vec![]);
    /// # let notif = AdsDeviceNotification::try_from_frame(&frame)?;
    /// for (timestamp, sample) in notif.iter_samples() {
    ///     if sample.handle() == ncount_notif_handle {
    ///         let value = i32::from_le_bytes(sample.data().try_into()?);
    ///         println!("nCount = {value} at {timestamp}");
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn iter_samples(
        &self,
    ) -> impl Iterator<Item = (WindowsFileTime, &AdsNotificationSample<'a>)> {
        self.stamps.iter().flat_map(|stamp| {
            let ts = stamp.timestamp();
            stamp.samples().iter().map(move |s| (ts, s))
        })
    }

    /// Converts this view into an owned [`AdsDeviceNotificationOwned`],
    /// copying all sample data.
    pub fn into_owned(self) -> AdsDeviceNotificationOwned {
        AdsDeviceNotificationOwned {
            header: self.header,
            stamps: self.stamps.into_iter().map(|s| s.into_owned()).collect(),
        }
    }

    /// Clones this view into an owned [`AdsDeviceNotificationOwned`],
    /// copying all sample data.
    pub fn to_owned(&self) -> AdsDeviceNotificationOwned {
        AdsDeviceNotificationOwned {
            header: self.header.clone(),
            stamps: self.stamps.iter().map(|s| s.to_owned()).collect(),
        }
    }

    /// Parses the ADS payload portion.
    ///
    /// Returns the parsed stamps and validates the outer length field against
    /// the actual payload length.
    fn parse_payload(payload: &'a [u8]) -> Result<Vec<AdsStampHeader<'a>>, ProtocolError> {
        if payload.len() < Self::MIN_PAYLOAD_SIZE {
            return Err(AdsError::UnexpectedDataLength {
                expected: Self::MIN_PAYLOAD_SIZE,
                got: payload.len(),
            })?;
        }

        let length = u32::from_le_bytes(payload[0..4].try_into().unwrap()) as usize;
        let stamp_count = u32::from_le_bytes(payload[4..8].try_into().unwrap()) as usize;

        let stamps_data = &payload[Self::MIN_PAYLOAD_SIZE..];

        if stamps_data.len() != length {
            return Err(AdsError::UnexpectedDataLength {
                expected: Self::MIN_PAYLOAD_SIZE + length,
                got: payload.len(),
            })?;
        }

        let mut stamps = Vec::with_capacity(stamp_count);
        let mut remaining = stamps_data;

        for _ in 0..stamp_count {
            let (stamp, rest) = AdsStampHeader::parse(remaining)?;
            stamps.push(stamp);
            remaining = rest;
        }

        Ok(stamps)
    }
}

impl<'a> TryFrom<&'a AmsFrame> for AdsDeviceNotification<'a> {
    type Error = ProtocolError;

    fn try_from(value: &'a AmsFrame) -> Result<Self, Self::Error> {
        let (header, data) = parse_ads_frame(value, AdsCommand::AdsDeviceNotification, false)?;

        let stamps = Self::parse_payload(data)?;

        Ok(Self { header, stamps })
    }
}

/// A fully owned ADS Device Notification (Command `0x0008`).
///
/// Owns all stamp and sample data, making it suitable for storage, sending across
/// channels, or constructing outgoing notifications on a server.
///
/// Obtain one by:
/// * Calling [`AdsDeviceNotificationOwned::new`] to construct a notification to send.
/// * Calling [`AdsDeviceNotification::into_owned`] or [`AdsDeviceNotification::to_owned`]
///   after parsing.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsDeviceNotificationOwned {
    header: AdsHeader,
    stamps: Vec<AdsStampHeaderOwned>,
}

impl AdsDeviceNotificationOwned {
    /// Minimum size of the ADS payload (Length field + Stamps count).
    pub const MIN_PAYLOAD_SIZE: usize = AdsDeviceNotification::MIN_PAYLOAD_SIZE;

    /// Creates a new owned Device Notification.
    ///
    /// Use this on a **server** to construct a notification to push to a client.
    pub fn new(
        target: AmsAddr,
        source: AmsAddr,
        stamps: impl Into<Vec<AdsStampHeaderOwned>>,
    ) -> Self {
        Self::with_invoke_id(target, source, 0, stamps)
    }

    /// Creates a new owned Device Notification with the given invoke ID.
    ///
    /// Use this on a **server** to construct a notification to push to a client.
    pub fn with_invoke_id(
        target: AmsAddr,
        source: AmsAddr,
        invoke_id: InvokeId,
        stamps: impl Into<Vec<AdsStampHeaderOwned>>,
    ) -> Self {
        let stamps = stamps.into();

        let stamps_wire_size: usize = stamps.iter().map(|s| s.wire_size()).sum();
        let ads_payload_size = Self::MIN_PAYLOAD_SIZE + stamps_wire_size;

        let header = AdsHeader::new(
            target,
            source,
            AdsCommand::AdsDeviceNotification,
            StateFlag::tcp_ads_response(),
            ads_payload_size as u32,
            AdsReturnCode::Ok,
            invoke_id,
        );

        Self { header, stamps }
    }

    /// Returns the ADS header.
    pub fn header(&self) -> &AdsHeader {
        &self.header
    }

    /// Returns the stamp groups in this notification.
    pub fn stamps(&self) -> &[AdsStampHeaderOwned] {
        &self.stamps
    }

    /// Returns the total wire size of all stamps in bytes.
    pub fn stamps_wire_size(&self) -> usize {
        self.stamps.iter().map(|s| s.wire_size()).sum()
    }

    /// Returns a flattened iterator over all samples in this notification.
    ///
    /// Yields `(timestamp, sample)` pairs across all stamp groups, in order.
    pub fn iter_samples(
        &self,
    ) -> impl Iterator<Item = (WindowsFileTime, &AdsNotificationSampleOwned)> {
        self.stamps.iter().flat_map(|stamp| {
            let ts = stamp.timestamp();
            stamp.samples().iter().map(move |s| (ts, s))
        })
    }

    /// Borrows this notification as a zero-copy [`AdsDeviceNotification`].
    pub fn as_view(&self) -> AdsDeviceNotification<'_> {
        AdsDeviceNotification {
            header: self.header.clone(),
            stamps: self.stamps.iter().map(|s| s.as_view()).collect(),
        }
    }

    /// Consumes the notification and converts it into an AMS Frame.
    pub fn into_frame(self) -> AmsFrame {
        AmsFrame::from(&self)
    }

    /// Serializes the notification into an AMS Frame.
    pub fn to_frame(&self) -> AmsFrame {
        AmsFrame::from(self)
    }
}

impl From<&AdsDeviceNotificationOwned> for AmsFrame {
    fn from(value: &AdsDeviceNotificationOwned) -> Self {
        let stamps_wire_size: usize = value.stamps_wire_size();
        let ads_payload_size = AdsDeviceNotificationOwned::MIN_PAYLOAD_SIZE + stamps_wire_size;

        let mut payload = Vec::with_capacity(AdsHeader::LENGTH + ads_payload_size);

        payload.extend_from_slice(&value.header.to_bytes());
        payload.extend_from_slice(&(stamps_wire_size as u32).to_le_bytes());
        payload.extend_from_slice(&(value.stamps.len() as u32).to_le_bytes());

        for stamp in &value.stamps {
            stamp.write_into(&mut payload);
        }

        AmsFrame::new(AmsCommand::AdsCommand, payload)
    }
}

impl From<AdsDeviceNotificationOwned> for AmsFrame {
    fn from(value: AdsDeviceNotificationOwned) -> Self {
        AmsFrame::from(&value)
    }
}

impl<'a> From<AdsDeviceNotification<'a>> for AdsDeviceNotificationOwned {
    fn from(value: AdsDeviceNotification<'a>) -> Self {
        value.into_owned()
    }
}

impl<'a> From<&'a AdsDeviceNotificationOwned> for AdsDeviceNotification<'a> {
    fn from(value: &'a AdsDeviceNotificationOwned) -> Self {
        value.as_view()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ads::NotificationHandle;
    use crate::ams::AmsNetId;

    fn make_addrs() -> (AmsAddr, AmsAddr) {
        let target = AmsAddr::new(AmsNetId::new(192, 168, 0, 1, 1, 1), 851);
        let source = AmsAddr::new(AmsNetId::new(172, 16, 0, 1, 1, 1), 30000);
        (target, source)
    }

    fn make_handle(val: u32) -> NotificationHandle {
        NotificationHandle::from(val)
    }

    fn make_timestamp() -> WindowsFileTime {
        WindowsFileTime::from_raw(133_503_504_000_000_000)
    }

    fn make_owned_notification(
        target: AmsAddr,
        source: AmsAddr,
        stamps: Vec<AdsStampHeaderOwned>,
    ) -> AdsDeviceNotificationOwned {
        AdsDeviceNotificationOwned::new(target, source, stamps)
    }

    #[test]
    fn test_single_stamp_single_sample_roundtrip() {
        let (target, source) = make_addrs();
        let handle = make_handle(42);
        let ts = make_timestamp();
        let data = 1234_i32.to_le_bytes().to_vec();

        let sample = AdsNotificationSampleOwned::new(handle, data.clone());
        let stamp = AdsStampHeaderOwned::new(ts, vec![sample]);
        let owned = make_owned_notification(target, source, vec![stamp]);
        let frame = owned.to_frame();

        let view = AdsDeviceNotification::try_from(&frame).expect("Should parse");

        assert_eq!(view.stamps().len(), 1);
        assert_eq!(view.stamps()[0].timestamp(), ts);
        assert_eq!(view.stamps()[0].samples().len(), 1);
        assert_eq!(view.stamps()[0].samples()[0].handle(), handle);
        assert_eq!(view.stamps()[0].samples()[0].data(), data.as_slice());
        assert!(view.header().state_flags().is_response());
    }

    #[test]
    fn test_multiple_stamps_multiple_samples() {
        let (target, source) = make_addrs();

        let h1 = make_handle(1);
        let h2 = make_handle(2);
        let h3 = make_handle(3);
        let ts1 = WindowsFileTime::from_raw(133_503_504_000_000_000);
        let ts2 = WindowsFileTime::from_raw(133_503_504_100_000_000); // 10s later

        let stamp1 = AdsStampHeaderOwned::new(
            ts1,
            vec![
                AdsNotificationSampleOwned::new(h1, 100_i32.to_le_bytes().to_vec()),
                AdsNotificationSampleOwned::new(h2, vec![0x01u8]), // BOOL true
            ],
        );
        let stamp2 = AdsStampHeaderOwned::new(
            ts2,
            vec![AdsNotificationSampleOwned::new(
                h3,
                200_i32.to_le_bytes().to_vec(),
            )],
        );

        let owned = make_owned_notification(target, source, vec![stamp1, stamp2]);
        let frame = owned.to_frame();

        let view = AdsDeviceNotification::try_from(&frame).expect("Should parse");

        assert_eq!(view.stamps().len(), 2);
        assert_eq!(view.stamps()[0].samples().len(), 2);
        assert_eq!(view.stamps()[1].samples().len(), 1);
        assert_eq!(view.stamps()[1].samples()[0].handle(), h3);
    }

    #[test]
    fn test_iter_samples_flattens_stamps() {
        let (target, source) = make_addrs();

        let h1 = make_handle(1);
        let h2 = make_handle(2);
        let h3 = make_handle(3);
        let ts1 = make_timestamp();
        let ts2 = WindowsFileTime::from_raw(ts1.as_raw() + 10_000_000);

        let stamp1 = AdsStampHeaderOwned::new(
            ts1,
            vec![
                AdsNotificationSampleOwned::new(h1, vec![1, 0, 0, 0]),
                AdsNotificationSampleOwned::new(h2, vec![0x01]),
            ],
        );
        let stamp2 = AdsStampHeaderOwned::new(
            ts2,
            vec![AdsNotificationSampleOwned::new(h3, vec![2, 0, 0, 0])],
        );

        let owned = make_owned_notification(target, source, vec![stamp1, stamp2]);
        let frame = owned.to_frame();
        let view = AdsDeviceNotification::try_from(&frame).expect("Should parse");

        let samples: Vec<_> = view.iter_samples().collect();

        assert_eq!(samples.len(), 3);
        assert_eq!(samples[0].0, ts1);
        assert_eq!(samples[0].1.handle(), h1);
        assert_eq!(samples[1].0, ts1);
        assert_eq!(samples[1].1.handle(), h2);
        assert_eq!(samples[2].0, ts2);
        assert_eq!(samples[2].1.handle(), h3);
    }

    #[test]
    fn test_iter_samples_dispatch_by_handle() {
        // Models the real usage pattern; dispatch by handle
        let (target, source) = make_addrs();

        let handle_ncount = make_handle(1);
        let handle_bflag = make_handle(2);
        let ts = make_timestamp();

        let stamp = AdsStampHeaderOwned::new(
            ts,
            vec![
                AdsNotificationSampleOwned::new(handle_ncount, 42_i32.to_le_bytes().to_vec()),
                AdsNotificationSampleOwned::new(handle_bflag, vec![0x01]),
            ],
        );

        let owned = make_owned_notification(target, source, vec![stamp]);
        let frame = owned.to_frame();
        let view = AdsDeviceNotification::try_from(&frame).expect("Should parse");

        let mut saw_ncount = false;
        let mut saw_bflag = false;

        for (_, sample) in view.iter_samples() {
            match sample.handle() {
                h if h == handle_ncount => {
                    let val = i32::from_le_bytes(sample.data().try_into().unwrap());
                    assert_eq!(val, 42);
                    saw_ncount = true;
                }
                h if h == handle_bflag => {
                    assert_eq!(sample.data(), &[0x01]);
                    saw_bflag = true;
                }
                _ => panic!("Unexpected handle"),
            }
        }

        assert!(saw_ncount);
        assert!(saw_bflag);
    }

    #[test]
    fn test_sample_data_zero_copy() {
        let (target, source) = make_addrs();
        let handle = make_handle(1);
        let ts = make_timestamp();
        let data = vec![0x01u8, 0x02, 0x03, 0x04];

        let sample = AdsNotificationSampleOwned::new(handle, data.clone());
        let stamp = AdsStampHeaderOwned::new(ts, vec![sample]);
        let owned = make_owned_notification(target, source, vec![stamp]);
        let frame = owned.to_frame();

        let view = AdsDeviceNotification::try_from(&frame).expect("Should parse");

        let sample_data_ptr = view.stamps()[0].samples()[0].data().as_ptr();
        let frame_payload_ptr = frame.payload().as_ptr();

        // Data lives inside the frame payload, not in a separate allocation
        let frame_payload_end = unsafe { frame_payload_ptr.add(frame.payload().len()) };
        assert!(sample_data_ptr >= frame_payload_ptr);
        assert!(sample_data_ptr < frame_payload_end);
    }

    #[test]
    fn test_into_owned_roundtrip() {
        let (target, source) = make_addrs();
        let handle = make_handle(7);
        let ts = make_timestamp();
        let data = vec![0xDE, 0xAD, 0xBE, 0xEF];

        let sample = AdsNotificationSampleOwned::new(handle, data.clone());
        let stamp = AdsStampHeaderOwned::new(ts, vec![sample]);
        let original = make_owned_notification(target, source, vec![stamp]);
        let frame = original.to_frame();

        let view = AdsDeviceNotification::try_from(&frame).expect("Should parse");
        let roundtripped = view.into_owned();

        assert_eq!(roundtripped.stamps().len(), 1);
        assert_eq!(roundtripped.stamps()[0].timestamp(), ts);
        assert_eq!(roundtripped.stamps()[0].samples()[0].handle(), handle);
        assert_eq!(
            roundtripped.stamps()[0].samples()[0].data(),
            data.as_slice()
        );
    }

    #[test]
    fn test_owned_iter_samples() {
        let (target, source) = make_addrs();
        let h1 = make_handle(1);
        let h2 = make_handle(2);
        let ts = make_timestamp();

        let stamp = AdsStampHeaderOwned::new(
            ts,
            vec![
                AdsNotificationSampleOwned::new(h1, vec![1, 0, 0, 0]),
                AdsNotificationSampleOwned::new(h2, vec![0x01]),
            ],
        );
        let owned = make_owned_notification(target, source, vec![stamp]);

        let samples: Vec<_> = owned.iter_samples().collect();
        assert_eq!(samples.len(), 2);
        assert_eq!(samples[0].1.handle(), h1);
        assert_eq!(samples[1].1.handle(), h2);
    }

    #[test]
    fn test_large_notification_no_copy() {
        // Simulates an array-of-structs notification; large sample data
        let (target, source) = make_addrs();
        let handle = make_handle(1);
        let ts = make_timestamp();
        let large_data = vec![0xAAu8; 16_384]; // 16KB struct array

        let sample = AdsNotificationSampleOwned::new(handle, large_data.clone());
        let stamp = AdsStampHeaderOwned::new(ts, vec![sample]);
        let owned = make_owned_notification(target, source, vec![stamp]);
        let frame = owned.to_frame();

        let view = AdsDeviceNotification::try_from(&frame).expect("Should parse");
        let sample_data = view.stamps()[0].samples()[0].data();

        assert_eq!(sample_data.len(), 16_384);
        assert_eq!(sample_data, large_data.as_slice());

        // Zero-copy: data pointer is inside the frame payload
        let frame_payload_ptr = frame.payload().as_ptr();
        let frame_payload_end = unsafe { frame_payload_ptr.add(frame.payload().len()) };
        assert!(sample_data.as_ptr() >= frame_payload_ptr);
        assert!(sample_data.as_ptr() < frame_payload_end);
    }

    #[test]
    fn test_empty_notification() {
        let (target, source) = make_addrs();

        let owned = make_owned_notification(target, source, vec![]);
        let frame = owned.to_frame();

        let view = AdsDeviceNotification::try_from(&frame).expect("Should parse");
        assert!(view.stamps().is_empty());
        assert_eq!(view.iter_samples().count(), 0);
    }

    #[test]
    fn test_wrong_command_rejected() {
        let (target, source) = make_addrs();

        let read_state = super::super::AdsReadStateRequest::new(target, source, 1);
        let frame = read_state.to_frame();

        let err = AdsDeviceNotification::try_from(&frame).unwrap_err();
        assert!(matches!(
            err,
            ProtocolError::UnexpectedAdsCommand {
                expected: AdsCommand::AdsDeviceNotification,
                ..
            }
        ));
    }

    #[test]
    fn test_owned_as_view_as_frame_roundtrip() {
        let (target, source) = make_addrs();
        let handle = make_handle(5);
        let ts = make_timestamp();
        let data = vec![0x11u8, 0x22, 0x33, 0x44];

        let stamp = AdsStampHeaderOwned::new(
            ts,
            vec![AdsNotificationSampleOwned::new(handle, data.clone())],
        );
        let owned = make_owned_notification(target, source, vec![stamp]);
        let frame = owned.to_frame();

        let view = AdsDeviceNotification::try_from(&frame).expect("Should parse");
        let back = view.into_owned();
        let frame2 = back.to_frame();

        // Both frames should produce identical bytes
        assert_eq!(frame.payload(), frame2.payload());
    }
}
