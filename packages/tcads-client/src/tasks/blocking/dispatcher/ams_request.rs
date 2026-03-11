use super::AmsRequestDispatchKey;
use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::sync::mpsc::{self, Receiver, Sender};
use tcads_core::InvokeId;
use tcads_core::io::AmsFrame;

/// Tracks pending requests and dispatches frames to the writer thread.
///
/// The single entry point for sending an AMS request. [`AmsRequestDispatcher::dispatch`]
/// registers the caller's response channel then forwards the frame to the writer.
/// When the reader thread receives a response, it calls [`AmsRequestDispatcher::complete`]
/// to route the frame back to the waiting caller.
pub struct AmsRequestDispatcher {
    /// Pending ADS command responses, keyed by [invoke ID](InvokeId).
    ads: Mutex<HashMap<InvokeId, Sender<AmsFrame>>>,
    /// Pending [PortConnect](tcads_core::protocol::PortConnectResponse) responses.
    port_connect: Mutex<VecDeque<Sender<AmsFrame>>>,
    /// Pending [GetLocalNetId](tcads_core::protocol::GetLocalNetIdResponse) responses;
    net_id: Mutex<VecDeque<Sender<AmsFrame>>>,
    /// Channel to the writer thread.
    write_tx: Sender<AmsFrame>,
}

impl AmsRequestDispatcher {
    /// Creates a new dispatcher with the given writer channel sender.
    pub fn new(write_tx: Sender<AmsFrame>) -> Self {
        Self {
            port_connect: Mutex::new(VecDeque::new()),
            ads: Mutex::new(HashMap::new()),
            net_id: Mutex::new(VecDeque::new()),
            write_tx,
        }
    }

    /// Registers a waiter, enqueues the frame for writing, and returns the response receiver.
    ///
    /// Registration and dispatch happen together, closing the window between the two.
    pub fn dispatch(
        &self,
        key: AmsRequestDispatchKey,
        frame: AmsFrame,
    ) -> Result<Receiver<AmsFrame>, crate::Error> {
        let (tx, rx) = mpsc::channel();
        self.register(key, tx)?;
        self.write_tx.send(frame)?;
        Ok(rx)
    }

    /// Called by the reader thread to complete a pending request.
    pub fn complete(&self, key: AmsRequestDispatchKey, frame: AmsFrame) -> crate::Result<()> {
        if let Some(tx) = self.take(key)? {
            tx.send(frame)?;
        }
        Ok(())
    }

    /// Sends `frame` directly to the writer thread without registering a response waiter.
    ///
    /// Use this for frames where no response is expected i.e.
    /// [`PortClose`](tcads_core::protocol::PortCloseRequest). For all other frames
    /// use [`dispatch`](Self::dispatch), which registers a waiter before sending to
    /// close the window between send and response arrival.
    ///
    /// Returns [`Err`] if the writer channel is already closed, which means the
    /// connection is already gone. Callers should generally ignore this error
    /// since the goal (closing the connection) is already achieved.
    pub fn send_only(&self, frame: AmsFrame) -> crate::Result<()> {
        self.write_tx.send(frame)?;
        Ok(())
    }

    /// Clears all pending requests, waking blocked callers with a disconnected error.
    ///
    /// Dropping the senders causes all waiting [`rx.recv()`](Receiver::recv) calls
    /// to return [`Err`], which maps to [`Error::Disconnected`].
    pub fn clear(&self) -> crate::Result<()> {
        self.port_connect.lock()?.clear();
        self.ads.lock()?.clear();
        self.net_id.lock()?.clear();
        Ok(())
    }

    fn register(&self, key: AmsRequestDispatchKey, sender: Sender<AmsFrame>) -> crate::Result<()> {
        match key {
            AmsRequestDispatchKey::AdsCommand(id) => {
                self.ads.lock()?.insert(id, sender);
            }
            AmsRequestDispatchKey::PortConnect => {
                self.port_connect.lock()?.push_back(sender);
            }
            AmsRequestDispatchKey::GetLocalNetId => {
                self.net_id.lock()?.push_back(sender);
            }
        }
        Ok(())
    }

    fn take(&self, key: AmsRequestDispatchKey) -> crate::Result<Option<Sender<AmsFrame>>> {
        let value = match key {
            AmsRequestDispatchKey::PortConnect => self.port_connect.lock()?.pop_front(),
            AmsRequestDispatchKey::GetLocalNetId => self.net_id.lock()?.pop_front(),
            AmsRequestDispatchKey::AdsCommand(id) => self.ads.lock()?.remove(&id),
        };

        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcads_core::ams::AmsCommand;

    fn make_dispatcher() -> (AmsRequestDispatcher, Receiver<AmsFrame>) {
        let (write_tx, write_rx) = mpsc::channel();
        (AmsRequestDispatcher::new(write_tx), write_rx)
    }

    #[test]
    fn dispatch_enqueues_frame_and_returns_receiver() {
        let (dispatcher, write_rx) = make_dispatcher();
        let frame = AmsFrame::empty(AmsCommand::AdsCommand);

        let rx = dispatcher
            .dispatch(AmsRequestDispatchKey::AdsCommand(1), frame.clone())
            .expect("dispatch should succeed");

        let sent = write_rx.recv().expect("writer should receive frame");
        assert_eq!(sent, frame);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn complete_routes_frame_to_waiting_caller() {
        let (dispatcher, _write_rx) = make_dispatcher();
        let frame = AmsFrame::empty(AmsCommand::AdsCommand);
        let response = AmsFrame::empty(AmsCommand::AdsCommand);

        let rx = dispatcher
            .dispatch(AmsRequestDispatchKey::AdsCommand(42), frame)
            .expect("dispatch should succeed");

        dispatcher
            .complete(AmsRequestDispatchKey::AdsCommand(42), response.clone())
            .expect("complete should succeed");

        assert_eq!(rx.recv().expect("should receive response"), response);
    }

    #[test]
    fn clear_wakes_waiting_callers_with_error() {
        let (dispatcher, _write_rx) = make_dispatcher();
        let frame = AmsFrame::empty(AmsCommand::AdsCommand);

        let rx = dispatcher
            .dispatch(AmsRequestDispatchKey::AdsCommand(1), frame)
            .expect("dispatch should succeed");

        dispatcher.clear().expect("clear should succeed");

        assert!(rx.recv().is_err());
    }

    #[test]
    fn netid_queue_handles_multiple_concurrent_callers() {
        let (dispatcher, _write_rx) = make_dispatcher();
        let frame = AmsFrame::empty(AmsCommand::GetLocalNetId);

        let rx1 = dispatcher
            .dispatch(AmsRequestDispatchKey::GetLocalNetId, frame.clone())
            .expect("first dispatch");
        let rx2 = dispatcher
            .dispatch(AmsRequestDispatchKey::GetLocalNetId, frame)
            .expect("second dispatch");

        let resp1 = AmsFrame::empty(AmsCommand::GetLocalNetId);
        let resp2 = AmsFrame::empty(AmsCommand::GetLocalNetId);

        dispatcher
            .complete(AmsRequestDispatchKey::GetLocalNetId, resp1.clone())
            .expect("complete should succeed");
        dispatcher
            .complete(AmsRequestDispatchKey::GetLocalNetId, resp2.clone())
            .expect("complete should succeed");

        assert_eq!(rx1.recv().unwrap(), resp1);
        assert_eq!(rx2.recv().unwrap(), resp2);
    }
}
