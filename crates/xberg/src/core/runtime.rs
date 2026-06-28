//! Shared process-wide Tokio runtime for synchronous wrappers around async work.
//!
//! Synchronous entry points (`extract_*_sync`, `embed_texts`, `rerank`, the
//! structured-extraction sync path) need to drive an async future to
//! completion. Building a fresh `new_current_thread` runtime per call is both
//! slow and unsafe: when that per-call [`tokio::runtime::Runtime`] is dropped
//! inside a caller's blocking context (e.g. a `spawn_blocking` task, or while a
//! parent runtime is being torn down) Tokio panics with
//! "Cannot drop a runtime in a context where blocking is not allowed".
//!
//! Reusing one lazily-initialized, never-dropped runtime avoids both problems:
//! no `Drop` ever runs, so the panic cannot occur, and runtime construction
//! happens at most once per process.

// Gated to its only callers — `embed_texts` (feature `embeddings`) and `rerank`
// (feature `reranker`); both imply `tokio-runtime`. Other feature sets that
// enable `tokio-runtime` without these (e.g. candle-only) don't reference it.
#[cfg(any(feature = "embeddings", feature = "reranker"))]
use once_cell::sync::OnceCell;

/// The shared runtime. Initialized on first use and intentionally never dropped
/// (it lives for the remainder of the process), so it can never trigger the
/// "drop a runtime in a blocking context" panic.
#[cfg(any(feature = "embeddings", feature = "reranker"))]
static GLOBAL_RUNTIME: OnceCell<tokio::runtime::Runtime> = OnceCell::new();

/// Returns a reference to the shared multi-thread Tokio runtime, building it on
/// first call.
///
/// Use this for every synchronous wrapper that needs to `block_on` a future
/// from a thread that is **not** already inside a Tokio runtime. Callers that
/// are already on a multi-thread runtime worker should instead use
/// [`tokio::task::block_in_place`] with the current [`tokio::runtime::Handle`].
///
/// # Errors
///
/// Returns an error if the runtime cannot be created (e.g. system resource
/// exhaustion).
#[cfg(any(feature = "embeddings", feature = "reranker"))]
pub(crate) fn global_runtime() -> crate::Result<&'static tokio::runtime::Runtime> {
    GLOBAL_RUNTIME.get_or_try_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|e| crate::XbergError::Plugin {
                message: format!("Failed to create global Tokio runtime: {e}"),
                plugin_name: "runtime".to_string(),
            })
    })
}
