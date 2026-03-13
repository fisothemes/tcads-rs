use tcads_core::RouterState;
use tokio::sync::broadcast;

/// Fans out router state changes to all registered subscribers using a
/// [`broadcast`] channel.
///
/// Unlike the blocking variant which uses `Vec<Sender>` with manual pruning,
/// the tokio variant delegates fan-out and subscriber lifecycle to
/// [`tokio::sync::broadcast`]. Lagged receivers receive
/// [`RecvError::Lagged`](broadcast::error::RecvError::Lagged) rather than
/// blocking the broadcaster.
///
/// # Capacity
///
/// The broadcast channel holds up to a [`capacity`](Self::capacity) of unread
/// messages per subscriber. Router state changes are infrequent, so a small
/// capacity is enough.
pub struct RouterNotificationDispatcher {
    tx: broadcast::Sender<RouterState>,
    capacity: usize,
}

impl RouterNotificationDispatcher {
    /// Creates a new dispatcher.
    ///
    /// # Panics
    ///
    /// This will panic if capacity is equal to 0.
    /// See [`broadcast::channel`] for more details.
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx, capacity }
    }

    /// Registers a new subscriber and returns a [`broadcast::Receiver<RouterState>`].
    ///
    /// The receiver yields every [`RouterState`] broadcast until the dispatcher
    /// is dropped or the connection is lost.
    pub fn subscribe(&self) -> broadcast::Receiver<RouterState> {
        self.tx.subscribe()
    }

    /// Broadcasts `state` to all live subscribers.
    ///
    /// Called by the reader task on every incoming
    /// [`RouterNotification`](tcads_core::protocol::RouterNotification) frame.
    pub fn broadcast(&self, state: RouterState) {
        let _ = self.tx.send(state);
    }

    /// Broadcast channel capacity.
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn single_subscriber_receives_broadcast() {
        let dispatcher = RouterNotificationDispatcher::new(16);
        let mut rx = dispatcher.subscribe();

        dispatcher.broadcast(RouterState::Removed);

        assert_eq!(rx.recv().await.unwrap(), RouterState::Removed);
    }

    #[tokio::test]
    async fn multiple_subscribers_all_receive_broadcast() {
        let dispatcher = RouterNotificationDispatcher::new(16);
        let mut rx1 = dispatcher.subscribe();
        let mut rx2 = dispatcher.subscribe();
        let mut rx3 = dispatcher.subscribe();

        dispatcher.broadcast(RouterState::Start);

        assert_eq!(rx1.recv().await.unwrap(), RouterState::Start);
        assert_eq!(rx2.recv().await.unwrap(), RouterState::Start);
        assert_eq!(rx3.recv().await.unwrap(), RouterState::Start);
    }

    #[tokio::test]
    async fn multiple_broadcasts_received_in_order() {
        let dispatcher = RouterNotificationDispatcher::new(16);
        let mut rx = dispatcher.subscribe();

        dispatcher.broadcast(RouterState::Stop);
        dispatcher.broadcast(RouterState::Start);
        dispatcher.broadcast(RouterState::Removed);

        assert_eq!(rx.recv().await.unwrap(), RouterState::Stop);
        assert_eq!(rx.recv().await.unwrap(), RouterState::Start);
        assert_eq!(rx.recv().await.unwrap(), RouterState::Removed);
    }

    #[tokio::test]
    async fn dropped_dispatcher_closes_receivers() {
        let dispatcher = RouterNotificationDispatcher::new(16);
        let mut rx = dispatcher.subscribe();

        drop(dispatcher);

        assert!(rx.recv().await.is_err());
    }

    #[tokio::test]
    async fn subscribe_after_broadcasts_starts_fresh() {
        let dispatcher = RouterNotificationDispatcher::new(16);

        dispatcher.broadcast(RouterState::Stop);

        // Subscribe after broadcast, should not receive past messages
        let mut rx = dispatcher.subscribe();
        dispatcher.broadcast(RouterState::Start);

        assert_eq!(rx.recv().await.unwrap(), RouterState::Start);
    }
}
