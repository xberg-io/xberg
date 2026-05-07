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
/// use kreuzberg::core::config::ConcurrencyConfig;
///
/// let config = ConcurrencyConfig {
///     max_threads: Some(2),
/// };
/// ```
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
/// use kreuzberg::core::config::ConcurrencyConfig;
/// use kreuzberg::core::config::concurrency::resolve_thread_budget;
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

/// Initialize the global Rayon thread pool with the given budget.
///
/// Safe to call multiple times — only the first call takes effect (subsequent
/// calls are silently ignored).
///
/// # Example
///
/// ```ignore
/// use kreuzberg::core::config::concurrency::init_thread_pools;
///
/// init_thread_pools(4);
/// init_thread_pools(2); // no-op: pool already initialized
/// ```
pub(crate) fn init_thread_pools(budget: usize) {
    POOL_INIT.call_once(|| {
        #[cfg(not(target_arch = "wasm32"))]
        rayon::ThreadPoolBuilder::new().num_threads(budget).build_global().ok();
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
