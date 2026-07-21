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

/// Internal worker/session allocation for one batch extraction.
#[cfg(all(
    not(target_arch = "wasm32"),
    any(
        test,
        feature = "tokio-runtime",
        feature = "late-interaction",
        feature = "reranker",
        feature = "sparse-embeddings"
    )
))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BatchExecutionPlan {
    pub workers: usize,
    pub thread_budget: usize,
}

/// Allocate batch workers and per-worker model threads without oversubscription.
///
/// Layout inference is intentionally limited to two workers because the retained
/// RT-DETR and TATR pools each contain two sessions. The total configured budget is
/// divided between those workers. `max_concurrent` is a ceiling, never an override
/// of the layout/RSS guard.
#[cfg(all(
    not(target_arch = "wasm32"),
    any(
        test,
        feature = "tokio-runtime",
        feature = "late-interaction",
        feature = "reranker",
        feature = "sparse-embeddings"
    )
))]
pub(crate) fn resolve_batch_execution_plan(
    config: Option<&ConcurrencyConfig>,
    layout_active: bool,
    input_count: usize,
    max_concurrent: Option<usize>,
) -> BatchExecutionPlan {
    const MAX_LAYOUT_WORKERS: usize = 2;

    let total_budget = resolve_thread_budget(config);
    let available_inputs = input_count.max(1);
    let workers = if layout_active {
        max_concurrent
            .unwrap_or(total_budget)
            .clamp(1, MAX_LAYOUT_WORKERS)
            .min(total_budget)
            .min(available_inputs)
    } else {
        max_concurrent
            .map_or(total_budget, |explicit| explicit.max(1))
            .min(available_inputs)
    }
    .max(1);
    let thread_budget = if layout_active {
        (total_budget / workers).max(1)
    } else {
        total_budget
    };

    debug_assert!(!layout_active || workers * thread_budget <= total_budget);
    BatchExecutionPlan { workers, thread_budget }
}

/// Resolve concurrency for model-level batches outside document extraction.
#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "late-interaction", feature = "reranker", feature = "sparse-embeddings")
))]
pub(crate) fn resolve_batch_concurrency(config: Option<&ConcurrencyConfig>, model_threads_active: bool) -> usize {
    let budget = resolve_thread_budget(config);
    if !model_threads_active {
        return budget;
    }
    let cores = num_cpus::get().max(1);
    (cores / budget).max(1).min(budget)
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
    #[cfg(not(target_arch = "wasm32"))]
    fn test_batch_plan_without_layout_uses_available_budget() {
        let budget = resolve_thread_budget(None);
        assert_eq!(
            resolve_batch_execution_plan(None, false, budget, None),
            BatchExecutionPlan {
                workers: budget,
                thread_budget: budget,
            }
        );
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_layout_batch_plan_table() {
        for (budget, workers, thread_budget) in [(1, 1, 1), (2, 2, 1), (4, 2, 2), (8, 2, 4)] {
            let config = ConcurrencyConfig {
                max_threads: Some(budget),
            };
            assert_eq!(
                resolve_batch_execution_plan(Some(&config), true, 16, None),
                BatchExecutionPlan { workers, thread_budget }
            );
        }
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_layout_batch_plan_respects_input_and_explicit_limits() {
        let config = ConcurrencyConfig { max_threads: Some(8) };
        assert_eq!(
            resolve_batch_execution_plan(Some(&config), true, 1, Some(8)),
            BatchExecutionPlan {
                workers: 1,
                thread_budget: 8,
            }
        );
        assert_eq!(
            resolve_batch_execution_plan(Some(&config), true, 8, Some(1)),
            BatchExecutionPlan {
                workers: 1,
                thread_budget: 8,
            }
        );
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_non_layout_batch_plan_preserves_explicit_worker_limit() {
        let config = ConcurrencyConfig { max_threads: Some(2) };
        let plan = resolve_batch_execution_plan(Some(&config), false, 8, Some(6));
        assert_eq!(plan.workers, 6);
        assert_eq!(plan.thread_budget, 2);
    }

    #[test]
    fn test_init_thread_pools_idempotent() {
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
