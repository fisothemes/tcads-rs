use super::AmsRouterCommand;
use crate::errors::AdsError;
use crate::types::{AmsAddr, AmsNetId};
use std::io::Read;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AmsPortConnectResponse {
    addr: AmsAddr,
}

impl AmsPortConnectResponse {
    pub fn new(addr: AmsAddr) -> Self {
        Self { addr }
    }

    pub fn addr(&self) -> &AmsAddr {
        &self.addr
    }

    pub fn read_from<R: Read>(r: &mut R) -> Result<AmsAddr, AdsError> {
        let mut header = [0u8; 6];

        r.read_exact(&mut header[0..2])?;

        let cmd = AmsRouterCommand::from(u16::from_le_bytes(header[0..2].try_into().unwrap()));

        if cmd != AmsRouterCommand::PortConnect {
            return Err(AdsError::MalformedPacket(
                format!(
                    "Expected `{:?}` response got `{cmd:?}`",
                    AmsRouterCommand::PortConnect
                )
                .into(),
            ));
        }

        r.read_exact(&mut header[2..6])?;

        let length = u16::from_le_bytes(header[2..4].try_into().unwrap());

        if length != 8 {
            return Err(AdsError::MalformedPacket(
                format!("PortConnect response payload must be 8 bytes got {length}").into(),
            ));
        }

        let mut payload = [0u8; 8];
        r.read_exact(&mut payload)?;

        let net_id = AmsNetId::try_from(&payload[0..6])?;
        let port = u16::from_le_bytes(payload[6..8].try_into().unwrap());

        Ok(AmsAddr::new(net_id, port))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::router::request::AmsPortCloseRequest;
    use std::net::TcpStream;

    #[test]
    fn it_works() {
        let mut stream = TcpStream::connect("127.0.0.1:48898").unwrap();

        //AmsPortConnectRequest::write_to(&mut stream).unwrap();

        //let addr = AmsPortConnectResponse::read_from(&mut stream).unwrap();

        //println!("Registered with {}", addr);

        let close = AmsPortCloseRequest::new(9000);

        close.write_to(&mut stream).unwrap();

        let mut resp = [0u8; 10];

        let n = stream.read(&mut resp).unwrap();

        panic!("{n} bytes received with {:?}", resp);
    }
}
