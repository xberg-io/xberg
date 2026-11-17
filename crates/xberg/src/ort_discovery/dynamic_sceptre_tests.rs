use std::error::Error;
use std::path::Path;
use std::process::Command;

use super::preload_sceptre_ort_runtime_from;

const CHILD_TEST: &str = "XBERG_DISCOVERY_TEST_CHILD";
const MISSING_LIBRARY: &str = "xberg-qualification-missing-runtime";
const CHILD_RUNTIME: &str = "XBERG_DISCOVERY_TEST_CHILD_RUNTIME";
const EXPLICIT_RUNTIME: &str = "ORT_DYLIB_PATH";

#[test]
fn valid_explicit_path_takes_precedence_over_an_invalid_candidate() {
    let temp = tempfile::tempdir().unwrap();
    let explicit = temp.path().join("explicit");
    let candidate = temp.path().join("candidate");
    std::fs::write(&explicit, []).unwrap();
    std::fs::write(&candidate, []).unwrap();

    preload_sceptre_ort_runtime_from(Some(&explicit), std::iter::once(candidate.as_path()))
        .expect("leave resolution of an existing explicit library to ORT");
}

#[test]
fn missing_candidate_leaves_bare_name_to_the_loader() {
    let temp = tempfile::tempdir().unwrap();
    let missing = temp.path().join("not-installed");

    preload_sceptre_ort_runtime_from(Some(Path::new("onnxruntime-name")), std::iter::once(missing.as_path()))
        .expect("continue with the loader's normal search");
}

#[test]
#[ignore = "requires ORT_DYLIB_PATH to name a verified native ONNX Runtime library"]
fn discovered_runtime_loads_without_overwriting_the_environment() {
    let test = "discovered_runtime_loads_without_overwriting_the_environment";
    if std::env::var(CHILD_TEST).as_deref() == Ok(test) {
        let before = std::env::var_os(EXPLICIT_RUNTIME);
        assert_eq!(before.as_deref(), Some(std::ffi::OsStr::new(MISSING_LIBRARY)));
        let runtime = std::env::var_os(CHILD_RUNTIME).expect("validated parent runtime");
        let temp = tempfile::tempdir().unwrap();
        let first = temp.path().join("missing");
        let last = temp.path().join("broken-library");
        std::fs::write(&last, []).unwrap();
        let candidates = [first.as_path(), Path::new(&runtime), last.as_path()];
        preload_sceptre_ort_runtime_from(Some(Path::new(MISSING_LIBRARY)), candidates.into_iter())
            .expect("locate and preload the first existing runtime");
        sceptre::ort_environment().expect("start the shared environment with the discovered runtime");
        assert_eq!(std::env::var_os(EXPLICIT_RUNTIME), before);
        return;
    }

    let runtime = std::env::var_os(EXPLICIT_RUNTIME).expect("set ORT_DYLIB_PATH to a verified native runtime");
    assert!(Path::new(&runtime).is_absolute() && Path::new(&runtime).is_file());
    let mut child = child_process(test);
    child.arg("--ignored");
    child.env(CHILD_RUNTIME, runtime);
    assert_single_test_passes(&mut child);
}

#[test]
fn broken_candidate_reports_a_typed_error_before_ort_initialization() {
    let test = "broken_candidate_reports_a_typed_error_before_ort_initialization";
    if std::env::var(CHILD_TEST).as_deref() == Ok(test) {
        let temp = tempfile::tempdir().unwrap();
        let broken = temp.path().join("broken-library");
        std::fs::write(&broken, []).unwrap();
        let explicit = std::env::var(EXPLICIT_RUNTIME).expect("child explicit library");
        assert_eq!(explicit, MISSING_LIBRARY);
        let error = preload_sceptre_ort_runtime_from(Some(Path::new(&explicit)), std::iter::once(broken.as_path()))
            .expect_err("a broken library must produce a typed loader error");
        let contextual = crate::XbergError::ocr_with_source("Could not preload the discovered ONNX Runtime", error);
        assert!(matches!(
            contextual.source().and_then(|cause| cause.downcast_ref::<ort::LoadDynamicError>()),
            Some(ort::LoadDynamicError::Dlopen { path, .. }) if path == &broken
        ));
        assert_eq!(std::env::var(EXPLICIT_RUNTIME).as_deref(), Ok(MISSING_LIBRARY));
        return;
    }

    assert_single_test_passes(&mut child_process(test));
}

fn child_process(test: &str) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", &format!("ort_discovery::dynamic_sceptre_tests::{test}")])
        .env(EXPLICIT_RUNTIME, MISSING_LIBRARY)
        .env(CHILD_TEST, test);
    command
}

fn assert_single_test_passes(child: &mut Command) {
    let output = child.output().unwrap();
    assert!(
        output.status.success() && String::from_utf8_lossy(&output.stdout).contains("1 passed; 0 failed"),
        "isolated discovery test failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}
