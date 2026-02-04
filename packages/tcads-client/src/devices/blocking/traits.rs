use crate::client::blocking::Client;
use crate::errors::Result;
use tcads_core::protocol::router::commands::ads::{AdsDeviceInfoResponse, AdsReadStateResponse};

pub type AdsDeviceInfo = AdsDeviceInfoResponse;
pub type AdsReadState = AdsReadStateResponse;

/// The core ADS interface.
///
/// Implements standard ADS operations (Read, Write, ReadState, Notifications)
/// exactly as defined in the ADS Specification.
pub trait AdsDevice {
    /// Returns a reference to the underlying ADS client connection.
    fn client(&self) -> &Client;

    // Returns the target AMS Address of this device.
    // fn addr(&self) -> AmsAddr;

    /// Reads the name and version of the ADS device.
    fn read_device_info(&self) -> Result<AdsDeviceInfo> {
        todo!()
    }

    /// Reads the ADS status and the device status of the ADS device.
    fn read_state(&self) -> Result<AdsReadState> {
        todo!()
    }

    // Changes the ADS status and the device status of the ADS device.
    // fn write_control(&self, ads_state: AdsState, device_state: u16, data: &[u8]) -> Result<()> {
    //     todo!()
    // }
}
