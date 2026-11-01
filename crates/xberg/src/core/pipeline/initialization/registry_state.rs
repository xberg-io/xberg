//! Per-test, isolated replacement for the process-wide post-processor registry, cache,
//! and registration gate this module otherwise exposes only as globals.
//!
//! Split out of `initialization` to keep that file under the project's line-count budget;
//! `use super::*` below pulls in everything this needs from the parent module, private items
//! included, the same way `initialization::tests` already does. ~keep

use super::*;

/// Bumps `0` again on drop. Mirrors [`RegistrationUpdate`]'s begin/end epoch pair, but
/// generalized over any `AtomicU64` instead of the process-wide `BUILTIN_REGISTRATION_EPOCH`,
/// so [`ProcessorRegistryState::with_mutation`] can reuse the same odd-epoch-means-in-progress
/// convention on its own, isolated epoch counter.
#[cfg(test)]
struct EpochUpdateGuard<'a>(&'a AtomicU64);

#[cfg(test)]
impl Drop for EpochUpdateGuard<'_> {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

/// A private, per-instance replacement for this module's process-wide registry, processor
/// cache, and registration gate (`POST_PROCESSOR_REGISTRY`, `PROCESSOR_CACHE`,
/// `BUILTIN_REGISTRATION_LOCK`, `BUILTIN_REGISTRATION_EPOCH`, `ACTIVE_PROCESSOR_SNAPSHOTS`).
///
/// #1749: tests that mutate a post-processor registry and assert on the outcome of that
/// exact mutation (e.g. "this `unregister` must fail because a lease is held right now")
/// raced every other, non-`#[serial]` test in the binary that runs a real extraction --
/// `#[serial]` only orders `#[serial]` tests against each other, and does nothing to stop
/// an unrelated extraction from holding a `ProcessorSnapshotLease` at the exact instant one
/// of these tests calls a lifecycle mutation. Retrying the mutation
/// (`retry_while_registry_in_use` in `pipeline::tests`) only narrows that window; a test
/// whose own synchronization depends on a mutation succeeding or failing at one precise
/// moment cannot tolerate any window at all, however small. Giving each such test its own
/// registry, cache and gate removes the shared state the race needs, by construction,
/// instead of narrowing the odds of hitting it. ~keep
#[cfg(test)]
pub(crate) struct ProcessorRegistryState {
    registry: std::sync::Arc<parking_lot::RwLock<crate::plugins::registry::PostProcessorRegistry>>,
    cache: parking_lot::RwLock<Option<ProcessorCache>>,
    registration_lock: Mutex<()>,
    registration_epoch: AtomicU64,
    active_processor_snapshots: std::sync::Arc<AtomicUsize>,
}

#[cfg(test)]
impl ProcessorRegistryState {
    /// Build a fresh, empty registry: an isolated test registers only what it needs and
    /// never sees (or is affected by) the crate's automatic built-in post-processors.
    pub(crate) fn new_isolated() -> Self {
        Self {
            registry: std::sync::Arc::new(parking_lot::RwLock::new(
                crate::plugins::registry::PostProcessorRegistry::new(),
            )),
            cache: parking_lot::RwLock::new(None),
            registration_lock: Mutex::new(()),
            registration_epoch: AtomicU64::new(0),
            active_processor_snapshots: std::sync::Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Isolated equivalent of [`with_registration_update`]: run `update` while holding this
    /// instance's own registration lock, rejecting it (retryably, same error text as the
    /// global gate) if a snapshot lease on *this* instance is active. The odd/even epoch
    /// bump matches [`RegistrationUpdate`] so a concurrent [`Self::try_get_snapshot`] on the
    /// same instance sees the same "an update is in progress" signal the global gate gives.
    fn with_mutation<T>(&self, update: impl FnOnce() -> Result<T>) -> Result<T> {
        let _registration_guard = self
            .registration_lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        self.registration_epoch.fetch_add(1, Ordering::SeqCst);
        let _epoch_guard = EpochUpdateGuard(&self.registration_epoch);
        // Sampled before the hook, matching `with_registration_update` -- see the ~keep
        // comment there for why the ordering matters under contention.
        let registry_in_use = self.active_processor_snapshots.load(Ordering::SeqCst) != 0;
        run_after_registration_update_began_hook();
        if registry_in_use {
            return Err(crate::XbergError::Other(
                "post-processor registry is in use by an active extraction; retry the lifecycle mutation after extraction completes"
                    .to_string(),
            ));
        }
        update()
    }

    /// Register a post-processor into this isolated registry.
    pub(crate) fn register(&self, processor: std::sync::Arc<dyn crate::plugins::PostProcessor>) -> Result<()> {
        self.with_mutation(|| self.registry.write().register(processor))
    }

    /// Unregister a post-processor from this isolated registry by name.
    pub(crate) fn unregister(&self, name: &str) -> Result<()> {
        self.with_mutation(|| self.registry.write().remove(name))
    }

    /// Run `update` under this instance's registration gate without touching the registry
    /// itself, for tests that exercise the gate (e.g. rejecting a mutation while a lease is
    /// held) rather than a specific register/unregister call.
    pub(crate) fn with_gate<T>(&self, update: impl FnOnce() -> Result<T>) -> Result<T> {
        self.with_mutation(update)
    }

    fn try_initialize_cache(&self, registration_epoch: u64) -> Result<bool> {
        let current_generation = self.registry.read().generation();
        if self.registration_epoch.load(Ordering::SeqCst) != registration_epoch {
            return Ok(false);
        }

        let mut cache_lock = self.cache.write();
        let is_stale = cache_lock
            .as_ref()
            .is_some_and(|cache| cache.generation != current_generation);

        if cache_lock.is_none() || is_stale {
            let candidate = ProcessorCache::from_registry(&self.registry, registration_epoch)?;
            if self.registration_epoch.load(Ordering::SeqCst) != registration_epoch {
                return Ok(false);
            }
            *cache_lock = Some(candidate);
        } else if let Some(cache) = cache_lock.as_mut() {
            cache.registration_epoch = registration_epoch;
        }
        Ok(self.registration_epoch.load(Ordering::SeqCst) == registration_epoch)
    }

    fn wait_for_update(&self) {
        let _registration_guard = self
            .registration_lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
    }

    /// Isolated equivalent of [`initialize_processor_cache`], minus the built-in feature
    /// registration step: an isolated registry holds only what the test explicitly
    /// registered, so there is nothing automatic to bootstrap.
    pub(crate) fn ensure_cache_current(&self) -> Result<()> {
        loop {
            let registration_epoch = self.registration_epoch.load(Ordering::SeqCst);
            if !registration_update_in_progress(registration_epoch) && self.try_initialize_cache(registration_epoch)? {
                return Ok(());
            }
            self.wait_for_update();
        }
    }

    pub(crate) fn try_get_snapshot(&self) -> Option<ProcessorSnapshot> {
        let registration_epoch = self.registration_epoch.load(Ordering::SeqCst);
        if registration_update_in_progress(registration_epoch) {
            return None;
        }
        run_before_processor_snapshot_hook();
        let lease = ProcessorSnapshotLease::acquire(&self.active_processor_snapshots);
        #[cfg(feature = "redaction")]
        let mut builtin_redaction = None;
        let stages = self.cache.try_read()?.as_ref().and_then(|cache| {
            (cache.registration_epoch == registration_epoch).then(|| {
                #[cfg(feature = "redaction")]
                {
                    builtin_redaction = cache.builtin_redaction.as_ref().map(std::sync::Arc::clone);
                }
                cached_processor_stages(cache)
            })
        })?;
        if self.registration_epoch.load(Ordering::SeqCst) != registration_epoch {
            return None;
        }
        run_after_processor_snapshot_validated_hook();
        Some(ProcessorSnapshot {
            early: stages.0,
            middle: stages.1,
            late: stages.2,
            #[cfg(feature = "redaction")]
            builtin_redaction,
            _lease: lease,
        })
    }

    fn build_snapshot_sync(&self) -> Result<ProcessorSnapshot> {
        loop {
            self.ensure_cache_current()?;
            if let Some(snapshot) = self.try_get_snapshot() {
                return Ok(snapshot);
            }
            self.wait_for_update();
        }
    }
}

/// Isolated equivalent of [`initialize_processor_cache_for_async_pipeline`], reading from a
/// test-owned [`ProcessorRegistryState`] instead of the process-wide globals.
///
/// Takes `&Arc<ProcessorRegistryState>` rather than `&ProcessorRegistryState`:
/// `tokio::runtime::Handle::spawn_blocking` requires its closure to be `'static`, and a
/// `ProcessorRegistryState` built on a test's stack is not -- cloning the `Arc` into the
/// closure sidesteps that without requiring the state itself to be `'static`. ~keep
#[cfg(all(test, feature = "tokio-runtime"))]
pub(crate) async fn processor_snapshot_from_state(
    state: &std::sync::Arc<ProcessorRegistryState>,
) -> Result<ProcessorSnapshot> {
    if let Some(snapshot) = state.try_get_snapshot() {
        return Ok(snapshot);
    }
    let Ok(runtime) = tokio::runtime::Handle::try_current() else {
        return state.build_snapshot_sync();
    };
    let state = std::sync::Arc::clone(state);
    runtime
        .spawn_blocking(move || state.build_snapshot_sync())
        .await
        .map_err(|error| crate::XbergError::Other(format!("processor cache task failed to join: {error}")))?
}
