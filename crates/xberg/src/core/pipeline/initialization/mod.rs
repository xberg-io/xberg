//! Pipeline initialization and setup logic.
//!
//! This module handles the initialization of features and processor cache
//! required for pipeline execution.

use crate::Result;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{LazyLock, Mutex, RwLock};

use super::cache::{PROCESSOR_CACHE, ProcessorCache};

/// Records the outcome of the latest built-in post-processor registration pass
/// so callers can learn when registration was incomplete.
static BUILTIN_REGISTRATION_ERROR: RwLock<Option<String>> = RwLock::new(None);
#[cfg(test)]
static FORCED_BUILTIN_REGISTRATION_ERROR: RwLock<Option<String>> = RwLock::new(None);
static BUILTIN_REGISTRATION_REQUIRED: AtomicBool = AtomicBool::new(true);
/// ~keep Odd epochs bracket registry mutation; cache snapshots publish only across one unchanged
/// even epoch, so public clear cannot expose its intentionally empty intermediate registry.
static BUILTIN_REGISTRATION_EPOCH: AtomicU64 = AtomicU64::new(0);
static BUILTIN_REGISTRATION_LOCK: Mutex<()> = Mutex::new(());
/// ~keep A validated snapshot holds a lease through processor execution; lifecycle mutations
/// fail as retryable while a lease is active, so shutdown never overlaps processing or blocks an async worker.
/// Wrapped in an `Arc` (rather than a bare `AtomicUsize`) so [`ProcessorSnapshotLease`] can hold
/// the same counter type whether it is leasing the global registry or an isolated, per-test
/// [`ProcessorRegistryState`] (`#[cfg(test)]`), instead of every snapshot path being hard-wired to
/// this one static. ~keep
static ACTIVE_PROCESSOR_SNAPSHOTS: LazyLock<std::sync::Arc<AtomicUsize>> =
    LazyLock::new(|| std::sync::Arc::new(AtomicUsize::new(0)));
static AUTOMATIC_REGISTRATION_SUPPRESSIONS: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

#[cfg(test)]
type InitializationHook = Box<dyn FnOnce() + Send>;
#[cfg(test)]
type InitializationRetryHook = Box<dyn FnMut() + Send>;
#[cfg(test)]
std::thread_local! {
    static BEFORE_REGISTRATION_CHECK_HOOK: std::cell::RefCell<Option<InitializationHook>> =
        std::cell::RefCell::new(None);
    static AFTER_FEATURE_INITIALIZATION_HOOK: std::cell::RefCell<Option<InitializationHook>> =
        std::cell::RefCell::new(None);
    static REGISTRATION_RETRY_HOOK: std::cell::RefCell<Option<InitializationRetryHook>> =
        std::cell::RefCell::new(None);
    static BEFORE_PROCESSOR_SNAPSHOT_HOOK: std::cell::RefCell<Option<InitializationHook>> =
        std::cell::RefCell::new(None);
    static BEFORE_BLOCKING_CACHE_INITIALIZATION_HOOK: std::cell::RefCell<Option<InitializationHook>> =
        std::cell::RefCell::new(None);
    static AFTER_PROCESSOR_SNAPSHOT_VALIDATED_HOOK: std::cell::RefCell<Option<InitializationHook>> =
        std::cell::RefCell::new(None);
    static AFTER_REGISTRATION_UPDATE_BEGAN_HOOK: std::cell::RefCell<Option<InitializationHook>> =
        std::cell::RefCell::new(None);
}

const REGISTRATION_UPDATE_BIT: u64 = 1;
const AUTOMATIC_POST_PROCESSOR_NAMES: &[&str] = &[
    "page-classification",
    "chunk-classification",
    "summarization",
    "translation",
    "captioning",
    "qr-codes",
    "ner",
    "redaction",
    "quality-processing",
    "keyword-extraction",
];

struct RegistrationUpdate;
struct ProcessorSnapshotLease(std::sync::Arc<AtomicUsize>);

pub(super) struct ProcessorSnapshot {
    pub(super) early: std::sync::Arc<Vec<std::sync::Arc<dyn crate::plugins::PostProcessor>>>,
    pub(super) middle: std::sync::Arc<Vec<std::sync::Arc<dyn crate::plugins::PostProcessor>>>,
    pub(super) late: std::sync::Arc<Vec<std::sync::Arc<dyn crate::plugins::PostProcessor>>>,
    _lease: ProcessorSnapshotLease,
}

impl RegistrationUpdate {
    fn begin() -> Self {
        BUILTIN_REGISTRATION_EPOCH.fetch_add(1, Ordering::SeqCst);
        Self
    }
}

impl Drop for RegistrationUpdate {
    fn drop(&mut self) {
        BUILTIN_REGISTRATION_EPOCH.fetch_add(1, Ordering::SeqCst);
    }
}

impl ProcessorSnapshotLease {
    fn acquire(active_snapshots: &std::sync::Arc<AtomicUsize>) -> Self {
        active_snapshots.fetch_add(1, Ordering::SeqCst);
        Self(std::sync::Arc::clone(active_snapshots))
    }
}

impl Drop for ProcessorSnapshotLease {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

fn registration_update_in_progress(epoch: u64) -> bool {
    epoch & REGISTRATION_UPDATE_BIT != 0
}

fn wait_for_registration_update() {
    let _registration_guard = BUILTIN_REGISTRATION_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
}

fn with_registration_update<T>(update: impl FnOnce() -> Result<T>) -> Result<T> {
    let _registration_guard = BUILTIN_REGISTRATION_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let _registration_update = RegistrationUpdate::begin();
    // Read the lease count BEFORE the hook, not after. The hook is how a test learns this
    // mutation has started, and its only caller then releases the parked pipeline that holds
    // the lease. Loading afterwards let that release win the race under CI contention: the
    // mutation observed a count of 0 and succeeded, failing an assertion that it be rejected.
    // Sampling first fixes the outcome while the lease is still provably held. Production is
    // unaffected -- the hook compiles away outside `cfg(test)`. ~keep
    let registry_in_use = ACTIVE_PROCESSOR_SNAPSHOTS.load(Ordering::SeqCst) != 0;
    #[cfg(test)]
    run_after_registration_update_began_hook();
    if registry_in_use {
        return Err(crate::XbergError::Other(
            "post-processor registry is in use by an active extraction; retry the lifecycle mutation after extraction completes"
                .to_string(),
        ));
    }
    update()
}

#[cfg(test)]
fn run_before_registration_check_hook() {
    let hook = BEFORE_REGISTRATION_CHECK_HOOK.with(|slot| slot.borrow_mut().take());
    if let Some(hook) = hook {
        hook();
    }
}

#[cfg(test)]
fn run_after_feature_initialization_hook() {
    let hook = AFTER_FEATURE_INITIALIZATION_HOOK.with(|slot| slot.borrow_mut().take());
    if let Some(hook) = hook {
        hook();
    }
}

#[cfg(test)]
fn run_registration_retry_hook() {
    REGISTRATION_RETRY_HOOK.with(|slot| {
        if let Some(hook) = slot.borrow_mut().as_mut() {
            hook();
        }
    });
}

#[cfg(test)]
fn run_before_processor_snapshot_hook() {
    let hook = BEFORE_PROCESSOR_SNAPSHOT_HOOK.with(|slot| slot.borrow_mut().take());
    if let Some(hook) = hook {
        hook();
    }
}

#[cfg(test)]
fn run_before_blocking_cache_initialization_hook() {
    let hook = BEFORE_BLOCKING_CACHE_INITIALIZATION_HOOK.with(|slot| slot.borrow_mut().take());
    if let Some(hook) = hook {
        hook();
    }
}

#[cfg(test)]
fn run_after_processor_snapshot_validated_hook() {
    let hook = AFTER_PROCESSOR_SNAPSHOT_VALIDATED_HOOK.with(|slot| slot.borrow_mut().take());
    if let Some(hook) = hook {
        hook();
    }
}

#[cfg(test)]
fn run_after_registration_update_began_hook() {
    let hook = AFTER_REGISTRATION_UPDATE_BEGAN_HOOK.with(|slot| slot.borrow_mut().take());
    if let Some(hook) = hook {
        hook();
    }
}

/// ~keep A public registry clear deliberately removes custom and built-in processors, while a
/// named unregister must remain effective. Clear and recovery share the registration mutex and
/// epoch so concurrent cache snapshots reject the intentionally empty intermediate registry.
pub(crate) fn with_builtin_registration_recovery<T>(clear: impl FnOnce() -> Result<T>) -> Result<T> {
    with_registration_update(|| {
        AUTOMATIC_REGISTRATION_SUPPRESSIONS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clear();
        BUILTIN_REGISTRATION_REQUIRED.store(true, Ordering::Release);
        clear()
    })
}

pub(crate) fn with_post_processor_suppressed<T>(name: &str, remove: impl FnOnce() -> Result<T>) -> Result<T> {
    with_registration_update(|| {
        if AUTOMATIC_POST_PROCESSOR_NAMES.contains(&name) {
            AUTOMATIC_REGISTRATION_SUPPRESSIONS
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .insert(name.to_string());
        }
        remove()
    })
}

pub(crate) fn with_post_processor_enabled<T>(name: &str, register: impl FnOnce() -> Result<T>) -> Result<T> {
    with_registration_update(|| {
        let is_automatic = AUTOMATIC_POST_PROCESSOR_NAMES.contains(&name);
        let was_suppressed = AUTOMATIC_REGISTRATION_SUPPRESSIONS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(name);
        let result = register();
        if result.is_err() && is_automatic {
            restore_registration_state_after_failure(name, was_suppressed);
        }
        result
    })
}

fn restore_registration_state_after_failure(name: &str, was_suppressed: bool) {
    if was_suppressed {
        AUTOMATIC_REGISTRATION_SUPPRESSIONS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(name.to_string());
    } else {
        BUILTIN_REGISTRATION_REQUIRED.store(true, Ordering::Release);
    }
}

#[cfg(any(
    feature = "classification",
    feature = "summarization",
    feature = "translation",
    feature = "captioning",
    feature = "qr-codes",
    feature = "ner",
    feature = "redaction",
    feature = "quality",
    feature = "keywords-yake",
    feature = "keywords-rake"
))]
pub(crate) fn automatic_registration_allowed(name: &str) -> bool {
    !AUTOMATIC_REGISTRATION_SUPPRESSIONS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .contains(name)
}

/// The error message from the built-in post-processor registration pass, if any
/// processor failed to register. `None` means either registration has not run
/// yet or every enabled processor registered successfully.
///
/// Called from `pipeline::mod::run_pipeline` (#271), which pushes a
/// `ProcessingWarning` naming this message whenever it is `Some(_)` — the
/// aggregate counterpart to the captioning-only "processor missing" warning at
/// the call site of `run_captioning_prepass`.
pub(crate) fn builtin_registration_error() -> Option<String> {
    #[cfg(test)]
    if let Some(error) = FORCED_BUILTIN_REGISTRATION_ERROR
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
    {
        return Some(error);
    }

    BUILTIN_REGISTRATION_ERROR
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

/// Test-only access to `BUILTIN_REGISTRATION_ERROR` for exercising the
/// `builtin_registration_error()` call site inside `pipeline::mod` without
/// racing the real registration pass.
#[cfg(test)]
pub(crate) mod test_support {
    use super::FORCED_BUILTIN_REGISTRATION_ERROR;
    #[cfg(feature = "tokio-runtime")]
    use super::{
        AFTER_PROCESSOR_SNAPSHOT_VALIDATED_HOOK, AFTER_REGISTRATION_UPDATE_BEGAN_HOOK, BEFORE_PROCESSOR_SNAPSHOT_HOOK,
        InitializationHook,
    };

    /// Set (or clear, with `None`) the recorded registration error. The static is
    /// process-global, so callers must restore it (typically to `None`) when done.
    pub(crate) fn set_registration_error(value: Option<String>) {
        let mut slot = FORCED_BUILTIN_REGISTRATION_ERROR
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *slot = value;
    }

    #[cfg(feature = "tokio-runtime")]
    pub(crate) fn set_before_processor_snapshot_hook(hook: InitializationHook) {
        BEFORE_PROCESSOR_SNAPSHOT_HOOK.with(|slot| *slot.borrow_mut() = Some(hook));
    }

    #[cfg(all(feature = "tokio-runtime", feature = "quality", feature = "summarization"))]
    pub(crate) fn set_before_blocking_cache_initialization_hook(hook: Option<InitializationHook>) {
        super::BEFORE_BLOCKING_CACHE_INITIALIZATION_HOOK.with(|slot| *slot.borrow_mut() = hook);
    }

    #[cfg(feature = "tokio-runtime")]
    pub(crate) fn set_after_processor_snapshot_validated_hook(hook: InitializationHook) {
        AFTER_PROCESSOR_SNAPSHOT_VALIDATED_HOOK.with(|slot| *slot.borrow_mut() = Some(hook));
    }

    #[cfg(feature = "tokio-runtime")]
    pub(crate) fn set_after_registration_update_began_hook(hook: InitializationHook) {
        AFTER_REGISTRATION_UPDATE_BEGAN_HOOK.with(|slot| *slot.borrow_mut() = Some(hook));
    }
}

/// Type alias for processor stages tuple (Early, Middle, Late).
type ProcessorStages = (
    std::sync::Arc<Vec<std::sync::Arc<dyn crate::plugins::PostProcessor>>>,
    std::sync::Arc<Vec<std::sync::Arc<dyn crate::plugins::PostProcessor>>>,
    std::sync::Arc<Vec<std::sync::Arc<dyn crate::plugins::PostProcessor>>>,
);

/// Initialize feature-specific systems that may be needed during pipeline execution.
pub(super) fn initialize_features() {
    #[cfg(test)]
    run_before_registration_check_hook();

    if !BUILTIN_REGISTRATION_REQUIRED.load(Ordering::Acquire) {
        return;
    }

    let _registration_guard = BUILTIN_REGISTRATION_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if !BUILTIN_REGISTRATION_REQUIRED.load(Ordering::Acquire) {
        return;
    }

    let _registration_update = RegistrationUpdate::begin();
    let mut failures = Vec::new();
    #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
    record_registration_result(
        &mut failures,
        "keyword-extraction",
        crate::keywords::ensure_initialized(),
    );
    record_registration_result(
        &mut failures,
        "quality-processing",
        register_quality_processor_if_missing(),
    );
    record_registration_result(
        &mut failures,
        "built-in post-processors",
        crate::plugins::processor::builtin::register_builtin(),
    );
    let registration_error = aggregate_registration_error(failures);
    let mut slot = BUILTIN_REGISTRATION_ERROR
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *slot = registration_error;
    let registration_complete = slot.is_none();
    drop(slot);
    BUILTIN_REGISTRATION_REQUIRED.store(!registration_complete, Ordering::Release);
}

fn record_registration_result(failures: &mut Vec<String>, name: &str, result: Result<()>) {
    if let Err(error) = result {
        tracing::error!(processor = name, %error, "Automatic post-processor registration failed");
        failures.push(format!("{name}: {error}"));
    }
}

fn aggregate_registration_error(failures: Vec<String>) -> Option<String> {
    (!failures.is_empty()).then(|| format!("automatic post-processor registration failed: {}", failures.join("; ")))
}

#[cfg(feature = "quality")]
fn register_quality_processor_if_missing() -> Result<()> {
    crate::plugins::processor::register_post_processor_if_absent(std::sync::Arc::new(crate::text::QualityProcessor))
        .map(|_| ())
}

#[cfg(not(feature = "quality"))]
fn register_quality_processor_if_missing() -> Result<()> {
    Ok(())
}

/// Initialize the processor cache if not already initialized, or rebuild it if
/// the post-processor registry has changed since it was last built.
///
/// #215: the cache used to populate once on the first pipeline run and never
/// again, so a post-processor registered (or removed) after that first run was
/// silently invisible unless a caller happened to know about and call
/// `clear_processor_cache()`. Comparing the registry's live generation against
/// the generation recorded in the cached snapshot makes this self-correcting.
pub(super) fn initialize_processor_cache() -> Result<()> {
    loop {
        initialize_features();
        #[cfg(test)]
        run_after_feature_initialization_hook();
        let registration_epoch = BUILTIN_REGISTRATION_EPOCH.load(Ordering::SeqCst);
        if !registration_update_in_progress(registration_epoch) && try_initialize_processor_cache(registration_epoch)? {
            return Ok(());
        }
        #[cfg(test)]
        run_registration_retry_hook();
        wait_for_registration_update();
    }
}

#[cfg(feature = "tokio-runtime")]
pub(super) async fn initialize_processor_cache_for_async_pipeline() -> Result<ProcessorSnapshot> {
    if let Some(stages) = try_get_processor_snapshot(true) {
        return Ok(stages);
    }
    #[cfg(test)]
    run_before_blocking_cache_initialization_hook();
    let Ok(runtime) = tokio::runtime::Handle::try_current() else {
        return initialize_processor_stages();
    };
    runtime
        .spawn_blocking(initialize_processor_stages)
        .await
        .map_err(|error| crate::XbergError::Other(format!("processor cache task failed to join: {error}")))?
}

#[cfg(not(feature = "tokio-runtime"))]
pub(super) async fn initialize_processor_cache_for_async_pipeline() -> Result<ProcessorSnapshot> {
    if let Some(stages) = try_get_processor_snapshot(true) {
        return Ok(stages);
    }
    #[cfg(test)]
    run_before_blocking_cache_initialization_hook();
    initialize_processor_stages()
}

fn initialize_processor_stages() -> Result<ProcessorSnapshot> {
    loop {
        initialize_processor_cache()?;
        if let Some(stages) = try_get_processor_snapshot(false) {
            return Ok(stages);
        }
        wait_for_registration_update();
    }
}

fn try_initialize_processor_cache(registration_epoch: u64) -> Result<bool> {
    let current_generation = crate::plugins::registry::get_post_processor_registry()
        .read()
        .generation();
    if BUILTIN_REGISTRATION_EPOCH.load(Ordering::SeqCst) != registration_epoch {
        return Ok(false);
    }

    let mut cache_lock = PROCESSOR_CACHE.write();
    let is_stale = cache_lock
        .as_ref()
        .is_some_and(|cache| cache.generation != current_generation);

    if cache_lock.is_none() || is_stale {
        let candidate = ProcessorCache::new(registration_epoch)?;
        if BUILTIN_REGISTRATION_EPOCH.load(Ordering::SeqCst) != registration_epoch {
            return Ok(false);
        }
        *cache_lock = Some(candidate);
    } else if let Some(cache) = cache_lock.as_mut() {
        cache.registration_epoch = registration_epoch;
    }
    Ok(BUILTIN_REGISTRATION_EPOCH.load(Ordering::SeqCst) == registration_epoch)
}

fn try_get_processor_snapshot(require_complete_registration: bool) -> Option<ProcessorSnapshot> {
    if require_complete_registration && BUILTIN_REGISTRATION_REQUIRED.load(Ordering::Acquire) {
        return None;
    }
    let registration_epoch = BUILTIN_REGISTRATION_EPOCH.load(Ordering::SeqCst);
    if registration_update_in_progress(registration_epoch) {
        return None;
    }
    #[cfg(test)]
    run_before_processor_snapshot_hook();
    let registry_generation = crate::plugins::registry::get_post_processor_registry()
        .read()
        .generation();
    let lease = ProcessorSnapshotLease::acquire(&ACTIVE_PROCESSOR_SNAPSHOTS);
    let stages = PROCESSOR_CACHE.try_read()?.as_ref().and_then(|cache| {
        (cache.registration_epoch == registration_epoch && cache.generation == registry_generation)
            .then(|| cached_processor_stages(cache))
    })?;
    if BUILTIN_REGISTRATION_EPOCH.load(Ordering::SeqCst) != registration_epoch
        || crate::plugins::registry::get_post_processor_registry()
            .read()
            .generation()
            != registry_generation
    {
        return None;
    }
    #[cfg(test)]
    run_after_processor_snapshot_validated_hook();
    Some(ProcessorSnapshot {
        early: stages.0,
        middle: stages.1,
        late: stages.2,
        _lease: lease,
    })
}

fn cached_processor_stages(cache: &ProcessorCache) -> ProcessorStages {
    (
        std::sync::Arc::clone(&cache.early),
        std::sync::Arc::clone(&cache.middle),
        std::sync::Arc::clone(&cache.late),
    )
}

/// Get processors from the cache, organized by stage.
// ~keep Liveness is configuration-dependent: the only consumers are compiled out on
// narrow feature legs, so `-D dead-code` fires there and nowhere else. A hand-kept
// union-of-consumers `cfg` is what drifted here and failed the 1.3.0 publish (GH#1951).
#[cfg(test)]
#[allow(dead_code)]
pub(super) fn get_processors_from_cache() -> Result<ProcessorStages> {
    let cache_lock = PROCESSOR_CACHE.read();
    let cache = cache_lock
        .as_ref()
        .ok_or_else(|| crate::XbergError::Other("Processor cache not initialized".to_string()))?;
    Ok(cached_processor_stages(cache))
}

#[cfg(all(test, feature = "tokio-runtime"))]
mod registry_state;
#[cfg(all(test, feature = "tokio-runtime"))]
pub(super) use registry_state::{ProcessorRegistryState, processor_snapshot_from_state};

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(all(feature = "quality", feature = "summarization"))]
    fn cached_processor_names() -> Vec<String> {
        let (early, middle, late) = get_processors_from_cache().unwrap();
        early
            .iter()
            .chain(middle.iter())
            .chain(late.iter())
            .map(|processor| processor.name().to_string())
            .collect()
    }

    #[test]
    #[serial_test::serial]
    #[cfg(all(feature = "quality", feature = "summarization"))]
    fn cache_snapshot_rejects_clear_started_after_feature_fast_path() {
        use crate::plugins::registry::test_support::PostProcessorRegistryGuard;
        use std::sync::mpsc;

        let _guard = PostProcessorRegistryGuard::acquire();
        initialize_features();
        initialize_processor_cache().unwrap();
        assert!(!BUILTIN_REGISTRATION_REQUIRED.load(Ordering::Acquire));
        let registration_epoch = BUILTIN_REGISTRATION_EPOCH.load(Ordering::Acquire);
        assert!(!registration_update_in_progress(registration_epoch));

        let (cleared_sender, cleared_receiver) = mpsc::channel();
        let (release_sender, release_receiver) = mpsc::channel();
        let clear_thread = std::thread::spawn(move || {
            with_builtin_registration_recovery(|| {
                let result = crate::plugins::registry::get_post_processor_registry()
                    .write()
                    .shutdown_all();
                cleared_sender.send(()).unwrap();
                release_receiver.recv().unwrap();
                result
            })
        });
        cleared_receiver.recv().unwrap();

        let accepted_stale_epoch = try_initialize_processor_cache(registration_epoch).unwrap();
        let cached_during_clear = cached_processor_names();
        release_sender.send(()).unwrap();
        clear_thread.join().unwrap().unwrap();

        assert!(!accepted_stale_epoch);
        assert!(cached_during_clear.iter().any(|name| name == "quality-processing"));

        initialize_processor_cache().unwrap();
        let names = cached_processor_names();
        assert!(names.iter().any(|name| name == "quality-processing"));
        assert!(names.iter().any(|name| name == "summarization"));
    }

    #[test]
    #[serial_test::serial]
    #[cfg(all(feature = "quality", feature = "summarization"))]
    fn cache_initialization_waits_without_polling_during_registry_update() {
        use crate::plugins::registry::test_support::PostProcessorRegistryGuard;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::{Arc, mpsc};
        use std::time::Duration;

        let _guard = PostProcessorRegistryGuard::acquire();
        initialize_processor_cache().unwrap();
        let (fast_path_sender, fast_path_receiver) = mpsc::channel();
        let (resume_sender, resume_receiver) = mpsc::channel();
        let retry_count = Arc::new(AtomicUsize::new(0));
        let thread_retry_count = Arc::clone(&retry_count);
        let initialize_thread = std::thread::spawn(move || {
            AFTER_FEATURE_INITIALIZATION_HOOK.with(|slot| {
                *slot.borrow_mut() = Some(Box::new(move || {
                    fast_path_sender.send(()).unwrap();
                    resume_receiver.recv().unwrap();
                }));
            });
            REGISTRATION_RETRY_HOOK.with(|slot| {
                *slot.borrow_mut() = Some(Box::new(move || {
                    thread_retry_count.fetch_add(1, Ordering::Relaxed);
                }));
            });
            initialize_processor_cache()
        });
        fast_path_receiver.recv().unwrap();

        let (update_started_sender, update_started_receiver) = mpsc::channel();
        let (update_release_sender, update_release_receiver) = mpsc::channel();
        let update_thread = std::thread::spawn(move || {
            with_post_processor_suppressed("unregistered-test-processor", || {
                update_started_sender.send(()).unwrap();
                update_release_receiver.recv().unwrap();
                Ok::<(), crate::XbergError>(())
            })
        });
        update_started_receiver.recv().unwrap();
        resume_sender.send(()).unwrap();

        std::thread::sleep(Duration::from_millis(25));
        assert_eq!(retry_count.load(Ordering::Relaxed), 1);

        update_release_sender.send(()).unwrap();
        update_thread.join().unwrap().unwrap();
        initialize_thread.join().unwrap().unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    #[serial_test::serial]
    #[cfg(all(feature = "tokio-runtime", feature = "quality", feature = "summarization"))]
    async fn stable_cache_handoff_skips_blocking_initialization() {
        use crate::plugins::registry::test_support::PostProcessorRegistryGuard;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let _guard = PostProcessorRegistryGuard::acquire();
        initialize_processor_cache().unwrap();
        let blocking_initializations = Arc::new(AtomicUsize::new(0));
        let hook_count = Arc::clone(&blocking_initializations);
        test_support::set_before_blocking_cache_initialization_hook(Some(Box::new(move || {
            hook_count.fetch_add(1, Ordering::SeqCst);
        })));

        // `try_get_processor_snapshot` probes `PROCESSOR_CACHE` with a non-blocking `try_read`,
        // which yields `None` while any other thread holds the write side -- and dozens of
        // non-`#[serial]` tests in this binary run real extractions that take it. A single
        // contended probe therefore diverts to the blocking path for reasons unrelated to the
        // behaviour under test. Retrying keeps the assertion meaningful: a genuinely broken fast
        // path fails every attempt, so this still fails outright rather than being weakened into
        // a check no defect could trip. ~keep
        const HANDOFF_ATTEMPTS: usize = 50;
        const HANDOFF_RETRY_DELAY: std::time::Duration = std::time::Duration::from_millis(20);
        let mut warm_snapshot = None;
        for _ in 0..HANDOFF_ATTEMPTS {
            blocking_initializations.store(0, Ordering::SeqCst);
            let candidate = initialize_processor_cache_for_async_pipeline().await.unwrap();
            if blocking_initializations.load(Ordering::SeqCst) == 0 {
                warm_snapshot = Some(candidate);
                break;
            }
            tokio::time::sleep(HANDOFF_RETRY_DELAY).await;
        }
        let snapshot =
            warm_snapshot.expect("the warm-cache fast path never won an uncontended probe across HANDOFF_ATTEMPTS");
        test_support::set_before_blocking_cache_initialization_hook(None);
        let names = snapshot
            .early
            .iter()
            .chain(snapshot.middle.iter())
            .chain(snapshot.late.iter())
            .map(|processor| processor.name())
            .collect::<Vec<_>>();

        assert!(names.contains(&"quality-processing"));
        assert!(names.contains(&"summarization"));
    }

    #[test]
    #[serial_test::serial]
    #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
    fn keyword_recovery_runs_after_a_concurrent_clear() {
        use crate::plugins::registry::test_support::PostProcessorRegistryGuard;
        use std::sync::mpsc;

        let _guard = PostProcessorRegistryGuard::acquire();
        initialize_processor_cache().unwrap();
        let (reached_sender, reached_receiver) = mpsc::channel();
        let (release_sender, release_receiver) = mpsc::channel();
        let initialize_thread = std::thread::spawn(move || {
            BEFORE_REGISTRATION_CHECK_HOOK.with(|slot| {
                *slot.borrow_mut() = Some(Box::new(move || {
                    reached_sender.send(()).unwrap();
                    release_receiver.recv().unwrap();
                }));
            });
            initialize_processor_cache()
        });
        reached_receiver.recv().unwrap();
        // `clear_post_processors` is refused whenever ACTIVE_PROCESSOR_SNAPSHOTS is non-zero,
        // and that counter is process-global: ANY concurrent test in this binary holding an
        // extraction lease refuses this clear, not just the parked initialize this test set up.
        // `#[serial]` does not help, since it excludes only other `#[serial]` tests. Retrying is
        // what the refusal itself instructs ("retry the lifecycle mutation after extraction
        // completes"), so the test now follows the contract instead of assuming exclusivity --
        // it was failing roughly one run in twenty. Only that specific refusal is retried; any
        // other error still fails the test. ~keep
        const REGISTRY_IN_USE: &str = "post-processor registry is in use by an active extraction; retry the lifecycle mutation after extraction completes";
        let mut clear_result = crate::plugins::clear_post_processors();
        for _ in 0..200 {
            match &clear_result {
                Err(crate::XbergError::Other(message)) if message == REGISTRY_IN_USE => {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                    clear_result = crate::plugins::clear_post_processors();
                }
                _ => break,
            }
        }
        release_sender.send(()).unwrap();
        initialize_thread.join().unwrap().unwrap();
        clear_result.unwrap();

        let names = crate::plugins::registry::get_post_processor_registry().read().list();
        assert!(
            names
                .iter()
                .any(|name| name == crate::keywords::processor::KEYWORD_PROCESSOR_NAME)
        );
    }

    #[test]
    fn registration_error_includes_quality_failure() {
        let mut failures = Vec::new();
        record_registration_result(
            &mut failures,
            "quality-processing",
            Err(crate::XbergError::Other("quality init failed".to_string())),
        );

        assert_eq!(
            aggregate_registration_error(failures).as_deref(),
            Some("automatic post-processor registration failed: quality-processing: quality init failed")
        );
    }

    /// #271: `builtin_registration_error` must round-trip whatever the
    /// registration pass recorded, so pipeline code has somewhere to look
    /// instead of the failure being dropped by `let _ = ...`.
    #[test]
    fn builtin_registration_error_reports_a_recorded_failure() {
        {
            let mut slot = BUILTIN_REGISTRATION_ERROR
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *slot = Some("1 of 3 built-in post-processor(s) failed to register: ner: boom".to_string());
        }
        assert_eq!(
            builtin_registration_error(),
            Some("1 of 3 built-in post-processor(s) failed to register: ner: boom".to_string())
        );

        // Reset: this static is process-global and shared with other tests in this binary.
        let mut slot = BUILTIN_REGISTRATION_ERROR
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *slot = None;
    }

    /// #215: a post-processor registered after the cache was already populated
    /// must become visible on the *next* extraction, not stay invisible until
    /// something remembers to call `clear_processor_cache()`.
    ///
    /// `#[serial]`: this test clears the global post-processor registry (via
    /// `PostProcessorRegistryGuard`) and mutates the process-global
    /// `PROCESSOR_CACHE` directly — there is no local/injectable variant of either,
    /// so any other test that runs the real pipeline concurrently and expects the
    /// built-in post-processors (or a previously cached, non-empty processor set)
    /// to be present would race this one. Serializing against the crate's other
    /// `#[serial]` tests (see `core::extractor::mod::test_concurrent_extractions_different_mimes`)
    /// is the same pattern used there for the same reason.
    ///
    /// The assertions are deliberately about **containment of this test's own
    /// processor**, never about the registry being empty or about an exact count.
    /// `#[serial]` only excludes the crate's other `#[serial]` tests, while *any*
    /// non-serial test that runs a real extraction calls `initialize_features()` and
    /// registers the built-in post-processors into the same global registry. An
    /// "assert the cache starts empty" check therefore failed roughly one run in four
    /// — a property of the test harness, not of the code under test. Emptiness was
    /// only ever scaffolding; the #215 invariant is that a *late* registration becomes
    /// visible, which containment states exactly and races nothing.
    // ~keep `ProcessorRegistryState` is imported only under `all(test, feature = "tokio-runtime")`;
    // without the matching gate this test failed to resolve it on every non-tokio leg (GH#1951).
    #[cfg(feature = "tokio-runtime")]
    #[serial_test::serial]
    #[test]
    fn processor_cache_rebuilds_when_registry_changes_after_first_use() {
        use crate::plugins::{Plugin, PostProcessor, ProcessingStage};
        use crate::types::ExtractedDocument;
        use async_trait::async_trait;
        use std::sync::Arc;

        let state = ProcessorRegistryState::new_isolated();

        #[derive(Debug)]
        struct LateAddedProcessor;

        impl Plugin for LateAddedProcessor {
            fn name(&self) -> &str {
                "late-added-215"
            }
            fn version(&self) -> String {
                "1.0.0".to_string()
            }
            fn initialize(&self) -> Result<()> {
                Ok(())
            }
            fn shutdown(&self) -> Result<()> {
                Ok(())
            }
        }

        #[async_trait]
        impl PostProcessor for LateAddedProcessor {
            async fn process(
                &self,
                _result: &mut ExtractedDocument,
                _config: &crate::core::config::ExtractionConfig,
            ) -> Result<()> {
                Ok(())
            }
            fn processing_stage(&self) -> ProcessingStage {
                ProcessingStage::Middle
            }
        }

        let has_late_added =
            |processors: &[std::sync::Arc<dyn PostProcessor>]| processors.iter().any(|p| p.name() == "late-added-215");

        // Populate the cache, exactly as the first pipeline run of a process would.
        state.ensure_cache_current().unwrap();
        let middle = state.try_get_snapshot().unwrap().middle;
        assert!(
            !has_late_added(&middle),
            "the processor under test must not be in the cache before it is registered"
        );

        // Register a processor *after* the cache already holds a snapshot.
        state.register(Arc::new(LateAddedProcessor)).unwrap();

        // Without the #215 fix, `initialize_processor_cache` is a no-op once the
        // cache is `Some(_)`, so the newly registered processor would never appear.
        state.ensure_cache_current().unwrap();
        let middle = state.try_get_snapshot().unwrap().middle;
        assert!(
            has_late_added(&middle),
            "the cache must pick up the post-registration processor"
        );
    }
}
