use crate::ams::{self, AmsAddr, AmsCommand, AmsPort};
use crate::io::frame::AmsFrame;
use crate::protocol::ProtocolError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct PortConnectRequest {
    desired_port: AmsPort,
}

impl PortConnectRequest {
    pub fn new(desired_port: AmsPort) -> Self {
        Self { desired_port }
    }

    pub fn desired_port(&self) -> AmsPort {
        self.desired_port
    }

    pub fn into_frame(self) -> AmsFrame {
        self.into()
    }

    pub fn to_frame(&self) -> AmsFrame {
        self.into()
    }
}

impl From<PortConnectRequest> for AmsFrame {
    fn from(value: PortConnectRequest) -> Self {
        Self::new(AmsCommand::PortConnect, value.desired_port.to_le_bytes())
    }
}

impl From<&PortConnectRequest> for AmsFrame {
    fn from(value: &PortConnectRequest) -> Self {
        (*value).into_frame()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PortConnectResponse {
    addr: AmsAddr,
}

impl PortConnectResponse {
    pub fn new(addr: AmsAddr) -> Self {
        Self { addr }
    }

    pub fn from_frame(frame: AmsFrame) -> Result<Self, ProtocolError> {
        Self::try_from(frame)
    }

    pub fn addr(&self) -> &AmsAddr {
        &self.addr
    }

    pub fn into_frame(self) -> AmsFrame {
        self.into()
    }

    pub fn to_frame(&self) -> AmsFrame {
        self.into()
    }
}

impl From<PortConnectResponse> for AmsFrame {
    fn from(value: PortConnectResponse) -> Self {
        Self::new(AmsCommand::PortConnect, value.addr.to_bytes())
    }
}

impl From<&PortConnectResponse> for AmsFrame {
    fn from(value: &PortConnectResponse) -> Self {
        (*value).into_frame()
    }
}

impl TryFrom<AmsFrame> for PortConnectResponse {
    type Error = ProtocolError;

    fn try_from(value: AmsFrame) -> Result<Self, Self::Error> {
        let header = value.header();

        if header.command() != AmsCommand::PortConnect {
            return Err(ProtocolError::UnexpectedCommand {
                expected: AmsCommand::PortConnect,
                actual: header.command(),
            });
        }

        if header.length() as usize != ams::AMS_ADDR_LEN {
            return Err(ProtocolError::UnexpectedLength {
                expected: ams::AMS_ADDR_LEN,
                actual: header.length() as usize,
            });
        }

        let addr =
            AmsAddr::try_from_slice(&value.payload()[..]).map_err(|e| ams::AmsError::from(e))?;

        Ok(Self { addr })
    }
}
