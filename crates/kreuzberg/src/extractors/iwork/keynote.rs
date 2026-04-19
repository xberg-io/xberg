//! Apple Keynote (.key) extractor.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extractors::iwork::{dedup_text, extract_metadata_from_zip, extract_text_from_proto, read_iwa_file};
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::internal::InternalDocument;
use crate::types::internal_builder::InternalDocumentBuilder;
use async_trait::async_trait;

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

/// Parsed Keynote data: per-slide text and metadata.
struct KeynoteData {
    /// Text extracted from individual slide IWA files, in path-sorted order.
    slide_texts: Vec<Vec<String>>,
    /// Additional text from non-slide IWA files (notes, master slides, etc.).
    other_texts: Vec<String>,
    /// Metadata extracted from the ZIP archive.
    metadata: crate::types::metadata::Metadata,
}

/// Parse a Keynote ZIP and extract all text from IWA files.
///
/// Keynote stores its content across many IWA files:
/// - `Index/Presentation.iwa` — master slide structure and layout
/// - `Index/Slide_*.iwa` — individual slide content and speaker notes
/// - `Index/MasterSlide_*.iwa` — master slide text
///
/// We separate slide-specific IWA files from other files to produce
/// per-slide structured output.
fn parse_keynote(content: &[u8]) -> Result<KeynoteData> {
    let iwa_paths = super::collect_iwa_paths(content)?;
    let metadata = extract_metadata_from_zip(content);

    // Separate individual slide paths from master slides and other files.
    // Slide paths typically look like `Index/Slide-NNNNN.iwa`.
    let mut slide_paths: Vec<&String> = iwa_paths
        .iter()
        .filter(|p| {
            // Match `Slide-` or `Slide_` but not `MasterSlide`
            let filename = p.rsplit('/').next().unwrap_or(p);
            filename.starts_with("Slide") && !filename.starts_with("MasterSlide")
        })
        .collect();

    // Sort slide paths so slides are in order
    slide_paths.sort();

    let other_paths: Vec<&String> = iwa_paths
        .iter()
        .filter(|p| {
            let filename = p.rsplit('/').next().unwrap_or(p);
            !filename.starts_with("Slide") || filename.starts_with("MasterSlide")
        })
        .collect();

    // Extract text per slide (each slide IWA becomes a separate group)
    let mut slide_texts: Vec<Vec<String>> = Vec::new();
    let mut seen_global = std::collections::HashSet::new();

    for path in &slide_paths {
        match read_iwa_file(content, path) {
            Ok(decompressed) => {
                let texts = extract_text_from_proto(&decompressed);
                let unique: Vec<String> = texts.into_iter().filter(|t| seen_global.insert(t.clone())).collect();
                if !unique.is_empty() {
                    slide_texts.push(unique);
                }
            }
            Err(_) => {
                tracing::debug!("Skipping IWA file (decompression failed): {path}");
            }
        }
    }

    // Extract remaining text from non-slide files
    let mut other_raw: Vec<String> = Vec::new();
    for path in &other_paths {
        match read_iwa_file(content, path) {
            Ok(decompressed) => {
                let texts = extract_text_from_proto(&decompressed);
                other_raw.extend(texts);
            }
            Err(_) => {
                tracing::debug!("Skipping IWA file (decompression failed): {path}");
            }
        }
    }

    let other_texts: Vec<String> = dedup_text(other_raw)
        .into_iter()
        .filter(|t| seen_global.insert(t.clone()))
        .collect();

    Ok(KeynoteData {
        slide_texts,
        other_texts,
        metadata,
    })
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl DocumentExtractor for KeynoteExtractor {
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        let data = {
            #[cfg(feature = "tokio-runtime")]
            if crate::core::batch_mode::is_batch_mode() {
                if config.cancel_token.as_ref().map(|t| t.is_cancelled()).unwrap_or(false) {
                    return Err(crate::error::KreuzbergError::Cancelled);
                }
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

        let mut doc = build_keynote_internal_document(&data);
        doc.mime_type = std::borrow::Cow::Owned(mime_type.to_string());
        Ok(doc)
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/x-iwork-keynote-sffkey"]
    }

    fn priority(&self) -> i32 {
        50
    }
}

/// Build an `InternalDocument` from parsed Keynote data.
///
/// Creates a slide element for each detected slide group, using the first text
/// line as the slide title. Additional lines become paragraphs within the slide.
/// Metadata from the ZIP archive is applied to the document.
fn build_keynote_internal_document(data: &KeynoteData) -> InternalDocument {
    let mut builder = InternalDocumentBuilder::new("keynote");

    // Apply metadata
    if data.metadata.title.is_some() || data.metadata.authors.is_some() {
        builder.set_metadata(data.metadata.clone());
    }

    // Emit per-slide elements
    for (idx, slide_lines) in data.slide_texts.iter().enumerate() {
        let slide_number = (idx + 1) as u32;

        if slide_lines.is_empty() {
            continue;
        }

        // Use the first line as the slide title
        let title = slide_lines[0].trim();
        builder.push_slide(slide_number, Some(title), None);

        // Remaining lines become body paragraphs
        for line in &slide_lines[1..] {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                builder.push_paragraph(trimmed, vec![], None, None);
            }
        }
    }

    // Emit any additional text that didn't belong to a specific slide
    // (master slide text, presentation-level notes, etc.)
    if !data.other_texts.is_empty() {
        let has_slides = !data.slide_texts.is_empty();
        if has_slides {
            builder.push_heading(2, "Additional Content", None, None);
        }
        for text in &data.other_texts {
            let trimmed = text.trim();
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
