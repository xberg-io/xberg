//! Apple Pages (.pages) extractor.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extractors::iwork::{dedup_text, extract_text_from_proto, read_iwa_file};
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::{ExtractionResult, Metadata};
use ahash::AHashMap;
use async_trait::async_trait;
use std::borrow::Cow;

/// Apple Pages document extractor.
///
/// Supports `.pages` files (modern iWork format, 2013+).
///
/// Extracts all text content from the document by parsing the IWA
/// (iWork Archive) container: ZIP → Snappy → protobuf text fields.
pub struct PagesExtractor;

impl PagesExtractor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PagesExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for PagesExtractor {
    fn name(&self) -> &str {
        "iwork-pages-extractor"
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
        "Apple Pages (.pages) text extraction via IWA container parser"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

/// Parse a Pages ZIP and extract all text from IWA files.
///
/// Pages stores its content in:
/// - `Index/Document.iwa` — main document text
/// - `Index/AnnotationAuthorStorage.iwa` — comments/annotations
/// - Any `DataRecords/*.iwa` — embedded data blocks
fn parse_pages(content: &[u8]) -> Result<String> {
    // Collect all IWA paths inside the archive
    let iwa_paths = super::collect_iwa_paths(content)?;

    let mut all_texts: Vec<String> = Vec::new();

    // Attempt to read each IWA file and extract its text
    for path in &iwa_paths {
        match read_iwa_file(content, path) {
            Ok(decompressed) => {
                let texts = extract_text_from_proto(&decompressed);
                all_texts.extend(texts);
            }
            Err(_) => {
                // Some IWA files may fail decompression (e.g., newer Snappy variants)
                // Skip gracefully to produce partial results rather than hard failure
                tracing::debug!("Skipping IWA file (decompression failed): {path}");
            }
        }
    }

    let deduplicated = dedup_text(all_texts);
    Ok(deduplicated.join("\n"))
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl DocumentExtractor for PagesExtractor {
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        let text = {
            #[cfg(feature = "tokio-runtime")]
            if crate::core::batch_mode::is_batch_mode() {
                let content_owned = content.to_vec();
                let span = tracing::Span::current();
                tokio::task::spawn_blocking(move || {
                    let _guard = span.entered();
                    parse_pages(&content_owned)
                })
                .await
                .map_err(|e| crate::error::KreuzbergError::parsing(format!("Pages extraction task failed: {e}")))??
            } else {
                parse_pages(content)?
            }

            #[cfg(not(feature = "tokio-runtime"))]
            parse_pages(content)?
        };

        let document = if config.include_document_structure {
            Some(build_pages_document_structure(&text))
        } else {
            None
        };

        let additional: AHashMap<Cow<'static, str>, serde_json::Value> = AHashMap::new();

        Ok(ExtractionResult {
            content: text,
            mime_type: mime_type.to_string().into(),
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
            document,
            #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
            extracted_keywords: None,
            quality_score: None,
            processing_warnings: Vec::new(),
            annotations: None,
        })
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/x-iwork-pages-sffpages"]
    }

    fn priority(&self) -> i32 {
        50
    }
}

/// Build a `DocumentStructure` from extracted Pages text.
///
/// Maps text content to paragraphs. If the text contains blank-line separators
/// (`\n\n`), each block becomes a paragraph. Otherwise, each non-empty line
/// becomes its own paragraph.
fn build_pages_document_structure(text: &str) -> crate::types::document_structure::DocumentStructure {
    use crate::types::builder::DocumentStructureBuilder;

    let mut builder = DocumentStructureBuilder::new().source_format("pages");

    if text.contains("\n\n") {
        // Multi-paragraph content separated by blank lines
        for paragraph in text.split("\n\n") {
            let trimmed = paragraph.trim();
            if !trimmed.is_empty() {
                builder.push_paragraph(trimmed, vec![], None, None);
            }
        }
    } else {
        // Single-spaced content: each line becomes a paragraph
        for line in text.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                builder.push_paragraph(trimmed, vec![], None, None);
            }
        }
    }

    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pages_extractor_plugin_interface() {
        let extractor = PagesExtractor::new();
        assert_eq!(extractor.name(), "iwork-pages-extractor");
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[test]
    fn test_pages_extractor_supported_mime_types() {
        let extractor = PagesExtractor::new();
        let types = extractor.supported_mime_types();
        assert!(types.contains(&"application/x-iwork-pages-sffpages"));
    }
}
