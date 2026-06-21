/// Represents an ADS symbol handle acquired from a PLC runtime.
///
/// Handles are dynamically allocated by the PLC and must be released
/// when no longer needed to prevent memory leaks in the runtime.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct SymbolHandle([u8; Self::LENGTH]);

impl SymbolHandle {
    /// Wire length of a single handle in bytes.
    pub const LENGTH: usize = 4;

    /// Creates a new instance from a raw `u32`.
    pub const fn new(handle: u32) -> Self {
        Self(handle.to_le_bytes())
    }

    /// Returns the raw `u32` value of the handle.
    pub fn as_u32(self) -> u32 {
        u32::from_le_bytes(self.0)
    }

    /// Serializes the handle into a 4-byte little-endian array.
    pub fn to_bytes(&self) -> [u8; Self::LENGTH] {
        self.0
    }

    /// Parses a handle from a 4-byte little-endian array.
    pub fn from_bytes(bytes: [u8; Self::LENGTH]) -> Self {
        Self(bytes)
    }

    /// Returns the handle as a byte slice.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl AsRef<[u8]> for SymbolHandle {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<u32> for SymbolHandle {
    fn from(handle: u32) -> Self {
        Self::new(handle)
    }
}

impl From<SymbolHandle> for u32 {
    fn from(handle: SymbolHandle) -> Self {
        handle.as_u32()
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

impl std::fmt::Debug for SymbolHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple(stringify!(SymbolHandle))
            .field(&self.as_u32())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_handle_conversions() {
        let handle = SymbolHandle::new(0x1234_5678);
        assert_eq!(handle.as_u32(), 0x1234_5678);
        assert_eq!(handle.to_bytes(), [0x78, 0x56, 0x34, 0x12]);
        assert_eq!(SymbolHandle::from_bytes([0x78, 0x56, 0x34, 0x12]), handle);
        assert_eq!(SymbolHandle::from(0x1234_5678), handle);
        assert_eq!(SymbolHandle::from(handle), handle);
        assert_eq!(SymbolHandle::from([0x78, 0x56, 0x34, 0x12]), handle);
    }
}
