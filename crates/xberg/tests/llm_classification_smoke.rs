//! Smoke test for the per-page LLM classification post-processor.
//!
//! Hits a real provider (defaults to OpenAI) and skips when no API key is set,
//! mirroring the existing `llm_integration.rs` pattern. Run with:
//!
//! ```text
//! cargo test -p xberg --features full --test llm_classification_smoke -- --nocapture
//! ```

#![cfg(all(feature = "classification", feature = "liter-llm", not(target_os = "windows")))]

use std::borrow::Cow;
use xberg::core::config::{LlmConfig, PageClassificationConfig};
use xberg::types::ExtractedDocument;

fn init() {
    let _ = dotenvy::dotenv();
}

fn build_result(text: &str) -> ExtractedDocument {
    let mut result = ExtractedDocument::default();
    result.content = text.to_string();
    result.mime_type = Cow::Borrowed("text/plain");
    result
}

#[tokio::test]
async fn classification_single_label_yields_an_allowed_label() {
    init();
    let api_key = match std::env::var("OPENAI_API_KEY") {
        Ok(k) if !k.is_empty() => k,
        _ => {
            eprintln!("SKIP: OPENAI_API_KEY not set");
            return;
        }
    };

    let config = PageClassificationConfig {
        prompt_template: None,
        labels: vec!["invoice".to_string(), "memo".to_string(), "purchase_order".to_string()],
        multi_label: false,
        llm: LlmConfig {
            model: "openai/gpt-4o-mini".to_string(),
            api_key: Some(api_key),
            timeout_secs: Some(60),
            max_retries: Some(1),
            ..Default::default()
        },
    };

    let mut result = build_result("INVOICE #12345\nBill To: Acme Corp\nItems: Widget x 10\nTotal: $1,250.00");

    xberg::text::classification::classify_pages(&mut result, &config)
        .await
        .expect("classification should succeed");

    let classifications = result.page_classifications.expect("page_classifications populated");
    assert_eq!(classifications.len(), 1, "exactly one page classification");
    assert_eq!(classifications[0].page_number, 1);
    let labels: Vec<&str> = classifications[0].labels.iter().map(|l| l.label.as_str()).collect();
    assert!(
        labels.iter().any(|l| ["invoice", "memo", "purchase_order"].contains(l)),
        "returned label must be one of the configured options, got {labels:?}",
    );
    assert!(result.llm_usage.is_some(), "llm_usage should be populated");
}
