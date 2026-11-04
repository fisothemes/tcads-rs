use std::fmt;
use std::io::{Read, Write};
use std::net::{SocketAddr, ToSocketAddrs};
#[cfg(unix)]
use std::path::Path;
use std::sync::mpsc::Receiver;
use std::time::Duration;
use tcads_core::protocol::{PortConnectRequest, PortConnectResponse};
use tcads_core::{AmsAddr, AmsNetId, AmsPort, RouterState};
#[cfg(unix)]
use tcads_io::blocking::UnixAmsStream;
use tcads_io::blocking::{AmsReader, AmsWriter, TcpAmsStream};

#[derive(Clone)]
pub struct AdsDevice {
    source: AmsAddr,
}

impl AdsDevice {
    pub fn bind(
        port: impl Into<AmsPort>,
        timeout: impl Into<Option<Duration>>,
    ) -> crate::Result<Self> {
        let port = port.into();
        let timeout = timeout.into();

        #[cfg(unix)]
        match Self::bind_uds("/run/ams/tcsyssrv.ams.sock", port, timeout) {
            Ok(device) => return Ok(device),
            Err(crate::Error::Io(e))
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::ConnectionRefused
                ) => {}
            Err(e) => return Err(e),
        }

        Self::bind_tcp("127.0.0.1:48898", port, timeout)
    }

    pub fn bind_tcp(
        addr: impl ToSocketAddrs,
        port: impl Into<AmsPort>,
        timeout: impl Into<Option<Duration>>,
    ) -> crate::Result<Self> {
        let timeout = timeout.into();
        let (mut reader, mut writer) = match timeout {
            Some(duration) => {
                let addr: SocketAddr = addr.to_socket_addrs()?.next().ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "could not resolve address",
                    )
                })?;
                TcpAmsStream::connect_timeout(&addr, duration)?.try_split()?
            }
            None => TcpAmsStream::connect(addr)?.try_split()?,
        };

        reader.get_ref().set_read_timeout(timeout)?;
        let addr = Self::handshake(&mut reader, &mut writer, port)?;
        reader.get_ref().set_read_timeout(None)?;

        Ok(Self::new(reader, writer, addr))
    }

    #[cfg(unix)]
    pub fn bind_uds(
        path: impl AsRef<Path>,
        port: impl Into<AmsPort>,
        timeout: impl Into<Option<Duration>>,
    ) -> crate::Result<Self> {
        let stream = UnixAmsStream::connect(path)?;
        let (mut reader, mut writer) = stream.try_split()?;
        reader.get_ref().set_read_timeout(timeout.into())?;
        let addr = Self::handshake(&mut reader, &mut writer, port)?;
        reader.get_ref().set_read_timeout(None)?;
        Ok(Self::new(reader, writer, addr))
    }

    pub fn bind_remote(
        addr: impl ToSocketAddrs,
        source: impl Into<AmsAddr>,
        timeout: impl Into<Option<Duration>>,
    ) -> crate::Result<Self> {
        let timeout = timeout.into();
        let (reader, writer) = match timeout {
            Some(duration) => {
                let addr: SocketAddr = addr.to_socket_addrs()?.next().ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "could not resolve address",
                    )
                })?;
                TcpAmsStream::connect_timeout(&addr, duration)?.try_split()?
            }
            None => TcpAmsStream::connect(addr)?.try_split()?,
        };
        Ok(Self::new(reader, writer, source))
    }

    /// Creates a new [`AdsDevice`] from an already split reader and writer.
    pub fn new<R, W>(reader: AmsReader<R>, writer: AmsWriter<W>, source: impl Into<AmsAddr>) -> Self
    where
        R: Read + Send + 'static,
        W: Write + Send + 'static,
    {
        Self {
            source: source.into(),
        }
    }

    /// Gracefully shuts down device connection.
    pub fn shutdown(&self) -> crate::Result<()> {
        todo!()
    }

    /// Returns the source address of the device.
    pub fn source(&self) -> AmsAddr {
        self.source
    }

    /// Queries the local AMS router's Net ID.
    pub fn get_local_net_id(&self) -> crate::Result<AmsNetId> {
        todo!()
    }

    /// Subscribes to the AMS router's state changes.
    pub fn subscribe_router(&self) -> crate::Result<Receiver<RouterState>> {
        todo!()
    }

    /// Returns an iterator over incoming ADS requests.
    pub fn incoming(&self) -> crate::Result<Incoming> {
        todo!();
    }

    /// Asks the AMS router for a port and connects to it if it's available.
    fn handshake<R: Read, W: Write>(
        reader: &mut AmsReader<R>,
        writer: &mut AmsWriter<W>,
        port: impl Into<AmsPort>,
    ) -> crate::Result<AmsAddr> {
        let port = port.into();
        writer.write_frame(&PortConnectRequest::new(port).into_frame())?;

        let frame = match reader.read_frame() {
            Ok(frame) => frame,
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Err(crate::Error::PortConnectClosed { port });
            }
            // Unix reports a read timeout as WouldBlock, Windows as TimedOut
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                return Err(crate::Error::PortConnectTimeout { port });
            }
            Err(e) => return Err(e.into()),
        };

        let addr = *PortConnectResponse::try_from(&frame)?.addr();
        if port != AmsPort::from(0) && addr.port() != port {
            return Err(crate::Error::PortMismatch {
                requested: port,
                assigned: addr,
            });
        }
        Ok(addr)
    }
}

impl fmt::Debug for AdsDevice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AdsDevice")
            .field("source", &self.source)
            .finish()
    }
}

pub struct Incoming;

impl Iterator for Incoming {
    type Item = crate::Result<AdsRequest>;
    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

/// Currently a placeholder for incoming ADS messages.
#[non_exhaustive]
pub enum AdsRequest {
    ReadDeviceInfo,
    Read,
    Write,
    ReadWrite,
    ReadState,
    WriteControl,
    AddNotification,
    DeleteNotification,
}
