//! Concurrency and thread pool configuration.

use std::sync::Once;

use serde::{Deserialize, Serialize};

/// Controls thread usage for constrained environments.
///
/// Set `max_threads` to cap all internal thread pools (Rayon, ONNX Runtime
/// intra-op) and batch concurrency to a single limit.
///
/// # Example
///
/// ```rust
/// use xberg::core::config::ConcurrencyConfig;
///
/// let config = ConcurrencyConfig {
///     max_threads: Some(2),
/// };
/// ```
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ConcurrencyConfig {
    /// Maximum number of threads for all internal thread pools.
    ///
    /// Caps Rayon global pool size, ONNX Runtime intra-op threads, and
    /// (when `max_concurrent_extractions` is unset) the batch concurrency
    /// semaphore. When `None`, system defaults are used.
    pub max_threads: Option<usize>,
}

static POOL_INIT: Once = Once::new();

/// Resolve the effective thread budget from config or auto-detection.
///
/// User-set `max_threads` takes priority. Otherwise auto-detects from `num_cpus`,
/// capped at 8 for sane defaults in serverless environments.
///
/// # Example
///
/// ```ignore
/// use xberg::core::config::ConcurrencyConfig;
/// use xberg::core::config::concurrency::resolve_thread_budget;
///
/// let config = ConcurrencyConfig { max_threads: Some(4) };
/// assert_eq!(resolve_thread_budget(Some(&config)), 4);
/// assert!(resolve_thread_budget(None) >= 1);
/// ```
pub(crate) fn resolve_thread_budget(config: Option<&ConcurrencyConfig>) -> usize {
    if let Some(n) = config.and_then(|c| c.max_threads) {
        return n.max(1);
    }
    num_cpus::get().min(8)
}

/// Resolve the batch concurrency limit, accounting for layout ONNX oversubscription.
///
/// Without layout, batch concurrency is just the thread budget (each extraction is
/// largely single-threaded on the hot path). With layout enabled, every concurrent
/// extraction builds ONNX sessions configured with [`resolve_thread_budget`] intra-op
/// threads. Running `budget` extractions concurrently therefore spawns `budget²`
/// compute threads (e.g. 8×8 = 64 on an 8-core host), thrashing the CPU and making
/// batch slower than serial single-file processing.
///
/// When layout is active this caps concurrency so `concurrency × intra_threads` stays
/// within the machine's core count. Single-file extraction does not go through the
/// batch path, so its latency is unaffected. An explicit
/// `max_concurrent_extractions` on the config always overrides this and is applied by
/// the caller.
///
/// # Example
///
/// ```ignore
/// use xberg::core::config::concurrency::resolve_batch_concurrency;
///
/// // Without layout: full thread budget is used for concurrency.
/// let plain = resolve_batch_concurrency(None, false);
/// // With layout: concurrency is capped so it does not oversubscribe ONNX threads.
/// let layout = resolve_batch_concurrency(None, true);
/// assert!(layout <= plain);
/// assert!(layout >= 1);
/// ```
pub(crate) fn resolve_batch_concurrency(config: Option<&ConcurrencyConfig>, layout_active: bool) -> usize {
    let budget = resolve_thread_budget(config);
    if !layout_active {
        return budget;
    }
    // Each concurrent layout extraction uses `budget` intra-op threads. Bound the
    // product `concurrency × budget` by the core count to avoid oversubscription.
    let cores = num_cpus::get().max(1);
    (cores / budget.max(1)).max(1).min(budget)
}

/// Initialize the global Rayon thread pool with the given budget.
///
/// Safe to call multiple times — only the first call takes effect (subsequent
/// calls are silently ignored).
///
/// # Example
///
/// ```ignore
/// use xberg::core::config::concurrency::init_thread_pools;
///
/// init_thread_pools(4);
/// init_thread_pools(2); // no-op: pool already initialized
/// ```
pub(crate) fn init_thread_pools(budget: usize) {
    POOL_INIT.call_once(|| {
        #[cfg(not(target_arch = "wasm32"))]
        if let Err(_err) = rayon::ThreadPoolBuilder::new().num_threads(budget).build_global() {
            // The global Rayon pool is already initialized — typically because the host
            // application (or another library) built it first, e.g. by calling
            // `rayon::ThreadPoolBuilder::new()...build_global()` before invoking xberg.
            // In that case xberg runs its parallel work on the existing (externally
            // provided) pool rather than silently failing. We surface this at debug level
            // instead of swallowing the error with `.ok()`. The configured `budget` only
            // takes effect when xberg wins the race and owns the global pool; a host
            // that wants a specific pool size should build the global pool itself first.
            tracing::debug!(
                budget,
                "global rayon pool already initialized; reusing the existing pool \
                 (xberg thread budget not applied)"
            );
        }
        #[cfg(target_arch = "wasm32")]
        let _ = budget;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_thread_budget_none() {
        let budget = resolve_thread_budget(None);
        assert!(budget >= 1);
        assert!(budget <= 8);
    }

    #[test]
    fn test_resolve_thread_budget_with_config() {
        let config = ConcurrencyConfig { max_threads: Some(4) };
        assert_eq!(resolve_thread_budget(Some(&config)), 4);
    }

    #[test]
    fn test_resolve_thread_budget_clamps_to_one() {
        let config = ConcurrencyConfig { max_threads: Some(0) };
        assert_eq!(resolve_thread_budget(Some(&config)), 1);
    }

    #[test]
    fn test_resolve_thread_budget_no_max() {
        let config = ConcurrencyConfig { max_threads: None };
        let budget = resolve_thread_budget(Some(&config));
        assert!(budget >= 1);
        assert!(budget <= 8);
    }

    #[test]
    fn test_batch_concurrency_without_layout_equals_budget() {
        // No layout: concurrency is the full thread budget.
        assert_eq!(resolve_batch_concurrency(None, false), resolve_thread_budget(None));
    }

    #[test]
    fn test_batch_concurrency_with_layout_does_not_exceed_no_layout() {
        // Layout must never raise concurrency above the plain budget, and stays >= 1.
        let plain = resolve_batch_concurrency(None, false);
        let layout = resolve_batch_concurrency(None, true);
        assert!(layout >= 1);
        assert!(layout <= plain, "layout concurrency {layout} exceeded plain {plain}");
    }

    #[test]
    fn test_batch_concurrency_with_layout_bounds_thread_product() {
        // concurrency × intra-op threads must stay within the core count so the
        // layout ONNX sessions do not oversubscribe the CPU.
        let config = ConcurrencyConfig { max_threads: Some(4) };
        let intra = resolve_thread_budget(Some(&config));
        let concurrency = resolve_batch_concurrency(Some(&config), true);
        assert!(concurrency >= 1);
        assert!(
            concurrency * intra <= num_cpus::get().max(intra),
            "product {} exceeded cores",
            concurrency * intra
        );
    }

    #[test]
    fn test_init_thread_pools_idempotent() {
        // Should not panic when called multiple times.
        init_thread_pools(2);
        init_thread_pools(4);
    }

    #[test]
    fn test_default() {
        let config = ConcurrencyConfig::default();
        assert!(config.max_threads.is_none());
    }

    #[test]
    fn test_serde_roundtrip() {
        let json = r#"{"max_threads": 2}"#;
        let config: ConcurrencyConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.max_threads, Some(2));

        let serialized = serde_json::to_string(&config).unwrap();
        let roundtripped: ConcurrencyConfig = serde_json::from_str(&serialized).unwrap();
        assert_eq!(roundtripped.max_threads, Some(2));
    }

    #[test]
    fn test_serde_empty() {
        let json = r#"{}"#;
        let config: ConcurrencyConfig = serde_json::from_str(json).unwrap();
        assert!(config.max_threads.is_none());
    }
}
