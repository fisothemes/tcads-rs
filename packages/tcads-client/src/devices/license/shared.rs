use tcads_core::{AdsError, AdsLicenseOnlineInfo};

/// Parses a contiguous buffer of back-to-back
/// [`AdsLicenseOnlineInfo`] entries.
///
/// `data` must be an exact multiple of
/// [`AdsLicenseOnlineInfo::LENGTH`]; this is the
/// shape returned by a [`LICENSE_ONLINE_INFO`](tcads_core::IndexGroup::LICENSE_ONLINE_INFO)
/// read once the entry count is known.
pub fn parse_online_info_entries(data: &[u8]) -> crate::Result<Vec<AdsLicenseOnlineInfo>> {
    let remainder = data.len() % AdsLicenseOnlineInfo::LENGTH;
    if remainder != 0 {
        return Err(AdsError::UnexpectedDataLength {
            expected: data.len() + (AdsLicenseOnlineInfo::LENGTH - remainder),
            got: data.len(),
        }
        .into());
    }

    data.chunks_exact(AdsLicenseOnlineInfo::LENGTH)
        .map(|chunk| {
            AdsLicenseOnlineInfo::try_from_slice(chunk).map_err(|e| AdsError::from(e).into())
        })
        .collect()
}

/// Decodes a little-endian `u32` from the start of `data`.
pub fn decode_u32_le(data: &[u8]) -> crate::Result<u32> {
    if data.len() != 4 {
        return Err(AdsError::UnexpectedDataLength {
            expected: 4,
            got: data.len(),
        }
        .into());
    }

    Ok(u32::from_le_bytes([data[0], data[1], data[2], data[3]]))
}

/// Decodes a null-terminated string from a fixed-size buffer, stopping at the first null.
///
/// The license server returns names and order numbers in a buffer larger than the string.
/// Anything after the first null is padding or leftover bytes, not part of the value.
pub fn c_string(data: &[u8]) -> String {
    let end = data.iter().position(|&b| b == 0).unwrap_or(data.len());
    String::from_utf8_lossy(&data[..end]).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_multiple_entries() {
        let mut data = vec![0u8; AdsLicenseOnlineInfo::LENGTH * 2];
        data[32..36].copy_from_slice(&0u32.to_le_bytes()); // first entry status: Valid
        data[AdsLicenseOnlineInfo::LENGTH + 32..AdsLicenseOnlineInfo::LENGTH + 36]
            .copy_from_slice(&596u32.to_le_bytes()); // second entry status: Trial

        let entries = parse_online_info_entries(&data).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn rejects_partial_entry() {
        let data = vec![0u8; AdsLicenseOnlineInfo::LENGTH + 1];
        assert!(parse_online_info_entries(&data).is_err());
    }

    #[test]
    fn empty_buffer_is_zero_entries() {
        let entries = parse_online_info_entries(&[]).unwrap();
        assert!(entries.is_empty());
    }
}
