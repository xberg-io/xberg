//! Private cache retention state shared only within one extraction operation (ADR 0008).

use std::cell::RefCell;
#[cfg(any(
    all(test, feature = "tokio-runtime", not(target_arch = "wasm32")),
    all(feature = "ocr", not(target_arch = "wasm32"))
))]
use std::collections::HashMap;
use std::future::Future;
#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
use std::path::Path;
#[cfg(any(
    all(test, feature = "tokio-runtime", not(target_arch = "wasm32")),
    all(feature = "ocr", not(target_arch = "wasm32"))
))]
use std::path::PathBuf;
use std::sync::Arc;
#[cfg(any(
    all(test, feature = "tokio-runtime", not(target_arch = "wasm32")),
    all(feature = "ocr", not(target_arch = "wasm32"))
))]
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(any(
    all(test, feature = "tokio-runtime", not(target_arch = "wasm32")),
    all(feature = "ocr", not(target_arch = "wasm32"))
))]
const MAX_MODEL_ROOTS: usize = 64;

#[derive(Clone, Default)]
pub(crate) struct Cacheability(
    Arc<Operation>,
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))] Option<crate::cancellation::CancellationToken>,
);

#[derive(Default)]
struct Operation {
    invalidated: AtomicBool,
    // Retry results carry model identity only in the OCR cache. ~keep
    outer_publication_blocked: AtomicBool,
    #[cfg(any(
        all(test, feature = "tokio-runtime", not(target_arch = "wasm32")),
        all(feature = "ocr", not(target_arch = "wasm32"))
    ))]
    revisions: Mutex<HashMap<PathBuf, blake3::Hash>>,
}

#[cfg(feature = "tokio-runtime")]
tokio::task_local! {
    static ASYNC_CONTEXT: Cacheability;
}

thread_local! {
    static BLOCKING_CONTEXT: RefCell<Option<Cacheability>> = const { RefCell::new(None) };
}

impl Cacheability {
    pub(crate) fn is_cacheable(&self) -> bool {
        !self.0.invalidated.load(Ordering::Acquire)
    }

    #[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
    pub(crate) fn invalidate(&self) {
        self.0.invalidated.store(true, Ordering::Release);
    }

    #[cfg(any(
        all(test, feature = "tokio-runtime", not(target_arch = "wasm32")),
        all(feature = "ocr", not(target_arch = "wasm32"))
    ))]
    fn observe(&self, root: PathBuf, revision: blake3::Hash) {
        let Ok(mut revisions) = self.0.revisions.lock() else {
            self.invalidate();
            tracing::warn!("Model observation state was poisoned; bypassing operation caches");
            return;
        };
        match revisions.get(&root) {
            Some(previous) if *previous != revision => self.invalidate(),
            Some(_) => {}
            None if revisions.len() >= MAX_MODEL_ROOTS => {
                self.invalidate();
                tracing::warn!("Model observation bound reached; bypassing operation caches");
            }
            None => {
                revisions.insert(root, revision);
            }
        }
    }
}

pub(crate) fn current() -> Option<Cacheability> {
    let blocking = BLOCKING_CONTEXT.with(|context| context.borrow().clone());
    #[cfg(feature = "tokio-runtime")]
    {
        blocking.or_else(|| ASYNC_CONTEXT.try_with(Clone::clone).ok())
    }
    #[cfg(not(feature = "tokio-runtime"))]
    {
        blocking
    }
}

pub(crate) fn is_cacheable() -> bool {
    current().is_none_or(|context| context.is_cacheable())
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
pub(crate) fn invalidate_current() {
    if let Some(context) = current() {
        context.invalidate();
    }
}

pub(crate) fn can_publish_outer_result() -> bool {
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    if ocr_stop_requested() {
        return false;
    }
    current()
        .is_none_or(|context| context.is_cacheable() && !context.0.outer_publication_blocked.load(Ordering::Acquire))
}

#[cfg(feature = "ocr")]
pub(crate) fn block_outer_result_publication() {
    if let Some(context) = current() {
        context.0.outer_publication_blocked.store(true, Ordering::Release);
    }
}

#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
pub(crate) async fn with_ocr_cancellation<F: Future>(
    token: Option<&crate::cancellation::CancellationToken>,
    future: F,
) -> F::Output {
    let Some(token) = token else {
        return future.await;
    };
    let mut context = current().unwrap_or_default();
    context.1 = Some(token.clone());
    ASYNC_CONTEXT
        .scope(context, async {
            let result = future.await;
            // The engine checks the shared operation after this per-input token leaves scope. ~keep
            if ocr_stop_requested() {
                block_outer_result_publication();
            }
            result
        })
        .await
}

#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
pub(crate) fn ocr_stop_requested() -> bool {
    current()
        .and_then(|context| context.1)
        .is_some_and(|token| token.is_cancelled() || token.time_limit_reached())
}

#[cfg(all(feature = "ocr", target_arch = "wasm32"))]
pub(crate) fn ocr_stop_requested() -> bool {
    false
}

#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
pub(crate) fn check_ocr_cancelled() -> crate::Result<()> {
    if current()
        .and_then(|context| context.1)
        .is_some_and(|token| token.is_cancelled())
    {
        return Err(crate::XbergError::Cancelled);
    }
    Ok(())
}

pub(crate) async fn scope<F: Future>(future: F) -> F::Output {
    #[cfg(feature = "tokio-runtime")]
    {
        ASYNC_CONTEXT.scope(current().unwrap_or_default(), future).await
    }
    #[cfg(not(feature = "tokio-runtime"))]
    {
        future.await
    }
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
pub(crate) fn inherit<F: Future>(future: F) -> impl Future<Output = F::Output> {
    use tracing::instrument::WithSubscriber;

    let context = current();
    // A spawned task runs on a worker with no thread-local dispatcher, so a caller scoped with
    // `with_default` / `set_default` would lose the spans of every spawned batch or OCR task.
    async move {
        match context {
            Some(context) => ASYNC_CONTEXT.scope(context, future).await,
            None => future.await,
        }
    }
    .with_current_subscriber()
}

#[cfg(any(
    all(test, feature = "tokio-runtime", not(target_arch = "wasm32")),
    all(feature = "ocr", not(target_arch = "wasm32"))
))]
struct BlockingScope(Option<Cacheability>);

#[cfg(any(
    all(test, feature = "tokio-runtime", not(target_arch = "wasm32")),
    all(feature = "ocr", not(target_arch = "wasm32"))
))]
impl Drop for BlockingScope {
    fn drop(&mut self) {
        BLOCKING_CONTEXT.with(|context| {
            context.replace(self.0.take());
        });
    }
}

#[cfg(any(
    all(test, feature = "tokio-runtime", not(target_arch = "wasm32")),
    all(feature = "ocr", not(target_arch = "wasm32"))
))]
pub(crate) fn blocking<F: FnOnce() -> T, T>(operation: F) -> impl FnOnce() -> T {
    let inherited = current();
    move || {
        let _restore = BlockingScope(BLOCKING_CONTEXT.with(|context| context.replace(inherited)));
        operation()
    }
}

#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
pub(crate) fn osd_revision(directory: &Path) -> crate::Result<blake3::Hash> {
    let result = read_osd_revision(directory);
    match &result {
        Ok((root, revision)) => {
            if let Some(context) = current() {
                context.observe(root.clone(), *revision);
            }
        }
        Err(_) => invalidate_current(),
    }
    result.map(|(_, revision)| revision)
}

#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
fn read_osd_revision(directory: &Path) -> crate::Result<(PathBuf, blake3::Hash)> {
    let root = std::fs::canonicalize(directory)?;
    let path = root.join("osd.traineddata");
    let revision = fingerprint_osd_file(&path, crate::cache::blake3_hash_file)?;
    Ok((root, revision))
}

#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
pub(crate) fn fingerprint_osd_file(
    path: &Path,
    read_hash: impl FnOnce(&Path) -> crate::Result<String>,
) -> crate::Result<blake3::Hash> {
    let mut hash = blake3::Hasher::new();
    match std::fs::metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            hash.update(b"missing-osd");
        }
        Err(error) => return Err(error.into()),
        Ok(metadata) if !metadata.is_file() => {
            hash.update(b"osd-is-not-file");
        }
        Ok(_) => {
            let resolved = std::fs::canonicalize(path)?;
            hash.update(b"present-osd");
            let name = resolved.as_os_str().as_encoded_bytes();
            hash.update(&(name.len() as u64).to_le_bytes());
            hash.update(name);
            hash.update(read_hash(&resolved)?.as_bytes());
        }
    }
    Ok(hash.finalize())
}

#[cfg(all(test, feature = "tokio-runtime", not(target_arch = "wasm32")))]
#[path = "cacheability/tests.rs"]
mod tests;
