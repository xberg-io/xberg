//! Apple Keynote (.key) extractor.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extractors::iwork::{dedup_text, extract_text_from_proto, read_iwa_file};
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::{ExtractionResult, Metadata};
use ahash::AHashMap;
use async_trait::async_trait;
use std::borrow::Cow;

/// Apple Keynote presentation extractor.
///
/// Supports `.key` files (modern iWork format, 2013+).
///
/// Extracts slide text and speaker notes from the IWA container:
/// ZIP → Snappy → protobuf text fields.
pub struct KeynoteExtractor;

impl KeynoteExtractor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for KeynoteExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for KeynoteExtractor {
    fn name(&self) -> &str {
        "iwork-keynote-extractor"
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
        "Apple Keynote (.key) text extraction via IWA container parser"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

/// Parse a Keynote ZIP and extract all text from IWA files.
///
/// Keynote stores its content across many IWA files:
/// - `Index/Presentation.iwa` — master slide structure and layout
/// - `Index/Slide_*.iwa` — individual slide content and speaker notes
/// - `Index/MasterSlide_*.iwa` — master slide text
fn parse_keynote(content: &[u8]) -> Result<String> {
    let iwa_paths = super::collect_iwa_paths(content)?;

    let mut all_texts: Vec<String> = Vec::new();

    // Prioritize slide IWA files for more structured output
    let slide_paths: Vec<&String> = iwa_paths
        .iter()
        .filter(|p| p.contains("Slide") || p.contains("Presentation"))
        .collect();

    let other_paths: Vec<&String> = iwa_paths
        .iter()
        .filter(|p| !p.contains("Slide") && !p.contains("Presentation"))
        .collect();

    for path in slide_paths.iter().chain(other_paths.iter()) {
        match read_iwa_file(content, path) {
            Ok(decompressed) => {
                let texts = extract_text_from_proto(&decompressed);
                all_texts.extend(texts);
            }
            Err(_) => {
                tracing::debug!("Skipping IWA file (decompression failed): {path}");
            }
        }
    }

    let deduplicated = dedup_text(all_texts);
    Ok(deduplicated.join("\n"))
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl DocumentExtractor for KeynoteExtractor {
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
                    parse_keynote(&content_owned)
                })
                .await
                .map_err(|e| crate::error::KreuzbergError::parsing(format!("Keynote extraction task failed: {e}")))??
            } else {
                parse_keynote(content)?
            }

            #[cfg(not(feature = "tokio-runtime"))]
            parse_keynote(content)?
        };

        let document = if config.include_document_structure {
            Some(build_keynote_document_structure(&text))
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
        &["application/x-iwork-keynote-sffkey"]
    }

    fn priority(&self) -> i32 {
        50
    }
}

/// Build a `DocumentStructure` from extracted Keynote text.
///
/// Maps text lines to slides with paragraphs. Each non-empty line group
/// separated by blank lines becomes a slide, with the first line as the
/// slide title.
fn build_keynote_document_structure(text: &str) -> crate::types::document_structure::DocumentStructure {
    use crate::types::builder::DocumentStructureBuilder;

    let mut builder = DocumentStructureBuilder::new().source_format("keynote");
    let mut slide_number: u32 = 0;

    // Split text into slide-like chunks (separated by blank lines)
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        // Skip blank lines
        if lines[i].trim().is_empty() {
            i += 1;
            continue;
        }

        slide_number += 1;
        let first_line = lines[i].trim();
        builder.push_slide(slide_number, Some(first_line));
        i += 1;

        // Collect subsequent non-blank lines as paragraphs in this slide
        while i < lines.len() && !lines[i].trim().is_empty() {
            builder.push_paragraph(lines[i].trim(), vec![], None, None);
            i += 1;
        }

        builder.exit_container();
    }

    // If no slides were created but there is content, push paragraphs
    if slide_number == 0 && !text.trim().is_empty() {
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
    fn test_keynote_extractor_plugin_interface() {
        let extractor = KeynoteExtractor::new();
        assert_eq!(extractor.name(), "iwork-keynote-extractor");
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[test]
    fn test_keynote_extractor_supported_mime_types() {
        let extractor = KeynoteExtractor::new();
        let types = extractor.supported_mime_types();
        assert!(types.contains(&"application/x-iwork-keynote-sffkey"));
    }
}
