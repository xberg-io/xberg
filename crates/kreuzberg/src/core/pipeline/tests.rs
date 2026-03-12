//! Pipeline orchestration tests.

use super::*;
use crate::core::config::OutputFormat;
use crate::types::Metadata;
use serial_test::serial;
use std::borrow::Cow;

const VALIDATION_MARKER_KEY: &str = "registry_validation_marker";
#[cfg(feature = "quality")]
const QUALITY_VALIDATION_MARKER: &str = "quality_validation_test";
const POSTPROCESSOR_VALIDATION_MARKER: &str = "postprocessor_validation_test";
const ORDER_VALIDATION_MARKER: &str = "order_validation_test";

#[tokio::test]
#[serial]
async fn test_run_pipeline_basic() {
    let mut result = ExtractionResult {
        content: "test".to_string(),
        mime_type: Cow::Borrowed("text/plain"),
        metadata: Metadata::default(),
        tables: vec![],
        detected_languages: None,
        chunks: None,
        images: None,
        djot_content: None,
        pages: None,
        elements: None,
        ocr_elements: None,
        document: None,
        #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
        extracted_keywords: None,
        quality_score: None,
        processing_warnings: Vec::new(),
        annotations: None,
    };
    result.metadata.additional.insert(
        Cow::Borrowed(VALIDATION_MARKER_KEY),
        serde_json::json!(ORDER_VALIDATION_MARKER),
    );
    let config = ExtractionConfig {
        postprocessor: Some(crate::core::config::PostProcessorConfig {
            enabled: false,
            ..Default::default()
        }),
        ..Default::default()
    };

    let processed = run_pipeline(result, &config).await.unwrap();
    assert_eq!(processed.content, "test");
}

#[tokio::test]
#[serial]
#[cfg(feature = "quality")]
async fn test_pipeline_with_quality_processing() {
    let result = ExtractionResult {
        content: "This is a test document with some meaningful content.".to_string(),
        mime_type: Cow::Borrowed("text/plain"),
        metadata: Metadata::default(),
        tables: vec![],
        detected_languages: None,
        chunks: None,
        images: None,
        djot_content: None,
        pages: None,
        elements: None,
        ocr_elements: None,
        document: None,
        #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
        extracted_keywords: None,
        quality_score: None,
        processing_warnings: Vec::new(),
        annotations: None,
    };
    let config = ExtractionConfig {
        enable_quality_processing: true,
        ..Default::default()
    };

    let processed = run_pipeline(result, &config).await.unwrap();
    assert!(processed.metadata.additional.contains_key("quality_score"));
}

#[tokio::test]
#[serial]
async fn test_pipeline_without_quality_processing() {
    let result = ExtractionResult {
        content: "test".to_string(),
        mime_type: Cow::Borrowed("text/plain"),
        metadata: Metadata::default(),
        tables: vec![],
        detected_languages: None,
        chunks: None,
        images: None,
        djot_content: None,
        pages: None,
        elements: None,
        ocr_elements: None,
        document: None,
        #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
        extracted_keywords: None,
        quality_score: None,
        processing_warnings: Vec::new(),
        annotations: None,
    };
    let config = ExtractionConfig {
        enable_quality_processing: false,
        postprocessor: Some(crate::core::config::PostProcessorConfig {
            enabled: false,
            ..Default::default()
        }),
        ..Default::default()
    };

    let processed = run_pipeline(result, &config).await.unwrap();
    assert!(!processed.metadata.additional.contains_key("quality_score"));
}

#[tokio::test]
#[serial]
#[cfg(feature = "chunking")]
async fn test_pipeline_with_chunking() {
    let result = ExtractionResult {
        content: "This is a long text that should be chunked. ".repeat(100),
        mime_type: Cow::Borrowed("text/plain"),
        metadata: Metadata::default(),
        tables: vec![],
        detected_languages: None,
        chunks: None,
        images: None,
        djot_content: None,
        pages: None,
        elements: None,
        ocr_elements: None,
        document: None,
        #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
        extracted_keywords: None,
        quality_score: None,
        processing_warnings: Vec::new(),
        annotations: None,
    };
    let config = ExtractionConfig {
        chunking: Some(crate::ChunkingConfig {
            max_characters: 500,
            overlap: 50,
            trim: true,
            chunker_type: crate::ChunkerType::Text,
            ..Default::default()
        }),
        ..Default::default()
    };

    let processed = run_pipeline(result, &config).await.unwrap();
    assert!(processed.metadata.additional.contains_key("chunk_count"));
    let chunk_count = processed.metadata.additional.get("chunk_count").unwrap();
    assert!(chunk_count.as_u64().unwrap() > 1);
}

#[tokio::test]
#[serial]
async fn test_pipeline_without_chunking() {
    let result = ExtractionResult {
        content: "test".to_string(),
        mime_type: Cow::Borrowed("text/plain"),
        metadata: Metadata::default(),
        tables: vec![],
        detected_languages: None,
        chunks: None,
        images: None,
        djot_content: None,
        pages: None,
        elements: None,
        ocr_elements: None,
        document: None,
        #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
        extracted_keywords: None,
        quality_score: None,
        processing_warnings: Vec::new(),
        annotations: None,
    };
    let config = ExtractionConfig {
        chunking: None,
        postprocessor: Some(crate::core::config::PostProcessorConfig {
            enabled: false,
            ..Default::default()
        }),
        ..Default::default()
    };

    let processed = run_pipeline(result, &config).await.unwrap();
    assert!(!processed.metadata.additional.contains_key("chunk_count"));
}

#[tokio::test]
#[serial]
async fn test_pipeline_preserves_metadata() {
    use ahash::AHashMap;
    let mut additional = AHashMap::new();
    additional.insert(Cow::Borrowed("source"), serde_json::json!("test"));
    additional.insert(Cow::Borrowed("page"), serde_json::json!(1));

    let result = ExtractionResult {
        content: "test".to_string(),
        mime_type: Cow::Borrowed("text/plain"),
        metadata: Metadata {
            additional,
            ..Default::default()
        },
        pages: None,
        tables: vec![],
        detected_languages: None,
        chunks: None,
        images: None,
        djot_content: None,
        elements: None,
        ocr_elements: None,
        document: None,
        #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
        extracted_keywords: None,
        quality_score: None,
        processing_warnings: Vec::new(),
        annotations: None,
    };
    let config = ExtractionConfig {
        postprocessor: Some(crate::core::config::PostProcessorConfig {
            enabled: false,
            ..Default::default()
        }),
        ..Default::default()
    };

    let processed = run_pipeline(result, &config).await.unwrap();
    assert_eq!(
        processed.metadata.additional.get("source").unwrap(),
        &serde_json::json!("test")
    );
    assert_eq!(
        processed.metadata.additional.get("page").unwrap(),
        &serde_json::json!(1)
    );
}

#[tokio::test]
#[serial]
async fn test_pipeline_preserves_tables() {
    use crate::types::Table;

    let table = Table {
        cells: vec![vec!["A".to_string(), "B".to_string()]],
        markdown: "| A | B |".to_string(),
        page_number: 0,
        bounding_box: None,
    };

    let result = ExtractionResult {
        content: "test".to_string(),
        mime_type: Cow::Borrowed("text/plain"),
        metadata: Metadata::default(),
        tables: vec![table],
        detected_languages: None,
        chunks: None,
        images: None,
        djot_content: None,
        pages: None,
        elements: None,
        ocr_elements: None,
        document: None,
        #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
        extracted_keywords: None,
        quality_score: None,
        processing_warnings: Vec::new(),
        annotations: None,
    };
    let config = ExtractionConfig {
        postprocessor: Some(crate::core::config::PostProcessorConfig {
            enabled: false,
            ..Default::default()
        }),
        ..Default::default()
    };

    let processed = run_pipeline(result, &config).await.unwrap();
    assert_eq!(processed.tables.len(), 1);
    assert_eq!(processed.tables[0].cells.len(), 1);
}

#[tokio::test]
#[serial]
async fn test_pipeline_empty_content() {
    {
        let registry = crate::plugins::registry::get_post_processor_registry();
        registry.write().unwrap().shutdown_all().unwrap();
    }
    {
        let registry = crate::plugins::registry::get_validator_registry();
        registry.write().unwrap().shutdown_all().unwrap();
    }

    let result = ExtractionResult {
        content: String::new(),
        mime_type: Cow::Borrowed("text/plain"),
        metadata: Metadata::default(),
        tables: vec![],
        detected_languages: None,
        chunks: None,
        images: None,
        djot_content: None,
        pages: None,
        elements: None,
        ocr_elements: None,
        document: None,
        #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
        extracted_keywords: None,
        quality_score: None,
        processing_warnings: Vec::new(),
        annotations: None,
    };
    let config = ExtractionConfig::default();

    let processed = run_pipeline(result, &config).await.unwrap();
    assert_eq!(processed.content, "");
}

#[tokio::test]
#[serial]
#[cfg(feature = "chunking")]
async fn test_pipeline_with_all_features() {
    let result = ExtractionResult {
        content: "This is a comprehensive test document. ".repeat(50),
        mime_type: Cow::Borrowed("text/plain"),
        metadata: Metadata::default(),
        tables: vec![],
        detected_languages: None,
        chunks: None,
        images: None,
        djot_content: None,
        pages: None,
        elements: None,
        ocr_elements: None,
        document: None,
        #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
        extracted_keywords: None,
        quality_score: None,
        processing_warnings: Vec::new(),
        annotations: None,
    };
    let config = ExtractionConfig {
        enable_quality_processing: true,
        chunking: Some(crate::ChunkingConfig {
            max_characters: 500,
            overlap: 50,
            trim: true,
            chunker_type: crate::ChunkerType::Text,
            ..Default::default()
        }),
        ..Default::default()
    };

    let processed = run_pipeline(result, &config).await.unwrap();
    assert!(processed.metadata.additional.contains_key("quality_score"));
    assert!(processed.metadata.additional.contains_key("chunk_count"));
}

#[tokio::test]
#[serial]
#[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
async fn test_pipeline_with_keyword_extraction() {
    crate::plugins::registry::get_validator_registry()
        .write()
        .unwrap()
        .shutdown_all()
        .unwrap();
    crate::plugins::registry::get_post_processor_registry()
        .write()
        .unwrap()
        .shutdown_all()
        .unwrap();

    // Register keyword processor directly (bypasses Lazy which only runs once per process)
    let _ = crate::keywords::register_keyword_processor();
    clear_processor_cache().unwrap();

    let result = ExtractionResult {
        content: r#"
Machine learning is a branch of artificial intelligence that focuses on
building systems that can learn from data. Deep learning is a subset of
machine learning that uses neural networks with multiple layers.
Natural language processing enables computers to understand human language.
            "#
        .to_string(),
        mime_type: Cow::Borrowed("text/plain"),
        metadata: Metadata::default(),
        tables: vec![],
        detected_languages: None,
        chunks: None,
        images: None,
        djot_content: None,
        pages: None,
        elements: None,
        ocr_elements: None,
        document: None,
        #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
        extracted_keywords: None,
        quality_score: None,
        processing_warnings: Vec::new(),
        annotations: None,
    };

    #[cfg(feature = "keywords-yake")]
    let keyword_config = crate::keywords::KeywordConfig::yake();

    #[cfg(all(feature = "keywords-rake", not(feature = "keywords-yake")))]
    let keyword_config = crate::keywords::KeywordConfig::rake();

    let config = ExtractionConfig {
        keywords: Some(keyword_config),
        ..Default::default()
    };

    let processed = run_pipeline(result, &config).await.unwrap();

    assert!(processed.metadata.additional.contains_key("keywords"));

    let keywords_value = processed.metadata.additional.get("keywords").unwrap();
    assert!(keywords_value.is_array());

    let keywords = keywords_value.as_array().unwrap();
    assert!(!keywords.is_empty(), "Should have extracted keywords");

    let first_keyword = &keywords[0];
    assert!(first_keyword.is_object());
    assert!(first_keyword.get("text").is_some());
    assert!(first_keyword.get("score").is_some());
    assert!(first_keyword.get("algorithm").is_some());
}

#[tokio::test]
#[serial]
#[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
async fn test_pipeline_without_keyword_config() {
    let result = ExtractionResult {
        content: "Machine learning and artificial intelligence.".to_string(),
        mime_type: Cow::Borrowed("text/plain"),
        metadata: Metadata::default(),
        tables: vec![],
        detected_languages: None,
        chunks: None,
        images: None,
        djot_content: None,
        pages: None,
        elements: None,
        ocr_elements: None,
        document: None,
        #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
        extracted_keywords: None,
        quality_score: None,
        processing_warnings: Vec::new(),
        annotations: None,
    };

    let config = ExtractionConfig {
        keywords: None,
        ..Default::default()
    };

    let processed = run_pipeline(result, &config).await.unwrap();

    assert!(!processed.metadata.additional.contains_key("keywords"));
}

#[tokio::test]
#[serial]
#[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
async fn test_pipeline_keyword_extraction_short_content() {
    crate::plugins::registry::get_validator_registry()
        .write()
        .unwrap()
        .shutdown_all()
        .unwrap();
    crate::plugins::registry::get_post_processor_registry()
        .write()
        .unwrap()
        .shutdown_all()
        .unwrap();

    let result = ExtractionResult {
        content: "Short text".to_string(),
        mime_type: Cow::Borrowed("text/plain"),
        metadata: Metadata::default(),
        tables: vec![],
        detected_languages: None,
        chunks: None,
        images: None,
        djot_content: None,
        pages: None,
        elements: None,
        ocr_elements: None,
        document: None,
        #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
        extracted_keywords: None,
        quality_score: None,
        processing_warnings: Vec::new(),
        annotations: None,
    };

    #[cfg(feature = "keywords-yake")]
    let keyword_config = crate::keywords::KeywordConfig::yake();

    #[cfg(all(feature = "keywords-rake", not(feature = "keywords-yake")))]
    let keyword_config = crate::keywords::KeywordConfig::rake();

    let config = ExtractionConfig {
        keywords: Some(keyword_config),
        ..Default::default()
    };

    let processed = run_pipeline(result, &config).await.unwrap();

    assert!(!processed.metadata.additional.contains_key("keywords"));
}

#[tokio::test]
#[serial]
async fn test_postprocessor_runs_before_validator() {
    use crate::plugins::{Plugin, PostProcessor, ProcessingStage, Validator};
    use async_trait::async_trait;
    use std::sync::Arc;

    struct TestPostProcessor;
    impl Plugin for TestPostProcessor {
        fn name(&self) -> &str {
            "test-processor"
        }
        fn version(&self) -> String {
            "1.0.0".to_string()
        }
        fn initialize(&self) -> Result<()> {
            Ok(())
        }
        fn shutdown(&self) -> Result<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl PostProcessor for TestPostProcessor {
        async fn process(&self, result: &mut ExtractionResult, _config: &ExtractionConfig) -> Result<()> {
            result
                .metadata
                .additional
                .insert(Cow::Borrowed("processed"), serde_json::json!(true));
            Ok(())
        }

        fn processing_stage(&self) -> ProcessingStage {
            ProcessingStage::Middle
        }
    }

    struct TestValidator;
    impl Plugin for TestValidator {
        fn name(&self) -> &str {
            "test-validator"
        }
        fn version(&self) -> String {
            "1.0.0".to_string()
        }
        fn initialize(&self) -> Result<()> {
            Ok(())
        }
        fn shutdown(&self) -> Result<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl Validator for TestValidator {
        async fn validate(&self, result: &ExtractionResult, _config: &ExtractionConfig) -> Result<()> {
            let should_validate = result
                .metadata
                .additional
                .get(VALIDATION_MARKER_KEY)
                .and_then(|v| v.as_str())
                == Some(POSTPROCESSOR_VALIDATION_MARKER);

            if !should_validate {
                return Ok(());
            }

            let processed = result
                .metadata
                .additional
                .get("processed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            if !processed {
                return Err(crate::KreuzbergError::Validation {
                    message: "Post-processor did not run before validator".to_string(),
                    source: None,
                });
            }
            Ok(())
        }
    }

    let pp_registry = crate::plugins::registry::get_post_processor_registry();
    let val_registry = crate::plugins::registry::get_validator_registry();

    clear_processor_cache().unwrap();
    pp_registry.write().unwrap().shutdown_all().unwrap();
    val_registry.write().unwrap().shutdown_all().unwrap();
    clear_processor_cache().unwrap();

    {
        let mut registry = pp_registry.write().unwrap();
        registry.register(Arc::new(TestPostProcessor), 0).unwrap();
    }

    {
        let mut registry = val_registry.write().unwrap();
        registry.register(Arc::new(TestValidator)).unwrap();
    }

    clear_processor_cache().unwrap();

    let mut result = ExtractionResult {
        content: "test".to_string(),
        mime_type: Cow::Borrowed("text/plain"),
        metadata: Metadata::default(),
        tables: vec![],
        detected_languages: None,
        chunks: None,
        images: None,
        djot_content: None,
        pages: None,
        elements: None,
        ocr_elements: None,
        document: None,
        #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
        extracted_keywords: None,
        quality_score: None,
        processing_warnings: Vec::new(),
        annotations: None,
    };
    result.metadata.additional.insert(
        Cow::Borrowed(VALIDATION_MARKER_KEY),
        serde_json::json!(POSTPROCESSOR_VALIDATION_MARKER),
    );

    let config = ExtractionConfig {
        postprocessor: Some(crate::core::config::PostProcessorConfig {
            enabled: true,
            enabled_set: None,
            disabled_set: None,
            enabled_processors: None,
            disabled_processors: None,
        }),
        ..Default::default()
    };

    let processed = run_pipeline(result, &config).await;

    pp_registry.write().unwrap().shutdown_all().unwrap();
    val_registry.write().unwrap().shutdown_all().unwrap();

    assert!(processed.is_ok(), "Validator should have seen post-processor metadata");
    let processed = processed.unwrap();
    assert_eq!(
        processed.metadata.additional.get("processed"),
        Some(&serde_json::json!(true)),
        "Post-processor metadata should be present"
    );
}

#[tokio::test]
#[serial]
#[cfg(feature = "quality")]
async fn test_quality_processing_runs_before_validator() {
    use crate::plugins::{Plugin, Validator};
    use async_trait::async_trait;
    use std::sync::Arc;

    struct QualityValidator;
    impl Plugin for QualityValidator {
        fn name(&self) -> &str {
            "quality-validator"
        }
        fn version(&self) -> String {
            "1.0.0".to_string()
        }
        fn initialize(&self) -> Result<()> {
            Ok(())
        }
        fn shutdown(&self) -> Result<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl Validator for QualityValidator {
        async fn validate(&self, result: &ExtractionResult, _config: &ExtractionConfig) -> Result<()> {
            let should_validate = result
                .metadata
                .additional
                .get(VALIDATION_MARKER_KEY)
                .and_then(|v| v.as_str())
                == Some(QUALITY_VALIDATION_MARKER);

            if !should_validate {
                return Ok(());
            }

            if !result.metadata.additional.contains_key("quality_score") {
                return Err(crate::KreuzbergError::Validation {
                    message: "Quality processing did not run before validator".to_string(),
                    source: None,
                });
            }
            Ok(())
        }
    }

    let val_registry = crate::plugins::registry::get_validator_registry();
    {
        let mut registry = val_registry.write().unwrap();
        registry.register(Arc::new(QualityValidator)).unwrap();
    }

    let mut result = ExtractionResult {
        content: "This is meaningful test content for quality scoring.".to_string(),
        mime_type: Cow::Borrowed("text/plain"),
        metadata: Metadata::default(),
        tables: vec![],
        detected_languages: None,
        chunks: None,
        images: None,
        djot_content: None,
        pages: None,
        elements: None,
        ocr_elements: None,
        document: None,
        #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
        extracted_keywords: None,
        quality_score: None,
        processing_warnings: Vec::new(),
        annotations: None,
    };
    result.metadata.additional.insert(
        Cow::Borrowed(VALIDATION_MARKER_KEY),
        serde_json::json!(QUALITY_VALIDATION_MARKER),
    );

    let config = ExtractionConfig {
        enable_quality_processing: true,
        ..Default::default()
    };

    let processed = run_pipeline(result, &config).await;

    {
        let mut registry = val_registry.write().unwrap();
        registry.remove("quality-validator").unwrap();
    }

    assert!(processed.is_ok(), "Validator should have seen quality_score");
}

#[tokio::test]
#[serial]
async fn test_multiple_postprocessors_run_before_validator() {
    use crate::plugins::{Plugin, PostProcessor, ProcessingStage, Validator};
    use async_trait::async_trait;
    use std::sync::Arc;

    struct EarlyProcessor;
    impl Plugin for EarlyProcessor {
        fn name(&self) -> &str {
            "early-proc"
        }
        fn version(&self) -> String {
            "1.0.0".to_string()
        }
        fn initialize(&self) -> Result<()> {
            Ok(())
        }
        fn shutdown(&self) -> Result<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl PostProcessor for EarlyProcessor {
        async fn process(&self, result: &mut ExtractionResult, _config: &ExtractionConfig) -> Result<()> {
            let mut order = result
                .metadata
                .additional
                .get("execution_order")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            order.push(serde_json::json!("early"));
            result
                .metadata
                .additional
                .insert(Cow::Borrowed("execution_order"), serde_json::json!(order));
            Ok(())
        }

        fn processing_stage(&self) -> ProcessingStage {
            ProcessingStage::Early
        }
    }

    struct LateProcessor;
    impl Plugin for LateProcessor {
        fn name(&self) -> &str {
            "late-proc"
        }
        fn version(&self) -> String {
            "1.0.0".to_string()
        }
        fn initialize(&self) -> Result<()> {
            Ok(())
        }
        fn shutdown(&self) -> Result<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl PostProcessor for LateProcessor {
        async fn process(&self, result: &mut ExtractionResult, _config: &ExtractionConfig) -> Result<()> {
            let mut order = result
                .metadata
                .additional
                .get("execution_order")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            order.push(serde_json::json!("late"));
            result
                .metadata
                .additional
                .insert(Cow::Borrowed("execution_order"), serde_json::json!(order));
            Ok(())
        }

        fn processing_stage(&self) -> ProcessingStage {
            ProcessingStage::Late
        }
    }

    struct OrderValidator;
    impl Plugin for OrderValidator {
        fn name(&self) -> &str {
            "order-validator"
        }
        fn version(&self) -> String {
            "1.0.0".to_string()
        }
        fn initialize(&self) -> Result<()> {
            Ok(())
        }
        fn shutdown(&self) -> Result<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl Validator for OrderValidator {
        async fn validate(&self, result: &ExtractionResult, _config: &ExtractionConfig) -> Result<()> {
            let should_validate = result
                .metadata
                .additional
                .get(VALIDATION_MARKER_KEY)
                .and_then(|v| v.as_str())
                == Some(ORDER_VALIDATION_MARKER);

            if !should_validate {
                return Ok(());
            }

            let order = result
                .metadata
                .additional
                .get("execution_order")
                .and_then(|v| v.as_array())
                .ok_or_else(|| crate::KreuzbergError::Validation {
                    message: "No execution order found".to_string(),
                    source: None,
                })?;

            if order.len() != 2 {
                return Err(crate::KreuzbergError::Validation {
                    message: format!("Expected 2 processors to run, got {}", order.len()),
                    source: None,
                });
            }

            if order[0] != "early" || order[1] != "late" {
                return Err(crate::KreuzbergError::Validation {
                    message: format!("Wrong execution order: {:?}", order),
                    source: None,
                });
            }

            Ok(())
        }
    }

    let pp_registry = crate::plugins::registry::get_post_processor_registry();
    let val_registry = crate::plugins::registry::get_validator_registry();

    pp_registry.write().unwrap().shutdown_all().unwrap();
    val_registry.write().unwrap().shutdown_all().unwrap();
    clear_processor_cache().unwrap();

    {
        let mut registry = pp_registry.write().unwrap();
        registry.register(Arc::new(EarlyProcessor), 0).unwrap();
        registry.register(Arc::new(LateProcessor), 0).unwrap();
    }

    {
        let mut registry = val_registry.write().unwrap();
        registry.register(Arc::new(OrderValidator)).unwrap();
    }

    // Clear the cache after registering new processors so it rebuilds with the test processors
    clear_processor_cache().unwrap();

    let result = ExtractionResult {
        content: "test".to_string(),
        mime_type: Cow::Borrowed("text/plain"),
        metadata: Metadata::default(),
        tables: vec![],
        detected_languages: None,
        chunks: None,
        images: None,
        djot_content: None,
        pages: None,
        elements: None,
        ocr_elements: None,
        document: None,
        #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
        extracted_keywords: None,
        quality_score: None,
        processing_warnings: Vec::new(),
        annotations: None,
    };

    let config = ExtractionConfig::default();

    let processed = run_pipeline(result, &config).await;

    pp_registry.write().unwrap().shutdown_all().unwrap();
    val_registry.write().unwrap().shutdown_all().unwrap();
    clear_processor_cache().unwrap();

    assert!(processed.is_ok(), "All processors should run before validator");
}

#[tokio::test]
#[serial]
async fn test_run_pipeline_with_output_format_plain() {
    let result = ExtractionResult {
        content: "test content".to_string(),
        mime_type: Cow::Borrowed("text/plain"),
        ..Default::default()
    };

    let config = crate::core::config::ExtractionConfig {
        output_format: OutputFormat::Plain,
        ..Default::default()
    };

    let processed = run_pipeline(result, &config).await.unwrap();
    assert_eq!(processed.content, "test content");
    assert_eq!(processed.metadata.output_format, Some("plain".to_string()));
}

#[tokio::test]
#[serial]
async fn test_run_pipeline_with_output_format_djot() {
    use crate::types::{BlockType, DjotContent, FormattedBlock, InlineElement, InlineType};

    let result = ExtractionResult {
        content: "test content".to_string(),
        mime_type: Cow::Borrowed("text/djot"),
        djot_content: Some(DjotContent {
            plain_text: "test content".to_string(),
            blocks: vec![FormattedBlock {
                block_type: BlockType::Paragraph,
                level: None,
                inline_content: vec![InlineElement {
                    element_type: InlineType::Text,
                    content: "test content".to_string(),
                    attributes: None,
                    metadata: None,
                }],
                attributes: None,
                language: None,
                code: None,
                children: vec![],
            }],
            metadata: Metadata::default(),
            tables: vec![],
            images: vec![],
            links: vec![],
            footnotes: vec![],
            attributes: Vec::new(),
        }),
        ..Default::default()
    };

    let config = crate::core::config::ExtractionConfig {
        output_format: OutputFormat::Djot,
        ..Default::default()
    };

    let processed = run_pipeline(result, &config).await.unwrap();
    // The content should still be present
    assert!(!processed.content.is_empty());
    assert_eq!(processed.metadata.output_format, Some("djot".to_string()));
}

#[tokio::test]
#[serial]
async fn test_run_pipeline_with_output_format_html() {
    let result = ExtractionResult {
        content: "test content".to_string(),
        mime_type: Cow::Borrowed("text/plain"),
        ..Default::default()
    };

    let config = crate::core::config::ExtractionConfig {
        output_format: OutputFormat::Html,
        ..Default::default()
    };

    let processed = run_pipeline(result, &config).await.unwrap();
    // For non-djot documents, HTML wraps content in <pre> tags
    assert!(processed.content.contains("<pre>"));
    assert!(processed.content.contains("test content"));
    assert!(processed.content.contains("</pre>"));
    assert_eq!(processed.metadata.output_format, Some("html".to_string()));
}

#[tokio::test]
#[serial]
#[cfg(feature = "quality")]
async fn test_nfc_normalization_decomposes_to_composed() {
    // NFC normalization should convert decomposed characters to composed form.
    // "e\u{0301}" (e + combining acute accent) → "\u{00e9}" (é precomposed)
    let result = ExtractionResult {
        content: "caf\u{0065}\u{0301}".to_string(), // "café" with decomposed é
        mime_type: Cow::Borrowed("text/plain"),
        ..Default::default()
    };
    let config = ExtractionConfig {
        postprocessor: Some(crate::core::config::PostProcessorConfig {
            enabled: false,
            ..Default::default()
        }),
        ..Default::default()
    };

    let processed = run_pipeline(result, &config).await.unwrap();
    assert_eq!(processed.content, "caf\u{00e9}"); // composed é
    assert!(!processed.content.contains('\u{0301}')); // no combining accent
}

#[tokio::test]
#[serial]
#[cfg(feature = "quality")]
async fn test_nfc_normalization_idempotent_on_ascii() {
    // NFC on already-normalized/ASCII text should be a no-op.
    let result = ExtractionResult {
        content: "Hello, world! 123".to_string(),
        mime_type: Cow::Borrowed("text/plain"),
        ..Default::default()
    };
    let config = ExtractionConfig {
        postprocessor: Some(crate::core::config::PostProcessorConfig {
            enabled: false,
            ..Default::default()
        }),
        ..Default::default()
    };

    let processed = run_pipeline(result, &config).await.unwrap();
    assert_eq!(processed.content, "Hello, world! 123");
}

#[tokio::test]
#[serial]
#[cfg(feature = "quality")]
async fn test_nfc_normalization_applies_to_page_content() {
    use crate::types::PageContent;

    let result = ExtractionResult {
        content: "caf\u{0065}\u{0301}".to_string(),
        mime_type: Cow::Borrowed("text/plain"),
        pages: Some(vec![PageContent {
            page_number: 1,
            content: "re\u{0301}sume\u{0301}".to_string(), // "résumé" decomposed
            tables: vec![],
            images: vec![],
            hierarchy: None,
            is_blank: None,
        }]),
        ..Default::default()
    };
    let config = ExtractionConfig {
        postprocessor: Some(crate::core::config::PostProcessorConfig {
            enabled: false,
            ..Default::default()
        }),
        ..Default::default()
    };

    let processed = run_pipeline(result, &config).await.unwrap();
    assert_eq!(processed.content, "caf\u{00e9}");
    let pages = processed.pages.unwrap();
    assert_eq!(pages[0].content, "r\u{00e9}sum\u{00e9}");
}

#[tokio::test]
#[serial]
async fn test_run_pipeline_applies_output_format_last() {
    // This test verifies that output format is applied after all other processing
    use crate::types::DjotContent;

    let result = ExtractionResult {
        content: "test".to_string(),
        mime_type: Cow::Borrowed("text/plain"),
        djot_content: Some(DjotContent {
            plain_text: "test".to_string(),
            blocks: vec![],
            metadata: Metadata::default(),
            tables: vec![],
            images: vec![],
            links: vec![],
            footnotes: vec![],
            attributes: Vec::new(),
        }),
        ..Default::default()
    };

    let config = crate::core::config::ExtractionConfig {
        output_format: OutputFormat::Djot,
        // Disable other processing to ensure pipeline runs cleanly
        enable_quality_processing: false,
        ..Default::default()
    };

    let processed = run_pipeline(result, &config).await.unwrap();
    // The result should have gone through the pipeline successfully
    assert!(processed.djot_content.is_some());
    assert_eq!(processed.metadata.output_format, Some("djot".to_string()));
}
