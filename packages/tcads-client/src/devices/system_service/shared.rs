use tcads_core::AdsError;

/// Builds the write payload for a
/// [`SYSTEM_SERVICE_START_PROCESS`](tcads_core::IndexGroup::SYSTEM_SERVICE_START_PROCESS)
/// request.
///
/// # Wire Format
///
/// | Offset  | Size | name      | Description                              |
/// |---------|------|-----------|------------------------------------------|
/// | 0       | 4    | `app_len` | Length of the application name in bytes. |
/// | 4       | 4    | `dir_len` | Length of the directory name in bytes.   |
/// | 8       | 4    | `cmd_len` | Length of the command line in bytes.     |
/// | 12      | dyn  | `app`     | Application name.                        |
/// | dyn + 1 | 1    | `0`       | Null terminator.                         |
/// | dyn     | dyn  | `dir`     | Directory name.                          |
/// | dyn + 1 | 1    | `0`       | Null terminator.                         |
/// | dyn     | dyn  | `args`    | Command line arguments.                  |
/// | dyn + 1 | 1    | `0`       | Null terminator.                         |
pub fn build_start_host_process_request(
    app: &str,
    dir: &str,
    args: &str,
) -> crate::Result<Vec<u8>> {
    let app_len = u32::try_from(app.len()).map_err(|_| crate::Error::InvalidPayload)?;
    let dir_len = u32::try_from(dir.len()).map_err(|_| crate::Error::InvalidPayload)?;
    let args_len = u32::try_from(args.len()).map_err(|_| crate::Error::InvalidPayload)?;

    let mut data = Vec::with_capacity(12 + app.len() + dir.len() + args.len() + 3);

    data.extend_from_slice(&app_len.to_le_bytes());
    data.extend_from_slice(&dir_len.to_le_bytes());
    data.extend_from_slice(&args_len.to_le_bytes());

    data.extend_from_slice(app.as_bytes());
    data.push(0);
    data.extend_from_slice(dir.as_bytes());
    data.push(0);
    data.extend_from_slice(args.as_bytes());
    data.push(0);

    Ok(data)
}

/// Builds the write payload for a
/// [`SYSTEM_SERVICE_FRENAME`](tcads_core::IndexGroup::SYSTEM_SERVICE_FRENAME) /
/// [`SYSTEM_SERVICE_FCOPY`](tcads_core::IndexGroup::SYSTEM_SERVICE_FCOPY) request.
pub fn build_rename_or_copy_request(from: &str, to: &str) -> crate::Result<Vec<u8>> {
    let mut data = Vec::with_capacity(from.len() + to.len() + 2);

    data.extend_from_slice(from.as_bytes());
    data.push(0);
    data.extend_from_slice(to.as_bytes());
    data.push(0);

    Ok(data)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_header_and_null_terminated_strings() {
        let data = build_start_host_process_request("/usr/bin/ls", "/tmp", "-la").unwrap();

        assert_eq!(&data[0..4], &11u32.to_le_bytes());
        assert_eq!(&data[4..8], &4u32.to_le_bytes());
        assert_eq!(&data[8..12], &3u32.to_le_bytes());

        let rest = &data[12..];
        assert_eq!(rest, b"/usr/bin/ls\0/tmp\0-la\0");
    }

    #[test]
    fn encodes_empty_optional_fields() {
        let data = build_start_host_process_request("/usr/bin/ls", "", "").unwrap();
        assert_eq!(&data[0..4], &11u32.to_le_bytes());
        assert_eq!(&data[4..8], &0u32.to_le_bytes());
        assert_eq!(&data[8..12], &0u32.to_le_bytes());
        assert_eq!(&data[12..], b"/usr/bin/ls\0\0\0");
    }
}
