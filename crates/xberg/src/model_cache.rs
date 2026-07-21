//! Bounded global model pool for ONNX-based models.
//!
//! Expensive sessions are checked out through an RAII lease. The pool caps
//! both live leases and in-progress construction, preventing concurrent cache
//! misses from creating an unbounded number of duplicate sessions.

use std::ops::{Deref, DerefMut};
use std::sync::{Condvar, Mutex, MutexGuard};

struct PoolState<T> {
    available: Vec<T>,
    checked_out: usize,
}

/// A bounded pool of reusable model instances.
#[cfg_attr(alef, alef(skip))]
pub struct ModelCache<T: Send> {
    capacity: usize,
    state: Mutex<PoolState<T>>,
    returned: Condvar,
}

/// Exclusive model checkout that returns the model to its pool on drop.
#[cfg_attr(alef, alef(skip))]
pub(crate) struct ModelLease<'a, T: Send> {
    model: Option<T>,
    cache: &'a ModelCache<T>,
}

struct ConstructionReservation<'a, T: Send> {
    cache: &'a ModelCache<T>,
    active: bool,
}

impl<T: Send> Default for ModelCache<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Send> ModelCache<T> {
    /// Create a single-instance model pool.
    pub const fn new() -> Self {
        Self::with_capacity(1)
    }

    /// Create a pool capped at `capacity` live or constructing models.
    pub const fn with_capacity(capacity: usize) -> Self {
        assert!(capacity > 0, "model cache capacity must be positive");
        Self {
            capacity,
            state: Mutex::new(PoolState {
                available: Vec::new(),
                checked_out: 0,
            }),
            returned: Condvar::new(),
        }
    }

    fn lock_state(&self) -> MutexGuard<'_, PoolState<T>> {
        self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Checkout a cached model, construct one within the capacity reservation,
    /// or wait until an existing lease is returned.
    #[cfg(test)]
    pub(crate) fn take_or_create<E>(&self, create_fn: impl FnOnce() -> Result<T, E>) -> Result<ModelLease<'_, T>, E> {
        self.take_or_create_matching(|_| true, create_fn)
    }

    /// Checkout a matching cached model while preserving the global capacity.
    ///
    /// When the pool is full of idle mismatches, one mismatch is evicted before
    /// construction. If every slot is checked out or constructing, the caller
    /// waits for a return before deciding whether to reuse or evict it.
    pub(crate) fn take_or_create_matching<E>(
        &self,
        mut matches: impl FnMut(&T) -> bool,
        create_fn: impl FnOnce() -> Result<T, E>,
    ) -> Result<ModelLease<'_, T>, E> {
        let mut state = self.lock_state();
        loop {
            if let Some(index) = state.available.iter().position(&mut matches) {
                let model = state.available.swap_remove(index);
                state.checked_out += 1;
                let lease = ModelLease {
                    model: Some(model),
                    cache: self,
                };
                drop(state);
                tracing::debug!(capacity = self.capacity, "Reusing pooled model");
                return Ok(lease);
            }

            let total_models = state.checked_out + state.available.len();
            if total_models < self.capacity || !state.available.is_empty() {
                let evicted = if total_models >= self.capacity {
                    state.available.pop()
                } else {
                    None
                };
                state.checked_out += 1;
                let mut reservation = ConstructionReservation {
                    cache: self,
                    active: true,
                };
                drop(state);
                drop(evicted);
                tracing::debug!(capacity = self.capacity, "Creating pooled model");
                let model = create_fn()?;
                let lease = ModelLease {
                    model: Some(model),
                    cache: self,
                };
                reservation.active = false;
                return Ok(lease);
            }

            state = self
                .returned
                .wait(state)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
    }

    fn release_reservation(&self) {
        let mut state = self.lock_state();
        debug_assert!(state.checked_out > 0);
        state.checked_out -= 1;
        self.returned.notify_one();
    }

    fn return_model(&self, model: T) {
        let mut state = self.lock_state();
        debug_assert!(state.checked_out > 0);
        state.checked_out -= 1;
        state.available.push(model);
        self.returned.notify_one();
    }
}

impl<T: Send> Drop for ConstructionReservation<'_, T> {
    fn drop(&mut self) {
        if self.active {
            self.cache.release_reservation();
        }
    }
}

impl<T: Send> Deref for ModelLease<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.model.as_ref().expect("model lease must contain a model")
    }
}

impl<T: Send> DerefMut for ModelLease<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.model.as_mut().expect("model lease must contain a model")
    }
}

impl<T: Send> Drop for ModelLease<'_, T> {
    fn drop(&mut self) {
        if let Some(model) = self.model.take() {
            self.cache.return_model(model);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Barrier};
    use std::time::Duration;

    #[test]
    fn concurrent_misses_never_construct_beyond_capacity() {
        let cache = Arc::new(ModelCache::with_capacity(2));
        let created = Arc::new(AtomicUsize::new(0));
        let active = Arc::new(AtomicUsize::new(0));
        let max_active = Arc::new(AtomicUsize::new(0));
        let holders = Arc::new(AtomicUsize::new(0));
        let start = Arc::new(Barrier::new(5));
        let first_two_holding = Arc::new(Barrier::new(3));
        let release_first_two = Arc::new(Barrier::new(3));

        let handles: Vec<_> = (0..4)
            .map(|_| {
                let cache = Arc::clone(&cache);
                let created = Arc::clone(&created);
                let active = Arc::clone(&active);
                let max_active = Arc::clone(&max_active);
                let holders = Arc::clone(&holders);
                let start = Arc::clone(&start);
                let first_two_holding = Arc::clone(&first_two_holding);
                let release_first_two = Arc::clone(&release_first_two);
                std::thread::spawn(move || {
                    start.wait();
                    let lease = cache
                        .take_or_create(|| {
                            created.fetch_add(1, Ordering::SeqCst);
                            Ok::<_, ()>(())
                        })
                        .unwrap();
                    let now = active.fetch_add(1, Ordering::SeqCst) + 1;
                    max_active.fetch_max(now, Ordering::SeqCst);
                    if holders.fetch_add(1, Ordering::SeqCst) < 2 {
                        first_two_holding.wait();
                        release_first_two.wait();
                    }
                    active.fetch_sub(1, Ordering::SeqCst);
                    drop(lease);
                })
            })
            .collect();

        start.wait();
        first_two_holding.wait();
        assert_eq!(created.load(Ordering::SeqCst), 2);
        assert_eq!(active.load(Ordering::SeqCst), 2);
        release_first_two.wait();
        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(created.load(Ordering::SeqCst), 2);
        assert_eq!(max_active.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn constructor_failure_releases_capacity_for_waiter() {
        let cache = Arc::new(ModelCache::with_capacity(1));
        let (constructor_started_tx, constructor_started_rx) = std::sync::mpsc::channel();
        let (release_constructor_tx, release_constructor_rx) = std::sync::mpsc::channel();
        let failing_cache = Arc::clone(&cache);
        let failing = std::thread::spawn(move || {
            failing_cache
                .take_or_create(|| {
                    constructor_started_tx.send(()).unwrap();
                    release_constructor_rx.recv().unwrap();
                    Err::<usize, _>("failed")
                })
                .map(|_| ())
                .unwrap_err()
        });
        constructor_started_rx.recv().unwrap();

        let waiting_cache = Arc::clone(&cache);
        let waiter = std::thread::spawn(move || {
            let lease = waiting_cache.take_or_create(|| Ok::<_, &str>(7)).unwrap();
            *lease
        });
        release_constructor_tx.send(()).unwrap();

        assert_eq!(failing.join().unwrap(), "failed");
        assert_eq!(waiter.join().unwrap(), 7);
    }

    #[test]
    fn dropped_lease_wakes_waiter_and_reuses_model() {
        let cache = Arc::new(ModelCache::with_capacity(1));
        let first = cache.take_or_create(|| Ok::<_, ()>(11)).unwrap();
        let (waiting_tx, waiting_rx) = std::sync::mpsc::channel();
        let (result_tx, result_rx) = std::sync::mpsc::channel();
        let waiting_cache = Arc::clone(&cache);
        let handle = std::thread::spawn(move || {
            waiting_tx.send(()).unwrap();
            let lease = waiting_cache.take_or_create(|| Ok::<_, ()>(99)).unwrap();
            result_tx.send(*lease).unwrap();
        });

        waiting_rx.recv().unwrap();
        assert!(matches!(
            result_rx.recv_timeout(Duration::from_millis(20)),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout)
        ));
        drop(first);

        assert_eq!(result_rx.recv().unwrap(), 11);
        handle.join().unwrap();
    }

    #[test]
    fn constructor_panic_releases_capacity() {
        let cache = ModelCache::with_capacity(1);
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = cache.take_or_create(|| -> Result<usize, ()> { panic!("constructor panic") });
        }));
        assert!(panic.is_err());

        let lease = cache.take_or_create(|| Ok::<_, ()>(17)).unwrap();
        assert_eq!(*lease, 17);
    }

    #[test]
    fn consistent_nested_pool_order_makes_progress() {
        let primary = Arc::new(ModelCache::with_capacity(2));
        let classifier = Arc::new(ModelCache::with_capacity(2));
        let alternate = Arc::new(ModelCache::with_capacity(2));
        let start = Arc::new(Barrier::new(5));

        let handles: Vec<_> = (0..4)
            .map(|_| {
                let primary = Arc::clone(&primary);
                let classifier = Arc::clone(&classifier);
                let alternate = Arc::clone(&alternate);
                let start = Arc::clone(&start);
                std::thread::spawn(move || {
                    start.wait();
                    let _primary = primary.take_or_create(|| Ok::<_, ()>(())).unwrap();
                    let _classifier = classifier.take_or_create(|| Ok::<_, ()>(())).unwrap();
                    let _alternate = alternate.take_or_create(|| Ok::<_, ()>(())).unwrap();
                })
            })
            .collect();

        start.wait();
        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn matching_checkout_reuses_only_matching_model() {
        let cache = ModelCache::with_capacity(2);
        drop(cache.take_or_create(|| Ok::<_, ()>(("a", 1))).unwrap());

        let matching = cache
            .take_or_create_matching(|model| model.0 == "a", || Ok::<_, ()>(("new", 2)))
            .unwrap();

        assert_eq!(*matching, ("a", 1));
    }

    #[test]
    fn mismatch_is_evicted_when_pool_is_full() {
        let cache = ModelCache::with_capacity(1);
        drop(cache.take_or_create(|| Ok::<_, ()>(("old", 1))).unwrap());

        let replacement = cache
            .take_or_create_matching(|model| model.0 == "new", || Ok::<_, ()>(("new", 2)))
            .unwrap();
        assert_eq!(*replacement, ("new", 2));
        drop(replacement);

        let reused = cache
            .take_or_create_matching(|model| model.0 == "new", || Ok::<_, ()>(("unexpected", 3)))
            .unwrap();
        assert_eq!(*reused, ("new", 2));
    }

    #[test]
    fn evicted_model_drop_panic_releases_construction_reservation() {
        struct PanicOnDrop {
            value: usize,
            panic_on_drop: bool,
        }

        impl Drop for PanicOnDrop {
            fn drop(&mut self) {
                assert!(!self.panic_on_drop, "evicted model drop panic");
            }
        }

        let cache = ModelCache::with_capacity(1);
        drop(
            cache
                .take_or_create(|| {
                    Ok::<_, ()>(PanicOnDrop {
                        value: 1,
                        panic_on_drop: true,
                    })
                })
                .unwrap(),
        );

        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = cache.take_or_create_matching(
                |model| model.value == 2,
                || {
                    Ok::<_, ()>(PanicOnDrop {
                        value: 2,
                        panic_on_drop: false,
                    })
                },
            );
        }));
        assert!(panic.is_err());

        let recovered = cache
            .take_or_create_matching(
                |_| true,
                || {
                    Ok::<_, ()>(PanicOnDrop {
                        value: 3,
                        panic_on_drop: false,
                    })
                },
            )
            .unwrap();
        assert_eq!(recovered.value, 3);
    }

    #[test]
    fn retained_and_checked_out_models_share_one_capacity() {
        let cache = ModelCache::with_capacity(2);
        drop(cache.take_or_create(|| Ok::<_, ()>(1)).unwrap());
        let checked_out = cache
            .take_or_create_matching(|model| *model == 2, || Ok::<_, ()>(2))
            .unwrap();

        let state = cache.lock_state();
        assert_eq!(state.available.len(), 1);
        assert_eq!(state.checked_out, 1);
        assert_eq!(state.available.len() + state.checked_out, 2);
        drop(state);
        drop(checked_out);
    }
}
