use super::{
    AdsNotificationDispatcher, AmsRequestDispatchKey, AmsRequestDispatcher,
    RouterNotificationDispatcher,
};
use std::sync::Arc;
use tcads_core::io::tokio::AmsReader;
use tcads_core::protocol::{AdsDeviceNotification, RouterNotification};
use tcads_core::{AdsCommand, AdsHeader, AmsCommand, RouterState};
use tokio::io::{self, AsyncRead};
use tokio::task::JoinHandle;

/// Spawns a dedicated reader task that routes incoming [`AmsFrame`](tcads_core::AmsFrame)s to the
/// appropriate dispatcher.
///
/// See the [blocking equivalent](crate::tasks::blocking::AmsResponseReader) for full routing
/// logic documentation.
pub struct AmsResponseReader;

impl AmsResponseReader {
    /// Spawns the reader task.
    ///
    /// The task runs until the underlying stream reaches EOF, a
    /// [`PortClose`](AmsCommand::PortClose) frame is received, or the connection
    /// is lost via [`RouterState::Removed`].
    ///
    /// On exit, all dispatchers are cleared unconditionally — pending callers receive
    /// [`Error::Disconnected`](crate::Error::Disconnected) and notification subscribers
    /// receive [`None`] on their next [`recv`](tokio::sync::mpsc::UnboundedReceiver::recv) call.
    ///
    /// The returned [`JoinHandle`] carries a [`crate::Result`] so the caller can
    /// surface any error that caused the reader to exit unexpectedly.
    pub fn spawn<R: AsyncRead + Unpin + Send + 'static>(
        reader: AmsReader<R>,
        ams_requests: Arc<AmsRequestDispatcher>,
        ads_notifs: Arc<AdsNotificationDispatcher>,
        router_notifs: Arc<RouterNotificationDispatcher>,
    ) -> JoinHandle<crate::Result<()>> {
        tokio::spawn(Self::proc(reader, ams_requests, ads_notifs, router_notifs))
    }

    async fn proc<R: AsyncRead + Unpin>(
        reader: AmsReader<R>,
        ams_requests: Arc<AmsRequestDispatcher>,
        ads_notifs: Arc<AdsNotificationDispatcher>,
        router_notifs: Arc<RouterNotificationDispatcher>,
    ) -> crate::Result<()> {
        let result = handle(reader, &ams_requests, &ads_notifs, &router_notifs).await;
        ams_requests.clear().await;
        ads_notifs.clear().await;
        router_notifs.clear().await;
        result
    }
}

async fn handle<R: AsyncRead + Unpin>(
    mut reader: AmsReader<R>,
    ams_requests: &AmsRequestDispatcher,
    ads_notifs: &AdsNotificationDispatcher,
    router_notifs: &RouterNotificationDispatcher,
) -> crate::Result<()> {
    loop {
        let frame = match reader.read_frame().await {
            Ok(frame) => frame,
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
            Err(_) => continue,
        };

        match frame.header().command() {
            AmsCommand::PortConnect => {
                ams_requests
                    .complete(AmsRequestDispatchKey::PortConnect, frame)
                    .await?
            }
            AmsCommand::GetLocalNetId => {
                ams_requests
                    .complete(AmsRequestDispatchKey::GetLocalNetId, frame)
                    .await?
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
                            ads_notifs
                                .dispatch(sample.handle(), sample.to_owned())
                                .await;
                        }
                    }
                    _ => {
                        ams_requests
                            .complete(AmsRequestDispatchKey::AdsCommand(header.invoke_id()), frame)
                            .await?
                    }
                }
            }
            AmsCommand::RouterNotification => {
                let Ok(notif) = RouterNotification::try_from(frame) else {
                    continue;
                };

                match notif.state() {
                    RouterState::Stop => {
                        ads_notifs.clear().await;
                        router_notifs.broadcast(RouterState::Stop).await;
                    }
                    RouterState::Removed => {
                        ads_notifs.clear().await;
                        ams_requests.clear().await;
                        router_notifs.broadcast(RouterState::Removed).await;
                        break;
                    }
                    state => router_notifs.broadcast(state).await,
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
    use tcads_core::AmsFrame;
    use tcads_core::ads::NotificationHandle;
    use tokio::io::{AsyncWriteExt, duplex};
    use tokio::sync::mpsc;

    fn make_dispatchers() -> (
        Arc<AmsRequestDispatcher>,
        Arc<AdsNotificationDispatcher>,
        Arc<RouterNotificationDispatcher>,
        mpsc::UnboundedReceiver<AmsFrame>,
    ) {
        let (write_tx, write_rx) = mpsc::unbounded_channel();
        (
            Arc::new(AmsRequestDispatcher::new(write_tx)),
            Arc::new(AdsNotificationDispatcher::new()),
            Arc::new(RouterNotificationDispatcher::new()),
            write_rx,
        )
    }

    async fn run_handle(
        frames: Vec<AmsFrame>,
        requests: &AmsRequestDispatcher,
        ads_notifs: &AdsNotificationDispatcher,
        router_notifs: &RouterNotificationDispatcher,
    ) -> crate::Result<()> {
        let data: Vec<u8> = frames.into_iter().flat_map(|f| f.to_vec()).collect();
        let (client, mut server) = duplex(65536);
        server.write_all(&data).await?;
        drop(server); // EOF after data
        let reader = AmsReader::new(client);
        handle(reader, requests, ads_notifs, router_notifs).await
    }

    #[tokio::test]
    async fn port_connect_is_routed_to_requests() {
        let (requests, ads_notifs, router_notifs, _write_rx) = make_dispatchers();
        let frame = AmsFrame::empty(AmsCommand::PortConnect);
        let mut rx = requests
            .dispatch(AmsRequestDispatchKey::PortConnect, frame.clone())
            .await
            .unwrap();

        run_handle(vec![frame], &requests, &ads_notifs, &router_notifs)
            .await
            .unwrap();

        assert!(rx.try_recv().is_ok());
    }

    #[tokio::test]
    async fn get_local_net_id_is_routed_to_requests() {
        let (requests, ads_notifs, router_notifs, _write_rx) = make_dispatchers();
        let frame = AmsFrame::empty(AmsCommand::GetLocalNetId);
        let mut rx = requests
            .dispatch(AmsRequestDispatchKey::GetLocalNetId, frame.clone())
            .await
            .unwrap();

        run_handle(vec![frame], &requests, &ads_notifs, &router_notifs)
            .await
            .unwrap();

        assert!(rx.try_recv().is_ok());
    }

    #[tokio::test]
    async fn port_close_exits_loop_cleanly() {
        let (requests, ads_notifs, router_notifs, _write_rx) = make_dispatchers();
        let frame = AmsFrame::empty(AmsCommand::PortClose);

        run_handle(vec![frame], &requests, &ads_notifs, &router_notifs)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn unknown_commands_are_ignored_silently() {
        let (requests, ads_notifs, router_notifs, _write_rx) = make_dispatchers();
        // GetLocalNetId with no pending waiter should not panic or hang
        let frame = AmsFrame::empty(AmsCommand::GetLocalNetId);

        run_handle(vec![frame], &requests, &ads_notifs, &router_notifs)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn router_stop_clears_ads_notifs_before_broadcasting() {
        let (requests, ads_notifs, router_notifs, _write_rx) = make_dispatchers();

        let handle = NotificationHandle::from(1u32);
        let mut notif_rx = ads_notifs.pre_register(1).await;
        ads_notifs.promote(1, handle).await;

        let mut router_rx = router_notifs.subscribe().await;

        let stop_frame = RouterNotification::new(RouterState::Stop).to_frame();
        run_handle(vec![stop_frame], &requests, &ads_notifs, &router_notifs)
            .await
            .unwrap();

        assert!(
            notif_rx.recv().await.is_none(),
            "Notification subscription should be dead"
        );
        assert_eq!(
            router_rx.recv().await.unwrap(),
            RouterState::Stop,
            "Router subscriber should have received Stop"
        );
    }

    #[tokio::test]
    async fn router_removed_clears_requests_and_exits() {
        let (requests, ads_notifs, router_notifs, _write_rx) = make_dispatchers();

        let pending_frame = AmsFrame::empty(AmsCommand::AdsCommand);
        let mut pending_rx = requests
            .dispatch(AmsRequestDispatchKey::AdsCommand(1), pending_frame)
            .await
            .unwrap();

        let mut router_rx = router_notifs.subscribe().await;

        let removed_frame = RouterNotification::new(RouterState::Removed).to_frame();
        run_handle(vec![removed_frame], &requests, &ads_notifs, &router_notifs)
            .await
            .unwrap();

        assert!(
            pending_rx.recv().await.is_none(),
            "Pending request should be woken with Disconnected"
        );
        assert_eq!(
            router_rx.recv().await.unwrap(),
            RouterState::Removed,
            "Router subscriber should have received Removed"
        );
    }

    #[tokio::test]
    async fn router_start_is_broadcast_without_clearing() {
        let (requests, ads_notifs, router_notifs, _write_rx) = make_dispatchers();

        let handle = NotificationHandle::from(1u32);
        let mut notif_rx = ads_notifs.pre_register(1).await;
        ads_notifs.promote(1, handle).await;

        let mut router_rx = router_notifs.subscribe().await;

        let start_frame = RouterNotification::new(RouterState::Start).to_frame();
        run_handle(vec![start_frame], &requests, &ads_notifs, &router_notifs)
            .await
            .unwrap();

        assert_eq!(
            router_rx.recv().await.unwrap(),
            RouterState::Start,
            "Router subscriber should have received Start"
        );
        assert!(
            notif_rx.try_recv().is_err(),
            "Notification subscription should still be alive"
        );
    }

    #[tokio::test]
    async fn multiple_frames_processed_in_order() {
        let (requests, ads_notifs, router_notifs, _write_rx) = make_dispatchers();
        let mut router_rx = router_notifs.subscribe().await;

        let frames = vec![
            RouterNotification::new(RouterState::Stop).to_frame(),
            RouterNotification::new(RouterState::Start).to_frame(),
        ];

        run_handle(frames, &requests, &ads_notifs, &router_notifs)
            .await
            .unwrap();

        assert_eq!(router_rx.recv().await.unwrap(), RouterState::Stop);
        assert_eq!(router_rx.recv().await.unwrap(), RouterState::Start);
    }
}
