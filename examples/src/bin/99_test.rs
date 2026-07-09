//! Scratch pad for testing.

// use std::time::Duration;
use tcads::client::devices::blocking::AdsDevice;
use tcads::core::*;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
/// Testing symbol change notification.
fn main() -> Result<()> {
    let device = AdsDevice::connect(None)?;
    let target = device.get_local_net_id()?;

    let (rx, _) = device.add_notification(
        (target, 851).into(),
        IndexGroup::SYMBOL_VERSION,
        IndexOffset::ZERO,
        AdsNotificationAttrib::new(1, AdsTransMode::ServerOnChange, 0, 0),
    )?;

    while let Ok(sample) = rx.recv() {
        println!("{:?}", sample);
    }

    Ok(())
}
