use super::symbol_cache::{SymbolCache, SymbolEntry};
use indexmap::IndexSet;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tcads_core::{AdsSymbolInfo, AdsTypeInfo, SymbolHandle};

/// The maximum number of levels the type-closure batch fetch loop will run
/// before giving up (a corrupted or pathologically deep type table). Shared
/// so both runtimes use the same bound.
pub const MAX_FETCH_LEVELS: usize = 128;

/// A type-erased closure that serializes its captured value directly into a
/// caller-provided buffer, given the PLC type metadata and cache needed to do
/// so. Writing in place (rather than returning a fresh `Vec<u8>`) is what
/// lets a write batch allocate one shared buffer for the whole thing instead
/// of one per pushed value. Plain and synchronous either way: nothing about
/// serializing a value needs `.await`.
pub type SerializerFn<'a> =
    Box<dyn FnOnce(&AdsTypeInfo, &SymbolCache, &mut [u8]) -> crate::Result<()> + 'a>;

/// Captures `value` for later serialization; used by both runtimes'
/// `WriteMultiValues::push`.
pub fn make_serializer<'a, T: serde::Serialize + ?Sized>(value: &'a T) -> SerializerFn<'a> {
    Box::new(move |type_info, cache, buf| {
        tcads_serde::to_bytes(value, buf, type_info, &*cache.types()?)?;
        Ok(())
    })
}

/// Per-entry metadata gathered from a [`SymbolEntry`] in a single lock
/// acquisition.
pub struct Resolved {
    pub handle: SymbolHandle,
    pub size: usize,
    pub type_info: Arc<AdsTypeInfo>,
    pub requires_rmw: bool,
}

/// Splits `paths` into the set that needs fresh symbol info and a handle
/// (never seen before) and the set that only needs a handle (metadata
/// already cached, e.g. via `preload`, but no handle acquired yet).
pub fn partition_missing<'p, S: AsRef<str>>(
    cache: &SymbolCache,
    paths: &'p [S],
) -> crate::Result<(IndexSet<&'p str>, IndexSet<&'p str>)> {
    let mut missing_info_paths = IndexSet::new();
    let mut missing_handle_paths = IndexSet::new();

    for path in paths {
        if let Some(entry) = cache.get(path.as_ref())? {
            let has_handle = entry.read()?.handle().is_some();
            if !has_handle {
                missing_handle_paths.insert(path.as_ref());
            }
        } else {
            missing_info_paths.insert(path.as_ref());
            missing_handle_paths.insert(path.as_ref());
        }
    }

    Ok((missing_info_paths, missing_handle_paths))
}

/// One level of the type-closure batch fetch: given the symbol infos fetched
/// so far, which additional type names does the cache still not have. Empty
/// once the closure is fully resolved; the caller loops this against a batch
/// type fetch (its own network call, so it stays in each runtime file) until
/// it returns empty or [`MAX_FETCH_LEVELS`] is hit.
pub fn batch_missing_types(
    cache: &SymbolCache,
    infos: &[AdsSymbolInfo],
) -> crate::Result<IndexSet<String>> {
    let mut missing = IndexSet::new();
    for info in infos {
        for t in cache.missing_types(info.type_name())? {
            missing.insert(t);
        }
    }
    Ok(missing)
}

/// Populates the cache from batch-fetched symbol infos and handles, keyed
/// (not positional) so a repeated path in the original input can't misalign
/// fetched data against the wrong name: draining the fetched vectors
/// sequentially while checking membership (which doesn't consume) against
/// a possibly-duplicated path list either panics on an exhausted iterator or,
/// worse, silently inserts one symbol's data under a different symbol's name.
pub fn apply_resolved(
    cache: &SymbolCache,
    missing_info_paths: &IndexSet<&str>,
    missing_handle_paths: &IndexSet<&str>,
    fetched_infos: Vec<AdsSymbolInfo>,
    fetched_handles: Vec<SymbolHandle>,
) -> crate::Result<()> {
    let mut info_map = HashMap::new();
    for (path, info) in missing_info_paths.iter().zip(fetched_infos) {
        info_map.insert(*path, info);
    }

    let mut handle_map = HashMap::new();
    for (path, handle) in missing_handle_paths.iter().zip(fetched_handles) {
        handle_map.insert(*path, handle);
    }

    for path in missing_info_paths {
        let info = info_map.remove(path).expect("matched zip above");
        let handle = handle_map.get(path).expect("matched zip above");
        let entry = cache.resolve_entry(info.type_name(), info.size())?;
        cache.insert(Arc::from(*path), entry.with_handle(*handle))?;
    }

    for path in missing_handle_paths {
        if missing_info_paths.contains(path) {
            continue;
        }
        let handle = handle_map.get(path).expect("matched zip above");
        cache.set_handle(path, *handle)?;
    }

    Ok(())
}

/// Looks every path back up from the now-fully-populated cache, in original
/// order, duplicates included: a lookup is idempotent, so repeats are safe
/// here even though they weren't safe in [`apply_resolved`]'s old
/// implementation.
pub fn collect_entries<S: AsRef<str>>(
    cache: &SymbolCache,
    paths: &[S],
) -> crate::Result<Vec<Arc<RwLock<SymbolEntry>>>> {
    let mut entries = Vec::with_capacity(paths.len());
    for path in paths {
        let entry = cache
            .get(path.as_ref())?
            .expect("guaranteed by apply_resolved above");
        entries.push(entry);
    }
    Ok(entries)
}

/// Gathers everything needed from each entry in a single lock acquisition
/// per entry (rather than re-locking later for handle and type info
/// separately), marking a poisoned entry's slot as failed rather than
/// aborting the whole batch over one bad lock.
pub fn gather_resolved(
    entries: &[Arc<RwLock<SymbolEntry>>],
) -> (Vec<Option<Resolved>>, Vec<Option<crate::Result<()>>>) {
    let mut resolved = Vec::with_capacity(entries.len());

    for entry_lock in entries {
        match entry_lock.read() {
            Ok(guard) => resolved.push(Some(Resolved {
                handle: guard
                    .handle()
                    .expect("resolve_multi_symbols attaches handles"),
                size: guard.size() as usize,
                type_info: guard.type_info().clone(),
                requires_rmw: guard.requires_rmw(),
            })),
            Err(_) => resolved.push(None),
        }
    }

    let mut failures = vec![None; entries.len()];
    for (i, r) in resolved.iter().enumerate() {
        if r.is_none() {
            failures[i] = Some(Err(crate::Error::PoisonedLock));
        }
    }

    (resolved, failures)
}

/// Lays out one shared, zero-filled buffer for every not-yet-failed entry,
/// contiguously, and returns each entry's `(offset, size)` slot within it.
///
/// Zero-filled, not just sized: any padding/reserved bytes a serializer
/// doesn't touch must reach the PLC as zero, not whatever the allocator left
/// behind.
pub fn plan_write_buffer(
    resolved: &[Option<Resolved>],
    failures: &[Option<crate::Result<()>>],
) -> (Vec<u8>, Vec<Option<(usize, usize)>>) {
    let mut slots = vec![None; resolved.len()];
    let mut total_size = 0usize;
    for (i, r) in resolved.iter().enumerate() {
        if failures[i].is_some() {
            continue;
        }
        let r = r.as_ref().expect("not failed, so resolution succeeded");
        slots[i] = Some((total_size, r.size));
        total_size += r.size;
    }
    (vec![0u8; total_size], slots)
}

/// Which not-yet-failed entries need a read-modify-write pre-read, as
/// `(handle, size)` request pairs plus their original indices.
pub fn collect_rmw_requests(
    resolved: &[Option<Resolved>],
    failures: &[Option<crate::Result<()>>],
) -> (Vec<(SymbolHandle, usize)>, Vec<usize>) {
    let mut requests = Vec::new();
    let mut indices = Vec::new();
    for (i, r) in resolved.iter().enumerate() {
        if failures[i].is_some() {
            continue;
        }
        let r = r.as_ref().expect("not failed, so resolution succeeded");
        if r.requires_rmw {
            requests.push((r.handle, r.size));
            indices.push(i);
        }
    }
    (requests, indices)
}

/// Copies a successfully read-modify-write result into its slot in `buf`.
pub fn apply_rmw_bytes(
    buf: &mut [u8],
    slots: &[Option<(usize, usize)>],
    index: usize,
    bytes: &[u8],
) {
    let (offset, size) = slots[index].expect("not marked failed above");
    buf[offset..offset + size].copy_from_slice(bytes);
}

/// Runs every not-yet-failed entry's serializer directly into its slot in
/// `buf` (on top of its RMW-fetched current bytes if it needed one, or the
/// zeroed buffer otherwise), recording a serialization failure the same way
/// every earlier stage does.
pub fn serialize_all(
    serializers: Vec<SerializerFn<'_>>,
    resolved: &[Option<Resolved>],
    slots: &[Option<(usize, usize)>],
    cache: &SymbolCache,
    buf: &mut [u8],
    failures: &mut [Option<crate::Result<()>>],
) {
    for (i, serializer) in serializers.into_iter().enumerate() {
        if failures[i].is_some() {
            continue;
        }
        let r = resolved[i]
            .as_ref()
            .expect("not failed, so resolution succeeded");
        let (offset, size) = slots[i].expect("not marked failed above");
        if let Err(e) = serializer(&r.type_info, cache, &mut buf[offset..offset + size]) {
            failures[i] = Some(Err(e));
        }
    }
}

/// Which not-yet-failed entries are ready to write, as `(handle, byte slice,
/// original index)`.
pub fn collect_write_items<'b>(
    resolved: &[Option<Resolved>],
    failures: &[Option<crate::Result<()>>],
    slots: &[Option<(usize, usize)>],
    buf: &'b [u8],
) -> Vec<(SymbolHandle, &'b [u8], usize)> {
    let mut items = Vec::new();
    for (i, r) in resolved.iter().enumerate() {
        if failures[i].is_some() {
            continue;
        }
        let r = r.as_ref().expect("not failed, so resolution succeeded");
        let (offset, size) = slots[i].expect("not marked failed above");
        items.push((r.handle, &buf[offset..offset + size], i));
    }
    items
}

/// Reports the first failure found across every stage (resolve, RMW read,
/// serialize, write), in path order, or `Ok(())` only if every entry made it
/// all the way through.
pub fn first_failure(failures: Vec<Option<crate::Result<()>>>) -> crate::Result<()> {
    for result in failures {
        if let Some(Err(e)) = result {
            return Err(e);
        }
    }
    Ok(())
}

/// Decodes a single already-fetched byte buffer into `T` against the shared
/// type cache; used by both runtimes' `ReadMultiValues::decode`.
pub fn decode<T: serde::de::DeserializeOwned>(
    cache: &SymbolCache,
    bytes: &[u8],
    type_info: &AdsTypeInfo,
) -> crate::Result<T> {
    tcads_serde::from_bytes(bytes, type_info, &*cache.types()?).map_err(Into::into)
}
