//! Python-facing CancellationToken wrapper.
//!
//! Exposes `CancellationToken` as a Python class so callers can cancel
//! in-progress extractions from Python code.
//!
//! # Example (Python)
//!
//! ```python
//! import asyncio
//! from kreuzberg import extract_file, ExtractionConfig, CancellationToken
//!
//! async def main():
//!     token = CancellationToken()
//!     config = ExtractionConfig()
//!     config.cancel_token = token
//!     task = asyncio.create_task(extract_file("large.pdf", config=config))
//!     await asyncio.sleep(0.1)
//!     token.cancel()
//!     try:
//!         await task
//!     except ExtractionCancelledError:
//!         print("Cancelled as expected")
//! ```

use pyo3::prelude::*;

/// A cancellation token that can be shared with an extraction to interrupt it.
///
/// Create a token, attach it to `ExtractionConfig.cancel_token`, then call
/// `.cancel()` from any thread to signal the extraction to stop.
///
/// Cancellation is cooperative: the extraction checks the token at well-defined
/// points (before acquiring the PDFium lock, before spawning blocking tasks).
/// It does **not** kill a thread mid-flight.
///
/// # Thread Safety
///
/// `CancellationToken` is safe to share across threads; the `cancel()` method
/// may be called from any thread (including a background thread or a signal handler).
#[pyclass(name = "CancellationToken", module = "kreuzberg", from_py_object)]
#[derive(Clone)]
pub struct PyCancellationToken {
    pub(crate) inner: kreuzberg::CancellationToken,
}

impl Default for PyCancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

#[pymethods]
impl PyCancellationToken {
    /// Create a new, un-cancelled cancellation token.
    #[new]
    pub fn new() -> Self {
        Self {
            inner: kreuzberg::CancellationToken::new(),
        }
    }

    /// Signal cancellation. Idempotent — safe to call multiple times.
    ///
    /// Any extraction that holds a clone of this token will observe
    /// `is_cancelled()` returning `True` on its next check.
    pub fn cancel(&self) {
        self.inner.cancel();
    }

    /// Return `True` if `cancel()` has been called, `False` otherwise.
    pub fn is_cancelled(&self) -> bool {
        self.inner.is_cancelled()
    }

    fn __repr__(&self) -> String {
        format!("CancellationToken(cancelled={})", self.inner.is_cancelled())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::Once;

    fn prepare_python() {
        static INIT: Once = Once::new();
        INIT.call_once(Python::initialize);
    }

    fn with_py<F, R>(f: F) -> R
    where
        F: FnOnce(Python<'_>) -> R,
    {
        prepare_python();
        Python::attach(f)
    }

    #[test]
    fn test_new_not_cancelled() {
        with_py(|_py| {
            let token = PyCancellationToken::new();
            assert!(!token.is_cancelled());
        });
    }

    #[test]
    fn test_cancel_sets_flag() {
        with_py(|_py| {
            let token = PyCancellationToken::new();
            token.cancel();
            assert!(token.is_cancelled());
        });
    }

    #[test]
    fn test_clone_shares_state() {
        with_py(|_py| {
            let token = PyCancellationToken::new();
            let clone = token.clone();
            token.cancel();
            assert!(clone.is_cancelled());
        });
    }

    #[test]
    fn test_repr_uncancelled() {
        with_py(|_py| {
            let token = PyCancellationToken::new();
            assert_eq!(token.__repr__(), "CancellationToken(cancelled=false)");
        });
    }

    #[test]
    fn test_repr_cancelled() {
        with_py(|_py| {
            let token = PyCancellationToken::new();
            token.cancel();
            assert_eq!(token.__repr__(), "CancellationToken(cancelled=true)");
        });
    }
}
