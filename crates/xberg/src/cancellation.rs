//! Cancellation token for extraction operations.
//!
//! Provides a lightweight, FFI-friendly cancellation primitive based on
//! `Arc<AtomicBool>`. The token can be cloned and shared across threads;
//! any holder can request cancellation and all other holders will observe
//! the request on their next check.
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
//! # Rust callers
//!
//! Rust callers may place one clone in `ExtractionConfig::cancel_token` and
//! retain another clone in the application layer. Calling [`CancellationToken::cancel`]
//! requests that extraction stop when the active pipeline reaches its next
//! cancellation checkpoint. Cancellation is cooperative rather than immediate,
//! so latency depends on the extractor and operation currently in progress.
//!
//! # FFI and bindings
//!
//! No binding exposes cancellation today. `crates/xberg-ffi` emits no
//! cancellation handle and no `xberg_cancel*` symbol, and neither do the
//! Python, Node, JNI, PHP or WASM crates. Both this type and the
//! `ExtractionConfig::cancel_token` field carry `#[cfg_attr(alef, alef(skip))]`,
//! so codegen can never emit one — any binding surface would have to be
//! hand-written and hand-maintained.
//!
//! Xberg also uses the same token internally for extraction timeouts and the
//! REST async-job cancellation path (`DELETE /jobs/{job_id}`).

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// A lightweight, cloneable, one-shot cancellation token.
///
/// Create one with [`CancellationToken::new`], pass one clone to extraction
/// through `ExtractionConfig::cancel_token`, and retain another clone in the
/// caller. Calling [`cancel`](Self::cancel) requests cooperative cancellation;
/// extraction stops when it reaches an existing cancellation checkpoint.
///
/// A token cannot be reset. Once cancellation has been requested, all current
/// and future clones continue to report that request. Create a new token for an
/// unrelated extraction operation.
///
/// Cloning is cheap (increments the `Arc` reference count only).
///
/// # Example
///
/// ```
/// use xberg::{ExtractionConfig, cancellation::CancellationToken};
///
/// let caller_token = CancellationToken::new();
/// let config = ExtractionConfig {
///     cancel_token: Some(caller_token.clone()),
///     ..Default::default()
/// };
///
/// caller_token.cancel();
/// assert!(config.cancel_token.is_some_and(|token| token.is_cancelled()));
/// ```
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
    stop: Arc<StopState>,
}

/// Soft-stop state shared by every clone of a token.
///
/// A soft stop is the extraction time limit asking page loops to finish the page in hand and
/// return what they hold, where [`CancellationToken::cancel`] makes them fail. It is internal:
/// only the timeout wrapper requests it and only page loops observe it.
#[derive(Debug)]
struct StopState {
    requested: AtomicBool,
    page_loop_registered: AtomicBool,
    first_unread_page: AtomicU32,
}

impl Default for StopState {
    fn default() -> Self {
        Self {
            requested: AtomicBool::new(false),
            page_loop_registered: AtomicBool::new(false),
            first_unread_page: AtomicU32::new(NO_UNREAD_PAGE),
        }
    }
}

const NO_UNREAD_PAGE: u32 = u32::MAX;

impl CancellationToken {
    /// Create a new, uncancelled token.
    pub fn new() -> Self {
        Self::default()
    }

    /// Request cooperative cancellation.
    ///
    /// All clones of this token will observe [`is_cancelled`](Self::is_cancelled)
    /// returning `true` on their next check. This operation is idempotent and
    /// the token remains cancelled permanently.
    ///
    /// This method does not forcibly interrupt the currently executing parser,
    /// native call, or model invocation. Extraction stops when the active path
    /// reaches an existing cancellation checkpoint, so no maximum cancellation
    /// latency is implied.
    #[inline]
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    /// Returns `true` if cancellation has been requested on any clone.
    ///
    /// This reports the token's request flag. It does not prove that a particular
    /// extraction terminated because cancellation can race with normal completion.
    #[inline]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    /// Ask page loops to stop at the next page boundary and return the pages they hold.
    #[cfg(any(test, all(feature = "tokio-runtime", not(target_arch = "wasm32"))))]
    pub(crate) fn request_stop(&self) {
        self.stop.requested.store(true, Ordering::Relaxed);
    }

    /// Returns `true` once [`request_stop`](Self::request_stop) was called on any clone.
    pub(crate) fn stop_requested(&self) -> bool {
        self.stop.requested.load(Ordering::Relaxed)
    }

    /// Record that a page loop that honours [`stop_requested`](Self::stop_requested) is running,
    /// so the timeout wrapper knows waiting for it can return a partial result.
    pub(crate) fn register_page_loop(&self) {
        self.stop.page_loop_registered.store(true, Ordering::Relaxed);
    }

    #[cfg(any(test, all(feature = "tokio-runtime", not(target_arch = "wasm32"))))]
    pub(crate) fn page_loop_registered(&self) -> bool {
        self.stop.page_loop_registered.load(Ordering::Relaxed)
    }

    /// Record that a page loop stopped before reading `page` (1-based). The lowest page wins.
    pub(crate) fn note_unread_page(&self, page: u32) {
        self.stop.first_unread_page.fetch_min(page, Ordering::Relaxed);
    }

    /// The lowest page (1-based) a stopped page loop did not read, if any loop stopped early.
    #[cfg(any(test, all(feature = "tokio-runtime", not(target_arch = "wasm32"))))]
    pub(crate) fn first_unread_page(&self) -> Option<u32> {
        match self.stop.first_unread_page.load(Ordering::Relaxed) {
            NO_UNREAD_PAGE => None,
            page => Some(page),
        }
    }
}

impl Serialize for CancellationToken {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let state = self.is_cancelled();
        state.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for CancellationToken {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let cancelled = bool::deserialize(deserializer)?;
        Ok(CancellationToken {
            cancelled: Arc::new(AtomicBool::new(cancelled)),
            stop: Arc::default(),
        })
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
