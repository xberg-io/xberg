use std::path::Path;
use std::process::Command;

use super::preload_sceptre_ort_runtime_from;

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
fn discovered_runtime_loads_without_overwriting_the_environment() {
    let missing = "xberg-qualification-missing-runtime";
    let temp = tempfile::tempdir().unwrap();
    if let Some(runtime) = std::env::var_os(CHILD_RUNTIME) {
        let before = std::env::var_os(EXPLICIT_RUNTIME);
        assert_eq!(before.as_deref(), Some(std::ffi::OsStr::new(missing)));
        let first = temp.path().join("missing");
        let last = temp.path().join("broken-library");
        std::fs::write(&last, []).unwrap();
        let candidates = [first.as_path(), Path::new(&runtime), last.as_path()];
        preload_sceptre_ort_runtime_from(Some(Path::new(missing)), candidates.into_iter())
            .expect("locate and preload the first existing runtime");
        sceptre::ort_environment().expect("start the shared environment with the discovered runtime");
        assert_eq!(std::env::var_os(EXPLICIT_RUNTIME), before);
        return;
    }

    let runtime = std::env::var_os(EXPLICIT_RUNTIME).expect("set ORT_DYLIB_PATH to a verified native runtime");
    assert!(Path::new(&runtime).is_absolute() && Path::new(&runtime).is_file());
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "ort_discovery::dynamic_sceptre_tests::discovered_runtime_loads_without_overwriting_the_environment",
        ])
        .env(EXPLICIT_RUNTIME, missing)
        .env(CHILD_RUNTIME, runtime)
        .output()
        .unwrap();
    assert!(
        output.status.success() && String::from_utf8_lossy(&output.stdout).contains("1 passed; 0 failed"),
        "isolated discovery test failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}
