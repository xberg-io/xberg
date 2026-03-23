//! Apple Numbers (.numbers) extractor.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extractors::iwork::{dedup_text, extract_text_from_proto, read_iwa_file};
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::{ExtractionResult, Metadata};
use ahash::AHashMap;
use async_trait::async_trait;
use std::borrow::Cow;

/// Apple Numbers spreadsheet extractor.
///
/// Supports `.numbers` files (modern iWork format, 2013+).
///
/// Extracts cell string values and sheet names from the IWA container:
/// ZIP → Snappy → protobuf text fields. Output is formatted as plain text
/// with one text token per line (representing cell values and labels).
pub struct NumbersExtractor;

impl NumbersExtractor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NumbersExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for NumbersExtractor {
    fn name(&self) -> &str {
        "iwork-numbers-extractor"
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
        "Apple Numbers (.numbers) text extraction via IWA container parser"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

/// Parse a Numbers ZIP and extract all text from IWA files.
///
/// Numbers stores its content across many IWA files:
/// - `Index/CalculationEngine.iwa` — formula cells and sheet data
/// - `Index/Document.iwa` — document structure and sheet names
/// - `tables/DataStore.iwa` — table cell string values
fn parse_numbers(content: &[u8]) -> Result<String> {
    let iwa_paths = super::collect_iwa_paths(content)?;

    let mut all_texts: Vec<String> = Vec::new();

    for path in &iwa_paths {
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

    // Filter out very short noise tokens (common in spreadsheet binary data)
    let filtered: Vec<String> = deduplicated
        .into_iter()
        .filter(|s| s.len() >= 2 && s.chars().any(|c| c.is_alphanumeric()))
        .collect();

    Ok(filtered.join("\n"))
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl DocumentExtractor for NumbersExtractor {
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
                    parse_numbers(&content_owned)
                })
                .await
                .map_err(|e| crate::error::KreuzbergError::parsing(format!("Numbers extraction task failed: {e}")))??
            } else {
                parse_numbers(content)?
            }

            #[cfg(not(feature = "tokio-runtime"))]
            parse_numbers(content)?
        };

        let document = if config.include_document_structure {
            Some(build_numbers_document_structure(&text))
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
        &["application/x-iwork-numbers-sffnumbers"]
    }

    fn priority(&self) -> i32 {
        50
    }
}

/// Build a `DocumentStructure` from extracted Numbers text.
///
/// Since Numbers extracts flat cell text values, we create a heading
/// for "Sheet Data" and push each line as a paragraph.
fn build_numbers_document_structure(text: &str) -> crate::types::document_structure::DocumentStructure {
    use crate::types::builder::DocumentStructureBuilder;

    let mut builder = DocumentStructureBuilder::new().source_format("numbers");

    if !text.trim().is_empty() {
        builder.push_heading(1, "Sheet Data", None, None);
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
    fn test_numbers_extractor_plugin_interface() {
        let extractor = NumbersExtractor::new();
        assert_eq!(extractor.name(), "iwork-numbers-extractor");
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[test]
    fn test_numbers_extractor_supported_mime_types() {
        let extractor = NumbersExtractor::new();
        let types = extractor.supported_mime_types();
        assert!(types.contains(&"application/x-iwork-numbers-sffnumbers"));
    }
}
