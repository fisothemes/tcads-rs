/// Represents an ADS symbol handle acquired from a PLC runtime.
///
/// Handles are dynamically allocated by the PLC and must be released
/// when no longer needed to prevent memory leaks in the runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct SymbolHandle(u32);

impl SymbolHandle {
    /// Wire length of a single handle in bytes.
    pub const LENGTH: usize = 4;

    /// Creates a new instance from a raw `u32`.
    pub const fn new(handle: u32) -> Self {
        Self(handle)
    }

    /// Returns the raw `u32` value of the handle.
    pub fn as_u32(self) -> u32 {
        self.0
    }

    /// Serializes the handle into a 4-byte little-endian array.
    pub fn to_bytes(&self) -> [u8; Self::LENGTH] {
        self.0.to_le_bytes()
    }

    /// Parses a handle from a 4-byte little-endian array.
    pub fn from_bytes(bytes: [u8; Self::LENGTH]) -> Self {
        Self(u32::from_le_bytes(bytes))
    }
}

impl From<u32> for SymbolHandle {
    fn from(handle: u32) -> Self {
        Self(handle)
    }
}

impl From<SymbolHandle> for u32 {
    fn from(handle: SymbolHandle) -> Self {
        handle.0
    }
}

impl From<[u8; Self::LENGTH]> for SymbolHandle {
    fn from(bytes: [u8; Self::LENGTH]) -> Self {
        Self::from_bytes(bytes)
    }
}

impl From<SymbolHandle> for [u8; SymbolHandle::LENGTH] {
    fn from(handle: SymbolHandle) -> Self {
        handle.to_bytes()
    }
}
