//! The process-exit cleanup must not free an engine that a thread still uses.
//!
//! The test runs its own binary as a child process. In the child, a worker thread
//! recognizes a synthetic page in a loop, and the main thread calls C `exit()` while the
//! worker is inside a recognition. `exit()` runs the exit cleanup. The child must exit
//! with status 0. A crash (SIGSEGV or SIGABRT) means the cleanup freed the engine under
//! the worker. The engine loads the bundled English model, so the test always runs.

#![cfg(all(unix, feature = "build-tesseract", not(target_arch = "wasm32")))]
#![allow(clippy::print_stdout)] // ~keep: the child reports its progress on stdout by design

use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc;
use std::time::Duration;
use xberg_tesseract::TesseractAPI;

/// The failure message when the build lacks the bundled model. The test fails and does not skip.
const NEEDS_MODEL: &str = "these tests need the bundled English model: run them with --features bundle-tessdata-eng";

const CHILD_ENV: &str = "XBERG_TESSERACT_EXIT_RACE_CHILD";
const MODELS_ENV: &str = "XBERG_TESSERACT_EXIT_RACE_MODELS";
const TEST_NAME: &str = "process_exit_during_recognition_does_not_crash";
const ATTEMPTS: u64 = 6;
const RECOGNIZING: &str = "RECOGNIZING";
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
            api.recognize().expect("recognize");
            if let Some(sender) = first.take() {
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
