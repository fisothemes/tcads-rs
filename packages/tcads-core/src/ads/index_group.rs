/// Address space within an ADS device that is usually combined with an
/// [`IndexOffset`](crate::IndexOffset) to forms a complete address used in every ADS read, write,
/// and notification request.
///
/// The well-known reserved groups are provided as associated constants.
#[derive(
    serde::Serialize, serde::Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub struct IndexGroup(u32);

impl IndexGroup {
    /// Wire length of an index group in bytes.
    pub const LENGTH: usize = 4;

    /// Read/write PLC input image bytes (`%I` field) via the PLC interface. (`0x4000`)
    pub const PLC_IO_INPUT_BYTES: Self = Self(0x4000);
    /// Read/write PLC output image bytes (`%Q` field) via the PLC interface. (`0x4010`)
    pub const PLC_IO_OUTPUT_BYTES: Self = Self(0x4010);

    /// Read/write PLC memory byte(s) (`%M` field).
    /// Index offset is a byte offset in the range `0x0000`–`0xFFFF`. (`0x4020`)
    pub const PLC_MEMORY_BYTES: Self = Self(0x4020);
    /// Read/write a single PLC memory bit (`%MX` field).
    /// Index offset encodes the bit address: `byte_offset * 8 + bit_number`. (`0x4021`)
    pub const PLC_MEMORY_BITS: Self = Self(0x4021);
    /// Read the byte length of the PLC memory range (`%M`). Index offset: `0`. (`0x4025`)
    pub const PLC_MEMORY_SIZE: Self = Self(0x4025);

    /// Read/write PLC retain data byte(s). Index offset is a byte offset. (`0x4030`)
    pub const PLC_RETAIN_BYTES: Self = Self(0x4030);
    /// Read the byte length of the PLC retain range. Index offset: `0`. (`0x4035`)
    pub const PLC_RETAIN_SIZE: Self = Self(0x4035);

    /// Read/write PLC data byte(s). Index offset is a byte offset. (`0x4040`)
    pub const PLC_DATA_BYTES: Self = Self(0x4040);
    /// Read the byte length of the PLC data range. Index offset: `0`. (`0x4045`)
    pub const PLC_DATA_SIZE: Self = Self(0x4045);

    /// Access the symbol table directly. (`0xF000`)
    pub const SYMBOL_TABLE: Self = Self(0xF000);
    /// Read a symbol name by its index offset. (`0xF001`)
    pub const SYMBOL_NAME: Self = Self(0xF001);
    /// Read/write a symbol's value by its index offset. (`0xF002`)
    pub const SYMBOL_VALUE: Self = Self(0xF002);
    /// Resolve a symbol's 32-bit handle from its instance path. (`0xF003`)
    pub const SYMBOL_HANDLE_BY_NAME: Self = Self(0xF003);
    /// Read/write a symbol's value directly by its instance path. (`0xF004`)
    pub const SYMBOL_VALUE_BY_NAME: Self = Self(0xF004);
    /// Read/write a symbol's value using a handle obtained from
    /// [`SYMBOL_HANDLE_BY_NAME`](Self::SYMBOL_HANDLE_BY_NAME). (`0xF005`)
    pub const SYMBOL_VALUE_BY_HANDLE: Self = Self(0xF005);
    /// Release a previously obtained symbol handle. (`0xF006`)
    pub const SYMBOL_RELEASE_HANDLE: Self = Self(0xF006);
    /// Fetch compact symbol metadata by instance path. (`0xF007`)
    pub const SYMBOL_INFO_BY_NAME: Self = Self(0xF007);
    /// Read the current symbol-table version counter. Usually changes during an online download. (`0xF008`)
    pub const SYMBOL_VERSION: Self = Self(0xF008);
    /// Fetch extended symbol metadata by instance path. (`0xF009`)
    pub const SYMBOL_INFO_BY_NAME_EX: Self = Self(0xF009);
    /// Download a symbol table to the PLC. (`0xF00A`)
    pub const SYMBOL_DOWNLOAD: Self = Self(0xF00A);
    /// Upload (read) the full symbol table blob from the PLC. (`0xF00B`)
    pub const SYMBOL_UPLOAD: Self = Self(0xF00B);
    /// Read the symbol/data-type upload info (original 12-byte [`AdsSymbolUploadInfoV1`](super::AdsSymbolUploadInfoV1)). (`0xF00C`)
    pub const SYMBOL_UPLOAD_INFO: Self = Self(0xF00C);
    /// Download a symbol table using the alternate command. (`0xF00D`)
    pub const SYMBOL_DOWNLOAD2: Self = Self(0xF00D);
    /// Upload (read) the full data-type blob from the PLC. (`0xF00E`)
    pub const DATA_TYPE_UPLOAD: Self = Self(0xF00E);
    /// Read extended symbol/data-type upload info (up to 64-byte [`AdsSymbolUploadInfoV3`](super::AdsSymbolUploadInfoV3)). (`0xF00F`)
    pub const SYMBOL_UPLOAD_INFO2: Self = Self(0xF00F);
    /// Subscribe to change notifications for a named symbol. (`0xF010`)
    pub const SYMBOL_NOTIFICATION: Self = Self(0xF010);
    /// Fetch full extended data-type metadata by type name. (`0xF011`)
    pub const DATA_TYPE_INFO_BY_NAME_EX: Self = Self(0xF011);

    /// Read/write physical input byte(s) (`%I` field). Index offset is a byte offset;
    /// a valid range starts at [`IndexOffset::IO_IMAGE_INPUT_BASE`] (`0x0001F400`). (`0xF020`)
    pub const IO_IMAGE_INPUT_BYTES: Self = Self(0xF020);
    /// Read/write a single physical input bit (`%IX` field). Index offset encodes the
    /// bit address: `base + byte_offset * 8 + bit_number`, where the base is
    /// [`IndexOffset::IO_IMAGE_INPUT_BIT_BASE`] (`0x000FA000`). (`0xF021`)
    pub const IO_IMAGE_INPUT_BITS: Self = Self(0xF021);
    /// Read the byte length of the physical input process image. Index offset: `0`. (`0xF025`)
    pub const IO_IMAGE_INPUT_SIZE: Self = Self(0xF025);
    /// Read/write physical output byte(s) (`%Q` field). Index offset is a byte offset;
    /// a valid range starts at [`IndexOffset::IO_IMAGE_OUTPUT_BASE`] (`0x0003E800`). (`0xF030`)
    pub const IO_IMAGE_OUTPUT_BYTES: Self = Self(0xF030);
    /// Read/write a single physical output bit (`%QX` field). Index offset encodes the
    /// bit address: `base + byte_offset * 8 + bit_number`, where the base is
    /// [`IndexOffset::IO_IMAGE_OUTPUT_BIT_BASE`] (`0x001F4000`). (`0xF031`)
    pub const IO_IMAGE_OUTPUT_BITS: Self = Self(0xF031);
    /// Read the byte length of the physical output process image. Index offset: `0`. (`0xF035`)
    pub const IO_IMAGE_OUTPUT_SIZE: Self = Self(0xF035);

    /// Batch read (up to 500 items). Index offset = n.
    /// Write: `n * [IG, IO, Len]`. Read: `n * Result` + `n * Data`. (`0xF080`)
    pub const SUM_READ: Self = Self(0xF080);
    /// Batch write (up to 500 items). Index offset = n.
    /// Write: `n * [IG, IO, Len]` + `n * Data`. Read: `n * Result`. (`0xF081`)
    pub const SUM_WRITE: Self = Self(0xF081);
    /// Batch read/write (up to 500 items). Index offset = n.
    /// Write: `n * [IG, IO, ReadLen, WriteLen]` + `n * WriteData`.
    /// Read: `n * [Result, ReturnLen]` + `n * ReadData`. (`0xF082`)
    pub const SUM_READ_WRITE: Self = Self(0xF082);
    /// Batch read-ex (up to 500 items). Response contains expected (requested) lengths.
    /// Index offset = n. Same wire format as [`SUM_READ`](Self::SUM_READ). (`0xF083`)
    pub const SUM_READ_EX: Self = Self(0xF083);
    /// Batch read-ex2 (up to 500 items). Response contains actual returned lengths,
    /// prefer this over [`SUM_READ_EX`](Self::SUM_READ_EX) for variable-length data.
    /// Index offset = n. Same wire format as [`SUM_READ`](Self::SUM_READ). (`0xF084`)
    pub const SUM_READ_EX2: Self = Self(0xF084);
    /// Batch add-device-notification (up to 500 items). Index offset = n.
    /// Write: `n * [IG, IO, Attrib]`. Read: `n * Result` + `n * Handle`. (`0xF085`)
    pub const SUM_ADD_NOTIFICATION: Self = Self(0xF085);
    /// Batch delete-device-notification (up to 500 items). Index offset = n.
    /// Write: `n * Handle`. Read: `n * Result`. (`0xF086`)
    pub const SUM_DELETE_NOTIFICATION: Self = Self(0xF086);

    /// Read device-specific data. (`0xF100`)
    pub const DEVICE_DATA: Self = Self(0xF100);

    /// Creates a new index group instance.
    pub const fn new(index_group: u32) -> Self {
        Self(index_group)
    }

    /// Returns the index group as an u32.
    pub const fn as_u32(&self) -> u32 {
        self.0
    }

    /// Converts the index group to a byte array.
    pub const fn to_bytes(&self) -> [u8; IndexGroup::LENGTH] {
        self.0.to_le_bytes()
    }

    /// Converts a byte array to an index group.
    pub const fn from_bytes(bytes: [u8; IndexGroup::LENGTH]) -> Self {
        Self(u32::from_le_bytes(bytes))
    }
}

impl From<u32> for IndexGroup {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<IndexGroup> for u32 {
    fn from(value: IndexGroup) -> Self {
        value.0
    }
}

impl From<[u8; IndexGroup::LENGTH]> for IndexGroup {
    fn from(bytes: [u8; IndexGroup::LENGTH]) -> Self {
        Self::from_bytes(bytes)
    }
}

impl From<&[u8; IndexGroup::LENGTH]> for IndexGroup {
    fn from(bytes: &[u8; IndexGroup::LENGTH]) -> Self {
        Self::from_bytes(*bytes)
    }
}

impl From<IndexGroup> for [u8; IndexGroup::LENGTH] {
    fn from(value: IndexGroup) -> Self {
        value.to_bytes()
    }
}

impl std::fmt::Display for IndexGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{:04X}", self.0)
    }
}

impl std::fmt::Debug for IndexGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "IndexGroup(0x{:04X})", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let ig = IndexGroup::new(0xF005);
        assert_eq!(ig.as_u32(), 0xF005);
        assert_eq!(IndexGroup::from_bytes(ig.to_bytes()), ig);
    }

    #[test]
    fn display_is_hex() {
        assert_eq!(IndexGroup::SYMBOL_VALUE_BY_HANDLE.to_string(), "0xF005");
    }
}
