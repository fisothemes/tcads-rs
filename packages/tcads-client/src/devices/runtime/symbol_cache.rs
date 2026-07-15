use std::collections::HashMap;
use std::sync::{Arc, RwLock, RwLockReadGuard};

use tcads_core::{AdsTypeInfo, SymbolHandle};
use tcads_serde::{AdsTypeCache, TypeProvider, resolvers::requires_read_modify_write};

/// A fully resolved symbol: type metadata plus an optionally acquired handle.
#[derive(Debug, Clone, PartialEq)]
pub struct SymbolEntry {
    type_info: Arc<AdsTypeInfo>,
    size: u32,
    requires_rmw: bool,
    handle: Option<SymbolHandle>,
}

impl SymbolEntry {
    /// Creates an entry without a handle (metadata only).
    pub fn new(type_info: Arc<AdsTypeInfo>, size: u32, requires_rmw: bool) -> Self {
        Self {
            type_info,
            size,
            requires_rmw,
            handle: None,
        }
    }

    /// Attaches an acquired handle.
    pub fn with_handle(self, handle: SymbolHandle) -> Self {
        Self {
            handle: Some(handle),
            ..self
        }
    }

    /// The symbol's resolved type description.
    pub fn type_info(&self) -> &Arc<AdsTypeInfo> {
        &self.type_info
    }

    /// The symbol's size in bytes on the PLC.
    pub fn size(&self) -> u32 {
        self.size
    }

    /// Whether writing this symbol requires a read-modify-write cycle.
    ///
    /// See [`requires_read_modify_write`] for the rules.
    pub fn requires_rmw(&self) -> bool {
        self.requires_rmw
    }

    /// The acquired symbol handle, if any.
    pub fn handle(&self) -> Option<SymbolHandle> {
        self.handle
    }
}

/// Cache mapping symbol instance paths to [`SymbolEntry`]s, backed by a shared
/// [`AdsTypeCache`] for type metadata.
pub struct SymbolCache {
    entries: RwLock<HashMap<Arc<str>, Arc<RwLock<SymbolEntry>>>>,
    types: RwLock<AdsTypeCache>,
}

impl SymbolCache {
    /// Creates an empty cache for the given platform pointer size.
    pub fn new(ptr_size: u8) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            types: RwLock::new(AdsTypeCache::new(ptr_size)),
        }
    }

    /// Returns the cached entry for a symbol `path`, if present.
    ///
    /// The returned `Arc` can be locked (`.read()`/`.write()`) by the caller
    /// directly, or held onto to skip the map lookup on repeat access.
    pub fn get(&self, path: &str) -> Option<Arc<RwLock<SymbolEntry>>> {
        self.entries
            .read()
            .ok()
            .and_then(|map| map.get(path).cloned())
    }

    /// Inserts (or replaces) the entry for `path`, returning the previous one.
    pub fn insert(&self, path: Arc<str>, entry: SymbolEntry) -> Option<Arc<RwLock<SymbolEntry>>> {
        self.entries
            .write()
            .ok()
            .and_then(|mut map| map.insert(path, Arc::new(RwLock::new(entry))))
    }

    /// Records an acquired handle on an existing entry.
    ///
    /// Only the target entry is locked for writing; concurrent access to every
    /// other cached symbol is unaffected. Returns `false` if `path` has no
    /// entry (e.g. it was flushed concurrently), in which case the caller
    /// should re-resolve rather than retry blindly.
    pub fn set_handle(&self, path: &str, handle: SymbolHandle) -> bool {
        let Some(entry) = self
            .entries
            .read()
            .ok()
            .and_then(|map| map.get(path).cloned())
        else {
            return false;
        };
        match entry.write() {
            Ok(mut guard) => {
                guard.handle = Some(handle);
                true
            }
            Err(_) => false,
        }
    }

    /// Removes a single entry, e.g. after a zero-length "handle invalid" advice
    /// notification for that symbol.
    pub fn remove(&self, path: &str) -> Option<Arc<RwLock<SymbolEntry>>> {
        self.entries
            .write()
            .ok()
            .and_then(|mut map| map.remove(path))
    }

    /// Flushes all entries and all cached types.
    ///
    /// Call on [`AdsErrDeviceSymbolVersionInvalid`](tcads_core::AdsReturnCode::AdsErrDeviceSymbolVersionInvalid),
    /// a symbol-version notification, or reconnect. Handles are not released: the PLC invalidated
    /// them already.
    pub fn clear(&self) {
        if let Ok(mut map) = self.entries.write() {
            map.clear();
        }
        if let Ok(mut types) = self.types.write() {
            types.clear();
        }
    }

    // Whether the type table already contains `type_name`.
    pub fn contains_type(&self, type_name: &str) -> bool {
        self.types
            .read()
            .map(|t| t.contains_type(type_name))
            .unwrap_or(false)
    }

    /// Inserts type descriptions into the shared type table.
    pub fn insert_types(&self, types: impl IntoIterator<Item = AdsTypeInfo>) {
        if let Ok(mut t) = self.types.write() {
            t.insert_all(types);
        }
    }

    /// Resolves the metadata half of an entry from the type table: clones the
    /// type out as an [`Arc`] and computes [`requires_read_modify_write`].
    ///
    /// Returns [`tcads_serde::Error::TypeNotFound`] if `type_name` or any nested
    /// type is missing; the caller should fetch it from the PLC, [`insert_types`](Self::insert_types),
    /// and call this again.
    pub fn resolve_entry(
        &self,
        type_name: &str,
        size: u32,
    ) -> Result<SymbolEntry, tcads_serde::Error> {
        let types = self.types();
        let info = types
            .get_type_info(type_name)
            .ok_or_else(|| tcads_serde::Error::TypeNotFound(type_name.to_string()))?;
        let requires_rmw = requires_read_modify_write(info, &*types)?;
        Ok(SymbolEntry::new(Arc::new(info.clone()), size, requires_rmw))
    }

    /// Read guard over the shared type table.
    ///
    /// [`AdsTypeCache`] already implements [`TypeProvider`], and
    /// the guard derefs to it, so pass `&*cache.types()` anywhere an
    /// `impl TypeProvider` is expected (e.g. [`tcads_serde::to_bytes`],
    /// [`tcads_serde::from_bytes`]). `SymbolCache` itself can't implement
    /// `TypeProvider`: that trait returns a bare `&AdsTypeInfo` tied only to
    /// `&self`, which can't be produced from behind a lock without holding a
    /// guard alive past the call, this guard *is* that lifetime, held
    /// explicitly by the caller instead.
    pub fn types(&self) -> RwLockReadGuard<'_, AdsTypeCache> {
        self.types.read().expect("SymbolCache type lock poisoned")
    }
}
