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

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
use once_cell::sync::OnceCell;

/// The shared runtime. Initialized on first use and intentionally never dropped
/// (it lives for the remainder of the process), so it can never trigger the
/// "drop a runtime in a blocking context" panic.
#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
static GLOBAL_RUNTIME: OnceCell<tokio::runtime::Runtime> = OnceCell::new();

/// Worker-thread stack size for the global runtime.
///
/// A deep extraction future (multi-stage OCR, a nested archive member) exceeds tokio's
/// ~2 MB default worker stack, and a stack overflow aborts the process with SIGBUS
/// instead of raising a catchable panic. `crates/xberg-node/src/lib.rs:62-64` raised its
/// pool to 16 MB for exactly this reason; this runtime had never been given the same
/// budget.
#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
const GLOBAL_RUNTIME_STACK_SIZE_BYTES: usize = 16 * 1024 * 1024;

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
#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
pub(crate) fn global_runtime() -> crate::Result<&'static tokio::runtime::Runtime> {
    GLOBAL_RUNTIME.get_or_try_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            // See GLOBAL_RUNTIME_STACK_SIZE_BYTES: the extraction future overflows
            // tokio's ~2 MB default worker stack and aborts rather than panicking.
            .thread_stack_size(GLOBAL_RUNTIME_STACK_SIZE_BYTES)
            .build()
            .map_err(|e| crate::XbergError::Plugin {
                message: format!("Failed to create global Tokio runtime: {e}"),
                plugin_name: "runtime".to_string(),
            })
    })
}

/// Run `future` as a task on the shared 16 MiB-stack runtime and await its result.
///
/// The caller's own thread may be a host thread of about 1 to 2 MB (an FFI caller) or a default tokio
/// worker of the same size, while the extraction poll chain needs more. The task keeps the caller's current
/// span and tracing subscriber, so `with_default` / `set_default` capture still sees its events; it does not
/// inherit the caller's other task-locals, so the future must establish any it needs.
///
/// Dropping the returned future aborts the task, which stops it at its next await point: a blocking call or
/// a long CPU-bound poll that is already running finishes first. A panic in the task resumes unwinding in
/// the caller. If the shared runtime cannot be created (thread exhaustion), the future runs on the caller's
/// stack instead, with a warning, so the call degrades rather than fails.
#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
pub(crate) async fn run_on_large_stack<T, F>(future: F) -> crate::Result<T>
where
    T: Send + 'static,
    F: std::future::Future<Output = T> + Send + 'static,
{
    use tracing::Instrument;
    use tracing::instrument::WithSubscriber;

    struct AbortOnDrop<T>(tokio::task::JoinHandle<T>);

    impl<T> Drop for AbortOnDrop<T> {
        fn drop(&mut self) {
            self.0.abort();
        }
    }

    let runtime = match global_runtime() {
        Ok(runtime) => runtime,
        Err(error) => {
            tracing::warn!(%error, "large-stack runtime unavailable; running on the caller's stack");
            return Ok(future.await);
        }
    };
    let mut task = AbortOnDrop(runtime.spawn(future.instrument(tracing::Span::current()).with_current_subscriber()));
    match (&mut task.0).await {
        Ok(value) => Ok(value),
        Err(error) if error.is_panic() => std::panic::resume_unwind(error.into_panic()),
        Err(_) => Err(crate::XbergError::Cancelled),
    }
}

#[cfg(all(test, feature = "tokio-runtime", not(target_arch = "wasm32")))]
mod large_stack_tests {
    use super::run_on_large_stack;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    #[tokio::test]
    async fn returns_the_task_output() {
        assert_eq!(run_on_large_stack(async { 7 }).await.unwrap(), 7);
    }

    #[tokio::test]
    async fn task_runs_on_a_thread_with_the_large_stack() {
        #[inline(never)]
        fn deep(depth: usize) -> usize {
            let frame = [depth as u8; 4096];
            if depth == 0 {
                0
            } else {
                deep(depth - 1) + usize::from(std::hint::black_box(frame)[0] & 1)
            }
        }
        // 4 MiB of frames: overflows a 2 MB worker, fits the 16 MiB runtime.
        assert!(run_on_large_stack(async { deep(1024) }).await.is_ok());
    }

    // Reverting to `.instrument(Span::current())` alone drops the event: the worker has no thread-local
    // dispatcher of its own. The interest-cache rebuilds follow `issue_282_telemetry_spans`. ~keep
    #[test]
    fn the_task_keeps_the_callers_thread_local_subscriber() {
        use std::io::Write;
        use std::sync::Mutex;

        #[derive(Clone)]
        struct Capture(Arc<Mutex<Vec<u8>>>);
        impl Write for Capture {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.0.lock().expect("log buffer poisoned").write_all(buf)?;
                Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        let buffer = Arc::new(Mutex::new(Vec::new()));
        let capture = Capture(Arc::clone(&buffer));
        let subscriber = tracing_subscriber::fmt()
            .with_ansi(false)
            .with_max_level(tracing::Level::INFO)
            .with_writer(move || capture.clone())
            .finish();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        tracing::callsite::rebuild_interest_cache();
        tracing::subscriber::with_default(subscriber, || {
            runtime
                .block_on(run_on_large_stack(async {
                    tracing::info!("emitted inside the large-stack task")
                }))
                .unwrap();
        });
        tracing::callsite::rebuild_interest_cache();

        let logged = String::from_utf8(buffer.lock().unwrap().clone()).unwrap();
        assert!(
            logged.contains("emitted inside the large-stack task"),
            "captured: {logged:?}"
        );
    }

    #[tokio::test]
    #[should_panic(expected = "boom from task")]
    async fn task_panic_resumes_in_the_caller() {
        let _ = run_on_large_stack(async { panic!("boom from task") }).await;
    }

    #[tokio::test]
    async fn dropping_the_future_aborts_the_task() {
        struct SetOnDrop(Arc<AtomicBool>);
        impl Drop for SetOnDrop {
            fn drop(&mut self) {
                self.0.store(true, Ordering::SeqCst);
            }
        }
        let started = Arc::new(tokio::sync::Notify::new());
        let dropped = Arc::new(AtomicBool::new(false));
        let (started_task, guard) = (Arc::clone(&started), SetOnDrop(Arc::clone(&dropped)));
        let outer = run_on_large_stack(async move {
            let _guard = guard;
            started_task.notify_one();
            std::future::pending::<()>().await;
        });
        let outcome = tokio::time::timeout(Duration::from_secs(10), async {
            tokio::select! {
                _ = outer => unreachable!("pending task cannot finish"),
                () = started.notified() => {}
            }
        })
        .await;
        assert!(outcome.is_ok());
        for _ in 0..200 {
            if dropped.load(Ordering::SeqCst) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        panic!("task kept running after the outer future was dropped");
    }
}
