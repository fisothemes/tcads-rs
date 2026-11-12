use crate::tasks::blocking::responder::AdsReadDeviceInfoResponder;
use tcads_core::protocol::RouterState;

#[non_exhaustive]
pub enum AdsRequest {
    RouterState(RouterState),
    ReadDeviceInfo(AdsReadDeviceInfoResponder),
    Read,
    Write,
    ReadWrite,
    ReadState,
    WriteControl,
    AddNotification,
    DeleteNotification,
}
