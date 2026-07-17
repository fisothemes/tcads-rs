pub mod ads_device;
pub mod logger;
pub mod runtime;

pub mod blocking {
    pub use super::ads_device::blocking::AdsDevice;
    pub use super::logger::blocking::{LogEntryReceiver, Logger};
    pub use super::runtime::blocking::{RuntimeDevice, SymbolVersionReceiver, ValueReceiver};
}

pub mod tokio {
    pub use super::ads_device::tokio::AdsDevice;
    pub use super::logger::tokio::{LogEntryReceiver, Logger};
    pub use super::runtime::tokio::{RuntimeDevice, SymbolVersionReceiver, ValueReceiver};
}
