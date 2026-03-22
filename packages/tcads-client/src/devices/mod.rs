pub mod ads_device;
pub mod logger;

pub mod blocking {
    pub use super::ads_device::blocking::AdsDevice;
    pub use super::logger::{
        LogEntry, MessageType,
        blocking::{LogEntryReceiver, Logger},
    };
}

pub mod tokio {
    pub use super::ads_device::tokio::AdsDevice;
    pub use super::logger::{
        LogEntry, MessageType,
        tokio::{LogEntryReceiver, Logger},
    };
}
