//! Plain text extractor.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::internal::InternalDocument;
use crate::types::internal_builder::InternalDocumentBuilder;
use crate::types::metadata::Metadata;
use async_trait::async_trait;

/// Plain text extractor.
///
/// Extracts content from plain text files (.txt).
pub struct PlainTextExtractor;

impl PlainTextExtractor {
    /// Create a new plain text extractor.
    pub(crate) fn new() -> Self {
        Self
    }
}

impl Default for PlainTextExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl PlainTextExtractor {
    /// Build an `InternalDocument` from plain text content.
    ///
    /// Splits on double-newlines into paragraphs.
    fn build_internal_document(text: &str) -> InternalDocument {
        let mut builder = InternalDocumentBuilder::new("text");
        for paragraph in text.split("\n\n") {
            let trimmed = paragraph.trim();
            if !trimmed.is_empty() {
                builder.push_paragraph(trimmed, vec![], None, None);
            }
        }
        builder.build()
    }
}

impl Plugin for PlainTextExtractor {
    fn name(&self) -> &str {
        "plain-text-extractor"
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
        "Extracts content from plain text files"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl DocumentExtractor for PlainTextExtractor {
    #[cfg_attr(feature = "otel", tracing::instrument(
        skip(self, content, _config),
        fields(
            extractor.name = self.name(),
            content.size_bytes = content.len(),
        )
    ))]
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        let text = String::from_utf8_lossy(content).into_owned();
        let text = text.trim_end_matches('\n').trim_end_matches('\r').to_string();
        let line_count = text.lines().count();
        let word_count = text.split_whitespace().count();
        let character_count = text.len();

        let mut doc = Self::build_internal_document(&text);

        doc.metadata = Metadata {
            format: Some(crate::types::FormatMetadata::Text(crate::types::TextMetadata {
                line_count,
                word_count,
                character_count,
                headers: None,
                links: None,
                code_blocks: None,
            })),
            ..Default::default()
        };
        doc.mime_type = mime_type.to_string();

        Ok(doc)
    }

    fn supported_mime_types(&self) -> &[&str] {
        &[
            "text/plain",
            "text/troff",
            "text/x-mdoc",
            "text/x-pod",
            "text/x-dokuwiki",
        ]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_plain_text_extractor() {
        let extractor = PlainTextExtractor::new();
        let content = b"Hello, World!\nThis is a test.";
        let config = ExtractionConfig::default();

        let result = extractor.extract_bytes(content, "text/plain", &config).await.unwrap();

        assert!(result.metadata.format.is_some());
        let text_meta = match result.metadata.format.as_ref().unwrap() {
            crate::types::FormatMetadata::Text(meta) => meta,
            _ => panic!("Expected Text metadata"),
        };
        assert_eq!(text_meta.line_count, 2);
        assert_eq!(text_meta.word_count, 6);
    }

    #[test]
    fn test_plain_text_plugin_interface() {
        let extractor = PlainTextExtractor::new();
        assert_eq!(extractor.name(), "plain-text-extractor");
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert_eq!(
            extractor.supported_mime_types(),
            &[
                "text/plain",
                "text/troff",
                "text/x-mdoc",
                "text/x-pod",
                "text/x-dokuwiki",
            ]
        );
        assert_eq!(extractor.priority(), 50);
    }
}
