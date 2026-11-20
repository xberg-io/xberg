//! Negative controls for the layout-failure rejection added for PP-DocLayoutV3.
//!
//! A request for a specific local layout detector that fails still returns plausible text from
//! the fallback read. Without these checks the harness would score that text as the requested
//! detector's output.

use crate::adapters::layout_failures::layout_failure_in_item;
use crate::adapters::subprocess::SubprocessAdapter;

const LAYOUT_FAILURE: &str = "layout detection failed (model not found); document extracted without layout hints";

fn item(warnings: serde_json::Value) -> serde_json::Value {
    serde_json::json!({ "content": "text", "processing_warnings": warnings })
}

fn rejecting_adapter() -> SubprocessAdapter {
    let directory = tempfile::tempdir().unwrap();
    let binary = directory.keep().join("framework");
    std::fs::write(&binary, b"test executable").unwrap();
    SubprocessAdapter::new(
        "xberg-markdown-layout-v3",
        binary,
        vec!["extract".to_string()],
        Vec::new(),
        vec!["pdf".to_string()],
    )
    .with_layout_failure_rejection()
}

#[test]
fn a_layout_failure_warning_is_recognised_with_its_reason() {
    let message = layout_failure_in_item(&item(serde_json::json!([
        { "source": "pdf-render", "message": "Page 1 OCR render DPI was reduced" },
        { "source": "layout", "message": LAYOUT_FAILURE },
    ])))
    .expect("the layout warning must be detected");
    assert!(message.contains("model not found"), "{message}");
    assert!(message.contains("without the requested detector"), "{message}");
}

#[test]
fn warnings_from_other_sources_do_not_reject_an_item() {
    assert_eq!(
        layout_failure_in_item(&item(serde_json::json!([
            { "source": "ocr", "message": "Page 1 OCR used 8 source-image strips." },
        ]))),
        None
    );
    assert_eq!(layout_failure_in_item(&serde_json::json!({ "content": "text" })), None);
}

#[test]
fn a_failed_layout_pass_is_rejected_only_when_the_adapter_asks_for_it() {
    let payload = item(serde_json::json!([{ "source": "layout", "message": LAYOUT_FAILURE }])).to_string();

    let error = rejecting_adapter()
        .parse_output(&payload)
        .expect_err("a V3 row must not be scored");
    assert!(error.to_string().contains("layout detection failed"), "{error}");

    let lenient = rejecting_adapter();
    let lenient = SubprocessAdapter {
        reject_layout_failures: false,
        ..lenient
    };
    assert!(
        lenient.parse_output(&payload).is_ok(),
        "other pipelines keep the fallback read"
    );
}
