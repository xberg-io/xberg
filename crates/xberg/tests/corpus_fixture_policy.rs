mod helpers;

use std::path::Path;
use std::process::{Command, Output};

const STRICT: &str = "XBERG_REQUIRE_TEST_FIXTURES";
const CHILD_FIXTURE: &str = "XBERG_CORPUS_FIXTURE_POLICY_CHILD_PATH";
const TEST_NAME: &str = "required_fixture_mode_follows_process_environment";

fn run_fixture_check(path: &Path, strict: Option<&str>) -> Output {
    let mut child = Command::new(std::env::current_exe().unwrap());
    child
        .args(["--exact", TEST_NAME, "--nocapture"])
        .env(CHILD_FIXTURE, path)
        .env_remove(STRICT);
    if let Some(value) = strict {
        child.env(STRICT, value);
    }
    child.output().expect("run the fixture check in a separate process")
}

fn assert_result(output: Output, expected: &str, success: bool) -> String {
    let report = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.status.success(), success, "{report}");
    assert!(report.contains(expected), "{report}");
    report
}

#[test]
fn required_fixture_mode_follows_process_environment() {
    if let Some(path) = std::env::var_os(CHILD_FIXTURE) {
        let path = Path::new(&path);
        assert_eq!(helpers::skip_if_missing(path.to_str().unwrap()), !path.is_file());
        return;
    }

    let directory = tempfile::tempdir().unwrap();
    let missing = directory.path().join("missing.pdf");
    let fixture = directory.path().join("sample.pdf");
    std::fs::write(&fixture, b"fixture").unwrap();
    let real_io_error = std::fs::metadata(&missing).unwrap_err().to_string();
    const PASSED: &str = "test result: ok. 1 passed; 0 failed";
    const FAILED: &str = "test result: FAILED. 0 passed; 1 failed";

    assert_result(run_fixture_check(&missing, None), PASSED, true);
    assert_result(run_fixture_check(&fixture, Some("1")), PASSED, true);
    for (path, source) in [
        (missing.as_path(), real_io_error.as_str()),
        (directory.path(), "expected a regular file"),
    ] {
        let report = assert_result(run_fixture_check(path, Some("1")), FAILED, false);
        assert!(report.contains("required test fixture unavailable"), "{report}");
        assert!(report.contains(&path.display().to_string()), "{report}");
        assert!(report.contains(source), "{report}");
    }
}
