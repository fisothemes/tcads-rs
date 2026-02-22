use super::sample::{AdsNotificationSample, AdsNotificationSampleOwned};
use crate::ads::WindowsFileTime;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsStampHeader<'a> {
    timestamp: WindowsFileTime,
    samples: Vec<AdsNotificationSample<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsStampHeaderOwned {
    timestamp: WindowsFileTime,
    samples: Vec<AdsNotificationSampleOwned>,
}
