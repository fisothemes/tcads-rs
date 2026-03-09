use std::sync::Mutex;
use std::sync::mpsc::{self, Receiver, Sender};
use tcads_core::RouterState;

/// Fans out [router state](RouterState) changes to all registered subscribers.
///
/// Subscribers register via [`subscribe`](Self::subscribe), which returns a
/// [`Receiver<RouterState>`]. Every call to [`broadcast`](Self::broadcast) sends
/// the state to all live subscribers. This will subsequently silently prune dead receivers.
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
    /// The receiver will yield every [`RouterState`] broadcast until either:
    /// - The receiver is dropped by the caller, or
    /// - [`clear`](Self::clear) is called on connection loss.
    pub fn subscribe(&self) -> crate::Result<Receiver<RouterState>> {
        let (tx, rx) = mpsc::channel();
        self.subscribers.lock()?.push(tx);
        Ok(rx)
    }

    /// Broadcasts `state` to all live subscribers, pruning dead receivers.
    ///
    /// Called by the reader thread on every incoming
    /// [`RouterNotification`](tcads_core::protocol::RouterNotification) frame.
    pub fn broadcast(&self, state: RouterState) -> crate::Result<()> {
        let mut subscribers = self.subscribers.lock()?;
        subscribers.retain(|tx| tx.send(state).is_ok());
        Ok(())
    }

    /// Drops all subscribers, causing their receivers to return [`Err`].
    ///
    /// Called on connection loss so subscriber loops can exit naturally.
    pub fn clear(&self) -> crate::Result<()> {
        self.subscribers.lock()?.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_subscriber_receives_broadcast() {
        let dispatcher = RouterNotificationDispatcher::new();
        let rx = dispatcher.subscribe().unwrap();

        dispatcher.broadcast(RouterState::Removed).unwrap();

        assert_eq!(rx.recv().unwrap(), RouterState::Removed);
    }

    #[test]
    fn multiple_subscribers_all_receive_broadcast() {
        let dispatcher = RouterNotificationDispatcher::new();
        let rx1 = dispatcher.subscribe().unwrap();
        let rx2 = dispatcher.subscribe().unwrap();
        let rx3 = dispatcher.subscribe().unwrap();

        dispatcher.broadcast(RouterState::Start).unwrap();

        assert_eq!(rx1.recv().unwrap(), RouterState::Start);
        assert_eq!(rx2.recv().unwrap(), RouterState::Start);
        assert_eq!(rx3.recv().unwrap(), RouterState::Start);
    }

    #[test]
    fn multiple_broadcasts_are_all_received() {
        let dispatcher = RouterNotificationDispatcher::new();
        let rx = dispatcher.subscribe().unwrap();

        dispatcher.broadcast(RouterState::Stop).unwrap();
        dispatcher.broadcast(RouterState::Start).unwrap();
        dispatcher.broadcast(RouterState::Removed).unwrap();

        assert_eq!(rx.recv().unwrap(), RouterState::Stop);
        assert_eq!(rx.recv().unwrap(), RouterState::Start);
        assert_eq!(rx.recv().unwrap(), RouterState::Removed);
    }

    #[test]
    fn dead_receivers_are_pruned_on_broadcast() {
        let dispatcher = RouterNotificationDispatcher::new();
        let rx1 = dispatcher.subscribe().unwrap();
        let rx2 = dispatcher.subscribe().unwrap();

        drop(rx2);

        dispatcher.broadcast(RouterState::Stop).unwrap();

        assert_eq!(rx1.recv().unwrap(), RouterState::Stop);
        assert_eq!(dispatcher.subscribers.lock().unwrap().len(), 1);
    }

    #[test]
    fn clear_wakes_all_subscribers_with_err() {
        let dispatcher = RouterNotificationDispatcher::new();
        let rx1 = dispatcher.subscribe().unwrap();
        let rx2 = dispatcher.subscribe().unwrap();

        dispatcher.clear().unwrap();

        assert!(rx1.recv().is_err());
        assert!(rx2.recv().is_err());
    }

    #[test]
    fn subscribe_after_clear_works() {
        let dispatcher = RouterNotificationDispatcher::new();
        let rx1 = dispatcher.subscribe().unwrap();
        dispatcher.clear().unwrap();
        assert!(rx1.recv().is_err());

        let rx2 = dispatcher.subscribe().unwrap();
        dispatcher.broadcast(RouterState::Start).unwrap();
        assert_eq!(rx2.recv().unwrap(), RouterState::Start);
    }
}
