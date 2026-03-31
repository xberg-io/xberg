//! PowerPoint presentation extractor.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::internal::InternalDocument;
use crate::types::internal_builder::InternalDocumentBuilder;
use crate::types::metadata::Metadata;
use crate::types::uri::Uri;
use ahash::AHashMap;
use async_trait::async_trait;
use std::borrow::Cow;
use std::path::Path;

/// PowerPoint presentation extractor.
///
/// Supports: .pptx, .pptm, .ppsx
pub struct PptxExtractor;

impl Default for PptxExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl PptxExtractor {
    pub fn new() -> Self {
        Self
    }
}

impl PptxExtractor {
    /// Build an `InternalDocument` from PPTX extracted text.
    ///
    /// Splits content by double-newlines into slide-like blocks. Each block
    /// becomes a slide element with its content as paragraphs.
    ///
    /// Note: For richer structure, the builder should be integrated into
    /// `crate::extraction::pptx` alongside the existing `DocumentStructure` building.
    /// Strip leading markdown heading markers (`# `, `## `, etc.) from a line.
    #[allow(dead_code)]
    fn strip_heading_prefix(line: &str) -> &str {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            let after_hashes = trimmed.trim_start_matches('#');
            if after_hashes.starts_with(' ') {
                after_hashes.trim_start()
            } else {
                line
            }
        } else {
            line
        }
    }

    fn build_internal_document(content: &str, slide_count: u32) -> InternalDocument {
        let mut builder = InternalDocumentBuilder::new("pptx");
        let mut slide_num: u32 = 0;
        let mut in_notes = false;

        // Split the content into logical blocks separated by blank lines.
        let blocks: Vec<&str> = content.split("\n\n").collect();

        for block in &blocks {
            let trimmed = block.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Skip notes sections (### Notes: or Notes:)
            if trimmed.starts_with("### Notes:") || trimmed == "Notes:" {
                in_notes = true;
                continue;
            }

            // A `# Title` heading marks a new slide title.
            // We render it as a ## heading (not a Slide element) to avoid
            // the `---` separator that the Slide renderer inserts.
            if let Some(title_text) = trimmed.strip_prefix("# ") {
                in_notes = false;
                slide_num += 1;
                let title = title_text.trim();
                if !title.is_empty() {
                    builder.push_heading(2, title, None, None);
                }
                continue;
            }

            // If we're inside a notes section, skip until next slide
            if in_notes {
                continue;
            }

            // Table block: starts with |
            if trimmed.starts_with('|') {
                let cells = Self::parse_markdown_table(trimmed);
                if !cells.is_empty() {
                    builder.push_table_from_cells(&cells, Some(slide_num), None);
                }
                continue;
            }

            // Process remaining lines: lists and paragraphs
            for line in trimmed.lines() {
                let lt = line.trim();
                if lt.is_empty() {
                    continue;
                }
                // Unordered list item
                if let Some(item_text) = lt.strip_prefix("- ") {
                    builder.push_paragraph(item_text, vec![], None, None);
                }
                // Ordered list item
                else if let Some(item_text) = lt.strip_prefix("1. ") {
                    builder.push_paragraph(item_text, vec![], None, None);
                }
                // Image or regular paragraph
                else {
                    builder.push_paragraph(lt, vec![], None, None);
                }
            }
        }

        // If no slides were found, create a default slide
        if slide_num == 0 && slide_count > 0 {
            builder.push_slide(1, None, Some(1));
        }

        builder.build()
    }

    /// Parse a markdown table block into a 2D cell grid.
    fn parse_markdown_table(table_text: &str) -> Vec<Vec<String>> {
        let mut cells = Vec::new();
        for line in table_text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            // Skip separator rows like |---|---|
            if trimmed.contains("---") {
                continue;
            }
            // Parse pipe-separated cells
            let row: Vec<String> = trimmed
                .trim_matches('|')
                .split('|')
                .map(|cell| cell.trim().to_string())
                .collect();
            if !row.is_empty() {
                cells.push(row);
            }
        }
        cells
    }
}

impl PptxExtractor {
    /// Build an InternalDocument from a PptxExtractionResult, mapping office
    /// metadata to standard `Metadata` struct fields.
    fn build_document_from_result(
        pptx_result: crate::types::PptxExtractionResult,
        mime_type: &str,
        extract_images: bool,
    ) -> InternalDocument {
        let mut additional: AHashMap<Cow<'static, str>, serde_json::Value> = AHashMap::new();

        // Populate image_count and table_count on PptxMetadata struct
        let mut pptx_metadata = pptx_result.metadata;
        pptx_metadata.image_count = Some(pptx_result.image_count);
        pptx_metadata.table_count = Some(pptx_result.table_count);

        // Map office metadata to standard Metadata fields
        let office_meta = &pptx_result.office_metadata;
        let title = office_meta.get("title").cloned();
        let subject = office_meta.get("subject").cloned();
        let created_by = office_meta.get("created_by").cloned();
        let modified_by = office_meta.get("modified_by").cloned();
        let created_at = office_meta.get("created_at").cloned();
        let modified_at = office_meta.get("modified_at").cloned();
        let authors = office_meta.get("author").map(|a| vec![a.clone()]);
        let keywords = office_meta.get("keywords").map(|k| {
            k.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        });

        // Put remaining office metadata into additional map
        for (key, value) in &pptx_result.office_metadata {
            match key.as_str() {
                // Skip fields already mapped to standard Metadata fields
                "title" | "subject" | "created_by" | "modified_by" | "created_at" | "modified_at" | "author"
                | "keywords" => {}
                // Skip slide_count — already in PptxMetadata.slide_count (as a numeric type)
                "slide_count" => {}
                // Numeric fields: parse as JSON numbers so they serialize as integers, not strings
                "notes_count" | "hidden_slides" => {
                    let json_value = value
                        .parse::<u64>()
                        .map(|n| serde_json::Value::Number(n.into()))
                        .unwrap_or_else(|_| serde_json::json!(value));
                    additional.insert(Cow::Owned(key.clone()), json_value);
                }
                _ => {
                    additional.insert(Cow::Owned(key.clone()), serde_json::json!(value));
                }
            }
        }

        let mut doc = Self::build_internal_document(&pptx_result.content, pptx_result.slide_count as u32);
        doc.mime_type = Cow::Owned(mime_type.to_string());

        let mut metadata = Metadata {
            title,
            subject,
            authors,
            keywords,
            created_at,
            modified_at,
            created_by,
            modified_by,
            format: Some(crate::types::FormatMetadata::Pptx(pptx_metadata)),
            additional,
            ..Default::default()
        };

        if let Some(page_structure) = pptx_result.page_structure {
            metadata.pages = Some(page_structure);
        }

        doc.metadata = metadata;

        // Push hyperlink URIs discovered in slides
        for (url, label) in pptx_result.hyperlinks {
            doc.push_uri(Uri::hyperlink(&url, label));
        }

        // Transfer images
        if extract_images {
            doc.images = pptx_result.images;
        }

        doc
    }
}

impl Plugin for PptxExtractor {
    fn name(&self) -> &str {
        "pptx-extractor"
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
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl DocumentExtractor for PptxExtractor {
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        let extract_images = config.images.as_ref().is_some_and(|img| img.extract_images);
        let plain = matches!(config.output_format, crate::core::config::OutputFormat::Plain);

        let pptx_result = {
            #[cfg(feature = "tokio-runtime")]
            {
                let pages_config = config.pages.clone();
                if crate::core::batch_mode::is_batch_mode() {
                    let content_owned = content.to_vec();
                    let span = tracing::Span::current();
                    tokio::task::spawn_blocking(move || {
                        let _guard = span.entered();
                        crate::extraction::pptx::extract_pptx_from_bytes(
                            &content_owned,
                            extract_images,
                            pages_config.as_ref(),
                            plain,
                            false, // include_structure not needed, we build InternalDocument
                        )
                    })
                    .await
                    .map_err(|e| {
                        crate::error::KreuzbergError::parsing(format!("PPTX extraction task failed: {}", e))
                    })??
                } else {
                    crate::extraction::pptx::extract_pptx_from_bytes(
                        content,
                        extract_images,
                        config.pages.as_ref(),
                        plain,
                        false,
                    )?
                }
            }

            #[cfg(not(feature = "tokio-runtime"))]
            {
                crate::extraction::pptx::extract_pptx_from_bytes(
                    content,
                    extract_images,
                    config.pages.as_ref(),
                    plain,
                    false,
                )?
            }
        };

        let mut doc = Self::build_document_from_result(pptx_result, mime_type, extract_images);

        // Recursively extract embedded objects from ppt/embeddings/
        if config.max_archive_depth > 0 {
            let (children, embed_warnings) = crate::extraction::ooxml_embedded::extract_ooxml_embedded_objects(
                content,
                "ppt/embeddings/",
                "pptx",
                config,
            )
            .await;
            if !children.is_empty() {
                doc.children = Some(children);
            }
            doc.processing_warnings.extend(embed_warnings);
        }

        Ok(doc)
    }

    #[cfg_attr(feature = "otel", tracing::instrument(
        skip(self, path, config),
        fields(
            extractor.name = self.name(),
        )
    ))]
    async fn extract_file(&self, path: &Path, mime_type: &str, config: &ExtractionConfig) -> Result<InternalDocument> {
        let path_str = path
            .to_str()
            .ok_or_else(|| crate::KreuzbergError::validation("Invalid file path".to_string()))?;

        let extract_images = config.images.as_ref().is_some_and(|img| img.extract_images);
        let plain = matches!(config.output_format, crate::core::config::OutputFormat::Plain);

        let pptx_result = crate::extraction::pptx::extract_pptx_from_path(
            path_str,
            extract_images,
            config.pages.as_ref(),
            plain,
            false,
        )?;

        let doc = Self::build_document_from_result(pptx_result, mime_type, extract_images);
        Ok(doc)
    }

    fn supported_mime_types(&self) -> &[&str] {
        &[
            "application/vnd.openxmlformats-officedocument.presentationml.presentation",
            "application/vnd.ms-powerpoint.presentation.macroEnabled.12",
            "application/vnd.openxmlformats-officedocument.presentationml.slideshow",
            "application/vnd.openxmlformats-officedocument.presentationml.template",
            "application/vnd.ms-powerpoint.template.macroEnabled.12",
        ]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pptx_extractor_plugin_interface() {
        let extractor = PptxExtractor::new();
        assert_eq!(extractor.name(), "pptx-extractor");
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[test]
    fn test_pptx_extractor_supported_mime_types() {
        let extractor = PptxExtractor::new();
        let mime_types = extractor.supported_mime_types();
        assert_eq!(mime_types.len(), 5);
        assert!(mime_types.contains(&"application/vnd.openxmlformats-officedocument.presentationml.presentation"));
    }
}
