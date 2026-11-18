//! Cancellation token for extraction operations.
//!
//! Provides a lightweight, FFI-friendly cancellation primitive based on
//! a shared atomic flag. The token can be cloned and shared across threads;
//! any holder can request cancellation and all other holders will observe
//! the request on their next check.
//!
//! # Design
//!
//! - Synchronous callers read the shared atomic flag. Native Tokio builds also
//!   notify async waiters that cannot perform a checkpoint until an event wakes them.
//! - `Ordering::Relaxed` is sufficient: we only need eventual visibility, not
//!   happens-before ordering relative to other memory accesses.
//! - The flag and notification share one allocation across regular and scoped clones.
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
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::time::Instant;

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
    cancelled: Arc<CancelState>,
    stop: Arc<StopState>,
}

#[derive(Debug, Default)]
struct CancelState {
    requested: AtomicBool,
    #[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
    notification: tokio::sync::Notify,
}

/// Soft-stop state of one timed extraction.
///
/// A soft stop is the extraction time limit asking page loops to finish the page in hand and
/// return what they hold, where [`CancellationToken::cancel`] makes them fail. It is internal:
/// only the timeout wrapper requests it and only page loops observe it. A timed extraction
/// works on a [`scoped`](CancellationToken::scoped) token, so a caller's token that is reused
/// for another extraction never carries it over.
// Read only by the page loops and the timeout wrapper, which are feature-gated; a build with
// neither has no reader. ~keep
#[cfg_attr(
    not(any(
        test,
        feature = "pdf",
        feature = "ocr",
        feature = "ocr-pipeline",
        all(feature = "tokio-runtime", not(target_arch = "wasm32"))
    )),
    allow(dead_code)
)]
#[derive(Debug)]
struct StopState {
    requested: AtomicBool,
    active_page_loops: AtomicUsize,
    first_unread_page: AtomicU32,
    deadline: OnceLock<Instant>,
    stage_cut: AtomicBool,
}

impl Default for StopState {
    fn default() -> Self {
        Self {
            requested: AtomicBool::new(false),
            active_page_loops: AtomicUsize::new(0),
            first_unread_page: AtomicU32::new(NO_UNREAD_PAGE),
            deadline: OnceLock::new(),
            stage_cut: AtomicBool::new(false),
        }
    }
}

const NO_UNREAD_PAGE: u32 = u32::MAX;

/// Marks a page loop that honours the soft stop as running; dropping it marks the loop as done.
#[cfg(any(test, feature = "pdf"))]
#[derive(Debug)]
pub(crate) struct PageLoopGuard(Arc<StopState>);

#[cfg(any(test, feature = "pdf"))]
impl Drop for PageLoopGuard {
    fn drop(&mut self) {
        self.0.active_page_loops.fetch_sub(1, Ordering::Relaxed);
    }
}

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
        self.mark_cancelled();
    }

    /// Request cancellation for a hard timeout and report whether this request set the flag.
    #[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
    pub(crate) fn cancel_for_timeout(&self) -> bool {
        self.mark_cancelled()
    }

    fn mark_cancelled(&self) -> bool {
        let first = !self.cancelled.requested.swap(true, Ordering::Relaxed);
        #[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
        if first {
            self.cancelled.notification.notify_waiters();
        }
        first
    }

    /// Wait for hard cancellation, including a deadline enforced by the extraction wrapper.
    #[cfg(all(
        any(feature = "office", feature = "excel", feature = "excel-wasm"),
        feature = "tokio-runtime",
        not(target_arch = "wasm32")
    ))]
    pub(crate) async fn cancelled(&self) {
        // Subscribe before reading the flag; notify_waiters also wakes a Notified not yet polled. ~keep
        let notified = self.cancelled.notification.notified();
        if !self.is_cancelled() {
            notified.await;
        }
    }

    /// Returns `true` if cancellation has been requested on any clone.
    ///
    /// This reports the token's request flag. It does not prove that a particular
    /// extraction terminated because cancellation can race with normal completion.
    #[inline]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.requested.load(Ordering::Relaxed)
    }

    /// A token that shares this token's cancellation but has its own soft-stop state.
    #[cfg(any(test, all(feature = "tokio-runtime", not(target_arch = "wasm32"))))]
    pub(crate) fn scoped(&self) -> Self {
        Self {
            cancelled: Arc::clone(&self.cancelled),
            stop: Arc::default(),
        }
    }

    /// Ask page loops to stop at the next page boundary and return the pages they hold.
    #[cfg(any(test, all(feature = "tokio-runtime", not(target_arch = "wasm32"))))]
    pub(crate) fn request_stop(&self) {
        self.stop.requested.store(true, Ordering::Relaxed);
    }

    /// Record the instant the extraction time limit ends, for stages that run too long between two
    /// awaits to be reached by the timeout: the future that enforces the limit is polled by the
    /// extraction's own task, so a model call that never yields keeps it from firing at all.
    #[cfg(any(test, all(feature = "tokio-runtime", not(target_arch = "wasm32"))))]
    pub(crate) fn set_deadline(&self, deadline: Instant) {
        let _ = self.stop.deadline.set(deadline);
    }

    /// Whether the extraction time limit has been reached: a stop was requested or the deadline
    /// passed. Stages that run model calls ask this between batches and stop early when it holds.
    pub(crate) fn time_limit_reached(&self) -> bool {
        self.stop_requested() || self.deadline_elapsed()
    }

    /// Whether the recorded deadline has passed, independently of the cooperative stop flag.
    pub(crate) fn deadline_elapsed(&self) -> bool {
        self.stop
            .deadline
            .get()
            .is_some_and(|deadline| Instant::now() >= *deadline)
    }

    /// Record that a post-processing stage gave up its output because the time limit was reached.
    pub(crate) fn note_stage_cut(&self) {
        self.stop.stage_cut.store(true, Ordering::Relaxed);
    }

    /// Whether the returned document is partial: a page loop left pages unread or a post-processing
    /// stage dropped its output. Such a document must not be cached.
    pub(crate) fn result_is_partial(&self) -> bool {
        self.cut_short_by_time_limit() || self.stop.stage_cut.load(Ordering::Relaxed)
    }

    /// Returns `true` once [`request_stop`](Self::request_stop) was called on this extraction's token.
    pub(crate) fn stop_requested(&self) -> bool {
        self.stop.requested.load(Ordering::Relaxed)
    }

    /// Mark a page loop that honours [`stop_requested`](Self::stop_requested) as running, so the
    /// timeout wrapper knows that waiting for it can return a partial result. The mark ends when
    /// the guard is dropped, so a timeout in a later stage does not wait for a loop that is over.
    #[cfg(any(test, feature = "pdf"))]
    pub(crate) fn enter_page_loop(&self) -> PageLoopGuard {
        self.stop.active_page_loops.fetch_add(1, Ordering::Relaxed);
        PageLoopGuard(Arc::clone(&self.stop))
    }

    #[cfg(any(test, all(feature = "tokio-runtime", not(target_arch = "wasm32"))))]
    pub(crate) fn page_loop_running(&self) -> bool {
        self.stop.active_page_loops.load(Ordering::Relaxed) > 0
    }

    /// Record that a page loop stopped before reading `page` (1-based). The lowest page wins.
    #[cfg(any(test, feature = "pdf"))]
    pub(crate) fn note_unread_page(&self, page: u32) {
        self.stop.first_unread_page.fetch_min(page, Ordering::Relaxed);
    }

    /// Whether the extraction time limit cut a page loop short: the stop was requested and a loop left
    /// pages unread. The stages after such a loop only decorate the pages already read, so they give way
    /// to returning them inside the grace period.
    pub(crate) fn cut_short_by_time_limit(&self) -> bool {
        self.stop_requested() && self.first_unread_page().is_some()
    }

    /// The lowest page (1-based) a stopped page loop did not read, if any loop stopped early.
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
        let token = Self::new();
        if cancelled {
            token.cancel();
        }
        Ok(token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(all(feature = "office", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    #[tokio::test]
    async fn hard_cancellation_wakes_waiters_on_all_scoped_clones() {
        use std::future::{Future, poll_fn};
        use std::time::Duration;
        use tokio::sync::oneshot;

        let spawn_waiter = |token: CancellationToken| {
            let (registered, waiting) = oneshot::channel();
            let task = tokio::spawn(async move {
                let mut registered = Some(registered);
                let mut cancelled = std::pin::pin!(token.cancelled());
                poll_fn(|cx| {
                    let result = cancelled.as_mut().poll(cx);
                    if result.is_pending()
                        && let Some(registered) = registered.take()
                    {
                        registered.send(()).unwrap();
                    }
                    result
                })
                .await;
            });
            (task, waiting)
        };

        let token = CancellationToken::new();
        let scope = token.scoped();
        let (first, first_waiting) = spawn_waiter(token.clone());
        let (second, second_waiting) = spawn_waiter(scope.clone());
        tokio::time::timeout(Duration::from_secs(1), async {
            tokio::try_join!(first_waiting, second_waiting).unwrap();
        })
        .await
        .unwrap();
        assert!(scope.cancel_for_timeout());
        assert!(!token.cancel_for_timeout());
        tokio::time::timeout(Duration::from_secs(1), async {
            tokio::try_join!(first, second).unwrap();
        })
        .await
        .expect("registered tasks must be woken by hard cancellation");
        tokio::time::timeout(Duration::from_secs(1), token.cancelled())
            .await
            .unwrap();
    }

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

    #[test]
    fn a_scoped_token_shares_cancellation_but_not_the_soft_stop() {
        let caller = CancellationToken::new();
        let scoped = caller.scoped();
        scoped.request_stop();
        scoped.note_unread_page(3);

        assert!(scoped.stop_requested());
        assert!(!caller.stop_requested(), "the caller's token must not carry the stop");
        assert_eq!(caller.first_unread_page(), None);

        caller.cancel();
        assert!(scoped.is_cancelled());
        let next_extraction = caller.scoped();
        assert!(!next_extraction.stop_requested());
    }

    #[test]
    fn a_page_loop_is_running_until_its_guard_drops() {
        let token = CancellationToken::new();
        assert!(!token.page_loop_running());

        let guard = token.enter_page_loop();
        let second = token.enter_page_loop();
        assert!(token.page_loop_running());
        drop(guard);
        assert!(token.page_loop_running());
        drop(second);
        assert!(!token.page_loop_running());
    }

    #[test]
    fn the_lowest_unread_page_wins() {
        let token = CancellationToken::new();
        token.note_unread_page(9);
        token.note_unread_page(4);
        token.note_unread_page(6);
        assert_eq!(token.first_unread_page(), Some(4));
    }

    #[test]
    fn a_time_limit_cuts_a_page_loop_short_only_with_a_stop_and_an_unread_page() {
        let token = CancellationToken::new();
        assert!(!token.cut_short_by_time_limit());

        token.request_stop();
        assert!(
            !token.cut_short_by_time_limit(),
            "a stop that left no page unread cut nothing short"
        );

        token.note_unread_page(5);
        assert!(token.cut_short_by_time_limit());

        let unstopped = CancellationToken::new();
        unstopped.note_unread_page(5);
        assert!(!unstopped.cut_short_by_time_limit());
    }

    #[test]
    fn the_time_limit_is_reached_by_a_stop_request_or_a_passed_deadline_only() {
        let token = CancellationToken::new();
        assert!(!token.time_limit_reached(), "no deadline and no stop is no limit");

        token.set_deadline(Instant::now() + std::time::Duration::from_secs(3600));
        assert!(!token.time_limit_reached());

        let past = CancellationToken::new();
        past.set_deadline(Instant::now());
        assert!(past.time_limit_reached(), "a passed deadline needs no stop request");

        let stopped = CancellationToken::new();
        stopped.request_stop();
        assert!(stopped.time_limit_reached());
    }

    #[test]
    fn a_stage_cut_makes_the_result_partial_even_when_no_page_was_left_unread() {
        let token = CancellationToken::new();
        assert!(!token.result_is_partial());

        token.note_stage_cut();
        assert!(token.result_is_partial());
        assert!(!token.cut_short_by_time_limit(), "the page loops were not cut short");

        let unread = CancellationToken::new();
        unread.request_stop();
        unread.note_unread_page(2);
        assert!(unread.result_is_partial());
    }
}
