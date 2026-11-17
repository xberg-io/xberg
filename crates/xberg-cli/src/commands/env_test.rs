use std::process::Command;

pub(super) const CASE: &str = "XBERG_CLI_ENV_TEST_CASE";
pub(super) const OBSERVED: &str = "CLI environment assertion ran";
pub(super) const MISMATCH: &str = "CLI environment assertion failed";

pub(super) fn run_case(test: &str, case: &str, variable: &str, value: Option<&str>, expect_success: bool) {
    let mut command = Command::new(std::env::current_exe().expect("find the test binary"));
    command
        .args([
            "--exact",
            test,
            "--ignored",
            "--nocapture",
            "--test-threads=1",
            "--color",
            "never",
        ])
        .env(CASE, case)
        .env_remove(variable);
    if let Some(value) = value {
        command.env(variable, value);
    }
    let output = command.output().expect("start the isolated test");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let (result, count) = if expect_success {
        ("ok", "test result: ok. 1 passed; 0 failed; 0 ignored;")
    } else {
        ("FAILED", "test result: FAILED. 0 passed; 1 failed; 0 ignored;")
    };
    let line = format!("test {test} ... {result}");
    assert!(
        output.status.success() == expect_success
            && stdout.lines().any(|actual| actual == line)
            && stdout.lines().any(|actual| actual.starts_with(count))
            && stderr.lines().any(|actual| actual == OBSERVED)
            && stderr.contains(MISMATCH) != expect_success,
        "unexpected result from isolated test {test} ({case}): {}",
        output.status,
    );
}
