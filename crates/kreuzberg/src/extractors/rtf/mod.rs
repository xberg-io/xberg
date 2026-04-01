//! RTF (Rich Text Format) extractor.
//!
//! Supports: Rich Text Format (.rtf)
//!
//! This native Rust extractor provides text extraction from RTF documents with:
//! - Character encoding support (Windows-1252 for 0x80-0x9F range)
//! - Common RTF control words (paragraph breaks, tabs, bullets, quotes, dashes)
//! - Unicode escape sequences
//! - Image metadata extraction
//! - Whitespace normalization

mod encoding;
mod formatting;
mod images;
mod metadata;
mod parser;
mod tables;

// Re-export public functions for backward compatibility
pub use encoding::{hex_digit_to_u8, parse_hex_byte, parse_rtf_control_word};
pub use formatting::normalize_whitespace;
pub use images::{extract_image_metadata, extract_pict_image};
pub use metadata::{extract_rtf_metadata, parse_rtf_datetime};
pub use parser::{ParagraphMeta, extract_rtf_formatting, extract_text_from_rtf, spans_to_annotations};

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::ExtractedImage;
use crate::types::internal::InternalDocument;
use crate::types::internal_builder::InternalDocumentBuilder;
use crate::types::metadata::Metadata;
use crate::types::uri::Uri;
use async_trait::async_trait;
use bytes::Bytes;
use std::borrow::Cow;

/// Native Rust RTF extractor.
///
/// Extracts text content, metadata, and structure from RTF documents
pub struct RtfExtractor;

impl RtfExtractor {
    /// Create a new RTF extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Default for RtfExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl RtfExtractor {
    /// Build an `InternalDocument` from RTF content.
    ///
    /// Extracts paragraphs (split on double-newlines), tables, formatting
    /// annotations, and header/footer content with appropriate content layers.
    fn build_internal_document(rtf_content: &str, plain: bool) -> InternalDocument {
        use crate::types::document_structure::ContentLayer;

        let (extracted_text, tables, rtf_images, para_metas) = extract_text_from_rtf(rtf_content, plain);
        let formatting = extract_rtf_formatting(rtf_content);

        let mut builder = InternalDocumentBuilder::new("rtf");

        // Extract URIs from hyperlinks found during RTF parsing
        for (_start, _end, url) in &formatting.hyperlinks {
            if !url.is_empty() {
                // Try to find link text from the extracted text
                let label = extracted_text.get(*_start..*_end).map(|s| s.to_string());
                builder.push_uri(Uri::hyperlink(url, label));
            }
        }

        let mut table_idx = 0;
        let mut meta_idx = 0;
        let mut byte_offset = 0usize;
        let mut in_table_rows = false; // tracking consecutive is_table paragraphs

        // Track list state for grouping list items
        let mut in_list = false;
        let mut list_id: Option<u16> = None;
        let mut list_depth: u8 = 0;

        for paragraph in extracted_text.split("\n\n") {
            let trimmed = paragraph.trim();
            if trimmed.is_empty() {
                byte_offset += paragraph.len() + 2;
                meta_idx += 1;
                continue;
            }

            let meta = para_metas.get(meta_idx).cloned().unwrap_or_default();
            meta_idx += 1;

            // Check if this paragraph is a table row
            if meta.is_table {
                // Close any open list (all nested levels)
                if in_list {
                    for _ in 0..=list_depth {
                        builder.end_list();
                    }
                    in_list = false;
                    list_id = None;
                    list_depth = 0;
                }
                in_table_rows = true;
                // Table rows are handled as a block — skip the text, use structured table data
                byte_offset += paragraph.len() + 2;
                continue;
            }

            // If we were in table rows and now we're not, push the table
            if in_table_rows {
                in_table_rows = false;
                if table_idx < tables.len() {
                    builder.push_table_from_cells(&tables[table_idx].cells, None, None);
                    table_idx += 1;
                }
            }

            // Check if this paragraph looks like a table (fallback for pipe-delimited text)
            let lines: Vec<&str> = trimmed.lines().collect();
            let is_table_like = lines.len() >= 2 && lines.iter().all(|l| l.contains('|'));
            if is_table_like && table_idx < tables.len() {
                // Close any open list (all nested levels)
                if in_list {
                    for _ in 0..=list_depth {
                        builder.end_list();
                    }
                    in_list = false;
                    list_id = None;
                    list_depth = 0;
                }
                builder.push_table_from_cells(&tables[table_idx].cells, None, None);
                table_idx += 1;
                byte_offset += paragraph.len() + 2;
                continue;
            }

            let trim_offset = byte_offset + (paragraph.len() - paragraph.trim_start().len());
            let annotations = spans_to_annotations(trim_offset, trim_offset + trimmed.len(), &formatting);

            // Handle headings
            if meta.heading_level > 0 && meta.heading_level <= 6 {
                // Close any open list (all nested levels)
                if in_list {
                    for _ in 0..=list_depth {
                        builder.end_list();
                    }
                    in_list = false;
                    list_id = None;
                    list_depth = 0;
                }
                builder.push_heading(meta.heading_level, trimmed, None, None);
            }
            // Handle list items
            else if let Some(level) = meta.list_level {
                let new_list_id = meta.list_id;
                let ordered = meta.ordered;

                // Check if we need to start a new list or adjust nesting
                if !in_list || list_id != new_list_id {
                    if in_list {
                        // Close all nested levels plus the root list
                        for _ in 0..=list_depth {
                            builder.end_list();
                        }
                    }
                    builder.push_list(ordered);
                    // If starting at a level > 0, nest immediately
                    for _ in 0..level {
                        builder.push_list(ordered);
                    }
                    in_list = true;
                    list_id = new_list_id;
                    list_depth = level;
                } else if level > list_depth {
                    // Nest deeper
                    for _ in list_depth..level {
                        builder.push_list(ordered);
                    }
                    list_depth = level;
                } else if level < list_depth {
                    // Un-nest
                    for _ in level..list_depth {
                        builder.end_list();
                    }
                    list_depth = level;
                }

                builder.push_list_item(trimmed, ordered, annotations, None, None);
            }
            // Regular paragraph
            else {
                // Close any open list (all nested levels)
                if in_list {
                    for _ in 0..=list_depth {
                        builder.end_list();
                    }
                    in_list = false;
                    list_id = None;
                    list_depth = 0;
                }
                builder.push_paragraph(trimmed, annotations, None, None);
            }
            byte_offset += paragraph.len() + 2;
        }

        // Close any open list (all nested levels)
        if in_list {
            for _ in 0..=list_depth {
                builder.end_list();
            }
        }

        // If we ended while in table rows, push the last table
        if in_table_rows && table_idx < tables.len() {
            builder.push_table_from_cells(&tables[table_idx].cells, None, None);
            table_idx += 1;
        }

        // Push tables that weren't matched to text paragraphs
        while table_idx < tables.len() {
            builder.push_table_from_cells(&tables[table_idx].cells, None, None);
            table_idx += 1;
        }

        // Push extracted images
        for (i, rtf_img) in rtf_images.into_iter().enumerate() {
            let image = ExtractedImage {
                data: Bytes::from(rtf_img.data),
                format: Cow::Borrowed(rtf_img.format),
                image_index: i,
                page_number: None,
                width: None,
                height: None,
                colorspace: None,
                bits_per_component: None,
                is_mask: false,
                description: Some("image".to_string()),
                ocr_result: None,
                bounding_box: None,
                source_path: None,
            };
            builder.push_image(None, image, None, None);
        }

        // Push header/footer with content layers
        if let Some(ref header) = formatting.header_text {
            let idx = builder.push_paragraph(header, vec![], None, None);
            builder.set_layer(idx, ContentLayer::Header);
        }
        if let Some(ref footer) = formatting.footer_text {
            let idx = builder.push_paragraph(footer, vec![], None, None);
            builder.set_layer(idx, ContentLayer::Footer);
        }

        builder.build()
    }
}

impl Plugin for RtfExtractor {
    fn name(&self) -> &str {
        "rtf-extractor"
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
        "Extracts content from RTF (Rich Text Format) files with native Rust parsing"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl DocumentExtractor for RtfExtractor {
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
        tracing::debug!(format = "rtf", size_bytes = content.len(), "extraction starting");
        let rtf_content = String::from_utf8_lossy(content);
        let plain = true; // InternalDocument doesn't need markdown formatting

        // extract_rtf_metadata needs the extracted text; get it from the same pass
        let (extracted_text, _tables, _images, _metas) = extract_text_from_rtf(&rtf_content, plain);
        let mut metadata_map = extract_rtf_metadata(&rtf_content, &extracted_text);

        // Map standard fields from metadata_map to typed Metadata fields
        let title = metadata_map
            .remove(&Cow::Borrowed("title"))
            .and_then(|v| v.as_str().map(|s| s.to_string()));
        let subject = metadata_map
            .remove(&Cow::Borrowed("subject"))
            .and_then(|v| v.as_str().map(|s| s.to_string()));
        let authors = metadata_map.remove(&Cow::Borrowed("authors")).and_then(|v| {
            v.as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        });
        let created_by = metadata_map
            .remove(&Cow::Borrowed("created_by"))
            .and_then(|v| v.as_str().map(|s| s.to_string()));
        let modified_by = metadata_map
            .remove(&Cow::Borrowed("modified_by"))
            .and_then(|v| v.as_str().map(|s| s.to_string()));
        let created_at = metadata_map
            .remove(&Cow::Borrowed("created_at"))
            .and_then(|v| v.as_str().map(|s| s.to_string()));
        let modified_at = metadata_map
            .remove(&Cow::Borrowed("modified_at"))
            .and_then(|v| v.as_str().map(|s| s.to_string()));

        let mut doc = Self::build_internal_document(&rtf_content, plain);
        doc.mime_type = Cow::Owned(mime_type.to_string());
        doc.metadata = Metadata {
            title,
            subject,
            authors,
            created_by,
            modified_by,
            created_at,
            modified_at,
            additional: metadata_map,
            ..Default::default()
        };

        tracing::debug!(
            element_count = doc.elements.len(),
            format = "rtf",
            "extraction complete"
        );
        Ok(doc)
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/rtf", "text/rtf"]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rtf_extractor_plugin_interface() {
        let extractor = RtfExtractor::new();
        assert_eq!(extractor.name(), "rtf-extractor");
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert!(extractor.supported_mime_types().contains(&"application/rtf"));
        assert_eq!(extractor.priority(), 50);
    }

    #[test]
    fn test_simple_rtf_extraction() {
        let _extractor = RtfExtractor;
        let rtf_content = r#"{\rtf1 Hello World}"#;
        let (extracted, _, _, _) = extract_text_from_rtf(rtf_content, false);
        assert!(extracted.contains("Hello") || extracted.contains("World"));
    }

    #[test]
    fn test_plain_text_no_image_markdown() {
        let rtf_content = r#"{\rtf1 Before image {\pict\jpegblip\picw100\pich100 ffd8ffe0} After image}"#;
        let (plain, _, _, _) = extract_text_from_rtf(rtf_content, true);
        assert!(!plain.contains("!["), "Plain text should not contain image markdown");
        assert!(
            !plain.contains("]("),
            "Plain text should not contain image markdown links"
        );

        let (md, _, _, _) = extract_text_from_rtf(rtf_content, false);
        assert!(md.contains("![image]"), "Markdown output should contain image markers");
    }

    #[test]
    fn test_plain_text_table_uses_pipes() {
        let rtf_content = r#"{\rtf1 {\trowd Cell1\cell Cell2\cell\row}}"#;
        let (plain, tables, _, _) = extract_text_from_rtf(rtf_content, true);
        // Table rows produce [TABLE_ROW] placeholders in plain text
        assert!(
            plain.contains("[TABLE_ROW]"),
            "Plain text should contain table row placeholder, got: {}",
            plain
        );
        assert!(!tables.is_empty(), "Tables should still be extracted");
    }

    #[test]
    fn test_rtf_bold_formatting_extraction() {
        let rtf_content = r#"{\rtf1 Normal {\b Bold text} more normal}"#;
        let formatting = extract_rtf_formatting(rtf_content);
        // Should have at least one bold span
        let has_bold = formatting.spans.iter().any(|s| s.bold);
        assert!(has_bold, "Should detect bold formatting in RTF");
    }

    #[test]
    fn test_rtf_italic_formatting_extraction() {
        let rtf_content = r#"{\rtf1 Normal {\i Italic text} more normal}"#;
        let formatting = extract_rtf_formatting(rtf_content);
        let has_italic = formatting.spans.iter().any(|s| s.italic);
        assert!(has_italic, "Should detect italic formatting in RTF");
    }

    #[test]
    fn test_rtf_underline_formatting_extraction() {
        let rtf_content = r#"{\rtf1 Normal {\ul Underlined text} more normal}"#;
        let formatting = extract_rtf_formatting(rtf_content);
        let has_underline = formatting.spans.iter().any(|s| s.underline);
        assert!(has_underline, "Should detect underline formatting in RTF");
    }

    #[test]
    fn test_rtf_color_table_extraction() {
        let rtf_content = r#"{\rtf1{\colortbl;\red255\green0\blue0;\red0\green255\blue0;}Normal {\cf1 Red text} more}"#;
        let formatting = extract_rtf_formatting(rtf_content);
        assert!(
            formatting.color_table.len() >= 2,
            "Should parse at least 2 color entries"
        );
        assert_eq!(formatting.color_table[1], "#ff0000", "First color should be red");
    }

    #[test]
    fn test_rtf_header_footer_extraction() {
        let rtf_content = r#"{\rtf1{\header My Header Text}{\footer My Footer Text}Body text}"#;
        let formatting = extract_rtf_formatting(rtf_content);
        assert!(formatting.header_text.is_some(), "Should extract header text");
        assert!(formatting.footer_text.is_some(), "Should extract footer text");
        assert!(
            formatting.header_text.as_deref().unwrap_or("").contains("Header"),
            "Header text should contain 'Header'"
        );
        assert!(
            formatting.footer_text.as_deref().unwrap_or("").contains("Footer"),
            "Footer text should contain 'Footer'"
        );
    }

    #[tokio::test]
    async fn test_rtf_document_structure_with_annotations() {
        let rtf_content = r#"{\rtf1 Normal text\par {\b Bold paragraph}\par More normal text}"#;
        let extractor = RtfExtractor::new();
        let config = ExtractionConfig {
            include_document_structure: true,
            ..Default::default()
        };
        let result = extractor
            .extract_bytes(rtf_content.as_bytes(), "application/rtf", &config)
            .await
            .expect("RTF extraction failed");
        let result =
            crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);
        assert!(result.document.is_some(), "Should produce document structure");
        let doc = result.document.unwrap();
        assert!(!doc.nodes.is_empty(), "Document structure should have nodes");
    }
}
