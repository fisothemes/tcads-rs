pub mod devices;
pub mod error;
pub mod tasks;

pub use tcads_core::{
    ads::{AdsReturnCode, AdsState, AdsTransMode, DeviceState, IndexGroup, IndexOffset, InvokeId},
    ams::{AmsAddr, AmsNetId, AmsPort, RouterState},
    protocol::{AdsNotificationSampleOwned, ProtocolError},
};

pub use error::{Error, Result};
