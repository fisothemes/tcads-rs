use crate::ads::{AdsHeader, AdsState, DeviceState};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsWriteControlRequest {
    header: AdsHeader,
    ads_state: AdsState,
    device_state: DeviceState,
    data: Vec<u8>,
}
