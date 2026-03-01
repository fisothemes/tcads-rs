use std::collections::HashMap;
use std::io::Result; // placeholder result type
use std::net::ToSocketAddrs;
use std::sync::atomic::AtomicU32;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use tcads_core::ads::{DeviceState, NotificationHandle};
use tcads_core::io::blocking::AmsWriter;
use tcads_core::{AdsState, AmsAddr, AmsFrame, IndexGroup, IndexOffset, InvokeId};

type PendingMap = Arc<Mutex<HashMap<InvokeId, Sender<AmsFrame>>>>;

pub struct AdsDevice {
    writer: Arc<Mutex<AmsWriter>>,
    pending: PendingMap,
    invoke_id: AtomicU32,
    source: AmsAddr,
}

impl AdsDevice {
    /// Connects to the local TwinCAT AMS Router (`127.0.0.1:48898`)
    /// and automatically requests an [AMS address](AmsAddr).
    pub fn connect() -> Result<Self> {
        todo!()
    }

    /// Connects to a custom AMS Router and automatically requests an [AMS address](AmsAddr).
    ///
    /// Useful if you are connecting to a remote PLC router but still want
    /// the router to assign your client an address.
    pub fn connect_to<A: ToSocketAddrs>(addr: A) -> Result<Self> {
        todo!()
    }

    /// Connects to a custom AMS Router using an explicitly provided Source Address.
    ///
    /// This bypasses the handshake entirely. Necessary for clients where you must explicitly
    /// match the "Static Route" configured on the target PLC.
    pub fn connect_with_source<A: ToSocketAddrs>(addr: A, source: AmsAddr) -> Result<Self> {
        todo!()
    }

    /// Sends a generic Read request to the target.
    pub fn read(
        &self,
        target: AmsAddr,
        index_group: IndexGroup,
        index_offset: IndexOffset,
        len: u32,
    ) -> Result<Vec<u8>> {
        todo!()
    }

    /// Sends a generic Write request to the target.
    pub fn write(
        &self,
        target: AmsAddr,
        index_group: IndexGroup,
        index_offset: IndexOffset,
        data: &[u8],
    ) -> Result<()> {
        todo!()
    }

    /// Sends an atomic ReadWrite request to the target.
    pub fn read_write(
        &self,
        target: AmsAddr,
        index_group: IndexGroup,
        index_offset: IndexOffset,
        read_len: u32,
        write_data: &[u8],
    ) -> Result<Vec<u8>> {
        todo!()
    }

    /// Reads the name and version of the target ADS device.
    pub fn read_device_info(&self, target: AmsAddr) -> Result<()> {
        todo!()
    }

    /// Reads the ADS State and Device State of the target.
    pub fn read_state(&self, target: AmsAddr) -> Result<(AdsState, DeviceState)> {
        todo!()
    }

    /// Changes the ADS State and Device State of the target.
    pub fn write_control(
        &self,
        target: AmsAddr,
        ads_state: AdsState,
        device_state: DeviceState,
        data: &[u8],
    ) -> Result<()> {
        todo!()
    }

    /// Subscribes to changes on a specific IndexGroup/IndexOffset.
    pub fn add_notification(
        &self,
        target: AmsAddr,
        index_group: IndexGroup,
        index_offset: IndexOffset,
        // Define a `NotificationAttributes` struct later probably...
    ) -> Result<NotificationHandle> {
        todo!()
    }

    /// Deletes an active notification.
    pub fn delete_notification(&self, target: AmsAddr, handle: NotificationHandle) -> Result<()> {
        todo!()
    }

    /// Returns the Source Address of this device.
    pub fn source(&self) -> AmsAddr {
        self.source
    }
}
