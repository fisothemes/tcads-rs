use super::error::AdsFileStatusError;
use super::file_attributes::AdsFileAttributes;
use super::filetime::WindowsFileTime;

/// A file's status, as returned by a
/// [`SYSTEM_SERVICE_FGETSTATUS`](super::IndexGroup::SYSTEM_SERVICE_FGETSTATUS)
/// request.
///
/// # Wire Format
///
/// | Offset | Size | Field            |
/// |--------|------|------------------|
/// | 0      | 8    | `size`           |
/// | 8      | 8    | `creation_time`  |
/// | 16     | 8    | `modified_time`  |
/// | 24     | 8    | `access_time`    |
/// | 32     | 4    | `attributes`     |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AdsFileStatus {
    size: u64,
    creation_time: WindowsFileTime,
    modified_time: WindowsFileTime,
    access_time: WindowsFileTime,
    attributes: AdsFileAttributes,
}

impl AdsFileStatus {
    /// Wire length of an [`AdsFileStatus`] in bytes.
    pub const LENGTH: usize = 36;

    /// Returns the file's size in bytes.
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Returns the file's creation time.
    pub fn creation_time(&self) -> WindowsFileTime {
        self.creation_time
    }

    /// Returns the file's last modified time.
    pub fn modified_time(&self) -> WindowsFileTime {
        self.modified_time
    }

    /// Returns the file's last access time.
    pub fn access_time(&self) -> WindowsFileTime {
        self.access_time
    }

    /// Returns the file's attributes.
    pub fn attributes(&self) -> AdsFileAttributes {
        self.attributes
    }

    /// Returns `true` if this entry is a directory.
    pub fn is_directory(&self) -> bool {
        self.attributes.is_directory()
    }

    /// Tries to parse an [`AdsFileStatus`] from a byte slice.
    pub fn try_from_slice(data: &[u8]) -> Result<Self, AdsFileStatusError> {
        if data.len() != Self::LENGTH {
            return Err(AdsFileStatusError::UnexpectedLength {
                expected: Self::LENGTH,
                got: data.len(),
            });
        }

        Ok(Self {
            size: u64::from_le_bytes([
                data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
            ]),
            creation_time: WindowsFileTime::from_bytes([
                data[8], data[9], data[10], data[11], data[12], data[13], data[14], data[15],
            ]),
            modified_time: WindowsFileTime::from_bytes([
                data[16], data[17], data[18], data[19], data[20], data[21], data[22], data[23],
            ]),
            access_time: WindowsFileTime::from_bytes([
                data[24], data[25], data[26], data[27], data[28], data[29], data[30], data[31],
            ]),
            attributes: AdsFileAttributes::from_bytes([data[32], data[33], data[34], data[35]]),
        })
    }
}

impl TryFrom<&[u8]> for AdsFileStatus {
    type Error = AdsFileStatusError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        Self::try_from_slice(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_all_fields() {
        let mut data = vec![0u8; AdsFileStatus::LENGTH];
        data[0..8].copy_from_slice(&123_456u64.to_le_bytes());
        data[8..16].copy_from_slice(&1_000u64.to_le_bytes());
        data[16..24].copy_from_slice(&2_000u64.to_le_bytes());
        data[24..32].copy_from_slice(&3_000u64.to_le_bytes());
        data[32..36].copy_from_slice(&0x0010u32.to_le_bytes()); // DIRECTORY

        let status = AdsFileStatus::try_from_slice(&data).unwrap();

        assert_eq!(status.size(), 123_456);
        assert_eq!(status.creation_time().as_raw(), 1_000);
        assert_eq!(status.modified_time().as_raw(), 2_000);
        assert_eq!(status.access_time().as_raw(), 3_000);
        assert!(status.is_directory());
    }

    #[test]
    fn rejects_short_payload() {
        let err = AdsFileStatus::try_from_slice(&[0u8; 10]).unwrap_err();
        assert!(matches!(
            err,
            AdsFileStatusError::UnexpectedLength {
                expected: 36,
                got: 10
            }
        ));
    }
}
