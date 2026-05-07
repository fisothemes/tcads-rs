use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Mutex, mpsc};
use tcads_core::{AdsNotificationSampleOwned, AdsReturnCode, InvokeId, NotificationHandle};

/// Manages ADS device notification subscriptions.
///
/// Subscriptions follow a two-phase lifecycle:
///
/// 1. **Pre-registration**: Before the [`AdsAddDeviceNotificationRequest`](tcads_core::protocol::AdsAddDeviceNotificationRequest)
///    is sent, a [`Sender`] is registered under the request's [`InvokeId`] via [`pre_register`](Self::pre_register).
///    This ensures no samples are lost if the PLC sends a notification before the response arrives.
///
/// 2. **Promotion**: Once the [`AdsAddDeviceNotificationResponse`](tcads_core::protocol::AdsAddDeviceNotificationResponse)
///    is received, the entry is re-keyed from its temporary [`InvokeId`] to the assigned
///    [`NotificationHandle`] via [`promote`](Self::promote).
///
/// Incoming samples are routed by [`dispatch`](Self::dispatch), which is called by the
/// reader thread for each [`AdsNotificationSample`](AdsNotificationSampleOwned) in an
/// incoming notification frame. Dead receivers are pruned silently on dispatch.
pub struct AdsNotificationDispatcher {
    /// Temporary storage keyed by invoke ID, waiting for handle assignment from the PLC.
    pending: Mutex<HashMap<InvokeId, Sender<AdsNotificationSampleOwned>>>,
    /// Temporary storage for batch operations, where one invoke ID yields multiple handles.
    pending_batches: Mutex<HashMap<InvokeId, Vec<Sender<AdsNotificationSampleOwned>>>>,
    /// Permanent storage keyed by notification handle once assigned by the PLC.
    subscriptions: Mutex<HashMap<NotificationHandle, Sender<AdsNotificationSampleOwned>>>,
}

impl AdsNotificationDispatcher {
    /// Creates a new dispatcher with empty pending and subscription maps.
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(HashMap::new()),
            pending_batches: Mutex::new(HashMap::new()),
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
    pub fn pre_register(
        &self,
        invoke_id: InvokeId,
    ) -> crate::Result<Receiver<AdsNotificationSampleOwned>> {
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
    /// Returns a collection of [`Receiver`]s that will yield incoming
    /// [`AdsNotificationSample`](AdsNotificationSampleOwned)s for this subscription.
    pub fn pre_register_batch(
        &self,
        invoke_id: InvokeId,
        count: usize,
    ) -> crate::Result<Vec<Receiver<AdsNotificationSampleOwned>>> {
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
    /// to the permanent [`NotificationHandle`] assigned by the PLC.
    ///
    /// Should be called immediately after the [`AdsAddDeviceNotificationResponse`](tcads_core::protocol::AdsAddDeviceNotificationResponse)
    /// is received.
    ///
    /// Returns `false` if no pre-registered entry was found for `invoke_id`.
    pub fn promote(&self, invoke_id: InvokeId, handle: NotificationHandle) -> crate::Result<bool> {
        let sender = self.pending.lock()?.remove(&invoke_id);

        match sender {
            Some(tx) => {
                self.subscriptions.lock()?.insert(handle, tx);
                Ok(true)
            }
            None => Ok(false),
        }
    }

    /// Promotes a pre-registered batch of subscriptions.
    ///
    /// The `results` array must exactly match the order and length of the batch
    /// requested in `pre_register_batch`. Senders corresponding to `Ok` results
    /// are promoted to active subscriptions. Senders corresponding to `Err` results
    /// are dropped, safely closing the channel for the user.
    pub fn promote_batch(
        &self,
        invoke_id: InvokeId,
        results: &[Result<NotificationHandle, AdsReturnCode>],
    ) -> crate::Result<bool> {
        let pending_senders = self.pending_batches.lock()?.remove(&invoke_id);

        match pending_senders {
            Some(senders) => {
                let mut subs = self.subscriptions.lock()?;

                for (sender, result) in senders.into_iter().zip(results.iter()) {
                    // Only insert successful registrations into the active routing table
                    if let Ok(handle) = result {
                        subs.insert(*handle, sender);
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
    /// Called by the reader thread for each sample in an incoming
    /// [`AdsDeviceNotification`](tcads_core::protocol::AdsDeviceNotification) frame.
    ///
    /// If no subscriber is registered for the handle the sample is dropped silently.
    /// If the subscriber's receiver has been dropped the entry is pruned.
    pub fn dispatch(
        &self,
        handle: NotificationHandle,
        sample: AdsNotificationSampleOwned,
    ) -> crate::Result<()> {
        let mut map = self.subscriptions.lock()?;

        let dead = if let Some(tx) = map.get(&handle) {
            tx.send(sample).is_err()
        } else {
            false
        };

        if dead {
            map.remove(&handle);
        }

        Ok(())
    }

    /// Removes the subscription for a [`NotificationHandle`], dropping the sender.
    pub fn remove(&self, handle: NotificationHandle) -> crate::Result<()> {
        self.subscriptions.lock()?.remove(&handle);
        Ok(())
    }

    /// Clears all pending and active subscriptions.
    pub fn clear(&self) -> crate::Result<()> {
        self.pending.lock()?.clear();
        self.pending_batches.lock()?.clear();
        self.subscriptions.lock()?.clear();
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

    fn make_sample(handle: NotificationHandle) -> AdsNotificationSampleOwned {
        AdsNotificationSampleOwned::new(handle, vec![0x01, 0x02, 0x03, 0x04])
    }

    #[test]
    fn pre_register_and_promote_routes_samples() {
        let dispatcher = AdsNotificationDispatcher::new();
        let handle = NotificationHandle::from(42);

        let rx = dispatcher.pre_register(1).unwrap();
        dispatcher.promote(1, handle).unwrap();

        let sample = make_sample(handle);
        dispatcher.dispatch(handle, sample.clone()).unwrap();

        assert_eq!(rx.recv().unwrap(), sample);
    }

    #[test]
    fn dispatch_before_promote_is_dropped() {
        let dispatcher = AdsNotificationDispatcher::new();
        let handle = NotificationHandle::from(42);

        let sample = make_sample(handle);
        assert!(dispatcher.dispatch(handle, sample).is_ok());
    }

    #[test]
    fn dispatch_prunes_dead_receiver() {
        let dispatcher = AdsNotificationDispatcher::new();
        let handle = NotificationHandle::from(1u32);

        let rx = dispatcher.pre_register(1).unwrap();
        dispatcher.promote(1, handle).unwrap();

        drop(rx);

        let sample = make_sample(handle);
        dispatcher.dispatch(handle, sample).unwrap();

        assert!(dispatcher.subscriptions.lock().unwrap().is_empty());
    }

    #[test]
    fn remove_causes_receiver_to_return_err() {
        let dispatcher = AdsNotificationDispatcher::new();
        let handle = NotificationHandle::from(1u32);

        let rx = dispatcher.pre_register(1).unwrap();
        dispatcher.promote(1, handle).unwrap();
        dispatcher.remove(handle).unwrap();

        assert!(rx.recv().is_err());
    }

    #[test]
    fn clear_wakes_all_subscribers() {
        let dispatcher = AdsNotificationDispatcher::new();
        let h1 = NotificationHandle::from(1u32);
        let h2 = NotificationHandle::from(2u32);

        let rx1 = dispatcher.pre_register(1).unwrap();
        let rx2 = dispatcher.pre_register(2).unwrap();
        dispatcher.promote(1, h1).unwrap();
        dispatcher.promote(2, h2).unwrap();

        dispatcher.clear().unwrap();

        assert!(rx1.recv().is_err());
        assert!(rx2.recv().is_err());
    }

    #[test]
    fn promote_returns_false_for_unknown_invoke_id() {
        let dispatcher = AdsNotificationDispatcher::new();
        let handle = NotificationHandle::from(1u32);

        let promoted = dispatcher.promote(999, handle).unwrap();
        assert!(!promoted);
    }
}
