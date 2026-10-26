use crate::notification::AdsNotification;
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Mutex, mpsc};
use tcads_core::protocol::AdsNotificationSample;
use tcads_core::{AdsNotificationHandle, AdsReturnCode, AmsAddr, InvokeId, WindowsFileTime};

/// Identifies an active subscription: the device that issued the handle, and the handle.
///
/// Each ADS device assigns handles on its own, so two devices reached over the same
/// connection (for example, the PLC on port 851 and the logger on port 100) can return
/// the same [`AdsNotificationHandle`]. The handle is only unique together with its device.
pub type SubscriptionKey = (AmsAddr, AdsNotificationHandle);

/// Manages ADS device notification subscriptions.
///
/// Subscriptions follow a two-phase lifecycle:
///
/// 1. **Pre-registration**: Before the [`AdsAddDeviceNotificationRequest`](tcads_core::protocol::AdsAddDeviceNotificationRequest)
///    is sent, a [`Sender`] is registered under the request's [`InvokeId`] via [`pre_register`](Self::pre_register).
///    This ensures no samples are lost if the PLC sends a notification before the response arrives.
///
/// 2. **Promotion**: Once the [`AdsAddDeviceNotificationResponse`](tcads_core::protocol::AdsAddDeviceNotificationResponse)
///    is received, the entry is re-keyed from its temporary [`InvokeId`] to a
///    [`SubscriptionKey`] (target address plus assigned [`AdsNotificationHandle`]) via
///    [`promote`](Self::promote).
///
/// Incoming samples are routed by [`dispatch`](Self::dispatch), which is called by the
/// reader thread for each [`AdsNotificationSample`] in an incoming notification frame,
/// matched on the frame's source address and the handle. Each subscriber receives an
/// [`AdsNotification`] carrying the sample's data and the timestamp of the stamp header
/// it arrived in. Dead receivers are pruned silently on dispatch.
pub struct AdsNotificationDispatcher {
    /// Temporary storage keyed by invoke ID, waiting for handle assignment from the PLC.
    pending: Mutex<HashMap<InvokeId, Sender<AdsNotification>>>,
    /// Temporary storage for batch operations, where one invoke ID yields multiple handles.
    pending_batches: Mutex<HashMap<InvokeId, Vec<Sender<AdsNotification>>>>,
    /// Permanent storage keyed by [`SubscriptionKey`] once the device assigns a handle.
    subscriptions: Mutex<HashMap<SubscriptionKey, Sender<AdsNotification>>>,
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
    pub fn record_route(&self, target: AmsAddr, source: AmsAddr) -> crate::Result<()> {
        let mut routes = self.routes.lock()?;
        if target == source {
            routes.remove(&target);
        } else {
            routes.insert(target, source);
        }
        Ok(())
    }

    /// Returns the address notifications from `target` arrive from.
    fn resolve(&self, target: AmsAddr) -> crate::Result<AmsAddr> {
        Ok(self.routes.lock()?.get(&target).copied().unwrap_or(target))
    }

    /// Registers a subscription under a temporary [`InvokeId`] key.
    ///
    /// Must be called before dispatching the add notification request to the PLC,
    /// since the PLC may send an initial sample before the response is received.
    ///
    /// Returns a [`Receiver`] that will yield an [`AdsNotification`] for each incoming
    /// sample on this subscription.
    pub fn pre_register(&self, invoke_id: InvokeId) -> crate::Result<Receiver<AdsNotification>> {
        let (tx, rx) = mpsc::channel();
        self.pending.lock()?.insert(invoke_id, tx);
        Ok(rx)
    }

    /// Registers a batch of subscriptions under a single temporary [`InvokeId`] key.
    ///
    /// Must be called before dispatching the add notification request to the PLC,
    /// since the PLC may send an initial sample before the response is received.
    ///
    /// Used by Sum Commands where a single network round-trip subscribes to multiple variables.
    ///
    /// Returns one [`Receiver`] per requested subscription, in request order.
    pub fn pre_register_batch(
        &self,
        invoke_id: InvokeId,
        count: usize,
    ) -> crate::Result<Vec<Receiver<AdsNotification>>> {
        let mut receivers = Vec::with_capacity(count);
        let mut senders = Vec::with_capacity(count);

        for _ in 0..count {
            let (tx, rx) = mpsc::channel();
            senders.push(tx);
            receivers.push(rx);
        }

        self.pending_batches.lock()?.insert(invoke_id, senders);
        Ok(receivers)
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
    pub fn promote(
        &self,
        invoke_id: InvokeId,
        source: AmsAddr,
        handle: AdsNotificationHandle,
    ) -> crate::Result<bool> {
        let sender = self.pending.lock()?.remove(&invoke_id);

        match sender {
            Some(tx) => {
                self.subscriptions.lock()?.insert((source, handle), tx);
                Ok(true)
            }
            None => Ok(false),
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
    pub fn promote_batch(
        &self,
        invoke_id: InvokeId,
        source: AmsAddr,
        results: &[Result<AdsNotificationHandle, AdsReturnCode>],
    ) -> crate::Result<bool> {
        let pending_senders = self.pending_batches.lock()?.remove(&invoke_id);

        match pending_senders {
            Some(senders) => {
                let mut subs = self.subscriptions.lock()?;

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

    /// Routes an incoming [`AdsNotificationSample`] to the registered subscriber as an
    /// [`AdsNotification`].
    ///
    /// Called by the reader thread for each `(timestamp, sample)` pair yielded by
    /// [`AdsDeviceNotification::iter_samples`](tcads_core::protocol::AdsDeviceNotification::iter_samples),
    /// with `source` taken from the frame's AMS header. `timestamp` is the one from the
    /// sample's enclosing stamp header.
    ///
    /// The sample's data is only copied when a subscriber is registered for
    /// `(source, handle)`. Otherwise, the sample is dropped silently. If the subscriber's
    /// receiver has been dropped, the entry is pruned.
    pub fn dispatch(
        &self,
        source: AmsAddr,
        timestamp: WindowsFileTime,
        sample: &AdsNotificationSample<'_>,
    ) -> crate::Result<()> {
        let key = (source, sample.handle());
        let mut map = self.subscriptions.lock()?;

        let dead = map.get(&key).is_some_and(|tx| {
            tx.send(AdsNotification::new(timestamp, sample.data().to_vec()))
                .is_err()
        });

        if dead {
            map.remove(&key);
        }

        Ok(())
    }

    /// Removes the subscription for `handle` on `target`, dropping the sender.
    ///
    /// `target` is the address the delete request was sent to. It is resolved through
    /// [`record_route`](Self::record_route), so the loopback alias works here too.
    pub fn remove(&self, target: AmsAddr, handle: AdsNotificationHandle) -> crate::Result<()> {
        let source = self.resolve(target)?;
        self.subscriptions.lock()?.remove(&(source, handle));
        Ok(())
    }

    /// Clears all pending and active subscriptions.
    pub fn clear(&self) -> crate::Result<()> {
        self.pending.lock()?.clear();
        self.pending_batches.lock()?.clear();
        self.subscriptions.lock()?.clear();
        // A router restart can change its Net ID, so learn the routes again.
        self.routes.lock()?.clear();
        Ok(())
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
    const TS: WindowsFileTime = WindowsFileTime::from_raw(133_503_504_000_000_000);

    fn addr(port: u16) -> AmsAddr {
        AmsAddr::from_local(port)
    }

    /// Dispatches 4 bytes of `byte` on `handle` from `source`, and returns what the
    /// subscriber should receive.
    fn send(
        dispatcher: &AdsNotificationDispatcher,
        source: AmsAddr,
        handle: AdsNotificationHandle,
        byte: u8,
    ) -> AdsNotification {
        let data = [byte; 4];
        dispatcher
            .dispatch(source, TS, &AdsNotificationSample::new(handle, &data))
            .unwrap();
        AdsNotification::new(TS, data.to_vec())
    }

    #[test]
    fn pre_register_and_promote_routes_samples() {
        let dispatcher = AdsNotificationDispatcher::new();
        let handle = AdsNotificationHandle::from(42u32);

        let rx = dispatcher.pre_register(1).unwrap();
        assert!(dispatcher.promote(1, addr(PLC), handle).unwrap());

        let expected = send(&dispatcher, addr(PLC), handle, 1);

        assert_eq!(rx.recv().unwrap(), expected);
    }

    #[test]
    fn dispatch_carries_the_stamp_timestamp() {
        let dispatcher = AdsNotificationDispatcher::new();
        let handle = AdsNotificationHandle::from(42u32);
        let ts = WindowsFileTime::from_raw(133_503_504_100_000_000);

        let rx = dispatcher.pre_register(1).unwrap();
        dispatcher.promote(1, addr(PLC), handle).unwrap();

        dispatcher
            .dispatch(addr(PLC), ts, &AdsNotificationSample::new(handle, &[7]))
            .unwrap();

        let n = rx.recv().unwrap();
        assert_eq!(n.timestamp(), ts);
        assert_eq!(n.value(), &vec![7]);
    }

    #[test]
    fn dispatch_before_promote_is_dropped() {
        let dispatcher = AdsNotificationDispatcher::new();
        let handle = AdsNotificationHandle::from(42u32);

        send(&dispatcher, addr(PLC), handle, 1);

        assert!(dispatcher.subscriptions.lock().unwrap().is_empty());
    }

    #[test]
    fn dispatch_from_other_source_is_dropped() {
        let dispatcher = AdsNotificationDispatcher::new();
        let handle = AdsNotificationHandle::from(1u32);

        let rx = dispatcher.pre_register(1).unwrap();
        dispatcher.promote(1, addr(PLC), handle).unwrap();

        send(&dispatcher, addr(LOGGER), handle, 1);

        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn dispatch_prunes_dead_receiver() {
        let dispatcher = AdsNotificationDispatcher::new();
        let handle = AdsNotificationHandle::from(1u32);

        let rx = dispatcher.pre_register(1).unwrap();
        dispatcher.promote(1, addr(PLC), handle).unwrap();

        drop(rx);

        send(&dispatcher, addr(PLC), handle, 1);

        assert!(dispatcher.subscriptions.lock().unwrap().is_empty());
    }

    #[test]
    fn remove_causes_receiver_to_return_err() {
        let dispatcher = AdsNotificationDispatcher::new();
        let handle = AdsNotificationHandle::from(1u32);

        let rx = dispatcher.pre_register(1).unwrap();
        dispatcher.promote(1, addr(PLC), handle).unwrap();
        dispatcher.remove(addr(PLC), handle).unwrap();

        assert!(rx.recv().is_err());
    }

    /// The PLC and the logger both hand out handle 1. Each subscription must
    /// keep its own samples, and removing one must not close the other.
    #[test]
    fn same_handle_on_two_devices_does_not_collide() {
        let dispatcher = AdsNotificationDispatcher::new();
        let handle = AdsNotificationHandle::from(1u32);

        let plc_rx = dispatcher.pre_register(1).unwrap();
        dispatcher.promote(1, addr(PLC), handle).unwrap();

        let logger_rx = dispatcher.pre_register(2).unwrap();
        dispatcher.promote(2, addr(LOGGER), handle).unwrap();

        let plc_expected = send(&dispatcher, addr(PLC), handle, 0xAA);
        let logger_expected = send(&dispatcher, addr(LOGGER), handle, 0xBB);

        assert_eq!(plc_rx.recv().unwrap(), plc_expected);
        assert_eq!(logger_rx.recv().unwrap(), logger_expected);
        assert!(plc_rx.try_recv().is_err());
        assert!(logger_rx.try_recv().is_err());

        dispatcher.remove(addr(PLC), handle).unwrap();

        assert!(plc_rx.recv().is_err());
        let logger_expected = send(&dispatcher, addr(LOGGER), handle, 0xBB);
        assert_eq!(logger_rx.recv().unwrap(), logger_expected);
    }

    #[test]
    fn promote_batch_keys_by_source() {
        let dispatcher = AdsNotificationDispatcher::new();
        let h1 = AdsNotificationHandle::from(1u32);
        let h2 = AdsNotificationHandle::from(2u32);

        let logger_rx = dispatcher.pre_register(1).unwrap();
        dispatcher.promote(1, addr(LOGGER), h1).unwrap();

        let batch = dispatcher.pre_register_batch(2, 3).unwrap();
        let results = [Ok(h1), Err(AdsReturnCode::from(0x0710u32)), Ok(h2)];
        assert!(dispatcher.promote_batch(2, addr(PLC), &results).unwrap());

        // The failed entry's sender was dropped.
        assert!(batch[1].recv().is_err());

        let expected = send(&dispatcher, addr(PLC), h1, 0xAA);
        assert_eq!(batch[0].recv().unwrap(), expected);
        assert!(logger_rx.try_recv().is_err());
    }

    #[test]
    fn clear_wakes_all_subscribers() {
        let dispatcher = AdsNotificationDispatcher::new();
        let h1 = AdsNotificationHandle::from(1u32);
        let h2 = AdsNotificationHandle::from(2u32);

        let rx1 = dispatcher.pre_register(1).unwrap();
        let rx2 = dispatcher.pre_register(2).unwrap();
        let batch = dispatcher.pre_register_batch(3, 1).unwrap();
        dispatcher.promote(1, addr(PLC), h1).unwrap();
        dispatcher.promote(2, addr(LOGGER), h2).unwrap();

        dispatcher.clear().unwrap();

        assert!(rx1.recv().is_err());
        assert!(rx2.recv().is_err());
        assert!(batch[0].recv().is_err());
    }

    #[test]
    fn promote_returns_false_for_unknown_invoke_id() {
        let dispatcher = AdsNotificationDispatcher::new();
        let handle = AdsNotificationHandle::from(1u32);

        assert!(!dispatcher.promote(999, addr(PLC), handle).unwrap());
    }

    #[test]
    fn remove_resolves_loopback_through_recorded_route() {
        let dispatcher = AdsNotificationDispatcher::new();
        let handle = AdsNotificationHandle::from(1u32);
        let real: AmsAddr = "39.139.122.3.1.1:100".parse().unwrap();

        let rx = dispatcher.pre_register(1).unwrap();
        dispatcher.promote(1, real, handle).unwrap();
        dispatcher.record_route(addr(LOGGER), real).unwrap();

        dispatcher.remove(addr(LOGGER), handle).unwrap();

        assert!(rx.recv().is_err());
    }

    #[test]
    fn clear_forgets_routes() {
        let dispatcher = AdsNotificationDispatcher::new();
        let real: AmsAddr = "39.139.122.3.1.1:100".parse().unwrap();

        dispatcher.record_route(addr(LOGGER), real).unwrap();
        dispatcher.clear().unwrap();

        assert!(dispatcher.routes.lock().unwrap().is_empty());
    }
}
