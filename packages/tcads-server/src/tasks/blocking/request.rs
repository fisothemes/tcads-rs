use tcads_core::protocol::RouterState;

#[non_exhaustive]
pub enum AdsRequest {
    RouterState(RouterState),
    ReadDeviceInfo,
    Read,
    Write,
    ReadWrite,
    ReadState,
    WriteControl,
    AddNotification,
    DeleteNotification,
}
