use std::time::Duration;
use tcads_client::blocking::AdsDevice;
use tcads_client::{AdsState, AdsTransMode, AmsAddr};

const GET_SYMHANDLE_BYNAME: u32 = 0xF003;
const READ_WRITE_SYMVAL_BYHANDLE: u32 = 0xF005;
const RELEASE_SYMHANDLE: u32 = 0xF006;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let device = AdsDevice::connect(None)?;

    println!(
        "AMS Router has assigned us the address {}!",
        device.source()?
    );

    let local_net_id = device.get_local_net_id()?;

    println!("Local Net ID is {}", local_net_id);

    let target = AmsAddr::new(local_net_id, 851).into();

    println!("Target address is {}", target);
    println!("Device info: {:?}", device.read_device_info(target)?);

    device.write_control(target, AdsState::Stop, 0, [])?;
    println!("PLC state: {:?}", device.read_state(target)?);

    device.write_control(target, AdsState::Run, 0, [])?;
    println!("PLC state: {:?}", device.read_state(target)?);

    let var_handle = u32::from_le_bytes(
        (*device.read_write(
            target,
            GET_SYMHANDLE_BYNAME,
            0,
            size_of::<u32>() as u32,
            b"MAIN.nCount\0",
        )?)
        .try_into()?,
    );

    println!("Variable handle is for MAIN.nCount is {}", var_handle);

    device.write(
        target,
        READ_WRITE_SYMVAL_BYHANDLE,
        var_handle,
        42u32.to_le_bytes(),
    )?;

    let value = u32::from_le_bytes(
        (*device.read(
            target,
            READ_WRITE_SYMVAL_BYHANDLE,
            var_handle,
            size_of::<u32>() as u32,
        )?)
        .try_into()?,
    );

    println!("Value of MAIN.nCount is {}", value);

    let (sample_rx, notif_handle) = device.add_notification(
        target,
        READ_WRITE_SYMVAL_BYHANDLE,
        var_handle,
        size_of::<u32>() as u32,
        AdsTransMode::ServerOnChange,
        0,
        10,
    )?;

    println!("Device notification added: {:?}", notif_handle);

    let sample = sample_rx.recv_timeout(Duration::from_secs(10))?;

    if sample.handle() == notif_handle {
        println!("Received notification for MAIN.nCount: {:?}", sample.data());
    } else {
        panic!(
            "Received notification for unknown variable: {:?}",
            sample.handle()
        );
    }

    device.delete_notification(target, notif_handle)?;
    device.write(target, RELEASE_SYMHANDLE, 0, var_handle.to_le_bytes())?;
    device.shutdown()?;

    Ok(())
}
