//! The process-exit cleanup frees an idle engine, so tesseract reports no leaked dictionary
//! on exit, and skips an engine in use, which stays leaked until the operating system
//! reclaims it. Each test runs its own binary as a child process that calls C `exit()` with
//! an engine alive. The engine loads the bundled English model, so the tests always run.

#![cfg(all(unix, feature = "build-tesseract", not(target_arch = "wasm32")))]

use std::path::PathBuf;
use std::process::Command;
use xberg_tesseract::TesseractAPI;

/// The failure message when the build lacks the bundled model. The test fails and does not skip.
const NEEDS_MODEL: &str = "these tests need the bundled English model: run them with --features bundle-tessdata-eng";

const CHILD_ENV: &str = "XBERG_TESSERACT_EXIT_LEAK_CHILD";
const MODELS_ENV: &str = "XBERG_TESSERACT_EXIT_LEAK_MODELS";
const LEAK_REPORT: &str = "LEAK";

/// A temporary model directory with the bundled English pack. The parent keeps it until the child exits.
fn english_models() -> tempfile::TempDir {
    let directory = tempfile::tempdir().expect("temporary model directory");
    std::fs::write(
        directory.path().join("eng.traineddata"),
        xberg_tesseract::bundled_eng_traineddata().expect(NEEDS_MODEL),
    )
    .expect("write the English model");
    directory
}

/// Runs in the child: recognize once, optionally park the engine as idle, and call `exit()`
/// with the engine still alive, as a pool that nobody drops does.
fn run_child(park_as_idle: bool) -> ! {
    let models = PathBuf::from(std::env::var_os(MODELS_ENV).expect("the parent passes the model directory"));
    let api = TesseractAPI::new().expect("create engine");
    api.init(models, "eng").expect("initialize engine");
    let page = vec![255u8; 200 * 100 * 3];
    api.set_image(&page, 200, 100, 3, 200 * 3).expect("set image");
    api.recognize().expect("recognize");
    if park_as_idle {
        // SAFETY: this child process runs no other thread that uses the engine.
        unsafe { api.mark_idle() };
    }
    unsafe { libc::exit(0) }
}

/// Runs the test named `test_name` as a child and returns its status and stderr.
fn exit_with_engine(test_name: &str, park_as_idle: bool) -> (std::process::ExitStatus, String) {
    let models = english_models();
    let exe = std::env::current_exe().expect("test binary path");
    let output = Command::new(exe)
        .args(["--exact", test_name, "--nocapture", "--test-threads=1"])
        .env(CHILD_ENV, if park_as_idle { "idle" } else { "busy" })
        .env(MODELS_ENV, models.path())
        .output()
        .expect("run the child process");
    (output.status, String::from_utf8_lossy(&output.stderr).into_owned())
}

#[test]
fn an_idle_engine_is_freed_at_exit_and_nothing_leaks() {
    if let Ok(mode) = std::env::var(CHILD_ENV) {
        run_child(mode == "idle");
    }
    let (status, stderr) = exit_with_engine("an_idle_engine_is_freed_at_exit_and_nothing_leaks", true);
    assert!(status.success(), "the child failed: {status:?}, stderr: {stderr}");
    // The busy test below proves the report text still appears for a leaked engine, so
    // this absence check can fail.
    assert!(
        !stderr.contains(LEAK_REPORT),
        "tesseract reported a leaked dictionary: {stderr}"
    );
}

#[test]
fn an_engine_in_use_is_skipped_at_exit_and_stays_leaked() {
    if let Ok(mode) = std::env::var(CHILD_ENV) {
        run_child(mode == "idle");
    }
    let (status, stderr) = exit_with_engine("an_engine_in_use_is_skipped_at_exit_and_stays_leaked", false);
    assert!(status.success(), "the child failed: {status:?}, stderr: {stderr}");
    assert!(
        stderr.contains(LEAK_REPORT),
        "the cleanup freed an engine in use, or tesseract printed no leak report to detect: {stderr}"
    );
}
