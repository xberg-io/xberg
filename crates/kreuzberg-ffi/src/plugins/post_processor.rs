//! PostProcessor plugin system FFI bindings
//!
//! Provides FFI functions for registering, managing, and executing custom post-processors
//! from C/Java/other FFI languages.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;
use std::sync::Arc;

use async_trait::async_trait;
use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::plugins::{Plugin, ProcessingStage};
use kreuzberg::types::ExtractionResult;
use kreuzberg::{KreuzbergError, Result};

use crate::helpers::{clear_last_error, set_last_error};
use crate::memory::kreuzberg_free_string;
use crate::{ffi_panic_guard, ffi_panic_guard_bool};

/// Type alias for the PostProcessor callback function.
///
/// # Parameters
///
/// - `result_json`: JSON-encoded ExtractionResult (null-terminated string)
///
/// # Returns
///
/// Null-terminated JSON string containing the processed ExtractionResult
/// (must be freed by Rust via kreuzberg_free_string), or NULL on error.
///
/// # Safety
///
/// The callback must:
/// - Not store the result_json pointer (it's only valid for the duration of the call)
/// - Return a valid null-terminated UTF-8 JSON string allocated by the caller
/// - Return NULL on error (error message should be retrievable separately)
pub type PostProcessorCallback = unsafe extern "C" fn(result_json: *const c_char) -> *mut c_char;

/// FFI wrapper for custom PostProcessors registered from Java/C.
///
/// This struct wraps a C function pointer and implements the PostProcessor trait,
/// allowing custom post-processing implementations from FFI languages to be registered
/// and used within the Rust extraction pipeline.
struct FfiPostProcessor {
    name: String,
    callback: PostProcessorCallback,
    stage: ProcessingStage,
}

impl FfiPostProcessor {
    fn new(name: String, callback: PostProcessorCallback, stage: ProcessingStage) -> Self {
        Self { name, callback, stage }
    }
}

impl Plugin for FfiPostProcessor {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> String {
        "ffi-1.0.0".to_string()
    }

    fn initialize(&self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl kreuzberg::plugins::PostProcessor for FfiPostProcessor {
    async fn process(&self, result: &mut ExtractionResult, _config: &ExtractionConfig) -> Result<()> {
        let result_json = serde_json::to_string(&*result).map_err(|e| KreuzbergError::Validation {
            message: format!("Failed to serialize ExtractionResult: {}", e),
            source: Some(Box::new(e)),
        })?;

        let callback = self.callback;
        let processor_name = self.name.clone();
        let result_json_owned = result_json.clone();

        let processed_json = tokio::task::spawn_blocking(move || {
            let result_cstring = CString::new(result_json_owned).map_err(|e| KreuzbergError::Validation {
                message: format!("Failed to create C string from result JSON: {}", e),
                source: Some(Box::new(e)),
            })?;

            let processed_ptr = unsafe { callback(result_cstring.as_ptr()) };

            if processed_ptr.is_null() {
                return Err(KreuzbergError::Plugin {
                    message: "PostProcessor returned NULL (operation failed)".to_string(),
                    plugin_name: processor_name.clone(),
                });
            }

            let processed_cstr = unsafe { CStr::from_ptr(processed_ptr) };
            let json = processed_cstr
                .to_str()
                .map_err(|e| KreuzbergError::Plugin {
                    message: format!("PostProcessor returned invalid UTF-8: {}", e),
                    plugin_name: processor_name.clone(),
                })?
                .to_string();

            unsafe { kreuzberg_free_string(processed_ptr) };

            Ok(json)
        })
        .await
        .map_err(|e| KreuzbergError::Plugin {
            message: format!("PostProcessor task panicked: {}", e),
            plugin_name: self.name.clone(),
        })??;

        let processed_result: ExtractionResult =
            serde_json::from_str(&processed_json).map_err(|e| KreuzbergError::Plugin {
                message: format!("Failed to deserialize processed result: {}", e),
                plugin_name: self.name.clone(),
            })?;

        *result = processed_result;

        Ok(())
    }

    fn processing_stage(&self) -> kreuzberg::plugins::ProcessingStage {
        self.stage
    }
}

fn parse_processing_stage(stage: Option<&str>) -> std::result::Result<ProcessingStage, String> {
    match stage {
        Some(value) => match value.to_lowercase().as_str() {
            "early" => Ok(ProcessingStage::Early),
            "middle" => Ok(ProcessingStage::Middle),
            "late" => Ok(ProcessingStage::Late),
            other => Err(format!(
                "Invalid processing stage '{}'. Expected one of: early, middle, late",
                other
            )),
        },
        None => Ok(ProcessingStage::Middle),
    }
}

/// Register a custom PostProcessor via FFI callback.
///
/// # Safety
///
/// - `name` must be a valid null-terminated C string
/// - `callback` must be a valid function pointer that:
///   - Does not store the result_json pointer
///   - Returns a null-terminated UTF-8 JSON string or NULL on error
///   - The returned string must be freeable by kreuzberg_free_string
/// - `priority` determines the order of execution (higher priority runs first)
/// - Returns true on success, false on error (check kreuzberg_last_error)
///
/// # Example (C)
///
/// ```c
/// char* my_post_processor(const char* result_json) {
///     // Parse result_json, modify it, return JSON string
///     return strdup("{\"content\":\"PROCESSED\"}");
/// }
///
/// bool success = kreuzberg_register_post_processor("my-processor", my_post_processor, 100);
/// if (!success) {
///     const char* error = kreuzberg_last_error();
///     printf("Failed to register: %s\n", error);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_register_post_processor(
    name: *const c_char,
    callback: PostProcessorCallback,
    priority: i32,
) -> bool {
    ffi_panic_guard_bool!("kreuzberg_register_post_processor", {
        clear_last_error();

        if name.is_null() {
            set_last_error("PostProcessor name cannot be NULL".to_string());
            return false;
        }

        // SAFETY: C callers may pass NULL for function pointer parameters.
        // `black_box` prevents the compiler from optimizing away the null check.
        if core::hint::black_box(callback as usize) == 0 {
            set_last_error("PostProcessor callback cannot be NULL".to_string());
            return false;
        }

        let name_str = match unsafe { CStr::from_ptr(name) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in PostProcessor name: {}", e));
                return false;
            }
        };

        if name_str.is_empty() {
            set_last_error("Plugin name cannot be empty".to_string());
            return false;
        }

        if name_str.chars().any(|c| c.is_whitespace()) {
            set_last_error("Plugin name cannot contain whitespace".to_string());
            return false;
        }

        let processor = Arc::new(FfiPostProcessor::new(
            name_str.to_string(),
            callback,
            ProcessingStage::Middle,
        ));

        let registry = kreuzberg::plugins::registry::get_post_processor_registry();
        let mut registry_guard = registry.write();

        match registry_guard.register(processor, priority) {
            Ok(()) => true,
            Err(e) => {
                set_last_error(format!("Failed to register PostProcessor: {}", e));
                false
            }
        }
    })
}

/// Register a custom PostProcessor with an explicit processing stage.
///
/// # Safety
///
/// - `name` must be a valid null-terminated C string
/// - `stage` must be a valid null-terminated C string containing "early", "middle", or "late"
/// - `callback` must be a valid function pointer that:
///   - Does not store the result_json pointer
///   - Returns a null-terminated UTF-8 JSON string or NULL on error
///   - The returned string must be freeable by kreuzberg_free_string
/// - `priority` determines the order of execution within the stage (higher priority runs first)
/// - Returns true on success, false on error (check kreuzberg_last_error)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_register_post_processor_with_stage(
    name: *const c_char,
    callback: PostProcessorCallback,
    priority: i32,
    stage: *const c_char,
) -> bool {
    ffi_panic_guard_bool!("kreuzberg_register_post_processor_with_stage", {
        clear_last_error();

        if name.is_null() {
            set_last_error("PostProcessor name cannot be NULL".to_string());
            return false;
        }

        // SAFETY: C callers may pass NULL for function pointer parameters.
        // `black_box` prevents the compiler from optimizing away the null check.
        if core::hint::black_box(callback as usize) == 0 {
            set_last_error("PostProcessor callback cannot be NULL".to_string());
            return false;
        }

        let name_str = match unsafe { CStr::from_ptr(name) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in PostProcessor name: {}", e));
                return false;
            }
        };

        if name_str.is_empty() {
            set_last_error("Plugin name cannot be empty".to_string());
            return false;
        }

        if name_str.chars().any(|c| c.is_whitespace()) {
            set_last_error("Plugin name cannot contain whitespace".to_string());
            return false;
        }

        let stage_str = if stage.is_null() {
            None
        } else {
            match unsafe { CStr::from_ptr(stage) }.to_str() {
                Ok(s) => Some(s),
                Err(e) => {
                    set_last_error(format!("Invalid UTF-8 in processing stage: {}", e));
                    return false;
                }
            }
        };

        let stage = match parse_processing_stage(stage_str) {
            Ok(stage) => stage,
            Err(e) => {
                set_last_error(e);
                return false;
            }
        };

        let processor = Arc::new(FfiPostProcessor::new(name_str.to_string(), callback, stage));

        let registry = kreuzberg::plugins::registry::get_post_processor_registry();
        let mut registry_guard = registry.write();

        match registry_guard.register(processor, priority) {
            Ok(()) => true,
            Err(e) => {
                set_last_error(format!("Failed to register PostProcessor: {}", e));
                false
            }
        }
    })
}

/// Unregister a PostProcessor by name.
///
/// # Safety
///
/// - `name` must be a valid null-terminated C string
/// - Returns true on success, false on error (check kreuzberg_last_error)
///
/// # Example (C)
///
/// ```c
/// bool success = kreuzberg_unregister_post_processor("my-processor");
/// if (!success) {
///     const char* error = kreuzberg_last_error();
///     printf("Failed to unregister: %s\n", error);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_unregister_post_processor(name: *const c_char) -> bool {
    ffi_panic_guard_bool!("kreuzberg_unregister_post_processor", {
        clear_last_error();

        if name.is_null() {
            set_last_error("PostProcessor name cannot be NULL".to_string());
            return false;
        }

        let name_str = match unsafe { CStr::from_ptr(name) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in PostProcessor name: {}", e));
                return false;
            }
        };

        let registry = kreuzberg::plugins::registry::get_post_processor_registry();
        let mut registry_guard = registry.write();

        match registry_guard.remove(name_str) {
            Ok(()) => true,
            Err(e) => {
                set_last_error(format!("Failed to remove PostProcessor: {}", e));
                false
            }
        }
    })
}

/// Clear all registered PostProcessors.
///
/// # Safety
///
/// - Removes all registered processors. Subsequent extractions will run without them.
/// - Returns true on success, false on error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_clear_post_processors() -> bool {
    ffi_panic_guard_bool!("kreuzberg_clear_post_processors", {
        clear_last_error();

        let registry = kreuzberg::plugins::registry::get_post_processor_registry();
        let mut registry_guard = registry.write();

        *registry_guard = Default::default();
        true
    })
}

/// List all registered PostProcessors as a JSON array of names.
///
/// # Safety
///
/// - Returned string must be freed with `kreuzberg_free_string`.
/// - Returns NULL on error (check `kreuzberg_last_error`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_list_post_processors() -> *mut c_char {
    ffi_panic_guard!("kreuzberg_list_post_processors", {
        clear_last_error();

        let registry = kreuzberg::plugins::registry::get_post_processor_registry();
        let registry_guard = registry.read();

        match serde_json::to_string(&registry_guard.list()) {
            Ok(json) => match CString::new(json) {
                Ok(cstr) => cstr.into_raw(),
                Err(e) => {
                    set_last_error(format!("Failed to create C string: {}", e));
                    ptr::null_mut()
                }
            },
            Err(e) => {
                set_last_error(format!("Failed to serialize PostProcessor list: {}", e));
                ptr::null_mut()
            }
        }
    })
}
