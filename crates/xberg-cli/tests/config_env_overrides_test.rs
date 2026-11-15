//! Regression test for issue #773.
//! Validates that environment variable overrides are correctly applied during configuration loading.
#![allow(unsafe_code, reason = "the tests set and remove process environment variables")]

use xberg::{EmbeddingModelType, ExtractionConfig};

#[test]
fn test_regression_773_env_override_loading() {
    let mut config = ExtractionConfig::default();

    if let Some(ref ocr) = config.ocr {
        assert_ne!(ocr.language, vec!["fra".to_string()]);
    }

    unsafe { std::env::set_var("XBERG_OCR_LANGUAGE", "fra") };
    config.apply_env_overrides().expect("Failed to apply overrides");
    unsafe { std::env::remove_var("XBERG_OCR_LANGUAGE") };

    let ocr = config
        .ocr
        .expect("OCR config should be Some when XBERG_OCR_LANGUAGE is set");
    assert_eq!(ocr.language, vec!["fra".to_string()]);
}

#[test]
fn test_regression_773_vlm_embedding_env_override() {
    let mut config = ExtractionConfig::default();

    unsafe { std::env::set_var("XBERG_VLM_EMBEDDING_MODEL", "openai/text-embedding-3-small") };
    config
        .apply_env_overrides()
        .expect("Failed to apply environment overrides");
    unsafe { std::env::remove_var("XBERG_VLM_EMBEDDING_MODEL") };

    let chunking = config
        .chunking
        .expect("Chunking should be enabled when VLM embedding is set");
    let embedding = chunking.embedding.expect("Embedding should be configured");

    match embedding.model {
        EmbeddingModelType::Llm { llm } => {
            assert_eq!(llm.model, "openai/text-embedding-3-small");
            assert!(llm.api_key.is_none());
        }
        _ => panic!("Expected Llm embedding model type"),
    }
}
