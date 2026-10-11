pub mod ads_device;
pub mod license;
pub mod logger;
pub mod runtime;
pub mod system_service;

#[cfg(feature = "blocking")]
pub mod blocking {
    pub use super::ads_device::blocking::{AdsDevice, AdsDeviceExt, StateReceiver};
    pub use super::license::blocking::AdsLicense;
    pub use super::logger::blocking::{AdsLogger, LogEntryReceiver};
    pub use super::runtime::blocking::{
        AdsRuntime, ReadMultiValues, ReadMultiValuesIter, SymbolVersionReceiver, ValueReceiver,
        WriteMultiValues,
    };
    pub use super::system_service::blocking::AdsSystemService;
}

#[cfg(feature = "tokio")]
pub mod tokio {
    pub use super::ads_device::tokio::{AdsDevice, AdsDeviceExt, StateReceiver};
    pub use super::license::tokio::AdsLicense;
    pub use super::logger::tokio::{AdsLogger, LogEntryReceiver};
    pub use super::runtime::tokio::{
        AdsRuntime, ReadMultiValues, ReadMultiValuesIter, SymbolVersionReceiver, ValueReceiver,
        WriteMultiValues,
    };
    pub use super::system_service::tokio::AdsSystemService;
}
