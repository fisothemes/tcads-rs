//! RAII notification handle guards.
//!
//! A [`NotificationGuard`](blocking::NotificationGuard) wraps a
//! [`NotificationHandle`] and automatically calls [`delete_notification`](blocking::AdsDevice::delete_notification)
//! when dropped, ensuring notification handles are always cleaned up on the router.

use tcads_core::{AmsAddr, NotificationHandle};

pub mod blocking {
    use super::*;
    use crate::devices::blocking::AdsDevice;

    /// RAII guard for a blocking ADS notification handle.
    ///
    /// Calls [`delete_notification`](AdsDevice::delete_notification)
    /// synchronously when dropped.
    pub struct NotificationGuard {
        handle: NotificationHandle,
        target: AmsAddr,
        device: AdsDevice,
        cancelled: bool,
    }

    impl NotificationGuard {
        /// Creates a new instance of the [`NotificationGuard`]
        pub fn new(handle: NotificationHandle, target: AmsAddr, device: AdsDevice) -> Self {
            Self {
                handle,
                target,
                device,
                cancelled: false,
            }
        }

        /// Returns the notification handle.
        pub fn handle(&self) -> NotificationHandle {
            self.handle
        }

        /// Returns the AMS Address of the target device.
        pub fn target(&self) -> AmsAddr {
            self.target
        }

        /// Returns a reference to the underlying [`AdsDevice`].
        pub fn device(&self) -> &AdsDevice {
            &self.device
        }

        /// Explicitly deletes the notification, returning any error from the router.
        ///
        /// Unlike letting the guard drop, the result is not discarded. Marks the guard as
        /// cancelled first, so the subsequent `Drop` is a no-op rather than deleting the
        /// same (now-invalid) handle a second time.
        pub fn cancel(mut self) -> crate::Result<()> {
            self.cancelled = true;
            self.device.delete_notification(self.target, self.handle)
        }
    }

    impl Drop for NotificationGuard {
        fn drop(&mut self) {
            if self.cancelled {
                return;
            }
            let _ = self.device.delete_notification(self.target, self.handle);
        }
    }
}

pub mod tokio {
    use super::*;
    use crate::devices::tokio::AdsDevice;
    use ::tokio as tokio_rt;

    /// RAII guard for a tokio ADS notification handle.
    ///
    /// Attempts to call [`delete_notification`](AdsDevice::delete_notification)
    /// when dropped by spawning a task on the current tokio runtime. If no runtime
    /// is available, the clean-up is skipped and the router will clean up the handle
    /// when the connection closes.
    pub struct NotificationGuard {
        handle: NotificationHandle,
        target: AmsAddr,
        device: AdsDevice,
        cancelled: bool,
    }

    impl NotificationGuard {
        /// Creates a new instance of the [`NotificationGuard`]
        pub fn new(handle: NotificationHandle, target: AmsAddr, device: AdsDevice) -> Self {
            Self {
                handle,
                target,
                device,
                cancelled: false,
            }
        }

        /// Returns the notification handle.
        pub fn handle(&self) -> NotificationHandle {
            self.handle
        }

        /// Returns the AMS Address of the target device.
        pub fn target(&self) -> AmsAddr {
            self.target
        }

        /// Returns a reference to the underlying [`AdsDevice`].
        pub fn device(&self) -> &AdsDevice {
            &self.device
        }

        /// Explicitly deletes the notification, returning any error from the router.
        ///
        /// Unlike letting the guard drop, the result is not discarded, and clean-up runs
        /// immediately on the current task rather than being spawned separately. Marks the
        /// guard as cancelled first, so the subsequent `Drop` is a no-op rather than
        /// deleting the same (now-invalid) handle a second time.
        pub async fn cancel(mut self) -> crate::Result<()> {
            self.cancelled = true;
            self.device
                .delete_notification(self.target, self.handle)
                .await
        }
    }

    impl Drop for NotificationGuard {
        fn drop(&mut self) {
            if self.cancelled {
                return;
            }

            let device = self.device.clone();
            let target = self.target;
            let handle = self.handle;

            if let Ok(rt) = tokio_rt::runtime::Handle::try_current() {
                rt.spawn(async move {
                    let _ = device.delete_notification(target, handle).await;
                });
            }
            // No runtime context, this is best-effort at best, router cleans up on connection close
            // anyway. 100% safe to ignore.
        }
    }
}
