//! Post-processor plugin system.
//!
//! This module provides the trait and registry for implementing custom post-processors.

mod registry;
mod r#trait;

// Re-export trait and enum for backward compatibility
pub use r#trait::{PostProcessor, ProcessingStage};

// Re-export registry functions for backward compatibility
pub use registry::list_post_processors;

use std::sync::Arc;

/// Register a post-processor plugin with the global registry.
///
/// The processor's execution priority is taken from its
/// [`PostProcessor::priority`] method (default: 50). Higher values run first
/// within the same processing stage.
///
/// # Arguments
///
/// * `processor` - The post-processor to register
pub fn register_post_processor(processor: Arc<dyn PostProcessor>) -> crate::Result<()> {
    use crate::plugins::registry::get_post_processor_registry;

    let registry = get_post_processor_registry();
    let mut registry = registry.write();

    registry.register(processor)
}

/// Unregister a post-processor by name.
pub fn unregister_post_processor(name: &str) -> crate::Result<()> {
    use crate::plugins::registry::get_post_processor_registry;

    let registry = get_post_processor_registry();
    let mut registry = registry.write();

    registry.remove(name)
}

/// Remove all registered post-processors.
pub fn clear_post_processors() -> crate::Result<()> {
    use crate::plugins::registry::get_post_processor_registry;

    let registry = get_post_processor_registry();
    let mut registry = registry.write();

    registry.shutdown_all()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Result;
    use crate::core::config::ExtractionConfig;
    use crate::plugins::Plugin;
    use crate::types::ExtractionResult;
    use ahash::AHashMap;
    use async_trait::async_trait;
    use std::borrow::Cow;

    struct MockPostProcessor {
        stage: ProcessingStage,
    }

    impl Plugin for MockPostProcessor {
        fn name(&self) -> &str {
            "mock-processor"
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
    impl PostProcessor for MockPostProcessor {
        async fn process(&self, result: &mut ExtractionResult, _config: &ExtractionConfig) -> Result<()> {
            result
                .metadata
                .additional
                .insert(Cow::Borrowed("processed_by"), serde_json::json!(self.name()));
            Ok(())
        }

        fn processing_stage(&self) -> ProcessingStage {
            self.stage
        }
    }

    #[tokio::test]
    async fn test_post_processor_process() {
        let processor = MockPostProcessor {
            stage: ProcessingStage::Early,
        };

        let mut result = ExtractionResult {
            content: "test content".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            ..Default::default()
        };

        let config = ExtractionConfig::default();
        processor.process(&mut result, &config).await.unwrap();

        assert_eq!(result.content, "test content");
        assert_eq!(
            result.metadata.additional.get("processed_by").unwrap(),
            &serde_json::json!("mock-processor")
        );
    }

    #[test]
    fn test_processing_stage_order() {
        assert!(ProcessingStage::Early < ProcessingStage::Middle);
        assert!(ProcessingStage::Middle < ProcessingStage::Late);
    }

    #[test]
    fn test_post_processor_stage() {
        let early = MockPostProcessor {
            stage: ProcessingStage::Early,
        };
        let middle = MockPostProcessor {
            stage: ProcessingStage::Middle,
        };
        let late = MockPostProcessor {
            stage: ProcessingStage::Late,
        };

        assert_eq!(early.processing_stage(), ProcessingStage::Early);
        assert_eq!(middle.processing_stage(), ProcessingStage::Middle);
        assert_eq!(late.processing_stage(), ProcessingStage::Late);
    }

    #[test]
    fn test_post_processor_should_process_default() {
        let processor = MockPostProcessor {
            stage: ProcessingStage::Early,
        };

        let result = ExtractionResult {
            content: "test".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            ..Default::default()
        };

        let config = ExtractionConfig::default();

        assert!(processor.should_process(&result, &config));
    }

    #[test]
    fn test_processing_stage_equality() {
        assert_eq!(ProcessingStage::Early, ProcessingStage::Early);
        assert_ne!(ProcessingStage::Early, ProcessingStage::Middle);
        assert_ne!(ProcessingStage::Middle, ProcessingStage::Late);
    }

    #[test]
    fn test_processing_stage_clone() {
        let stage = ProcessingStage::Middle;
        let cloned = stage;
        assert_eq!(stage, cloned);
    }

    #[test]
    fn test_processing_stage_debug() {
        let stage = ProcessingStage::Early;
        let debug_str = format!("{:?}", stage);
        assert!(debug_str.contains("Early"));
    }

    #[test]
    fn test_processing_stage_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(ProcessingStage::Early);
        set.insert(ProcessingStage::Middle);
        set.insert(ProcessingStage::Late);

        assert_eq!(set.len(), 3);
        assert!(set.contains(&ProcessingStage::Early));
    }

    #[tokio::test]
    async fn test_post_processor_plugin_interface() {
        let processor = MockPostProcessor {
            stage: ProcessingStage::Middle,
        };

        assert_eq!(processor.name(), "mock-processor");
        assert_eq!(processor.version(), "1.0.0");
        assert!(processor.initialize().is_ok());
        assert!(processor.shutdown().is_ok());
    }

    #[tokio::test]
    async fn test_post_processor_empty_content() {
        let processor = MockPostProcessor {
            stage: ProcessingStage::Early,
        };

        let mut result = ExtractionResult {
            mime_type: Cow::Borrowed("text/plain"),
            ..Default::default()
        };

        let config = ExtractionConfig::default();
        processor.process(&mut result, &config).await.unwrap();

        assert_eq!(result.content, "");
        assert!(result.metadata.additional.contains_key("processed_by"));
    }

    #[tokio::test]
    async fn test_post_processor_preserves_metadata() {
        let processor = MockPostProcessor {
            stage: ProcessingStage::Early,
        };

        let mut additional = AHashMap::new();
        additional.insert(Cow::Borrowed("existing_key"), serde_json::json!("existing_value"));

        let mut result = ExtractionResult {
            content: "test".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            metadata: crate::types::Metadata {
                additional,
                ..Default::default()
            },
            ..Default::default()
        };

        let config = ExtractionConfig::default();
        processor.process(&mut result, &config).await.unwrap();

        assert_eq!(
            result.metadata.additional.get("existing_key").unwrap(),
            &serde_json::json!("existing_value")
        );
        assert!(result.metadata.additional.contains_key("processed_by"));
    }

    #[test]
    fn test_post_processor_estimated_duration_default() {
        let processor = MockPostProcessor {
            stage: ProcessingStage::Early,
        };

        let result = ExtractionResult {
            content: "test".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            ..Default::default()
        };

        assert_eq!(processor.estimated_duration_ms(&result), 0);
    }

    #[test]
    fn test_post_processor_should_process_conditional() {
        struct PdfOnlyProcessor;

        impl Plugin for PdfOnlyProcessor {
            fn name(&self) -> &str {
                "pdf-only"
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
        impl PostProcessor for PdfOnlyProcessor {
            async fn process(&self, _result: &mut ExtractionResult, _config: &ExtractionConfig) -> Result<()> {
                Ok(())
            }

            fn processing_stage(&self) -> ProcessingStage {
                ProcessingStage::Middle
            }

            fn should_process(&self, result: &ExtractionResult, _config: &ExtractionConfig) -> bool {
                result.mime_type == "application/pdf"
            }
        }

        let processor = PdfOnlyProcessor;
        let config = ExtractionConfig::default();

        let pdf_result = ExtractionResult {
            content: "test".to_string(),
            mime_type: Cow::Borrowed("application/pdf"),
            ..Default::default()
        };

        let txt_result = ExtractionResult {
            content: "test".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            ..Default::default()
        };

        assert!(processor.should_process(&pdf_result, &config));
        assert!(!processor.should_process(&txt_result, &config));
    }

    #[tokio::test]
    async fn test_post_processor_preserves_tables() {
        use crate::types::Table;

        let processor = MockPostProcessor {
            stage: ProcessingStage::Early,
        };

        let table = Table {
            cells: vec![vec!["A".to_string(), "B".to_string()]],
            markdown: "| A | B |".to_string(),
            page_number: 0,
            bounding_box: None,
        };

        let mut result = ExtractionResult {
            content: "test".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            tables: vec![table],
            ..Default::default()
        };

        let config = ExtractionConfig::default();
        processor.process(&mut result, &config).await.unwrap();

        assert_eq!(result.tables.len(), 1);
        assert_eq!(result.tables[0].cells.len(), 1);
    }
}
