//! Primitive constants for the ADS protocol.

use std::ops::Range;

/// Standard ADS TCP Port
pub const PORT_AMS_TCP: u16 = 48898;
/// Standard ADS UDP Port
pub const PORT_AMS_UDP: u16 = 48899;

/// Maximum allowed AMS Packet Size (64KB)
pub const AMS_PACKET_MAX_LEN: usize = 65535 - AMS_TCP_HEADER_LEN;

/// Length of the AMS NetId (6 bytes)
pub const AMS_NETID_LEN: usize = 6;

/// Length of the AMS/TCP Header (6 bytes)
pub const AMS_TCP_HEADER_LEN: usize = 6;
/// Length of the AMS Header (32 bytes)
pub const AMS_HEADER_LEN: usize = 32;

/// Range of the reserved field in the [`AmsTcpHeader`](crate::protocol::header::AmsTcpHeader) (2 bytes)
pub const AMS_TCP_HEADER_RESERVED_RANGE: Range<usize> = 0..2;
/// Range of the length field in the [`AmsTcpHeader`](crate::protocol::header::AmsTcpHeader) (4 bytes)
pub const AMS_TCP_HEADER_LENGTH_RANGE: Range<usize> = 2..AMS_TCP_HEADER_LEN;

/// Range of the Target NetId field in the [`AmsHeader`](crate::protocol::header::AmsHeader) (6 bytes)
pub const AMS_HEADER_TARGET_NETID_RANGE: Range<usize> = 0..6;
/// Range of the Target Port field in the [`AmsHeader`](crate::protocol::header::AmsHeader) (2 bytes)
pub const AMS_HEADER_TARGET_PORT_RANGE: Range<usize> = 6..8;
/// Range of the Source NetId field in the [`AmsHeader`](crate::protocol::header::AmsHeader) (6 bytes)
pub const AMS_HEADER_SOURCE_NETID_RANGE: Range<usize> = 8..14;
/// Range of the Source Port field in the [`AmsHeader`](crate::protocol::header::AmsHeader) (2 bytes)
pub const AMS_HEADER_SOURCE_PORT_RANGE: Range<usize> = 14..16;
/// Range of the Command ID field in the [`AmsHeader`](crate::protocol::header::AmsHeader) (2 bytes)
pub const AMS_HEADER_COMMAND_ID_RANGE: Range<usize> = 16..18;
/// Range of the State Flags field in the [`AmsHeader`](crate::protocol::header::AmsHeader) (2 bytes)
pub const AMS_HEADER_STATE_FLAGS_RANGE: Range<usize> = 18..20;
/// Range of the Length field in the [`AmsHeader`](crate::protocol::header::AmsHeader) (4 bytes)
pub const AMS_HEADER_LENGTH_RANGE: Range<usize> = 20..24;
/// Range of the Error Code field in the [`AmsHeader`](crate::protocol::header::AmsHeader) (4 bytes)
pub const AMS_HEADER_ERROR_CODE_RANGE: Range<usize> = 24..28;
/// Range of the Invoke ID field in the [`AmsHeader`](crate::protocol::header::AmsHeader) (4 bytes)
pub const AMS_HEADER_INVOKE_ID_RANGE: Range<usize> = 28..AMS_HEADER_LEN;
