//! Driving extraction work from the synchronous extract and batch commands.

use xberg::{
    ExtractInput, ExtractInputWithOptions, ExtractionConfig, ExtractionResult, extract,
    extract_batch_with_input_options,
};

/// Worker-thread stack size for every runtime this crate builds.
///
/// The extraction future is deep: a multi-stage OCR or PDF pipeline overflows tokio's
/// default ~2 MB worker stack and aborts the process with SIGBUS rather than a catchable
/// panic. `crates/xberg-node/src/lib.rs:62-64` hit this first and raised its pool to 16 MB;
/// the CLI's own runtimes were never given the same budget, so the API and MCP servers ran
/// every request on a 2 MB worker.
#[cfg(any(
    feature = "api",
    feature = "mcp",
    all(test, feature = "pdf-surface", any(feature = "pdf-ocr", feature = "ocr"))
))]
pub(crate) const RUNTIME_WORKER_STACK_SIZE_BYTES: usize = 16 * 1024 * 1024;

/// Drive one extraction call on the core library's shared runtime.
///
/// `xberg::extract*` runs the whole extraction as a task on that process-global 16 MiB-stack
/// multi-thread runtime (`core::runtime::run_on_large_stack`). A runtime of the CLI's own would
/// only await that task's `JoinHandle`, at the cost of one more worker thread, so the calling thread
/// blocks on the shared runtime instead. `block_in_place` stays available because that runtime is
/// multi-thread, and its first use sizes it from `concurrency.max_threads`.
fn block_on_shared<F: std::future::Future>(future: F, config: &ExtractionConfig) -> xberg::Result<F::Output> {
    xberg::core::runtime::block_on_shared_runtime(future, config.concurrency.as_ref())
}

pub(super) fn block_on_extract(input: ExtractInput, config: &ExtractionConfig) -> xberg::Result<ExtractionResult> {
    // ~keep Boxed in a frame of its own so the extract future is not copied through this one.
    block_on_shared(xberg::core::runtime::pin_boxed(|| extract(input, config)), config)?
        .map_err(xberg::XbergError::into_cli_extraction_diagnostic)
}

pub(super) fn block_on_extract_batch(
    inputs: Vec<ExtractInputWithOptions>,
    config: &ExtractionConfig,
) -> xberg::Result<ExtractionResult> {
    block_on_shared(extract_batch_with_input_options(inputs, config), config)?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_single_extractions_reuse_the_shared_runtime() {
        for _ in 0..2 {
            let input = ExtractInput::from_bytes(b"hello".to_vec(), "text/plain", None);
            let output = block_on_extract(input, &ExtractionConfig::default()).unwrap();
            assert_eq!(output.results.len(), 1);
            assert!(output.results[0].content.contains("hello"));
        }
    }

    #[test]
    fn batch_extraction_runs_on_the_shared_runtime() {
        let inputs = ["first", "second"]
            .map(|text| {
                ExtractInputWithOptions::new(ExtractInput::from_bytes(text.as_bytes().to_vec(), "text/plain", None))
            })
            .to_vec();
        let output = block_on_extract_batch(inputs, &ExtractionConfig::default()).unwrap();
        assert_eq!(output.results.len(), 2);
        assert!(output.results[0].content.contains("first"));
        assert!(output.results[1].content.contains("second"));
    }

    #[test]
    fn the_driver_allows_block_in_place() {
        let value = block_on_shared(
            async { tokio::task::block_in_place(|| 7) },
            &ExtractionConfig::default(),
        )
        .unwrap();
        assert_eq!(value, 7);
    }
}
