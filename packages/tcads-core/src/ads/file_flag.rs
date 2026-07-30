use bitflags::bitflags;
use std::fmt;

bitflags! {
    /// Flags describing the mode for opening or deleting a remote file via the TwinCAT System Service.
    ///
    /// This is a bitmask, so multiple flags can be combined (e.g. [`READ`](Self::READ) | [`BINARY`](Self::BINARY)).
    #[derive(
        serde::Serialize, serde::Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default,
    )]
    #[repr(transparent)]
    pub struct AdsFileFlags: u32 {
        /// File read mode.
        const READ = 1 << 0;
        /// File write mode.
        const WRITE = 1 << 1;
        /// File append mode.
        const APPEND = 1 << 2;
        /// Open for both reading and writing (update).
        const PLUS = 1 << 3;
        /// Binary file mode.
        const BINARY = 1 << 4;
        /// Text file mode.
        const TEXT = 1 << 5;
        /// Ensure the directory exists when opening/creating.
        const ENSURE_DIR = 1 << 6;
        /// Enable directory operations (e.g., for deleting).
        const ENABLE_DIR = 1 << 7;
        /// Overwrite an existing file.
        const OVERWRITE = 1 << 8;
        /// Overwrite an existing file and rename the old one.
        const OVERWRITE_RENAME = 1 << 9;
    }
}

impl AdsFileFlags {
    /// Wire size in bytes.
    pub const LENGTH: usize = 4;

    /// Creates a new [`AdsFileFlags`] from a raw `u32`.
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

    /// Returns `true` if the [`READ`](Self::READ) flag is set.
    pub const fn is_read(self) -> bool {
        self.contains(Self::READ)
    }
    /// Returns `true` if the [`WRITE`](Self::WRITE) flag is set.
    pub const fn is_write(self) -> bool {
        self.contains(Self::WRITE)
    }
    /// Returns `true` if the [`APPEND`](Self::APPEND) flag is set.
    pub const fn is_append(self) -> bool {
        self.contains(Self::APPEND)
    }
    /// Returns `true` if the [`PLUS`](Self::PLUS) flag is set.
    pub const fn is_plus(self) -> bool {
        self.contains(Self::PLUS)
    }
    /// Returns `true` if the [`BINARY`](Self::BINARY) flag is set.
    pub const fn is_binary(self) -> bool {
        self.contains(Self::BINARY)
    }
    /// Returns `true` if the [`TEXT`](Self::TEXT) flag is set.
    pub const fn is_text(self) -> bool {
        self.contains(Self::TEXT)
    }
    /// Returns `true` if the [`ENSURE_DIR`](Self::ENSURE_DIR) flag is set.
    pub const fn has_ensure_dir(self) -> bool {
        self.contains(Self::ENSURE_DIR)
    }
    /// Returns `true` if the [`ENABLE_DIR`](Self::ENABLE_DIR) flag is set.
    pub const fn has_enable_dir(self) -> bool {
        self.contains(Self::ENABLE_DIR)
    }
    /// Returns `true` if the [`OVERWRITE`](Self::OVERWRITE) flag is set.
    pub const fn is_overwrite(self) -> bool {
        self.contains(Self::OVERWRITE)
    }
    /// Returns `true` if the [`OVERWRITE_RENAME`](Self::OVERWRITE_RENAME) flag is set.
    pub const fn is_overwrite_rename(self) -> bool {
        self.contains(Self::OVERWRITE_RENAME)
    }
}

impl From<u32> for AdsFileFlags {
    fn from(raw: u32) -> Self {
        Self::from_bits_retain(raw)
    }
}

impl From<AdsFileFlags> for u32 {
    fn from(flags: AdsFileFlags) -> Self {
        flags.bits()
    }
}

impl From<[u8; AdsFileFlags::LENGTH]> for AdsFileFlags {
    fn from(bytes: [u8; AdsFileFlags::LENGTH]) -> Self {
        Self::from_bytes(bytes)
    }
}

impl From<AdsFileFlags> for [u8; AdsFileFlags::LENGTH] {
    fn from(flags: AdsFileFlags) -> Self {
        flags.to_bytes()
    }
}

impl TryFrom<&[u8]> for AdsFileFlags {
    type Error = std::array::TryFromSliceError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let bytes: [u8; AdsFileFlags::LENGTH] = value.try_into()?;
        Ok(Self::from_bytes(bytes))
    }
}

impl fmt::Display for AdsFileFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            f.write_str("None")
        } else {
            bitflags::parser::to_writer(self, f)
        }
    }
}

impl fmt::Debug for AdsFileFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple(stringify!(AdsFileFlags))
            .field(&format_args!("{:#010X}", self.bits()))
            .field(&format_args!("{}", self))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_read_binary_ensure_dir() {
        // Simulating: READ | BINARY | ENSURE_DIR
        let flags = AdsFileFlags::new(0x00000051);
        assert!(flags.is_read());
        assert!(flags.is_binary());
        assert!(flags.has_ensure_dir());
        assert!(!flags.is_write());
        assert!(!flags.is_text());
    }

    #[test]
    fn roundtrip_bytes() {
        let flags = AdsFileFlags::WRITE | AdsFileFlags::BINARY | AdsFileFlags::PLUS;
        assert_eq!(AdsFileFlags::from_bytes(flags.to_bytes()), flags);
    }

    #[test]
    fn display_shows_active_flags() {
        let flags = AdsFileFlags::WRITE | AdsFileFlags::BINARY;
        let s = flags.to_string();
        assert!(s.contains("WRITE"));
        assert!(s.contains("BINARY"));
        assert!(!s.contains("READ"));
    }

    #[test]
    fn zero_displays_none() {
        assert_eq!(AdsFileFlags::default().to_string(), "None");
    }

    #[test]
    fn bitor_combines() {
        let a = AdsFileFlags::READ;
        let b = AdsFileFlags::ENABLE_DIR;
        assert_eq!((a | b).as_raw(), 0x00000081);
    }
}
