use super::{
    SUM_DELETE_NOTIFICATION_INDEX_GROUP, SUM_READ_EX_INDEX_GROUP, SUM_READ_WRITE_INDEX_GROUP,
    SUM_WRITE_INDEX_GROUP,
};
use crate::devices::blocking::AdsDevice;
use std::net::ToSocketAddrs;
use std::sync::mpsc::Receiver;
use std::time::Duration;
use tcads_core::{
    AdsError, AdsNotificationSampleOwned, AdsReturnCode, AmsAddr, IndexOffset, NotificationHandle,
    SumAddNotificationRequest, SumDeleteNotificationResponse, SumReadRequest, SumReadResponse,
    SumReadWriteRequest, SumReadWriteResponse, SumWriteRequest, SumWriteResponse,
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

    pub fn read<'a>(
        &self,
        target: AmsAddr,
        requests: &'a [SumReadRequest],
    ) -> crate::Result<SumReadResponse<'a>> {
        let n = requests.len() as u32;

        if n == 0 {
            return Ok(SumReadResponse::new(vec![], requests));
        }

        let mut expected_data_len = 0;
        let mut buf = Vec::with_capacity(n as usize * SumReadRequest::LENGTH);

        for req in requests {
            req.write_to(&mut buf);
            expected_data_len += req.length();
        }

        let read_len = (n * 8) + expected_data_len;
        let resp = self
            .inner
            .read_write(target, SUM_READ_EX_INDEX_GROUP, n, read_len, buf)?;

        Ok(SumReadResponse::new(resp, requests))
    }

    pub fn write(
        &self,
        target: AmsAddr,
        requests: &[SumWriteRequest],
    ) -> crate::Result<SumWriteResponse> {
        let n = requests.len();
        if n == 0 {
            return Ok(SumWriteResponse::empty());
        }

        let total_header_len = n * SumWriteRequest::HEADER_LENGTH;
        let total_data_len: usize = requests.iter().map(|r| r.data().len()).sum();

        let mut buf = Vec::with_capacity(total_header_len + total_data_len);

        buf.resize(total_header_len, 0);

        for (i, req) in requests.iter().enumerate() {
            let header = &mut buf
                [i * SumWriteRequest::HEADER_LENGTH..(i + 1) * SumWriteRequest::HEADER_LENGTH];
            header.copy_from_slice(&req.header_to_bytes());
            buf.extend_from_slice(req.data());
        }

        let resp = self.inner.read_write(
            target,
            SUM_WRITE_INDEX_GROUP,
            n as IndexOffset,
            (n * AdsReturnCode::LENGTH) as u32,
            buf,
        )?;

        Ok(SumWriteResponse::new(resp).map_err(AdsError::from)?)
    }

    pub fn read_write<'a, 'b>(
        &self,
        target: AmsAddr,
        requests: &'a [SumReadWriteRequest<'b>],
    ) -> crate::Result<SumReadWriteResponse<'a, 'b>> {
        let n = requests.len();
        if n == 0 {
            return Ok(SumReadWriteResponse::new(vec![], requests));
        }

        let total_header_len = n * SumReadWriteRequest::HEADER_LENGTH;
        let mut expected_read_data_len = 0;
        let mut total_write_data_len = 0;

        for req in requests {
            expected_read_data_len += req.read_length() as usize;
            total_write_data_len += req.write_data().len();
        }

        let mut buf = Vec::with_capacity(total_header_len + total_write_data_len);
        buf.resize(total_header_len, 0);

        for (i, req) in requests.iter().enumerate() {
            let header = &mut buf[i * SumReadWriteRequest::HEADER_LENGTH
                ..(i + 1) * SumReadWriteRequest::HEADER_LENGTH];
            header.copy_from_slice(&req.header_to_bytes());
            buf.extend_from_slice(req.write_data());
        }

        let read_len = (n * 4) + expected_read_data_len;

        let resp = self.inner.read_write(
            target,
            SUM_READ_WRITE_INDEX_GROUP,
            n as IndexOffset,
            read_len as u32,
            buf,
        )?;

        if resp.len() < n * 4 {
            return Err(crate::Error::InvalidPayload);
        }

        Ok(SumReadWriteResponse::new(resp, requests))
    }

    pub fn add_notification(
        &self,
        _target: AmsAddr,
        _requests: &[SumAddNotificationRequest],
    ) -> crate::Result<
        Vec<Result<(NotificationHandle, Receiver<AdsNotificationSampleOwned>), AdsReturnCode>>,
    > {
        todo!()
    }

    pub fn delete_notification(
        &self,
        target: AmsAddr,
        handles: &[NotificationHandle],
    ) -> crate::Result<SumDeleteNotificationResponse> {
        let n = handles.len();
        if n == 0 {
            return Ok(SumDeleteNotificationResponse::empty());
        }

        let mut buf = Vec::with_capacity(n * 4);
        for handle in handles {
            buf.extend_from_slice(&handle.to_bytes());
        }

        let resp_bytes = self.inner.read_write(
            target,
            SUM_DELETE_NOTIFICATION_INDEX_GROUP,
            n as u32,
            (n * 4) as u32,
            buf,
        )?;

        let resp = SumDeleteNotificationResponse::new(resp_bytes)
            .map_err(|e| crate::Error::from(AdsError::from(e)))?;

        for (i, result) in resp.iter().enumerate() {
            if result.is_ok() {
                let _ = self.inner.inner().ads_notifs.remove(handles[i]);
            }
        }

        Ok(resp)
    }
}
