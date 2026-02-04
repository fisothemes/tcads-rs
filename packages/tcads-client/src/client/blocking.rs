use crate::errors::Result;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use tcads_core::types::AmsNetId;

struct InnerClient {}

#[allow(unused)] // TODO: Delete this.
#[derive(Clone)]
pub struct Client {
    inner: Arc<InnerClient>,
}

impl Client {
    /// Connects to the local AMS Router (127.0.0.1:48898).
    ///
    /// This attempts to perform a "Port Request" handshake to assign a dynamic AMS NetID.
    pub fn new() -> Result<Self> {
        Self::connect("127.0.0.1:48898", None)
    }

    /// Connects to a specific AMS Router.
    ///
    /// # Arguments
    /// * `addr` - The TCP address of the router (e.g. `"192.168.0.10:48898"`).
    /// * `source_id` - Optional. The NetID to use for this client.
    ///                 If `None`, the client asks the router to assign one.
    pub fn connect<A: ToSocketAddrs>(addr: A, source_id: Option<AmsNetId>) -> Result<Self> {
        todo!()
    }
}
