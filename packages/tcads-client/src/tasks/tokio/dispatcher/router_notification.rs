use tcads_core::RouterState;
use tokio::sync::Mutex;
use tokio::sync::mpsc::{self, UnboundedReceiver as Receiver, UnboundedSender as Sender};

/// Fans out [router state](RouterState) changes to all registered subscribers.
///
/// Identical in design to the [blocking equivalent](crate::tasks::blocking::RouterNotificationDispatcher)
/// with the only difference being the usage of [`tokio::sync::Mutex`] and [`tokio::sync::mpsc`] channels
/// in place of their [`std::sync`] counterparts.
pub struct RouterNotificationDispatcher {
    subscribers: Mutex<Vec<Sender<RouterState>>>,
}

impl RouterNotificationDispatcher {
    /// Creates a new dispatcher with no subscribers.
    pub fn new() -> Self {
        Self {
            subscribers: Mutex::new(Vec::new()),
        }
    }

    /// Registers a new subscriber and returns a [`Receiver<RouterState>`].
    ///
    /// The receiver yields every [`RouterState`] broadcast until either:
    /// - The receiver is dropped by the caller, or
    /// - [`clear`](Self::clear) is called on connection loss.
    pub async fn subscribe(&self) -> Receiver<RouterState> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.subscribers.lock().await.push(tx);
        rx
    }

    /// Broadcasts `state` to all live subscribers, pruning dead receivers.
    ///
    /// Called by the reader thread on every incoming
    /// [`RouterNotification`](tcads_core::protocol::RouterNotification) frame.
    pub async fn broadcast(&self, state: RouterState) {
        self.subscribers
            .lock()
            .await
            .retain(|tx| tx.send(state).is_ok());
    }

    /// Clears all subscribers, waking them with [`None`] on their next
    /// [`recv`](Receiver::recv) call.
    ///
    /// Called on connection loss so subscriber loops can exit naturally.
    pub async fn clear(&self) {
        self.subscribers.lock().await.clear();
    }
}

impl Default for RouterNotificationDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn single_subscriber_receives_broadcast() {
        let dispatcher = RouterNotificationDispatcher::new();
        let mut rx = dispatcher.subscribe().await;

        dispatcher.broadcast(RouterState::Removed).await;

        assert_eq!(rx.recv().await.unwrap(), RouterState::Removed);
    }

    #[tokio::test]
    async fn multiple_subscribers_all_receive_broadcast() {
        let dispatcher = RouterNotificationDispatcher::new();
        let mut rx1 = dispatcher.subscribe().await;
        let mut rx2 = dispatcher.subscribe().await;
        let mut rx3 = dispatcher.subscribe().await;

        dispatcher.broadcast(RouterState::Start).await;

        assert_eq!(rx1.recv().await.unwrap(), RouterState::Start);
        assert_eq!(rx2.recv().await.unwrap(), RouterState::Start);
        assert_eq!(rx3.recv().await.unwrap(), RouterState::Start);
    }

    #[tokio::test]
    async fn multiple_broadcasts_received_in_order() {
        let dispatcher = RouterNotificationDispatcher::new();
        let mut rx = dispatcher.subscribe().await;

        dispatcher.broadcast(RouterState::Stop).await;
        dispatcher.broadcast(RouterState::Start).await;
        dispatcher.broadcast(RouterState::Removed).await;

        assert_eq!(rx.recv().await.unwrap(), RouterState::Stop);
        assert_eq!(rx.recv().await.unwrap(), RouterState::Start);
        assert_eq!(rx.recv().await.unwrap(), RouterState::Removed);
    }

    #[tokio::test]
    async fn dropped_dispatcher_closes_receivers() {
        let dispatcher = RouterNotificationDispatcher::new();
        let mut rx = dispatcher.subscribe().await;

        drop(dispatcher);

        assert!(rx.recv().await.is_none());
    }

    #[tokio::test]
    async fn subscribe_after_broadcasts_starts_fresh() {
        let dispatcher = RouterNotificationDispatcher::new();

        dispatcher.broadcast(RouterState::Stop).await;

        // Subscribe after broadcast, should not receive past messages
        let mut rx = dispatcher.subscribe().await;
        dispatcher.broadcast(RouterState::Start).await;

        assert_eq!(rx.recv().await.unwrap(), RouterState::Start);
    }
}
