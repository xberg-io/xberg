//! Regression test for issue #773.
//! Validates that environment variable overrides are correctly applied during configuration loading.

use xberg::{EmbeddingModelType, ExtractionConfig};
use xberg_child_test::{child_test_root, run_child_test};

const OVERRIDE_CASE: &str = "XBERG_CONFIG_OVERRIDE_TEST_CASE";

fn run_override_case(child: &str, variable: &str, value: &str) {
    run_child_test(child, OVERRIDE_CASE, variable, |command| {
        command
            .env_clear()
            .envs(
                std::env::vars_os()
                    .filter(|(key, _)| !key.to_string_lossy().to_ascii_uppercase().starts_with("XBERG_")),
            )
            .env(OVERRIDE_CASE, variable)
            .env(variable, value);
    });
}

#[test]
fn test_regression_773_env_override_loading() {
    run_override_case("env_override_loading_child", "XBERG_OCR_LANGUAGE", "fra");
}

#[test]
#[ignore = "run in an isolated subprocess by the config override test"]
fn env_override_loading_child() {
    let Some(variable) = child_test_root(OVERRIDE_CASE) else {
        return;
    };
    assert_eq!(variable.to_str(), Some("XBERG_OCR_LANGUAGE"));
    let mut config = ExtractionConfig::default();

    if let Some(ref ocr) = config.ocr {
        assert_ne!(ocr.language, vec!["fra".to_string()]);
    }

    config.apply_env_overrides().expect("Failed to apply overrides");

    let ocr = config
        .ocr
        .expect("OCR config should be Some when XBERG_OCR_LANGUAGE is set");
    assert_eq!(ocr.language, vec!["fra".to_string()]);
}

#[test]
fn test_regression_773_vlm_embedding_env_override() {
    run_override_case(
        "vlm_embedding_env_override_child",
        "XBERG_VLM_EMBEDDING_MODEL",
        "openai/text-embedding-3-small",
    );
}

#[test]
#[ignore = "run in an isolated subprocess by the config override test"]
fn vlm_embedding_env_override_child() {
    let Some(variable) = child_test_root(OVERRIDE_CASE) else {
        return;
    };
    assert_eq!(variable.to_str(), Some("XBERG_VLM_EMBEDDING_MODEL"));
    let mut config = ExtractionConfig::default();

    config
        .apply_env_overrides()
        .expect("Failed to apply environment overrides");

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
