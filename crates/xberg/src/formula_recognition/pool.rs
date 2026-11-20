//! The process-wide recognizer pool.
//!
//! ORT's `Session::run` needs `&mut Session`, so a recognizer serves one decode
//! at a time. Handing it to that decode — instead of holding a process-wide mutex
//! across inference — keeps the lock off the decode path while still building at
//! most `RECOGNIZER_SLOTS` recognizers.

use super::{AccelerationConfig, FormulaRecognizer, LAST_INIT_FAILURE, ensure_models};

/// Pool state: the loaded recognizer plus the acceleration it was built with,
/// so a changed acceleration config rebuilds the sessions, like the layout
/// engine's `matches_config`.
struct PooledRecognizer {
    recognizer: FormulaRecognizer,
    acceleration: Option<AccelerationConfig>,
}

/// Recognizer slots the process may build.
///
/// One, because the paired four-client qualification measured 0.69 docs/s with a
/// single two-thread recognizer against 0.68 with two, while the second set of
/// sessions added roughly 186 MiB of peak RSS: a second recognizer buys no
/// throughput, so the pool exists to keep the lock off the decode, not to grow.
const RECOGNIZER_SLOTS: usize = 1;

/// A bounded pool of owned resources handed to one caller at a time.
///
/// The formula path stores recognizers here. ORT's `Session::run` needs
/// `&mut Session`, so a recognizer can serve one decode at a time; taking it out
/// of the pool for the decode keeps the process-wide mutex off the inference
/// call. Callers wait while every slot is borrowed instead of building a second
/// recognizer, so the slot count — not the arrival rate — bounds memory.
struct ResourcePool<T> {
    idle: Vec<T>,
    borrowed: usize,
    capacity: usize,
}

impl<T> ResourcePool<T> {
    const fn new(capacity: usize) -> Self {
        Self {
            idle: Vec::new(),
            borrowed: 0,
            capacity,
        }
    }

    fn release(&mut self, resource: T) {
        self.borrowed = self.borrowed.saturating_sub(1);
        if self.idle.len() < self.capacity {
            self.idle.push(resource);
        }
    }
}

/// Borrow a resource `usable` accepts, building one when no slot is out and none
/// matches. Waits while every slot is borrowed.
fn acquire_resource<T, U, B>(
    pool: &std::sync::Mutex<ResourcePool<T>>,
    ready: &std::sync::Condvar,
    usable: U,
    build: B,
) -> Result<T, String>
where
    U: Fn(&T) -> bool,
    B: FnOnce() -> Result<T, String>,
{
    let mut pooled = pool.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    loop {
        if let Some(index) = pooled.idle.iter().position(&usable) {
            let resource = pooled.idle.swap_remove(index);
            pooled.borrowed += 1;
            return Ok(resource);
        }
        if pooled.borrowed == 0 {
            // Idle resources nothing accepts, and no decode can hand a usable one
            // back: replace them instead of waiting for a slot that will not free.
            pooled.idle.clear();
        }
        if pooled.borrowed < pooled.capacity && pooled.idle.is_empty() {
            let resource = build()?;
            pooled.borrowed += 1;
            return Ok(resource);
        }
        pooled = ready.wait(pooled).unwrap_or_else(std::sync::PoisonError::into_inner);
    }
}

/// One decode's exclusive ownership of a pooled recognizer.
///
/// Restoring on drop, rather than after the call, returns the recognizer to the
/// pool when a decode panics — the same recovery the previous mutex's
/// `PoisonError::into_inner` provided.
pub(super) struct RecognizerLease {
    recognizer: Option<FormulaRecognizer>,
    acceleration: Option<AccelerationConfig>,
}

impl RecognizerLease {
    pub(super) fn recognizer(&mut self) -> &mut FormulaRecognizer {
        self.recognizer
            .as_mut()
            .expect("a lease holds its recognizer until drop")
    }
}

impl Drop for RecognizerLease {
    fn drop(&mut self) {
        let Some(recognizer) = self.recognizer.take() else {
            return;
        };
        let mut pool = RECOGNIZERS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        pool.release(PooledRecognizer {
            recognizer,
            acceleration: self.acceleration.take(),
        });
        RECOGNIZER_READY.notify_one();
    }
}

/// Process-wide recognizer pool. Lock poisoning recovers via `into_inner`: the
/// recognizer holds no cross-call state, so a panic mid-recognition leaves
/// nothing inconsistent behind.
static RECOGNIZERS: std::sync::Mutex<ResourcePool<PooledRecognizer>> =
    std::sync::Mutex::new(ResourcePool::new(RECOGNIZER_SLOTS));
static RECOGNIZER_READY: std::sync::Condvar = std::sync::Condvar::new();

/// Borrow the pooled recognizer that matches `accel`, loading (model download
/// included) or rebuilding it when no idle slot does.
///
/// A rebuild keeps the old pool-wide behaviour of running under the lock, so
/// concurrent callers still pay one download and one session build.
pub(super) fn acquire_recognizer(accel: Option<&AccelerationConfig>) -> Result<RecognizerLease, String> {
    let pooled = acquire_resource(
        &RECOGNIZERS,
        &RECOGNIZER_READY,
        |pooled| pooled.acceleration.as_ref() == accel,
        || {
            let recognizer = ensure_models().and_then(|paths| {
                FormulaRecognizer::load(&paths, accel).map_err(|e| format!("formula model load failed: {e}"))
            });
            match recognizer {
                Ok(recognizer) => {
                    *LAST_INIT_FAILURE
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
                    Ok(PooledRecognizer {
                        recognizer,
                        acceleration: accel.cloned(),
                    })
                }
                Err(error) => {
                    *LAST_INIT_FAILURE
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(std::time::Instant::now());
                    Err(error)
                }
            }
        },
    )?;
    Ok(RecognizerLease {
        recognizer: Some(pooled.recognizer),
        acceleration: pooled.acceleration,
    })
}

#[cfg(all(test, feature = "tokio-runtime", not(target_arch = "wasm32")))]
mod tests {
    use super::{ResourcePool, acquire_resource};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc;
    use std::sync::{Arc, Condvar, Mutex, PoisonError};
    use std::time::Duration;

    const TEST_TIMEOUT: Duration = Duration::from_secs(5);

    fn pool<T>(capacity: usize) -> Arc<(Mutex<ResourcePool<T>>, Condvar)> {
        Arc::new((Mutex::new(ResourcePool::new(capacity)), Condvar::new()))
    }

    fn borrow<T, U, B>(pool: &Arc<(Mutex<ResourcePool<T>>, Condvar)>, usable: U, build: B) -> Result<T, String>
    where
        U: Fn(&T) -> bool,
        B: FnOnce() -> Result<T, String>,
    {
        let (state, ready) = pool.as_ref();
        acquire_resource(state, ready, usable, build)
    }

    fn release<T>(pool: &Arc<(Mutex<ResourcePool<T>>, Condvar)>, resource: T) {
        let (state, ready) = pool.as_ref();
        state.lock().unwrap_or_else(PoisonError::into_inner).release(resource);
        ready.notify_one();
    }

    #[test]
    fn two_slots_serve_two_decodes_at_once() {
        let pool = pool::<u8>(2);
        let first = borrow(&pool, |_| true, || Ok(1)).expect("first slot");
        let second = borrow(&pool, |_| true, || Ok(2)).expect("second slot");
        assert_eq!((first, second), (1, 2));

        let borrowed = pool.0.lock().unwrap_or_else(PoisonError::into_inner).borrowed;
        assert_eq!(borrowed, 2, "both slots are out at the same time");
    }

    #[test]
    fn a_borrower_waits_while_every_slot_is_out() {
        let builds = Arc::new(AtomicUsize::new(0));
        let pool = pool::<u8>(1);
        let held = {
            let builds = Arc::clone(&builds);
            borrow(
                &pool,
                |_| true,
                move || {
                    builds.fetch_add(1, Ordering::SeqCst);
                    Ok(7)
                },
            )
        }
        .expect("slot");

        let (started_tx, started_rx) = mpsc::channel();
        let waiter = {
            let pool = Arc::clone(&pool);
            let builds = Arc::clone(&builds);
            std::thread::spawn(move || {
                started_tx.send(()).unwrap();
                borrow(
                    &pool,
                    |_| true,
                    move || {
                        builds.fetch_add(1, Ordering::SeqCst);
                        Ok(8)
                    },
                )
            })
        };
        started_rx.recv_timeout(TEST_TIMEOUT).unwrap();
        std::thread::sleep(Duration::from_millis(100));
        assert_eq!(
            pool.0.lock().unwrap_or_else(PoisonError::into_inner).borrowed,
            1,
            "the waiter is still blocked on the taken slot"
        );

        release(&pool, held);
        assert_eq!(
            waiter.join().expect("waiter").expect("second borrow"),
            7,
            "the waiting caller took the returned slot instead of building"
        );
        assert_eq!(
            builds.load(Ordering::SeqCst),
            1,
            "waiting never builds a second resource"
        );
    }

    #[test]
    fn the_pool_lock_is_not_held_while_a_slot_is_out() {
        let pool = pool::<u8>(1);
        let held = borrow(&pool, |_| true, || Ok(3)).expect("slot");

        assert!(pool.0.try_lock().is_ok(), "a decode must not hold the pool lock");
        release(&pool, held);
        assert_eq!(pool.0.lock().unwrap_or_else(PoisonError::into_inner).idle, vec![3]);
    }

    #[test]
    fn a_mismatched_idle_resource_is_rebuilt_instead_of_reused() {
        let builds = AtomicUsize::new(0);
        let pool = pool::<String>(1);

        let stale = borrow(
            &pool,
            |value| value == "current",
            || {
                builds.fetch_add(1, Ordering::SeqCst);
                Ok("stale".to_string())
            },
        )
        .expect("first build");
        release(&pool, stale);

        let current = borrow(
            &pool,
            |value| value == "current",
            || {
                builds.fetch_add(1, Ordering::SeqCst);
                Ok("current".to_string())
            },
        )
        .expect("rebuild");
        assert_eq!(current, "current");
        assert_eq!(builds.load(Ordering::SeqCst), 2, "the mismatched resource was replaced");
        assert_eq!(
            pool.0.lock().unwrap_or_else(PoisonError::into_inner).borrowed,
            1,
            "the replaced resource is not double-counted"
        );
    }
}
