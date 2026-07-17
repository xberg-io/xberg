//! Option decoding for R bindings.

use extendr_api::prelude::*;

/// Helper: extract and convert a value from an R list by name.
fn list_get(list: &List, key: &str) -> Option<Robj> {
    list.iter().find(|(n, _)| *n == key).map(|(_, v)| v)
}

/// Decode a execution provider type enum from its string representation.
fn decode_execution_provider_type(val: Robj) -> std::result::Result<crate::ExecutionProviderType, String> {
    let s = String::try_from(&val).map_err(|e| format!("execution_provider_type: {e}"))?;
    match s.as_str() {
        "Auto" => Ok(crate::ExecutionProviderType::Auto),
        "Cpu" => Ok(crate::ExecutionProviderType::Cpu),
        "CoreMl" => Ok(crate::ExecutionProviderType::CoreMl),
        "Cuda" => Ok(crate::ExecutionProviderType::Cuda),
        "TensorRt" => Ok(crate::ExecutionProviderType::TensorRt),
        _ => Err(format!("execution_provider_type: unknown variant '{}'", s)),
    }
}

/// Decode an R ExternalPtr, NULL, or named list into AccelerationConfig.
///
/// Accepts:
/// - ExternalPtr of the configured options type (from $default() or builder methods) — unwraps and converts
/// - NULL — returns the configured options type's default
/// - Named list with field names matching struct fields — decodes field by field
///
/// Fields are optional: omitted fields retain their defaults. Unknown fields are ignored.
pub fn decode_options(options: Robj) -> std::result::Result<crate::AccelerationConfig, String> {
    if options.is_null() {
        return Ok(crate::AccelerationConfig::default());
    }

    // Accept the wrapper struct returned by the options type's default() / builder methods,
    // which extendr exposes as an `ExternalPtr`. The binding struct is returned directly
    // from the #[extendr] impl methods, so unwrap it as the binding type.
    if let Ok(ext) = ExternalPtr::<crate::AccelerationConfig>::try_from(&options) {
        // Clone the binding struct and convert to core type via the generated From impl
        return Ok((*ext).clone().into());
    }

    // Try to decode as a named list
    let list =
        List::try_from(&options).map_err(|e| format!("options must be NULL, ExternalPtr, or named list: {e}"))?;
    let mut opts = crate::AccelerationConfig::default();

    if let Some(v) = list_get(&list, "provider") {
        opts.provider = decode_execution_provider_type(v)?;
    }
    if let Some(v) = list_get(&list, "device_id") {
        opts.device_id = u32::try_from(&v).map_err(|e| format!("device_id: {e}"))?;
    }
    // Note: visitor field is skipped — R has no visitor concept, so it remains at default None

    Ok(opts)
}
