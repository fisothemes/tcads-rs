use super::{
    SUM_DELETE_NOTIFICATION_INDEX_GROUP, SUM_READ_EX_INDEX_GROUP, SUM_READ_WRITE_INDEX_GROUP,
    SUM_WRITE_INDEX_GROUP,
};
use crate::devices::tokio::AdsDevice;
use crate::tasks::tokio::AmsRequestDispatchKey;
use std::net::ToSocketAddrs;
use std::time::Duration;
use tcads_core::{
    AdsError, AdsNotificationSampleOwned, AdsReturnCode, AmsAddr, IndexOffset, NotificationHandle,
    SumAddNotificationRequest, SumAddNotificationResponse, SumDeleteNotificationResponse,
    SumReadRequest, SumReadResponse, SumReadWriteRequest, SumReadWriteResponseOwned,
    SumWriteRequest, SumWriteResponse,
};
use tokio::sync::mpsc::UnboundedReceiver as Receiver;

/// An asynchronous ADS device client for executing batch operations (Sum Commands).
///
/// Sum Commands allow you to handle multiple ADS reads, writes, or subscriptions
/// in a single network round-trip. This significantly reduces network latency and
/// protocol overhead compared to sending individual requests.
#[derive(Clone)]
pub struct SumDevice {
    inner: AdsDevice,
}

impl SumDevice {
    /// Connects to the local TwinCAT router using the default port.
    ///
    /// This is a convenience wrapper around [`AdsDevice::connect`]
    pub async fn connect(timeout: impl Into<Option<Duration>>) -> crate::Result<Self> {
        let device = AdsDevice::connect(timeout).await?;
        Ok(Self::new(device))
    }

    /// Connects to a specific TwinCAT router address.
    ///
    /// This is a convenience wrapper around [`AdsDevice::connect_to`]
    pub async fn connect_to(
        addr: impl ToSocketAddrs,
        timeout: impl Into<Option<Duration>>,
    ) -> crate::Result<Self> {
        let device = AdsDevice::connect_to(addr, timeout).await?;
        Ok(Self::new(device))
    }

    /// Connects to a remote TwinCAT router, explicitly defining the local source `AmsAddr`.
    ///
    /// This is a convenience wrapper around [`AdsDevice::connect_remote`].
    pub async fn connect_remote(
        addr: impl ToSocketAddrs,
        source: AmsAddr,
        timeout: impl Into<Option<Duration>>,
    ) -> crate::Result<Self> {
        let device = AdsDevice::connect_remote(addr, source, timeout).await?;
        Ok(Self::new(device))
    }

    /// Wraps an existing, connected [`AdsDevice`] with batch-processing capabilities.
    pub fn new(device: AdsDevice) -> Self {
        Self { inner: device }
    }

    /// Gracefully closes the underlying TCP connection and tears down the background routing tasks.
    pub async fn shutdown(&self) {
        self.inner.shutdown().await
    }

    /// Returns a reference to the underlying standard [`AdsDevice`].
    pub fn get_ref(&self) -> &AdsDevice {
        &self.inner
    }

    /// Sends multiple Reads ADS requests to the PLC in a single network transaction.
    ///
    /// Returns a [`SumReadResponse`] which lazily parses the network buffer. Iterating over
    /// the response yields a `Result<&[u8], AdsReturnCode>` for each requested variable,
    /// guaranteeing zero-copy data extraction and safe alignment even if individual variables fail.
    pub async fn read<'a>(
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
            .read_write(target, SUM_READ_EX_INDEX_GROUP, n, read_len, buf)
            .await?;

        Ok(SumReadResponse::new(resp, requests))
    }

    /// Sends multiple Write ADS requests to the PLC in a single network transaction.
    ///
    /// Iterating over the returned [`SumWriteResponse`] yields a `Result<(), AdsReturnCode>`
    /// for each variable, indicating whether the PLC successfully accepted the write payload.
    pub async fn write(
        &self,
        target: AmsAddr,
        requests: &[SumWriteRequest<'_>],
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

        let resp = self
            .inner
            .read_write(
                target,
                SUM_WRITE_INDEX_GROUP,
                n as IndexOffset,
                (n * AdsReturnCode::LENGTH) as u32,
                buf,
            )
            .await?;

        Ok(SumWriteResponse::new(resp).map_err(AdsError::from)?)
    }

    /// Send an ADS read-write request to the PLC in a single network transaction.
    ///
    /// This is most commonly used to dynamically resolve multiple symbol names into
    /// handle integers using Index Group `0xF003`.
    pub async fn read_write(
        &self,
        target: AmsAddr,
        requests: &[SumReadWriteRequest<'_>],
    ) -> crate::Result<SumReadWriteResponseOwned> {
        let n = requests.len();
        if n == 0 {
            return Ok(SumReadWriteResponseOwned::new(vec![], requests));
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

        let read_len = (n * 8) + expected_read_data_len;

        let resp = self
            .inner
            .read_write(
                target,
                SUM_READ_WRITE_INDEX_GROUP,
                n as IndexOffset,
                read_len as u32,
                buf,
            )
            .await?;

        if resp.len() < n * 8 {
            return Err(crate::Error::InvalidPayload);
        }

        Ok(SumReadWriteResponseOwned::new(resp, requests))
    }

    /// Registers a batch of variable notifications with the PLC simultaneously.
    ///
    /// # Returns
    ///
    /// A vector containing a `Result` for every request.
    /// * **Success:** Yields the assigned `NotificationHandle` and a dedicated `Receiver` channel for that specific variable's data stream.
    /// * **Failure:** Yields an `AdsReturnCode`. The internal channel is automatically dropped, preventing memory leaks.
    pub async fn add_notification(
        &self,
        target: AmsAddr,
        requests: &[SumAddNotificationRequest],
    ) -> crate::Result<
        Vec<Result<(NotificationHandle, Receiver<AdsNotificationSampleOwned>), AdsReturnCode>>,
    > {
        let n = requests.len();
        if n == 0 {
            return Ok(vec![]);
        }

        let invoke_id = self.inner.next_invoke_id();

        let receivers = self
            .inner
            .inner()
            .ads_notifs
            .pre_register_batch(invoke_id, n)
            .await;

        let mut write_buf = Vec::with_capacity(n * SumAddNotificationRequest::LENGTH);
        for req in requests {
            req.write_to(&mut write_buf);
        }

        let expected_read_len = (n * 8) as u32;
        let frame = tcads_core::protocol::AdsReadWriteRequestOwned::new(
            target,
            self.inner.source().await,
            invoke_id,
            super::SUM_ADD_NOTIFICATION_INDEX_GROUP,
            n as u32,
            expected_read_len,
            write_buf,
        )
        .into_frame();

        let mut rx = self
            .inner
            .inner()
            .ams_requests
            .dispatch(AmsRequestDispatchKey::AdsCommand(invoke_id), frame)
            .await?;

        let response_frame = match self.inner.inner().timeout {
            Some(duration) => {
                let maybe_msg = tokio::time::timeout(duration, rx.recv())
                    .await
                    .map_err(|_| crate::Error::Timeout)?;
                maybe_msg.ok_or(crate::Error::Disconnected)?
            }
            None => rx.recv().await.ok_or(crate::Error::Disconnected)?,
        };

        let read_write_resp =
            tcads_core::protocol::AdsReadWriteResponse::try_from_frame(&response_frame)?;

        if read_write_resp.result() != AdsReturnCode::Ok {
            return Err(crate::Error::from(read_write_resp.result()));
        }

        let response = SumAddNotificationResponse::new(read_write_resp.data())
            .map_err(|e| crate::Error::from(AdsError::from(e)))?;

        let parsed_results: Vec<Result<NotificationHandle, AdsReturnCode>> =
            response.iter().collect();

        self.inner
            .inner()
            .ads_notifs
            .promote_batch(invoke_id, &parsed_results)
            .await?;

        let final_output = receivers
            .into_iter()
            .zip(parsed_results.into_iter())
            .map(|(rx, res)| res.map(|handle| (handle, rx)))
            .collect();

        Ok(final_output)
    }

    /// Deletes a batch of variable notifications from the PLC simultaneously.
    ///
    /// This method safely synchronizes with the background network tasks. If the PLC
    /// successfully deletes a handle, the local routing channel is immediately closed,
    /// allowing any listening threads to safely terminate.
    pub async fn delete_notification(
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

        let resp_bytes = self
            .inner
            .read_write(
                target,
                SUM_DELETE_NOTIFICATION_INDEX_GROUP,
                n as u32,
                (n * 4) as u32,
                buf,
            )
            .await?;

        let resp = SumDeleteNotificationResponse::new(resp_bytes)
            .map_err(|e| crate::Error::from(AdsError::from(e)))?;

        for (i, result) in resp.iter().enumerate() {
            if result.is_ok() {
                let _ = self.inner.inner().ads_notifs.remove(handles[i]).await;
            }
        }

        Ok(resp)
    }
}
