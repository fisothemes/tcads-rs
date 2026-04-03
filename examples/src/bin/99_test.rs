//! Scratch pad for testing.

use std::time::Duration;
use tcads::client::devices::blocking::DataTypeDevice;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    let device = DataTypeDevice::connect(851, Duration::from_secs(5))?;
    let dt_info = device.get_data_type_info("ARRAY [1..1] OF PLC.PlcTaskSystemInfo")?;

    println!("{:#?}", dt_info);

    let upload_info = device.get_upload_info()?;

    println!("{:?}", upload_info);

    //let all_dt_info = device.get_all_data_type_info()?;
    //println!("length: {} | {:?}", all_dt_info.len(), all_dt_info);

    // This is here for testing purposes.
    let _val: [u8; _] = [
        //--------------- fixed length data -----------------
        120, 0, 0, 0, // Length
        1, 0, 0, 0, // Version
        0, 0, 0, 0, // Hash value | Code Offset to getter method
        0, 0, 0, 0, // Base Type Hash Value | Code Offset to setter method
        4, 0, 0, 0, // Data type size in bytes
        0, 0, 0, 0, // Offset of data type sub-item (if is a child type)
        19, 0, 0, 0, // ADS type ID (19 = ADST_UINT32)
        129, 16, 0, 0, // Flags
        5, 0, // Name length excluding null terminator
        0, 0, // Sub-item Name length excluding null terminator
        0, 0, // length of comment excluding null terminator
        0, 0, // Array dimension count
        0, 0, // Sub-item count
        //--------------- variable length data -----------------
        85, 68, 73, 78, 84, 0, // Name - "UDINT\0"
        0, // Sub-item name including null terminator
        0, // Comment including null terminator
        149, 25, 7, 24, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, //
        2, 0, 15, 1, 68, 105, 115, 112, 108, 97, 121, 77, 105, 110, 86, 97, 108, 117, 101, 0, 48,
        0, 15, 10, 68, 105, 115, 112, 108, 97, 121, 77, 97, 120, 86, 97, 108, 117, 101, 0, 35, 120,
        102, 102, 102, 102, 102, 102, 102, 102, 0, 0, 0, 0,
    ];

    Ok(())
}
