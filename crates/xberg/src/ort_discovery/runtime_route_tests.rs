use std::process::Command;

const CHILD: &str = "XBERG_ORT_ROUTE_TEST_CHILD";

fn isolated(test: &str, run: impl FnOnce()) {
    if std::env::var(CHILD).as_deref() == Ok(test) {
        assert_eq!(std::env::var_os("ORT_DYLIB_PATH"), None);
        run();
        assert_eq!(std::env::var_os("ORT_DYLIB_PATH"), None);
        return;
    }

    let output = Command::new(std::env::current_exe().expect("locate route test executable"))
        .args([
            "--exact",
            &format!("ort_discovery::runtime_route_tests::{test}"),
            "--include-ignored",
        ])
        .env_remove("ORT_DYLIB_PATH")
        .env(CHILD, test)
        .output()
        .expect("run isolated runtime route test");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("1 passed; 0 failed; 0 ignored"),
        "runtime route test failed: stdout={stdout} stderr={}",
        String::from_utf8_lossy(&output.stderr),
    );
}

#[cfg(feature = "sceptre-ocr-ort")]
#[test]
#[ignore = "requires ONNX Runtime staged in the platform discovery paths"]
fn standalone_sceptre_preloads_before_reading_the_image() {
    use crate::plugins::OcrBackend;

    isolated("standalone_sceptre_preloads_before_reading_the_image", || {
        let backend = crate::sceptre_ocr::SceptreOcrBackend::new().expect("create Sceptre backend");
        let config = crate::OcrConfig {
            backend_options: Some(serde_json::json!({"model": {"backend": "ort"}})),
            ..Default::default()
        };
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("create test executor");
        let error = runtime
            .block_on(backend.process_image(b"invalid image", &config))
            .expect_err("stop at image decoding before downloading models");
        assert!(
            matches!(&error, crate::XbergError::Ocr { message, .. } if message == "Sceptre OCR operation failed"),
            "{error}"
        );
        sceptre::ort_environment().expect("start Sceptre with the staged runtime loaded by the OCR route");
    });
}

#[cfg(all(feature = "auto-rotate", not(feature = "sceptre-ocr-ort")))]
#[test]
#[ignore = "requires ONNX Runtime staged in the platform discovery paths"]
fn environment_preloads_for_consumers_without_sceptre() {
    isolated("environment_preloads_for_consumers_without_sceptre", || {
        super::ort_environment().expect("start the Xberg environment using the staged runtime");
    });
}
