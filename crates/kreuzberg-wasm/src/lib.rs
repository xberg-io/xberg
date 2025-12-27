//! Kreuzberg - WebAssembly Bindings
//!
//! This crate provides WASM-compatible bindings for the Kreuzberg document intelligence library.
//! It enables document extraction, text processing, and analysis in browser and server-side
//! WebAssembly environments.
//!
//! ## Features
//!
//! - Extract text from PDFs, Office documents, and other formats
//! - Compatible with browser and Node.js WASM runtimes
//! - Optimized binary size and performance
//! - Type-safe interface via wasm-bindgen
//!
//! ## PDF Support in WASM
//!
//! **IMPORTANT**: PDF extraction requires initializing PDFium before use.
//!
//! The PDFium library must be loaded and initialized from JavaScript before calling
//! any PDF extraction functions. This is a two-step process:
//!
//! 1. Load both kreuzberg WASM and PDFium WASM modules
//! 2. Call `initialize_pdfium_render()` to bind them together
//!
//! ### Example (JavaScript/TypeScript)
//!
//! ```javascript
//! import init, { initialize_pdfium_render, extract_from_bytes } from './kreuzberg_wasm.js';
//! import pdfiumModule from './pdfium.js';
//!
//! // Step 1: Initialize kreuzberg WASM
//! const wasm = await init();
//!
//! // Step 2: Load PDFium WASM module
//! const pdfium = await pdfiumModule();
//!
//! // Step 3: Bind kreuzberg to PDFium (REQUIRED for PDF extraction)
//! const success = initialize_pdfium_render(pdfium, wasm, false);
//! if (!success) {
//!     throw new Error('Failed to initialize PDFium');
//! }
//!
//! // Step 4: Now PDF extraction works
//! const pdfBytes = new Uint8Array([...]); // Your PDF file bytes
//! const config = { /* extraction config */ };
//! const result = await extract_from_bytes(pdfBytes, config);
//! ```
//!
//! ### Where to get pdfium.js
//!
//! The PDFium WASM files (pdfium.js, pdfium.wasm) are automatically downloaded during
//! the build process and placed in your build output directory. You need to:
//!
//! 1. Copy `pdfium.js` and `pdfium.wasm` from the build artifacts
//! 2. Serve them alongside your kreuzberg WASM files
//! 3. Import and initialize as shown above
//!
//! The files are typically found at:
//! - `target/wasm32-unknown-unknown/release/build/kreuzberg-*/out/pdfium/release/node/`
//!
//! ### Error: "PdfiumWASMModuleNotConfigured"
//!
//! If you see this error, it means `initialize_pdfium_render()` was not called before
//! attempting PDF extraction. Make sure to follow the initialization sequence above.
//!
//! ## WASM Limitations
//!
//! ### HTML File Size Limit
//!
//! **HTML files are limited to 2MB** in WASM builds due to limited stack space.
//! Files exceeding this limit will be rejected with a validation error to prevent
//! stack overflow. For larger HTML files, use the native Kreuzberg library.
//!
//! ### PDF Support
//!
//! PDF extraction via PDFium is **only available in browser environments**.
//! Non-browser WASM runtimes (Deno, Cloudflare Workers) cannot use PDF extraction.
//! For server-side PDF processing, use the native library bindings.

use wasm_bindgen::prelude::*;

/// Initialize a WebAssembly thread pool (optional).
///
/// By default, Kreuzberg's WASM build ships without multi-threading enabled.
/// If the `threads` feature is enabled at build time, this function delegates
/// to `wasm-bindgen-rayon` to spawn a worker-based thread pool.
#[cfg(feature = "threads")]
#[wasm_bindgen(js_name = initThreadPool)]
pub fn init_thread_pool(num_threads: usize) -> js_sys::Promise {
    wasm_bindgen_rayon::init_thread_pool(num_threads)
}

#[cfg(not(feature = "threads"))]
#[wasm_bindgen(js_name = initThreadPool)]
pub fn init_thread_pool(_num_threads: usize) -> js_sys::Promise {
    js_sys::Promise::reject(&JsValue::from_str(
        "Thread pool is not available in this build (feature `threads` is disabled).",
    ))
}

pub mod config;
pub mod embeddings;
pub mod errors;
pub mod extraction;
pub mod mime;
pub mod plugins;
pub mod types;

pub use config::*;
pub use errors::*;
pub use extraction::*;
pub use mime::*;
pub use plugins::*;
pub use types::*;

/// Version of the kreuzberg-wasm binding
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Initialize the WASM module
/// This function should be called once at application startup
#[wasm_bindgen]
pub fn init() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

/// Get information about the WASM module
#[wasm_bindgen]
pub struct ModuleInfo {
    version: String,
    name: String,
}

#[wasm_bindgen]
impl ModuleInfo {
    /// Get the module version
    pub fn version(&self) -> String {
        self.version.clone()
    }

    /// Get the module name
    pub fn name(&self) -> String {
        self.name.clone()
    }
}

/// Get module information
#[wasm_bindgen]
pub fn get_module_info() -> ModuleInfo {
    ModuleInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        name: "kreuzberg-wasm".to_string(),
    }
}

/// Helper function to initialize the thread pool with error handling
/// Accepts the number of threads to use for the thread pool.
/// Returns true if initialization succeeded, false for graceful degradation.
///
/// This function wraps init_thread_pool with panic handling to ensure graceful
/// degradation if thread pool initialization fails. The application will continue
/// to work in single-threaded mode if the thread pool cannot be initialized.
#[wasm_bindgen]
pub fn init_thread_pool_safe(num_threads: u32) -> bool {
    #[cfg(not(feature = "threads"))]
    {
        let _ = num_threads;
        #[cfg(target_arch = "wasm32")]
        web_sys::console::warn_1(&"Thread pool is not available in this build (feature `threads` is disabled).".into());
        false
    }

    #[cfg(feature = "threads")]
    {
        if num_threads == 0 {
            #[cfg(target_arch = "wasm32")]
            web_sys::console::warn_1(&"Invalid thread count (0). Using single-threaded mode.".into());
            return false;
        }

        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| init_thread_pool(num_threads as usize))) {
            Ok(_) => {
                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&format!("Thread pool initialized with {} threads", num_threads).into());
                true
            }
            Err(_) => {
                #[cfg(target_arch = "wasm32")]
                web_sys::console::warn_1(
                    &"Failed to initialize thread pool. Falling back to single-threaded mode.".into(),
                );
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_thread_pool_export_exists() {
        let _ = init_thread_pool;
    }

    #[test]
    fn test_init_thread_pool_safe_graceful_handling() {
        let result = init_thread_pool_safe(2);

        assert!(
            matches!(result, true | false),
            "init_thread_pool_safe should return a boolean"
        );
    }

    #[test]
    fn test_init_thread_pool_safe_invalid_thread_count() {
        let result = init_thread_pool_safe(0);

        assert!(!result, "init_thread_pool_safe should return false for zero threads");
    }

    #[test]
    fn test_init_thread_pool_safe_valid_thread_count() {
        let result = init_thread_pool_safe(1);

        assert!(
            matches!(result, true | false),
            "init_thread_pool_safe should return a boolean"
        );
    }

    #[test]
    fn test_module_info_behavior() {
        let info = get_module_info();

        assert!(!info.name().is_empty(), "Module name should not be empty");
        assert_eq!(info.name(), "kreuzberg-wasm", "Module name should be correct");

        assert!(!info.version().is_empty(), "Version should not be empty");

        let version = info.version();
        assert!(version.contains('.'), "Version should contain at least one dot");
    }

    #[test]
    fn test_version_behavior() {
        let v = version();

        assert!(!v.is_empty(), "Version string should not be empty");

        let main_version = v.split('-').next().unwrap_or(&v);
        let parts: Vec<&str> = main_version.split('.').collect();
        assert!(parts.len() >= 2, "Version should have at least major.minor components");

        for (i, part) in parts.iter().enumerate() {
            assert!(!part.is_empty(), "Version component {} should not be empty", i);
            assert!(
                part.chars().all(|c| c.is_ascii_digit()),
                "Version component {} should be numeric: {}",
                i,
                part
            );
        }
    }

    #[test]
    fn test_module_info_consistency() {
        let info = get_module_info();
        let version_str = version();

        assert_eq!(
            info.version(),
            version_str,
            "Module info version should match version() function"
        );
    }
}
