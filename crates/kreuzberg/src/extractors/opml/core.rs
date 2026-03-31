//! Core OPML extractor implementation.
//!
//! This module provides the main `OpmlExtractor` struct and implements the
//! `Plugin` and `DocumentExtractor` traits for OPML document processing.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::internal::InternalDocument;
use crate::types::metadata::Metadata;
use async_trait::async_trait;

#[cfg(feature = "office")]
use super::parser;

/// OPML format extractor.
///
/// Extracts outline structure and metadata from OPML documents using native Rust parsing.
pub struct OpmlExtractor;

impl OpmlExtractor {
    /// Create a new OPML extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Default for OpmlExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for OpmlExtractor {
    fn name(&self) -> &str {
        "opml-extractor"
    }

    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    fn initialize(&self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    fn description(&self) -> &str {
        "Extracts content and metadata from OPML (Outline Processor Markup Language) documents"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

#[cfg(feature = "office")]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl DocumentExtractor for OpmlExtractor {
    #[cfg_attr(
        feature = "otel",
        tracing::instrument(
            skip(self, content, _config),
            fields(
                extractor.name = self.name(),
                content.size_bytes = content.len(),
            )
        )
    )]
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        tracing::debug!(format = "opml", size_bytes = content.len(), "extraction starting");
        let (_extracted_content, mut metadata_map) = parser::extract_content_and_metadata(content)?;

        // Map standard OPML metadata to typed Metadata fields
        let meta_title = metadata_map
            .remove("title")
            .and_then(|v| v.as_str().map(|s| s.to_string()));
        let meta_created_by = metadata_map
            .remove("ownerName")
            .and_then(|v| v.as_str().map(|s| s.to_string()));
        let meta_created_at = metadata_map
            .remove("dateCreated")
            .and_then(|v| v.as_str().map(|s| s.to_string()));
        let meta_modified_at = metadata_map
            .remove("dateModified")
            .and_then(|v| v.as_str().map(|s| s.to_string()));

        let mut doc = parser::build_internal_document(content)?;
        doc.mime_type = std::borrow::Cow::Owned(mime_type.to_string());
        doc.metadata = Metadata {
            title: meta_title,
            created_by: meta_created_by,
            created_at: meta_created_at,
            modified_at: meta_modified_at,
            additional: metadata_map,
            ..Default::default()
        };

        tracing::debug!(
            element_count = doc.elements.len(),
            format = "opml",
            "extraction complete"
        );
        Ok(doc)
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["text/x-opml", "application/xml+opml", "application/x-opml+xml"]
    }

    fn priority(&self) -> i32 {
        55
    }
}

#[cfg(all(test, feature = "office"))]
mod tests {
    use super::*;

    #[test]
    fn test_opml_extractor_plugin_interface() {
        let extractor = OpmlExtractor::new();
        assert_eq!(extractor.name(), "opml-extractor");
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert_eq!(extractor.priority(), 55);
        assert!(!extractor.supported_mime_types().is_empty());
    }

    #[test]
    fn test_opml_extractor_default() {
        let extractor = OpmlExtractor;
        assert_eq!(extractor.name(), "opml-extractor");
    }

    #[tokio::test]
    async fn test_opml_extractor_initialize_shutdown() {
        let extractor = OpmlExtractor::new();
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[test]
    fn test_opml_supported_mime_types() {
        let extractor = OpmlExtractor::new();
        let supported = extractor.supported_mime_types();
        assert!(supported.contains(&"text/x-opml"));
        assert!(supported.contains(&"application/xml+opml"));
        assert!(supported.contains(&"application/x-opml+xml"));
    }

    #[tokio::test]
    async fn test_opml_extractor_async_extraction() {
        let extractor = OpmlExtractor::new();
        let opml = br#"<?xml version="1.0"?>
<opml version="2.0">
  <head>
    <title>Async Test</title>
  </head>
  <body>
    <outline text="Item" />
  </body>
</opml>"#;

        let result = extractor
            .extract_bytes(opml, "text/x-opml", &ExtractionConfig::default())
            .await
            .expect("Should extract OPML asynchronously");
        let result =
            crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);

        assert_eq!(result.mime_type, "text/x-opml");
        assert!(result.content.contains("Item"));
        assert_eq!(result.metadata.title.as_deref(), Some("Async Test"));
    }
}
