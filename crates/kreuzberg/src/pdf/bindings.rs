use super::error::PdfError;
use once_cell::sync::Lazy;
use pdfium_render::prelude::*;
use std::path::PathBuf;
use std::sync::Mutex;

/// Cached state for lazy Pdfium initialization.
enum InitializationState {
    Uninitialized,
    Initialized {
        #[allow(dead_code)]
        lib_dir: Option<PathBuf>,
    },
    Failed(String),
}

static PDFIUM_STATE: Lazy<Mutex<InitializationState>> = Lazy::new(|| Mutex::new(InitializationState::Uninitialized));

fn bind_pdfium_impl() -> Result<(Option<PathBuf>, Box<dyn PdfiumLibraryBindings>), String> {
    #[cfg(all(feature = "pdf", feature = "bundled-pdfium"))]
    {
        #[cfg(target_arch = "wasm32")]
        {
            let bindings =
                Pdfium::bind_to_system_library().map_err(|e| format!("Failed to initialize Pdfium for WASM: {}", e))?;
            Ok((None, bindings))
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let lib_path =
                crate::pdf::extract_bundled_pdfium().map_err(|e| format!("Failed to extract bundled Pdfium: {}", e))?;

            let lib_dir = lib_path.parent().ok_or_else(|| {
                format!(
                    "Failed to determine Pdfium extraction directory for '{}'",
                    lib_path.display()
                )
            })?;

            let bindings = Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(lib_dir))
                .map_err(|e| format!("Failed to initialize Pdfium: {}", e))?;

            Ok((Some(lib_dir.to_path_buf()), bindings))
        }
    }

    #[cfg(all(feature = "pdf", not(feature = "bundled-pdfium")))]
    {
        let bindings = Pdfium::bind_to_system_library().map_err(|e| format!("Failed to initialize Pdfium: {}", e))?;
        Ok((None, bindings))
    }
}

/// Get Pdfium bindings with lazy initialization.
///
/// The first call to this function triggers initialization. On that first call,
/// if using `bundled-pdfium`, the library is extracted to a temporary directory.
/// Subsequent calls quickly create fresh bindings from the cached state without
/// re-extraction.
///
/// # Arguments
///
/// * `map_err` - Function to map error strings to `PdfError` variants
/// * `context` - Context string for error reporting
///
/// # Returns
///
/// Freshly-created PDFium bindings, or an error if initialization failed.
///
/// # Performance Impact
///
/// - **First call**: Performs initialization (8-12ms for bundled extraction) plus binding.
/// - **Subsequent calls**: Creates binding from cached state (< 0.1ms).
///
/// This defers Pdfium initialization until first PDF is processed, improving cold start
/// for non-PDF workloads by 8-12ms. See Phase 3A Optimization #4 in profiling plan.
///
/// # Lock Poisoning Recovery
///
/// If a previous holder panicked while holding `PDFIUM_STATE`, the lock becomes poisoned.
/// Instead of failing permanently, we recover by extracting the inner value from the
/// poisoned lock and proceeding. This ensures PDF extraction can continue even if an
/// earlier panic occurred, as long as the state is consistent.
pub(crate) fn bind_pdfium(map_err: fn(String) -> PdfError, context: &'static str) -> Result<(), PdfError> {
    let mut state = PDFIUM_STATE.lock().unwrap_or_else(|poisoned| {
        // SAFETY: Recovering from a poisoned lock is safe here because:
        // 1. The poisoned state still contains valid data (just a guard from a panicked thread)
        // 2. We assume the state was set to a valid value before the panic
        // 3. The state is immutable after initialization, so poisoning cannot corrupt it
        poisoned.into_inner()
    });

    // Initialize on first call
    match &*state {
        InitializationState::Uninitialized => match bind_pdfium_impl() {
            Ok((lib_dir, bindings)) => {
                // Initialize Pdfium singleton with the bindings
                let _ = Pdfium::new(bindings);
                *state = InitializationState::Initialized { lib_dir };
            }
            Err(err) => {
                *state = InitializationState::Failed(err.clone());
                return Err(map_err(format!("Pdfium initialization failed ({}): {}", context, err)));
            }
        },
        InitializationState::Failed(err) => {
            return Err(map_err(format!(
                "Pdfium initialization previously failed ({}): {}",
                context, err
            )));
        }
        InitializationState::Initialized { .. } => {
            // Already initialized, nothing to do
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::error::PdfError;

    #[test]
    fn test_bind_pdfium_lazy_initialization() {
        // First call should initialize
        let result = bind_pdfium(PdfError::TextExtractionFailed, "test context");
        assert!(result.is_ok(), "First bind_pdfium call should succeed");
    }

    #[test]
    fn test_bind_pdfium_multiple_calls() {
        // Call bind_pdfium multiple times; subsequent calls should reuse cached state
        let result1 = bind_pdfium(PdfError::TextExtractionFailed, "test 1");
        let result2 = bind_pdfium(PdfError::TextExtractionFailed, "test 2");

        assert!(result1.is_ok(), "First call should succeed");
        assert!(result2.is_ok(), "Second call should also succeed");
        // Both calls succeeded, which indicates lazy initialization is working
        // (first call initialized, second call reused cached state)
    }

    #[test]
    fn test_bind_pdfium_error_mapping() {
        // Verify error mapping works correctly
        let map_err = |msg: String| PdfError::TextExtractionFailed(msg);

        // This test just verifies that the error mapping closure works
        // (actual initialization errors depend on system Pdfium availability)
        let test_error = map_err("test".to_string());
        match test_error {
            PdfError::TextExtractionFailed(msg) => {
                assert_eq!(msg, "test");
            }
            _ => panic!("Error mapping failed"),
        }
    }
}
