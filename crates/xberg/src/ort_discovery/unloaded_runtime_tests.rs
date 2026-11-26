//! Session entry points with an unloadable runtime must return errors, never panic, and never
//! let ort fall back to its own name-based `dlopen`. Each case runs in a child process because
//! the load outcome is cached per process.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::process::Command;

const CHILD: &str = "XBERG_UNLOADED_RUNTIME_CHILD";
const EXPLICIT_RUNTIME: &str = "ORT_DYLIB_PATH";

#[test]
fn every_session_entry_point_errors_without_loading_a_library() {
    let test = "every_session_entry_point_errors_without_loading_a_library";
    if std::env::var(CHILD).as_deref() == Ok(test) {
        let missing = PathBuf::from(std::env::var_os(EXPLICIT_RUNTIME).expect("parent sets the explicit path"));
        assert!(!missing.exists());
        assert_entry_points_fail_cleanly(&missing);
        let loaded = loaded_onnxruntime_images();
        assert!(loaded.is_empty(), "an ONNX Runtime library was mapped: {loaded:?}");
        return;
    }

    let temp = tempfile::tempdir().expect("create a directory for the missing runtime");
    let output = Command::new(std::env::current_exe().expect("locate the test executable"))
        .args([
            "--exact",
            &format!("ort_discovery::unloaded_runtime_tests::{test}"),
            "--nocapture",
        ])
        .env(EXPLICIT_RUNTIME, temp.path().join("libonnxruntime-missing"))
        .env(CHILD, test)
        .output()
        .expect("run the isolated entry-point test");
    assert!(
        output.status.success() && String::from_utf8_lossy(&output.stdout).contains("1 passed; 0 failed"),
        "isolated entry-point test failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

/// Runs `call` and fails the test, naming `entry`, if it panics instead of returning.
fn no_panic<T>(entry: &str, call: impl FnOnce() -> T) -> T {
    catch_unwind(AssertUnwindSafe(call)).unwrap_or_else(|_| panic!("{entry} panicked instead of returning an error"))
}

fn assert_names_missing_path(entry: &str, message: &str, missing: &Path) {
    assert!(
        message.contains(&missing.display().to_string()),
        "{entry} error does not name the explicit path: {message}"
    );
}

fn assert_entry_points_fail_cleanly(missing: &Path) {
    let preload = no_panic("preload_ort_runtime", super::preload_ort_runtime).expect_err("nothing loadable");
    assert_names_missing_path("preload_ort_runtime", &preload.to_string(), missing);

    for attempt in ["first", "cached"] {
        let entry = format!("ort_environment ({attempt})");
        let error = no_panic(&entry, super::ort_environment).expect_err("nothing loadable");
        assert!(matches!(error, super::OrtSessionError::Runtime(_)), "{entry}: {error}");
        assert_names_missing_path(&entry, &error.to_string(), missing);
    }

    no_panic("ensure_ort_available", super::ensure_ort_available);

    #[cfg(feature = "layout-detection")]
    {
        let error = no_panic("layout::session::session_builder", || {
            crate::layout::session::session_builder(None, 1).map(|_| ())
        })
        .expect_err("nothing loadable");
        assert!(
            matches!(error, super::OrtSessionError::Runtime(_)),
            "layout session_builder: {error}"
        );
        let error = no_panic("layout::session::build_session", || {
            crate::layout::session::build_session("model.onnx", None, 1).map(|_| ())
        })
        .expect_err("nothing loadable");
        assert!(
            matches!(&error, crate::layout::LayoutError::Inference(message) if message.contains(&missing.display().to_string())),
            "layout build_session: {error}"
        );
    }

    #[cfg(all(inference_ort, any(layout_detection, auto_rotate)))]
    {
        use crate::inference::InferenceBackend as _;

        let backend = crate::inference::default_backend();
        let error = no_panic("inference backend load_from_memory", || {
            backend.load_from_memory(b"not a model", None).map(|_| ())
        })
        .expect_err("nothing loadable");
        assert_names_missing_path("inference backend load_from_memory", &error.to_string(), missing);
    }

    #[cfg(feature = "onnx-runtime")]
    {
        let error = no_panic("onnx::build_session_with", || {
            crate::onnx::build_session_with(
                &crate::ort_options::OrtSessionOptions::default(),
                None,
                |message| crate::XbergError::embedding(message),
                |builder| builder.commit_from_memory(b"not a model"),
            )
            .map(|_| ())
        })
        .expect_err("nothing loadable");
        assert!(
            matches!(&error, crate::XbergError::MissingDependency(message) if message.contains(&missing.display().to_string())),
            "onnx build_session_with: {error}"
        );
    }
}

/// Paths of mapped images whose name contains `onnxruntime`. Fails when the image list itself is
/// empty, so a broken probe cannot pass as "nothing loaded".
fn loaded_onnxruntime_images() -> Vec<String> {
    let images = mapped_images();
    assert!(
        images.iter().any(|image| image.contains("xberg")),
        "the image probe did not list the test executable: {images:?}"
    );
    images
        .into_iter()
        .filter(|image| image.contains("onnxruntime"))
        .collect()
}

#[cfg(target_os = "linux")]
fn mapped_images() -> Vec<String> {
    std::fs::read_to_string("/proc/self/maps")
        .expect("read /proc/self/maps")
        .lines()
        .filter_map(|line| line.split_whitespace().nth(5).map(str::to_string))
        .collect()
}

#[cfg(target_os = "macos")]
fn mapped_images() -> Vec<String> {
    let output = Command::new("lsof")
        .args(["-p", &std::process::id().to_string(), "-Fn"])
        .output()
        .expect("run lsof for this process");
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.strip_prefix('n').map(str::to_string))
        .collect()
}
