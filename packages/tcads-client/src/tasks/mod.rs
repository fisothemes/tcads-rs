use tcads_core::InvokeId;

#[cfg(feature = "blocking")]
pub mod blocking;
#[cfg(feature = "tokio")]
pub mod tokio;

/// Identifies the type of pending request for routing incoming responses.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum AmsRequestDispatchKey {
    AdsCommand(InvokeId),
    PortConnect,
    GetLocalNetId,
}
