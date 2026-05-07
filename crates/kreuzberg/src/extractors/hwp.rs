//! Hangul Word Processor (.hwp) extractor.
//!
//! Extracts text content from HWP 5.0 documents using the vendored HWP parser
//! in `crate::extraction::hwp`.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::internal::InternalDocument;
use crate::types::internal_builder::InternalDocumentBuilder;
use async_trait::async_trait;

/// Extractor for Hangul Word Processor (.hwp) files.
///
/// Supports HWP 5.0 format, the standard document format in South Korea.
pub struct HwpExtractor;

impl HwpExtractor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HwpExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for HwpExtractor {
    fn name(&self) -> &str {
        "hwp-extractor"
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
        "Hangul Word Processor (.hwp) text extraction"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

fn extract_hwp_content(content: &[u8]) -> Result<String> {
    crate::extraction::hwp::extract_hwp_text(content)
        .map_err(|e| crate::KreuzbergError::parsing(format!("Failed to read HWP file: {e}")))
}

/// Build an `InternalDocument` from HWP extracted text.
///
/// Splits on double-newlines into paragraphs.
fn build_hwp_internal_document(text: &str) -> InternalDocument {
    let mut builder = InternalDocumentBuilder::new("hwp");
    for paragraph in text.split("\n\n") {
        let trimmed = paragraph.trim();
        if !trimmed.is_empty() {
            builder.push_paragraph(trimmed, vec![], None, None);
        }
    }
    builder.build()
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl DocumentExtractor for HwpExtractor {
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        let text = extract_hwp_content(content)?;
        let mut doc = build_hwp_internal_document(&text);
        doc.mime_type = std::borrow::Cow::Owned(mime_type.to_string());
        Ok(doc)
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/x-hwp"]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hwp_extractor_plugin_interface() {
        let extractor = HwpExtractor::new();
        assert_eq!(extractor.name(), "hwp-extractor");
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert_eq!(extractor.priority(), 50);
        assert_eq!(extractor.supported_mime_types(), &["application/x-hwp"]);
    }

    #[test]
    fn test_hwp_extractor_initialize_shutdown() {
        let extractor = HwpExtractor::new();
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[test]
    fn test_hwpx_mime_not_routed_to_hwp_extractor() {
        use crate::KreuzbergError;
        use crate::plugins::registry::DocumentExtractorRegistry;
        use std::sync::Arc;

        let mut registry = DocumentExtractorRegistry::new();
        registry.register(Arc::new(HwpExtractor::new())).unwrap();

        let result = registry.get("application/haansofthwpx");
        assert!(
            matches!(result, Err(KreuzbergError::UnsupportedFormat(_))),
            "application/haansofthwpx must not be routed to HwpExtractor"
        );
    }
}
