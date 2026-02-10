use crate::ams::{self, AmsCommand, AmsNetId};
use crate::io::frame::AmsFrame;
use crate::protocol::ProtocolError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct GetLocalNetIdRequest;

impl GetLocalNetIdRequest {
    pub fn into_frame() -> AmsFrame {
        AmsFrame::from(Self)
    }
}

impl From<GetLocalNetIdRequest> for AmsFrame {
    fn from(_: GetLocalNetIdRequest) -> Self {
        Self::new(AmsCommand::GetLocalNetId, [0u8; 4])
    }
}
