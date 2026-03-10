use super::{
    AdsNotificationDispatcher, AmsRequestDispatchKey, AmsRequestDispatcher,
    RouterNotificationDispatcher,
};
use std::io::Read;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use tcads_core::io::blocking::AmsReader;
use tcads_core::protocol::{AdsDeviceNotification, RouterNotification};
use tcads_core::{AdsCommand, AdsHeader, AmsCommand, RouterState};

/// Spawns a dedicated reader thread for deserializing incoming [`AmsFrame`](tcads_core::AmsFrame)s
/// and routing them to their waiting callers.
///
/// # Frame Routing
///
/// | [`AmsCommand`]                      | Routed to                          |
/// |-------------------------------------|------------------------------------|
/// | [`PortConnect`]                     | [`AmsRequestDispatcher`]           |
/// | [`GetLocalNetId`]                   | [`AmsRequestDispatcher`]           |
/// | [`AdsCommand`] (non-notification)   | [`AmsRequestDispatcher`]           |
/// | [`AdsCommand`] (notification)       | [`AdsNotificationDispatcher`]      |
/// | [`RouterNotification`]              | [`RouterNotificationDispatcher`]   |
/// | [`PortClose`]                       | exits loop                         |
/// | Malformed / unknown                 | skipped silently                   |
pub struct AmsResponseReader;

impl AmsResponseReader {
    /// Spawns the reader thread.
    ///
    /// The thread runs until the underlying stream reaches EOF or a
    /// [`PortClose`](AmsCommand::PortClose) frame is received, or the connection
    /// is lost via [`RouterState::Removed`].
    ///
    /// On exit, all dispatchers are cleared unconditionally. Pending callers receive
    /// [`Error::Disconnected`](crate::Error::Disconnected) and notification subscribers
    /// receive [`Err`] on their next [`recv`](std::sync::mpsc::Receiver::recv) call.
    ///
    /// The returned [`JoinHandle`] carries a [`crate::Result`] so the caller can
    /// surface any error that caused the reader to exit unexpectedly.
    pub fn spawn<R: Read + Send + 'static>(
        reader: AmsReader<R>,
        ams_requests: Arc<AmsRequestDispatcher>,
        ads_notifs: Arc<AdsNotificationDispatcher>,
        router_notifs: Arc<RouterNotificationDispatcher>,
    ) -> JoinHandle<crate::Result<()>> {
        thread::spawn(move || {
            let result = handle(reader, &ams_requests, &ads_notifs, &router_notifs);
            ams_requests.clear()?;
            ads_notifs.clear()?;
            router_notifs.clear()?;
            result
        })
    }
}

fn handle<R: Read>(
    reader: AmsReader<R>,
    ams_requests: &AmsRequestDispatcher,
    ads_notifs: &AdsNotificationDispatcher,
    router_notifs: &RouterNotificationDispatcher,
) -> crate::Result<()> {
    for result in reader.incoming() {
        let frame = match result {
            Ok(frame) => frame,
            Err(_) => continue,
        };

        match frame.header().command() {
            AmsCommand::PortConnect => {
                ams_requests.complete(AmsRequestDispatchKey::PortConnect, frame)?
            }
            AmsCommand::GetLocalNetId => {
                ams_requests.complete(AmsRequestDispatchKey::GetLocalNetId, frame)?
            }
            AmsCommand::AdsCommand => {
                let Ok((header, _)) = AdsHeader::parse_prefix(frame.payload()) else {
                    continue;
                };

                match header.command_id() {
                    AdsCommand::AdsDeviceNotification => {
                        let Ok(notif) = AdsDeviceNotification::try_from(&frame) else {
                            continue;
                        };

                        for (_, sample) in notif.iter_samples() {
                            ads_notifs.dispatch(sample.handle(), sample.to_owned())?;
                        }
                    }
                    _ => ams_requests
                        .complete(AmsRequestDispatchKey::AdsCommand(header.invoke_id()), frame)?,
                }
            }
            AmsCommand::RouterNotification => {
                let Ok(notif) = RouterNotification::try_from(frame) else {
                    continue;
                };

                match notif.state() {
                    RouterState::Stop => {
                        ads_notifs.clear()?;
                        router_notifs.broadcast(RouterState::Stop)?;
                    }
                    RouterState::Removed => {
                        ads_notifs.clear()?;
                        ams_requests.clear()?;
                        router_notifs.broadcast(RouterState::Removed)?;
                        break;
                    }
                    state => router_notifs.broadcast(state)?,
                }
            }
            AmsCommand::PortClose => break,
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::sync::mpsc::{self, Receiver};
    use tcads_core::AmsFrame;
    use tcads_core::ads::NotificationHandle;

    fn make_dispatchers() -> (
        Arc<AmsRequestDispatcher>,
        Arc<AdsNotificationDispatcher>,
        Arc<RouterNotificationDispatcher>,
        Receiver<AmsFrame>,
    ) {
        let (write_tx, write_rx) = mpsc::channel();
        (
            Arc::new(AmsRequestDispatcher::new(write_tx)),
            Arc::new(AdsNotificationDispatcher::new()),
            Arc::new(RouterNotificationDispatcher::new()),
            write_rx,
        )
    }

    fn run_handle(
        frames: Vec<AmsFrame>,
        requests: &AmsRequestDispatcher,
        ads_notifs: &AdsNotificationDispatcher,
        router_notifs: &RouterNotificationDispatcher,
    ) -> crate::Result<()> {
        let data: Vec<u8> = frames.into_iter().flat_map(|f| f.to_vec()).collect();
        let reader = AmsReader::new(Cursor::new(data));

        handle(reader, requests, ads_notifs, router_notifs)
    }

    #[test]
    fn port_connect_is_routed_to_requests() {
        let (requests, ads_notifs, router_notifs, _write_rx) = make_dispatchers();
        let frame = AmsFrame::empty(AmsCommand::PortConnect);
        let rx = requests
            .dispatch(AmsRequestDispatchKey::PortConnect, frame.clone())
            .unwrap();

        // Simulate reader receiving the response
        run_handle(vec![frame], &requests, &ads_notifs, &router_notifs).unwrap();

        assert!(rx.try_recv().is_ok());
    }

    #[test]
    fn get_local_net_id_is_routed_to_requests() {
        let (requests, ads_notifs, router_notifs, _write_rx) = make_dispatchers();
        let frame = AmsFrame::empty(AmsCommand::GetLocalNetId);
        let rx = requests
            .dispatch(AmsRequestDispatchKey::GetLocalNetId, frame.clone())
            .unwrap();

        run_handle(vec![frame], &requests, &ads_notifs, &router_notifs).unwrap();

        assert!(rx.try_recv().is_ok());
    }

    #[test]
    fn port_close_exits_loop_cleanly() {
        let (requests, ads_notifs, router_notifs, _write_rx) = make_dispatchers();
        let frame = AmsFrame::empty(AmsCommand::PortClose);

        run_handle(vec![frame], &requests, &ads_notifs, &router_notifs).unwrap();
    }

    #[test]
    fn unknown_commands_are_ignored_silently() {
        let (requests, ads_notifs, router_notifs, _write_rx) = make_dispatchers();
        // GetLocalNetId with no pending waiter should not panic or hang
        let frame = AmsFrame::empty(AmsCommand::GetLocalNetId);

        run_handle(vec![frame], &requests, &ads_notifs, &router_notifs).unwrap();
    }

    #[test]
    fn router_stop_clears_ads_notifs_before_broadcasting() {
        let (requests, ads_notifs, router_notifs, _write_rx) = make_dispatchers();

        // Register a notification subscription
        let handle = NotificationHandle::from(1u32);
        let notif_rx = ads_notifs.pre_register(1).unwrap();
        ads_notifs.promote(1, handle).unwrap();

        // Register a router subscriber
        let router_rx = router_notifs.subscribe().unwrap();

        let stop_frame = RouterNotification::new(RouterState::Stop).to_frame();
        run_handle(vec![stop_frame], &requests, &ads_notifs, &router_notifs).unwrap();

        assert!(
            notif_rx.recv().is_err(),
            "Notification subscription should be dead"
        );
        assert_eq!(
            router_rx.recv().unwrap(),
            RouterState::Stop,
            "Router subscriber should have received Stop"
        );
    }

    #[test]
    fn router_removed_clears_requests_and_exits() {
        let (requests, ads_notifs, router_notifs, _write_rx) = make_dispatchers();

        let pending_frame = AmsFrame::empty(AmsCommand::AdsCommand);
        let pending_rx = requests
            .dispatch(AmsRequestDispatchKey::AdsCommand(1), pending_frame)
            .unwrap();

        let router_rx = router_notifs.subscribe().unwrap();

        let removed_frame = RouterNotification::new(RouterState::Removed).to_frame();
        run_handle(vec![removed_frame], &requests, &ads_notifs, &router_notifs).unwrap();

        assert!(
            pending_rx.recv().is_err(),
            "Pending request should be woken with Disconnected"
        );
        assert_eq!(
            router_rx.recv().unwrap(),
            RouterState::Removed,
            "Router subscriber should have received Removed"
        );
    }

    #[test]
    fn router_start_is_broadcast_without_clearing() {
        let (requests, ads_notifs, router_notifs, _write_rx) = make_dispatchers();

        let handle = NotificationHandle::from(1u32);
        let notif_rx = ads_notifs.pre_register(1).unwrap();
        ads_notifs.promote(1, handle).unwrap();

        let router_rx = router_notifs.subscribe().unwrap();

        let start_frame = RouterNotification::new(RouterState::Start).to_frame();
        run_handle(vec![start_frame], &requests, &ads_notifs, &router_notifs).unwrap();

        assert_eq!(
            router_rx.recv().unwrap(),
            RouterState::Start,
            "Router subscriber should have received Start"
        );
        assert!(
            notif_rx.try_recv().is_err(),
            "Notification subscription should still be alive"
        );
    }

    #[test]
    fn multiple_frames_processed_in_order() {
        let (requests, ads_notifs, router_notifs, _write_rx) = make_dispatchers();
        let router_rx = router_notifs.subscribe().unwrap();

        let frames = vec![
            RouterNotification::new(RouterState::Stop).to_frame(),
            RouterNotification::new(RouterState::Start).to_frame(),
        ];

        run_handle(frames, &requests, &ads_notifs, &router_notifs).unwrap();

        assert_eq!(router_rx.recv().unwrap(), RouterState::Stop);
        assert_eq!(router_rx.recv().unwrap(), RouterState::Start);
    }
}
