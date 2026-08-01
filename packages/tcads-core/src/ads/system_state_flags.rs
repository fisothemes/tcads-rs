use bitflags::bitflags;
use std::fmt;

bitflags! {
    /// Flags reported in the `Flags` field of a
    /// [`SYSTEM_SERVICE_STATE`](super::IndexGroup::SYSTEM_SERVICE_STATE) read.
    ///
    /// # Wire Format
    ///
    /// | Offset | Size | Field   | Description      |
    /// |--------|------|---------|-------------------|
    /// | 0      | 2    | `flags` | Bitmask (LE u16) |
    #[derive(
        serde::Serialize, serde::Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord,
        Hash, Default,
    )]
    #[repr(transparent)]
    pub struct AdsSystemStateFlags: u16 {
        /// The system is running in router mode only.
        const ROUTER_MODE_ONLY = 0x0001;
        /// The system is part of a controller redundancy pair.
        const REDUNDANCY_SYSTEM = 0x0002;
        /// The system is the primary controller in a redundancy pair.
        const REDUNDANCY_PRIMARY = 0x0004;
        /// The system is currently active, i.e. controlling the machine.
        const REDUNDANCY_ACTIVE = 0x0010;
        /// The system supports the data folder feature.
        const DATA_FOLDER_SUPPORT = 0x0020;
        /// Redundancy is currently down (not synchronized).
        const REDUNDANCY_IN_OP = 0x0040;
        /// The standby system is currently suspended, e.g. during an online change.
        const REDUNDANCY_SUSPENDED = 0x0080;
        /// A new current config is being created.
        const NEW_CURRENT_CONFIG = 0x0100;
    }
}

impl AdsSystemStateFlags {
    /// Wire size in bytes.
    pub const LENGTH: usize = 2;

    /// Creates a new [`AdsSystemStateFlags`] from a raw `u16`, retaining any
    /// unrecognized bits rather than discarding them.
    pub const fn new(raw: u16) -> Self {
        Self::from_bits_retain(raw)
    }

    /// Parses the flags from a 2-byte little-endian array, safely retaining any
    /// unknown bits.
    pub const fn from_bytes(bytes: [u8; Self::LENGTH]) -> Self {
        Self::from_bits_retain(u16::from_le_bytes(bytes))
    }

    /// Serializes the flags into a 2-byte little-endian array.
    pub const fn to_bytes(self) -> [u8; Self::LENGTH] {
        self.bits().to_le_bytes()
    }

    /// Returns the raw `u16` representation of the flags.
    pub const fn as_raw(&self) -> u16 {
        self.bits()
    }

    /// Returns `true` if the system is running in router mode only.
    pub const fn is_router_mode_only(&self) -> bool {
        self.contains(Self::ROUTER_MODE_ONLY)
    }

    /// Returns `true` if the system is part of a controller redundancy pair.
    pub const fn is_redundancy_system(&self) -> bool {
        self.contains(Self::REDUNDANCY_SYSTEM)
    }

    /// Returns `true` if the system is the primary controller.
    pub const fn is_redundancy_primary(&self) -> bool {
        self.contains(Self::REDUNDANCY_PRIMARY)
    }

    /// Returns `true` if the system is currently active (controlling the machine).
    pub const fn is_redundancy_active(&self) -> bool {
        self.contains(Self::REDUNDANCY_ACTIVE)
    }

    /// Returns `true` if the system supports the data folder feature.
    pub const fn has_data_folder_support(&self) -> bool {
        self.contains(Self::DATA_FOLDER_SUPPORT)
    }

    /// Returns `true` if redundancy is currently down (not synchronized).
    pub const fn is_redundancy_in_op(&self) -> bool {
        self.contains(Self::REDUNDANCY_IN_OP)
    }

    /// Returns `true` if the standby system is currently suspended.
    pub const fn is_redundancy_suspended(&self) -> bool {
        self.contains(Self::REDUNDANCY_SUSPENDED)
    }

    /// Returns `true` if a new current config is being created.
    pub const fn is_new_current_config(&self) -> bool {
        self.contains(Self::NEW_CURRENT_CONFIG)
    }
}

impl From<u16> for AdsSystemStateFlags {
    fn from(raw: u16) -> Self {
        Self::from_bits_retain(raw)
    }
}

impl From<AdsSystemStateFlags> for u16 {
    fn from(flags: AdsSystemStateFlags) -> Self {
        flags.bits()
    }
}

impl From<[u8; AdsSystemStateFlags::LENGTH]> for AdsSystemStateFlags {
    fn from(bytes: [u8; AdsSystemStateFlags::LENGTH]) -> Self {
        Self::from_bytes(bytes)
    }
}

impl fmt::Display for AdsSystemStateFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            f.write_str("None")
        } else {
            bitflags::parser::to_writer(self, f)
        }
    }
}

impl fmt::Debug for AdsSystemStateFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple(stringify!(AdsSystemStateFlags))
            .field(&format_args!("{:#010X}", self.bits()))
            .field(&format_args!("{}", self))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_bytes() {
        let flags =
            AdsSystemStateFlags::REDUNDANCY_ACTIVE | AdsSystemStateFlags::DATA_FOLDER_SUPPORT;
        let bytes = flags.to_bytes();
        assert_eq!(AdsSystemStateFlags::from_bytes(bytes), flags);
    }

    #[test]
    fn retains_unknown_bits() {
        let flags = AdsSystemStateFlags::new(0x0008);
        assert_eq!(flags.as_raw(), 0x0008);
    }

    #[test]
    fn predicate_helpers() {
        let flags = AdsSystemStateFlags::ROUTER_MODE_ONLY | AdsSystemStateFlags::REDUNDANCY_PRIMARY;
        assert!(flags.is_router_mode_only());
        assert!(flags.is_redundancy_primary());
        assert!(!flags.is_redundancy_active());
    }
}
