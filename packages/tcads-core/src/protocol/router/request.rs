use std::io::{self, Write};

use super::{AmsRouterCommand, AmsRouterFrame};

#[derive(Debug, Clone, Copy, Default)]
pub struct AmsPortConnectRequest;

impl AmsPortConnectRequest {
    pub fn write_to<W: Write>(w: &mut W) -> io::Result<usize> {
        AmsRouterFrame::new(AmsRouterCommand::PortConnect, [0u8; 2]).write_to(w)
    }
}
