//! Centralized cache directory resolution for all xberg modules.
//!
//! Provides a single function that all modules use to determine where to store
//! cached data (models, OCR results, tessdata, etc.). This avoids per-CWD
//! `.xberg/` directories and uses platform-appropriate global cache locations.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

pub(crate) const OCR_MODEL_CACHE_DIR_ENV: &str = "XBERG_OCR_MODEL_CACHE_DIR";

pub(crate) fn ocr_model_cache_override(explicit: Option<&Path>) -> crate::Result<Option<PathBuf>> {
    ocr_model_cache_override_for(explicit, std::env::var_os(OCR_MODEL_CACHE_DIR_ENV).as_deref())
}

fn ocr_model_cache_override_for(
    explicit: Option<&Path>,
    environment: Option<&OsStr>,
) -> crate::Result<Option<PathBuf>> {
    if let Some(path) = explicit {
        if path.as_os_str().is_empty() {
            return Err(crate::XbergError::validation("OCR model cache_dir must not be empty"));
        }
        return Ok(Some(path.to_path_buf()));
    }
    Ok(environment.filter(|path| !path.is_empty()).map(PathBuf::from))
}

/// Resolve the xberg cache base directory (without a module suffix).
///
/// Uses the same resolution order as [`resolve_cache_dir`] but returns
/// the top-level xberg cache directory.
#[allow(dead_code)]
pub(crate) fn resolve_cache_base() -> PathBuf {
    cache_base_for(std::env::var("XBERG_CACHE_DIR").ok().as_deref().map(Path::new))
}

/// The cache base for a given `XBERG_CACHE_DIR` value, without reading the environment.
pub(crate) fn cache_base_for(env_override: Option<&Path>) -> PathBuf {
    if let Some(env_path) = env_override {
        return env_path.to_path_buf();
    }
    if let Some(cache) = dirs::cache_dir() {
        return cache.join("xberg");
    }
    if let Some(home) = dirs::home_dir() {
        return home.join(".cache").join("xberg");
    }
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".xberg")
}

/// Resolve the xberg cache directory for a given module.
///
/// Resolution order:
/// 1. `XBERG_CACHE_DIR` env var + `/{module}` (explicit override)
/// 2. Platform-appropriate global cache directory:
///    - macOS: `~/Library/Caches/xberg/{module}`
///    - Linux: `$XDG_CACHE_HOME/xberg/{module}` or `~/.cache/xberg/{module}`
///    - Windows: `%LOCALAPPDATA%/xberg/{module}`
/// 3. Home directory fallback: `~/.cache/xberg/{module}`
/// 4. CWD-relative fallback: `.xberg/{module}` (last resort, e.g. no HOME set)
#[cfg_attr(alef, alef(skip))]
pub(crate) fn resolve_cache_dir(module: &str) -> PathBuf {
    if let Ok(env_path) = std::env::var("XBERG_CACHE_DIR") {
        return PathBuf::from(env_path).join(module);
    }
    if let Some(cache) = dirs::cache_dir() {
        return cache.join("xberg").join(module);
    }
    if let Some(home) = dirs::home_dir() {
        return home.join(".cache").join("xberg").join(module);
    }
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".xberg")
        .join(module)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;

    #[test]
    fn explicit_ocr_model_cache_root_wins_over_environment() {
        let resolved =
            ocr_model_cache_override_for(Some(Path::new("/request/cache")), Some(OsStr::new("/deployment/cache")))
                .expect("a non-empty explicit root should be valid");

        assert_eq!(resolved, Some(PathBuf::from("/request/cache")));
    }

    #[test]
    fn deployment_ocr_model_cache_root_is_used_without_explicit_root() {
        let resolved = ocr_model_cache_override_for(None, Some(OsStr::new("/deployment/cache")))
            .expect("a non-empty environment root should be valid");

        assert_eq!(resolved, Some(PathBuf::from("/deployment/cache")));
    }

    #[test]
    fn empty_deployment_ocr_model_cache_root_is_ignored() {
        let resolved = ocr_model_cache_override_for(None, Some(OsStr::new("")))
            .expect("an empty environment value should behave as unset");

        assert_eq!(resolved, None);
    }

    #[test]
    fn empty_explicit_ocr_model_cache_root_is_rejected() {
        let error = ocr_model_cache_override_for(Some(Path::new("")), Some(OsStr::new("/deployment/cache")))
            .expect_err("an empty explicit root must not fall through to the environment");

        assert_eq!(
            error.to_string(),
            "Validation error: OCR model cache_dir must not be empty"
        );
    }

    #[test]
    fn ocr_model_cache_override_does_not_redirect_general_cache_resolution() {
        let general = cache_base_for(Some(Path::new("/general/cache")));
        let ocr = ocr_model_cache_override_for(None, Some(OsStr::new("/ocr/cache")))
            .expect("an OCR-only environment root should be valid");

        assert_eq!(ocr, Some(PathBuf::from("/ocr/cache")));
        assert_eq!(general, PathBuf::from("/general/cache"));
    }
}
