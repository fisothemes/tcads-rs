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
