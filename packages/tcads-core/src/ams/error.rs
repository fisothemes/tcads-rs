/// Errors specific to AMS protocol
#[derive(Debug, Clone, thiserror::Error)]
pub enum AmsError {
    /// Invalid AMS/TCP header
    #[error("Invalid AMS/TCP header: {0}")]
    InvalidAmsTcpHeader(#[from] AmsTcpHeaderError),

    /// Unknown command code
    #[error("Unknown command: {0:#06x}")]
    UnknownCommand(u16),

    /// Frame too large
    #[error("Frame too large: {0} bytes (max: {1})")]
    FrameTooLarge(usize, usize),

    /// Invalid AmsAddr format or content
    #[error("Invalid AMS address: {0}")]
    InvalidAddr(#[from] AddrError),

    /// Invalid NetId format or content
    #[error("Invalid NetId: {0}")]
    InvalidNetId(#[from] NetIdError),
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum AmsTcpHeaderError {
    /// Unknown command code
    #[error("Unknown command: {0:#06x}")]
    UnknownCommand(u16),
    #[error("Invalid length: expected {} bytes, found {} bytes", expected, found)]
    InvalidateLength { expected: usize, found: usize },
    /// Buffer too small for AmsTcpHeader (needs 6 bytes: 2 for [`AmsCommand`](super::command::AmsCommand) + 4 for payload length)
    #[error("Buffer too small: expected {} bytes, found {}", expected, found)]
    BufferTooSmall { expected: usize, found: usize },
    /// Invalid format
    #[error("Invalid format: {0}")]
    InvalidFormat(String),
}

/// Errors when parsing AmsAddr
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum AddrError {
    /// Invalid NetId part
    #[error("Invalid NetId: {0}")]
    InvalidNetId(#[from] NetIdError),

    /// Invalid port number
    #[error("Invalid port: '{0}'")]
    InvalidPort(String),

    /// Missing separator between NetId and port
    #[error("Missing ':' separator between NetId and port")]
    MissingSeparator,

    /// Buffer too small for address (needs 8 bytes: 6 for NetId + 2 for port)
    #[error("Buffer too small: expected {} bytes, found {}", expected, found)]
    BufferTooSmall { expected: usize, found: usize },

    /// Invalid format
    #[error("Invalid format: {0}")]
    InvalidFormat(String),
}

/// Errors when parsing AmsNetId
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum NetIdError {
    /// Wrong number of octets (expected 6)
    #[error("Expected {} octets, found {}", expected, found)]
    WrongOctetCount { expected: usize, found: usize },

    /// Invalid octet value (not a valid u8)
    #[error("Invalid octet at position {}: '{}'", position, value)]
    InvalidOctet { position: usize, value: String },

    /// Buffer too small for NetId
    #[error("Buffer too small: expected {} bytes, found {}", expected, found)]
    BufferTooSmall { expected: usize, found: usize },

    /// Invalid format (e.g. missing dots)
    #[error("Invalid format: {0}")]
    InvalidFormat(String),
}
