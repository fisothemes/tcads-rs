use super::{AmsRouterCommand, AmsRouterFrame};
use crate::types::AmsPort;
use std::io::{self, Write};

#[derive(Debug, Clone, Copy, Default)]
pub struct AmsPortConnectRequest;

impl AmsPortConnectRequest {
    pub fn write_to<W: Write>(w: &mut W) -> io::Result<usize> {
        AmsRouterFrame::new(AmsRouterCommand::PortConnect, [0u8; 2]).write_to(w)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AmsPortCloseRequest {
    port: AmsPort,
}

impl AmsPortCloseRequest {
    pub fn new(port: AmsPort) -> Self {
        Self { port }
    }

    pub fn write_to<W: Write>(&self, w: &mut W) -> io::Result<usize> {
        AmsRouterFrame::new(AmsRouterCommand::PortClose, self.port.to_le_bytes()).write_to(w)
    }
}
