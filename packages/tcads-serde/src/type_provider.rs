use tcads_core::ads::AdsTypeInfo;

/// A trait for fetching TwinCAT type metadata.
///
/// # Note
///
/// The provider is responsible for caching [`AdsTypeInfo`] entries to avoid
/// expensive network round‑trips. Since each [`get_type_info`](TypeProvider::get_type_info) call
/// would otherwise require an ADS read, it is strongly recommended to implement
/// this with a cache (e.g. `HashMap<String, AdsTypeInfo>`).
pub trait TypeProvider {
    /// Returns the type definition for the given type name, or [`None`] if not found.
    ///
    /// # Performance
    ///
    /// This method is called frequently during serialization and deserialization of nested structs.
    /// The provider should return `O(1)` lookups.
    fn get_type_info(&self, type_name: &str) -> Option<&AdsTypeInfo>;

    /// Returns the platform pointer size.
    ///
    /// This can be used to correctly interpret between interfaces vs FBs in some situations.
    fn get_platform_ptr_size(&self) -> u8;
}
