use crate::tasks::blocking::{AdsRequest, AmsRequestReader, AmsResponseWriter};
use std::fmt;
use std::io::{Read, Write};
use std::net::ToSocketAddrs;
#[cfg(unix)]
use std::path::Path;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::time::Duration;
use tcads_core::protocol::{PortCloseRequest, PortConnectRequest, PortConnectResponse};
use tcads_core::{AmsAddr, AmsFrame, AmsPort};
#[cfg(unix)]
use tcads_io::blocking::UnixAmsStream;
use tcads_io::blocking::{AmsReader, AmsWriter, TcpAmsStream};

pub struct AdsDevice {
    write_tx: Sender<AmsFrame>,
    request_rx: Receiver<AdsRequest>,
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
        let (mut reader, mut writer) = Self::connect_tcp(addr, timeout)?.try_split()?;

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
        let source = source.into();
        if source.port() == AmsPort::new(0) {
            return Err(crate::Error::InvalidSource(source));
        }

        let (reader, writer) = Self::connect_tcp(addr, timeout.into())?.try_split()?;
        Ok(Self::new(reader, writer, source))
    }

    /// Creates a new [`AdsDevice`] from an already split reader and writer.
    pub fn new<R, W>(reader: AmsReader<R>, writer: AmsWriter<W>, source: impl Into<AmsAddr>) -> Self
    where
        R: Read + Send + 'static,
        W: Write + Send + 'static,
    {
        let source = source.into();
        let (write_tx, _) = AmsResponseWriter::spawn(writer);
        let (request_rx, _) = AmsRequestReader::spawn(write_tx.clone(), reader);

        Self {
            write_tx,
            request_rx,
            source,
        }
    }

    /// Gracefully shuts down device connection.
    pub fn shutdown(self) -> crate::Result<()> {
        let port = self.source.port();
        Ok(self
            .write_tx
            .send(PortCloseRequest::new(port).into_frame())?)
    }

    /// Returns the source address of the device.
    pub fn source(&self) -> AmsAddr {
        self.source
    }

    pub fn recv(&self) -> crate::Result<AdsRequest> {
        Ok(self.request_rx.recv()?)
    }

    pub fn recv_timeout(&self, timeout: Duration) -> crate::Result<AdsRequest> {
        Ok(self.request_rx.recv_timeout(timeout)?)
    }

    pub fn try_recv(&self) -> crate::Result<Option<AdsRequest>> {
        match self.request_rx.try_recv() {
            Ok(request) => Ok(Some(request)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(crate::Error::Disconnected),
        }
    }

    /// Returns an iterator over incoming ADS requests.
    pub fn incoming(self) -> crate::Result<(Incoming, ShutdownToken)> {
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

    /// Connects over TCP. With a timeout, tries every resolved address in turn, the
    /// same as [`std::net::TcpStream::connect`] does without one.
    fn connect_tcp(
        addr: impl ToSocketAddrs,
        timeout: Option<Duration>,
    ) -> crate::Result<TcpAmsStream> {
        let Some(timeout) = timeout else {
            return Ok(TcpAmsStream::connect(addr)?);
        };

        let mut last_err = None;
        for addr in addr.to_socket_addrs()? {
            match TcpAmsStream::connect_timeout(&addr, timeout) {
                Ok(stream) => return Ok(stream),
                Err(e) => last_err = Some(e),
            }
        }
        Err(crate::Error::from(last_err.unwrap_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "could not resolve address",
            )
        })))
    }
}

impl fmt::Debug for AdsDevice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AdsDevice")
            .field("source", &self.source)
            .finish()
    }
}

pub struct ShutdownToken;

pub struct Incoming;

impl Iterator for Incoming {
    type Item = crate::Result<AdsRequest>;
    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}
