use std::sync::Arc;
use std::time::{Duration, Instant};

use super::LocalClassificationOptions;
use super::control::{RequestState, run_controlled};

async fn aborted_request_state() -> (bool, bool) {
    let semaphore = Arc::new(tokio::sync::Semaphore::new(1));
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let worker_semaphore = Arc::clone(&semaphore);
    let worker = tokio::spawn(async move {
        run_controlled(worker_semaphore, Default::default(), Instant::now(), move |state| {
            let _ = started_tx.send(Arc::clone(&state));
            release_rx.recv_timeout(Duration::from_secs(2)).unwrap();
            Ok(())
        })
        .await
    });
    let state = started_rx.await.unwrap();
    worker.abort();
    assert!(worker.await.unwrap_err().is_cancelled());
    let held = semaphore.try_acquire().is_err();
    let cancelled = state.is_cancelled();
    release_tx.send(()).unwrap();
    let _returned = tokio::time::timeout(Duration::from_secs(1), semaphore.acquire())
        .await
        .unwrap()
        .unwrap();
    (held, cancelled)
}

#[tokio::test]
async fn dropping_a_request_does_not_release_a_still_running_worker_permit() {
    let (held, _) = aborted_request_state().await;
    assert!(held, "a dropped caller must not admit another native worker");
}

#[tokio::test]
async fn dropping_a_request_marks_the_native_work_for_termination() {
    let (_, cancelled) = aborted_request_state().await;
    assert!(
        cancelled,
        "the native worker must observe cancellation after caller drop"
    );
}

#[tokio::test]
async fn a_pre_cancelled_queued_request_does_not_wait_for_a_permit() {
    let semaphore = Arc::new(tokio::sync::Semaphore::new(0));
    let token = crate::cancellation::CancellationToken::new();
    token.cancel();
    let options = LocalClassificationOptions {
        cancel_token: Some(token),
        deadline_ms: None,
    };
    let result = tokio::time::timeout(
        Duration::from_secs(1),
        run_controlled(semaphore, options, Instant::now(), |_| Ok(())),
    )
    .await
    .unwrap();
    assert!(matches!(result, Err(crate::XbergError::Cancelled)));
}

#[tokio::test]
async fn cancellation_signals_active_work_and_waits_for_its_native_return() {
    let token = crate::cancellation::CancellationToken::new();
    let options = LocalClassificationOptions {
        cancel_token: Some(token.clone()),
        deadline_ms: None,
    };
    let returned = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let observed = Arc::clone(&returned);
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let worker = tokio::spawn(async move {
        run_controlled(
            Arc::new(tokio::sync::Semaphore::new(1)),
            options,
            Instant::now(),
            move |state| {
                let _ = started_tx.send(());
                let started = Instant::now();
                while !state.is_cancelled() && started.elapsed() < Duration::from_secs(2) {
                    std::thread::sleep(Duration::from_millis(1));
                }
                observed.store(true, std::sync::atomic::Ordering::Release);
                Ok(())
            },
        )
        .await
    });
    started_rx.await.unwrap();
    token.cancel();
    let result = worker.await.unwrap();
    assert!(returned.load(std::sync::atomic::Ordering::Acquire));
    assert!(matches!(result, Err(crate::XbergError::Cancelled)));
}

#[test]
fn cancellation_terminates_the_actual_native_run_options_before_graph_execution() {
    let environment = crate::ort_discovery::ort_environment().unwrap();
    let mut session = ort::session::Session::builder(environment)
        .unwrap()
        .with_no_environment_execution_providers()
        .unwrap()
        .with_intra_threads(1)
        .unwrap()
        .with_inter_threads(1)
        .unwrap()
        .commit_from_memory(&super::toy_model::add_model_bytes())
        .unwrap();
    let input = ort::value::Tensor::from_array(([1usize], vec![1.0f32])).unwrap();
    let output = session.run(ort::inputs!["X" => input]).unwrap();
    let (shape, values) = output["Y"].try_extract_tensor::<f32>().unwrap();
    assert_eq!(&shape[..], &[1]);
    assert_eq!(values, &[2.0]);
    drop(output);
    let options = Arc::new(ort::session::RunOptions::new().unwrap());
    let state = RequestState::default();
    state.register_run_options(Arc::clone(&options)).unwrap();
    state.cancel().unwrap();
    let input = ort::value::Tensor::from_array(([1usize], vec![1.0f32])).unwrap();
    let error = session
        .run_with_options(ort::inputs!["X" => input], options.as_ref())
        .expect_err("a pre-terminated native run must not execute the Add graph");
    assert!(error.to_string().to_ascii_lowercase().contains("terminate"), "{error}");
}
