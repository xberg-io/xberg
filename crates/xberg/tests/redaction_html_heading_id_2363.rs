#![cfg(all(feature = "html", feature = "redaction", feature = "tokio-runtime"))]

use xberg::core::config::redaction::{RedactionConfig, RedactionTerm};
use xberg::types::metadata::HeaderMetadata;
use xberg::types::redaction::RedactionStrategy;
use xberg::{ExtractInput, ExtractedDocument, ExtractionConfig, OutputFormat, extract};

const PRIVATE_ID: &str = "PrivateHeading17";
const SAFE_ID: &str = "public-section";
const MASK: &str = "[REDACTED]";
const HTML: &str = concat!(
    "<html><body>",
    "<h2 id='PrivateHeading17'>Project overview</h2>",
    "<h2 id='public-section'>Public section</h2>",
    "<h2>Unlabelled section</h2>",
    "</body></html>"
);

async fn extract_headings(redact: bool) -> ExtractedDocument {
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        use_cache: false,
        redaction: redact.then(|| RedactionConfig {
            strategy: RedactionStrategy::Mask,
            custom_terms: vec![RedactionTerm::labeled("private", PRIVATE_ID)],
            ..Default::default()
        }),
        ..Default::default()
    };
    let mut batch = extract(ExtractInput::from_bytes(HTML.as_bytes(), "text/html", None), &config)
        .await
        .expect("public HTML extraction");
    assert!(batch.errors.is_empty(), "{:?}", batch.errors);
    assert_eq!(batch.results.len(), 1);
    batch.results.remove(0)
}

fn headers(document: &ExtractedDocument) -> &[HeaderMetadata] {
    &document
        .metadata
        .format
        .as_ref()
        .and_then(|format| format.html())
        .expect("extracted HTML metadata")
        .headers
}

#[tokio::test]
async fn masks_a_private_source_heading_id_in_the_serialized_result() {
    let baseline = extract_headings(false).await;
    let baseline_headers = headers(&baseline);
    assert_eq!(baseline_headers.len(), 3);
    assert_eq!(baseline_headers[0].id.as_deref(), Some(PRIVATE_ID));
    assert!(!baseline.content.contains(PRIVATE_ID));

    let redacted = extract_headings(true).await;
    let redacted_headers = headers(&redacted);
    assert_eq!(redacted_headers.len(), 3);
    assert_eq!(redacted_headers[0].id.as_deref(), Some(MASK));
    assert_eq!(redacted_headers[1].id.as_deref(), Some(SAFE_ID));
    assert_eq!(redacted_headers[2].id, None);
    for (before, after) in baseline_headers.iter().zip(redacted_headers) {
        assert_eq!(after.text, before.text);
        assert_eq!(after.level, before.level);
        assert_eq!(after.depth, before.depth);
        assert_eq!(after.html_offset, before.html_offset);
    }
    assert_eq!(redacted.content, baseline.content);
    let output = serde_json::to_vec(&redacted).expect("serialize the full extraction result");
    assert!(
        !output
            .windows(PRIVATE_ID.len())
            .any(|part| part == PRIVATE_ID.as_bytes())
    );
}
