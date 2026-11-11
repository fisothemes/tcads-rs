/// Currently a placeholder for incoming ADS messages.
#[non_exhaustive]
pub enum AdsRequest {
    RouterState,
    ReadDeviceInfo,
    Read,
    Write,
    ReadWrite,
    ReadState,
    WriteControl,
    AddNotification,
    DeleteNotification,
}
