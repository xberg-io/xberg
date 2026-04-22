//! Regression test for issue #773.
//! Validates that environment variable overrides are correctly applied during configuration loading.

use kreuzberg::{EmbeddingModelType, ExtractionConfig};

#[test]
fn test_regression_773_env_override_loading() {
    let mut config = ExtractionConfig::default();

    if let Some(ref ocr) = config.ocr {
        assert_ne!(ocr.language, "fra");
    }

    unsafe { std::env::set_var("KREUZBERG_OCR_LANGUAGE", "fra") };
    config.apply_env_overrides().expect("Failed to apply overrides");
    unsafe { std::env::remove_var("KREUZBERG_OCR_LANGUAGE") };

    let ocr = config
        .ocr
        .expect("OCR config should be Some when KREUZBERG_OCR_LANGUAGE is set");
    assert_eq!(ocr.language, "fra");
}

#[test]
fn test_regression_773_vlm_embedding_env_override() {
    let mut config = ExtractionConfig::default();

    unsafe { std::env::set_var("KREUZBERG_VLM_EMBEDDING_MODEL", "openai/text-embedding-3-small") };
    config
        .apply_env_overrides()
        .expect("Failed to apply environment overrides");
    unsafe { std::env::remove_var("KREUZBERG_VLM_EMBEDDING_MODEL") };

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
