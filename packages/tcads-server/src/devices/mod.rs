pub mod ads_device;

#[cfg(feature = "blocking")]
pub mod blocking {
    pub use super::ads_device::blocking::AdsDevice;
}

#[cfg(feature = "tokio")]
pub mod tokio {}
