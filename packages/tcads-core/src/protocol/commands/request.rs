//! Definition of ADS Request Payloads

use std::io::{self, Read, Write};
use std::time::Duration;

use super::{AdsState, AdsTransMode, NotificationHandle};

/// Payload for [`CommandId::AdsRead`](super::CommandId::AdsRead).
///
/// Direction: Client -> Server
///
/// A request to read data from an ADS device.
/// The data is addressed by the Index Group and the Index Offset
///
/// # Layout
/// - **Index Group:** 4 bytes
/// - **Index Offset:** 4 bytes
/// - **Length:** 4 bytes (How many bytes to read)
///
/// ```text
/// [ Index Group (4) ] [ Index Offset (4) ] [ Length (4) ]
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdsReadRequest {
    index_group: u32,
    index_offset: u32,
    length: u32,
}

impl AdsReadRequest {
    /// Size of the fixed header of the request.
    pub const SIZE: usize = 12;

    /// Creates a new AdsReadRequest.
    pub fn new(index_group: u32, index_offset: u32, length: u32) -> Self {
        Self {
            index_group,
            index_offset,
            length,
        }
    }

    /// Returns the Index Group of the data which should be read.
    pub fn index_group(&self) -> u32 {
        self.index_group
    }

    /// Returns the Index Offset of the data which should be read.
    pub fn index_offset(&self) -> u32 {
        self.index_offset
    }

    /// Returns the length of the data (in bytes) which should be read.
    pub fn length(&self) -> u32 {
        self.length
    }

    /// Writes the fixed header of the request.
    pub fn write_to<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(&self.index_group.to_le_bytes())?;
        w.write_all(&self.index_offset.to_le_bytes())?;
        w.write_all(&self.length.to_le_bytes())?;
        Ok(())
    }

    /// Reads the fixed header of the request.
    pub fn read_from<R: Read>(r: &mut R) -> io::Result<Self> {
        let mut buf = [0u8; 12];
        r.read_exact(&mut buf)?;
        Ok(Self {
            index_group: u32::from_le_bytes(buf[0..4].try_into().unwrap()),
            index_offset: u32::from_le_bytes(buf[4..8].try_into().unwrap()),
            length: u32::from_le_bytes(buf[8..12].try_into().unwrap()),
        })
    }
}

/// Payload Header for [`CommandId::AdsWrite`](super::CommandId::AdsWrite).
///
/// Direction: Client -> Server
///
/// A request to write data to an ADS device.
/// The data is addressed by the Index Group and the Index Offset
///
/// # Layout
/// - **Index Group:** 4 bytes
/// - **Index Offset:** 4 bytes
/// - **Length:** 4 bytes (Size of the data to write)
///
/// # Usage
/// This struct parses the *fixed header* of the request.
/// The data to be written immediately follows this structure in the stream.
///
/// ```text
/// [ Index Group (4) ] [ Index Offset (4) ] [ Length (4) ] [ Data (n bytes...) ]
/// ^-----------------------------------------------------^
///              AdsWriteRequest parses this
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdsWriteRequest {
    index_group: u32,
    index_offset: u32,
    length: u32,
}

impl AdsWriteRequest {
    /// Size of the fixed header of the request.
    pub const SIZE: usize = 12;

    pub fn new(index_group: u32, index_offset: u32, length: u32) -> Self {
        Self {
            index_group,
            index_offset,
            length,
        }
    }

    /// Returns the Index Group in which the data should be written.
    pub fn index_group(&self) -> u32 {
        self.index_group
    }

    /// Returns the Index Offset in which the data should be written.
    pub fn index_offset(&self) -> u32 {
        self.index_offset
    }

    /// Returns the length of the data (in bytes) which are to be written.
    pub fn length(&self) -> u32 {
        self.length
    }

    /// Writes the fixed header of the request.
    pub fn write_to<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(&self.index_group.to_le_bytes())?;
        w.write_all(&self.index_offset.to_le_bytes())?;
        w.write_all(&self.length.to_le_bytes())?;
        Ok(())
    }

    /// Reads the fixed header of the request.
    pub fn read_from<R: Read>(r: &mut R) -> io::Result<Self> {
        let mut buf = [0u8; 12];
        r.read_exact(&mut buf)?;
        Ok(Self {
            index_group: u32::from_le_bytes(buf[0..4].try_into().unwrap()),
            index_offset: u32::from_le_bytes(buf[4..8].try_into().unwrap()),
            length: u32::from_le_bytes(buf[8..12].try_into().unwrap()),
        })
    }
}

/// Payload Header for [`CommandId::AdsReadWrite`](super::CommandId::AdsReadWrite).
///
/// Direction: Client -> Server
///
/// A request to write data to an ADS device and immediately read data back.
///
/// # Layout
/// - **Index Group:** 4 bytes
/// - **Index Offset:** 4 bytes
/// - **Read Length:** 4 bytes (Bytes expected in response)
/// - **Write Length:** 4 bytes (Bytes to write)
///
/// # Usage
/// This struct parses the *fixed header* of the request.
/// The data to be written immediately follows this structure in the stream.
///
/// ```text
/// [ Group (4) ] [ Offset (4) ] [ ReadLen (4) ] [ WriteLen (4) ] [ Write Data (n bytes...) ]
/// ^-----------------------------------------------------------^
///                AdsReadWriteRequest parses this
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdsReadWriteRequest {
    index_group: u32,
    index_offset: u32,
    read_length: u32,
    write_length: u32,
}

impl AdsReadWriteRequest {
    /// Size of the fixed header of the request.
    pub const SIZE: usize = 16;

    pub fn new(index_group: u32, index_offset: u32, read_length: u32, write_length: u32) -> Self {
        Self {
            index_group,
            index_offset,
            read_length,
            write_length,
        }
    }

    /// Returns the Index Group in which the data should be written.
    pub fn index_group(&self) -> u32 {
        self.index_group
    }

    /// Returns the Index Offset in which the data should be written.
    pub fn index_offset(&self) -> u32 {
        self.index_offset
    }

    /// Returns the length of the data (in bytes) which are to be read.
    pub fn read_length(&self) -> u32 {
        self.read_length
    }

    /// Returns the length of the data (in bytes) which are to be written.
    pub fn write_length(&self) -> u32 {
        self.write_length
    }

    /// Writes the fixed header of the request.
    pub fn write_to<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(&self.index_group.to_le_bytes())?;
        w.write_all(&self.index_offset.to_le_bytes())?;
        w.write_all(&self.read_length.to_le_bytes())?;
        w.write_all(&self.write_length.to_le_bytes())?;
        Ok(())
    }

    /// Reads the fixed header of the request.
    pub fn read_from<R: Read>(r: &mut R) -> io::Result<Self> {
        let mut buf = [0u8; 16];
        r.read_exact(&mut buf)?;
        Ok(Self {
            index_group: u32::from_le_bytes(buf[0..4].try_into().unwrap()),
            index_offset: u32::from_le_bytes(buf[4..8].try_into().unwrap()),
            read_length: u32::from_le_bytes(buf[8..12].try_into().unwrap()),
            write_length: u32::from_le_bytes(buf[12..16].try_into().unwrap()),
        })
    }
}

/// Payload for [`CommandId::AdsWriteControl`](super::CommandId::AdsWriteControl).
///
/// Direction: Client -> Server
///
/// Changes the ADS state and Device state of the target. Additionally, it is possible to
/// send data to the target to transfer further information. These data were not analysed
/// from the current ADS devices (PLC, NC, ...).
///
/// # Layout
/// - **ADS State:** 2 bytes (The target state to switch to)
/// - **Device State:** 2 bytes (Usually 0)
/// - **Length:** 4 bytes (Size of additional data)
///
/// ```text
/// [ AdsState (2) ] [ DevState (2) ] [ Length (4) ] [ Data... ]
/// ^----------------------------------------------^
///       AdsWriteControlRequest parses this
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdsWriteControlRequest {
    ads_state: AdsState,
    device_state: u16,
    length: u32,
}

impl AdsWriteControlRequest {
    /// Size of the fixed header of the request.
    pub const SIZE: usize = 8;

    pub fn new(ads_state: AdsState, device_state: u16, length: u32) -> Self {
        Self {
            ads_state,
            device_state,
            length,
        }
    }

    /// Returns the ADS state which should be set on the target.
    pub fn ads_state(&self) -> AdsState {
        self.ads_state
    }

    /// Returns the Device state which should be set on the target.
    pub fn device_state(&self) -> u16 {
        self.device_state
    }

    /// Returns the length of the additional data which should be sent to the target.
    pub fn length(&self) -> u32 {
        self.length
    }

    /// Writes the fixed header of the request.
    pub fn write_to<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(&u16::from(self.ads_state).to_le_bytes())?;
        w.write_all(&self.device_state.to_le_bytes())?;
        w.write_all(&self.length.to_le_bytes())?;
        Ok(())
    }

    /// Reads the fixed header of the request.
    pub fn read_from<R: Read>(r: &mut R) -> io::Result<Self> {
        let mut buf = [0u8; 8];
        r.read_exact(&mut buf)?;
        Ok(Self {
            ads_state: AdsState::from(u16::from_le_bytes(buf[0..2].try_into().unwrap())),
            device_state: u16::from_le_bytes(buf[2..4].try_into().unwrap()),
            length: u32::from_le_bytes(buf[4..8].try_into().unwrap()),
        })
    }
}

/// Payload for [`CommandId::AdsAddDeviceNotification`](super::CommandId::AdsAddDeviceNotification).
///
/// Direction: Client -> Server
///
/// A request to register a notification (subscription) on the ADS device.
///
/// # Layout
/// - **Index Group:** 4 bytes
/// - **Index Offset:** 4 bytes
/// - **Length:** 4 bytes
/// - **Transmission Mode:** 4 bytes
/// - **Max Delay:** 4 bytes
/// - **Cycle Time:** 4 bytes
/// - **Reserved:** 16 bytes (Must be zero)
///
/// ```text
/// [ Group (4) ] [ Offset (4) ] [ Len (4) ] [ Mode (4) ] [ Delay (4) ] [ Cycle (4) ] [ Reserved (16) ]
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdsAddDeviceNotificationRequest {
    index_group: u32,
    index_offset: u32,
    length: u32,
    transmission_mode: AdsTransMode,
    max_delay: Duration,
    cycle_time: Duration,
    reserved: [u8; 16],
}

impl AdsAddDeviceNotificationRequest {
    /// Size of the fixed header of the request.
    pub const SIZE: usize = 40;

    pub fn new(
        index_group: u32,
        index_offset: u32,
        length: u32,
        transmission_mode: AdsTransMode,
        max_delay: Duration,
        cycle_time: Duration,
    ) -> Self {
        Self {
            index_group,
            index_offset,
            length,
            transmission_mode,
            max_delay,
            cycle_time,
            reserved: [0; 16],
        }
    }

    /// Creates a new AdsAddDeviceNotificationRequest with reserved values.
    pub fn with_reserve(
        index_group: u32,
        index_offset: u32,
        length: u32,
        transmission_mode: AdsTransMode,
        max_delay: Duration,
        cycle_time: Duration,
        reserved: [u8; 16],
    ) -> Self {
        Self {
            index_group,
            index_offset,
            length,
            transmission_mode,
            max_delay,
            cycle_time,
            reserved,
        }
    }

    /// Returns the Index Group of the data which should be monitored.
    pub fn index_group(&self) -> u32 {
        self.index_group
    }

    /// Returns the Index Offset of the data which should be monitored.
    pub fn index_offset(&self) -> u32 {
        self.index_offset
    }

    /// Returns the length of the data (in bytes) which should be monitored.
    pub fn length(&self) -> u32 {
        self.length
    }

    /// Returns the transmission mode of the notification.
    pub fn transmission_mode(&self) -> AdsTransMode {
        self.transmission_mode
    }

    /// Returns the maximum delay before notification is sent (converted to 100ns ticks).
    pub fn max_delay(&self) -> Duration {
        self.max_delay
    }

    /// Returns the cycle time to check for changes (converted to 100ns ticks).
    pub fn cycle_time(&self) -> Duration {
        self.cycle_time
    }

    /// Writes the fixed header of the request.
    pub fn write_to<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(&self.index_group.to_le_bytes())?;
        w.write_all(&self.index_offset.to_le_bytes())?;
        w.write_all(&self.length.to_le_bytes())?;
        w.write_all(&u32::from(self.transmission_mode).to_le_bytes())?;

        let delay_ticks = (self.max_delay.as_nanos() / 100) as u32;
        w.write_all(&delay_ticks.to_le_bytes())?;

        let cycle_ticks = (self.cycle_time.as_nanos() / 100) as u32;
        w.write_all(&cycle_ticks.to_le_bytes())?;

        w.write_all(&self.reserved)?;
        Ok(())
    }

    /// Reads the fixed header of the request.
    pub fn read_from<R: Read>(r: &mut R) -> io::Result<Self> {
        let mut buf = [0u8; 40];
        r.read_exact(&mut buf)?;

        let delay_ticks = u32::from_le_bytes(buf[16..20].try_into().unwrap());
        let cycle_ticks = u32::from_le_bytes(buf[20..24].try_into().unwrap());

        Ok(Self {
            index_group: u32::from_le_bytes(buf[0..4].try_into().unwrap()),
            index_offset: u32::from_le_bytes(buf[4..8].try_into().unwrap()),
            length: u32::from_le_bytes(buf[8..12].try_into().unwrap()),
            transmission_mode: AdsTransMode::from(u32::from_le_bytes(
                buf[12..16].try_into().unwrap(),
            )),
            max_delay: Duration::from_nanos(delay_ticks as u64 * 100),
            cycle_time: Duration::from_nanos(cycle_ticks as u64 * 100),
            reserved: buf[24..40].try_into().unwrap(),
        })
    }
}

/// Payload for [`CommandId::AdsDeleteDeviceNotification`](super::CommandId::AdsDeleteDeviceNotification).
///
/// Direction: Client -> Server
///
/// A request to stop a previously registered notification.
///
/// # Layout
/// - **Notification Handle:** 4 bytes
///
/// ```text
/// [ Handle (4) ]
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdsDeleteDeviceNotificationRequest {
    handle: NotificationHandle,
}

impl AdsDeleteDeviceNotificationRequest {
    /// Size of the fixed header of the request.
    pub const SIZE: usize = 4;

    pub fn new(handle: NotificationHandle) -> Self {
        Self { handle }
    }

    /// Returns the notification handle of the notification to delete.
    pub fn handle(&self) -> NotificationHandle {
        self.handle
    }

    /// Writes the fixed header of the request.
    pub fn write_to<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(&self.handle.as_u32().to_le_bytes())
    }

    /// Reads the fixed header of the request.
    pub fn read_from<R: Read>(r: &mut R) -> io::Result<Self> {
        let mut buf = [0u8; 4];
        r.read_exact(&mut buf)?;
        Ok(Self {
            handle: NotificationHandle::new(u32::from_le_bytes(buf)),
        })
    }
}
