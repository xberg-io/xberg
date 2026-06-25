//! Stack management for HTML extraction with support for large documents.
//!
//! This module handles the specialized concern of managing stack size for HTML conversion,
//! particularly for large HTML documents that may require more stack space than the default.
//! On WASM, stack size is limited and cannot be increased, so size limits are enforced.
//! On native platforms, dedicated threads with larger stacks are used for large HTML.

use crate::error::{Result, XbergError};

#[cfg(not(target_arch = "wasm32"))]
use std::{any::Any, thread};

#[cfg(target_arch = "wasm32")]
pub const MAX_HTML_SIZE_BYTES: usize = 2 * 1024 * 1024;

#[cfg(not(target_arch = "wasm32"))]
pub const LARGE_HTML_STACK_THRESHOLD_BYTES: usize = 512 * 1024;

#[cfg(not(target_arch = "wasm32"))]
pub const HTML_CONVERSION_STACK_SIZE_BYTES: usize = 16 * 1024 * 1024;

/// Check if HTML size exceeds WASM limit and return error if so.
///
/// WASM builds have a fixed stack size that cannot be increased, so we enforce
/// a 2MB limit to prevent stack overflow during HTML conversion.
#[cfg(target_arch = "wasm32")]
pub(crate) fn check_wasm_size_limit(html: &str) -> Result<()> {
    if html.len() > MAX_HTML_SIZE_BYTES {
        return Err(XbergError::validation(format!(
            "HTML file size ({} bytes) exceeds WASM limit of {} bytes (2MB). \
             Large HTML files cannot be processed in WASM due to stack constraints. \
             Consider using the native library for files of this size.",
            html.len(),
            MAX_HTML_SIZE_BYTES
        )));
    }
    Ok(())
}

/// Check if HTML size exceeds WASM limit and return error if so.
///
/// No-op on non-WASM platforms.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn check_wasm_size_limit(_html: &str) -> Result<()> {
    Ok(())
}

/// Determine if HTML requires a dedicated stack due to size.
///
/// On native platforms, HTML larger than the threshold will be processed
/// on a dedicated thread with a larger stack to prevent overflow.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn html_requires_large_stack(len: usize) -> bool {
    len >= LARGE_HTML_STACK_THRESHOLD_BYTES
}

/// Run a job on a dedicated thread with a large stack.
///
/// This is useful for HTML conversion of large documents that might
/// overflow the default thread stack on native platforms.
///
/// # Arguments
///
/// * `job` - The closure to execute on the dedicated thread
///
/// # Returns
///
/// The result of the job execution, or an error if the thread panics
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn run_on_dedicated_stack<T, F>(job: F) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T> + Send + 'static,
{
    let handle = thread::Builder::new()
        .name("xberg-html-conversion".to_string())
        .stack_size(HTML_CONVERSION_STACK_SIZE_BYTES)
        .spawn(job)
        .map_err(|err| XbergError::Other(format!("Failed to spawn HTML conversion thread: {}", err)))?;

    match handle.join() {
        Ok(result) => result,
        Err(panic) => {
            let reason = extract_panic_reason(&panic);
            Err(XbergError::Other(format!("HTML conversion panicked: {}", reason)))
        }
    }
}

/// Extract a human-readable reason from a panic.
///
/// Attempts to downcast the panic value to either &str or String,
/// falling back to a generic message if neither succeeds.
#[cfg(not(target_arch = "wasm32"))]
fn extract_panic_reason(panic: &Box<dyn Any + Send + 'static>) -> String {
    if let Some(msg) = panic.downcast_ref::<&str>() {
        (*msg).to_string()
    } else if let Some(msg) = panic.downcast_ref::<String>() {
        msg.clone()
    } else {
        "unknown panic".to_string()
    }
}
