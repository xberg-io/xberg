//! Cross-platform timing utilities.
//!
//! `std::time::Instant` is not available on `wasm32` targets (causes `RuntimeError: unreachable`).
//! This module provides a lightweight [`Instant`] wrapper that uses the native monotonic clock on
//! non-WASM platforms and falls back to a no-op (zero elapsed) on WASM, where the timing data is
//! only used for optional tracing diagnostics.

/// A platform-aware instant for measuring elapsed time.
///
/// On native targets this delegates to [`std::time::Instant`].
/// On `wasm32` targets it is a zero-cost no-op to avoid the `unreachable` trap.
#[derive(Debug, Clone, Copy)]
pub struct Instant {
    #[cfg(all(not(target_arch = "wasm32"), feature = "pdf"))]
    inner: std::time::Instant,
}

#[cfg(feature = "pdf")]
impl Instant {
    /// Capture the current instant.
    #[inline]
    pub(crate) fn now() -> Self {
        Self {
            #[cfg(all(not(target_arch = "wasm32"), feature = "pdf"))]
            inner: std::time::Instant::now(),
        }
    }

    /// Seconds elapsed since this instant was captured (as `f64`).
    #[inline]
    pub(crate) fn elapsed_secs_f64(&self) -> f64 {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.inner.elapsed().as_secs_f64()
        }
        #[cfg(target_arch = "wasm32")]
        {
            0.0
        }
    }

    /// Milliseconds elapsed since this instant was captured (as `f64`).
    #[inline]
    pub(crate) fn elapsed_ms(&self) -> f64 {
        self.elapsed_secs_f64() * 1000.0
    }
}
