//! Pipeline orchestration tests.

use super::*;
use crate::core::config::OutputFormat;
use crate::types::Metadata;
use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
use serial_test::serial;
use std::borrow::Cow;

/// Build an `InternalDocument` with a single paragraph element for pipeline tests.
fn make_doc(content: &str, mime: &str) -> InternalDocument {
    let mut doc = InternalDocument::new("plain");
    doc.mime_type = Cow::Owned(mime.to_string());
    if !content.is_empty() {
        doc.push_element(InternalElement::text(ElementKind::Paragraph, content, 0));
    }
    doc
}

/// Build an `InternalDocument` with content, mime, and custom metadata.
fn make_doc_with_metadata(content: &str, mime: &str, metadata: Metadata) -> InternalDocument {
    let mut doc = make_doc(content, mime);
    doc.metadata = metadata;
    doc
}

const VALIDATION_MARKER_KEY: &str = "registry_validation_marker";
#[cfg(feature = "quality")]
const QUALITY_VALIDATION_MARKER: &str = "quality_validation_test";
const POSTPROCESSOR_VALIDATION_MARKER: &str = "postprocessor_validation_test";
const ORDER_VALIDATION_MARKER: &str = "order_validation_test";

/// Ensure the quality processor is registered and cache is fresh.
/// Needed because other tests may call `shutdown_all()` on the registry,
/// and the `OnceLock` in `initialize_features` prevents re-registration.
#[cfg(feature = "quality")]
fn ensure_quality_processor() {
    let registry = crate::plugins::registry::get_post_processor_registry();
    let mut reg = registry.write();
    let _ = reg.register(std::sync::Arc::new(crate::text::QualityProcessor));
    drop(reg);
    let _ = clear_processor_cache();
}

#[tokio::test]
#[serial]
async fn test_run_pipeline_basic() {
    let mut doc = make_doc("test", "text/plain");
    doc.metadata.additional.insert(
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

    let processed = run_pipeline(doc, &config).await.unwrap();
    assert_eq!(processed.content, "test");
}

#[tokio::test]
#[serial]
#[cfg(feature = "quality")]
async fn test_pipeline_with_quality_processing() {
    ensure_quality_processor();
    let doc = make_doc("This is a test document with some meaningful content.", "text/plain");
    let config = ExtractionConfig {
        enable_quality_processing: true,
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await.unwrap();
    assert!(processed.quality_score.is_some());
}

#[tokio::test]
#[serial]
async fn test_pipeline_without_quality_processing() {
    let doc = make_doc("test", "text/plain");
    let config = ExtractionConfig {
        enable_quality_processing: false,
        postprocessor: Some(crate::core::config::PostProcessorConfig {
            enabled: false,
            ..Default::default()
        }),
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await.unwrap();
    assert!(processed.quality_score.is_none());
}

#[tokio::test]
#[serial]
#[cfg(feature = "chunking")]
async fn test_pipeline_with_chunking() {
    let doc = make_doc(
        &"This is a long text that should be chunked. ".repeat(100),
        "text/plain",
    );
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

    let processed = run_pipeline(doc, &config).await.unwrap();
    let chunks = processed.chunks.as_ref().expect("chunks should be present");
    assert!(chunks.len() > 1);
}

#[tokio::test]
#[serial]
async fn test_pipeline_without_chunking() {
    let doc = make_doc("test", "text/plain");
    let config = ExtractionConfig {
        chunking: None,
        postprocessor: Some(crate::core::config::PostProcessorConfig {
            enabled: false,
            ..Default::default()
        }),
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await.unwrap();
    assert!(processed.chunks.is_none());
}

#[tokio::test]
#[serial]
async fn test_pipeline_preserves_metadata() {
    use ahash::AHashMap;
    let mut additional = AHashMap::new();
    additional.insert(Cow::Borrowed("source"), serde_json::json!("test"));
    additional.insert(Cow::Borrowed("page"), serde_json::json!(1));

    let doc = make_doc_with_metadata(
        "test",
        "text/plain",
        Metadata {
            additional,
            ..Default::default()
        },
    );
    let config = ExtractionConfig {
        postprocessor: Some(crate::core::config::PostProcessorConfig {
            enabled: false,
            ..Default::default()
        }),
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await.unwrap();
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

    let mut doc = make_doc("test", "text/plain");
    doc.tables.push(table);
    let config = ExtractionConfig {
        postprocessor: Some(crate::core::config::PostProcessorConfig {
            enabled: false,
            ..Default::default()
        }),
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await.unwrap();
    assert_eq!(processed.tables.len(), 1);
    assert_eq!(processed.tables[0].cells.len(), 1);
}

#[tokio::test]
#[serial]
async fn test_pipeline_empty_content() {
    {
        let registry = crate::plugins::registry::get_post_processor_registry();
        registry.write().shutdown_all().unwrap();
    }
    {
        let registry = crate::plugins::registry::get_validator_registry();
        registry.write().shutdown_all().unwrap();
    }

    let doc = make_doc("", "text/plain");
    let config = ExtractionConfig::default();

    let processed = run_pipeline(doc, &config).await.unwrap();
    assert_eq!(processed.content, "");
}

#[tokio::test]
#[serial]
#[cfg(feature = "chunking")]
async fn test_pipeline_with_all_features() {
    #[cfg(feature = "quality")]
    ensure_quality_processor();
    let doc = make_doc(&"This is a comprehensive test document. ".repeat(50), "text/plain");
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

    let processed = run_pipeline(doc, &config).await.unwrap();
    #[cfg(feature = "quality")]
    assert!(processed.quality_score.is_some());
    assert!(processed.chunks.is_some());
}

#[tokio::test]
#[serial]
#[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
async fn test_pipeline_with_keyword_extraction() {
    crate::plugins::registry::get_validator_registry()
        .write()
        .shutdown_all()
        .unwrap();
    crate::plugins::registry::get_post_processor_registry()
        .write()
        .shutdown_all()
        .unwrap();

    // Register keyword processor directly (bypasses Lazy which only runs once per process)
    let _ = crate::keywords::register_keyword_processor();
    clear_processor_cache().unwrap();

    let doc = make_doc(
        r#"
Machine learning is a branch of artificial intelligence that focuses on
building systems that can learn from data. Deep learning is a subset of
machine learning that uses neural networks with multiple layers.
Natural language processing enables computers to understand human language.
            "#,
        "text/plain",
    );
    #[cfg(feature = "keywords-yake")]
    let keyword_config = crate::keywords::KeywordConfig::yake();

    #[cfg(all(feature = "keywords-rake", not(feature = "keywords-yake")))]
    let keyword_config = crate::keywords::KeywordConfig::rake();

    let config = ExtractionConfig {
        keywords: Some(keyword_config),
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await.unwrap();

    let keywords = processed
        .extracted_keywords
        .as_ref()
        .expect("Should have extracted keywords");
    assert!(!keywords.is_empty(), "Should have extracted keywords");

    let first_keyword = &keywords[0];
    assert!(!first_keyword.text.is_empty());
    assert!(first_keyword.score > 0.0);
}

#[tokio::test]
#[serial]
#[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
async fn test_pipeline_without_keyword_config() {
    let doc = make_doc("Machine learning and artificial intelligence.", "text/plain");

    let config = ExtractionConfig {
        keywords: None,
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await.unwrap();

    assert!(!processed.metadata.additional.contains_key("keywords"));
}

#[tokio::test]
#[serial]
#[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
async fn test_pipeline_keyword_extraction_short_content() {
    crate::plugins::registry::get_validator_registry()
        .write()
        .shutdown_all()
        .unwrap();
    crate::plugins::registry::get_post_processor_registry()
        .write()
        .shutdown_all()
        .unwrap();

    let doc = make_doc("Short text", "text/plain");

    #[cfg(feature = "keywords-yake")]
    let keyword_config = crate::keywords::KeywordConfig::yake();

    #[cfg(all(feature = "keywords-rake", not(feature = "keywords-yake")))]
    let keyword_config = crate::keywords::KeywordConfig::rake();

    let config = ExtractionConfig {
        keywords: Some(keyword_config),
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await.unwrap();

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
    pp_registry.write().shutdown_all().unwrap();
    val_registry.write().shutdown_all().unwrap();
    clear_processor_cache().unwrap();

    {
        let mut registry = pp_registry.write();
        registry.register(Arc::new(TestPostProcessor)).unwrap();
    }

    {
        let mut registry = val_registry.write();
        registry.register(Arc::new(TestValidator)).unwrap();
    }

    clear_processor_cache().unwrap();

    let mut doc = make_doc("test", "text/plain");
    doc.metadata.additional.insert(
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

    let processed = run_pipeline(doc, &config).await;

    pp_registry.write().shutdown_all().unwrap();
    val_registry.write().shutdown_all().unwrap();

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
    ensure_quality_processor();
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

            if result.quality_score.is_none() {
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
        let mut registry = val_registry.write();
        registry.register(Arc::new(QualityValidator)).unwrap();
    }

    let mut doc = make_doc("This is meaningful test content for quality scoring.", "text/plain");
    doc.metadata.additional.insert(
        Cow::Borrowed(VALIDATION_MARKER_KEY),
        serde_json::json!(QUALITY_VALIDATION_MARKER),
    );

    let config = ExtractionConfig {
        enable_quality_processing: true,
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await;

    {
        let mut registry = val_registry.write();
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

    pp_registry.write().shutdown_all().unwrap();
    val_registry.write().shutdown_all().unwrap();
    clear_processor_cache().unwrap();

    {
        let mut registry = pp_registry.write();
        registry.register(Arc::new(EarlyProcessor)).unwrap();
        registry.register(Arc::new(LateProcessor)).unwrap();
    }

    {
        let mut registry = val_registry.write();
        registry.register(Arc::new(OrderValidator)).unwrap();
    }

    // Clear the cache after registering new processors so it rebuilds with the test processors
    clear_processor_cache().unwrap();

    let doc = make_doc("test", "text/plain");

    let config = ExtractionConfig::default();

    let processed = run_pipeline(doc, &config).await;

    pp_registry.write().shutdown_all().unwrap();
    val_registry.write().shutdown_all().unwrap();
    clear_processor_cache().unwrap();

    assert!(processed.is_ok(), "All processors should run before validator");
}

#[tokio::test]
#[serial]
async fn test_run_pipeline_with_output_format_plain() {
    let doc = make_doc("test content", "text/plain");

    let config = crate::core::config::ExtractionConfig {
        output_format: OutputFormat::Plain,
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await.unwrap();
    assert_eq!(processed.content, "test content");
    assert_eq!(processed.metadata.output_format, Some("plain".to_string()));
}

#[tokio::test]
#[serial]
async fn test_run_pipeline_with_output_format_djot() {
    let doc = make_doc("test content", "text/djot");

    let config = crate::core::config::ExtractionConfig {
        output_format: OutputFormat::Djot,
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await.unwrap();
    // The content should still be present
    assert!(!processed.content.is_empty());
    assert_eq!(processed.metadata.output_format, Some("djot".to_string()));
}

#[tokio::test]
#[serial]
async fn test_run_pipeline_with_output_format_html() {
    let doc = make_doc("test content", "text/plain");

    let config = crate::core::config::ExtractionConfig {
        output_format: OutputFormat::Html,
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await.unwrap();
    // HTML renderer produces semantic tags from InternalDocument
    assert!(processed.content.contains("test content"));
    assert_eq!(processed.metadata.output_format, Some("html".to_string()));
}

#[tokio::test]
#[serial]
#[cfg(feature = "quality")]
async fn test_nfc_normalization_decomposes_to_composed() {
    // NFC normalization should convert decomposed characters to composed form.
    // "e\u{0301}" (e + combining acute accent) → "\u{00e9}" (é precomposed)
    let doc = make_doc("caf\u{0065}\u{0301}", "text/plain"); // "café" with decomposed é
    let config = ExtractionConfig {
        postprocessor: Some(crate::core::config::PostProcessorConfig {
            enabled: false,
            ..Default::default()
        }),
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await.unwrap();
    assert_eq!(processed.content, "caf\u{00e9}"); // composed é
    assert!(!processed.content.contains('\u{0301}')); // no combining accent
}

#[tokio::test]
#[serial]
#[cfg(feature = "quality")]
async fn test_nfc_normalization_idempotent_on_ascii() {
    // NFC on already-normalized/ASCII text should be a no-op.
    let doc = make_doc("Hello, world! 123", "text/plain");
    let config = ExtractionConfig {
        postprocessor: Some(crate::core::config::PostProcessorConfig {
            enabled: false,
            ..Default::default()
        }),
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await.unwrap();
    assert_eq!(processed.content, "Hello, world! 123");
}

#[tokio::test]
#[serial]
#[cfg(feature = "quality")]
async fn test_nfc_normalization_applies_to_page_content() {
    // Create a doc with a page-1 element containing decomposed characters
    let mut doc = InternalDocument::new("plain");
    doc.mime_type = Cow::Borrowed("text/plain");
    doc.push_element(InternalElement::text(ElementKind::Paragraph, "re\u{0301}sume\u{0301}", 0).with_page(1));
    let config = ExtractionConfig {
        postprocessor: Some(crate::core::config::PostProcessorConfig {
            enabled: false,
            ..Default::default()
        }),
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await.unwrap();
    // Content derived from page element
    assert!(processed.content.contains("r\u{00e9}sum\u{00e9}"));
    let pages = processed.pages.unwrap();
    assert_eq!(pages[0].content, "r\u{00e9}sum\u{00e9}");
}

#[tokio::test]
#[serial]
async fn test_run_pipeline_applies_output_format_last() {
    // This test verifies that output format is applied after all other processing
    let doc = make_doc("test", "text/plain");

    let config = crate::core::config::ExtractionConfig {
        output_format: OutputFormat::Djot,
        // Disable other processing to ensure pipeline runs cleanly
        enable_quality_processing: false,
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await.unwrap();
    // The result should have gone through the pipeline successfully
    assert_eq!(processed.metadata.output_format, Some("djot".to_string()));
}

#[tokio::test]
#[serial]
#[cfg(all(feature = "pdf", feature = "chunking"))]
async fn test_chunking_populates_page_numbers_for_pdf() {
    use crate::core::config::ChunkingConfig;

    let pdf_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/pdf/issue-636-chunk-pages.pdf");

    if !pdf_path.exists() {
        // Skip if test document not available
        return;
    }

    let pdf_bytes = std::fs::read(&pdf_path).unwrap();

    // Configure chunking WITHOUT explicit pages config (the default user scenario)
    let config = ExtractionConfig {
        chunking: Some(ChunkingConfig {
            max_characters: 500,
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = crate::core::extractor::extract_bytes(&pdf_bytes, "application/pdf", &config)
        .await
        .unwrap();

    // Chunks should exist
    assert!(result.chunks.is_some(), "Chunks should be produced");
    let chunks = result.chunks.as_ref().unwrap();
    assert!(!chunks.is_empty(), "Should have at least one chunk");

    // At least some chunks should have page numbers
    let chunks_with_pages = chunks.iter().filter(|c| c.metadata.first_page.is_some()).count();
    assert!(
        chunks_with_pages > 0,
        "At least some chunks should have page numbers, but none do. Total chunks: {}",
        chunks.len()
    );
}
