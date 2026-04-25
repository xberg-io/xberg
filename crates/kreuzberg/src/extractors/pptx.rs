//! PowerPoint presentation extractor.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extractors::security::SecurityBudget;
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
    pub(crate) fn new() -> Self {
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
    /// Try to strip an ordered-list prefix like `1. `, `2. `, `10. ` from a line.
    /// Returns the remaining text after the prefix, or `None` if the line does not
    /// start with a `<digits>. ` pattern.
    fn strip_ordered_prefix(line: &str) -> Option<&str> {
        let bytes = line.as_bytes();
        let mut i = 0;
        // Consume digits
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        // Need at least one digit followed by ". "
        if i == 0 || i + 2 > bytes.len() {
            return None;
        }
        if bytes[i] == b'.' && bytes[i + 1] == b' ' {
            Some(&line[i + 2..])
        } else {
            None
        }
    }

    fn build_internal_document(
        content: &str,
        slide_count: u32,
        budget: &mut SecurityBudget,
    ) -> Result<InternalDocument> {
        let mut builder = InternalDocumentBuilder::new("pptx");
        let mut slide_num: u32 = 0;
        let mut in_notes = false;

        // Split the content into logical blocks separated by blank lines.
        let blocks: Vec<&str> = content.split("\n\n").collect();

        for block in &blocks {
            budget.step()?;
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
                    budget.account_text(title.len())?;
                    builder.push_heading(2, title, None, None);
                }
                continue;
            }

            // If we're inside a notes section, skip content that looks like
            // continuation notes (not a slide heading or other content).
            // However, any non-notes, non-heading block means we've moved past
            // the notes into the next slide's body content. Reset in_notes so
            // untitled slides (slides with no `# Title` heading) are not lost.
            if in_notes {
                in_notes = false;
            }

            // Table block: starts with |
            if trimmed.starts_with('|') {
                let cells = Self::parse_markdown_table(trimmed);
                if !cells.is_empty() {
                    builder.push_table_from_cells(&cells, Some(slide_num), None);
                }
                continue;
            }

            // Process remaining lines: lists and paragraphs.
            // We track whether we are inside an unordered or ordered list so
            // that we can emit proper ListStart / ListEnd wrappers.
            let mut in_list: Option<bool> = None; // Some(ordered)

            for line in trimmed.lines() {
                let lt = line.trim();
                if lt.is_empty() {
                    // Close any open list on blank line
                    if in_list.is_some() {
                        builder.end_list();
                        in_list = None;
                    }
                    continue;
                }

                // Detect list item type from the trimmed line.
                // Unordered: "- text"
                // Ordered:   "1. text", "2. text", etc.
                let list_match = if let Some(item_text) = lt.strip_prefix("- ") {
                    Some((false, item_text))
                } else {
                    Self::strip_ordered_prefix(lt).map(|item_text| (true, item_text))
                };

                if let Some((ordered, item_text)) = list_match {
                    // Start a new list or switch list type if needed
                    match in_list {
                        Some(prev_ordered) if prev_ordered != ordered => {
                            builder.end_list();
                            builder.push_list(ordered);
                            in_list = Some(ordered);
                        }
                        None => {
                            builder.push_list(ordered);
                            in_list = Some(ordered);
                        }
                        _ => {}
                    }
                    budget.account_text(item_text.len())?;
                    builder.push_list_item(item_text, ordered, vec![], Some(slide_num), None);
                } else {
                    // Close any open list before emitting a paragraph
                    if in_list.is_some() {
                        builder.end_list();
                        in_list = None;
                    }
                    budget.account_text(lt.len())?;
                    builder.push_paragraph(lt, vec![], None, None);
                }
            }

            // Close any trailing open list at end of block
            if in_list.is_some() {
                builder.end_list();
            }
        }

        // If no slides were found, create a default slide
        if slide_num == 0 && slide_count > 0 {
            builder.push_slide(1, None, Some(1));
        }

        Ok(builder.build())
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
    ///
    /// `budget` is threaded into the internal document builder to enforce
    /// hostile-input limits on the extracted content.
    fn build_document_from_result(
        pptx_result: crate::types::PptxExtractionResult,
        mime_type: &str,
        extract_images: bool,
        budget: &mut SecurityBudget,
    ) -> Result<InternalDocument> {
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

        let mut doc = Self::build_internal_document(&pptx_result.content, pptx_result.slide_count as u32, budget)?;
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

        Ok(doc)
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
        tracing::debug!(format = "pptx", size_bytes = content.len(), "extraction starting");
        let extract_images = config.images.as_ref().is_some_and(|img| img.extract_images);
        let inject_placeholders = config
            .images
            .as_ref()
            .map(|img| img.inject_placeholders)
            .unwrap_or(true);
        let plain = matches!(config.output_format, crate::core::config::OutputFormat::Plain);

        let pptx_result = {
            #[cfg(feature = "tokio-runtime")]
            {
                if crate::core::batch_mode::is_batch_mode() {
                    if config.cancel_token.as_ref().map(|t| t.is_cancelled()).unwrap_or(false) {
                        return Err(crate::error::KreuzbergError::Cancelled);
                    }
                    let content_owned = content.to_vec();
                    let options = crate::extraction::pptx::PptxExtractionOptions {
                        extract_images,
                        page_config: config.pages.clone(),
                        plain,
                        include_structure: false, // we build InternalDocument separately
                        inject_placeholders,
                    };
                    let span = tracing::Span::current();
                    tokio::task::spawn_blocking(move || {
                        let _guard = span.entered();
                        crate::extraction::pptx::extract_pptx_from_bytes(&content_owned, &options)
                    })
                    .await
                    .map_err(|e| {
                        crate::error::KreuzbergError::parsing(format!("PPTX extraction task failed: {}", e))
                    })??
                } else {
                    let options = crate::extraction::pptx::PptxExtractionOptions {
                        extract_images,
                        page_config: config.pages.clone(),
                        plain,
                        include_structure: false,
                        inject_placeholders,
                    };
                    crate::extraction::pptx::extract_pptx_from_bytes(content, &options)?
                }
            }

            #[cfg(not(feature = "tokio-runtime"))]
            {
                let options = crate::extraction::pptx::PptxExtractionOptions {
                    extract_images,
                    page_config: config.pages.clone(),
                    plain,
                    include_structure: false,
                    inject_placeholders,
                };
                crate::extraction::pptx::extract_pptx_from_bytes(content, &options)?
            }
        };

        let mut budget = SecurityBudget::from_config(config);
        let mut doc = Self::build_document_from_result(pptx_result, mime_type, extract_images, &mut budget)?;

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

        tracing::debug!(
            element_count = doc.elements.len(),
            format = "pptx",
            "extraction complete"
        );
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
        let inject_placeholders = config
            .images
            .as_ref()
            .map(|img| img.inject_placeholders)
            .unwrap_or(true);
        let plain = matches!(config.output_format, crate::core::config::OutputFormat::Plain);

        let options = crate::extraction::pptx::PptxExtractionOptions {
            extract_images,
            page_config: config.pages.clone(),
            plain,
            include_structure: false,
            inject_placeholders,
        };
        let pptx_result = crate::extraction::pptx::extract_pptx_from_path(path_str, &options)?;

        let mut budget = SecurityBudget::from_config(config);
        let doc = Self::build_document_from_result(pptx_result, mime_type, extract_images, &mut budget)?;
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
