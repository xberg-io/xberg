//! Smoke test for the LLM translation post-processor.
//!
//! Hits a real provider (defaults to OpenAI) and skips when no API key is set,
//! mirroring the existing `llm_integration.rs` pattern. Run with:
//!
//! ```text
//! cargo test -p xberg --features full --test llm_translation_smoke -- --nocapture
//! ```

#![cfg(all(feature = "translation", feature = "liter-llm", not(target_os = "windows")))]

use std::borrow::Cow;
use xberg::core::config::{LlmConfig, TranslationConfig};
use xberg::types::ExtractedDocument;

fn init() {
    let _ = dotenvy::dotenv();
}

#[tokio::test]
async fn translation_writes_translated_content_and_records_usage() {
    init();
    let api_key = match std::env::var("OPENAI_API_KEY") {
        Ok(k) if !k.is_empty() => k,
        _ => {
            eprintln!("SKIP: OPENAI_API_KEY not set");
            return;
        }
    };

    let config = TranslationConfig {
        target_lang: "de".to_string(),
        source_lang: Some("en".to_string()),
        preserve_markup: false,
        llm: LlmConfig {
            model: "openai/gpt-4o-mini".to_string(),
            api_key: Some(api_key),
            timeout_secs: Some(60),
            max_retries: Some(1),
            ..Default::default()
        },
    };

    let mut result = ExtractedDocument::default();
    result.content = "Hello world. This is a test sentence.".to_string();
    result.mime_type = Cow::Borrowed("text/plain");

    xberg::text::translation::translate_result(&mut result, &config)
        .await
        .expect("translation should succeed");

    let translation = result.translation.expect("translation populated");
    assert_eq!(translation.target_lang, "de");
    assert!(!translation.content.is_empty(), "translated content must be non-empty");
    assert!(
        translation.content != "Hello world. This is a test sentence.",
        "translation should differ from source",
    );
    assert!(result.llm_usage.is_some(), "llm_usage should be populated");
}
