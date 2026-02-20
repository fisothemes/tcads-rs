use super::{ProtocolError, parse_ads_frame};
use crate::ads::{AdsHeader, AdsReturnCode, IndexGroup, IndexOffset};
use crate::ams::{AmsAddr, AmsCommand};
use crate::io::AmsFrame;

/// A zero-copy view of an ADS Write Request (Command `0x0003`).
///
/// Borrows the write data directly from the [`AmsFrame`] that was parsed, avoiding
/// a copy. This is the preferred type for servers that process incoming writes
/// without needing to store the data beyond the current frame.
///
/// For cases where the request must be stored, sent across a channel, or used after
/// the frame is dropped, convert to [`AdsWriteRequestOwned`] via
/// [`into_owned`](Self::into_owned) or [`to_owned`](Self::to_owned).
///
/// # Usage
/// * **Client:** Sends this request to write a variable or memory area on the target.
/// * **Server:** Receives this, writes the data, and replies with an [`AdsWriteResponse`].
///
/// # Protocol Details
/// * **AMS Command:** [`AdsCommand`](AmsCommand::AdsCommand) (`0x0000`)
/// * **ADS Command:** [`AdsWrite`](AdsCommand::AdsWrite) (`0x0003`)
/// * **ADS Payload Length:** 12 + n bytes
/// * **ADS Payload Layout:**
///   * **Index Group:** 4 bytes (u32) - Specifies the Index Group of the data to write.
///   * **Index Offset:** 4 bytes (u32) - Specifies the Index Offset of the data to write.
///   * **Length:** 4 bytes (u32) - The length of the data (in bytes) to write.
///   * **Data:** n bytes - The data to write.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsWriteRequest<'a> {
    header: AdsHeader,
    index_group: IndexGroup,
    index_offset: IndexOffset,
    data: &'a [u8],
}

/// A fully owned ADS Write Request (Command `0x0003`).
///
/// Owns its data buffer, making it suitable for storage, sending across channels,
/// or constructing requests on a client to send to a device.
///
/// # Usage
/// * **Client:** Sends this request to write a variable or memory area on the target.
/// * **Server:** Receives this, writes the data, and replies with an [`AdsWriteResponse`].
///
/// Obtain one by:
/// * Calling [`AdsWriteRequestOwned::new`] to construct a request to send.
/// * Calling [`AdsWriteRequest::into_owned`] or [`AdsWriteRequest::to_owned`] after parsing.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsWriteRequestOwned {
    header: AdsHeader,
    index_group: IndexGroup,
    index_offset: IndexOffset,
    data: Vec<u8>,
}

/// Represents an ADS Write Response (Command `0x0003`).
///
/// This is the reply sent by the ADS device indicating the success or failure of the
/// write operation.
///
/// # Usage
/// * **Server:** Sends this to acknowledge a write request.
/// * **Client:** Receives this to confirm the write was successful.
///
/// # Protocol Details
/// * **AMS Command:** [`AdsCommand`](AmsCommand::AdsCommand) (`0x0000`)
/// * **ADS Command:** [`AdsWrite`](AdsCommand::AdsWrite) (`0x0003`)
/// * **ADS Payload Length:** 4 bytes
/// * **ADS Payload Layout:**
///   * **Result Code:** 4 bytes ([`AdsReturnCode`])
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsWriteResponse {
    header: AdsHeader,
    result: AdsReturnCode,
}
