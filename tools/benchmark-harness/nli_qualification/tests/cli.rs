use std::process::Command;
mod common;

#[cfg(not(feature = "runtime"))]
#[test]
fn run_without_runtime_feature_is_an_explicit_failure_not_skipped_success() {
    let result = Command::new(env!("CARGO_BIN_EXE_nli-qualification"))
        .args(["run", "missing-plan", "digest", "unused-output"])
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(
        String::from_utf8(result.stderr)
            .unwrap()
            .contains("no inference was performed")
    );
}

#[test]
fn freeze_and_validate_bind_actual_files_and_refuse_overwrite_or_wrong_digest() {
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("input.json");
    let frozen = directory.path().join("frozen.json");
    let report = directory.path().join("report.json");
    let plan = common::plan();
    std::fs::write(&input, serde_json::to_vec(&plan).unwrap()).unwrap();
    std::fs::write(&report, serde_json::to_vec(&common::report(&plan)).unwrap()).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_nli-qualification"))
        .arg("freeze")
        .arg(&input)
        .arg(&frozen)
        .output()
        .unwrap();
    assert!(output.status.success(), "{:?}", output);
    let sha = String::from_utf8(output.stdout).unwrap();
    assert_eq!(sha.trim(), plan.sha256().unwrap());
    let output = Command::new(env!("CARGO_BIN_EXE_nli-qualification"))
        .arg("freeze")
        .arg(&input)
        .arg(&frozen)
        .output()
        .unwrap();
    assert!(!output.status.success());
    let output = Command::new(env!("CARGO_BIN_EXE_nli-qualification"))
        .arg("validate")
        .arg(&frozen)
        .arg(sha.trim())
        .arg(&report)
        .output()
        .unwrap();
    assert!(output.status.success(), "{:?}", output);
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("4 / 4 inferences; qualified=false")
    );
    let output = Command::new(env!("CARGO_BIN_EXE_nli-qualification"))
        .arg("validate")
        .arg(&frozen)
        .arg("0".repeat(64))
        .arg(&report)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8(output.stderr).unwrap().contains("plan digest"));
}
