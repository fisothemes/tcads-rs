use crate::TypeProvider;
use tcads_core::{AdsTypeCategory, AdsTypeInfo};

// The maximum allowed depth when recursively unwrapping `ALIAS` types.
/// Prevents cyclic alias declarations in the PLC from causing an infinite loop.
pub const MAX_ALIAS_DEPTH: usize = 32;

/// Recursively resolves an IEC 61131-3 `ALIAS` type down to its underlying base memory type.
///
/// If the provided `type_info` is not an alias, it is immediately returned.
/// If it is an alias, this function requests the target types sequentially from the
/// [`TypeProvider`] until a non-alias type (like a Struct, Enum, or Primitive) is reached.
///
/// # Errors
///
/// - Returns [`crate::Error::TypeNotFound`] if the `TypeProvider` is missing a type in the chain.
///
/// - Returns [`crate::Error::Custom`] if the resolution chain exceeds `MAX_ALIAS_DEPTH`,
/// which typically indicates a cyclic type definition in the PLC.
pub fn resolve_alias<'a>(
    type_info: &'a AdsTypeInfo,
    provider: &'a impl TypeProvider,
    platform_ptr_size: u8,
) -> Result<&'a AdsTypeInfo, crate::Error> {
    let mut type_info = type_info;
    for _ in 0..MAX_ALIAS_DEPTH {
        if AdsTypeCategory::determine(type_info, platform_ptr_size) != AdsTypeCategory::Alias {
            return Ok(type_info);
        }
        type_info = provider
            .get_type_info(type_info.type_name())
            .ok_or_else(|| crate::Error::TypeNotFound(type_info.type_name().to_string()))?;
    }
    Err(crate::Error::Custom(format!(
        "alias chain exceeds {MAX_ALIAS_DEPTH} levels resolving '{}': type table likely contains a cycle",
        type_info.name(),
    )))
}

/// The maximum number of distinct type names to visit before assuming a
/// corrupted type table. Distinct from cycle detection (which is exact, via a
/// visited set): this only guards against pathologically wide type graphs.
pub const MAX_TYPES_VISITED: usize = 4096;

/// Returns `true` if writing a value of this type requires a read-modify-write cycle.
///
/// Function block (and program) instances carry hidden runtime state at the start of
/// their memory layout (the vtable pointer). The [serializer](crate::ser) only writes
/// declared field bytes, so serializing into a zeroed buffer and sending it to the PLC
/// would clobber that state with zeros. For such types the caller must first read the
/// current bytes from the PLC, serialize the value *into* that buffer, and write the
/// result back.
///
/// The check is a graph search: a struct containing an FB, an array of FBs, or any
/// deeper composition thereof all require read-modify-write. Plain structs, unions,
/// arrays, strings, enums, and primitives do not. Implemented iteratively with an
/// explicit worklist and a visited-set, rather than recursion: a genuine cycle in the
/// type graph terminates cleanly (a type is only ever expanded once), a deep-but-acyclic
/// type isn't mistaken for one, and there's no call-stack growth to bound.
///
/// Pointer, reference, and interface *fields* do not trigger RMW on their own: they map
/// to pointer-sized integers during (de)serialization, so the value supplied by the
/// caller covers those bytes. Preserving or corrupting them is the caller's
/// responsibility.
///
/// # Errors
///
/// - Returns [`crate::Error::TypeNotFound`] if the [`TypeProvider`] is missing a nested type.
/// - Returns [`crate::Error::Custom`] if more than [`MAX_TYPES_VISITED`] distinct types are
///   reachable from `type_info`, which typically indicates a corrupted type table
///   (genuine cycles do not hit this: they terminate via the visited set instead).
pub fn requires_read_modify_write(
    type_info: &AdsTypeInfo,
    provider: &impl TypeProvider,
) -> Result<bool, crate::Error> {
    let ptr_size = provider.get_platform_ptr_size();
    let mut worklist = vec![type_info.name().to_string()];
    let mut visited = std::collections::HashSet::new();

    while let Some(name) = worklist.pop() {
        if !visited.insert(name.clone()) {
            continue;
        }
        if visited.len() > MAX_TYPES_VISITED {
            return Err(crate::Error::Custom(format!(
                "type graph of '{}' reaches over {MAX_TYPES_VISITED} distinct types: \
                 type table likely corrupted",
                type_info.name(),
            )));
        }

        let current = provider
            .get_type_info(&name)
            .ok_or_else(|| crate::Error::TypeNotFound(name.clone()))?;
        let current = resolve_alias(current, provider, ptr_size)?;

        match AdsTypeCategory::determine(current, ptr_size) {
            AdsTypeCategory::FunctionBlock | AdsTypeCategory::Program => return Ok(true),
            AdsTypeCategory::Array => {
                worklist.push(current.type_name().to_string());
            }
            AdsTypeCategory::Struct | AdsTypeCategory::Union => {
                for field in current.field_infos() {
                    worklist.push(field.type_name().to_string());
                }
            }
            _ => {}
        }
    }

    Ok(false)
}
