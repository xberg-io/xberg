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

fn blocking_workers(count: usize) -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .max_blocking_threads(count)
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
    blocking_workers(2).block_on(async {
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
fn aborting_a_formula_caller_keeps_the_active_permit_until_decode_finishes() {
    blocking_workers(2).block_on(async {
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

        active.abort();
        assert!(active.await.unwrap_err().is_cancelled());
        assert_eq!(slot.available_permits(), 0, "active worker still holds its slot");

        let mut next = Box::pin(run_with_formula_slot(slot, || 42));
        poll_queued(next.as_mut()).await;
        release_active.send(()).unwrap();
        assert_eq!(tokio::time::timeout(TEST_TIMEOUT, next).await.unwrap().unwrap(), 42);
    });
}

#[test]
fn dropping_admitted_work_releases_the_slot_across_runtimes() {
    let saturated = blocking_workers(1);
    let other = blocking_workers(1);
    let slot = Arc::new(Semaphore::new(1));
    let (entered, started) = oneshot::channel();
    let (release, gate) = mpsc::channel();
    let blocker = saturated.spawn_blocking(move || {
        entered.send(()).unwrap();
        gate.recv().unwrap();
    });

    let stale_ran = Arc::new(AtomicBool::new(false));
    let worker_ran = stale_ran.clone();
    let captured = Arc::new(());
    let captured_weak = Arc::downgrade(&captured);
    let mut queued = Box::pin(run_with_formula_slot(slot.clone(), move || {
        drop(captured);
        worker_ran.store(true, Ordering::SeqCst);
    }));
    saturated.block_on(async {
        tokio::time::timeout(TEST_TIMEOUT, started).await.unwrap().unwrap();
        poll_queued(queued.as_mut()).await;
    });
    assert_eq!(slot.available_permits(), 0, "the queued decode was admitted");
    drop(queued);
    assert!(captured_weak.upgrade().is_none(), "withdrawal dropped the queued work");

    let result = other.block_on(async { tokio::time::timeout(TEST_TIMEOUT, run_with_formula_slot(slot, || 42)).await });
    release.send(()).unwrap();
    saturated.block_on(blocker).unwrap();
    drop(saturated);
    assert_eq!(
        result.expect("the second runtime can use the withdrawn slot").unwrap(),
        42
    );
    assert!(!stale_ran.load(Ordering::SeqCst), "the withdrawn worker never decoded");
}
