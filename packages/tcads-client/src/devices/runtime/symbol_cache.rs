//! Path-keyed symbol resolution cache.
//!
//! A [`SymbolCache`] holds everything needed to read or write a symbol by its
//! instance path: its resolved [`AdsTypeInfo`], its size, whether writes require
//! a read-modify-write cycle, and (once acquired) its [`SymbolHandle`]. Type
//! metadata is shared through an inner [`AdsTypeCache`] so that a thousand
//! symbols of the same function block type reference one type description.
//!
//! All entries go stale together when the symbol version changes; call [`SymbolCache::clear`]
//! when [`AdsReturnCode::AdsErrDeviceSymbolVersionInvalid`](tcads_core::AdsReturnCode)
//! is observed or a symbol-version notification fires. Stale handles need no
//! release call: the PLC already discarded them when it reloaded the symbol table.

use std::collections::HashMap;
use std::sync::{Arc, RwLock, RwLockReadGuard};

use tcads_core::{AdsTypeInfo, SymbolHandle};
use tcads_serde::{AdsTypeCache, TypeProvider, resolvers};

/// A fully resolved symbol: type metadata plus an optionally acquired handle.
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

    /// Attaches an acquired handle. To update a handle on an entry already in
    /// the cache, use [`SymbolCache::set_handle`] instead, which locks just
    /// that one entry rather than replacing it.
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
    /// See [`resolvers::requires_read_modify_write`] for the rules.
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
    pub fn get(&self, path: &str) -> crate::Result<Option<Arc<RwLock<SymbolEntry>>>> {
        Ok(self.entries.read()?.get(path).cloned())
    }

    /// Inserts (or replaces) the entry for `path`, returning the previous one.
    pub fn insert(
        &self,
        path: Arc<str>,
        entry: SymbolEntry,
    ) -> crate::Result<Option<Arc<RwLock<SymbolEntry>>>> {
        Ok(self
            .entries
            .write()?
            .insert(path, Arc::new(RwLock::new(entry))))
    }

    /// Records an acquired handle on an existing entry.
    ///
    /// Only the target entry is locked for writing; concurrent access to every
    /// other cached symbol is unaffected. Returns `Ok(false)` if `path` has no
    /// entry (e.g. it was flushed concurrently), in which case the caller
    /// should re-resolve rather than retry blindly. Returns
    /// [`Error::PoisonedLock`](crate::Error::PoisonedLock), not `Ok(false)`,
    /// if a lock is poisoned: that's a different problem than "not cached
    /// yet" and re-resolving won't fix it.
    pub fn set_handle(&self, path: &str, handle: SymbolHandle) -> crate::Result<bool> {
        let Some(entry) = self.entries.read()?.get(path).cloned() else {
            return Ok(false);
        };
        entry.write()?.handle = Some(handle);
        Ok(true)
    }

    /// Removes a single entry, e.g. after a zero-length "handle invalid" advice
    /// notification for that symbol.
    pub fn remove(&self, path: &str) -> crate::Result<Option<Arc<RwLock<SymbolEntry>>>> {
        Ok(self.entries.write()?.remove(path))
    }

    /// Flushes all entries and all cached types.
    ///
    /// Call on `AdsErrDeviceSymbolVersionInvalid`, a symbol-version notification,
    /// or reconnect. Handles are not released: the PLC invalidated them already.
    ///
    /// A poisoned lock here is deliberately swallowed rather than propagated:
    /// clearing is itself the recovery action for a wide range of failure
    /// modes, and a caller invalidating the cache almost never wants that
    /// specific call to fail. If a lock is poisoned, the very next [`get`](Self::get),
    /// [`insert`](Self::insert), or [`resolve_entry`](Self::resolve_entry) call will still
    /// surface it properly.
    pub fn clear(&self) {
        if let Ok(mut map) = self.entries.write() {
            map.clear();
        }
        if let Ok(mut types) = self.types.write() {
            types.clear();
        }
    }

    /// Whether the type table already contains `type_name`.
    pub fn contains_type(&self, type_name: &str) -> crate::Result<bool> {
        Ok(self.types()?.contains_type(type_name))
    }

    /// Inserts type descriptions into the shared type table.
    pub fn insert_types(&self, types: impl IntoIterator<Item = AdsTypeInfo>) -> crate::Result<()> {
        self.types.write()?.insert_all(types);
        Ok(())
    }

    /// Type names reachable from `type_name` that aren't cached yet, for
    /// batched fetching (see [`tcads_serde::resolvers::missing_types`]).
    ///
    /// Returns an empty `Vec` (not an error) if `type_name` itself is missing
    /// too; the caller fetches it like any other entry in the returned list.
    pub fn missing_types(&self, type_name: &str) -> crate::Result<Vec<String>> {
        let types = self.types()?;
        Ok(match types.get_type_info(type_name) {
            Some(info) => resolvers::missing_types(info, &*types),
            None => vec![type_name.to_string()],
        })
    }

    /// Resolves the metadata half of an entry from the type table: clones the
    /// type out as an [`Arc`] and computes
    /// [`requires_read_modify_write`](resolvers::requires_read_modify_write).
    ///
    /// Returns [`tcads_serde::Error::TypeNotFound`] (wrapped in
    /// [`Error::Serde`](crate::Error::Serde)) if `type_name` or any nested type
    /// is missing; the caller should fetch it from the PLC via
    /// [`missing_types`](Self::missing_types) and [`insert_types`](Self::insert_types),
    /// and call this again.
    pub fn resolve_entry(&self, type_name: &str, size: u32) -> crate::Result<SymbolEntry> {
        let types = self.types()?;
        let info = types
            .get_type_info(type_name)
            .ok_or_else(|| tcads_serde::Error::TypeNotFound(type_name.to_string()))?;
        let requires_rmw = resolvers::requires_read_modify_write(info, &*types)?;
        Ok(SymbolEntry::new(Arc::new(info.clone()), size, requires_rmw))
    }

    /// Read guard over the shared type table.
    ///
    /// [`AdsTypeCache`] already implements [`TypeProvider`], and
    /// the guard derefs to it, so pass `&*cache.types()?` anywhere an
    /// `impl TypeProvider` is expected (e.g. [`tcads_serde::to_bytes`],
    /// [`tcads_serde::from_bytes`]). `SymbolCache` itself can't implement
    /// `TypeProvider`: that trait returns a bare `&AdsTypeInfo` tied only to
    /// `&self`, which can't be produced from behind a lock without holding a
    /// guard alive past the call, this guard *is* that lifetime, held
    /// explicitly by the caller instead.
    pub fn types(&self) -> crate::Result<RwLockReadGuard<'_, AdsTypeCache>> {
        Ok(self.types.read()?)
    }
}
