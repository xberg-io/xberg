//! Process exit while engines are still in use must not crash.
//!
//! Each test runs its own binary as a child process that calls C `exit()`, which runs the
//! exit cleanup, and asserts that the child exits with status 0. A crash (SIGSEGV or
//! SIGABRT) means native code ran against freed engine memory or a destroyed tesseract
//! static. The engine loads the bundled English model, so `required-features` in
//! `Cargo.toml` builds this file only with `bundle-tessdata-eng`.

#![cfg(all(unix, not(target_arch = "wasm32")))]
#![allow(
    unsafe_code,
    reason = "the child calls C `exit()` while an engine is in use to reach the exit cleanup"
)]
#![allow(clippy::print_stdout)] // ~keep: the child reports its progress on stdout by design

use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, mpsc};
use std::time::{Duration, Instant};
use xberg_tesseract::TesseractAPI;

/// The failure message when the build lacks the bundled model. The test fails and does not skip.
const NEEDS_MODEL: &str = "these tests need the bundled English model: run them with --features bundle-tessdata-eng";

const CHILD_ENV: &str = "XBERG_TESSERACT_EXIT_RACE_CHILD";
const MODELS_ENV: &str = "XBERG_TESSERACT_EXIT_RACE_MODELS";
const TEST_NAME: &str = "process_exit_during_recognition_does_not_crash";
const ATTEMPTS: u64 = 6;
const DROP_TEST_NAME: &str = "dropping_a_busy_engine_after_exit_began_does_not_crash";
const STOP_TEST_NAME: &str = "exit_stops_a_recognition_in_flight";
const RECOGNIZING: &str = "RECOGNIZING";
const DROPPED: &str = "DROPPED";
const STOPPED: &str = "RECOGNITION-STOPPED";
const RAN_ON: &str = "RECOGNITION-RAN-ON";
/// How long the late exit hook waits for the worker to see its recognition fail. A
/// recognition that exit does not stop runs for longer: the worker restarts it in a loop.
const STOP_WAIT: Duration = Duration::from_secs(20);
const WIDTH: usize = 2400;
const HEIGHT: usize = 3200;

/// A temporary model directory with the bundled English pack. The parent keeps it until every child exits.
fn english_models() -> tempfile::TempDir {
    let directory = tempfile::tempdir().expect("temporary model directory");
    std::fs::write(
        directory.path().join("eng.traineddata"),
        xberg_tesseract::bundled_eng_traineddata().expect(NEEDS_MODEL),
    )
    .expect("write the English model");
    directory
}

/// A white RGB page with rows of stroke groups, so layout analysis finds text lines and
/// the recognizer has real work. The content is fixed by a small linear generator.
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

/// Runs in the child: recognize in a loop on a worker thread, then call `exit()` while the
/// worker is inside a later recognition. The first recognition has to finish first, because
/// the exit cleanup registers itself after the first successful recognition.
fn run_child(exit_delay: Duration) -> ! {
    let tessdata = PathBuf::from(std::env::var_os(MODELS_ENV).expect("the parent passes the model directory"));
    let (first_done, first_done_receiver) = mpsc::channel::<()>();
    std::thread::spawn(move || {
        let api = TesseractAPI::new().expect("create engine");
        api.init(&tessdata, "eng").expect("initialize engine");
        let page = synthetic_page();
        let mut first = Some(first_done);
        loop {
            api.set_image(&page, WIDTH as i32, HEIGHT as i32, 3, (WIDTH * 3) as i32)
                .expect("set image");
            // Recognition fails once exit has begun. The loop goes on using the engine, so a
            // cleanup that freed it would still crash the child.
            let recognized = api.recognize();
            if let Some(sender) = first.take() {
                recognized.expect("the first recognition");
                let _ = sender.send(());
            }
        }
    });
    first_done_receiver
        .recv_timeout(Duration::from_secs(120))
        .expect("the first recognition finishes");
    println!("{RECOGNIZING}");
    std::thread::sleep(exit_delay);
    unsafe { libc::exit(0) }
}

#[test]
fn process_exit_during_recognition_does_not_crash() {
    if std::env::var_os(CHILD_ENV).is_some() {
        let delay_ms: u64 = std::env::var(CHILD_ENV).unwrap().parse().unwrap();
        run_child(Duration::from_millis(delay_ms));
    }
    let models = english_models();
    let exe = std::env::current_exe().expect("test binary path");
    let mut failures = Vec::new();
    for attempt in 0..ATTEMPTS {
        let output = Command::new(&exe)
            .args(["--exact", TEST_NAME, "--nocapture", "--test-threads=1"])
            .env(CHILD_ENV, (200 + attempt * 331).to_string())
            .env(MODELS_ENV, models.path())
            .output()
            .expect("run the child process");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains(RECOGNIZING),
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
        "process exit during recognition crashed the child in {} of {ATTEMPTS} attempts: {failures:?}",
        failures.len()
    );
}

/// Prints one line and flushes it, so the parent sees it even when the child crashes later.
fn say(line: &str) {
    let mut out = std::io::stdout();
    let _ = writeln!(out, "{line}");
    let _ = out.flush();
}

/// Runs `test_name` in a child process with `mode` and the English models.
fn run_mode(test_name: &str, mode: &str) -> Output {
    let models = english_models();
    Command::new(std::env::current_exe().expect("test binary path"))
        .args(["--exact", test_name, "--nocapture", "--test-threads=1"])
        .env(CHILD_ENV, mode)
        .env(MODELS_ENV, models.path())
        .output()
        .expect("run the child process")
}

/// Asserts a clean exit and that the child printed `marker` before it ended. libtest prints
/// the test name on the same line as the child's first output, so this matches a substring.
fn assert_clean_exit(output: &Output, marker: &str) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "the child crashed: exit code {:?}, signal {:?}. stdout: {stdout}, stderr: {stderr}",
        output.status.code(),
        output.status.signal()
    );
    assert!(
        stdout.contains(marker),
        "the child never printed {marker}. stdout: {stdout}, stderr: {stderr}"
    );
}

/// The engine the late exit hook drops. Nothing marks it idle, so the exit cleanup skips it.
static BUSY_ENGINE: Mutex<Option<TesseractAPI>> = Mutex::new(None);

/// Drops the busy engine the way an abandoned OCR thread does when its page ends after
/// `exit()` has begun.
extern "C" fn drop_busy_engine() {
    let engine = BUSY_ENGINE.lock().map(|mut slot| slot.take()).unwrap_or(None);
    let had_engine = engine.is_some();
    drop(engine);
    if had_engine {
        say(DROPPED);
    }
}

/// Runs in the child: drop a busy engine from an exit hook that runs after tesseract's
/// `DawgCache` static is destroyed, then exit.
///
/// C runs exit handlers in reverse order of registration. The hook is registered before
/// the first engine loads a dictionary, which constructs the function-local `DawgCache`
/// and registers its destructor; the Tesseract cleanup registers itself after the first
/// recognition. So `exit()` runs the cleanup, then the `DawgCache` destructor, then the hook.
fn drop_child() -> ! {
    // SAFETY: `drop_busy_engine` is an `extern "C" fn()` that lives for the whole process.
    let registered = unsafe { libc::atexit(drop_busy_engine) };
    assert_eq!(registered, 0, "register the exit hook");
    let tessdata = PathBuf::from(std::env::var_os(MODELS_ENV).expect("the parent passes the model directory"));
    let api = TesseractAPI::new().expect("create engine");
    api.init(&tessdata, "eng").expect("initialize engine");
    let page = vec![255u8; 200 * 100 * 3];
    api.set_image(&page, 200, 100, 3, 200 * 3).expect("set image");
    api.recognize().expect("recognize");
    *BUSY_ENGINE.lock().expect("engine slot") = Some(api);
    // SAFETY: no other thread runs; `exit` runs the registered handlers and ends the process.
    unsafe { libc::exit(0) }
}

#[test]
fn dropping_a_busy_engine_after_exit_began_does_not_crash() {
    if let Ok(mode) = std::env::var(CHILD_ENV) {
        assert_eq!(mode, "drop", "the parent runs this test's child with the drop mode");
        drop_child();
    }
    assert_clean_exit(&run_mode(DROP_TEST_NAME, "drop"), DROPPED);
}

/// Set by the worker once a recognition fails after the first one succeeded.
static RECOGNITION_STOPPED: AtomicBool = AtomicBool::new(false);

/// Waits for the worker to report that its recognition stopped, and says whether it did.
/// It runs after the Tesseract cleanup, for the reason given on [`drop_child`].
extern "C" fn report_stopped_recognition() {
    let deadline = Instant::now() + STOP_WAIT;
    while !RECOGNITION_STOPPED.load(Ordering::SeqCst) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    say(if RECOGNITION_STOPPED.load(Ordering::SeqCst) {
        STOPPED
    } else {
        RAN_ON
    });
}

/// Runs in the child: a worker recognizes a large page in a loop through the cancellable
/// path, with a stop check that never asks to stop, and the main thread calls `exit()`
/// while the worker is inside a recognition. The worker keeps its engine and parks once a
/// recognition fails, which only exit can cause.
fn stop_child() -> ! {
    // SAFETY: `report_stopped_recognition` is an `extern "C" fn()` that lives for the whole process.
    let registered = unsafe { libc::atexit(report_stopped_recognition) };
    assert_eq!(registered, 0, "register the exit hook");
    let tessdata = PathBuf::from(std::env::var_os(MODELS_ENV).expect("the parent passes the model directory"));
    let (first_done, first_done_receiver) = mpsc::channel::<()>();
    std::thread::spawn(move || {
        let api = TesseractAPI::new().expect("create engine");
        api.init(&tessdata, "eng").expect("initialize engine");
        let page = synthetic_page();
        let mut first = Some(first_done);
        loop {
            api.set_image(&page, WIDTH as i32, HEIGHT as i32, 3, (WIDTH * 3) as i32)
                .expect("set image");
            let recognized = api.recognize_with_cancel(|| false);
            if let Some(sender) = first.take() {
                recognized.expect("the first recognition");
                let _ = sender.send(());
            } else if recognized.is_err() {
                RECOGNITION_STOPPED.store(true, Ordering::SeqCst);
                loop {
                    std::thread::park();
                }
            }
        }
    });
    first_done_receiver
        .recv_timeout(Duration::from_secs(120))
        .expect("the first recognition finishes");
    say(RECOGNIZING);
    std::thread::sleep(Duration::from_millis(300));
    // SAFETY: `exit` runs the registered handlers and ends the process; the worker keeps
    // its engine, which the exit cleanup skips as in use.
    unsafe { libc::exit(0) }
}

#[test]
fn exit_stops_a_recognition_in_flight() {
    if let Ok(mode) = std::env::var(CHILD_ENV) {
        assert_eq!(mode, "stop", "the parent runs this test's child with the stop mode");
        stop_child();
    }
    let output = run_mode(STOP_TEST_NAME, "stop");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(RECOGNIZING),
        "the child never reached its recognition loop. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_clean_exit(&output, STOPPED);
}
