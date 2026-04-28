use crate::devices::blocking::AdsDevice;
use std::net::ToSocketAddrs;
use std::sync::mpsc::Receiver;
use std::time::Duration;
use tcads_core::{
    AdsNotificationSampleOwned, AdsReturnCode, AmsAddr, NotificationHandle, SumAddNotificationReq,
    SumReadReq, SumReadWriteReq, SumWriteReq,
};

/// An ADS device client for executing high-performance batch operations (Sum Commands).
///
/// Sum Commands allow you to read, write, or subscribe to hundreds of variables
/// in a single network round-trip, significantly reducing latency overhead.
#[derive(Clone)]
pub struct SumDevice {
    inner: AdsDevice,
}

impl SumDevice {
    pub fn connect(timeout: impl Into<Option<Duration>>) -> crate::Result<Self> {
        let device = AdsDevice::connect(timeout)?;
        Ok(Self::new(device))
    }

    pub fn connect_to(
        addr: impl ToSocketAddrs,
        timeout: impl Into<Option<Duration>>,
    ) -> crate::Result<Self> {
        let device = AdsDevice::connect_to(addr, timeout)?;
        Ok(Self::new(device))
    }

    pub fn connect_remote(
        addr: impl ToSocketAddrs,
        source: AmsAddr,
        timeout: impl Into<Option<Duration>>,
    ) -> crate::Result<Self> {
        let device = AdsDevice::connect_remote(addr, source, timeout)?;
        Ok(Self::new(device))
    }

    pub fn new(device: AdsDevice) -> Self {
        Self { inner: device }
    }

    pub fn shutdown(&self) -> crate::Result<()> {
        self.inner.shutdown()
    }

    pub fn get_ref(&self) -> &AdsDevice {
        &self.inner
    }

    pub fn read(
        &self,
        _target: AmsAddr,
        _requests: &[SumReadReq],
    ) -> crate::Result<Vec<(AdsReturnCode, Vec<u8>)>> {
        todo!()
    }

    pub fn write(
        &self,
        _target: AmsAddr,
        _requests: &[SumWriteReq],
    ) -> crate::Result<Vec<AdsReturnCode>> {
        todo!()
    }

    pub fn read_write(
        &self,
        _target: AmsAddr,
        _requests: &[SumReadWriteReq],
    ) -> crate::Result<Vec<(AdsReturnCode, Vec<u8>)>> {
        todo!()
    }

    pub fn add_notification(
        &self,
        _target: AmsAddr,
        _requests: &[SumAddNotificationReq],
    ) -> crate::Result<
        Vec<(
            AdsReturnCode,
            Receiver<AdsNotificationSampleOwned>,
            NotificationHandle,
        )>,
    > {
        todo!()
    }

    pub fn delete_notification(
        &self,
        _target: AmsAddr,
        _handles: &[NotificationHandle],
    ) -> crate::Result<Vec<AdsReturnCode>> {
        todo!()
    }
}
