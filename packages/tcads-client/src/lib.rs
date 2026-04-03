pub mod devices;
pub mod error;
pub mod notif_guard;
pub mod tasks;

pub use tcads_core::{
    ads::{
        AdsDataTypeFlags, AdsDataTypeId, AdsDataTypeInfo, AdsReturnCode, AdsState,
        AdsSymbolUploadInfo, AdsTransMode, DeviceState, IndexGroup, IndexOffset, InvokeId,
        LogEntry, LogMessageType, NotificationHandle, WindowsFileTime,
    },
    ams::{AmsAddr, AmsNetId, AmsPort, RouterState},
    protocol::{AdsNotificationSampleOwned, ProtocolError},
};

pub use error::{Error, Result};
