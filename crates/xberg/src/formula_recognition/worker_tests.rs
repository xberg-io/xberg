use super::run_with_formula_slot;
use std::future::{Future, poll_fn};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::task::Poll;
use std::time::Duration;
use tokio::sync::{Semaphore, oneshot};

const TEST_TIMEOUT: Duration = Duration::from_secs(5);

fn two_blocking_workers() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .max_blocking_threads(2)
        .enable_time()
        .build()
        .unwrap()
}

async fn poll_queued(mut future: Pin<&mut impl Future>) {
    poll_fn(|context| {
        assert!(future.as_mut().poll(context).is_pending());
        Poll::Ready(())
    })
    .await;
}

#[test]
fn waiting_formula_crop_leaves_the_other_blocking_worker_free() {
    two_blocking_workers().block_on(async {
        let slot = Arc::new(Semaphore::new(1));
        let (active_entered, active_started) = oneshot::channel();
        let (release_active, active_gate) = mpsc::channel();
        let active = tokio::spawn(run_with_formula_slot(slot.clone(), move || {
            active_entered.send(()).unwrap();
            active_gate.recv().unwrap();
        }));
        tokio::time::timeout(TEST_TIMEOUT, active_started)
            .await
            .unwrap()
            .unwrap();

        let (release_queued, queued_gate) = mpsc::channel();
        let mut queued = Box::pin(run_with_formula_slot(slot, move || {
            queued_gate.recv().unwrap();
        }));
        poll_queued(queued.as_mut()).await;
        let canary = tokio::time::timeout(TEST_TIMEOUT, tokio::task::spawn_blocking(|| 42)).await;

        release_active.send(()).unwrap();
        release_queued.send(()).unwrap();
        assert_eq!(
            canary
                .expect("a shared formula waiter occupied the spare worker")
                .unwrap(),
            42
        );
        active.await.unwrap().unwrap();
        tokio::time::timeout(TEST_TIMEOUT, queued).await.unwrap().unwrap();
    });
}

#[test]
fn dropping_a_queued_formula_skips_it_and_keeps_the_active_slot() {
    two_blocking_workers().block_on(async {
        let slot = Arc::new(Semaphore::new(1));
        let (active_entered, active_started) = oneshot::channel();
        let (release_active, active_gate) = mpsc::channel();
        let active = tokio::spawn(run_with_formula_slot(slot.clone(), move || {
            active_entered.send(()).unwrap();
            active_gate.recv().unwrap();
        }));
        tokio::time::timeout(TEST_TIMEOUT, active_started)
            .await
            .unwrap()
            .unwrap();

        let cancelled_ran = Arc::new(AtomicBool::new(false));
        let worker_ran = cancelled_ran.clone();
        let mut queued = Box::pin(run_with_formula_slot(slot.clone(), move || {
            worker_ran.store(true, Ordering::SeqCst);
        }));
        poll_queued(queued.as_mut()).await;
        drop(queued);

        active.abort();
        assert!(active.await.unwrap_err().is_cancelled());
        assert_eq!(slot.available_permits(), 0, "active worker still holds its slot");

        let mut next = Box::pin(run_with_formula_slot(slot, || 42));
        poll_queued(next.as_mut()).await;
        release_active.send(()).unwrap();
        assert_eq!(tokio::time::timeout(TEST_TIMEOUT, next).await.unwrap().unwrap(), 42);
        assert!(!cancelled_ran.load(Ordering::SeqCst), "dropped queued work never ran");
    });
}
