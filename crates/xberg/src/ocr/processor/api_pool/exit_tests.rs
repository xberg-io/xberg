//! Process exit through the real pool path. Each test runs its own binary as a child
//! process that calls C `exit()`, which runs the Tesseract exit cleanup.

use super::*;
use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Output};
use std::sync::OnceLock;
use std::sync::mpsc;
use std::time::Duration;

const CHILD_ENV: &str = "XBERG_POOL_EXIT_CHILD";
const RECOGNIZING: &str = "RECOGNIZING";
const AFTER_CLEANUP: &str = "AFTER-CLEANUP";
const RACE_TEST: &str = "ocr::processor::api_pool::exit_tests::exit_during_a_pool_leased_recognition_does_not_crash";
const AFTER_CLEANUP_TEST: &str =
    "ocr::processor::api_pool::exit_tests::a_checkout_after_the_exit_cleanup_fails_and_discards_the_engine";
const WIDTH: usize = 2400;
const HEIGHT: usize = 3200;

fn say(line: &str) {
    let mut out = std::io::stdout();
    let _ = writeln!(out, "{line}");
    let _ = out.flush();
}

/// A model directory with the bundled English pack. The child owns it until it exits.
fn english_models(directory: &tempfile::TempDir) -> super::super::model_context::ModelContext {
    std::fs::write(
        directory.path().join("eng.traineddata"),
        xberg_tesseract::bundled_eng_traineddata().expect("bundled English model"),
    )
    .expect("write the English model");
    super::super::model_context::ModelContext::for_path(directory.path().to_str().expect("UTF-8 path").into(), 3)
        .expect("model context")
}

/// A white RGB page with rows of stroke groups, so layout analysis finds text lines and
/// the recognizer has real work. A small linear generator fixes the content.
fn synthetic_page() -> Vec<u8> {
    let mut page = vec![255u8; WIDTH * HEIGHT * 3];
    let mut state = 0x2545_f491_4f6c_dd1d_u64;
    let mut next = move |bound: usize| {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((state >> 33) as usize) % bound
    };
    let mut top = 120;
    while top + 60 < HEIGHT - 120 {
        let mut x = 120;
        while x + 300 < WIDTH - 120 {
            let word_end = x + 60 + next(200);
            while x < word_end {
                let stroke = 4 + next(5);
                let drop = next(10);
                for column in x..x + stroke {
                    for row in top + drop..top + 40 {
                        let at = (row * WIDTH + column) * 3;
                        page[at..at + 3].fill(0);
                    }
                }
                x += stroke + 3 + next(4);
            }
            x += 30 + next(20);
        }
        top += 70;
    }
    page
}

fn recognize(lease: &TesseractApiLease, page: &[u8], width: usize) {
    let height = page.len() / (width * 3);
    lease
        .set_image(page, width as i32, height as i32, 3, (width * 3) as i32)
        .expect("set image");
    lease.recognize().expect("recognize");
}

fn run_child(test_name: &str, value: &str) -> Output {
    Command::new(std::env::current_exe().expect("test binary path"))
        .args(["--exact", test_name, "--nocapture", "--test-threads=1"])
        .env(CHILD_ENV, value)
        .output()
        .expect("run the child process")
}

/// Runs in the child: lease an engine, recognize, return it, lease it again, and call
/// `exit()` while a worker recognizes with the reused engine.
#[allow(unsafe_code)] // ~keep: C `exit()` is the only way to run the atexit cleanup under test
fn race_child(exit_delay: Duration) -> ! {
    let directory = tempfile::tempdir().expect("temporary model directory");
    let models = english_models(&directory);
    let pool = TesseractApiPool::new();
    let page = synthetic_page();
    recognize(&pool.checkout(&models, "eng", 1).expect("first lease"), &page, WIDTH);

    let lease = pool
        .checkout(&models, "eng", 1)
        .expect("second lease reuses the parked engine");
    // The reused engine is in use. The check fails here, without any timing, when the
    // checkout leaves a parked engine marked idle.
    assert!(!lease.is_idle(), "a leased engine must not be marked idle");
    let (first_done, first_done_receiver) = mpsc::channel::<()>();
    std::thread::spawn(move || {
        let mut first = Some(first_done);
        loop {
            recognize(&lease, &page, WIDTH);
            if let Some(sender) = first.take() {
                let _ = sender.send(());
            }
        }
    });
    first_done_receiver
        .recv_timeout(Duration::from_secs(120))
        .expect("the worker finishes a recognition");
    say(RECOGNIZING);
    std::thread::sleep(exit_delay);
    unsafe { libc::exit(0) }
}

#[test]
fn exit_during_a_pool_leased_recognition_does_not_crash() {
    if let Ok(delay) = std::env::var(CHILD_ENV) {
        race_child(Duration::from_millis(delay.parse().expect("delay in milliseconds")));
    }
    let mut failures = Vec::new();
    for attempt in 0..4u64 {
        let output = run_child(RACE_TEST, &(200 + attempt * 331).to_string());
        assert!(
            String::from_utf8_lossy(&output.stdout).contains(RECOGNIZING),
            "attempt {attempt}: the child never reached its recognition loop. stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        if !output.status.success() {
            failures.push(format!(
                "attempt {attempt}: exit code {:?}, signal {:?}",
                output.status.code(),
                output.status.signal()
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "process exit crashed a child that leased a pooled engine in {} of 4 attempts: {failures:?}",
        failures.len()
    );
}

/// What the exit handler of the next test needs. The handler is a C function without
/// arguments, so it reads them from here.
static AFTER_CLEANUP_STATE: OnceLock<(Arc<TesseractApiPool>, super::super::model_context::ModelContext)> =
    OnceLock::new();

/// Runs at exit after the Tesseract cleanup: C runs exit handlers in reverse order of
/// registration, and the child registers this one before the cleanup registers itself.
extern "C" fn checkout_after_cleanup() {
    let Some((pool, models)) = AFTER_CLEANUP_STATE.get() else {
        return;
    };
    let counts = |pool: &Arc<TesseractApiPool>| pool.resources.counts();
    match pool.checkout(models, "eng", 1) {
        Ok(_lease) => say(&format!("{AFTER_CLEANUP} checkout=ok")),
        Err(error) => say(&format!(
            "{AFTER_CLEANUP} checkout=err counts={:?} message={error}",
            counts(pool)
        )),
    }
}

/// Runs in the child: park a recognized engine, then call `exit()`. The Tesseract cleanup
/// frees the parked engine, and `checkout_after_cleanup` asks the pool for it.
#[allow(unsafe_code)] // ~keep: C `exit()` and `atexit` are the only way to run the cleanup under test
fn after_cleanup_child() -> ! {
    // Registered first, so it runs after the cleanup that the first recognition registers.
    unsafe { libc::atexit(checkout_after_cleanup) };
    let directory = Box::leak(Box::new(tempfile::tempdir().expect("temporary model directory")));
    let models = english_models(directory);
    let pool = TesseractApiPool::new();
    let blank = vec![255u8; 3 * 200 * 100];
    recognize(&pool.checkout(&models, "eng", 1).expect("first lease"), &blank, 200);
    assert!(pool.resources.counts() == (0, 1), "the engine is parked");
    let _ = AFTER_CLEANUP_STATE.set((pool, models));
    unsafe { libc::exit(0) }
}

#[test]
fn a_checkout_after_the_exit_cleanup_fails_and_discards_the_engine() {
    if std::env::var_os(CHILD_ENV).is_some() {
        after_cleanup_child();
    }
    let output = run_child(AFTER_CLEANUP_TEST, "after-cleanup");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "the child failed: {:?}, stderr: {stderr}",
        output.status
    );
    assert!(
        stdout.contains(&format!("{AFTER_CLEANUP} checkout=err counts=(0, 0) message=")),
        "a checkout after the cleanup must fail and leave no engine in the pool. stdout: {stdout}, stderr: {stderr}"
    );
}
