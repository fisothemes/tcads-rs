use bitflags::bitflags;
use std::fmt;

bitflags! {
    /// Win32 `FILE_ATTRIBUTE_*` bits, as reported by System Service file operations
    /// (find/browse, [`SYSTEM_SERVICE_FGETSTATUS`](super::IndexGroup::SYSTEM_SERVICE_FGETSTATUS)).
    #[derive(
        serde::Serialize, serde::Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default,
    )]
    #[repr(transparent)]
    pub struct AdsFileAttributes: u32 {
        /// The file is read-only.
        const READ_ONLY = 0x0001;
        /// The file is hidden.
        const HIDDEN = 0x0002;
        /// The file is a system file.
        const SYSTEM = 0x0004;
        /// The entry is a directory.
        const DIRECTORY = 0x0010;
        /// The file is marked for backup/removal (archive bit).
        const ARCHIVE = 0x0020;
        /// The entry is a device.
        const DEVICE = 0x0040;
        /// The file has no other attributes set.
        const NORMAL = 0x0080;
        /// The file is being used for temporary storage.
        const TEMPORARY = 0x0100;
        /// The file is a sparse file.
        const SPARSE_FILE = 0x0200;
        /// The file has a reparse point.
        const REPARSE_POINT = 0x0400;
        /// The file or directory is compressed.
        const COMPRESSED = 0x0800;
    }
}

impl AdsFileAttributes {
    /// Wire size in bytes.
    pub const LENGTH: usize = 4;

    /// Creates a new [`AdsFileAttributes`] from a raw `u32`.
    pub const fn new(raw: u32) -> Self {
        Self::from_bits_retain(raw)
    }

    /// Creates from a 4-byte little-endian array.
    pub const fn from_bytes(bytes: [u8; Self::LENGTH]) -> Self {
        Self::from_bits_retain(u32::from_le_bytes(bytes))
    }

    /// Converts to a 4-byte little-endian array.
    pub const fn to_bytes(self) -> [u8; Self::LENGTH] {
        self.bits().to_le_bytes()
    }

    /// Returns the raw `u32` value.
    pub const fn as_raw(self) -> u32 {
        self.bits()
    }

    /// Returns `true` if the [`READ_ONLY`](Self::READ_ONLY) flag is set.
    pub const fn is_read_only(self) -> bool {
        self.contains(Self::READ_ONLY)
    }
    /// Returns `true` if the [`HIDDEN`](Self::HIDDEN) flag is set.
    pub const fn is_hidden(self) -> bool {
        self.contains(Self::HIDDEN)
    }
    /// Returns `true` if the [`SYSTEM`](Self::SYSTEM) flag is set.
    pub const fn is_system(self) -> bool {
        self.contains(Self::SYSTEM)
    }
    /// Returns `true` if the entry is a directory.
    pub const fn is_directory(self) -> bool {
        self.contains(Self::DIRECTORY)
    }
    /// Returns `true` if the [`ARCHIVE`](Self::ARCHIVE) flag is set.
    pub const fn is_archive(self) -> bool {
        self.contains(Self::ARCHIVE)
    }
    /// Returns `true` if the [`DEVICE`](Self::DEVICE) flag is set.
    pub const fn is_device(self) -> bool {
        self.contains(Self::DEVICE)
    }
    /// Returns `true` if the [`NORMAL`](Self::NORMAL) flag is set.
    pub const fn is_normal(self) -> bool {
        self.contains(Self::NORMAL)
    }
    /// Returns `true` if the [`TEMPORARY`](Self::TEMPORARY) flag is set.
    pub const fn is_temporary(self) -> bool {
        self.contains(Self::TEMPORARY)
    }
    /// Returns `true` if the [`SPARSE_FILE`](Self::SPARSE_FILE) flag is set.
    pub const fn is_sparse_file(self) -> bool {
        self.contains(Self::SPARSE_FILE)
    }
    /// Returns `true` if the [`REPARSE_POINT`](Self::REPARSE_POINT) flag is set.
    pub const fn is_reparse_point(self) -> bool {
        self.contains(Self::REPARSE_POINT)
    }
    /// Returns `true` if the [`COMPRESSED`](Self::COMPRESSED) flag is set.
    pub const fn is_compressed(self) -> bool {
        self.contains(Self::COMPRESSED)
    }
}

impl From<u32> for AdsFileAttributes {
    fn from(raw: u32) -> Self {
        Self::from_bits_retain(raw)
    }
}

impl From<AdsFileAttributes> for u32 {
    fn from(flags: AdsFileAttributes) -> Self {
        flags.bits()
    }
}

impl From<[u8; AdsFileAttributes::LENGTH]> for AdsFileAttributes {
    fn from(bytes: [u8; AdsFileAttributes::LENGTH]) -> Self {
        Self::from_bytes(bytes)
    }
}

impl From<AdsFileAttributes> for [u8; AdsFileAttributes::LENGTH] {
    fn from(flags: AdsFileAttributes) -> Self {
        flags.to_bytes()
    }
}

impl TryFrom<&[u8]> for AdsFileAttributes {
    type Error = std::array::TryFromSliceError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let bytes: [u8; AdsFileAttributes::LENGTH] = value.try_into()?;
        Ok(Self::from_bytes(bytes))
    }
}

impl fmt::Display for AdsFileAttributes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            f.write_str("None")
        } else {
            bitflags::parser::to_writer(self, f)
        }
    }
}

impl fmt::Debug for AdsFileAttributes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple(stringify!(AdsFileAttributes))
            .field(&format_args!("{:#010X}", self.bits()))
            .field(&format_args!("{}", self))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_directory_hidden() {
        let attrs = AdsFileAttributes::new(0x0012); // DIRECTORY | HIDDEN
        assert!(attrs.is_directory());
        assert!(attrs.is_hidden());
        assert!(!attrs.is_read_only());
    }

    #[test]
    fn roundtrip_bytes() {
        let attrs = AdsFileAttributes::ARCHIVE | AdsFileAttributes::COMPRESSED;
        assert_eq!(AdsFileAttributes::from_bytes(attrs.to_bytes()), attrs);
    }

    #[test]
    fn zero_displays_none() {
        assert_eq!(AdsFileAttributes::default().to_string(), "None");
    }
}
