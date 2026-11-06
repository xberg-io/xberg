//! Tokio runtime sizing and construction for extraction work.

use xberg::{
    ExtractInput, ExtractInputWithOptions, ExtractionConfig, ExtractionResult, extract,
    extract_batch_with_input_options,
};

const DEFAULT_RUNTIME_THREAD_LIMIT: usize = 8;
const MIN_RUNTIME_THREAD_COUNT: usize = 1;

fn runtime_worker_threads(config: &ExtractionConfig) -> usize {
    config
        .concurrency
        .as_ref()
        .and_then(|concurrency| concurrency.max_threads)
        .unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(usize::from)
                .unwrap_or(MIN_RUNTIME_THREAD_COUNT)
                .min(DEFAULT_RUNTIME_THREAD_LIMIT)
        })
        .max(MIN_RUNTIME_THREAD_COUNT)
}

/// Size the async scheduler to the number of documents that can run concurrently.
///
/// `concurrency.max_threads` remains the core's total CPU/Rayon budget; it is only
/// an upper bound here so the CLI does not create a second equally-sized worker pool.
fn batch_runtime_worker_threads(config: &ExtractionConfig, input_count: usize) -> usize {
    let total_cpu_budget = runtime_worker_threads(config);
    let available_inputs = input_count.max(MIN_RUNTIME_THREAD_COUNT);
    let document_workers = config
        .max_concurrent_extractions
        .unwrap_or(total_cpu_budget)
        .max(MIN_RUNTIME_THREAD_COUNT);

    total_cpu_budget.min(document_workers).min(available_inputs)
}

/// Worker-thread stack size for every runtime this crate builds.
///
/// The extraction future is deep: a multi-stage OCR or PDF pipeline overflows tokio's
/// default ~2 MB worker stack and aborts the process with SIGBUS rather than a catchable
/// panic. `crates/xberg-node/src/lib.rs:62-64` hit this first and raised its pool to 16 MB;
/// the CLI's own runtimes were never given the same budget, so the API and MCP servers ran
/// every request on a 2 MB worker.
pub(crate) const RUNTIME_WORKER_STACK_SIZE_BYTES: usize = 16 * 1024 * 1024;

fn build_runtime(config: &ExtractionConfig) -> std::io::Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(runtime_worker_threads(config))
        .enable_all()
        .thread_stack_size(RUNTIME_WORKER_STACK_SIZE_BYTES)
        .build()
}

pub(super) fn block_on_extract(input: ExtractInput, config: &ExtractionConfig) -> xberg::Result<ExtractionResult> {
    build_runtime(config)?.block_on(extract(input, config))
}

pub(super) fn block_on_extract_batch(
    inputs: Vec<ExtractInputWithOptions>,
    config: &ExtractionConfig,
) -> xberg::Result<ExtractionResult> {
    let worker_threads = batch_runtime_worker_threads(config, inputs.len());
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(worker_threads)
        .enable_all()
        .thread_stack_size(RUNTIME_WORKER_STACK_SIZE_BYTES)
        .build()?
        .block_on(extract_batch_with_input_options(inputs, config))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_worker_threads_honors_explicit_budget() {
        let config = ExtractionConfig {
            concurrency: Some(xberg::core::config::ConcurrencyConfig {
                max_threads: Some(3),
                max_concurrent_ocr: None,
            }),
            ..Default::default()
        };

        assert_eq!(runtime_worker_threads(&config), 3);
    }

    #[test]
    fn runtime_worker_threads_clamps_zero_budget() {
        let config = ExtractionConfig {
            concurrency: Some(xberg::core::config::ConcurrencyConfig {
                max_threads: Some(0),
                max_concurrent_ocr: None,
            }),
            ..Default::default()
        };

        assert_eq!(runtime_worker_threads(&config), MIN_RUNTIME_THREAD_COUNT);
    }

    #[test]
    fn runtime_worker_threads_caps_automatic_budget() {
        let worker_threads = runtime_worker_threads(&ExtractionConfig::default());

        assert!((MIN_RUNTIME_THREAD_COUNT..=DEFAULT_RUNTIME_THREAD_LIMIT).contains(&worker_threads));
    }

    #[test]
    fn batch_runtime_worker_threads_caps_b4_to_four_with_eight_thread_budget() {
        let config = ExtractionConfig {
            concurrency: Some(xberg::core::config::ConcurrencyConfig {
                max_threads: Some(8),
                max_concurrent_ocr: None,
            }),
            max_concurrent_extractions: Some(8),
            ..Default::default()
        };

        assert_eq!(batch_runtime_worker_threads(&config, 4), 4);
    }

    #[test]
    fn batch_runtime_worker_threads_uses_one_worker_for_zero_or_one_input() {
        let config = ExtractionConfig {
            concurrency: Some(xberg::core::config::ConcurrencyConfig {
                max_threads: Some(8),
                max_concurrent_ocr: None,
            }),
            max_concurrent_extractions: Some(4),
            ..Default::default()
        };

        assert_eq!(batch_runtime_worker_threads(&config, 0), MIN_RUNTIME_THREAD_COUNT);
        assert_eq!(batch_runtime_worker_threads(&config, 1), MIN_RUNTIME_THREAD_COUNT);
    }

    #[test]
    fn batch_runtime_worker_threads_respects_document_worker_limit() {
        let config = ExtractionConfig {
            concurrency: Some(xberg::core::config::ConcurrencyConfig {
                max_threads: Some(8),
                max_concurrent_ocr: None,
            }),
            max_concurrent_extractions: Some(2),
            ..Default::default()
        };

        assert_eq!(batch_runtime_worker_threads(&config, 6), 2);
    }

    #[test]
    fn batch_runtime_worker_threads_never_exceeds_total_cpu_budget() {
        let config = ExtractionConfig {
            concurrency: Some(xberg::core::config::ConcurrencyConfig {
                max_threads: Some(3),
                max_concurrent_ocr: None,
            }),
            max_concurrent_extractions: Some(8),
            ..Default::default()
        };

        assert_eq!(batch_runtime_worker_threads(&config, 6), 3);
    }
}
