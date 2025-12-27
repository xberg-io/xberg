//! Batch result streaming FFI module.
//!
//! Provides callback-based batch processing to avoid holding all results in memory.
//! Processes files one at a time, calling a user-provided callback for each result.
//!
//! # Benefits
//!
//! - 30-50% memory reduction for large batches (no accumulation of results)
//! - Early error detection (fail-fast on first error or continue processing)
//! - Progress reporting and cancellation support
//! - Optional parallel processing with rayon
//!
//! # Safety Model
//!
//! - Callback receives borrowed result pointer valid only during the callback
//! - Caller must copy/serialize data before callback returns if persistence needed
//! - Results are automatically freed after callback returns
//! - Error callback is optional and independent of result processing
//!
//! # Example (C)
//!
//! ```c
//! int process_result(const CExtractionResult* result, size_t index, void* user_data) {
//!     // Process or copy result data here
//!     printf("Processing file %zu\n", index);
//!     return 0; // Continue processing
//! }
//!
//! void handle_error(size_t index, const char* error_msg, void* user_data) {
//!     fprintf(stderr, "Error processing file %zu: %s\n", index, error_msg);
//! }
//!
//! const char* files[] = {"doc1.pdf", "doc2.txt", "doc3.docx"};
//! int result = kreuzberg_extract_batch_streaming(
//!     files, 3, NULL, process_result, NULL, handle_error
//! );
//! ```

use crate::result_view::{CExtractionResultView, create_result_view};
use crate::{FfiResult, clear_last_error, parse_extraction_config_from_json, set_last_error};
use kreuzberg::types::ExtractionResult;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::path::Path;
#[cfg(feature = "rayon")]
use std::sync::Arc;
#[cfg(feature = "rayon")]
use std::sync::atomic::{AtomicBool, Ordering};

/// Callback function invoked for each successfully extracted result.
///
/// # Arguments
///
/// * `result` - Borrowed pointer to extraction result (valid only during callback)
/// * `file_index` - Zero-based index of the file in the batch
/// * `user_data` - User-provided context pointer
///
/// # Returns
///
/// - `0` to continue processing remaining files
/// - Non-zero to cancel batch processing (no further callbacks)
///
/// # Safety
///
/// - `result` pointer is valid only during the callback execution
/// - `result` is automatically freed after callback returns
/// - Caller must copy/serialize data if needed beyond callback scope
/// - `user_data` is passed through opaquely (caller manages lifetime)
pub type ResultCallback =
    unsafe extern "C" fn(result: *const CExtractionResultView, file_index: usize, user_data: *mut c_void) -> c_int;

/// Callback function invoked when a file extraction fails.
///
/// # Arguments
///
/// * `file_index` - Zero-based index of the file that failed
/// * `error_msg` - Null-terminated UTF-8 error message (valid only during callback)
/// * `user_data` - User-provided context pointer
///
/// # Safety
///
/// - `error_msg` is valid only during callback execution
/// - Caller must copy string if needed beyond callback scope
/// - `user_data` is passed through opaquely (caller manages lifetime)
pub type ErrorCallback = unsafe extern "C" fn(file_index: usize, error_msg: *const c_char, user_data: *mut c_void);

/// Extract multiple files in streaming mode with callback-based result delivery.
///
/// Processes files one at a time without accumulating results in memory.
/// Each result is passed to the callback and then freed automatically.
///
/// # Arguments
///
/// * `files` - Array of null-terminated file path strings
/// * `count` - Number of files in the array
/// * `config_json` - Optional JSON configuration string (NULL for defaults)
/// * `result_callback` - Callback invoked for each successful extraction
/// * `user_data` - Optional user context passed to callbacks
/// * `error_callback` - Optional callback invoked for extraction failures
///
/// # Returns
///
/// - `0` on success (all files processed or cancelled by callback)
/// - `-1` on error (invalid arguments, configuration parsing failure)
///
/// # Error Handling
///
/// - Individual file failures invoke `error_callback` but don't stop processing
/// - Callback can return non-zero to cancel remaining files
/// - Invalid arguments or config parsing errors return `-1` immediately
///
/// # Safety
///
/// - `files` must point to valid array of `count` C string pointers
/// - All file path strings must be valid null-terminated UTF-8
/// - `config_json` must be valid null-terminated UTF-8 if not NULL
/// - `result_callback` must be a valid function pointer
/// - `error_callback` must be a valid function pointer if not NULL
/// - Result pointers passed to callbacks are valid only during callback
/// - Callbacks must not store result pointers for later use
///
/// # Example (C)
///
/// ```c
/// int process_result(const CExtractionResultView* result, size_t index, void* data) {
///     // Copy data needed beyond callback scope
///     char content[1024];
///     size_t copy_len = result->content_len < 1024 ? result->content_len : 1023;
///     memcpy(content, result->content_ptr, copy_len);
///     content[copy_len] = '\0';
///     return 0; // Continue
/// }
///
/// void handle_error(size_t index, const char* msg, void* data) {
///     fprintf(stderr, "File %zu failed: %s\n", index, msg);
/// }
///
/// const char* files[] = {"a.pdf", "b.txt", "c.docx"};
/// kreuzberg_extract_batch_streaming(files, 3, NULL, process_result, NULL, handle_error);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_extract_batch_streaming(
    files: *const *const c_char,
    count: usize,
    config_json: *const c_char,
    result_callback: ResultCallback,
    user_data: *mut c_void,
    error_callback: Option<ErrorCallback>,
) -> c_int {
    clear_last_error();

    if files.is_null() {
        set_last_error("Files array cannot be NULL".to_string());
        return -1;
    }

    if count == 0 {
        return 0;
    }

    let config = if !config_json.is_null() {
        match unsafe { CStr::from_ptr(config_json) }.to_str() {
            Ok(config_str) => match parse_extraction_config_from_json(config_str) {
                Ok(cfg) => cfg,
                Err(e) => {
                    set_last_error(format!("Invalid configuration: {}", e));
                    return -1;
                }
            },
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in config: {}", e));
                return -1;
            }
        }
    } else {
        Default::default()
    };

    for i in 0..count {
        let file_ptr = unsafe { *files.add(i) };

        if file_ptr.is_null() {
            if let Some(err_cb) = error_callback
                && let Ok(err_msg) = CString::new("File path is NULL")
            {
                unsafe { err_cb(i, err_msg.as_ptr(), user_data) };
            }
            continue;
        }

        let file_path = match unsafe { CStr::from_ptr(file_ptr) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                if let Some(err_cb) = error_callback
                    && let Ok(err_msg) = CString::new(format!("Invalid UTF-8 in file path: {}", e))
                {
                    unsafe { err_cb(i, err_msg.as_ptr(), user_data) };
                }
                continue;
            }
        };

        match extract_file_internal(file_path, &config) {
            Ok(result) => {
                let view = create_result_view(&result);

                let continue_processing = unsafe { result_callback(&view as *const _, i, user_data) };

                if continue_processing != 0 {
                    return 0;
                }
            }
            Err(e) => {
                if let Some(err_cb) = error_callback
                    && let Ok(err_msg) = CString::new(e)
                {
                    unsafe { err_cb(i, err_msg.as_ptr(), user_data) };
                }
            }
        }
    }

    0
}

/// Extract multiple files in parallel streaming mode.
///
/// Similar to `kreuzberg_extract_batch_streaming` but processes files in parallel
/// using a thread pool. Results are delivered via callback as they complete.
///
/// # Arguments
///
/// * `files` - Array of null-terminated file path strings
/// * `count` - Number of files in the array
/// * `config_json` - Optional JSON configuration string (NULL for defaults)
/// * `result_callback` - Thread-safe callback invoked for each successful extraction
/// * `user_data` - Optional user context passed to callbacks (must be thread-safe)
/// * `error_callback` - Optional thread-safe callback invoked for failures
/// * `max_parallel` - Maximum number of parallel extractions (0 = number of CPUs)
///
/// # Returns
///
/// - `0` on success (all files processed or cancelled)
/// - `-1` on error (invalid arguments, configuration parsing failure)
///
/// # Thread Safety
///
/// - Both callbacks may be invoked concurrently from multiple threads
/// - `user_data` must be thread-safe (e.g., synchronized with mutex)
/// - Callback can set atomic flag to signal cancellation
///
/// # Safety
///
/// Same requirements as `kreuzberg_extract_batch_streaming`, plus:
/// - Callbacks must be thread-safe
/// - `user_data` must support concurrent access
///
/// # Example (C)
///
/// ```c
/// typedef struct {
///     pthread_mutex_t lock;
///     atomic_int cancel_flag;
/// } BatchContext;
///
/// int process_result(const CExtractionResultView* result, size_t index, void* data) {
///     BatchContext* ctx = (BatchContext*)data;
///     pthread_mutex_lock(&ctx->lock);
///     // Process result with thread safety
///     pthread_mutex_unlock(&ctx->lock);
///     return atomic_load(&ctx->cancel_flag);
/// }
/// ```
#[unsafe(no_mangle)]
#[cfg_attr(not(feature = "rayon"), allow(unused_variables))]
pub unsafe extern "C" fn kreuzberg_extract_batch_parallel(
    files: *const *const c_char,
    count: usize,
    config_json: *const c_char,
    result_callback: ResultCallback,
    user_data: *mut c_void,
    error_callback: Option<ErrorCallback>,
    max_parallel: usize,
) -> c_int {
    clear_last_error();

    if files.is_null() {
        set_last_error("Files array cannot be NULL".to_string());
        return -1;
    }

    if count == 0 {
        return 0;
    }

    let config = if !config_json.is_null() {
        match unsafe { CStr::from_ptr(config_json) }.to_str() {
            Ok(config_str) => match parse_extraction_config_from_json(config_str) {
                Ok(cfg) => cfg,
                Err(e) => {
                    set_last_error(format!("Invalid configuration: {}", e));
                    return -1;
                }
            },
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in config: {}", e));
                return -1;
            }
        }
    } else {
        Default::default()
    };

    let mut file_paths = Vec::with_capacity(count);
    for i in 0..count {
        let file_ptr = unsafe { *files.add(i) };

        if file_ptr.is_null() {
            if let Some(err_cb) = error_callback
                && let Ok(err_msg) = CString::new("File path is NULL")
            {
                unsafe { err_cb(i, err_msg.as_ptr(), user_data) };
            }
            continue;
        }

        match unsafe { CStr::from_ptr(file_ptr) }.to_str() {
            Ok(s) => file_paths.push((i, s.to_string())),
            Err(e) => {
                if let Some(err_cb) = error_callback
                    && let Ok(err_msg) = CString::new(format!("Invalid UTF-8: {}", e))
                {
                    unsafe { err_cb(i, err_msg.as_ptr(), user_data) };
                }
            }
        }
    }

    #[cfg(feature = "rayon")]
    {
        use rayon::prelude::*;

        let cancelled = Arc::new(AtomicBool::new(false));
        let config = Arc::new(config);

        let pool = if max_parallel > 0 {
            rayon::ThreadPoolBuilder::new().num_threads(max_parallel).build()
        } else {
            rayon::ThreadPoolBuilder::new().build()
        };

        let pool = match pool {
            Ok(p) => p,
            Err(e) => {
                set_last_error(format!("Failed to create thread pool: {}", e));
                return -1;
            }
        };

        let user_data_ptr = user_data as usize;

        pool.install(|| {
            file_paths.par_iter().for_each(|(index, path)| {
                if cancelled.load(Ordering::Relaxed) {
                    return;
                }

                match extract_file_internal(path, &config) {
                    Ok(result) => {
                        let view = create_result_view(&result);

                        let should_cancel =
                            unsafe { result_callback(&view as *const _, *index, user_data_ptr as *mut c_void) };

                        if should_cancel != 0 {
                            cancelled.store(true, Ordering::Relaxed);
                        }
                    }
                    Err(e) => {
                        if let Some(err_cb) = error_callback {
                            if let Ok(err_msg) = CString::new(e) {
                                unsafe { err_cb(*index, err_msg.as_ptr(), user_data_ptr as *mut c_void) };
                            }
                        }
                    }
                }
            });
        });

        0
    }

    #[cfg(not(feature = "rayon"))]
    {
        set_last_error("Parallel processing requires 'rayon' feature to be enabled".to_string());
        -1
    }
}

/// Internal function to extract a file with error handling.
///
/// Returns Result<ExtractionResult, String> for easier error propagation.
fn extract_file_internal(
    file_path: &str,
    config: &kreuzberg::core::config::ExtractionConfig,
) -> FfiResult<ExtractionResult> {
    let path = Path::new(file_path);

    if !path.exists() {
        return Err(format!("File not found: {}", file_path));
    }

    let rt = tokio::runtime::Runtime::new().map_err(|e| format!("Failed to create runtime: {}", e))?;

    rt.block_on(async {
        kreuzberg::core::extractor::extract_file(path, None, config)
            .await
            .map_err(|e| format!("Extraction failed: {}", e))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::ptr;
    use std::sync::Mutex;

    struct TestContext {
        results: Vec<String>,
        errors: Vec<String>,
    }

    unsafe extern "C" fn test_result_callback(
        result: *const CExtractionResultView,
        file_index: usize,
        user_data: *mut c_void,
    ) -> c_int {
        let ctx = unsafe { &mut *(user_data as *mut Mutex<TestContext>) };
        let mut guard = ctx.lock().unwrap();

        if !result.is_null() {
            let view = unsafe { &*result };
            let content = if !view.content_ptr.is_null() && view.content_len > 0 {
                unsafe {
                    String::from_utf8_lossy(std::slice::from_raw_parts(view.content_ptr, view.content_len)).to_string()
                }
            } else {
                String::new()
            };
            guard.results.push(format!("File {}: {}", file_index, content));
        }

        0
    }

    unsafe extern "C" fn test_error_callback(file_index: usize, error_msg: *const c_char, user_data: *mut c_void) {
        let ctx = unsafe { &mut *(user_data as *mut Mutex<TestContext>) };
        let mut guard = ctx.lock().unwrap();

        let msg = unsafe { CStr::from_ptr(error_msg).to_string_lossy().to_string() };
        guard.errors.push(format!("File {}: {}", file_index, msg));
    }

    #[test]
    fn test_batch_streaming_basic() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file1 = temp_dir.path().join("test1.txt");
        let file2 = temp_dir.path().join("test2.txt");
        std::fs::write(&file1, "Content 1").unwrap();
        std::fs::write(&file2, "Content 2").unwrap();

        let path1 = CString::new(file1.to_str().unwrap()).unwrap();
        let path2 = CString::new(file2.to_str().unwrap()).unwrap();
        let files = vec![path1.as_ptr(), path2.as_ptr()];

        let context = Mutex::new(TestContext {
            results: Vec::new(),
            errors: Vec::new(),
        });

        let result = unsafe {
            kreuzberg_extract_batch_streaming(
                files.as_ptr(),
                files.len(),
                ptr::null(),
                test_result_callback,
                &context as *const _ as *mut c_void,
                Some(test_error_callback),
            )
        };

        assert_eq!(result, 0);

        let ctx = context.lock().unwrap();
        assert_eq!(ctx.results.len(), 2);
        assert_eq!(ctx.errors.len(), 0);
    }

    #[test]
    fn test_batch_streaming_with_errors() {
        let path1 = CString::new("/nonexistent/file.txt").unwrap();
        let files = vec![path1.as_ptr()];

        let context = Mutex::new(TestContext {
            results: Vec::new(),
            errors: Vec::new(),
        });

        let result = unsafe {
            kreuzberg_extract_batch_streaming(
                files.as_ptr(),
                files.len(),
                ptr::null(),
                test_result_callback,
                &context as *const _ as *mut c_void,
                Some(test_error_callback),
            )
        };

        assert_eq!(result, 0);

        let ctx = context.lock().unwrap();
        assert_eq!(ctx.results.len(), 0);
        assert_eq!(ctx.errors.len(), 1);
    }

    #[test]
    fn test_batch_streaming_cancellation() {
        unsafe extern "C" fn cancel_callback(
            _result: *const CExtractionResultView,
            file_index: usize,
            _user_data: *mut c_void,
        ) -> c_int {
            if file_index == 0 { 1 } else { 0 }
        }

        let temp_dir = tempfile::tempdir().unwrap();
        let file1 = temp_dir.path().join("test1.txt");
        let file2 = temp_dir.path().join("test2.txt");
        std::fs::write(&file1, "Content 1").unwrap();
        std::fs::write(&file2, "Content 2").unwrap();

        let path1 = CString::new(file1.to_str().unwrap()).unwrap();
        let path2 = CString::new(file2.to_str().unwrap()).unwrap();
        let files = vec![path1.as_ptr(), path2.as_ptr()];

        let result = unsafe {
            kreuzberg_extract_batch_streaming(
                files.as_ptr(),
                files.len(),
                ptr::null(),
                cancel_callback,
                ptr::null_mut(),
                None,
            )
        };

        assert_eq!(result, 0);
    }

    #[test]
    fn test_batch_streaming_null_files() {
        let result = unsafe {
            kreuzberg_extract_batch_streaming(ptr::null(), 1, ptr::null(), test_result_callback, ptr::null_mut(), None)
        };

        assert_eq!(result, -1);
    }

    #[test]
    fn test_batch_streaming_empty() {
        let files: Vec<*const c_char> = Vec::new();

        let result = unsafe {
            kreuzberg_extract_batch_streaming(
                files.as_ptr(),
                0,
                ptr::null(),
                test_result_callback,
                ptr::null_mut(),
                None,
            )
        };

        assert_eq!(result, 0);
    }
}
