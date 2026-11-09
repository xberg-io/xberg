#[path = "common/refusals.rs"]
mod cases;
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

#[test]
fn freeze_refuses_each_bad_plan_without_writing_a_frozen_artifact() {
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("input.json");
    let output = directory.path().join("frozen.json");
    for case in cases::plan_cases() {
        std::fs::write(&input, serde_json::to_vec(&case.input).unwrap()).unwrap();
        let result = Command::new(env!("CARGO_BIN_EXE_nli-qualification"))
            .arg("freeze")
            .arg(&input)
            .arg(&output)
            .output()
            .unwrap();
        assert!(!result.status.success(), "accepted {}", case.name);
        let error = String::from_utf8(result.stderr).unwrap();
        assert!(error.contains(case.error), "{}: {error}", case.name);
        assert!(result.stdout.is_empty(), "printed success for {}", case.name);
        assert!(!output.exists(), "wrote frozen artifact for {}", case.name);
    }
}

#[test]
fn validate_refuses_each_tampered_report_through_the_public_cli() {
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("plan.json");
    let report = directory.path().join("report.json");
    let plan = common::plan();
    std::fs::write(&input, serde_json::to_vec(&plan).unwrap()).unwrap();
    for case in cases::report_cases() {
        std::fs::write(&report, serde_json::to_vec(&case.input).unwrap()).unwrap();
        let result = Command::new(env!("CARGO_BIN_EXE_nli-qualification"))
            .arg("validate")
            .arg(&input)
            .arg(plan.sha256().unwrap())
            .arg(&report)
            .output()
            .unwrap();
        assert!(!result.status.success(), "accepted {}", case.name);
        let error = String::from_utf8(result.stderr).unwrap();
        assert!(error.contains(case.error), "{}: {error}", case.name);
        assert!(result.stdout.is_empty(), "printed success for {}", case.name);
    }
}

#[test]
fn cli_refuses_empty_truncated_and_duplicate_key_plan_and_report_files() {
    let directory = tempfile::tempdir().unwrap();
    let plan = common::plan();
    let plan_path = directory.path().join("plan.json");
    let invalid = directory.path().join("invalid.json");
    let frozen = directory.path().join("frozen.json");
    std::fs::write(&plan_path, serde_json::to_vec(&plan).unwrap()).unwrap();
    for command in ["freeze", "validate"] {
        let value = if command == "freeze" {
            serde_json::to_value(&plan).unwrap()
        } else {
            serde_json::to_value(common::report(&plan)).unwrap()
        };
        let duplicate = serde_json::to_string(&value)
            .unwrap()
            .replace("\"qualified\":false", "\"qualified\":false,\"qualified\":true");
        for (bytes, expected) in [
            ("".to_string(), "EOF"),
            ("{".to_string(), "EOF"),
            (duplicate, "duplicate JSON key: qualified"),
        ] {
            std::fs::write(&invalid, bytes).unwrap();
            let mut invocation = Command::new(env!("CARGO_BIN_EXE_nli-qualification"));
            invocation.arg(command);
            if command == "freeze" {
                invocation.arg(&invalid).arg(&frozen);
            } else {
                invocation.arg(&plan_path).arg(plan.sha256().unwrap()).arg(&invalid);
            }
            let result = invocation.output().unwrap();
            assert!(!result.status.success(), "{command} accepted {expected}");
            let error = String::from_utf8(result.stderr).unwrap();
            assert!(error.contains(expected), "{command}: expected {expected}, got {error}");
            assert!(result.stdout.is_empty());
            assert!(!frozen.exists());
        }
    }
}
