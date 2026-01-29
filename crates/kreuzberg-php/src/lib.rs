//! Kreuzberg PHP Bindings
//!
//! This module exposes the Rust core extraction API to PHP using ext-php-rs.
//!
//! # Architecture
//!
//! - All extraction logic is in the Rust core (crates/kreuzberg)
//! - PHP is a thin wrapper that adds language-specific features
//! - Zero duplication of core functionality
//! - Modern ext-php-rs patterns throughout

#![cfg_attr(windows, feature(abi_vectorcall))]
#![allow(dead_code, unused_imports)]

use ext_php_rs::builders::FunctionBuilder;
use ext_php_rs::prelude::*;
use ext_php_rs::types::Zval;

pub mod config;
pub mod embeddings;
pub mod error;
pub mod extraction;
pub mod plugins;
pub mod types;
pub mod validation;

/// Setup ONNX Runtime library path early to prevent initialization panics.
///
/// This function checks for ONNX Runtime in common installation paths and sets
/// the ORT_DYLIB_PATH environment variable if found. This is called via a
/// constructor attribute to ensure it runs before any code that might trigger
/// ONNX Runtime initialization.
#[ctor::ctor]
fn setup_onnx_runtime_path() {
    // Check if ORT_DYLIB_PATH is already set
    if std::env::var("ORT_DYLIB_PATH").is_ok() {
        return;
    }

    // Check common installation paths
    #[cfg(target_os = "macos")]
    {
        let paths = vec![
            "/opt/homebrew/lib/libonnxruntime.dylib",
            "/usr/local/lib/libonnxruntime.dylib",
        ];
        for path in paths {
            if std::path::Path::new(path).exists() {
                #[allow(unsafe_code)]
                unsafe {
                    std::env::set_var("ORT_DYLIB_PATH", path);
                }
                return;
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let paths = vec![
            "/usr/lib/libonnxruntime.so",
            "/usr/local/lib/libonnxruntime.so",
            "/usr/lib/x86_64-linux-gnu/libonnxruntime.so",
            "/usr/lib/aarch64-linux-gnu/libonnxruntime.so",
        ];
        for path in paths {
            if std::path::Path::new(path).exists() {
                #[allow(unsafe_code)]
                unsafe {
                    std::env::set_var("ORT_DYLIB_PATH", path);
                }
                return;
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        let paths = vec![
            "C:\\Program Files\\onnxruntime\\bin\\onnxruntime.dll",
            "C:\\Windows\\System32\\onnxruntime.dll",
        ];
        for path in paths {
            if std::path::Path::new(path).exists() {
                #[allow(unsafe_code)]
                unsafe {
                    std::env::set_var("ORT_DYLIB_PATH", path);
                }
                return;
            }
        }
    }
}

/// Get the Kreuzberg library version.
///
/// # Returns
///
/// Version string in semver format (e.g., "4.2.4")
///
/// # Example
///
/// ```php
/// $version = kreuzberg_version();
/// echo "Kreuzberg version: $version\n";
/// ```
#[php_function]
pub fn kreuzberg_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Kreuzberg PHP extension module.
///
/// Exports all extraction functions, configuration types, error handling, and plugin management.
#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    // Note: ORT_DYLIB_PATH is set by setup_onnx_runtime_path() which runs automatically
    // via the #[ctor::ctor] attribute before this function is called

    // Register the main module function
    let mut module = module.function(wrap_function!(kreuzberg_version));

    // Register functions from all submodules
    for builder in config::get_function_builders() {
        module = module.function(builder);
    }
    for builder in embeddings::get_function_builders() {
        module = module.function(builder);
    }
    for builder in error::get_function_builders() {
        module = module.function(builder);
    }
    for builder in extraction::get_function_builders() {
        module = module.function(builder);
    }
    for builder in plugins::get_function_builders() {
        module = module.function(builder);
    }
    for builder in validation::get_function_builders() {
        module = module.function(builder);
    }

    // Register all PHP classes (order matters for dependencies)
    // Types module - base result types
    module = module
        .class::<types::Metadata>()
        .class::<types::ExtractedImage>()
        .class::<types::ExtractedTable>()
        .class::<types::ChunkMetadata>()      // Must be registered before TextChunk
        .class::<types::TextChunk>()
        .class::<types::PageResult>()
        .class::<types::Keyword>()            // Must be registered before ExtractionResult
        .class::<types::ExtractionResult>();

    // Note: Config classes are pure PHP (packages/php/src/Config/*.php)
    // No Rust config classes are exposed - configs are passed as JSON

    // Embeddings module
    module = module.class::<embeddings::EmbeddingPreset>();

    // Error module
    module = module.class::<error::ErrorClassification>();

    module
}
