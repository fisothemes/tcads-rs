use std::collections::HashMap;
use tcads_core::{
    AdsNotificationHandle, AdsNotificationSampleOwned, AdsReturnCode, AmsAddr, InvokeId,
};
use tokio::sync::Mutex;
use tokio::sync::mpsc::{self, UnboundedReceiver as Receiver, UnboundedSender as Sender};

/// Identifies an active subscription: the device that issued the handle, and the handle.
///
/// Each ADS device assigns handles on its own, so two devices reached over the same
/// connection (for example the PLC on port 851 and the logger on port 100) can return
/// the same [`AdsNotificationHandle`]. The handle is only unique together with its device.
pub type SubscriptionKey = (AmsAddr, AdsNotificationHandle);

/// Manages ADS device notification subscriptions.
///
/// See the [blocking equivalent](crate::tasks::blocking::AdsNotificationDispatcher) for
/// full design documentation. The tokio variant is identical in shape, and the only
/// difference is [`tokio::sync::Mutex`] and [`tokio::sync::mpsc`] channels.
pub struct AdsNotificationDispatcher {
    /// Temporary storage keyed by invoke ID, waiting for handle assignment from the device.
    pending: Mutex<HashMap<InvokeId, Sender<AdsNotificationSampleOwned>>>,
    /// Temporary storage for batch operations, where one invoke ID yields multiple handles.
    pending_batches: Mutex<HashMap<InvokeId, Vec<Sender<AdsNotificationSampleOwned>>>>,
    /// Permanent storage keyed by [`SubscriptionKey`] once the device assigns a handle.
    subscriptions: Mutex<HashMap<SubscriptionKey, Sender<AdsNotificationSampleOwned>>>,
    /// The address each requested target answered from, when the two differ.
    ///
    /// A request to `127.0.0.1.1.1` is answered, and its notifications sent, from the
    /// real Net ID of whichever router resolved it. See [`record_route`](Self::record_route).
    routes: Mutex<HashMap<AmsAddr, AmsAddr>>,
}

impl AdsNotificationDispatcher {
    /// Creates a new dispatcher with empty pending and subscription maps.
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(HashMap::new()),
            pending_batches: Mutex::new(HashMap::new()),
            subscriptions: Mutex::new(HashMap::new()),
            routes: Mutex::new(HashMap::new()),
        }
    }

    /// Records that requests sent to `target` were answered from `source`.
    ///
    /// Called with the source address of every add notification response. Later calls
    /// to [`remove`](Self::remove) with `target` then find the subscription under the
    /// address its notifications actually arrive from. This works on local and remote
    /// routers alike, because the router reports the address and nothing is guessed.
    pub async fn record_route(&self, target: AmsAddr, source: AmsAddr) {
        let mut routes = self.routes.lock().await;
        if target == source {
            routes.remove(&target);
        } else {
            routes.insert(target, source);
        }
    }

    /// Returns the address notifications from `target` arrive from.
    async fn resolve(&self, target: AmsAddr) -> AmsAddr {
        self.routes
            .lock()
            .await
            .get(&target)
            .copied()
            .unwrap_or(target)
    }

    /// Registers a subscription under a temporary [`InvokeId`] key.
    ///
    /// Must be called before dispatching the add notification request to the device,
    /// since the device may send an initial sample before the response is received.
    ///
    /// Returns a [`Receiver`] that will yield incoming [`AdsNotificationSample`](AdsNotificationSampleOwned)s
    /// for this subscription.
    pub async fn pre_register(&self, invoke_id: InvokeId) -> Receiver<AdsNotificationSampleOwned> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.pending.lock().await.insert(invoke_id, tx);
        rx
    }

    /// Registers a batch of subscriptions under a single temporary [`InvokeId`] key.
    ///
    /// Must be called before dispatching the add notification request to the device,
    /// since the device may send an initial sample before the response is received.
    ///
    /// Used by Sum Commands where a single network round-trip subscribes to multiple variables.
    ///
    /// Returns a collection of [`Receiver`]s that will yield incoming
    /// [`AdsNotificationSample`](AdsNotificationSampleOwned)s for this subscription.
    pub async fn pre_register_batch(
        &self,
        invoke_id: InvokeId,
        count: usize,
    ) -> Vec<Receiver<AdsNotificationSampleOwned>> {
        let mut receivers = Vec::with_capacity(count);
        let mut senders = Vec::with_capacity(count);

        for _ in 0..count {
            let (tx, rx) = mpsc::unbounded_channel();
            senders.push(tx);
            receivers.push(rx);
        }

        self.pending_batches.lock().await.insert(invoke_id, senders);
        receivers
    }

    /// Promotes a pre-registered subscription from its temporary [`InvokeId`] key
    /// to the permanent `(source, handle)` key.
    ///
    /// `source` must be the source address of the add notification response, not the
    /// address the request was sent to. The reader matches incoming samples against
    /// the source address of the notification frame, and the two differ when the
    /// request went to the loopback alias `127.0.0.1.1.1`.
    ///
    /// Should be called immediately after the [`AdsAddDeviceNotificationResponse`](tcads_core::protocol::AdsAddDeviceNotificationResponse)
    /// is received.
    ///
    /// Returns `false` if no pre-registered entry was found for `invoke_id`.
    pub async fn promote(
        &self,
        invoke_id: InvokeId,
        source: AmsAddr,
        handle: AdsNotificationHandle,
    ) -> bool {
        let sender = self.pending.lock().await.remove(&invoke_id);
        match sender {
            Some(tx) => {
                self.subscriptions.lock().await.insert((source, handle), tx);
                true
            }
            None => false,
        }
    }

    /// Promotes a pre-registered batch of subscriptions answered from `source`.
    ///
    /// As with [`promote`](Self::promote), `source` is the response's source address.
    ///
    /// The `results` array must exactly match the order and length of the batch
    /// requested in `pre_register_batch`. Senders corresponding to `Ok` results
    /// are promoted to active subscriptions. Senders corresponding to `Err` results
    /// are dropped, safely closing the channel for the user.
    pub async fn promote_batch(
        &self,
        invoke_id: InvokeId,
        source: AmsAddr,
        results: &[Result<AdsNotificationHandle, AdsReturnCode>],
    ) -> crate::Result<bool> {
        let pending_senders = self.pending_batches.lock().await.remove(&invoke_id);

        match pending_senders {
            Some(senders) => {
                let mut subs = self.subscriptions.lock().await;

                for (sender, result) in senders.into_iter().zip(results.iter()) {
                    // Only insert successful registrations into the active routing table
                    if let Ok(handle) = result {
                        subs.insert((source, *handle), sender);
                    }
                    // If it's an Err, `sender` falls out of scope and is dropped.
                }
                Ok(true)
            }
            None => Ok(false),
        }
    }

    /// Routes an incoming [`AdsNotificationSample`](AdsNotificationSampleOwned)
    /// to the registered subscriber.
    ///
    /// Called by the reader task for each sample in an incoming
    /// [`AdsDeviceNotification`](tcads_core::protocol::AdsDeviceNotification) frame,
    /// with `source` taken from the frame's AMS header.
    ///
    /// If no subscriber is registered for `(source, handle)` the sample is dropped silently.
    /// If the subscriber's receiver has been dropped the entry is pruned.
    pub async fn dispatch(
        &self,
        source: AmsAddr,
        handle: AdsNotificationHandle,
        sample: AdsNotificationSampleOwned,
    ) {
        let key = (source, handle);
        let mut map = self.subscriptions.lock().await;
        let dead = map.get(&key).is_some_and(|tx| tx.send(sample).is_err());
        if dead {
            map.remove(&key);
        }
    }

    /// Removes the subscription for `handle` on `target`, dropping the sender.
    ///
    /// `target` is the address the delete request was sent to. It is resolved through
    /// [`record_route`](Self::record_route), so the loopback alias works here too.
    pub async fn remove(&self, target: AmsAddr, handle: AdsNotificationHandle) {
        let source = self.resolve(target).await;
        self.subscriptions.lock().await.remove(&(source, handle));
    }

    /// Clears all pending and active subscriptions.
    pub async fn clear(&self) {
        self.pending.lock().await.clear();
        self.pending_batches.lock().await.clear();
        self.subscriptions.lock().await.clear();
        // A router restart can change its Net ID, so learn the routes again.
        self.routes.lock().await.clear();
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

    const PLC: u16 = 851;
    const LOGGER: u16 = 100;

    fn addr(port: u16) -> AmsAddr {
        AmsAddr::from_local(port)
    }

    fn make_sample(handle: AdsNotificationHandle, data: u8) -> AdsNotificationSampleOwned {
        AdsNotificationSampleOwned::new(handle, vec![data; 4])
    }

    #[tokio::test]
    async fn pre_register_and_promote_routes_samples() {
        let dispatcher = AdsNotificationDispatcher::new();
        let handle = AdsNotificationHandle::from(42u32);

        let mut rx = dispatcher.pre_register(1).await;
        assert!(dispatcher.promote(1, addr(PLC), handle).await);

        let sample = make_sample(handle, 1);
        dispatcher.dispatch(addr(PLC), handle, sample.clone()).await;

        assert_eq!(rx.recv().await.unwrap(), sample);
    }

    #[tokio::test]
    async fn dispatch_before_promote_is_dropped() {
        let dispatcher = AdsNotificationDispatcher::new();
        let handle = AdsNotificationHandle::from(42u32);

        dispatcher
            .dispatch(addr(PLC), handle, make_sample(handle, 1))
            .await;

        assert!(dispatcher.subscriptions.lock().await.is_empty());
    }

    #[tokio::test]
    async fn dispatch_from_other_source_is_dropped() {
        let dispatcher = AdsNotificationDispatcher::new();
        let handle = AdsNotificationHandle::from(1u32);

        let mut rx = dispatcher.pre_register(1).await;
        dispatcher.promote(1, addr(PLC), handle).await;

        dispatcher
            .dispatch(addr(LOGGER), handle, make_sample(handle, 1))
            .await;

        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn dispatch_prunes_dead_receiver() {
        let dispatcher = AdsNotificationDispatcher::new();
        let handle = AdsNotificationHandle::from(1u32);

        let rx = dispatcher.pre_register(1).await;
        dispatcher.promote(1, addr(PLC), handle).await;

        drop(rx);

        dispatcher
            .dispatch(addr(PLC), handle, make_sample(handle, 1))
            .await;

        assert!(dispatcher.subscriptions.lock().await.is_empty());
    }

    #[tokio::test]
    async fn remove_causes_receiver_to_return_none() {
        let dispatcher = AdsNotificationDispatcher::new();
        let handle = AdsNotificationHandle::from(1u32);

        let mut rx = dispatcher.pre_register(1).await;
        dispatcher.promote(1, addr(PLC), handle).await;
        dispatcher.remove(addr(PLC), handle).await;

        assert!(rx.recv().await.is_none());
    }

    /// The PLC and the logger both hand out handle 1. Each subscription must
    /// keep its own samples, and removing one must not close the other.
    #[tokio::test]
    async fn same_handle_on_two_devices_does_not_collide() {
        let dispatcher = AdsNotificationDispatcher::new();
        let handle = AdsNotificationHandle::from(1u32);

        let mut plc_rx = dispatcher.pre_register(1).await;
        dispatcher.promote(1, addr(PLC), handle).await;

        let mut logger_rx = dispatcher.pre_register(2).await;
        dispatcher.promote(2, addr(LOGGER), handle).await;

        let plc_sample = make_sample(handle, 0xAA);
        let logger_sample = make_sample(handle, 0xBB);
        dispatcher
            .dispatch(addr(PLC), handle, plc_sample.clone())
            .await;
        dispatcher
            .dispatch(addr(LOGGER), handle, logger_sample.clone())
            .await;

        assert_eq!(plc_rx.recv().await.unwrap(), plc_sample);
        assert_eq!(logger_rx.recv().await.unwrap(), logger_sample);
        assert!(plc_rx.try_recv().is_err());
        assert!(logger_rx.try_recv().is_err());

        dispatcher.remove(addr(PLC), handle).await;

        assert!(plc_rx.recv().await.is_none());
        dispatcher
            .dispatch(addr(LOGGER), handle, logger_sample.clone())
            .await;
        assert_eq!(logger_rx.recv().await.unwrap(), logger_sample);
    }

    #[tokio::test]
    async fn promote_batch_keys_by_source() {
        let dispatcher = AdsNotificationDispatcher::new();
        let h1 = AdsNotificationHandle::from(1u32);
        let h2 = AdsNotificationHandle::from(2u32);

        let mut logger_rx = dispatcher.pre_register(1).await;
        dispatcher.promote(1, addr(LOGGER), h1).await;

        let mut batch = dispatcher.pre_register_batch(2, 3).await;
        let results = [Ok(h1), Err(AdsReturnCode::from(0x0710u32)), Ok(h2)];
        assert!(
            dispatcher
                .promote_batch(2, addr(PLC), &results)
                .await
                .unwrap()
        );

        // The failed entry's sender was dropped.
        assert!(batch[1].recv().await.is_none());

        let sample = make_sample(h1, 0xAA);
        dispatcher.dispatch(addr(PLC), h1, sample.clone()).await;
        assert_eq!(batch[0].recv().await.unwrap(), sample);
        assert!(logger_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn clear_wakes_all_subscribers() {
        let dispatcher = AdsNotificationDispatcher::new();
        let h1 = AdsNotificationHandle::from(1u32);
        let h2 = AdsNotificationHandle::from(2u32);

        let mut rx1 = dispatcher.pre_register(1).await;
        let mut rx2 = dispatcher.pre_register(2).await;
        let mut batch = dispatcher.pre_register_batch(3, 1).await;
        dispatcher.promote(1, addr(PLC), h1).await;
        dispatcher.promote(2, addr(LOGGER), h2).await;

        dispatcher.clear().await;

        assert!(rx1.recv().await.is_none());
        assert!(rx2.recv().await.is_none());
        assert!(batch[0].recv().await.is_none());
    }

    #[tokio::test]
    async fn promote_returns_false_for_unknown_invoke_id() {
        let dispatcher = AdsNotificationDispatcher::new();
        let handle = AdsNotificationHandle::from(1u32);

        assert!(!dispatcher.promote(999, addr(PLC), handle).await);
    }

    #[tokio::test]
    async fn remove_resolves_loopback_through_recorded_route() {
        let dispatcher = AdsNotificationDispatcher::new();
        let handle = AdsNotificationHandle::from(1u32);
        let real: AmsAddr = "39.139.122.3.1.1:100".parse().unwrap();

        let mut rx = dispatcher.pre_register(1).await;
        dispatcher.promote(1, real, handle).await;
        dispatcher.record_route(addr(LOGGER), real).await;

        dispatcher.remove(addr(LOGGER), handle).await;

        assert!(rx.recv().await.is_none());
    }

    #[tokio::test]
    async fn clear_forgets_routes() {
        let dispatcher = AdsNotificationDispatcher::new();
        let real: AmsAddr = "39.139.122.3.1.1:100".parse().unwrap();

        dispatcher.record_route(addr(LOGGER), real).await;
        dispatcher.clear().await;

        assert!(dispatcher.routes.lock().await.is_empty());
    }
}
