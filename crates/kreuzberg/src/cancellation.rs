//! Cancellation token for extraction operations.
//!
//! Provides a lightweight, FFI-friendly cancellation primitive based on
//! `Arc<AtomicBool>`. The token can be cloned and shared across threads;
//! any holder can cancel the operation and all other holders will observe
//! the cancellation on their next check.
//!
//! # Design
//!
//! - `Arc<AtomicBool>` is used rather than `tokio_util::CancellationToken` so
//!   the type has no Tokio dependency at the type level and is usable from both
//!   sync and async contexts.
//! - `Ordering::Relaxed` is sufficient: we only need eventual visibility, not
//!   happens-before ordering relative to other memory accesses.
//! - The token wraps an `Arc<AtomicBool>` and can be stored in
//!   `ExtractionConfig` without layout surprises.
//!
//! # FFI
//!
//! The FFI crate wraps this type in an opaque `*mut CancellationToken` handle
//! (see `crates/kreuzberg-ffi/src/cancellation.rs`).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// A lightweight, cloneable cancellation token.
///
/// Create one with [`CancellationToken::new`], pass clones to the extraction
/// call (via [`ExtractionConfig::cancel_token`]) and to the caller. Call
/// [`CancellationToken::cancel`] from the caller side when the operation
/// should be aborted. The extraction code polls
/// [`CancellationToken::is_cancelled`] at safe checkpoints and returns
/// [`KreuzbergError::Cancelled`] if set.
///
/// Cloning is cheap (increments the `Arc` reference count only).
#[derive(Debug, Clone, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    /// Create a new, un-cancelled token.
    #[inline]
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Signal cancellation.
    ///
    /// All clones of this token will observe [`is_cancelled`] returning `true`
    /// on their next check. This operation is idempotent.
    #[inline]
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    /// Returns `true` if [`cancel`] has been called on any clone of this token.
    #[inline]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_token_is_not_cancelled() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());
    }

    #[test]
    fn test_cancel_sets_flag() {
        let token = CancellationToken::new();
        token.cancel();
        assert!(token.is_cancelled());
    }

    #[test]
    fn test_clone_shares_state() {
        let token = CancellationToken::new();
        let clone = token.clone();
        assert!(!clone.is_cancelled());
        token.cancel();
        assert!(clone.is_cancelled());
    }

    #[test]
    fn test_cancel_is_idempotent() {
        let token = CancellationToken::new();
        token.cancel();
        token.cancel();
        assert!(token.is_cancelled());
    }

    #[test]
    fn test_default_is_not_cancelled() {
        let token = CancellationToken::default();
        assert!(!token.is_cancelled());
    }
}
