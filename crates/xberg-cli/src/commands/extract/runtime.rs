//! Tokio runtime construction for extraction work.

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
pub(crate) const RUNTIME_WORKER_STACK_SIZE_BYTES: usize = 16 * 1024 * 1024;

#[cfg(test)]
thread_local! {
    static RUNTIMES_BUILT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Build the single runtime that drives one extraction call.
///
/// `xberg::extract*` hands the whole extraction to the core library's process-global
/// multi-thread runtime (`core::runtime::run_on_large_stack`), so this runtime only
/// awaits that task's `JoinHandle`. A current-thread runtime is enough and spawns no
/// worker threads, where a second multi-thread pool would add startup cost for nothing.
/// `concurrency.max_threads` keeps capping the Rayon and ONNX pools inside the core.
fn build_runtime() -> std::io::Result<tokio::runtime::Runtime> {
    #[cfg(test)]
    RUNTIMES_BUILT.with(|built| built.set(built.get() + 1));
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .thread_stack_size(RUNTIME_WORKER_STACK_SIZE_BYTES)
        .build()
}

pub(super) fn block_on_extract(input: ExtractInput, config: &ExtractionConfig) -> xberg::Result<ExtractionResult> {
    build_runtime()?
        .block_on(extract(input, config))
        .map_err(xberg::XbergError::into_cli_extraction_diagnostic)
}

pub(super) fn block_on_extract_batch(
    inputs: Vec<ExtractInputWithOptions>,
    config: &ExtractionConfig,
) -> xberg::Result<ExtractionResult> {
    build_runtime()?.block_on(extract_batch_with_input_options(inputs, config))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runtimes_built() -> usize {
        RUNTIMES_BUILT.with(std::cell::Cell::get)
    }

    #[test]
    fn build_runtime_is_current_thread() {
        let runtime = build_runtime().unwrap();

        assert_eq!(
            runtime.handle().runtime_flavor(),
            tokio::runtime::RuntimeFlavor::CurrentThread
        );
    }

    #[test]
    fn single_extraction_builds_exactly_one_cli_runtime() {
        let before = runtimes_built();
        let input = ExtractInput::from_bytes(b"hello".to_vec(), "text/plain", None);

        let _ = block_on_extract(input, &ExtractionConfig::default());

        assert_eq!(runtimes_built() - before, 1);
    }

    #[test]
    fn batch_extraction_builds_exactly_one_cli_runtime() {
        let before = runtimes_built();

        let _ = block_on_extract_batch(Vec::new(), &ExtractionConfig::default());

        assert_eq!(runtimes_built() - before, 1);
    }
}
