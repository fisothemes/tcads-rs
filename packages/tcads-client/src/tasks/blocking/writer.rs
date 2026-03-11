use std::io::Write;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use tcads_core::AmsCommand;
use tcads_core::io::AmsFrame;
use tcads_core::io::blocking::AmsWriter;

/// Spawns a dedicated writer thread for serializing [AMS frames](AmsFrame) onto a byte stream.
///
/// Returns a [`Sender`] for enqueuing frames and a [`JoinHandle`] for awaiting
/// thread completion. TThe thread exits cleanly when all [`Sender`] clones are dropped,
/// or immediately after writing a [`PortClose`](AmsCommand::PortClose) frame.
///
/// Frames are written in FIFO order, eliminating lock contention between
/// concurrent callers.
///
/// # Note
///
/// Dropping the last [`Sender`] also drops the [`Receiver`] and invalidates all
/// remaining [`Sender`] clones, causing future [`send`](Sender::send) calls to return [`Err`].
pub struct AmsRequestWriter;

impl AmsRequestWriter {
    /// Spawns a writer thread and returns a [`Sender`] for enqueuing frames and
    /// a [`JoinHandle`] for awaiting thread completion.
    pub fn spawn<W: Write + Send + 'static>(
        writer: AmsWriter<W>,
    ) -> (Sender<AmsFrame>, JoinHandle<()>) {
        let (tx, rx) = mpsc::channel::<AmsFrame>();
        let handle = Self::spawn_thread(writer, rx);
        (tx, handle)
    }

    fn spawn_thread<W: Write + Send + 'static>(
        mut writer: AmsWriter<W>,
        rx: Receiver<AmsFrame>,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            for frame in rx {
                let is_close = frame.header().command() == AmsCommand::PortClose;
                if writer.write_frame(&frame).is_err() {
                    break;
                }
                if is_close {
                    break;
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tcads_core::ams::AmsCommand;

    #[derive(Clone, Default)]
    struct MockWriter(Arc<Mutex<Vec<u8>>>);

    impl MockWriter {
        fn bytes(&self) -> Vec<u8> {
            self.0.lock().unwrap().clone()
        }
    }

    impl Write for MockWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    struct FailingWriter;

    impl Write for FailingWriter {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "pipe broken"))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn spawn_mock() -> (Sender<AmsFrame>, JoinHandle<()>, MockWriter) {
        let mock = MockWriter::default();
        let (tx, handle) = AmsRequestWriter::spawn(AmsWriter::new(mock.clone()));
        (tx, handle, mock)
    }

    #[test]
    fn frames_are_written_in_order() {
        let (tx, handle, mock) = spawn_mock();

        let f1 = AmsFrame::new(AmsCommand::AdsCommand, vec![0x01]);
        let f2 = AmsFrame::new(AmsCommand::AdsCommand, vec![0x02]);

        tx.send(f1.clone()).unwrap();
        tx.send(f2.clone()).unwrap();

        drop(tx);
        thread::sleep(Duration::from_millis(100));
        assert!(handle.is_finished());

        assert_eq!(mock.bytes(), [f1.to_vec(), f2.to_vec()].concat());
    }

    #[test]
    fn thread_exits_on_write_error() {
        let (tx, handle) = AmsRequestWriter::spawn(AmsWriter::new(FailingWriter));
        tx.send(AmsFrame::empty(AmsCommand::AdsCommand)).unwrap();

        thread::sleep(Duration::from_millis(100));
        assert!(handle.is_finished())
    }

    #[test]
    fn multiple_senders_all_deliver() {
        let (tx, handle, mock) = spawn_mock();
        let tx2 = tx.clone();

        tx.send(AmsFrame::new(AmsCommand::AdsCommand, [0xAA]))
            .unwrap();
        tx2.send(AmsFrame::new(AmsCommand::AdsCommand, [0xBB]))
            .unwrap();

        drop(tx);
        drop(tx2);
        thread::sleep(Duration::from_millis(100));
        assert!(handle.is_finished());

        // Both frames serialized, exact order is not guaranteed across threads
        assert_eq!(mock.bytes().len(), 14); // 2 * (6 byte header + 1 byte payload)
    }

    #[test]
    fn port_close_exits_thread_and_invalidates_sender() {
        let (tx, handle, mock) = spawn_mock();

        let before = AmsFrame::new(AmsCommand::AdsCommand, vec![0xAA]);
        let close = AmsFrame::empty(AmsCommand::PortClose);
        let after = AmsFrame::new(AmsCommand::AdsCommand, vec![0xBB]);

        tx.send(before.clone()).unwrap();
        tx.send(close.clone()).unwrap();

        thread::sleep(Duration::from_millis(100));

        assert!(handle.is_finished());
        assert!(
            tx.send(after).is_err(),
            "Frame after PortClose was never enqueued, tx is now invalid"
        );
        assert_eq!(
            mock.bytes(),
            [before.to_vec(), close.to_vec()].concat(),
            "Only the two frames before and including PortClose were written"
        );
    }

    #[test]
    fn frames_after_port_close_are_never_written() {
        let (tx, handle, mock) = spawn_mock();

        tx.send(AmsFrame::empty(AmsCommand::PortClose)).unwrap();

        thread::sleep(Duration::from_millis(100));

        assert!(handle.is_finished());
        assert!(
            tx.send(AmsFrame::empty(AmsCommand::AdsCommand)).is_err(),
            "Only PortClose written, sender now invalid"
        );
        assert_eq!(
            mock.bytes(),
            AmsFrame::empty(AmsCommand::PortClose).to_vec()
        );
    }
}
