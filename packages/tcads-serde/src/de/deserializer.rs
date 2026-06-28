use crate::TypeProvider;
use tcads_core::AdsTypeInfo;

pub struct AdsDeserializer<'a, 'de, P: TypeProvider> {
    input: &'de [u8],
    type_info: &'a AdsTypeInfo,
    provider: &'a P,
}

impl<'a, 'de, P: TypeProvider> AdsDeserializer<'a, 'de, P> {
    pub fn new(input: &'de [u8], type_info: &'a AdsTypeInfo, provider: &'a P) -> Self {
        Self {
            input,
            type_info,
            provider,
        }
    }
}
