use super::AmsRequestDispatchKey;
use std::collections::{HashMap, VecDeque};
use tcads_core::AmsFrame;
use tcads_core::InvokeId;
use tokio::sync::Mutex;
use tokio::sync::mpsc::{self, UnboundedReceiver as Receiver, UnboundedSender as Sender};

/// Tracks pending requests and dispatches frames to the writer task.
///
/// See the [blocking equivalent](crate::tasks::blocking::AmsRequestDispatcher) for
/// full design documentation. The tokio variant is identical in shape, and the only
/// differences are [`tokio::sync::Mutex`] for async-safe locking and [`tokio::sync::mpsc`] channels.
pub struct AmsRequestDispatcher {
    /// Pending ADS command responses, keyed by [invoke ID](InvokeId).
    ads: Mutex<HashMap<InvokeId, Sender<AmsFrame>>>,
    /// Pending [PortConnect](tcads_core::protocol::PortConnectResponse) responses.
    port_connect: Mutex<VecDeque<Sender<AmsFrame>>>,
    /// Pending [GetLocalNetId](tcads_core::protocol::GetLocalNetIdResponse) responses.
    net_id: Mutex<VecDeque<Sender<AmsFrame>>>,
    /// Channel to the writer thread.
    write_tx: Sender<AmsFrame>,
}

impl AmsRequestDispatcher {
    /// Creates a new dispatcher with the given writer channel sender.
    pub fn new(write_tx: Sender<AmsFrame>) -> Self {
        Self {
            port_connect: Mutex::new(VecDeque::new()),
            net_id: Mutex::new(VecDeque::new()),
            ads: Mutex::new(HashMap::new()),
            write_tx,
        }
    }

    /// Registers a waiter, enqueues the frame for writing, and returns the response receiver.
    pub async fn dispatch(
        &self,
        key: AmsRequestDispatchKey,
        frame: AmsFrame,
    ) -> Result<Receiver<AmsFrame>, crate::Error> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.register(key, tx).await;
        self.write_tx.send(frame)?;
        Ok(rx)
    }

    /// Called by the reader task to complete a pending request.
    pub async fn complete(&self, key: AmsRequestDispatchKey, frame: AmsFrame) -> crate::Result<()> {
        if let Some(tx) = self.take(key).await {
            let _ = tx.send(frame)?;
        }
        Ok(())
    }

    /// Sends a `frame` directly to the writer task without registering a response waiter.
    ///
    /// Use this for frames where no response is expected i.e.
    /// [`PortClose`](tcads_core::protocol::PortCloseRequest). For all other frames
    /// use [`dispatch`](Self::dispatch), which registers a waiter before sending to
    /// close the window between send and response arrival.
    ///
    /// Returns [`Err`] if the writer channel is already closed. Callers should
    /// generally ignore this error since the goal (closing the connection) is already achieved.
    pub async fn send_only(&self, frame: AmsFrame) -> crate::Result<()> {
        self.write_tx.send(frame)?;
        Ok(())
    }

    /// Clears all pending requests, waking blocked callers with a disconnected error.
    ///
    /// Dropping the senders causes all waiting [`rx.recv()`](Receiver::recv) calls
    /// to return [`None`], which maps to [`Error::Disconnected`](crate::Error::Disconnected).
    pub async fn clear(&self) {
        self.port_connect.lock().await.clear();
        self.net_id.lock().await.clear();
        self.ads.lock().await.clear();
    }

    async fn register(&self, key: AmsRequestDispatchKey, sender: Sender<AmsFrame>) {
        match key {
            AmsRequestDispatchKey::PortConnect => {
                self.port_connect.lock().await.push_back(sender);
            }
            AmsRequestDispatchKey::GetLocalNetId => {
                self.net_id.lock().await.push_back(sender);
            }
            AmsRequestDispatchKey::AdsCommand(id) => {
                self.ads.lock().await.insert(id, sender);
            }
        }
    }

    async fn take(&self, key: AmsRequestDispatchKey) -> Option<Sender<AmsFrame>> {
        match key {
            AmsRequestDispatchKey::PortConnect => self.port_connect.lock().await.pop_front(),
            AmsRequestDispatchKey::GetLocalNetId => self.net_id.lock().await.pop_front(),
            AmsRequestDispatchKey::AdsCommand(id) => self.ads.lock().await.remove(&id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcads_core::ams::AmsCommand;

    fn make_dispatcher() -> (AmsRequestDispatcher, Receiver<AmsFrame>) {
        let (write_tx, write_rx) = mpsc::unbounded_channel();
        (AmsRequestDispatcher::new(write_tx), write_rx)
    }

    #[tokio::test]
    async fn dispatch_enqueues_frame_and_returns_receiver() {
        let (dispatcher, mut write_tx) = make_dispatcher();
        let frame = AmsFrame::empty(AmsCommand::AdsCommand);

        let mut rx = dispatcher
            .dispatch(AmsRequestDispatchKey::AdsCommand(1), frame.clone())
            .await
            .expect("dispatch should succeed");

        let sent = write_tx
            .recv()
            .await
            .expect("write_tx should have received frame");
        assert_eq!(sent, frame);
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn complete_routes_frame_to_waiting_caller() {
        let (dispatcher, _write_tx) = make_dispatcher();
        let frame = AmsFrame::empty(AmsCommand::AdsCommand);
        let response = AmsFrame::empty(AmsCommand::AdsCommand);

        let mut rx = dispatcher
            .dispatch(AmsRequestDispatchKey::AdsCommand(42), frame)
            .await
            .expect("dispatch should succeed");

        dispatcher
            .complete(AmsRequestDispatchKey::AdsCommand(42), response.clone())
            .await
            .expect("complete should succeed");

        assert_eq!(rx.recv().await.expect("should receive response"), response);
    }

    #[tokio::test]
    async fn clear_wakes_waiting_callers_with_error() {
        let (dispatcher, _write_tx) = make_dispatcher();
        let frame = AmsFrame::empty(AmsCommand::AdsCommand);

        let mut rx = dispatcher
            .dispatch(AmsRequestDispatchKey::AdsCommand(1), frame)
            .await
            .expect("dispatch should succeed");

        dispatcher.clear().await;

        assert!(rx.recv().await.is_none())
    }

    #[tokio::test]
    async fn netid_queue_handles_multiple_concurrent_callers() {
        let (dispatcher, _write_rx) = make_dispatcher();
        let frame = AmsFrame::empty(AmsCommand::GetLocalNetId);

        let mut rx1 = dispatcher
            .dispatch(AmsRequestDispatchKey::GetLocalNetId, frame.clone())
            .await
            .expect("first dispatch");
        let mut rx2 = dispatcher
            .dispatch(AmsRequestDispatchKey::GetLocalNetId, frame)
            .await
            .expect("second dispatch");

        let resp1 = AmsFrame::empty(AmsCommand::GetLocalNetId);
        let resp2 = AmsFrame::empty(AmsCommand::GetLocalNetId);

        dispatcher
            .complete(AmsRequestDispatchKey::GetLocalNetId, resp1.clone())
            .await
            .expect("complete should succeed");
        dispatcher
            .complete(AmsRequestDispatchKey::GetLocalNetId, resp2.clone())
            .await
            .expect("complete should succeed");

        assert_eq!(rx1.recv().await.unwrap(), resp1);
        assert_eq!(rx2.recv().await.unwrap(), resp2);
    }
}
