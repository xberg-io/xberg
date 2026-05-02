//! Document extractor plugin system.
//!
//! This module provides the trait and registry for implementing custom document extractors.

mod registry;
mod r#trait;

#[cfg(feature = "otel")]
pub(crate) mod instrumented;

// Re-export trait for backward compatibility
pub use r#trait::DocumentExtractor;

/// List names of all registered document extractors.
pub fn list_document_extractors() -> crate::Result<Vec<String>> {
    use crate::plugins::registry::get_document_extractor_registry;

    let registry = get_document_extractor_registry();
    let registry = registry.read();

    Ok(registry.list())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Result;
    use crate::core::config::ExtractionConfig;
    use crate::plugins::Plugin;
    use async_trait::async_trait;

    struct MockExtractor {
        mime_types: Vec<&'static str>,
        priority: i32,
    }

    impl Plugin for MockExtractor {
        fn name(&self) -> &str {
            "mock-extractor"
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
    impl DocumentExtractor for MockExtractor {
        async fn extract_bytes(
            &self,
            content: &[u8],
            mime_type: &str,
            _config: &ExtractionConfig,
        ) -> Result<crate::types::internal::InternalDocument> {
            let mut doc = crate::types::internal::InternalDocument::new("mock");
            doc.mime_type = std::borrow::Cow::Owned(mime_type.to_string());
            let text = String::from_utf8_lossy(content).to_string();
            if !text.is_empty() {
                doc.push_element(crate::types::internal::InternalElement::text(
                    crate::types::internal::ElementKind::Paragraph,
                    text,
                    0,
                ));
            }
            Ok(doc)
        }

        fn supported_mime_types(&self) -> &[&str] {
            &self.mime_types
        }

        fn priority(&self) -> i32 {
            self.priority
        }
    }

    #[tokio::test]
    async fn test_document_extractor_extract_bytes() {
        let extractor = MockExtractor {
            mime_types: vec!["text/plain"],
            priority: 50,
        };

        let config = ExtractionConfig::default();
        let doc = extractor
            .extract_bytes(b"test content", "text/plain", &config)
            .await
            .unwrap();
        let result =
            crate::extraction::derive::derive_extraction_result(doc, true, crate::core::config::OutputFormat::Plain);

        assert_eq!(result.content, "test content");
        assert_eq!(result.mime_type, "text/plain");
    }

    #[test]
    fn test_document_extractor_supported_mime_types() {
        let extractor = MockExtractor {
            mime_types: vec!["text/plain", "text/markdown"],
            priority: 50,
        };

        let supported = extractor.supported_mime_types();
        assert_eq!(supported.len(), 2);
        assert!(supported.contains(&"text/plain"));
        assert!(supported.contains(&"text/markdown"));
    }

    #[test]
    fn test_document_extractor_priority() {
        let low_priority = MockExtractor {
            mime_types: vec!["text/plain"],
            priority: 10,
        };

        let high_priority = MockExtractor {
            mime_types: vec!["text/plain"],
            priority: 100,
        };

        assert_eq!(low_priority.priority(), 10);
        assert_eq!(high_priority.priority(), 100);
    }

    #[test]
    fn test_document_extractor_can_handle_default() {
        let extractor = MockExtractor {
            mime_types: vec!["text/plain"],
            priority: 50,
        };

        use std::path::PathBuf;
        let path = PathBuf::from("test.txt");

        assert!(extractor.can_handle(&path, "text/plain"));
        assert!(extractor.can_handle(&path, "application/pdf"));
    }

    #[tokio::test]
    async fn test_document_extractor_extract_file_default_impl() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let extractor = MockExtractor {
            mime_types: vec!["text/plain"],
            priority: 50,
        };

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"file content").unwrap();
        let path = temp_file.path();

        let config = ExtractionConfig::default();
        let doc = extractor.extract_file(path, "text/plain", &config).await.unwrap();
        let result =
            crate::extraction::derive::derive_extraction_result(doc, true, crate::core::config::OutputFormat::Plain);

        assert_eq!(result.content, "file content");
        assert_eq!(result.mime_type, "text/plain");
    }

    #[tokio::test]
    async fn test_document_extractor_empty_content() {
        let extractor = MockExtractor {
            mime_types: vec!["text/plain"],
            priority: 50,
        };

        let config = ExtractionConfig::default();
        let doc = extractor.extract_bytes(b"", "text/plain", &config).await.unwrap();
        let result =
            crate::extraction::derive::derive_extraction_result(doc, true, crate::core::config::OutputFormat::Plain);

        assert_eq!(result.content, "");
        assert_eq!(result.mime_type, "text/plain");
    }

    #[tokio::test]
    async fn test_document_extractor_non_utf8_content() {
        let extractor = MockExtractor {
            mime_types: vec!["text/plain"],
            priority: 50,
        };

        let invalid_utf8 = vec![0xFF, 0xFE, 0xFD];
        let config = ExtractionConfig::default();
        let doc = extractor
            .extract_bytes(&invalid_utf8, "text/plain", &config)
            .await
            .unwrap();
        let result =
            crate::extraction::derive::derive_extraction_result(doc, true, crate::core::config::OutputFormat::Plain);

        assert!(!result.content.is_empty());
    }

    #[test]
    fn test_document_extractor_plugin_interface() {
        let extractor = MockExtractor {
            mime_types: vec!["text/plain"],
            priority: 50,
        };

        assert_eq!(extractor.name(), "mock-extractor");
        assert_eq!(extractor.version(), "1.0.0");
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[test]
    fn test_document_extractor_default_priority() {
        struct DefaultPriorityExtractor;

        impl Plugin for DefaultPriorityExtractor {
            fn name(&self) -> &str {
                "default"
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
        impl DocumentExtractor for DefaultPriorityExtractor {
            async fn extract_bytes(
                &self,
                _content: &[u8],
                _mime_type: &str,
                _config: &ExtractionConfig,
            ) -> Result<crate::types::internal::InternalDocument> {
                Ok(crate::types::internal::InternalDocument::new("mock"))
            }

            fn supported_mime_types(&self) -> &[&str] {
                &["text/plain"]
            }
        }

        let extractor = DefaultPriorityExtractor;
        assert_eq!(extractor.priority(), 50);
    }

    #[test]
    fn test_document_extractor_empty_mime_types() {
        let extractor = MockExtractor {
            mime_types: vec![],
            priority: 50,
        };

        assert_eq!(extractor.supported_mime_types().len(), 0);
    }

    #[test]
    fn test_document_extractor_wildcard_mime_types() {
        let extractor = MockExtractor {
            mime_types: vec!["text/*", "image/*"],
            priority: 50,
        };

        let supported = extractor.supported_mime_types();
        assert_eq!(supported.len(), 2);
        assert!(supported.contains(&"text/*"));
        assert!(supported.contains(&"image/*"));
    }

    #[test]
    fn test_document_extractor_priority_ranges() {
        let fallback = MockExtractor {
            mime_types: vec!["text/plain"],
            priority: 10,
        };
        let alternative = MockExtractor {
            mime_types: vec!["text/plain"],
            priority: 40,
        };
        let default = MockExtractor {
            mime_types: vec!["text/plain"],
            priority: 50,
        };
        let premium = MockExtractor {
            mime_types: vec!["text/plain"],
            priority: 75,
        };
        let specialized = MockExtractor {
            mime_types: vec!["text/plain"],
            priority: 100,
        };

        assert!(fallback.priority() < alternative.priority());
        assert!(alternative.priority() < default.priority());
        assert!(default.priority() < premium.priority());
        assert!(premium.priority() < specialized.priority());
    }

    #[tokio::test]
    async fn test_document_extractor_preserves_mime_type() {
        let extractor = MockExtractor {
            mime_types: vec!["application/json"],
            priority: 50,
        };

        let config = ExtractionConfig::default();
        let result = extractor
            .extract_bytes(b"{\"key\":\"value\"}", "application/json", &config)
            .await
            .unwrap();

        assert_eq!(result.mime_type, "application/json");
    }
}
