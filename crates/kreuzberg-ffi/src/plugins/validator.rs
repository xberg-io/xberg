//! Validator plugin FFI bindings
//!
//! Provides FFI bindings for registering and managing custom validators.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;
use std::sync::Arc;

use async_trait::async_trait;
use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::plugins::Plugin;
use kreuzberg::types::ExtractionResult;
use kreuzberg::{KreuzbergError, Result};

use crate::helpers::{clear_last_error, set_last_error};
use crate::{ffi_panic_guard, ffi_panic_guard_bool};

/// Validator callback function type for FFI.
///
/// This is a C function pointer that validates extraction results.
///
/// # Safety
///
/// The callback must:
/// - Not store the result_json pointer (it's only valid for the duration of the call)
/// - Return a valid null-terminated UTF-8 string (error message) if validation fails
/// - Return NULL if validation passes
/// - The returned string must be freeable by kreuzberg_free_string
type ValidatorCallback = unsafe extern "C" fn(result_json: *const c_char) -> *mut c_char;

/// FFI wrapper for custom Validators registered from Java/C.
///
/// This struct wraps a C function pointer and implements the Validator trait,
/// allowing custom validation implementations from FFI languages to be registered
/// and used within the Rust extraction pipeline.
struct FfiValidator {
    name: String,
    callback: ValidatorCallback,
    priority: i32,
}

impl FfiValidator {
    fn new(name: String, callback: ValidatorCallback, priority: i32) -> Self {
        Self {
            name,
            callback,
            priority,
        }
    }
}

impl Plugin for FfiValidator {
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
impl kreuzberg::plugins::Validator for FfiValidator {
    fn priority(&self) -> i32 {
        self.priority
    }

    async fn validate(&self, result: &ExtractionResult, _config: &ExtractionConfig) -> Result<()> {
        let result_json = serde_json::to_string(result).map_err(|e| KreuzbergError::Validation {
            message: format!("Failed to serialize ExtractionResult: {}", e),
            source: Some(Box::new(e)),
        })?;

        let callback = self.callback;
        let validator_name = self.name.clone();
        let result_json_owned = result_json.clone();

        let error_msg = tokio::task::spawn_blocking(move || {
            let result_cstring = CString::new(result_json_owned).map_err(|e| KreuzbergError::Validation {
                message: format!("Failed to create C string from result JSON: {}", e),
                source: Some(Box::new(e)),
            })?;

            let error_ptr = unsafe { callback(result_cstring.as_ptr()) };

            if error_ptr.is_null() {
                return Ok::<Option<String>, KreuzbergError>(None);
            }

            let error_cstr = unsafe { CStr::from_ptr(error_ptr) };
            let error_msg = error_cstr
                .to_str()
                .map_err(|e| KreuzbergError::Plugin {
                    message: format!("Validator returned invalid UTF-8: {}", e),
                    plugin_name: validator_name.clone(),
                })?
                .to_string();

            unsafe { crate::memory::kreuzberg_free_string(error_ptr) };

            Ok(Some(error_msg))
        })
        .await
        .map_err(|e| KreuzbergError::Plugin {
            message: format!("Validator task panicked: {}", e),
            plugin_name: self.name.clone(),
        })??;

        if let Some(msg) = error_msg {
            return Err(KreuzbergError::Validation {
                message: msg,
                source: None,
            });
        }

        Ok(())
    }
}

/// Register a custom Validator via FFI callback.
///
/// # Safety
///
/// - `name` must be a valid null-terminated C string
/// - `callback` must be a valid function pointer that:
///   - Does not store the result_json pointer
///   - Returns a null-terminated UTF-8 string (error message) if validation fails
///   - Returns NULL if validation passes
///   - The returned string must be freeable by kreuzberg_free_string
/// - `priority` determines the order of validation (higher priority runs first)
/// - Returns true on success, false on error (check kreuzberg_last_error)
///
/// # Example (C)
///
/// ```c
/// char* my_validator(const char* result_json) {
///     // Parse result_json, validate it
///     // Return error message if validation fails, NULL if passes
///     if (invalid) {
///         return strdup("Validation failed: content too short");
///     }
///     return NULL;
/// }
///
/// bool success = kreuzberg_register_validator("my-validator", my_validator, 100);
/// if (!success) {
///     const char* error = kreuzberg_last_error();
///     printf("Failed to register: %s\n", error);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_register_validator(
    name: *const c_char,
    callback: ValidatorCallback,
    priority: i32,
) -> bool {
    ffi_panic_guard_bool!("kreuzberg_register_validator", {
        clear_last_error();

        if name.is_null() {
            set_last_error("Validator name cannot be NULL".to_string());
            return false;
        }

        // SAFETY: C callers may pass NULL for function pointer parameters.
        // `black_box` prevents the compiler from optimizing away the null check.
        if core::hint::black_box(callback as usize) == 0 {
            set_last_error("Validator callback cannot be NULL".to_string());
            return false;
        }

        let name_str = match unsafe { CStr::from_ptr(name) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in Validator name: {}", e));
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

        let validator = Arc::new(FfiValidator::new(name_str.to_string(), callback, priority));

        let registry = kreuzberg::plugins::registry::get_validator_registry();
        let mut registry_guard = registry.write();

        match registry_guard.register(validator) {
            Ok(()) => true,
            Err(e) => {
                set_last_error(format!("Failed to register Validator: {}", e));
                false
            }
        }
    })
}

/// Unregister a Validator by name.
///
/// # Safety
///
/// - `name` must be a valid null-terminated C string
/// - Returns true on success, false on error (check kreuzberg_last_error)
///
/// # Example (C)
///
/// ```c
/// bool success = kreuzberg_unregister_validator("my-validator");
/// if (!success) {
///     const char* error = kreuzberg_last_error();
///     printf("Failed to unregister: %s\n", error);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_unregister_validator(name: *const c_char) -> bool {
    ffi_panic_guard_bool!("kreuzberg_unregister_validator", {
        clear_last_error();

        if name.is_null() {
            set_last_error("Validator name cannot be NULL".to_string());
            return false;
        }

        let name_str = match unsafe { CStr::from_ptr(name) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in Validator name: {}", e));
                return false;
            }
        };

        let registry = kreuzberg::plugins::registry::get_validator_registry();
        let mut registry_guard = registry.write();

        match registry_guard.remove(name_str) {
            Ok(()) => true,
            Err(e) => {
                set_last_error(format!("Failed to remove Validator: {}", e));
                false
            }
        }
    })
}

/// Clear all registered Validators.
///
/// # Safety
///
/// - Removes all validators. Subsequent extractions will skip custom validation.
/// - Returns true on success, false on error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_clear_validators() -> bool {
    ffi_panic_guard_bool!("kreuzberg_clear_validators", {
        clear_last_error();

        let registry = kreuzberg::plugins::registry::get_validator_registry();
        let mut registry_guard = registry.write();

        *registry_guard = Default::default();
        true
    })
}

/// List all registered Validators as a JSON array of names.
///
/// # Safety
///
/// - Returned string must be freed with `kreuzberg_free_string`.
/// - Returns NULL on error (check `kreuzberg_last_error`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_list_validators() -> *mut c_char {
    ffi_panic_guard!("kreuzberg_list_validators", {
        clear_last_error();

        let registry = kreuzberg::plugins::registry::get_validator_registry();
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
                set_last_error(format!("Failed to serialize Validator list: {}", e));
                ptr::null_mut()
            }
        }
    })
}
