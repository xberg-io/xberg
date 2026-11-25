//! Negative controls for the layout-failure rejection added for PP-DocLayoutV3.
//!
//! A request for a specific local layout detector that fails still returns plausible text from
//! the fallback read. Without these checks the harness would score that text as the requested
//! detector's output.

use crate::adapters::layout_failures::layout_failure_in_item;
use crate::adapters::subprocess::SubprocessAdapter;
use crate::adapters::xberg::pipeline_runs_layout;
use crate::types::XbergPipeline;

const LAYOUT_FAILURE: &str = "layout detection failed (model not found); document extracted without layout hints";

fn item(warnings: serde_json::Value) -> serde_json::Value {
    serde_json::json!({ "content": "text", "processing_warnings": warnings })
}

const IMAGE_LAYOUT_FAILURE: &str =
    "Layout-region OCR failed after whole-image OCR succeeded; retained whole-image output";

const LAYOUT_PIPELINES: [XbergPipeline; 4] = [
    XbergPipeline::Layout,
    XbergPipeline::LayoutV3,
    XbergPipeline::LayoutPaddle,
    XbergPipeline::SceptreOrtLayout,
];

const NON_LAYOUT_PIPELINES: [XbergPipeline; 2] = [XbergPipeline::Baseline, XbergPipeline::BaselinePaddle];

fn plain_adapter(pipeline: XbergPipeline) -> SubprocessAdapter {
    let directory = tempfile::tempdir().unwrap();
    let binary = directory.keep().join("framework");
    std::fs::write(&binary, b"test executable").unwrap();
    SubprocessAdapter::new(
        &format!("xberg-markdown-{pipeline}"),
        binary,
        vec!["extract".to_string()],
        Vec::new(),
        vec!["pdf".to_string()],
    )
}

/// Applies the same rejection policy `create_xberg_adapter` applies to a pipeline.
fn adapter_for(pipeline: XbergPipeline) -> SubprocessAdapter {
    let adapter = plain_adapter(pipeline);
    if pipeline_runs_layout(pipeline) {
        adapter.with_layout_failure_rejection()
    } else {
        adapter
    }
}

fn rejecting_adapter() -> SubprocessAdapter {
    plain_adapter(XbergPipeline::LayoutV3).with_layout_failure_rejection()
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

#[test]
fn every_layout_variant_rejects_the_pdf_and_image_layout_failure_warnings() {
    let pdf = item(serde_json::json!([{ "source": "layout", "message": LAYOUT_FAILURE }])).to_string();
    let image = item(serde_json::json!([{ "source": "layout-ocr", "message": IMAGE_LAYOUT_FAILURE }])).to_string();
    for pipeline in LAYOUT_PIPELINES {
        for payload in [&pdf, &image] {
            let error = adapter_for(pipeline)
                .parse_output(payload)
                .expect_err(&format!("{pipeline} must not score a failed layout pass"));
            assert!(
                error.to_string().contains("layout detection failed"),
                "{pipeline}: {error}"
            );
        }
    }
}

#[test]
fn a_cpu_recovery_that_also_lost_pages_is_rejected_by_the_subprocess_adapter() {
    let combined = "automatic layout inference failed (CoreML ExecuteKernel failed); recovered on CPU; \
                    layout: 1 page(s) could not be prepared for layout analysis and were treated as empty: \
                    page 2 failed to render: boom";
    let recovered_only = "automatic layout inference failed (CoreML ExecuteKernel failed); recovered on CPU";
    let rejected = item(serde_json::json!([{ "source": "layout", "message": combined }])).to_string();
    let accepted = item(serde_json::json!([{ "source": "layout", "message": recovered_only }])).to_string();
    for pipeline in LAYOUT_PIPELINES {
        let error = adapter_for(pipeline)
            .parse_output(&rejected)
            .expect_err(&format!("{pipeline} must reject lost pages"));
        assert!(
            error.to_string().contains("could not be prepared"),
            "{pipeline}: {error}"
        );
        assert!(adapter_for(pipeline).parse_output(&accepted).is_ok(), "{pipeline}");
    }
}

#[test]
fn baseline_variants_ignore_the_same_layout_warnings() {
    let pdf = item(serde_json::json!([{ "source": "layout", "message": LAYOUT_FAILURE }])).to_string();
    let image = item(serde_json::json!([{ "source": "layout-ocr", "message": IMAGE_LAYOUT_FAILURE }])).to_string();
    for pipeline in NON_LAYOUT_PIPELINES {
        for payload in [&pdf, &image] {
            assert!(adapter_for(pipeline).parse_output(payload).is_ok(), "{pipeline}");
        }
    }
}

#[test]
fn layout_variants_accept_a_result_without_layout_failure_warnings() {
    let clean = item(serde_json::json!([
        { "source": "ocr", "message": "Page 1 OCR used 8 source-image strips." },
    ]))
    .to_string();
    for pipeline in LAYOUT_PIPELINES {
        assert!(adapter_for(pipeline).parse_output(&clean).is_ok(), "{pipeline}");
    }
}
