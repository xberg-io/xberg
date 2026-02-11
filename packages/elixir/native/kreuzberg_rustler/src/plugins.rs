//! Plugin management NIFs
//!
//! This module provides NIFs for managing document extractor plugins
//! through the Kreuzberg core plugin registry.

use crate::atoms;
use rustler::{Encoder, Env, NifResult, Term};

/// List all registered document extractors.
///
/// Returns the names of all document extractors currently registered
/// in the global plugin registry.
///
/// # Returns
/// * `{:ok, names}` - List of extractor name strings
/// * `{:error, reason}` - Error message string
#[rustler::nif]
pub fn list_document_extractors<'a>(env: Env<'a>) -> NifResult<Term<'a>> {
    match kreuzberg::plugins::list_extractors() {
        Ok(names) => Ok((atoms::ok(), names).encode(env)),
        Err(e) => {
            let error_msg = format!("{}", e);
            Ok((atoms::error(), error_msg).encode(env))
        }
    }
}

/// Unregister a document extractor by name.
///
/// Removes the extractor from the global registry and calls its shutdown method.
/// Safe to call even if the extractor doesn't exist.
///
/// # Arguments
/// * `name` - String name of the extractor to unregister
///
/// # Returns
/// * `:ok` - Extractor unregistered or didn't exist
/// * `{:error, reason}` - Error message string
#[rustler::nif]
pub fn unregister_document_extractor<'a>(env: Env<'a>, name: String) -> NifResult<Term<'a>> {
    match kreuzberg::plugins::unregister_extractor(&name) {
        Ok(()) => Ok(atoms::ok().encode(env)),
        Err(e) => {
            let error_msg = format!("{}", e);
            Ok((atoms::error(), error_msg).encode(env))
        }
    }
}

/// Clear all registered document extractors.
///
/// Removes all extractors from the global registry and calls their shutdown methods.
///
/// # Returns
/// * `:ok` - All extractors cleared
/// * `{:error, reason}` - Error message string
#[rustler::nif]
pub fn clear_document_extractors<'a>(env: Env<'a>) -> NifResult<Term<'a>> {
    match kreuzberg::plugins::clear_extractors() {
        Ok(()) => Ok(atoms::ok().encode(env)),
        Err(e) => {
            let error_msg = format!("{}", e);
            Ok((atoms::error(), error_msg).encode(env))
        }
    }
}
