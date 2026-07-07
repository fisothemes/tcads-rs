use crate::TypeProvider;
use std::collections::HashMap;
use std::ops::Index;
use tcads_core::AdsTypeInfo;

/// A simple, owned in‑memory cache for ADS type metadata.
///
/// This implements [`TypeProvider`] using a `HashMap`. It's suitable for most
/// use cases and can be wrapped in `Arc<RwLock<...>>` if thread‑safety is needed.
///
/// # Example
/// ```
/// use tcads_serde::{AdsTypeCache, TypeProvider};
///
/// let mut cache = AdsTypeCache::new(4);
/// // In practice, you'd fetch metadata from the PLC and insert it.
/// // cache.insert(motor_type_info);
/// ```
#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, PartialEq, Eq, Default)]
pub struct AdsTypeCache {
    map: HashMap<String, AdsTypeInfo>,
    ptr_size: u8,
}

impl AdsTypeCache {
    /// Creates a new empty cache with the given platform pointer size.
    ///
    /// The pointer size should be 4 for 32‑bit platforms or 8 for 64‑bit
    /// platforms. This is used to correctly interpret pointer‑sized types
    /// like `PVOID`, `REFERENCE TO`, and `INTERFACE`.
    pub fn new(ptr_size: u8) -> Self {
        Self {
            map: HashMap::new(),
            ptr_size,
        }
    }

    /// Inserts a type definition into the cache.
    ///
    /// If the type already exists, it is overwritten.
    pub fn insert(&mut self, info: AdsTypeInfo) -> Option<AdsTypeInfo> {
        self.map.insert(info.name().to_string(), info)
    }

    /// Inserts multiple type definitions into the cache.
    ///
    /// This is a convenience method for bulk insertion, e.g., from
    /// `RuntimeDevice::get_all_type_infos()`.
    pub fn insert_all(&mut self, types: impl IntoIterator<Item = AdsTypeInfo>) {
        for info in types {
            self.insert(info);
        }
    }

    /// Removes a type definition from the cache.
    ///
    /// Returns the removed type, or `None` if it was not present.
    pub fn remove(&mut self, name: &str) -> Option<AdsTypeInfo> {
        self.map.remove(name)
    }

    /// Returns `true` if the cache contains a type with the given name.
    pub fn contains_type(&self, name: &str) -> bool {
        self.map.contains_key(name)
    }

    /// Returns an iterator over all type names in the cache.
    pub fn get_type_names(&self) -> impl Iterator<Item = &str> {
        self.map.keys().map(|s| s.as_str())
    }

    /// Returns a mutable reference to the type definition for the given name.
    ///
    /// This allows in‑place modifications of cached type metadata.
    pub fn get_type_info_mut(&mut self, name: &str) -> Option<&mut AdsTypeInfo> {
        self.map.get_mut(name)
    }

    /// Clears all type definitions from the cache.
    pub fn clear(&mut self) {
        self.map.clear();
    }

    /// Sets the platform pointer size.
    pub fn set_platform_ptr_size(&mut self, ptr_size: u8) {
        self.ptr_size = ptr_size;
    }

    /// Returns an iterator over all cached type definitions.
    pub fn iter(&self) -> impl Iterator<Item = &AdsTypeInfo> {
        self.map.values()
    }
}

impl TypeProvider for AdsTypeCache {
    fn get_type_info(&self, type_name: &str) -> Option<&AdsTypeInfo> {
        self.map.get(type_name)
    }

    fn get_platform_ptr_size(&self) -> u8 {
        self.ptr_size
    }
}

impl Index<&str> for AdsTypeCache {
    type Output = AdsTypeInfo;

    /// Returns a reference to the type definition for the given name.
    ///
    /// # Panics
    /// Panics if the type name is not present in the cache.
    fn index(&self, index: &str) -> &AdsTypeInfo {
        self.map.get(index).expect("Type not found in cache")
    }
}
