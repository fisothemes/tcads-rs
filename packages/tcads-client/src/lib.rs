pub mod devices;
pub mod error;

pub use tcads_core::{
    ads::{AdsReturnCode, IndexGroup, IndexOffset},
    ams::{AmsAddr, AmsNetId, AmsPort},
};

pub use error::{Error, Result};
