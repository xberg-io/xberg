//! Integration tests verifying that malformed --log-level arguments do not panic the CLI.

use std::process::Command;

/// Get the path to the kreuzberg binary.
fn get_binary_path() -> &'static str {
    env!("CARGO_BIN_EXE_kreuzberg")
}

#[test]
fn malformed_log_level_does_not_panic() {
    let output = Command::new(get_binary_path())
        .args(["--log-level", "garbage:::invalid", "--help"])
        .output()
        .expect("failed to spawn kreuzberg binary");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("panicked"),
        "binary panicked on malformed --log-level: {stderr}"
    );
}
