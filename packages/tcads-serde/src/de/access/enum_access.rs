use serde::de::value::StrDeserializer;
use serde::de::{DeserializeSeed, EnumAccess, VariantAccess, Visitor};

/// Provides access to TwinCAT Enum variants.
///
/// Because PLC enums are strictly integer-backed constants with string names attached
/// via metadata, this accessor always maps to a Serde Unit Variant (a variant with no payload).
/// It passes the string name of the PLC variant to Serde to match against the Rust enum.
pub struct AdsEnumAccess<'de> {
    variant_name: &'de str,
}

impl<'de> AdsEnumAccess<'de> {
    /// Creates a new [`AdsEnumAccess`] bound to the resolved PLC variant string name.
    pub fn new(variant_name: &'de str) -> Self {
        Self { variant_name }
    }
}

impl<'de> EnumAccess<'de> for AdsEnumAccess<'de> {
    type Error = crate::Error;
    type Variant = Self;

    fn variant_seed<S>(self, seed: S) -> Result<(S::Value, Self::Variant), Self::Error>
    where
        S: DeserializeSeed<'de>,
    {
        let value = seed.deserialize(StrDeserializer::<crate::Error>::new(self.variant_name))?;
        Ok((value, self))
    }
}

impl<'de> VariantAccess<'de> for AdsEnumAccess<'de> {
    type Error = crate::Error;

    fn unit_variant(self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, _seed: T) -> Result<T::Value, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        Err(crate::Error::TypeMismatch {
            expected: "unit enum variant (PLC enums carry no payload)".into(),
        })
    }

    fn tuple_variant<V>(self, _len: usize, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(crate::Error::TypeMismatch {
            expected: "unit enum variant (PLC enums carry no payload)".into(),
        })
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(crate::Error::TypeMismatch {
            expected: "unit enum variant (PLC enums carry no payload)".into(),
        })
    }
}
