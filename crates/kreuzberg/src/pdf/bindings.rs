use super::error::PdfError;
use pdfium_render::prelude::*;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::OnceLock;

/// Global singleton for the Pdfium instance.
///
/// The pdfium-render library only allows binding to the Pdfium library ONCE per process.
/// Subsequent calls to `Pdfium::bind_to_library()` or `Pdfium::bind_to_system_library()`
/// will fail with a library loading error because the dynamic library is already loaded.
///
/// Additionally, `Pdfium::new()` calls `FPDF_InitLibrary()` which must only be called once,
/// and when `Pdfium` is dropped, it calls `FPDF_DestroyLibrary()` which would invalidate
/// all subsequent PDF operations.
///
/// This singleton ensures:
/// 1. Library binding happens exactly once (on first access)
/// 2. `FPDF_InitLibrary()` is called exactly once
/// 3. The `Pdfium` instance is never dropped, so `FPDF_DestroyLibrary()` is never called
/// 4. All callers share the same `Pdfium` instance safely
static PDFIUM_SINGLETON: OnceLock<Result<Pdfium, String>> = OnceLock::new();

/// Extract the bundled pdfium library and return its directory path.
///
/// This is only called on first initialization when `bundled-pdfium` feature is enabled.
fn extract_and_get_lib_dir() -> Result<Option<PathBuf>, String> {
    #[cfg(all(feature = "pdf", feature = "bundled-pdfium", not(target_arch = "wasm32")))]
    {
        let lib_path =
            crate::pdf::extract_bundled_pdfium().map_err(|e| format!("Failed to extract bundled Pdfium: {}", e))?;

        let lib_dir = lib_path.parent().ok_or_else(|| {
            format!(
                "Failed to determine Pdfium extraction directory for '{}'",
                lib_path.display()
            )
        })?;

        Ok(Some(lib_dir.to_path_buf()))
    }

    #[cfg(any(not(feature = "bundled-pdfium"), target_arch = "wasm32"))]
    {
        Ok(None)
    }
}

/// Bind to the Pdfium library and create bindings.
///
/// This function is only called once during singleton initialization.
fn create_pdfium_bindings(lib_dir: &Option<PathBuf>) -> Result<Box<dyn PdfiumLibraryBindings>, String> {
    let _ = lib_dir;

    #[cfg(all(feature = "pdf", feature = "bundled-pdfium", not(target_arch = "wasm32")))]
    {
        if let Some(dir) = lib_dir {
            return Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(dir))
                .map_err(|e| format!("Failed to bind to Pdfium library: {}", e));
        }
    }

    // For system library or WASM
    Pdfium::bind_to_system_library().map_err(|e| format!("Failed to bind to system Pdfium library: {}", e))
}

/// Initialize the Pdfium singleton.
///
/// This function performs the one-time initialization:
/// 1. Extracts bundled library if using `bundled-pdfium` feature
/// 2. Creates bindings to the Pdfium library
/// 3. Creates and returns the `Pdfium` instance
///
/// This is only called once, on first access to the singleton.
fn initialize_pdfium() -> Result<Pdfium, String> {
    // Step 1: Extract bundled library (if applicable)
    let lib_dir = extract_and_get_lib_dir()?;

    // Step 2: Create bindings to the library
    let bindings = create_pdfium_bindings(&lib_dir)?;

    // Step 3: Create Pdfium instance (this calls FPDF_InitLibrary)
    Ok(Pdfium::new(bindings))
}

/// A handle to the global Pdfium instance.
///
/// This wrapper provides access to the singleton `Pdfium` instance. It implements
/// `Deref<Target = Pdfium>` so it can be used anywhere a `&Pdfium` is expected.
///
/// # Design
///
/// The handle does not own the `Pdfium` instance - it merely provides access to
/// the global singleton. When a `PdfiumHandle` is dropped, the underlying `Pdfium`
/// instance continues to exist and can be accessed by future calls to `bind_pdfium()`.
///
/// This design ensures:
/// - The Pdfium library is initialized exactly once
/// - The library is never destroyed during the process lifetime
/// - Multiple callers can safely use Pdfium concurrently (via `&Pdfium`)
pub(crate) struct PdfiumHandle {
    // This is a zero-sized marker type. The actual Pdfium instance
    // is accessed via the PDFIUM_SINGLETON static.
    _private: (),
}

impl Deref for PdfiumHandle {
    type Target = Pdfium;

    fn deref(&self) -> &Self::Target {
        // SAFETY: We only create PdfiumHandle after successfully initializing
        // the singleton, so this unwrap is guaranteed to succeed.
        // The Result inside is also guaranteed to be Ok because bind_pdfium()
        // only returns PdfiumHandle on success.
        PDFIUM_SINGLETON.get().unwrap().as_ref().unwrap()
    }
}

/// Get a handle to the Pdfium library with lazy initialization.
///
/// The first call to this function triggers initialization of the global Pdfium singleton.
/// This includes:
/// - Extracting the bundled Pdfium library (if using `bundled-pdfium` feature)
/// - Loading and binding to the Pdfium dynamic library
/// - Calling `FPDF_InitLibrary()` to initialize the library
///
/// Subsequent calls return immediately with a handle to the same singleton instance.
///
/// # Arguments
///
/// * `map_err` - Function to convert error strings into `PdfError` variants
/// * `context` - Context string for error messages (e.g., "text extraction")
///
/// # Returns
///
/// A `PdfiumHandle` that provides access to the global `Pdfium` instance via `Deref`.
/// The handle can be used anywhere a `&Pdfium` reference is expected.
///
/// # Performance
///
/// - **First call**: Performs full initialization (~8-12ms for bundled extraction + binding)
/// - **Subsequent calls**: Returns immediately (just fetches from `OnceLock`, ~nanoseconds)
///
/// This lazy initialization defers Pdfium setup until the first PDF is processed,
/// improving cold start time for non-PDF workloads.
///
/// # Thread Safety
///
/// This function is thread-safe. Multiple threads can call `bind_pdfium()` concurrently:
/// - The `OnceLock` ensures initialization happens exactly once
/// - All threads receive handles to the same singleton instance
/// - The underlying `Pdfium` instance is safe for concurrent `&self` access
///
/// # Error Handling
///
/// If initialization fails (e.g., library not found, extraction failed), the error
/// is cached and returned on all subsequent calls. The process cannot recover from
/// a failed initialization - restart the process to retry.
///
/// # Example
///
/// ```ignore
/// // First call initializes the singleton
/// let pdfium = bind_pdfium(PdfError::TextExtractionFailed, "text extraction")?;
///
/// // Use it like a &Pdfium
/// let document = pdfium.load_pdf_from_byte_slice(bytes, None)?;
///
/// // Subsequent calls return immediately
/// let pdfium2 = bind_pdfium(PdfError::RenderingFailed, "page rendering")?;
/// // pdfium and pdfium2 reference the same underlying instance
/// ```
pub(crate) fn bind_pdfium(map_err: fn(String) -> PdfError, context: &'static str) -> Result<PdfiumHandle, PdfError> {
    // Initialize the singleton on first access, or get the cached result
    let result = PDFIUM_SINGLETON.get_or_init(initialize_pdfium);

    // Convert the cached Result into our return type
    match result {
        Ok(_) => Ok(PdfiumHandle { _private: () }),
        Err(cached_error) => Err(map_err(format!(
            "Pdfium initialization failed ({}): {}",
            context, cached_error
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::error::PdfError;

    #[test]
    fn test_bind_pdfium_lazy_initialization() {
        let result = bind_pdfium(PdfError::TextExtractionFailed, "test context");
        assert!(result.is_ok(), "First bind_pdfium call should succeed");
    }

    #[test]
    fn test_bind_pdfium_multiple_calls() {
        let result1 = bind_pdfium(PdfError::TextExtractionFailed, "test 1");
        let result2 = bind_pdfium(PdfError::TextExtractionFailed, "test 2");

        assert!(result1.is_ok(), "First call should succeed");
        assert!(result2.is_ok(), "Second call should also succeed");
    }

    #[test]
    fn test_bind_pdfium_returns_same_instance() {
        let handle1 = bind_pdfium(PdfError::TextExtractionFailed, "test 1").unwrap();
        let handle2 = bind_pdfium(PdfError::TextExtractionFailed, "test 2").unwrap();

        // Both handles should dereference to the same Pdfium instance
        let ptr1 = &*handle1 as *const Pdfium;
        let ptr2 = &*handle2 as *const Pdfium;
        assert_eq!(ptr1, ptr2, "Both handles should reference the same Pdfium instance");
    }

    #[test]
    fn test_bind_pdfium_error_mapping() {
        let map_err = |msg: String| PdfError::TextExtractionFailed(msg);

        let test_error = map_err("test".to_string());
        match test_error {
            PdfError::TextExtractionFailed(msg) => {
                assert_eq!(msg, "test");
            }
            _ => panic!("Error mapping failed"),
        }
    }

    #[test]
    fn test_pdfium_handle_deref() {
        let handle = bind_pdfium(PdfError::TextExtractionFailed, "test").unwrap();

        // Test that we can use the handle like a &Pdfium by calling a method
        // that requires &Pdfium. create_new_pdf() takes &self and returns a Result.
        let result = handle.create_new_pdf();
        assert!(result.is_ok(), "Should be able to create a new PDF document");
    }
}
