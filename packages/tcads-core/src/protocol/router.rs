use std::io::{self, Read, Write};

use crate::prelude::{AmsAddr, AmsNetId, AmsPort};

/// The request to the AMS router to claim a dynamic [AMS Address](AmsAddr).
///
/// This is typically the first packet sent over the TCP connection.
///
/// # Wire Format
/// Sends: `[00 10] [02 00 00 00] [00 00]`
///
/// # Example
/// ```rust, no_run
/// use std::net::TcpStream;
/// use tcads_core::protocol::router::{AmsPortConnectRequest, AmsPortConnectResponse};
///
/// let mut stream = TcpStream::connect("127.0.0.1:48898").unwrap();
/// AmsPortConnectRequest::write_to(&mut stream).unwrap();
/// let response = AmsPortConnectResponse::read_from(&mut stream).unwrap();
/// println!("Assigned Address: {}", AmsAddr::from(response));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct AmsPortConnectRequest;

impl AmsPortConnectRequest {
    /// Writes the raw bytes to the stream.
    pub fn write_to<W: Write>(w: &mut W) -> io::Result<()> {
        w.write_all(&[0x00, 0x10, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00])
    }
}

/// The response from the AMS Router containing the assigned [AMS address](AmsAddr).
///
/// This corresponds to an [AmsPortConnectRequest].
///
/// # Wire Format
/// Received: `[00 10] [08 00 00 00] [NetID (6)] [Port (2)]`
///
/// # Example
/// ```rust
/// use std::io::Cursor;
/// use tcads_core::protocol::router::AmsPortConnectResponse;
/// use tcads_core::types::{AmsAddr, AmsNetId};
///
/// // Simulating bytes received from the network
/// // [00 10] (Cmd), [08 00 00 00] (Len), [NetID...], [Port...]
/// let mock_network_data = [
///     0x00, 0x10,
///     0x08, 0x00, 0x00, 0x00,
///     192, 168, 0, 1, 1, 1,
///     0x53, 0x03 // Port 851
/// ];
/// let mut cursor = Cursor::new(mock_network_data);
///
/// let response = AmsPortConnectResponse::read_from(&mut cursor).unwrap();
/// let addr = AmsAddr::from(response);
///
/// assert_eq!(addr.net_id(), AmsNetId::new(192, 168, 0, 1, 1, 1));
/// assert_eq!(addr.port(), 851);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AmsPortConnectResponse {
    addr: AmsAddr,
}

impl AmsPortConnectResponse {
    /// Creates a new response struct manually.
    pub fn new(net_id: AmsNetId, port: AmsPort) -> Self {
        Self {
            addr: AmsAddr::new(net_id, port),
        }
    }

    /// Reads the response from the stream.
    ///
    /// This method validates the TCP header and parses the assigned address.
    pub fn read_from<R: Read>(r: &mut R) -> io::Result<Self> {
        let mut header = [0u8; 6];
        r.read_exact(&mut header)?;

        // Verify Command [0x00, 0x10]
        if header[0..2] != [0x00, 0x10] {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid Router Response Header",
            ));
        }
        // Verify Length (should be 8) [0x08, 0x00, 0x00, 0x00]
        if header[2..6] != [0x08, 0x00, 0x00, 0x00] {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid Router Response Length",
            ));
        }

        // Read Payload (8 bytes)
        let mut buf = [0u8; 8];
        r.read_exact(&mut buf)?;

        Ok(Self::new(
            AmsNetId::try_from(&buf[0..6])
                .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?,
            u16::from_le_bytes(buf[6..8].try_into().unwrap()),
        ))
    }

    /// Writes the response to a stream (e.g. for a Mock Router/Server).
    ///
    /// Sends 14 bytes: `[00 10] [08 00 00 00] [NetID (6)] [Port (2)]`
    pub fn write_to<W: Write>(&self, w: &mut W) -> io::Result<()> {
        let mut buf = [0u8; 14];

        // Write Header
        buf[0..2].copy_from_slice(&[0x00, 0x10]);
        // Write Length
        buf[2..6].copy_from_slice(&[0x08, 0x00, 0x00, 0x00]);
        // Write NetID
        buf[6..12].copy_from_slice(&self.addr.net_id().0);
        // Write Port
        buf[12..14].copy_from_slice(&self.addr.port().to_le_bytes());

        w.write_all(&buf)
    }
}

impl From<AmsAddr> for AmsPortConnectResponse {
    fn from(addr: AmsAddr) -> Self {
        Self { addr }
    }
}

impl From<AmsPortConnectResponse> for AmsAddr {
    fn from(response: AmsPortConnectResponse) -> Self {
        response.addr
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{TcpListener, TcpStream};
    use std::thread;

    #[test]
    fn test_ams_port_connect_flow() {
        // 1. Set up a CI-friendly Mock Router
        // Binding to port 0 asks the OS to assign a random free port.
        // This ensures the test won't fail if port 48898 is busy or unavailable.
        let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to ephemeral port");
        let server_addr = listener.local_addr().expect("Failed to get local address");

        // 2. Spawn the Mock Router (Server)
        let server_handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("Failed to accept connection");

            // --- Server Read Request ---
            let mut req_buf = [0u8; 8];
            stream
                .read_exact(&mut req_buf)
                .expect("Failed to read request");

            // Validate that the client sent the correct magic bytes
            assert_eq!(req_buf, [0x00, 0x10, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00]);

            // --- Server Write Response ---
            // Simulate assigning a dynamic ID: 192.168.100.10.1.1:32000
            let assigned_addr: AmsAddr = "192.168.100.10.1.1:32000"
                .parse()
                .expect("Failed to parse AmsAddr");

            let response = AmsPortConnectResponse::from(assigned_addr);
            response
                .write_to(&mut stream)
                .expect("Failed to write response");
        });

        // 3. Run the Client
        let mut client_stream =
            TcpStream::connect(server_addr).expect("Failed to connect to mock router");

        // --- Client Write Request ---
        AmsPortConnectRequest::write_to(&mut client_stream).expect("Failed to write request");

        // --- Client Read Response ---
        let response =
            AmsPortConnectResponse::read_from(&mut client_stream).expect("Failed to read response");

        // 4. Assertions
        // Verify the client correctly parsed the data sent by the mock server
        let expected_addr: AmsAddr = "192.168.100.10.1.1:32000"
            .parse()
            .expect("Failed to parse AmsAddr");
        assert_eq!(AmsAddr::from(response), expected_addr);

        // Cleanup
        server_handle.join().unwrap();
    }
}
