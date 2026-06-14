/// The offset of an [`IndexGroup`](crate::IndexGroup).
#[derive(
    serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub struct IndexOffset(u32);

impl IndexOffset {
    /// Wire length of an index offset in bytes.
    pub const LENGTH: usize = 4;

    /// An index offset of zero.
    pub const ZERO: Self = Self(0);

    /// Base byte offset for [`IndexGroup::IO_IMAGE_INPUT_BYTES`] (`%I` field).
    /// Valid offsets start at `0x0001F400`. (`0x0001F400`)
    pub const IO_IMAGE_INPUT_BASE: Self = Self(0x0001_F400);
    /// Base bit-address offset for [`IndexGroup::IO_IMAGE_INPUT_BITS`] (`%IX` field).
    /// Bit address = `IO_IMAGE_INPUT_BIT_BASE + byte_offset * 8 + bit_number`. (`0x000FA000`)
    pub const IO_IMAGE_INPUT_BIT_BASE: Self = Self(0x000F_A000);
    /// Base byte offset for [`IndexGroup::IO_IMAGE_OUTPUT_BYTES`] (`%Q` field).
    /// Valid offsets start at `0x0003E800`. (`0x0003E800`)
    pub const IO_IMAGE_OUTPUT_BASE: Self = Self(0x0003_E800);
    /// Base bit-address offset for [`IndexGroup::IO_IMAGE_OUTPUT_BITS`] (`%QX` field).
    /// Bit address = `IO_IMAGE_OUTPUT_BIT_BASE + byte_offset * 8 + bit_number`. (`0x001F4000`)
    pub const IO_IMAGE_OUTPUT_BIT_BASE: Self = Self(0x001F_4000);

    /// Create a new index offset instance.
    pub const fn new(index_offset: u32) -> Self {
        Self(index_offset)
    }

    /// Returns the index offset as an u32.
    pub const fn as_u32(&self) -> u32 {
        self.0
    }

    /// Convert the index offset to a byte array.
    pub const fn to_bytes(&self) -> [u8; IndexOffset::LENGTH] {
        self.0.to_le_bytes()
    }

    /// Convert a byte array to an index offset.
    pub const fn from_bytes(bytes: [u8; IndexOffset::LENGTH]) -> Self {
        Self(u32::from_le_bytes(bytes))
    }
}

impl From<u32> for IndexOffset {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<IndexOffset> for u32 {
    fn from(value: IndexOffset) -> Self {
        value.0
    }
}

impl From<[u8; IndexOffset::LENGTH]> for IndexOffset {
    fn from(bytes: [u8; IndexOffset::LENGTH]) -> Self {
        Self::from_bytes(bytes)
    }
}

impl From<&[u8; IndexOffset::LENGTH]> for IndexOffset {
    fn from(bytes: &[u8; IndexOffset::LENGTH]) -> Self {
        Self::from_bytes(*bytes)
    }
}

impl From<IndexOffset> for [u8; IndexOffset::LENGTH] {
    fn from(value: IndexOffset) -> Self {
        value.to_bytes()
    }
}

impl Default for IndexOffset {
    fn default() -> Self {
        Self::ZERO
    }
}

impl std::fmt::Display for IndexOffset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<T: Into<u32>> std::ops::Add<T> for IndexOffset {
    type Output = Self;
    fn add(self, rhs: T) -> Self::Output {
        Self(self.0 + rhs.into())
    }
}

impl<T: Into<u32>> std::ops::AddAssign<T> for IndexOffset {
    fn add_assign(&mut self, rhs: T) {
        self.0 += rhs.into();
    }
}

impl<T: Into<u32>> std::ops::Sub<T> for IndexOffset {
    type Output = Self;
    fn sub(self, rhs: T) -> Self::Output {
        Self(self.0 - rhs.into())
    }
}

impl<T: Into<u32>> std::ops::SubAssign<T> for IndexOffset {
    fn sub_assign(&mut self, rhs: T) {
        self.0 -= rhs.into();
    }
}

impl<T: Into<u32>> std::ops::Mul<T> for IndexOffset {
    type Output = Self;
    fn mul(self, rhs: T) -> Self::Output {
        Self(self.0 * rhs.into())
    }
}

impl<T: Into<u32>> std::ops::MulAssign<T> for IndexOffset {
    fn mul_assign(&mut self, rhs: T) {
        self.0 *= rhs.into();
    }
}

impl<T: Into<u32>> std::ops::Div<T> for IndexOffset {
    type Output = Self;
    fn div(self, rhs: T) -> Self::Output {
        Self(self.0 / rhs.into())
    }
}

impl<T: Into<u32>> std::ops::DivAssign<T> for IndexOffset {
    fn div_assign(&mut self, rhs: T) {
        self.0 /= rhs.into();
    }
}

impl<T: Into<u32>> std::ops::Rem<T> for IndexOffset {
    type Output = Self;
    fn rem(self, rhs: T) -> Self::Output {
        Self(self.0 % rhs.into())
    }
}

impl<T: Into<u32>> std::ops::RemAssign<T> for IndexOffset {
    fn rem_assign(&mut self, rhs: T) {
        self.0 %= rhs.into();
    }
}

impl<T: Into<u32>> std::ops::BitAnd<T> for IndexOffset {
    type Output = Self;
    fn bitand(self, rhs: T) -> Self::Output {
        Self(self.0 & rhs.into())
    }
}

impl<T: Into<u32>> std::ops::BitAndAssign<T> for IndexOffset {
    fn bitand_assign(&mut self, rhs: T) {
        self.0 &= rhs.into();
    }
}

impl<T: Into<u32>> std::ops::BitOr<T> for IndexOffset {
    type Output = Self;
    fn bitor(self, rhs: T) -> Self::Output {
        Self(self.0 | rhs.into())
    }
}

impl<T: Into<u32>> std::ops::BitOrAssign<T> for IndexOffset {
    fn bitor_assign(&mut self, rhs: T) {
        self.0 |= rhs.into();
    }
}

impl<T: Into<u32>> std::ops::BitXor<T> for IndexOffset {
    type Output = Self;
    fn bitxor(self, rhs: T) -> Self::Output {
        Self(self.0 ^ rhs.into())
    }
}

impl<T: Into<u32>> std::ops::BitXorAssign<T> for IndexOffset {
    fn bitxor_assign(&mut self, rhs: T) {
        self.0 ^= rhs.into();
    }
}

impl<T: Into<u32>> std::ops::Shl<T> for IndexOffset {
    type Output = Self;
    fn shl(self, rhs: T) -> Self::Output {
        Self(self.0 << rhs.into())
    }
}

impl<T: Into<u32>> std::ops::ShlAssign<T> for IndexOffset {
    fn shl_assign(&mut self, rhs: T) {
        self.0 <<= rhs.into();
    }
}

impl<T: Into<u32>> std::ops::Shr<T> for IndexOffset {
    type Output = Self;
    fn shr(self, rhs: T) -> Self::Output {
        Self(self.0 >> rhs.into())
    }
}

impl<T: Into<u32>> std::ops::ShrAssign<T> for IndexOffset {
    fn shr_assign(&mut self, rhs: T) {
        self.0 >>= rhs.into();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let io = IndexOffset::new(0x0001_F400);
        assert_eq!(io.as_u32(), 0x0001_F400);
        assert_eq!(IndexOffset::from_bytes(io.to_bytes()), io);
    }

    #[test]
    fn default_is_zero() {
        assert_eq!(IndexOffset::default(), IndexOffset::ZERO);
    }
}
