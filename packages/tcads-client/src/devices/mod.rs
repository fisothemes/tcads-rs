pub mod ads_device;

pub mod blocking {
    pub use super::ads_device::blocking::AdsDevice;
}

pub mod tokio {}
