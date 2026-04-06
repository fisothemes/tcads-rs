use super::{AdsDataTypeId, Guid};

/// The return type of a TwinCAT RPC method.
///
/// Aggregated from fields in the [`AdsMethodInfo`](super::AdsMethodInfo) wire header rather
/// than having its own wire layout. It's not present when the method has no return value; callers
/// receive [`None`] from [`AdsMethodInfo::return_type`](super::AdsMethodInfo::return_type)
/// in that case.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct AdsMethodReturnTypeInfo {
    size: u32,
    align_size: u32,
    type_id: AdsDataTypeId,
    guid: Guid,
    name: String,
}

impl AdsMethodReturnTypeInfo {
    pub fn new(
        size: u32,
        align_size: u32,
        type_id: AdsDataTypeId,
        guid: Guid,
        name: impl Into<String>,
    ) -> Self {
        Self {
            size,
            align_size,
            type_id,
            guid,
            name: name.into(),
        }
    }

    pub fn size(&self) -> u32 {
        self.size
    }

    pub fn align_size(&self) -> u32 {
        self.align_size
    }

    pub fn type_id(&self) -> AdsDataTypeId {
        self.type_id
    }

    pub fn guid(&self) -> &Guid {
        &self.guid
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_ads_method_return_type_info_new() {
        let guid: Guid = "12345678-9abc-def0-1234-56789abcdef0".parse().unwrap();
        let info =
            AdsMethodReturnTypeInfo::new(10, 20, AdsDataTypeId::Int32, guid.clone(), "MyType");
        assert_eq!(info.size(), 10);
        assert_eq!(info.align_size(), 20);
        assert_eq!(info.type_id(), AdsDataTypeId::Int32);
        assert_eq!(*info.guid(), guid);
        assert_eq!(info.name(), "MyType");
    }
}
