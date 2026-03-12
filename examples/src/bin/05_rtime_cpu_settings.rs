//! Example 5: Real-Time CPU Settings
//! Run with: `cargo run --bin 05_rtime_cpu_settings`
//!
//! Demonstrates sending an ADS read request to the TwinCAT Real-Time system (Port 200)
//! and manually parsing the raw little-endian bytes into a Rust struct.

use tcads::core::io::blocking::AmsStream;
use tcads::core::protocol::{
    AdsReadRequest, AdsReadResponse, GetLocalNetIdRequest, GetLocalNetIdResponse,
    PortConnectRequest, PortConnectResponse,
};
use tcads::core::{AdsReturnCode, AmsAddr};

type BoxError = Box<dyn std::error::Error>;
type Result<T> = std::result::Result<T, BoxError>;

pub const ADSSRVID_READDEVICEINFO: u32 = 0x01;
pub const RTIME_CPU_SETTINGS: u32 = 0xD;

fn main() -> Result<()> {
    let mut stream = AmsStream::connect("127.0.0.1:48898")?;

    // 1. Get our source Address
    stream.write_frame(&PortConnectRequest::default().into_frame())?;
    let response = PortConnectResponse::try_from(&stream.read_frame()?)?;
    let source = *response.addr();

    // 2. Get the target NetID
    stream.write_frame(&GetLocalNetIdRequest.into())?;
    let local_net_id = GetLocalNetIdResponse::try_from(&stream.read_frame()?)?.net_id();

    // 3. Query the RTime System (Port 200), Index Group 1, Index Offset 0xD
    let target = AmsAddr::new(local_net_id, 200);
    let index_group = ADSSRVID_READDEVICEINFO;
    let index_offset = RTIME_CPU_SETTINGS;
    let length = RTimeCpuSettings::LENGTH as u32;

    stream.write_frame(
        &AdsReadRequest::new(target, source, 2, index_group, index_offset, length).into_frame(),
    )?;

    // 4. Read and parse the response
    let read_response_frame = stream.read_frame()?;
    let read_response = AdsReadResponse::try_from(&read_response_frame)?;

    // Check if the router returned an error code
    if read_response.result() != AdsReturnCode::Ok {
        return Err(format!("ADS Error: {:?}", read_response.result()).into());
    }

    // Parse the bytes into our strongly typed struct!
    let cpu_settings = RTimeCpuSettings::try_from(read_response.data())?;

    println!("Real-Time CPU Settings:");
    println!("{:#?}", cpu_settings);

    Ok(())
}

/// Represents the TwinCAT RTimeCpuSettings structure (32 bytes)
#[derive(Debug, Clone)]
pub struct RTimeCpuSettings {
    pub win_cpus: u32,
    pub non_win_cpus: u32,
    pub affinity_mask: u64,
    pub rt_cpus: u32,
    pub cpu_type: u32,
    pub cpu_family: u32,
    pub cpu_freq: u32,
}

impl RTimeCpuSettings {
    /// Length of the RTimeCpuSettings structure in bytes.
    pub const LENGTH: usize = 32;
}

impl TryFrom<&[u8]> for RTimeCpuSettings {
    type Error = BoxError;
    /// Parses the raw 32-byte response from the TwinCAT router
    fn try_from(data: &[u8]) -> std::result::Result<Self, Self::Error> {
        if data.len() < 32 {
            return Err(Box::from("Not enough bytes to parse RTimeCpuSettings"));
        }

        Ok(Self {
            win_cpus: u32::from_le_bytes(data[0..4].try_into()?),
            non_win_cpus: u32::from_le_bytes(data[4..8].try_into()?),
            affinity_mask: u64::from_le_bytes(data[8..16].try_into()?),
            rt_cpus: u32::from_le_bytes(data[16..20].try_into()?),
            cpu_type: u32::from_le_bytes(data[20..24].try_into()?),
            cpu_family: u32::from_le_bytes(data[24..28].try_into()?),
            cpu_freq: u32::from_le_bytes(data[28..32].try_into()?),
        })
    }
}
