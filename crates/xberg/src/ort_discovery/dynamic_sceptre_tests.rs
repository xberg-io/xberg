use std::error::Error;
use std::path::Path;
use std::process::Command;

use super::library::SearchPolicy;
use super::{OrtRuntimeError, preload_ort_runtime_from};

const CHILD_TEST: &str = "XBERG_DISCOVERY_TEST_CHILD";
const MISSING_LIBRARY: &str = "xberg-qualification-missing-runtime";
const CHILD_RUNTIME: &str = "XBERG_DISCOVERY_TEST_CHILD_RUNTIME";
const EXPLICIT_RUNTIME: &str = "ORT_DYLIB_PATH";

#[test]
#[ignore = "requires ORT_DYLIB_PATH to name a verified native ONNX Runtime library"]
fn discovered_runtime_loads_without_overwriting_the_environment() {
    let test = "discovered_runtime_loads_without_overwriting_the_environment";
    if std::env::var(CHILD_TEST).as_deref() == Ok(test) {
        let before = std::env::var_os(EXPLICIT_RUNTIME);
        assert_eq!(before.as_deref(), Some(std::ffi::OsStr::new(MISSING_LIBRARY)));
        let runtime = std::env::var_os(CHILD_RUNTIME).expect("validated parent runtime");
        let temp = tempfile::tempdir().unwrap();
        let missing = temp.path().join("missing");
        let broken = temp.path().join("broken-library");
        std::fs::write(&broken, []).unwrap();
        // The broken library before the real one proves the search moves past a failed load.
        let candidates = [missing.as_path(), broken.as_path(), Path::new(&runtime)];
        let loaded = preload_ort_runtime_from(None, None, &candidates, SearchPolicy::Strict)
            .expect("load the first runtime that loads");
        assert_eq!(loaded, std::fs::canonicalize(&runtime).unwrap());
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

#[cfg(unix)]
#[test]
#[ignore = "requires ORT_DYLIB_PATH to name a verified native ONNX Runtime library"]
fn library_consumers_fall_back_to_the_platform_library_search_path() {
    let test = "library_consumers_fall_back_to_the_platform_library_search_path";
    if std::env::var(CHILD_TEST).as_deref() == Ok(test) {
        let name = super::library::ORT_LIBRARY_NAMES[0];
        let loaded = preload_ort_runtime_from(None, None, &[], SearchPolicy::LoaderFallback)
            .expect("the platform loader finds the runtime on its search path");
        assert_eq!(loaded, Path::new(name));
        sceptre::ort_environment().expect("start the shared environment with the runtime the loader found");
        return;
    }

    let runtime = std::env::var_os(EXPLICIT_RUNTIME).expect("set ORT_DYLIB_PATH to a verified native runtime");
    assert!(Path::new(&runtime).is_absolute() && Path::new(&runtime).is_file());
    let search_path = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(&runtime, search_path.path().join(super::library::ORT_LIBRARY_NAMES[0])).unwrap();
    let mut child = child_process(test);
    child.arg("--ignored");
    for variable in ["LD_LIBRARY_PATH", "DYLD_LIBRARY_PATH"] {
        child.env(variable, search_path.path());
    }
    assert_single_test_passes(&mut child);
}

#[test]
fn broken_candidate_reports_a_typed_error_before_ort_initialization() {
    let test = "broken_candidate_reports_a_typed_error_before_ort_initialization";
    if std::env::var(CHILD_TEST).as_deref() == Ok(test) {
        let temp = tempfile::tempdir().unwrap();
        let broken = temp.path().join("broken-library");
        std::fs::write(&broken, []).unwrap();
        let broken = std::fs::canonicalize(&broken).unwrap();
        let error = preload_ort_runtime_from(None, None, &[broken.as_path()], SearchPolicy::Strict)
            .expect_err("a broken library must produce a typed loader error");
        assert!(
            matches!(&error, OrtRuntimeError::Load { path, .. } if path == &broken),
            "{error}"
        );
        let contextual = crate::XbergError::ocr_with_source("Could not load ONNX Runtime", error);
        let runtime_error = contextual
            .source()
            .and_then(|cause| cause.downcast_ref::<OrtRuntimeError>())
            .expect("the runtime error is the OCR error's source");
        assert!(matches!(
            runtime_error.source().and_then(|cause| cause.downcast_ref::<ort::LoadDynamicError>()),
            Some(ort::LoadDynamicError::Dlopen { path, .. }) if path == &broken
        ));
        assert_eq!(std::env::var(EXPLICIT_RUNTIME).as_deref(), Ok(MISSING_LIBRARY));
        return;
    }

    assert_single_test_passes(&mut child_process(test));
}

#[test]
fn missing_explicit_library_fails_instead_of_loading_a_discovered_one() {
    let test = "missing_explicit_library_fails_instead_of_loading_a_discovered_one";
    if std::env::var(CHILD_TEST).as_deref() == Ok(test) {
        let temp = tempfile::tempdir().unwrap();
        let explicit = temp.path().join("missing-explicit");
        let discovered = temp.path().join("discovered");
        std::fs::write(&discovered, []).unwrap();
        let error = preload_ort_runtime_from(
            Some(explicit.as_os_str()),
            None,
            &[discovered.as_path()],
            SearchPolicy::LoaderFallback,
        )
        .expect_err("a missing explicit library must not fall back");
        assert!(
            matches!(&error, OrtRuntimeError::Load { path, .. } if path == &explicit),
            "{error}"
        );
        assert!(error.to_string().contains(&explicit.display().to_string()), "{error}");
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
