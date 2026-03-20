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
    }

    impl NotificationGuard {
        /// Creates a new instance of the [`NotificationGuard`]
        pub fn new(handle: NotificationHandle, target: AmsAddr, device: AdsDevice) -> Self {
            Self {
                handle,
                target,
                device,
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
    }

    impl Drop for NotificationGuard {
        fn drop(&mut self) {
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
    }

    impl NotificationGuard {
        /// Creates a new instance of the [`NotificationGuard`]
        pub fn new(handle: NotificationHandle, target: AmsAddr, device: AdsDevice) -> Self {
            Self {
                handle,
                target,
                device,
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
    }

    impl Drop for NotificationGuard {
        fn drop(&mut self) {
            let device = self.device.clone();
            let target = self.target;
            let handle = self.handle;
            todo!()
        }
    }
}
