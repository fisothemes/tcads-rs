use super::symbol_cache::SymbolCache;
use indexmap::IndexSet;
use tcads_core::{AdsMethodInfo, AdsTypeCategory, AdsTypeInfo};
use tcads_serde::TypeProvider;
use tcads_serde::resolvers::ResolvedField;

/// Finds a method by name on an already-resolved type.
pub fn find_method<'m>(
    type_info: &'m AdsTypeInfo,
    method_name: &str,
) -> crate::Result<&'m AdsMethodInfo> {
    let method = type_info
        .method_infos()
        .iter()
        .find(|m| m.name().eq_ignore_ascii_case(method_name))
        .ok_or_else(|| crate::Error::MethodNotFound {
            type_name: type_info.name().into(),
            method_name: method_name.into(),
        })?;

    if method.flags().is_not_callable() {
        return Err(crate::Error::MethodNotCallable {
            method_name: method.name().into(),
        });
    }

    Ok(method)
}

/// Every type name a method's parameters (return value, `VAR_INPUT`,
/// `VAR_OUTPUT`, `VAR_IN_OUT`) reference that isn't already cached.
pub fn missing_method_types(
    cache: &SymbolCache,
    method: &AdsMethodInfo,
) -> crate::Result<IndexSet<String>> {
    let mut missing = IndexSet::new();

    if let Some(ret) = method.return_type() {
        if !cache.contains_type(ret.name())? {
            missing.insert(ret.name().to_string());
        }
    }
    for p in method.parameters() {
        if !cache.contains_type(p.type_name())? {
            missing.insert(p.type_name().to_string());
        }
    }

    Ok(missing)
}

/// Type names reachable from `type_names`' own nested fields that the cache
/// doesn't have yet, for the next round of batched fetching: the same
/// level-by-level pattern `resolve_multi_symbols` already uses, just seeded
/// from a method's parameter type names instead of a symbol's own type name.
pub fn missing_nested_types<'a>(
    cache: &SymbolCache,
    type_names: impl IntoIterator<Item = &'a str>,
) -> crate::Result<IndexSet<String>> {
    let mut missing = IndexSet::new();
    for name in type_names {
        for t in cache.missing_types(name)? {
            missing.insert(t);
        }
    }
    Ok(missing)
}

/// The RPC-specific half of pre-fetching: for each of `type_names` that's
/// already cached and turns out to be a `Reference` or `Alias`, returns the
/// real underlying type name it points to, if that isn't cached yet either.
pub fn missing_rpc_indirections<'a>(
    cache: &SymbolCache,
    type_names: impl IntoIterator<Item = &'a str>,
) -> crate::Result<IndexSet<String>> {
    let types = cache.types()?;
    let ptr_size = types.get_platform_ptr_size();
    let mut missing = IndexSet::new();

    for name in type_names {
        let Some(info) = types.get_type_info(name) else {
            continue;
        };
        if matches!(
            AdsTypeCategory::determine(info, ptr_size),
            AdsTypeCategory::Reference | AdsTypeCategory::Alias
        ) {
            let target = info.type_name();
            if types.get_type_info(target).is_none() {
                missing.insert(target.to_string());
            }
        }
    }

    Ok(missing)
}

/// Peels through `Reference`/`Alias` wrapping to the real underlying type,
/// for a single RPC parameter's type name.
pub fn resolve_rpc_parameter_type<'p>(
    type_name: &str,
    provider: &'p impl TypeProvider,
) -> crate::Result<&'p AdsTypeInfo> {
    let raw = provider
        .get_type_info(type_name)
        .ok_or_else(|| tcads_serde::Error::TypeNotFound(type_name.to_string()))?;
    let ptr_size = provider.get_platform_ptr_size();
    Ok(tcads_serde::resolvers::resolve_indirection(
        raw, provider, ptr_size,
    )?)
}

/// Resolved input fields: one per `VAR_INPUT` or `VAR_IN_OUT` parameter, in declared
/// order, tightly packed.
///
/// Every parameter's type must already be resolved in `provider`
/// (see [`missing_method_types`]/[`missing_nested_types`]).
pub fn input_fields<'p>(
    method: &'p AdsMethodInfo,
    provider: &'p impl TypeProvider,
) -> crate::Result<Vec<ResolvedField<'p>>> {
    let mut offset = 0u32;
    method
        .parameters()
        .iter()
        .filter(|p| p.flags().is_input())
        .map(|p| {
            let type_info = resolve_rpc_parameter_type(p.type_name(), provider)?;
            let size = type_info.size();
            let field = ResolvedField::new(offset, size, type_info).with_name(p.name());
            offset += size;
            Ok(field)
        })
        .collect()
}

/// Resolved output fields: the return value first (if the method has one),
/// then one per `VAR_OUTPUT` or `VAR_IN_OUT` parameter, in declared order, tightly
/// packed.
///
/// Every parameter's type must already be resolved in `provider`
/// (see [`missing_method_types`]/[`missing_nested_types`]).
pub fn output_fields<'p>(
    method: &'p AdsMethodInfo,
    provider: &'p impl TypeProvider,
) -> crate::Result<Vec<ResolvedField<'p>>> {
    let mut offset = 0u32;
    let mut fields = Vec::new();

    if let Some(ret) = method.return_type() {
        let type_info = resolve_rpc_parameter_type(ret.name(), provider)?;
        let size = type_info.size();
        fields.push(ResolvedField::new(offset, size, type_info).with_name("<return>"));
        offset += size;
    }

    for p in method.parameters().iter().filter(|p| p.flags().is_output()) {
        let type_info = resolve_rpc_parameter_type(p.type_name(), provider)?;
        let size = type_info.size();
        fields.push(ResolvedField::new(offset, size, type_info).with_name(p.name()));
        offset += size;
    }

    Ok(fields)
}
