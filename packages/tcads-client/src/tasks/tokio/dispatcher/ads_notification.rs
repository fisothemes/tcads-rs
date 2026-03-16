use std::collections::HashMap;
use tcads_core::{AdsNotificationSampleOwned, InvokeId, NotificationHandle};
use tokio::sync::Mutex;
use tokio::sync::mpsc::{self, UnboundedReceiver as Receiver, UnboundedSender as Sender};

/// Manages ADS device notification subscriptions.
///
/// See the [blocking equivalent](crate::tasks::blocking::AdsNotificationDispatcher) for
/// full design documentation. The tokio variant is identical in shape, and the only
/// difference is [`tokio::sync::Mutex`] and [`tokio::sync::mpsc`] channels.
pub struct AdsNotificationDispatcher {
    pending: Mutex<HashMap<InvokeId, Sender<AdsNotificationSampleOwned>>>,
    subscriptions: Mutex<HashMap<NotificationHandle, Sender<AdsNotificationSampleOwned>>>,
}

impl AdsNotificationDispatcher {
    /// Creates a new dispatcher with empty pending and subscription maps.
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(HashMap::new()),
            subscriptions: Mutex::new(HashMap::new()),
        }
    }

    /// Registers a subscription under a temporary [`InvokeId`] key.
    ///
    /// Must be called before dispatching the add notification request to the PLC,
    /// since the PLC may send an initial sample before the response is received.
    ///
    /// Returns a [`Receiver`] that will yield incoming [`AdsNotificationSample`](AdsNotificationSampleOwned)s
    /// for this subscription.
    pub async fn pre_register(&self, invoke_id: InvokeId) -> Receiver<AdsNotificationSampleOwned> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.pending.lock().await.insert(invoke_id, tx);
        rx
    }

    /// Promotes a pre-registered subscription from its temporary [`InvokeId`] key
    /// to the permanent [`NotificationHandle`] assigned by the PLC.
    ///
    /// Should be called immediately after the [`AdsAddDeviceNotificationResponse`](tcads_core::protocol::AdsAddDeviceNotificationResponse)
    /// is received.
    ///
    /// Returns `false` if no pre-registered entry was found for `invoke_id`.
    pub async fn promote(&self, invoke_id: InvokeId, handle: NotificationHandle) -> bool {
        let sender = self.pending.lock().await.remove(&invoke_id);
        match sender {
            Some(tx) => {
                self.subscriptions.lock().await.insert(handle, tx);
                true
            }
            None => false,
        }
    }

    /// Routes an incoming [`AdsNotificationSample`](AdsNotificationSampleOwned)
    /// to the registered subscriber.
    ///
    /// Called by the reader task for each sample in an incoming
    /// [`AdsDeviceNotification`](tcads_core::protocol::AdsDeviceNotification) frame.
    /// Dead receivers are pruned silently.
    pub async fn dispatch(&self, handle: NotificationHandle, sample: AdsNotificationSampleOwned) {
        let mut map = self.subscriptions.lock().await;
        let dead = if let Some(tx) = map.get(&handle) {
            tx.send(sample).is_err()
        } else {
            false
        };
        if dead {
            map.remove(&handle);
        }
    }

    /// Removes the subscription for a [`NotificationHandle`], dropping the sender.
    pub async fn remove(&self, handle: NotificationHandle) {
        self.subscriptions.lock().await.remove(&handle);
    }

    /// Clears all pending and active subscriptions.
    pub async fn clear(&self) {
        self.pending.lock().await.clear();
        self.subscriptions.lock().await.clear();
    }
}

impl Default for AdsNotificationDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sample(handle: NotificationHandle) -> AdsNotificationSampleOwned {
        AdsNotificationSampleOwned::new(handle, vec![0x01, 0x02, 0x03, 0x04])
    }

    #[tokio::test]
    async fn pre_register_and_promote_routes_samples() {
        let dispatcher = AdsNotificationDispatcher::new();
        let handle = NotificationHandle::new(42);

        let mut rx = dispatcher.pre_register(1).await;
        dispatcher.promote(1, handle).await;

        let sample = make_sample(handle);
        dispatcher.dispatch(handle, sample.clone()).await;

        assert_eq!(rx.recv().await.unwrap(), sample);
    }

    #[tokio::test]
    async fn dispatch_before_promote_is_dropped() {
        let dispatcher = AdsNotificationDispatcher::new();
        let handle = NotificationHandle::new(42);

        let sample = make_sample(handle);
        dispatcher.dispatch(handle, sample).await;

        assert!(dispatcher.subscriptions.lock().await.is_empty());
    }

    #[tokio::test]
    async fn dispatch_prunes_dead_receiver() {
        let dispatcher = AdsNotificationDispatcher::new();
        let handle = NotificationHandle::new(1u32);

        let rx = dispatcher.pre_register(1).await;
        dispatcher.promote(1, handle).await;

        drop(rx);

        let sample = make_sample(handle);
        dispatcher.dispatch(handle, sample).await;

        assert!(dispatcher.subscriptions.lock().await.is_empty());
    }

    #[tokio::test]
    async fn remove_causes_receiver_to_return_none() {
        let dispatcher = AdsNotificationDispatcher::new();
        let handle = NotificationHandle::new(1u32);

        let mut rx = dispatcher.pre_register(1).await;
        dispatcher.promote(1, handle).await;
        dispatcher.remove(handle).await;

        assert!(rx.recv().await.is_none());
    }

    #[tokio::test]
    async fn clear_wakes_all_subscribers() {
        let dispatcher = AdsNotificationDispatcher::new();
        let h1 = NotificationHandle::new(1u32);
        let h2 = NotificationHandle::new(2u32);

        let mut rx1 = dispatcher.pre_register(1).await;
        let mut rx2 = dispatcher.pre_register(2).await;
        dispatcher.promote(1, h1).await;
        dispatcher.promote(2, h2).await;

        dispatcher.clear().await;

        assert!(rx1.recv().await.is_none());
        assert!(rx2.recv().await.is_none());
    }

    #[tokio::test]
    async fn promote_returns_false_for_unknown_invoke_id() {
        let dispatcher = AdsNotificationDispatcher::new();
        let handle = NotificationHandle::new(1u32);

        let promoted = dispatcher.promote(999, handle).await;
        assert!(!promoted);
    }
}
