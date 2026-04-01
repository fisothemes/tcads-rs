//! Scratch pad for testing.

use std::time::Duration;
use tcads::client::devices::blocking::DataTypeDevice;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    let device = DataTypeDevice::connect(851, Duration::from_secs(5))?;

    let info = device.get_data_type_info("UDINT")?;

    println!("{:?}", info);

    // This is here for testing purposes.
    let _val: [u8; _] = [
        120, 0, 0, 0, // Length
        1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // ??
        4, 0, 0, 0, 0, 0, 0, 0, // Type Length
        19, 0, 0, 0, // Type ID (19 = ADST_UINT32)
        129, 16, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 85, 68, 73, 78, 84, 0, 0, 0, 149, 25, 7, 24,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 2, 0, 15, 1, 68, 105, 115, 112, 108, 97, 121, 77, 105,
        110, 86, 97, 108, 117, 101, 0, 48, 0, 15, 10, 68, 105, 115, 112, 108, 97, 121, 77, 97, 120,
        86, 97, 108, 117, 101, 0, 35, 120, 102, 102, 102, 102, 102, 102, 102, 102, 0, 0, 0, 0,
    ];

    Ok(())
}
