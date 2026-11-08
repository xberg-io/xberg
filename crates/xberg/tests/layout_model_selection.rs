//! Integration test for the experimental PP-DocLayout-V3 layout model selector.
//!
//! Marked `#[ignore]` because it downloads the ~131 MB V3 weights and needs ONNX Runtime.
//! It proves V3 ran by capturing the `layout_model` field of the engine-acquisition event.

#![cfg(all(feature = "pdf", feature = "layout-detection"))]

mod helpers;

use std::sync::Mutex;

use helpers::{extract_uri_document_blocking, get_test_file_path, test_documents_available};
use tracing::field::{Field, Visit};
use tracing_subscriber::layer::{Context, SubscriberExt};
use xberg::core::config::{ExtractionConfig, OutputFormat, layout::LayoutDetectionConfig};
use xberg::{LayoutDetectionModel, LayoutStrategy};

static ACQUIRED_MODELS: Mutex<Vec<String>> = Mutex::new(Vec::new());

struct ModelVisitor(Option<String>);

impl Visit for ModelVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "layout_model" {
            self.0 = Some(value.to_string());
        }
    }

    fn record_debug(&mut self, _field: &Field, _value: &dyn std::fmt::Debug) {}
}

struct CaptureLayoutModel;

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for CaptureLayoutModel {
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = ModelVisitor(None);
        event.record(&mut visitor);
        if let Some(model) = visitor.0 {
            ACQUIRED_MODELS.lock().unwrap().push(model);
        }
    }
}

fn run(model: LayoutDetectionModel) {
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        layout: Some(LayoutDetectionConfig {
            model,
            strategy: LayoutStrategy::Always,
            ..Default::default()
        }),
        use_layout_for_markdown: true,
        use_cache: false,
        ..Default::default()
    };
    let path = get_test_file_path("pdf/google_doc_document.pdf");
    let result = extract_uri_document_blocking(&path, None, &config).expect("extraction should succeed");
    assert!(!result.content.trim().is_empty(), "extraction must produce text");
    assert!(
        result
            .processing_warnings
            .iter()
            .all(|warning| warning.source != "layout"),
        "layout must run without warnings: {:?}",
        result.processing_warnings
    );
}

#[test]
#[ignore = "requires PP-DocLayout-V3 and RT-DETR model files (ORT inference, ~300 MB download)"]
fn selected_model_is_the_one_that_runs() {
    assert!(
        test_documents_available(),
        "test_documents corpus is required for this ignored test"
    );
    tracing::subscriber::set_global_default(tracing_subscriber::registry().with(CaptureLayoutModel))
        .expect("no other global subscriber in this test binary");

    run(LayoutDetectionModel::PpDocLayoutV3);
    let after_v3 = ACQUIRED_MODELS.lock().unwrap().clone();
    assert!(
        !after_v3.is_empty(),
        "the layout engine was never acquired: layout was skipped"
    );
    assert!(
        after_v3.iter().all(|name| name == "PP-DocLayout-V3"),
        "V3 was selected but engines acquired were {after_v3:?}"
    );

    run(LayoutDetectionModel::RtDetr);
    let all = ACQUIRED_MODELS.lock().unwrap().clone();
    assert!(
        all[after_v3.len()..].iter().all(|name| name == "Docling RT-DETR v2") && all.len() > after_v3.len(),
        "RT-DETR run acquired {:?}",
        &all[after_v3.len()..]
    );
}
