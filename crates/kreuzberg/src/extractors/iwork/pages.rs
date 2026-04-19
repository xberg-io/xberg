//! Apple Pages (.pages) extractor.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extractors::iwork::{dedup_text, extract_metadata_from_zip, extract_text_from_proto, read_iwa_file};
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::internal::InternalDocument;
use crate::types::internal_builder::InternalDocumentBuilder;
use async_trait::async_trait;

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

/// Parsed Pages data: document text blocks and metadata.
struct PagesData {
    /// Text blocks from the main document IWA files (prioritized).
    document_texts: Vec<String>,
    /// Additional text from annotation/data-record IWA files.
    supplementary_texts: Vec<String>,
    /// Metadata extracted from the ZIP archive.
    metadata: crate::types::metadata::Metadata,
}

/// Parse a Pages ZIP and extract all text from IWA files.
///
/// Pages stores its content in:
/// - `Index/Document.iwa` — main document text
/// - `Index/AnnotationAuthorStorage.iwa` — comments/annotations
/// - Any `DataRecords/*.iwa` — embedded data blocks
///
/// We prioritize Document IWA files for the main body and separate
/// annotation/data content.
fn parse_pages(content: &[u8]) -> Result<PagesData> {
    let iwa_paths = super::collect_iwa_paths(content)?;
    let metadata = extract_metadata_from_zip(content);

    // Separate document-content IWA files from annotations and data records
    let mut doc_paths: Vec<&String> = Vec::new();
    let mut other_paths: Vec<&String> = Vec::new();

    for path in &iwa_paths {
        let filename = path.rsplit('/').next().unwrap_or(path);
        if filename.starts_with("Document") || filename.starts_with("Section") || filename.starts_with("Text") {
            doc_paths.push(path);
        } else {
            other_paths.push(path);
        }
    }

    // If no document-specific paths were found, treat all paths as document content
    if doc_paths.is_empty() {
        doc_paths = iwa_paths.iter().collect();
        other_paths.clear();
    }

    let mut doc_texts: Vec<String> = Vec::new();
    for path in &doc_paths {
        match read_iwa_file(content, path) {
            Ok(decompressed) => {
                let texts = extract_text_from_proto(&decompressed);
                doc_texts.extend(texts);
            }
            Err(_) => {
                tracing::debug!("Skipping IWA file (decompression failed): {path}");
            }
        }
    }

    let mut other_texts_raw: Vec<String> = Vec::new();
    for path in &other_paths {
        match read_iwa_file(content, path) {
            Ok(decompressed) => {
                let texts = extract_text_from_proto(&decompressed);
                other_texts_raw.extend(texts);
            }
            Err(_) => {
                tracing::debug!("Skipping IWA file (decompression failed): {path}");
            }
        }
    }

    let document_texts = dedup_text(doc_texts);
    let supplementary_texts: Vec<String> = dedup_text(other_texts_raw)
        .into_iter()
        .filter(|t| !document_texts.contains(t))
        .collect();

    Ok(PagesData {
        document_texts,
        supplementary_texts,
        metadata,
    })
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl DocumentExtractor for PagesExtractor {
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

        let mut doc = build_pages_internal_document(&data);
        doc.mime_type = std::borrow::Cow::Owned(mime_type.to_string());
        Ok(doc)
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/x-iwork-pages-sffpages"]
    }

    fn priority(&self) -> i32 {
        50
    }
}

/// Build an `InternalDocument` from parsed Pages data.
///
/// Applies heading detection heuristics: short lines (under 80 chars) that
/// appear before longer text blocks are treated as headings. Metadata from the
/// ZIP archive is applied to the document.
fn build_pages_internal_document(data: &PagesData) -> InternalDocument {
    let mut builder = InternalDocumentBuilder::new("pages");

    // Apply metadata
    if data.metadata.title.is_some() || data.metadata.authors.is_some() {
        builder.set_metadata(data.metadata.clone());
    }

    // Emit the first text block as the document title if it looks like one
    // (short, no sentence-ending punctuation, appears before body text).
    let texts = &data.document_texts;
    let mut start_idx = 0;
    if let Some(first) = texts.first() {
        let trimmed = first.trim();
        if !trimmed.is_empty() && is_likely_title(trimmed) && texts.len() > 1 {
            builder.push_title(trimmed, None, None);
            start_idx = 1;
        }
    }

    // Emit remaining document text with heading detection
    for text in &texts[start_idx..] {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            continue;
        }

        if is_likely_heading(trimmed) {
            builder.push_heading(2, trimmed, None, None);
        } else {
            builder.push_paragraph(trimmed, vec![], None, None);
        }
    }

    // Emit supplementary text (annotations, data records) under a separate section
    if !data.supplementary_texts.is_empty() {
        let has_body = !data.document_texts.is_empty();
        if has_body {
            builder.push_heading(2, "Annotations", None, None);
        }
        for text in &data.supplementary_texts {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                builder.push_paragraph(trimmed, vec![], None, None);
            }
        }
    }

    builder.build()
}

/// Heuristic: a string looks like a document title if it is short, does not
/// end with sentence-terminating punctuation, and contains at least one
/// alphabetic character.
fn is_likely_title(s: &str) -> bool {
    s.len() <= 100
        && !s.ends_with('.')
        && !s.ends_with('!')
        && !s.ends_with('?')
        && s.chars().any(|c| c.is_alphabetic())
        && !s.contains('\n')
}

/// Heuristic: a string looks like a heading if it is relatively short, does
/// not end with sentence-terminating punctuation, and starts with an uppercase
/// letter or a digit.
fn is_likely_heading(s: &str) -> bool {
    s.len() <= 80
        && !s.ends_with('.')
        && !s.ends_with(',')
        && !s.contains('\n')
        && s.chars().next().is_some_and(|c| c.is_uppercase() || c.is_ascii_digit())
        && s.split_whitespace().count() <= 10
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
