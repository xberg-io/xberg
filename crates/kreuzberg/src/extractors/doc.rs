//! Native DOC extractor for Word 97-2003 binary format.
//!
//! Extracts text directly from OLE/CFB compound documents without LibreOffice.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::core::mime::LEGACY_WORD_MIME_TYPE;
use crate::extraction::doc::extract_doc_text;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::{ExtractionResult, Metadata};
use ahash::AHashMap;
use async_trait::async_trait;
use std::borrow::Cow;

/// Native DOC extractor using OLE/CFB parsing.
///
/// This extractor handles Word 97-2003 binary (.doc) files without
/// requiring LibreOffice, providing ~50x faster extraction.
pub struct DocExtractor;

impl DocExtractor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DocExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for DocExtractor {
    fn name(&self) -> &str {
        "doc-extractor"
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
        "Native DOC text extraction via OLE/CFB parsing"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl DocumentExtractor for DocExtractor {
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        let result = {
            #[cfg(feature = "tokio-runtime")]
            if crate::core::batch_mode::is_batch_mode() {
                let content_owned = content.to_vec();
                let span = tracing::Span::current();
                tokio::task::spawn_blocking(move || -> crate::error::Result<_> {
                    let _guard = span.entered();
                    extract_doc_text(&content_owned)
                })
                .await
                .map_err(|e| crate::error::KreuzbergError::parsing(format!("DOC extraction task failed: {e}")))?
            } else {
                extract_doc_text(content)
            }

            #[cfg(not(feature = "tokio-runtime"))]
            extract_doc_text(content)
        }?;

        let mut metadata_map = AHashMap::new();

        if let Some(title) = result.metadata.title {
            metadata_map.insert(Cow::Borrowed("title"), serde_json::Value::String(title));
        }
        if let Some(author) = result.metadata.author {
            metadata_map.insert(
                Cow::Borrowed("authors"),
                serde_json::Value::Array(vec![serde_json::Value::String(author.clone())]),
            );
            metadata_map.insert(Cow::Borrowed("created_by"), serde_json::Value::String(author));
        }
        if let Some(subject) = result.metadata.subject {
            metadata_map.insert(Cow::Borrowed("subject"), serde_json::Value::String(subject));
        }
        if let Some(last_author) = result.metadata.last_author {
            metadata_map.insert(Cow::Borrowed("modified_by"), serde_json::Value::String(last_author));
        }
        if let Some(revision) = result.metadata.revision_number {
            metadata_map.insert(Cow::Borrowed("revision"), serde_json::Value::String(revision));
        }

        metadata_map.insert(
            Cow::Borrowed("extraction_method"),
            serde_json::Value::String("native_ole".to_string()),
        );

        Ok(ExtractionResult {
            content: result.text,
            mime_type: mime_type.to_string().into(),
            metadata: Metadata {
                additional: metadata_map,
                ..Default::default()
            },
            pages: None,
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: Some(vec![]),
            djot_content: None,
            elements: None,
            ocr_elements: None,
            document: None,
            #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
            extracted_keywords: None,
            quality_score: None,
            processing_warnings: Vec::new(),
            annotations: None,
        })
    }

    fn supported_mime_types(&self) -> &[&str] {
        &[LEGACY_WORD_MIME_TYPE]
    }

    fn priority(&self) -> i32 {
        60 // Higher than default (50) to take precedence
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_doc_extractor_plugin_interface() {
        let extractor = DocExtractor::new();
        assert_eq!(extractor.name(), "doc-extractor");
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert_eq!(extractor.priority(), 60);
        assert_eq!(extractor.supported_mime_types(), &["application/msword"]);
    }

    #[tokio::test]
    async fn test_doc_extractor_initialize_shutdown() {
        let extractor = DocExtractor::new();
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[tokio::test]
    async fn test_doc_extractor_real_file() {
        let test_file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test_documents/vendored/unstructured/doc/simple.doc");
        if !test_file.exists() {
            return;
        }
        let content = std::fs::read(&test_file).expect("Failed to read test DOC");
        let extractor = DocExtractor::new();
        let config = ExtractionConfig::default();
        let result = extractor
            .extract_bytes(&content, "application/msword", &config)
            .await
            .expect("DOC extraction failed");
        assert!(!result.content.is_empty(), "Should extract text from DOC");
        assert_eq!(&*result.mime_type, "application/msword");
    }
}
