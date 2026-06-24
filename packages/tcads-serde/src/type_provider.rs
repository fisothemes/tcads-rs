use tcads_core::ads::AdsTypeInfo;

pub trait TypeProvider {
    fn get_type_info(&self, type_name: &str) -> Option<&AdsTypeInfo>;
}
