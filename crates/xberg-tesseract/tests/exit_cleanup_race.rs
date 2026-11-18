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
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, mpsc};
use std::time::{Duration, Instant};
use xberg_tesseract::{TessPageSegMode, TesseractAPI};

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
const FAILED_ITERATION: &str = "FAILED-ITERATION";
/// The recognition the main thread exits during. The first one has to finish, because the
/// exit cleanup registers itself after the first successful recognition.
const EXIT_ITERATION: u64 = 2;
/// How long the child waits for each step it expects of another thread.
const STEP_WAIT: Duration = Duration::from_secs(120);
/// A page of printed text. Layout analysis finds words on it, so recognition reaches its
/// stop check before each one.
const TEXT_PAGE: &[u8] = include_bytes!("../../xberg/tests/fixtures/ocr/embedded_psm6_panel_2327.png");
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
///
/// It prints how many engines `Drop` freed meanwhile. On a C++ runtime where using the
/// destroyed `DawgCache` does not crash, that count is what tells a freed engine from a
/// leaked one.
extern "C" fn drop_busy_engine() {
    let engine = BUSY_ENGINE.lock().map(|mut slot| slot.take()).unwrap_or(None);
    let had_engine = engine.is_some();
    let freed_before = TesseractAPI::engines_freed_by_drop();
    drop(engine);
    let freed = TesseractAPI::engines_freed_by_drop() - freed_before;
    if had_engine {
        say(&format!("{DROPPED} freed={freed};"));
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
    assert_clean_exit(&run_mode(DROP_TEST_NAME, "drop"), &format!("{DROPPED} freed=0;"));
}

/// The recognition the worker's stop check last saw in flight.
static IN_FLIGHT_ITERATION: AtomicU64 = AtomicU64::new(0);
/// The recognition that failed, or 0 while none has.
static FAILED_AT: AtomicU64 = AtomicU64::new(0);

/// Waits until `ready` holds or [`STEP_WAIT`] has passed, and returns whether it holds.
fn wait_for(ready: impl Fn() -> bool) -> bool {
    let deadline = Instant::now() + STEP_WAIT;
    while !ready() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    ready()
}

/// Waits for the worker to report a failed recognition, and prints which one failed. It
/// runs after the Tesseract cleanup, for the reason given on [`drop_child`].
extern "C" fn report_failed_iteration() {
    wait_for(|| FAILED_AT.load(Ordering::SeqCst) != 0);
    say(&format!("{FAILED_ITERATION}={};", FAILED_AT.load(Ordering::SeqCst)));
}

/// Runs in the child: a worker recognizes a page of text in a loop through the cancellable
/// path, with a stop check that never asks to stop, and the main thread calls `exit()`
/// while recognition [`EXIT_ITERATION`] is in flight.
///
/// In that recognition the stop check reports the iteration and then holds the worker
/// until the exit cleanup has started, so the recognition cannot finish first. Only the
/// cleanup can then stop it, at the next word: the page has more than one. Refusing the next recognition alone would
/// fail iteration `EXIT_ITERATION + 1` instead. The worker keeps its engine and parks
/// after the first failure.
fn stop_child() -> ! {
    // SAFETY: `report_failed_iteration` is an `extern "C" fn()` that lives for the whole process.
    let registered = unsafe { libc::atexit(report_failed_iteration) };
    assert_eq!(registered, 0, "register the exit hook");
    let tessdata = PathBuf::from(std::env::var_os(MODELS_ENV).expect("the parent passes the model directory"));
    std::thread::spawn(move || {
        let api = TesseractAPI::new().expect("create engine");
        api.init(&tessdata, "eng").expect("initialize engine");
        api.set_page_seg_mode(TessPageSegMode::PSM_SINGLE_BLOCK)
            .expect("set page segmentation mode");
        let page = image::load_from_memory(TEXT_PAGE)
            .expect("decode the text page")
            .to_rgb8();
        let (width, height) = (page.width() as i32, page.height() as i32);
        let mut iteration = 0;
        loop {
            iteration += 1;
            api.set_image(page.as_raw(), width, height, 3, width * 3)
                .expect("set image");
            let recognized = api.recognize_with_cancel(|| {
                if iteration == EXIT_ITERATION && IN_FLIGHT_ITERATION.swap(iteration, Ordering::SeqCst) != iteration {
                    wait_for(TesseractAPI::exit_cleanup_started);
                }
                false
            });
            if iteration == 1 {
                recognized.expect("the first recognition");
            } else if recognized.is_err() {
                FAILED_AT.store(iteration, Ordering::SeqCst);
                loop {
                    std::thread::park();
                }
            }
        }
    });
    assert!(
        wait_for(|| IN_FLIGHT_ITERATION.load(Ordering::SeqCst) == EXIT_ITERATION),
        "recognition {EXIT_ITERATION} reaches its stop check"
    );
    say(RECOGNIZING);
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
        "the child never reached recognition {EXIT_ITERATION}. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_clean_exit(&output, &format!("{FAILED_ITERATION}={EXIT_ITERATION};"));
}
