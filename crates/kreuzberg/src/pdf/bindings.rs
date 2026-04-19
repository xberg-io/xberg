use super::error::PdfError;
use crate::cancellation::CancellationToken;
use pdfium_render::prelude::*;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

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
///
/// CRITICAL: We use `&'static Pdfium` (a leaked reference) instead of `Pdfium` to prevent
/// the instance from being dropped during process exit. Without this, when Rust's runtime
/// cleans up static variables during process teardown, the Pdfium destructor runs and calls
/// `FPDF_DestroyLibrary()`, which can cause segfaults/SIGTRAP (exit code 201 on macOS) in
/// FFI scenarios, especially in Go tests where cgo cleanup happens in a specific order.
static PDFIUM_SINGLETON: OnceLock<Result<&'static Pdfium, String>> = OnceLock::new();

/// Global mutex to serialize all PDFium operations.
///
/// PDFium is NOT thread-safe. While the pdfium-render library provides a safe Rust API,
/// the underlying C library can crash when accessed concurrently from multiple threads.
/// This is especially problematic in batch processing mode where multiple `spawn_blocking`
/// tasks may try to process PDFs simultaneously.
///
/// This mutex ensures that only one thread can be executing PDFium operations at any time.
/// While this serializes PDF processing and eliminates parallelism for PDFs, it prevents
/// crashes and ensures correctness.
///
/// # Performance Impact
///
/// In batch mode, PDFs will be processed sequentially rather than in parallel. However,
/// other document types (text, HTML, etc.) can still be processed in parallel. For
/// workloads with mixed document types, this provides good overall performance.
///
/// # Alternatives Considered
///
/// 1. **Process-based parallelism**: Spawn separate processes for PDF extraction.
///    This would allow true parallelism but adds significant complexity and overhead.
///
/// 2. **Thread-local PDFium instances**: Not possible because the library only allows
///    binding once per process (`FPDF_InitLibrary` can only be called once).
///
/// 3. **Disable batch mode for PDFs**: Would require changes to the batch orchestration
///    to detect PDF types and process them differently.
static PDFIUM_OPERATION_LOCK: Mutex<()> = Mutex::new(());

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
/// 3. Creates and leaks the `Pdfium` instance to prevent cleanup during process exit
///
/// This is only called once, on first access to the singleton.
///
/// CRITICAL: We intentionally leak the Pdfium instance using `Box::leak()` to prevent
/// it from being dropped during process exit. If the instance were dropped, it would call
/// `FPDF_DestroyLibrary()` which causes segfaults/SIGTRAP in FFI scenarios (exit code 201
/// on macOS), particularly visible in Go tests where cgo cleanup order matters.
fn initialize_pdfium() -> Result<&'static Pdfium, String> {
    // Step 1: Extract bundled library (if applicable)
    let lib_dir = extract_and_get_lib_dir()?;

    // Step 2: Create bindings to the library
    let bindings = create_pdfium_bindings(&lib_dir)?;

    // Step 3: Create Pdfium instance (this calls FPDF_InitLibrary)
    let pdfium = Pdfium::new(bindings);

    // Step 4: Leak the instance to prevent Drop from being called during process exit
    // This is intentional and necessary for FFI safety across language boundaries
    Ok(Box::leak(Box::new(pdfium)))
}

/// A handle to the global Pdfium instance with exclusive access.
///
/// This wrapper provides access to the singleton `Pdfium` instance. It implements
/// `Deref<Target = Pdfium>` so it can be used anywhere a `&Pdfium` is expected.
///
/// # Design
///
/// The handle holds an exclusive lock on PDFium operations via `PDFIUM_OPERATION_LOCK`.
/// When the handle is dropped, the lock is released, allowing other threads to
/// acquire PDFium access.
///
/// This design ensures:
/// - The Pdfium library is initialized exactly once
/// - The library is never destroyed during the process lifetime
/// - Only one thread can access PDFium at a time (thread safety)
/// - The lock is automatically released when the handle goes out of scope
///
/// # Thread Safety
///
/// PDFium is NOT thread-safe, so this handle serializes all PDFium operations.
/// While this prevents parallel PDF processing, it ensures correctness and
/// prevents crashes in batch processing scenarios.
pub(crate) struct PdfiumHandle<'a> {
    // Hold the mutex guard to ensure exclusive access to PDFium.
    // The guard is automatically released when PdfiumHandle is dropped.
    #[allow(dead_code)]
    _guard: MutexGuard<'a, ()>,
}

impl Deref for PdfiumHandle<'_> {
    type Target = Pdfium;

    fn deref(&self) -> &Self::Target {
        // SAFETY: We only create PdfiumHandle after successfully initializing
        // the singleton, so this unwrap is guaranteed to succeed.
        // The Result inside is also guaranteed to be Ok because bind_pdfium()
        // only returns PdfiumHandle on success.
        // Since we now store &'static Pdfium, we can directly dereference it.
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
/// * `cancel_token` - Optional cancellation token. When provided, the function will
///   return `PdfError::Cancelled` if the token is cancelled while waiting for the lock.
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
/// When a `cancel_token` is provided, the lock is acquired via a spin-sleep loop (10ms
/// sleep between attempts) rather than a blocking `lock()`, so that cancellation can be
/// observed without waiting indefinitely.
///
/// # Thread Safety
///
/// This function is thread-safe but SERIALIZES access to PDFium:
/// - The `OnceLock` ensures initialization happens exactly once
/// - The `PDFIUM_OPERATION_LOCK` mutex ensures only one thread can access PDFium at a time
/// - The returned `PdfiumHandle` holds the mutex guard; when dropped, the lock is released
///
/// This serialization is necessary because PDFium is NOT thread-safe. Concurrent access
/// to PDFium from multiple threads causes crashes (segfaults, abort traps).
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
/// let pdfium = bind_pdfium(PdfError::TextExtractionFailed, "text extraction", None)?;
///
/// // Use it like a &Pdfium
/// let document = pdfium.load_pdf_from_byte_slice(bytes, None)?;
///
/// // With cancellation support
/// let pdfium2 = bind_pdfium(PdfError::RenderingFailed, "page rendering", cancel_token.as_ref())?;
/// ```
pub(crate) fn bind_pdfium(
    map_err: fn(String) -> PdfError,
    context: &'static str,
    cancel_token: Option<&CancellationToken>,
) -> Result<PdfiumHandle<'static>, PdfError> {
    // Acquire exclusive lock on PDFium operations.
    // This prevents concurrent access to PDFium which is NOT thread-safe.
    // The lock is held for the duration of the PdfiumHandle's lifetime.
    //
    // When a cancellation token is provided we spin with try_lock so we can
    // observe cancellation while waiting.  When there is no token the simpler
    // blocking lock() path is used to avoid the spin overhead.
    let guard = if let Some(token) = cancel_token {
        loop {
            if token.is_cancelled() {
                return Err(PdfError::Cancelled);
            }
            match PDFIUM_OPERATION_LOCK.try_lock() {
                Ok(g) => break g,
                Err(std::sync::TryLockError::WouldBlock) => {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(std::sync::TryLockError::Poisoned(e)) => {
                    return Err(map_err(format!("PDFium operation lock poisoned ({}): {}", context, e)));
                }
            }
        }
    } else {
        PDFIUM_OPERATION_LOCK
            .lock()
            .map_err(|e| map_err(format!("PDFium operation lock poisoned ({}): {}", context, e)))?
    };

    // Initialize the singleton on first access, or get the cached result
    let result = PDFIUM_SINGLETON.get_or_init(initialize_pdfium);

    // Convert the cached Result into our return type
    match result {
        Ok(_) => Ok(PdfiumHandle { _guard: guard }),
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
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_bind_pdfium_lazy_initialization() {
        let result = bind_pdfium(PdfError::TextExtractionFailed, "test context", None);
        assert!(result.is_ok(), "First bind_pdfium call should succeed");
    }

    #[test]
    #[serial]
    fn test_bind_pdfium_multiple_calls() {
        // First call - acquire lock, test success, then drop handle to release lock
        {
            let result1 = bind_pdfium(PdfError::TextExtractionFailed, "test 1", None);
            assert!(result1.is_ok(), "First call should succeed");
        } // result1 dropped here, releasing the lock

        // Second call - can now acquire lock since first handle was dropped
        {
            let result2 = bind_pdfium(PdfError::TextExtractionFailed, "test 2", None);
            assert!(result2.is_ok(), "Second call should also succeed");
        }
    }

    #[test]
    #[serial]
    fn test_bind_pdfium_returns_same_instance() {
        // Get pointer from first handle, then drop it to release lock
        let ptr1 = {
            let handle1 = bind_pdfium(PdfError::TextExtractionFailed, "test 1", None).unwrap();
            &*handle1 as *const Pdfium
        }; // handle1 dropped here, releasing the lock

        // Get pointer from second handle
        let ptr2 = {
            let handle2 = bind_pdfium(PdfError::TextExtractionFailed, "test 2", None).unwrap();
            &*handle2 as *const Pdfium
        };

        // Both handles should dereference to the same Pdfium instance
        assert_eq!(ptr1, ptr2, "Both handles should reference the same Pdfium instance");
    }

    #[test]
    #[serial]
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
    #[serial]
    fn test_pdfium_handle_deref() {
        let handle = bind_pdfium(PdfError::TextExtractionFailed, "test", None).unwrap();

        // Test that we can use the handle like a &Pdfium by calling a method
        // that requires &Pdfium. create_new_pdf() takes &self and returns a Result.
        let result = handle.create_new_pdf();
        assert!(result.is_ok(), "Should be able to create a new PDF document");
    }
}
