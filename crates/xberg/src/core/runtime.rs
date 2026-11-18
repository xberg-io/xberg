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

/// Keep direct calls on foreign runtimes inline: they may already hold the only blocking worker.
#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
#[allow(dead_code, reason = "Native builds may omit the blocking format extractors")]
pub(crate) fn current_global_runtime() -> Option<tokio::runtime::Handle> {
    let handle = tokio::runtime::Handle::try_current().ok()?;
    (GLOBAL_RUNTIME.get()?.handle().id() == handle.id()).then_some(handle)
}

#[cfg(any(
    all(feature = "tokio-runtime", not(target_arch = "wasm32")),
    feature = "office",
    feature = "excel",
    feature = "excel-wasm"
))]
pub(crate) fn check_blocking_cancelled(cancel: Option<&crate::cancellation::CancellationToken>) -> crate::Result<()> {
    if cancel.is_some_and(crate::cancellation::CancellationToken::is_cancelled) {
        return Err(crate::XbergError::Cancelled);
    }
    Ok(())
}

#[cfg(all(
    any(feature = "office", feature = "excel", feature = "excel-wasm"),
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]
pub(crate) struct OfficeBlockingPool {
    pub(crate) runtime: tokio::runtime::Runtime,
    pub(crate) admission: std::sync::Arc<tokio::sync::Semaphore>,
    #[cfg(test)]
    pub(crate) slots: usize,
}

#[cfg(all(
    any(feature = "office", feature = "excel", feature = "excel-wasm"),
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]
pub(crate) fn current_office_pool() -> Option<&'static OfficeBlockingPool> {
    current_global_runtime()?;
    static POOL: std::sync::LazyLock<Option<OfficeBlockingPool>> = std::sync::LazyLock::new(|| {
        let slots = std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);
        match tokio::runtime::Builder::new_current_thread()
            .thread_stack_size(GLOBAL_RUNTIME_STACK_SIZE_BYTES)
            .build()
        {
            Ok(runtime) => Some(OfficeBlockingPool {
                runtime,
                admission: std::sync::Arc::new(tokio::sync::Semaphore::new(slots)),
                #[cfg(test)]
                slots,
            }),
            Err(error) => {
                tracing::warn!(%error, "Office blocking pool unavailable; parsing inline");
                None
            }
        }
    });
    POOL.as_ref()
}

#[cfg(all(
    any(feature = "office", feature = "excel", feature = "excel-wasm"),
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]
pub(crate) async fn acquire_office_slot(
    admission: std::sync::Arc<tokio::sync::Semaphore>,
    cancel: Option<&crate::cancellation::CancellationToken>,
) -> crate::Result<tokio::sync::OwnedSemaphorePermit> {
    let acquire = admission.acquire_owned();
    let permit = if let Some(cancel) = cancel {
        tokio::select! {
            biased;
            () = cancel.cancelled() => return Err(crate::XbergError::Cancelled),
            permit = acquire => permit,
        }
    } else {
        acquire.await
    };
    let permit = permit.map_err(|_| crate::XbergError::Cancelled)?;
    check_blocking_cancelled(cancel)?;
    Ok(permit)
}

/// Run synchronous work on the selected pool, preserving tracing and typed cancellation.
///
/// Dropping the future aborts queued work. Work that has already started must cooperate
/// through the token or finish before releasing its resources.
#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
#[allow(dead_code, reason = "Native builds may omit the blocking format extractors")]
pub(crate) async fn run_blocking<T, F>(
    runtime: &tokio::runtime::Handle,
    cancel: Option<&crate::cancellation::CancellationToken>,
    work: F,
) -> crate::Result<T>
where
    T: Send + 'static,
    F: FnOnce(Option<&crate::cancellation::CancellationToken>) -> crate::Result<T> + Send + 'static,
{
    let cancel = cancel.cloned();
    let span = tracing::Span::current();
    let dispatcher = tracing::dispatcher::get_default(Clone::clone);
    let task = runtime.spawn_blocking(move || {
        let _dispatcher = tracing::dispatcher::set_default(&dispatcher);
        let _span = span.entered();
        check_blocking_cancelled(cancel.as_ref())?;
        let result = work(cancel.as_ref())?;
        check_blocking_cancelled(cancel.as_ref())?;
        Ok(result)
    });
    await_task(task).await?
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
async fn await_task<T>(task: tokio::task::JoinHandle<T>) -> crate::Result<T> {
    struct AbortOnDrop<T>(tokio::task::JoinHandle<T>);

    impl<T> Drop for AbortOnDrop<T> {
        fn drop(&mut self) {
            self.0.abort();
        }
    }

    let mut task = AbortOnDrop(task);
    match (&mut task.0).await {
        Ok(value) => Ok(value),
        Err(error) if error.is_panic() => std::panic::resume_unwind(error.into_panic()),
        Err(_) => Err(crate::XbergError::Cancelled),
    }
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

    let runtime = match global_runtime() {
        Ok(runtime) => runtime,
        Err(error) => {
            tracing::warn!(%error, "large-stack runtime unavailable; running on the caller's stack");
            return Ok(future.await);
        }
    };
    await_task(runtime.spawn(future.instrument(tracing::Span::current()).with_current_subscriber())).await
}

#[cfg(all(
    test,
    any(feature = "office", feature = "excel"),
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]
pub(crate) fn poll_with_blocking_worker_held<T>(
    runtime: &tokio::runtime::Runtime,
    read: impl std::future::Future<Output = T>,
    after_first_poll: impl FnOnce(),
) -> (bool, bool, T) {
    use std::future::poll_fn;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Mutex, mpsc};
    use std::task::Poll;
    use std::thread::{self, ThreadId};
    use std::time::Duration;
    use tracing::Instrument as _;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{Layer, layer::Context};

    struct Gate {
        caller: ThreadId,
        entered: AtomicBool,
        reached: mpsc::Sender<()>,
        release: Mutex<mpsc::Receiver<()>>,
    }

    impl<S: tracing::Subscriber> Layer<S> for Gate {
        fn on_enter(&self, _id: &tracing::span::Id, _context: Context<'_, S>) {
            if thread::current().id() != self.caller && !self.entered.swap(true, Ordering::Relaxed) {
                self.reached.send(()).unwrap();
                self.release
                    .lock()
                    .unwrap()
                    .recv_timeout(Duration::from_secs(5))
                    .unwrap();
            }
        }
    }

    let (release_tx, release_rx) = mpsc::sync_channel(1);
    let (reached_tx, reached_rx) = mpsc::channel();
    let subscriber = tracing_subscriber::registry().with(Gate {
        caller: thread::current().id(),
        entered: AtomicBool::new(false),
        reached: reached_tx,
        release: Mutex::new(release_rx),
    });
    tracing::callsite::rebuild_interest_cache();
    let result = tracing::subscriber::with_default(subscriber, || {
        runtime.block_on(async {
            let mut read = std::pin::pin!(read.instrument(tracing::info_span!("blocking_worker_test")));
            let first = poll_fn(|context| Poll::Ready(read.as_mut().poll(context))).await;
            let entered = first.is_pending() && reached_rx.recv_timeout(Duration::from_secs(5)).is_ok();
            after_first_poll();
            release_tx.send(()).unwrap();
            match first {
                Poll::Pending => (true, entered, read.await),
                Poll::Ready(result) => (false, false, result),
            }
        })
    });
    tracing::callsite::rebuild_interest_cache();
    result
}

#[cfg(all(test, feature = "tokio-runtime", not(target_arch = "wasm32")))]
mod large_stack_tests {
    use super::run_on_large_stack;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    #[tokio::test]
    async fn blocking_work_reports_cancellation_after_the_parser_returns() {
        let token = crate::cancellation::CancellationToken::new();
        let result = super::run_blocking(&tokio::runtime::Handle::current(), Some(&token), |cancel| {
            cancel.unwrap().cancel();
            Ok(17)
        })
        .await;
        assert!(matches!(result, Err(crate::XbergError::Cancelled)));
    }

    #[test]
    fn dropping_a_queued_blocking_worker_prevents_parser_entry() {
        use std::future::{Future, poll_fn};
        use std::sync::mpsc;
        use std::task::Poll;

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .max_blocking_threads(1)
            .build()
            .unwrap();
        let (entered, wait_entered) = mpsc::sync_channel(1);
        let (release, wait_release) = mpsc::sync_channel(1);
        let blocker = runtime.spawn_blocking(move || {
            entered.send(()).unwrap();
            wait_release.recv_timeout(Duration::from_secs(5)).unwrap();
        });
        wait_entered.recv_timeout(Duration::from_secs(5)).unwrap();

        let parsed = Arc::new(AtomicBool::new(false));
        let parse_started = Arc::clone(&parsed);
        runtime.block_on(async {
            let mut read = Box::pin(super::run_blocking(runtime.handle(), None, move |_| {
                parse_started.store(true, Ordering::SeqCst);
                Ok(())
            }));
            assert!(poll_fn(|cx| Poll::Ready(read.as_mut().poll(cx))).await.is_pending());
            drop(read);

            let drained = runtime.spawn_blocking(|| {});
            release.send(()).unwrap();
            tokio::time::timeout(Duration::from_secs(5), async {
                blocker.await.unwrap();
                drained.await.unwrap();
            })
            .await
            .unwrap();
        });
        assert!(!parsed.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn blocking_work_keeps_the_callers_thread_local_dispatcher_and_span() {
        use tracing::Instrument;
        use tracing::instrument::WithSubscriber;
        use tracing_subscriber::{Layer, layer::Context, prelude::*};

        struct Capture(Arc<AtomicBool>);
        impl<S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>> Layer<S> for Capture {
            fn on_event(&self, event: &tracing::Event<'_>, context: Context<'_, S>) {
                if event.metadata().target() == "test.office_worker" {
                    assert_eq!(context.event_span(event).unwrap().name(), "office_test");
                    self.0.store(true, Ordering::SeqCst);
                }
            }
        }

        let captured = Arc::new(AtomicBool::new(false));
        let subscriber = tracing_subscriber::registry().with(Capture(Arc::clone(&captured)));
        async {
            super::run_blocking(&tokio::runtime::Handle::current(), None, |_| {
                tracing::info!(target: "test.office_worker", "event from the blocking parser");
                Ok(())
            })
            .instrument(tracing::info_span!("office_test"))
            .await
            .unwrap();
        }
        .with_subscriber(subscriber)
        .await;
        assert!(captured.load(Ordering::SeqCst));
    }

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
