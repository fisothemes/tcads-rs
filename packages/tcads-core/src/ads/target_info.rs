/// The target device category, as reported by a
/// [`SYSTEM_SERVICE_TARGET_INFO_TYPE`](super::IndexOffset::SYSTEM_SERVICE_TARGET_INFO_TYPE) read.
#[derive(
    serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub enum AdsTargetType {
    /// A regular Windows PC.
    Pc,
    /// An Embedded PC (`CX` series).
    Cx,
    /// A Bus Coupler (`BC` series).
    Bc,
    /// A Bus Terminal Controller (`BX` series).
    Bx,
    /// A target type not defined in the library.
    Unknown(u32),
}

impl AdsTargetType {
    /// Wire size in bytes.
    pub const LENGTH: usize = 4;

    /// Creates a new [`AdsTargetType`] from an array of little-endian bytes.
    pub fn from_bytes(bytes: [u8; Self::LENGTH]) -> Self {
        let value = u32::from_le_bytes(bytes);
        value.into()
    }

    /// Serializes the target type into an array of little-endian bytes.
    pub fn to_bytes(self) -> [u8; Self::LENGTH] {
        let value: u32 = self.into();
        value.to_le_bytes()
    }
}

impl From<u32> for AdsTargetType {
    fn from(value: u32) -> Self {
        match value {
            1 => Self::Pc,
            2 => Self::Cx,
            3 => Self::Bc,
            4 => Self::Bx,
            n => Self::Unknown(n),
        }
    }
}

impl From<AdsTargetType> for u32 {
    fn from(value: AdsTargetType) -> Self {
        match value {
            AdsTargetType::Pc => 1,
            AdsTargetType::Cx => 2,
            AdsTargetType::Bc => 3,
            AdsTargetType::Bx => 4,
            AdsTargetType::Unknown(n) => n,
        }
    }
}
