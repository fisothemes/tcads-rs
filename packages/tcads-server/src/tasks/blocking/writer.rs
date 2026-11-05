use std::io::Write;
use std::sync::mpsc::{self, Sender};
use std::thread::{self, JoinHandle};
use tcads_core::{AmsCommand, AmsFrame};
use tcads_io::blocking::AmsWriter;

pub struct AmsResponseWriter;

impl AmsResponseWriter {
    pub fn spawn<W: Write + Send + 'static>(
        mut writer: AmsWriter<W>,
    ) -> (Sender<AmsFrame>, JoinHandle<()>) {
        let (tx, rx) = mpsc::channel::<AmsFrame>();

        let handle = thread::spawn(move || {
            while let Ok(frame) = rx.recv() {
                let is_close = frame.header().command() == AmsCommand::PortClose;
                if writer.write_frame(&frame).is_err() || is_close {
                    break;
                }
            }
        });

        (tx, handle)
    }
}
