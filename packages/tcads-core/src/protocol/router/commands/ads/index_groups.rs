//! Reserved Index Groups for System Services.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReservedIndexGroup {
    // --- PLC Specific Ranges (0x0000 - 0xEFFF) ---
    /// PLC ADS Parameter Range (0x1000)
    PlcAdsParam,
    /// PLC ADS Status Range (0x2000)
    /// Contains information about the PLC state.
    PlcAdsStatus,

    /// PLC ADS Unit Function Range (0x3000)
    PlcAdsUnitFunc,
    /// PLC ADS Services (0x4000)
    /// Includes services to access PLC memory range (%M field).
    PlcAdsServices,
    /// PLC Memory Area (%M) Byte Offset (0x4020)
    /// Part of the 0x4000 service range.
    PlcMemoryArea,
    /// PLC Memory Area (%M) Bit Offset (0x4021)
    /// Part of the 0x4000 service range.
    PlcMemoryAreaBits,
    /// PLC Data Area (0x4040)
    /// Often used for Retain data or specific data areas.
    PlcDataArea,

    // --- General TwinCAT ADS System Services (0xF000 - 0xFFFF) ---
    /// Symbol Table (0xF000)
    /// Read: The full symbol table (ADSIGRP_SYM_TAB).
    SymbolTable,
    /// Get a symbol handle by name (0xF003)
    /// Write: Name, Read: Handle
    GetSymHandleByName,
    /// Read/Write Symbol Value by Handle (0xF005)
    ReadWriteSymValByHandle,
    /// Release Symbol Handle (0xF006)
    /// Write: Handle
    ReleaseSymHandle,
    /// PLC Process Image Inputs (Byte Offset) (0xF020)
    /// %I field
    PlcRwInputs,
    /// PLC Process Image Inputs (Bit Offset) (0xF021)
    /// %IX field
    PlcRwInputsBits,
    /// Input Image Size (0xF025)
    /// Read: ULONG size
    PlcReadInputImageSize,
    /// PLC Process Image Outputs (Byte Offset) (0xF030)
    /// %Q field
    PlcRwOutputs,
    /// PLC Process Image Outputs (Bit Offset) (0xF031)
    /// %QX field
    PlcRwOutputsBits,
    /// Output Image Size (0xF035)
    /// Read: ULONG size
    PlcReadOutputImageSize,

    // --- Sum Commands (0xF080 - 0xF086) ---
    /// Sum Command: Read (0xF080)
    SumUpRead,
    /// Sum Command: Write (0xF081)
    SumUpWrite,
    /// Sum Command: Read/Write (0xF082)
    SumUpReadWrite,
    /// Sum Command: ReadEx (0xF083)
    SumUpReadEx,
    /// Sum Command: ReadEx2 (0xF084)
    SumUpReadEx2,
    /// Sum Command: Add Device Notification (0xF085)
    SumUpAddDevNote,
    /// Sum Command: Delete Device Notification (0xF086)
    SumUpDelDevNote,
    /// A raw IndexGroup not defined in this enum (e.g. user defined)
    Unknown(u32),
}

impl ReservedIndexGroup {
    /// Returns true if this is a known, named system service or PLC range.
    /// Returns false if it is Unknown (which includes Reserved ranges).
    pub fn is_known(&self) -> bool {
        !matches!(self, Self::Unknown(_))
    }

    /// Checks if the raw value falls into the Beckhoff "Reserved" range (0x0000 - 0x0FFF).
    /// Useful for Server validation.
    pub fn is_reserved_low(&self) -> bool {
        let val: u32 = (*self).into();
        val <= 0x0FFF
    }

    /// Checks if this is a PLC-specific range (0x1000 - 0xEFFF).
    pub fn is_plc_range(&self) -> bool {
        (0x1000..=0xEFFF).contains(&u32::from(*self))
    }

    /// Checks if this is a System Service (0xF000 - 0xFFFF).
    pub fn is_system_service(&self) -> bool {
        (0xF000..=0xFFFF).contains(&u32::from(*self))
    }
}

impl From<u32> for ReservedIndexGroup {
    fn from(val: u32) -> Self {
        match val {
            // PLC Ranges
            0x1000 => Self::PlcAdsParam,
            0x2000 => Self::PlcAdsStatus,
            0x3000 => Self::PlcAdsUnitFunc,
            0x4000 => Self::PlcAdsServices,
            0x4020 => Self::PlcMemoryArea,
            0x4021 => Self::PlcMemoryAreaBits,
            0x4040 => Self::PlcDataArea,

            // System Services
            0xF000 => Self::SymbolTable,
            0xF003 => Self::GetSymHandleByName,
            0xF005 => Self::ReadWriteSymValByHandle,
            0xF006 => Self::ReleaseSymHandle,
            0xF020 => Self::PlcRwInputs,
            0xF021 => Self::PlcRwInputsBits,
            0xF025 => Self::PlcReadInputImageSize,
            0xF030 => Self::PlcRwOutputs,
            0xF031 => Self::PlcRwOutputsBits,
            0xF035 => Self::PlcReadOutputImageSize,

            // Sum Commands
            0xF080 => Self::SumUpRead,
            0xF081 => Self::SumUpWrite,
            0xF082 => Self::SumUpReadWrite,
            0xF083 => Self::SumUpReadEx,
            0xF084 => Self::SumUpReadEx2,
            0xF085 => Self::SumUpAddDevNote,
            0xF086 => Self::SumUpDelDevNote,

            n => Self::Unknown(n),
        }
    }
}

impl From<ReservedIndexGroup> for u32 {
    fn from(val: ReservedIndexGroup) -> Self {
        match val {
            ReservedIndexGroup::PlcAdsParam => 0x1000,
            ReservedIndexGroup::PlcAdsStatus => 0x2000,
            ReservedIndexGroup::PlcAdsUnitFunc => 0x3000,
            ReservedIndexGroup::PlcAdsServices => 0x4000,
            ReservedIndexGroup::PlcMemoryArea => 0x4020,
            ReservedIndexGroup::PlcMemoryAreaBits => 0x4021,
            ReservedIndexGroup::PlcDataArea => 0x4040,

            ReservedIndexGroup::SymbolTable => 0xF000,
            ReservedIndexGroup::GetSymHandleByName => 0xF003,
            ReservedIndexGroup::ReadWriteSymValByHandle => 0xF005,
            ReservedIndexGroup::ReleaseSymHandle => 0xF006,
            ReservedIndexGroup::PlcRwInputs => 0xF020,
            ReservedIndexGroup::PlcRwInputsBits => 0xF021,
            ReservedIndexGroup::PlcReadInputImageSize => 0xF025,
            ReservedIndexGroup::PlcRwOutputs => 0xF030,
            ReservedIndexGroup::PlcRwOutputsBits => 0xF031,
            ReservedIndexGroup::PlcReadOutputImageSize => 0xF035,

            ReservedIndexGroup::SumUpRead => 0xF080,
            ReservedIndexGroup::SumUpWrite => 0xF081,
            ReservedIndexGroup::SumUpReadWrite => 0xF082,
            ReservedIndexGroup::SumUpReadEx => 0xF083,
            ReservedIndexGroup::SumUpReadEx2 => 0xF084,
            ReservedIndexGroup::SumUpAddDevNote => 0xF085,
            ReservedIndexGroup::SumUpDelDevNote => 0xF086,

            ReservedIndexGroup::Unknown(n) => n,
        }
    }
}

impl PartialOrd for ReservedIndexGroup {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ReservedIndexGroup {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        u32::from(*self).cmp(&u32::from(*other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_group_conversion() {
        assert_eq!(u32::from(ReservedIndexGroup::PlcRwInputs), 0xF020);
        assert_eq!(
            ReservedIndexGroup::from(0xF080),
            ReservedIndexGroup::SumUpRead
        );
    }

    #[test]
    fn test_index_group_ord() {
        assert!(ReservedIndexGroup::PlcRwInputs < ReservedIndexGroup::SumUpRead);
    }

    #[test]
    fn test_index_group_is_known() {
        assert!(ReservedIndexGroup::SymbolTable.is_known());
        assert!(!ReservedIndexGroup::Unknown(0).is_known());
    }
}
