use tcads_core::AmsCommand;
use tcads_core::io::AmsFrame;
use tcads_core::io::tokio::AmsWriter;
use tokio::io::AsyncWrite;
use tokio::sync::mpsc::{self, UnboundedReceiver as Receiver, UnboundedSender as Sender};
use tokio::task::JoinHandle;

/// Spawns a dedicated writer task for serializing [`AmsFrame`]s onto an async byte stream.
///
/// Returns a [`Sender`] for enqueuing frames and a [`JoinHandle`] for awaiting
/// task completion. The task exits cleanly when all [`Sender`] clones are dropped,
/// or immediately after writing a [`PortClose`](AmsCommand::PortClose) frame.
///
/// Dropping the last [`Sender`] also exits the task cleanly, which closes the
/// underlying write half of the TCP stream. The read half will subsequently return
/// EOF, causing the reader task to exit and clear all dispatchers.
pub struct AmsRequestWriter;

impl AmsRequestWriter {
    /// Spawns a writer task and returns a [`Sender`] for enqueuing frames and
    /// a [`JoinHandle`] for awaiting task completion.
    pub fn spawn<W: AsyncWrite + Unpin + Send + 'static>(
        writer: AmsWriter<W>,
    ) -> (Sender<AmsFrame>, JoinHandle<()>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let handle = tokio::spawn(Self::proc(writer, rx));
        (tx, handle)
    }

    async fn proc<W: AsyncWrite + Unpin>(mut writer: AmsWriter<W>, mut rx: Receiver<AmsFrame>) {
        while let Some(frame) = rx.recv().await {
            let is_close = frame.header().command() == AmsCommand::PortClose;
            if writer.write_frame(&frame).await.is_err() {
                break;
            }
            if is_close {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{self, AsyncReadExt};
    use tokio::time::Duration;

    fn spawn_duplex() -> (Sender<AmsFrame>, JoinHandle<()>, io::DuplexStream) {
        let (client, server) = io::duplex(65536);
        let (tx, handle) = AmsRequestWriter::spawn(AmsWriter::new(client));
        (tx, handle, server)
    }

    #[tokio::test]
    async fn frames_are_written_in_order() {
        let (tx, handle, mut server) = spawn_duplex();

        let f1 = AmsFrame::new(AmsCommand::AdsCommand, vec![0x01]);
        let f2 = AmsFrame::new(AmsCommand::AdsCommand, vec![0x02]);

        tx.send(f1.clone()).unwrap();
        tx.send(f2.clone()).unwrap();

        drop(tx);
        handle.await.unwrap();

        let expected = [f1.to_vec(), f2.to_vec()].concat();
        let mut buf = vec![0u8; expected.len()];
        server.read_exact(&mut buf).await.unwrap();
        assert_eq!(buf, expected);
    }

    #[tokio::test]
    async fn task_exits_on_write_error() {
        // Close the server side immediately so the client write fails.
        let (client, server) = io::duplex(64);
        drop(server);

        let (tx, handle) = AmsRequestWriter::spawn(AmsWriter::new(client));
        let _ = tx.send(AmsFrame::empty(AmsCommand::AdsCommand));

        handle.await.unwrap();
    }

    #[tokio::test]
    async fn multiple_senders_all_deliver() {
        let (tx, handle, mut server) = spawn_duplex();
        let tx2 = tx.clone();

        tx.send(AmsFrame::new(AmsCommand::AdsCommand, [0xAA]))
            .unwrap();
        tx2.send(AmsFrame::new(AmsCommand::AdsCommand, [0xBB]))
            .unwrap();

        drop(tx);
        drop(tx2);
        handle.await.unwrap();

        // Both frames written, 2 * (6 byte header + 1 byte payload)
        let mut buf = vec![0u8; 14];
        server.read_exact(&mut buf).await.unwrap();
        assert_eq!(buf.len(), 14);
    }

    #[tokio::test]
    async fn port_close_exits_task_and_invalidates_sender() {
        let (tx, handle, mut server) = spawn_duplex();

        let before = AmsFrame::new(AmsCommand::AdsCommand, vec![0xAA]);
        let close = AmsFrame::empty(AmsCommand::PortClose);

        tx.send(before.clone()).unwrap();
        tx.send(close.clone()).unwrap();

        handle.await.unwrap();

        // Sender is now invalid, the channel should be closed when the task exited
        assert!(tx.send(AmsFrame::empty(AmsCommand::AdsCommand)).is_err());

        // Only the frames up to and including PortClose were written
        let expected = [before.to_vec(), close.to_vec()].concat();
        let mut buf = vec![0u8; expected.len()];
        server.read_exact(&mut buf).await.unwrap();
        assert_eq!(buf, expected);
    }

    #[tokio::test]
    async fn frames_after_port_close_are_never_written() {
        let (tx, handle, mut server) = spawn_duplex();

        let close = AmsFrame::empty(AmsCommand::PortClose);
        tx.send(close.clone()).unwrap();

        handle.await.unwrap();

        // Sender is now invalid
        assert!(tx.send(AmsFrame::empty(AmsCommand::AdsCommand)).is_err());

        // Only PortClose was written
        let expected = close.to_vec();
        let mut buf = vec![0u8; expected.len()];
        server.read_exact(&mut buf).await.unwrap();
        assert_eq!(buf, expected);
    }

    #[tokio::test]
    async fn task_exits_cleanly_when_all_senders_dropped() {
        let (tx, handle, _server) = spawn_duplex();
        let tx2 = tx.clone();

        drop(tx);
        drop(tx2);

        // Should complete without hanging
        tokio::time::timeout(Duration::from_secs(1), handle)
            .await
            .expect("task did not exit after all senders dropped")
            .unwrap();
    }
}
