use super::super::ProtocolError;
use super::sample::{AdsNotificationSample, AdsNotificationSampleOwned};
use crate::ads::{AdsError, NotificationHandle, WindowsFileTime};

/// A zero-copy view of an ADS stamp header.
///
/// A stamp groups one or more [`AdsNotificationSample`]s that share the same
/// timestamp. The server may batch multiple variable changes that occurred at
/// the same scan cycle into a single stamp.
///
/// The `samples` field contains borrowed views into the originating
/// [`AmsFrame`](crate::io::AmsFrame) therefore, no data is copied. The `Vec` of sample
/// structs is allocated during parsing, but the variable-length data each sample
/// points to stays in the frame buffer.
///
/// For storage or use after the frame is dropped, convert to [`AdsStampHeaderOwned`]
/// via [`into_owned`](Self::into_owned) or [`to_owned`](Self::to_owned).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsStampHeader<'a> {
    timestamp: WindowsFileTime,
    samples: Vec<AdsNotificationSample<'a>>,
}

impl<'a> AdsStampHeader<'a> {
    /// Fixed size of the stamp header fields (Timestamp + Sample Count).
    pub const HEADER_SIZE: usize = 12;

    /// Parses one stamp from the front of `data`.
    ///
    /// Returns the parsed stamp and the remaining unparsed bytes, allowing
    /// the caller to parse stamps sequentially without knowing their lengths
    /// upfront.
    ///
    /// Wire layout per stamp:
    /// * Timestamp (8 bytes) - [`WindowsFileTime`]
    /// * Sample Count (4 bytes) - the number of samples that follow
    /// * Per sample:
    ///   * Handle (4 bytes) - [`NotificationHandle`]
    ///   * Sample Size (4 bytes) - length of the data that follows
    ///   * Data (n bytes)
    pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), ProtocolError> {
        if data.len() < Self::HEADER_SIZE {
            return Err(AdsError::UnexpectedDataLength {
                expected: Self::HEADER_SIZE,
                got: data.len(),
            })?;
        }

        let timestamp = WindowsFileTime::from_bytes(data[0..8].try_into().unwrap());
        let sample_count = u32::from_le_bytes(data[8..12].try_into().unwrap()) as usize;

        let mut samples = Vec::with_capacity(sample_count);
        let mut offset = Self::HEADER_SIZE;

        for _ in 0..sample_count {
            if data.len() < offset + AdsNotificationSample::MIN_SAMPLE_SIZE {
                return Err(AdsError::UnexpectedDataLength {
                    expected: offset + AdsNotificationSample::MIN_SAMPLE_SIZE,
                    got: data.len(),
                })?;
            }

            let handle = NotificationHandle::try_from_slice(
                &data[offset..offset + NotificationHandle::LENGTH],
            )
            .map_err(AdsError::from)?;

            offset += NotificationHandle::LENGTH;

            let sample_size =
                u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;

            offset += 4;

            if data.len() < offset + sample_size {
                return Err(AdsError::UnexpectedDataLength {
                    expected: offset + sample_size,
                    got: data.len(),
                })?;
            }

            let sample_data = &data[offset..offset + sample_size];

            offset += sample_size;

            samples.push(AdsNotificationSample::new(handle, sample_data));
        }

        Ok((Self { timestamp, samples }, &data[offset..]))
    }

    /// Returns the timestamp of this stamp group.
    pub fn timestamp(&self) -> WindowsFileTime {
        self.timestamp
    }

    /// Returns the samples in this stamp group.
    pub fn samples(&self) -> &[AdsNotificationSample<'a>] {
        &self.samples
    }

    /// Converts this view into an owned [`AdsStampHeaderOwned`], copying all sample data.
    pub fn into_owned(self) -> AdsStampHeaderOwned {
        AdsStampHeaderOwned {
            timestamp: self.timestamp,
            samples: self.samples.into_iter().map(|s| s.into_owned()).collect(),
        }
    }

    /// Clones this view into an owned [`AdsStampHeaderOwned`], copying all sample data.
    pub fn to_owned(&self) -> AdsStampHeaderOwned {
        AdsStampHeaderOwned {
            timestamp: self.timestamp,
            samples: self.samples.iter().map(|s| s.to_owned()).collect(),
        }
    }
}

/// A fully owned ADS stamp header.
///
/// Owns all sample data, making it suitable for storage, sending across channels,
/// or constructing outgoing [`AdsDeviceNotification`](super::AdsDeviceNotification)
/// frames on a server.
///
/// Obtain one by:
/// * Calling [`AdsStampHeaderOwned::new`] to construct a stamp to send.
/// * Calling [`AdsStampHeader::into_owned`] or [`AdsStampHeader::to_owned`] after parsing.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsStampHeaderOwned {
    timestamp: WindowsFileTime,
    samples: Vec<AdsNotificationSampleOwned>,
}

impl AdsStampHeaderOwned {
    /// Fixed size of the stamp header fields (Timestamp + Sample Count).
    pub const HEADER_SIZE: usize = AdsStampHeader::HEADER_SIZE;

    /// Creates a new owned stamp header.
    ///
    /// Use this on a **server** to construct notification stamps to send to a client.
    pub fn new(
        timestamp: WindowsFileTime,
        samples: impl Into<Vec<AdsNotificationSampleOwned>>,
    ) -> Self {
        Self {
            timestamp,
            samples: samples.into(),
        }
    }

    /// Returns the timestamp of this stamp group.
    pub fn timestamp(&self) -> WindowsFileTime {
        self.timestamp
    }

    /// Returns the samples in this stamp group.
    pub fn samples(&self) -> &[AdsNotificationSampleOwned] {
        &self.samples
    }

    /// Borrows this stamp as a zero-copy [`AdsStampHeader`].
    pub fn as_view(&self) -> AdsStampHeader<'_> {
        AdsStampHeader {
            timestamp: self.timestamp,
            samples: self.samples.iter().map(|s| s.as_view()).collect(),
        }
    }

    /// Returns the number of bytes this stamp occupies on the wire.
    ///
    /// Wire layout: Timestamp (8) + SampleCount (4) + sum of each sample's wire size.
    pub fn wire_size(&self) -> usize {
        AdsStampHeader::HEADER_SIZE + self.samples.iter().map(|s| s.wire_size()).sum::<usize>()
    }

    /// Serializes this stamp into `buf`.
    pub fn write_into(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.timestamp.to_bytes());
        buf.extend_from_slice(&(self.samples.len() as u32).to_le_bytes());
        for sample in &self.samples {
            sample.write_into(buf);
        }
    }
}

impl<'a> From<AdsStampHeader<'a>> for AdsStampHeaderOwned {
    fn from(value: AdsStampHeader<'a>) -> Self {
        value.into_owned()
    }
}

impl<'a> From<&'a AdsStampHeaderOwned> for AdsStampHeader<'a> {
    fn from(value: &'a AdsStampHeaderOwned) -> Self {
        value.as_view()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_handle(val: u32) -> NotificationHandle {
        NotificationHandle::from(val)
    }

    fn make_timestamp() -> WindowsFileTime {
        WindowsFileTime::from_raw(133_503_504_000_000_000) // 2024-01-15 12:00:00 UTC
    }

    /// Builds a raw stamp payload for testing.
    ///
    /// Layout: Timestamp (8) + SampleCount (4) + [Handle (4) + Size (4) + Data (n)] * samples
    fn build_stamp_bytes(
        timestamp: WindowsFileTime,
        samples: &[(NotificationHandle, &[u8])],
    ) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&timestamp.to_bytes());
        buf.extend_from_slice(&(samples.len() as u32).to_le_bytes());
        for (handle, data) in samples {
            buf.extend_from_slice(&handle.to_bytes());
            buf.extend_from_slice(&(data.len() as u32).to_le_bytes());
            buf.extend_from_slice(data);
        }
        buf
    }

    #[test]
    fn test_parse_single_sample() {
        let ts = make_timestamp();
        let handle = make_handle(42);
        let data = [0x01u8, 0x02, 0x03, 0x04];

        let bytes = build_stamp_bytes(ts, &[(handle, &data)]);
        let (stamp, remaining) = AdsStampHeader::parse(&bytes).expect("Should parse");

        assert_eq!(stamp.timestamp(), ts);
        assert_eq!(stamp.samples().len(), 1);
        assert_eq!(stamp.samples()[0].handle(), handle);
        assert_eq!(stamp.samples()[0].data(), &data);
        assert!(remaining.is_empty());
    }

    #[test]
    fn test_parse_multiple_samples() {
        let ts = make_timestamp();
        let h1 = make_handle(1);
        let h2 = make_handle(2);
        let d1 = [0x01u8, 0x02, 0x03, 0x04]; // DINT
        let d2 = [0x01u8]; // BOOL

        let bytes = build_stamp_bytes(ts, &[(h1, &d1), (h2, &d2)]);
        let (stamp, remaining) = AdsStampHeader::parse(&bytes).expect("Should parse");

        assert_eq!(stamp.samples().len(), 2);
        assert_eq!(stamp.samples()[0].handle(), h1);
        assert_eq!(stamp.samples()[0].data(), &d1);
        assert_eq!(stamp.samples()[1].handle(), h2);
        assert_eq!(stamp.samples()[1].data(), &d2);
        assert!(remaining.is_empty());
    }

    #[test]
    fn test_parse_returns_remaining_bytes() {
        let ts = make_timestamp();
        let handle = make_handle(1);
        let data = [0xAAu8; 4];

        let mut bytes = build_stamp_bytes(ts, &[(handle, &data)]);
        let trailing = [0xFFu8; 8]; // simulate a second stamp following
        bytes.extend_from_slice(&trailing);

        let (stamp, remaining) = AdsStampHeader::parse(&bytes).expect("Should parse");

        assert_eq!(stamp.samples().len(), 1);
        assert_eq!(remaining, &trailing);
    }

    #[test]
    fn test_parse_zero_samples() {
        let ts = make_timestamp();
        let bytes = build_stamp_bytes(ts, &[]);

        let (stamp, remaining) = AdsStampHeader::parse(&bytes).expect("Should parse");

        assert_eq!(stamp.timestamp(), ts);
        assert!(stamp.samples().is_empty());
        assert!(remaining.is_empty());
    }

    #[test]
    fn test_sample_data_points_into_original_bytes() {
        let ts = make_timestamp();
        let handle = make_handle(1);
        let data = [0x01u8, 0x02, 0x03, 0x04];
        let bytes = build_stamp_bytes(ts, &[(handle, &data)]);

        let (stamp, _) = AdsStampHeader::parse(&bytes).expect("Should parse");

        // Data offset: Timestamp (8) + SampleCount (4) + Handle (4) + SampleSize (4) = 20
        let expected_ptr = unsafe { bytes.as_ptr().add(20) };
        assert_eq!(stamp.samples()[0].data().as_ptr(), expected_ptr);
    }

    #[test]
    fn test_into_owned() {
        let ts = make_timestamp();
        let handle = make_handle(7);
        let data = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let bytes = build_stamp_bytes(ts, &[(handle, &data)]);

        let (stamp, _) = AdsStampHeader::parse(&bytes).expect("Should parse");
        let owned = stamp.into_owned();

        assert_eq!(owned.timestamp(), ts);
        assert_eq!(owned.samples().len(), 1);
        assert_eq!(owned.samples()[0].handle(), handle);
        assert_eq!(owned.samples()[0].data(), data.as_slice());
    }

    #[test]
    fn test_owned_as_view() {
        let ts = make_timestamp();
        let handle = make_handle(99);
        let data = vec![0x01u8, 0x02];

        let sample = AdsNotificationSampleOwned::new(handle, data.clone());
        let owned = AdsStampHeaderOwned::new(ts, vec![sample]);
        let view = owned.as_view();

        assert_eq!(view.timestamp(), ts);
        assert_eq!(view.samples().len(), 1);
        assert_eq!(view.samples()[0].data(), data.as_slice());
    }

    #[test]
    fn test_owned_wire_size() {
        let ts = make_timestamp();
        let handle = make_handle(1);
        let data = vec![0u8; 4];

        let sample = AdsNotificationSampleOwned::new(handle, data);
        let owned = AdsStampHeaderOwned::new(ts, vec![sample]);

        // Timestamp (8) + SampleCount (4) + Handle (4) + SampleSize (4) + Data (4) = 24
        assert_eq!(owned.wire_size(), 24);
    }

    #[test]
    fn test_owned_write_into_roundtrip() {
        let ts = make_timestamp();
        let handle = make_handle(0x0000_001A);
        let data = vec![0x01u8, 0x02, 0x03, 0x04];

        let sample = AdsNotificationSampleOwned::new(handle, data.clone());
        let owned = AdsStampHeaderOwned::new(ts, vec![sample]);

        let mut buf = Vec::new();
        owned.write_into(&mut buf);

        // Parse back and verify
        let (parsed, remaining) = AdsStampHeader::parse(&buf).expect("Should parse");
        assert!(remaining.is_empty());
        assert_eq!(parsed.timestamp(), ts);
        assert_eq!(parsed.samples()[0].handle(), handle);
        assert_eq!(parsed.samples()[0].data(), data.as_slice());
    }

    #[test]
    fn test_too_short_for_header() {
        let err = AdsStampHeader::parse(&[0u8; 8]).unwrap_err();
        assert!(matches!(
            err,
            ProtocolError::Ads(AdsError::UnexpectedDataLength {
                expected: 12,
                got: 8
            })
        ));
    }

    #[test]
    fn test_truncated_sample_data() {
        let ts = make_timestamp();
        let handle = make_handle(1);
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&ts.to_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes()); // 1 sample
        bytes.extend_from_slice(&handle.to_bytes());
        bytes.extend_from_slice(&100u32.to_le_bytes()); // claims 100 bytes
        bytes.extend_from_slice(&[0u8; 10]); // only 10 bytes present

        let err = AdsStampHeader::parse(&bytes).unwrap_err();
        assert!(matches!(
            err,
            ProtocolError::Ads(AdsError::UnexpectedDataLength { .. })
        ));
    }
}
