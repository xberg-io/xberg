//! Error handling and conversion to Ruby exceptions
//!
//! Provides error conversion from Kreuzberg errors to Magnus Ruby exceptions,
//! panic context retrieval, and error code utilities.

use kreuzberg::KreuzbergError;
use magnus::{Error, exception::ExceptionClass, Ruby};
use std::ffi::CStr;

pub use kreuzberg_ffi::{
    kreuzberg_free_string, kreuzberg_last_error_code,
    kreuzberg_last_panic_context,
};

/// Retrieve panic context from FFI if available
pub fn get_panic_context() -> Option<String> {
    unsafe {
        let ctx_ptr = kreuzberg_last_panic_context();
        if ctx_ptr.is_null() {
            return None;
        }

        let c_str = CStr::from_ptr(ctx_ptr);
        let context = c_str.to_string_lossy().to_string();
        kreuzberg_free_string(ctx_ptr as *mut std::ffi::c_char);

        if context.is_empty() { None } else { Some(context) }
    }
}

/// Retrieve error code from FFI
pub fn get_error_code() -> i32 {
    unsafe { kreuzberg_last_error_code() }
}

/// Convert Kreuzberg errors to Ruby exceptions
pub fn kreuzberg_error(err: KreuzbergError) -> Error {
    let ruby = Ruby::get().expect("Ruby not initialized");

    let fetch_error_class = |name: &str| -> Option<ExceptionClass> {
        ruby.eval::<ExceptionClass>(&format!("Kreuzberg::Errors::{}", name))
            .ok()
    };

    match err {
        KreuzbergError::Validation { message, .. } => {
            if let Some(class) = fetch_error_class("ValidationError") {
                Error::new(class, message)
            } else {
                Error::new(ruby.exception_arg_error(), message)
            }
        }
        KreuzbergError::Parsing { message, .. } => {
            if let Some(class) = fetch_error_class("ParsingError") {
                Error::new(class, message)
            } else {
                Error::new(ruby.exception_runtime_error(), format!("ParsingError: {}", message))
            }
        }
        KreuzbergError::Ocr { message, .. } => {
            if let Some(class) = fetch_error_class("OCRError") {
                Error::new(class, message)
            } else {
                Error::new(ruby.exception_runtime_error(), format!("OCRError: {}", message))
            }
        }
        KreuzbergError::MissingDependency(message) => {
            if let Some(class) = fetch_error_class("MissingDependencyError") {
                Error::new(class, message)
            } else {
                Error::new(
                    ruby.exception_runtime_error(),
                    format!("MissingDependencyError: {}", message),
                )
            }
        }
        KreuzbergError::Plugin { message, plugin_name } => {
            if let Some(class) = fetch_error_class("PluginError") {
                Error::new(class, format!("{}: {}", plugin_name, message))
            } else {
                Error::new(
                    ruby.exception_runtime_error(),
                    format!("Plugin error in '{}': {}", plugin_name, message),
                )
            }
        }
        KreuzbergError::Io(err) => {
            if let Some(class) = fetch_error_class("IOError") {
                Error::new(class, err.to_string())
            } else {
                Error::new(ruby.exception_runtime_error(), format!("IO error: {}", err))
            }
        }
        KreuzbergError::UnsupportedFormat(message) => {
            if let Some(class) = fetch_error_class("UnsupportedFormatError") {
                Error::new(class, message)
            } else {
                Error::new(
                    ruby.exception_runtime_error(),
                    format!("UnsupportedFormatError: {}", message),
                )
            }
        }
        other => Error::new(ruby.exception_runtime_error(), other.to_string()),
    }
}

/// Create a generic runtime error
pub fn runtime_error(message: impl Into<String>) -> Error {
    let ruby = Ruby::get().expect("Ruby not initialized");
    Error::new(ruby.exception_runtime_error(), message.into())
}

/// Create a validation error (Kreuzberg::Errors::ValidationError)
pub fn validation_error(message: impl Into<String>) -> Error {
    let ruby = Ruby::get().expect("Ruby not initialized");

    // Try to get the ValidationError class from Ruby
    if let Ok(class) = ruby.eval::<ExceptionClass>("Kreuzberg::Errors::ValidationError") {
        Error::new(class, message.into())
    } else {
        // Fall back to ArgumentError if the class doesn't exist
        Error::new(ruby.exception_arg_error(), message.into())
    }
}
