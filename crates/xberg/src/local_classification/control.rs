use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::Instant;

use super::LocalClassificationOptions;

pub(super) fn check_control(options: &LocalClassificationOptions, started: Instant) -> crate::Result<()> {
    if options.cancel_token.as_ref().is_some_and(|token| token.is_cancelled()) {
        return Err(crate::XbergError::Cancelled);
    }
    if let Some(limit_ms) = options.deadline_ms {
        let elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        if elapsed_ms >= limit_ms {
            return Err(crate::XbergError::Timeout { elapsed_ms, limit_ms });
        }
    }
    Ok(())
}

#[derive(Default)]
pub(super) struct RequestState {
    cancelled: AtomicBool,
    active_run: Mutex<Option<Arc<ort::session::RunOptions>>>,
}

impl RequestState {
    pub(super) fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    pub(super) fn register_run_options(&self, options: Arc<ort::session::RunOptions>) -> crate::Result<()> {
        let mut active = self
            .active_run
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if self.is_cancelled() {
            options.terminate().map_err(native_control_error)?;
            return Err(crate::XbergError::Cancelled);
        }
        *active = Some(options);
        Ok(())
    }

    pub(super) fn cancel(&self) -> crate::Result<()> {
        let active = self
            .active_run
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        self.cancelled.store(true, Ordering::Release);
        if let Some(options) = active.as_ref() {
            options.terminate().map_err(native_control_error)?;
        }
        Ok(())
    }

    pub(super) fn clear_run_options(&self) {
        self.active_run
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
    }
}

pub(super) struct WorkerControl {
    pub(super) state: Arc<RequestState>,
    pub(super) options: LocalClassificationOptions,
    pub(super) started: Instant,
}

impl WorkerControl {
    pub(super) fn check(&self) -> crate::Result<()> {
        if self.state.is_cancelled() {
            return Err(crate::XbergError::Cancelled);
        }
        check_control(&self.options, self.started)
    }
}

fn native_control_error(error: ort::Error) -> crate::XbergError {
    crate::XbergError::Other(format!("Local classification termination failed: {error}"))
}

struct CancelOnDrop(Arc<RequestState>);

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        if let Err(error) = self.0.cancel() {
            tracing::error!(%error, "Failed to terminate dropped local classification request");
        }
    }
}

async fn wait_for_control(options: &LocalClassificationOptions, started: Instant) -> crate::XbergError {
    const CONTROL_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(5);
    loop {
        if let Err(error) = check_control(options, started) {
            return error;
        }
        tokio::time::sleep(CONTROL_POLL_INTERVAL).await;
    }
}

pub(super) async fn run_controlled<T, F>(
    semaphore: Arc<tokio::sync::Semaphore>,
    options: LocalClassificationOptions,
    started: Instant,
    task: F,
) -> crate::Result<T>
where
    T: Send + 'static,
    F: FnOnce(Arc<RequestState>) -> crate::Result<T> + Send + 'static,
{
    run_controlled_with_factory(semaphore, options, started, || task).await
}

pub(super) async fn run_controlled_with_factory<T, F, W>(
    semaphore: Arc<tokio::sync::Semaphore>,
    options: LocalClassificationOptions,
    started: Instant,
    factory: F,
) -> crate::Result<T>
where
    T: Send + 'static,
    F: FnOnce() -> W,
    W: FnOnce(Arc<RequestState>) -> crate::Result<T> + Send + 'static,
{
    check_control(&options, started)?;
    let permit = tokio::select! {
        permit = semaphore.acquire_owned() => permit
            .map_err(|error| super::policy::invalid(format!("Local classifier semaphore closed: {error}")))?,
        error = wait_for_control(&options, started) => return Err(error),
    };
    check_control(&options, started)?;
    let task = factory();
    let state = Arc::new(RequestState::default());
    let _cancel_on_drop = CancelOnDrop(Arc::clone(&state));
    let worker_state = Arc::clone(&state);
    let mut worker = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        if worker_state.is_cancelled() {
            return Err(crate::XbergError::Cancelled);
        }
        task(worker_state)
    });
    let result = tokio::select! {
        result = &mut worker => result,
        error = wait_for_control(&options, started) => {
            let termination = state.cancel();
            let _worker_result = worker.await.map_err(|error| crate::XbergError::Other(
                format!("Local classification worker failed during cancellation: {error}")))?;
            termination?;
            return Err(error);
        },
    };
    check_control(&options, started)?;
    result.map_err(|error| crate::XbergError::Other(format!("Local classification worker failed: {error}")))?
}
