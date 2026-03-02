#![cfg(feature = "office")]

//! DOCX extractor for high-performance text extraction.
//!
//! Supports: Microsoft Word (.docx)

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extraction::{cells_to_markdown, office_metadata};
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::ExtractedImage;
use crate::types::{
    DocxMetadata, ExtractionResult, FormatMetadata, Metadata, PageBoundary, PageContent, PageInfo, PageStructure,
    PageUnitType, Table,
};
use ahash::AHashMap;
use async_trait::async_trait;
use bytes::Bytes;
use std::borrow::Cow;
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;

/// High-performance DOCX extractor.
///
/// This extractor provides:
/// - Fast text extraction via streaming XML parsing
/// - Comprehensive metadata extraction (core.xml, app.xml, custom.xml)
pub struct DocxExtractor;

impl DocxExtractor {
    /// Create a new DOCX extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Default for DocxExtractor {
    fn default() -> Self {
        Self::new()
    }
}

/// Build a DocumentStructure from parsed DOCX data.
///
/// Creates a hierarchical tree with heading-based sections, paragraphs,
/// lists, tables, images, headers/footers, and footnotes/endnotes.
fn build_document_structure(doc: &crate::extraction::docx::parser::Document) -> crate::types::DocumentStructure {
    use crate::types::{
        ContentLayer, DocumentNode, DocumentStructure, GridCell, NodeContent, NodeId, NodeIndex, TableGrid,
    };

    let mut structure = DocumentStructure::with_capacity(
        doc.paragraphs.len() + doc.tables.len() + doc.drawings.len() + doc.headers.len() + doc.footers.len() + 16,
    );
    let mut section_stack: Vec<(u8, NodeIndex)> = Vec::new();
    let mut node_count: u32 = 0;

    // Helper to find the current parent based on section_stack
    let current_parent = |stack: &[(u8, NodeIndex)]| -> Option<NodeIndex> { stack.last().map(|(_, idx)| *idx) };

    // Process body elements in document order
    for element in &doc.elements {
        match element {
            crate::extraction::docx::parser::DocumentElement::Paragraph(idx) => {
                let paragraph = &doc.paragraphs[*idx];
                let text = paragraph.runs_to_markdown();
                if text.is_empty() {
                    continue;
                }

                // Check if this paragraph is a heading
                let heading_level = paragraph.style.as_deref().and_then(|s| doc.resolve_heading_level(s));

                if let Some(level) = heading_level {
                    // Pop sections at same or deeper level
                    while section_stack.last().is_some_and(|(l, _)| *l >= level) {
                        section_stack.pop();
                    }

                    // Create a Group node for the heading section
                    let group = DocumentNode {
                        id: NodeId::generate("group", &text, None, node_count),
                        content: NodeContent::Group {
                            label: None,
                            heading_level: Some(level),
                            heading_text: Some(text.clone()),
                        },
                        parent: current_parent(&section_stack),
                        children: Vec::new(),
                        content_layer: ContentLayer::Body,
                        page: None,
                        page_end: None,
                        bbox: None,
                        annotations: Vec::new(),
                    };
                    node_count += 1;
                    let group_idx = structure.push_node(group);
                    if let Some(parent_idx) = current_parent(&section_stack) {
                        structure.add_child(parent_idx, group_idx);
                    }

                    // Add heading node as child of group
                    let heading = DocumentNode {
                        id: NodeId::generate("heading", &text, None, node_count),
                        content: NodeContent::Heading { level, text },
                        parent: Some(group_idx),
                        children: Vec::new(),
                        content_layer: ContentLayer::Body,
                        page: None,
                        page_end: None,
                        bbox: None,
                        annotations: Vec::new(),
                    };
                    node_count += 1;
                    let heading_idx = structure.push_node(heading);
                    structure.add_child(group_idx, heading_idx);

                    section_stack.push((level, group_idx));
                } else if paragraph.numbering_id.is_some() {
                    // List item - create as paragraph for now (list grouping done by transform)
                    let node = DocumentNode {
                        id: NodeId::generate("list_item", &text, None, node_count),
                        content: NodeContent::ListItem { text },
                        parent: current_parent(&section_stack),
                        children: Vec::new(),
                        content_layer: ContentLayer::Body,
                        page: None,
                        page_end: None,
                        bbox: None,
                        annotations: Vec::new(),
                    };
                    node_count += 1;
                    let idx = structure.push_node(node);
                    if let Some(parent_idx) = current_parent(&section_stack) {
                        structure.add_child(parent_idx, idx);
                    }
                } else {
                    // Regular paragraph
                    let node = DocumentNode {
                        id: NodeId::generate("paragraph", &text, None, node_count),
                        content: NodeContent::Paragraph { text },
                        parent: current_parent(&section_stack),
                        children: Vec::new(),
                        content_layer: ContentLayer::Body,
                        page: None,
                        page_end: None,
                        bbox: None,
                        annotations: Vec::new(),
                    };
                    node_count += 1;
                    let idx = structure.push_node(node);
                    if let Some(parent_idx) = current_parent(&section_stack) {
                        structure.add_child(parent_idx, idx);
                    }
                }
            }
            crate::extraction::docx::parser::DocumentElement::Table(idx) => {
                let table = &doc.tables[*idx];
                let rows = table.rows.len() as u32;
                let cols = table.rows.first().map_or(0, |r| r.cells.len()) as u32;
                let mut cells = Vec::new();
                for (row_idx, row) in table.rows.iter().enumerate() {
                    let is_header = row.properties.as_ref().is_some_and(|p| p.is_header) || row_idx == 0;
                    for (col_idx, cell) in row.cells.iter().enumerate() {
                        let content: String = cell
                            .paragraphs
                            .iter()
                            .map(|p| p.runs_to_markdown())
                            .collect::<Vec<_>>()
                            .join(" ")
                            .trim()
                            .to_string();
                        let col_span = cell.properties.as_ref().and_then(|p| p.grid_span).unwrap_or(1);
                        cells.push(GridCell {
                            content,
                            row: row_idx as u32,
                            col: col_idx as u32,
                            row_span: 1,
                            col_span,
                            is_header,
                            bbox: None,
                        });
                    }
                }
                let grid = TableGrid { rows, cols, cells };
                let node = DocumentNode {
                    id: NodeId::generate("table", "", None, node_count),
                    content: NodeContent::Table { grid },
                    parent: current_parent(&section_stack),
                    children: Vec::new(),
                    content_layer: ContentLayer::Body,
                    page: None,
                    page_end: None,
                    bbox: None,
                    annotations: Vec::new(),
                };
                node_count += 1;
                let table_idx = structure.push_node(node);
                if let Some(parent_idx) = current_parent(&section_stack) {
                    structure.add_child(parent_idx, table_idx);
                }
            }
            crate::extraction::docx::parser::DocumentElement::Drawing(idx) => {
                let drawing = &doc.drawings[*idx];
                let description = drawing.doc_properties.as_ref().and_then(|dp| dp.description.clone());
                let node = DocumentNode {
                    id: NodeId::generate("image", "", None, node_count),
                    content: NodeContent::Image {
                        description,
                        image_index: Some(*idx as u32),
                    },
                    parent: current_parent(&section_stack),
                    children: Vec::new(),
                    content_layer: ContentLayer::Body,
                    page: None,
                    page_end: None,
                    bbox: None,
                    annotations: Vec::new(),
                };
                node_count += 1;
                let img_idx = structure.push_node(node);
                if let Some(parent_idx) = current_parent(&section_stack) {
                    structure.add_child(parent_idx, img_idx);
                }
            }
        }
    }

    // Add headers and footers with appropriate content layers
    {
        let items_and_layers: &[(&[crate::extraction::docx::parser::HeaderFooter], ContentLayer)] = &[
            (&doc.headers, ContentLayer::Header),
            (&doc.footers, ContentLayer::Footer),
        ];
        for (items, layer) in items_and_layers {
            for hf in *items {
                let text: String = hf
                    .paragraphs
                    .iter()
                    .map(|p| p.runs_to_markdown())
                    .collect::<Vec<_>>()
                    .join("\n");
                if text.is_empty() {
                    continue;
                }
                let node = DocumentNode {
                    id: NodeId::generate("paragraph", &text, None, node_count),
                    content: NodeContent::Paragraph { text },
                    parent: None,
                    children: Vec::new(),
                    content_layer: *layer,
                    page: None,
                    page_end: None,
                    bbox: None,
                    annotations: Vec::new(),
                };
                node_count += 1;
                structure.push_node(node);
            }
        }
    }

    // Add footnotes and endnotes as Footnote content layer
    for note in doc.footnotes.iter().chain(doc.endnotes.iter()) {
        let text: String = note
            .paragraphs
            .iter()
            .map(|p| p.runs_to_markdown())
            .collect::<Vec<_>>()
            .join(" ");
        if text.is_empty() {
            continue;
        }
        let node = DocumentNode {
            id: NodeId::generate("footnote", &text, None, node_count),
            content: NodeContent::Footnote { text },
            parent: None,
            children: Vec::new(),
            content_layer: ContentLayer::Footnote,
            page: None,
            page_end: None,
            bbox: None,
            annotations: Vec::new(),
        };
        node_count += 1;
        structure.push_node(node);
    }

    debug_assert!(structure.validate().is_ok());
    structure
}

type DocxParseResult = (
    String,
    Vec<Table>,
    Option<Vec<PageBoundary>>,
    Vec<crate::extraction::docx::drawing::Drawing>,
    HashMap<String, String>,
    Option<crate::types::DocumentStructure>,
);

/// Parse DOCX document content and extract text, tables, page boundaries, drawings, image relationships, and optional document structure.
fn parse_docx_core(
    content: &[u8],
    include_doc_structure: bool,
    output_format: crate::core::config::OutputFormat,
) -> crate::error::Result<DocxParseResult> {
    let doc = crate::extraction::docx::parser::parse_document(content)?;
    let text = match output_format {
        crate::core::config::OutputFormat::Markdown
        | crate::core::config::OutputFormat::Djot
        | crate::core::config::OutputFormat::Html => doc.to_markdown(),
        _ => doc.to_plain_text(),
    };
    let tables: Vec<Table> = doc
        .tables
        .iter()
        .enumerate()
        .map(|(idx, table)| convert_docx_table_to_table(table, idx))
        .collect();
    let page_boundaries = crate::extraction::docx::detect_page_breaks_from_docx(content)?;
    let drawings = doc.drawings.clone();
    let image_rels = doc.image_relationships.clone();
    let doc_structure = if include_doc_structure {
        Some(build_document_structure(&doc))
    } else {
        None
    };
    Ok((text, tables, page_boundaries, drawings, image_rels, doc_structure))
}

impl Plugin for DocxExtractor {
    fn name(&self) -> &str {
        "docx-extractor"
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
        "High-performance DOCX text extraction with metadata support"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

/// Convert parsed DOCX table to Kreuzberg Table struct with markdown representation.
///
/// # Arguments
/// * `docx_table` - The parsed DOCX table
/// * `table_index` - Index of the table in the document (used as page_number)
///
/// # Returns
/// * `Table` - Converted table with cells and markdown representation
fn convert_docx_table_to_table(docx_table: &crate::extraction::docx::parser::Table, table_index: usize) -> Table {
    let cells: Vec<Vec<String>> = docx_table
        .rows
        .iter()
        .map(|row| {
            row.cells
                .iter()
                .map(|cell| {
                    let is_vmerge_continue = cell.properties.as_ref().is_some_and(|p| {
                        matches!(p.v_merge, Some(crate::extraction::docx::table::VerticalMerge::Continue))
                    });
                    if is_vmerge_continue {
                        String::new()
                    } else {
                        cell.paragraphs
                            .iter()
                            .map(|para| para.runs_to_markdown())
                            .collect::<Vec<_>>()
                            .join(" ")
                            .trim()
                            .to_string()
                    }
                })
                .collect()
        })
        .collect();

    let markdown = cells_to_markdown(&cells);

    Table {
        cells,
        markdown,
        page_number: table_index + 1,
        bounding_box: None,
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl DocumentExtractor for DocxExtractor {
    #[cfg_attr(feature = "otel", tracing::instrument(
        skip(self, content, config),
        fields(
            extractor.name = self.name(),
            content.size_bytes = content.len(),
        )
    ))]
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        let include_doc_structure = config.include_document_structure;
        let output_format = config.output_format;

        let (text, tables, page_boundaries, drawings, image_rels, doc_structure) = {
            #[cfg(feature = "tokio-runtime")]
            if crate::core::batch_mode::is_batch_mode() {
                let content_owned = content.to_vec();
                let span = tracing::Span::current();
                tokio::task::spawn_blocking(move || {
                    let _guard = span.entered();
                    parse_docx_core(&content_owned, include_doc_structure, output_format)
                })
                .await
                .map_err(|e| crate::error::KreuzbergError::parsing(format!("DOCX extraction task failed: {}", e)))??
            } else {
                parse_docx_core(content, include_doc_structure, output_format)?
            }

            #[cfg(not(feature = "tokio-runtime"))]
            parse_docx_core(content, include_doc_structure, output_format)?
        };

        let mut archive = {
            #[cfg(feature = "tokio-runtime")]
            if crate::core::batch_mode::is_batch_mode() {
                let content_owned = content.to_vec();
                let span = tracing::Span::current();
                tokio::task::spawn_blocking(move || -> crate::error::Result<_> {
                    let _guard = span.entered();
                    let cursor = Cursor::new(content_owned);
                    zip::ZipArchive::new(cursor).map_err(|e| {
                        crate::error::KreuzbergError::parsing(format!("Failed to open ZIP archive: {}", e))
                    })
                })
                .await
                .map_err(|e| crate::error::KreuzbergError::parsing(format!("Task join error: {}", e)))??
            } else {
                let content_owned = content.to_vec();
                let cursor = Cursor::new(content_owned);
                zip::ZipArchive::new(cursor)
                    .map_err(|e| crate::error::KreuzbergError::parsing(format!("Failed to open ZIP archive: {}", e)))?
            }

            #[cfg(not(feature = "tokio-runtime"))]
            {
                let content_owned = content.to_vec();
                let cursor = Cursor::new(content_owned);
                zip::ZipArchive::new(cursor)
                    .map_err(|e| crate::error::KreuzbergError::parsing(format!("Failed to open ZIP archive: {}", e)))?
            }
        };

        let mut metadata_map = AHashMap::new();
        let mut parsed_keywords: Option<Vec<String>> = None;
        let mut docx_core_properties = None;
        let mut docx_app_properties = None;
        let mut docx_custom_properties: Option<HashMap<String, serde_json::Value>> = None;

        if let Ok(core) = office_metadata::extract_core_properties(&mut archive) {
            if let Some(ref title) = core.title {
                metadata_map.insert(Cow::Borrowed("title"), serde_json::Value::String(title.clone()));
            }
            if let Some(ref creator) = core.creator {
                metadata_map.insert(
                    Cow::Borrowed("authors"),
                    serde_json::Value::Array(vec![serde_json::Value::String(creator.clone())]),
                );
                metadata_map.insert(Cow::Borrowed("created_by"), serde_json::Value::String(creator.clone()));
            }
            if let Some(ref subject) = core.subject {
                metadata_map.insert(Cow::Borrowed("subject"), serde_json::Value::String(subject.clone()));
            }
            if let Some(ref keywords) = core.keywords {
                // Parse comma-separated keywords into Vec<String>
                parsed_keywords = Some(
                    keywords
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect(),
                );
            }
            if let Some(ref description) = core.description {
                metadata_map.insert(
                    Cow::Borrowed("description"),
                    serde_json::Value::String(description.clone()),
                );
            }
            if let Some(ref modified_by) = core.last_modified_by {
                metadata_map.insert(
                    Cow::Borrowed("modified_by"),
                    serde_json::Value::String(modified_by.clone()),
                );
            }
            if let Some(ref created) = core.created {
                metadata_map.insert(Cow::Borrowed("created_at"), serde_json::Value::String(created.clone()));
            }
            if let Some(ref modified) = core.modified {
                metadata_map.insert(
                    Cow::Borrowed("modified_at"),
                    serde_json::Value::String(modified.clone()),
                );
            }
            if let Some(ref revision) = core.revision {
                metadata_map.insert(Cow::Borrowed("revision"), serde_json::Value::String(revision.clone()));
            }
            if let Some(ref category) = core.category {
                metadata_map.insert(Cow::Borrowed("category"), serde_json::Value::String(category.clone()));
            }
            if let Some(ref content_status) = core.content_status {
                metadata_map.insert(
                    Cow::Borrowed("content_status"),
                    serde_json::Value::String(content_status.clone()),
                );
            }
            if let Some(ref language) = core.language {
                metadata_map.insert(Cow::Borrowed("language"), serde_json::Value::String(language.clone()));
            }
            docx_core_properties = Some(core);
        }

        if let Ok(app) = office_metadata::extract_docx_app_properties(&mut archive) {
            if let Some(pages) = app.pages {
                metadata_map.insert(Cow::Borrowed("page_count"), serde_json::Value::Number(pages.into()));
            }
            if let Some(words) = app.words {
                metadata_map.insert(Cow::Borrowed("word_count"), serde_json::Value::Number(words.into()));
            }
            if let Some(chars) = app.characters {
                metadata_map.insert(
                    Cow::Borrowed("character_count"),
                    serde_json::Value::Number(chars.into()),
                );
            }
            if let Some(lines) = app.lines {
                metadata_map.insert(Cow::Borrowed("line_count"), serde_json::Value::Number(lines.into()));
            }
            if let Some(paragraphs) = app.paragraphs {
                metadata_map.insert(
                    Cow::Borrowed("paragraph_count"),
                    serde_json::Value::Number(paragraphs.into()),
                );
            }
            if let Some(ref template) = app.template {
                metadata_map.insert(Cow::Borrowed("template"), serde_json::Value::String(template.clone()));
            }
            if let Some(ref company) = app.company {
                metadata_map.insert(Cow::Borrowed("company"), serde_json::Value::String(company.clone()));
            }
            if let Some(time) = app.total_time {
                metadata_map.insert(
                    Cow::Borrowed("total_editing_time_minutes"),
                    serde_json::Value::Number(time.into()),
                );
            }
            if let Some(ref application) = app.application {
                metadata_map.insert(
                    Cow::Borrowed("application"),
                    serde_json::Value::String(application.clone()),
                );
            }
            docx_app_properties = Some(app);
        }

        if let Ok(custom) = office_metadata::extract_custom_properties(&mut archive) {
            for (key, value) in &custom {
                metadata_map.insert(Cow::Owned(format!("custom_{}", key)), value.clone());
            }
            docx_custom_properties = Some(custom);
        }

        let page_structure = if let Some(boundaries) = page_boundaries {
            let total_count = boundaries.len();
            Some(PageStructure {
                total_count,
                unit_type: PageUnitType::Page,
                boundaries: Some(boundaries),
                pages: Some(
                    (1..=total_count)
                        .map(|page_num| PageInfo {
                            number: page_num,
                            title: None,
                            dimensions: None,
                            image_count: None,
                            table_count: None,
                            hidden: None,
                            is_blank: None,
                        })
                        .collect(),
                ),
            })
        } else {
            None
        };

        // Extract images from drawings if configured
        let extracted_images = if config.images.as_ref().is_some_and(|i| i.extract_images) {
            let mut images = Vec::new();
            for (idx, drawing) in drawings.iter().enumerate() {
                if let Some(ref rid) = drawing.image_ref
                    && let Some(target) = image_rels.get(rid)
                {
                    // Reject path traversal attempts within the archive
                    if target.contains("..") {
                        continue;
                    }
                    let zip_path = if let Some(stripped) = target.strip_prefix('/') {
                        stripped.to_string()
                    } else {
                        format!("word/{}", target)
                    };
                    if let Ok(mut file) = archive.by_name(&zip_path) {
                        if file.size() > crate::extraction::docx::MAX_IMAGE_FILE_SIZE {
                            continue;
                        }
                        let mut data = Vec::with_capacity(file.size() as usize);
                        if std::io::Read::read_to_end(&mut file, &mut data).is_ok() {
                            let format = crate::extraction::image_format::detect_image_format(&data);
                            let emus_per_px = crate::extraction::docx::EMUS_PER_PIXEL_96DPI;
                            let (width, height) = drawing
                                .extent
                                .as_ref()
                                .map(|e| {
                                    (
                                        Some(u32::try_from(e.cx.max(0) / emus_per_px).unwrap_or(0)),
                                        Some(u32::try_from(e.cy.max(0) / emus_per_px).unwrap_or(0)),
                                    )
                                })
                                .unwrap_or((None, None));
                            let description = drawing.doc_properties.as_ref().and_then(|dp| dp.description.clone());
                            // Determine page number from image placeholder position in text
                            let page_number = {
                                let placeholder = format!("![](image_{})", idx);
                                // Also try with description
                                let placeholder_with_desc =
                                    description.as_ref().map(|d| format!("![{}](image_{})", d, idx));

                                let byte_pos = text
                                    .find(&placeholder)
                                    .or_else(|| placeholder_with_desc.as_deref().and_then(|p| text.find(p)));

                                if let Some(pos) = byte_pos {
                                    if let Some(ref ps) = page_structure
                                        && let Some(ref boundaries) = ps.boundaries
                                    {
                                        boundaries
                                            .iter()
                                            .find(|b| pos >= b.byte_start && pos < b.byte_end)
                                            .map(|b| b.page_number)
                                    } else {
                                        Some(1)
                                    }
                                } else {
                                    Some(1) // Default to page 1 if placeholder not found
                                }
                            };

                            images.push(ExtractedImage {
                                data: Bytes::from(data),
                                format,
                                image_index: idx,
                                page_number,
                                width,
                                height,
                                colorspace: None,
                                bits_per_component: None,
                                is_mask: false,
                                description,
                                ocr_result: None,
                                bounding_box: None,
                            });
                        }
                    }
                }
            }
            images
        } else {
            Vec::new()
        };

        // Run OCR on extracted images if configured
        #[cfg(all(feature = "ocr", feature = "tokio-runtime"))]
        let extracted_images = crate::extraction::image_ocr::process_images_with_ocr(extracted_images, config).await?;

        // Build PageContent from page boundaries
        let page_contents = {
            let arc_tables: Vec<Arc<Table>> = tables.iter().map(|t| Arc::new(t.clone())).collect();
            let arc_images: Vec<Arc<ExtractedImage>> = extracted_images.iter().map(|i| Arc::new(i.clone())).collect();

            if let Some(ref ps) = page_structure
                && let Some(ref boundaries) = ps.boundaries
                && !boundaries.is_empty()
            {
                let mut pages = Vec::with_capacity(boundaries.len());
                for boundary in boundaries {
                    let page_num = boundary.page_number;
                    // Extract text slice for this page
                    let page_text = if boundary.byte_start < text.len() {
                        let mut start = boundary.byte_start.min(text.len());
                        while start < text.len() && !text.is_char_boundary(start) {
                            start += 1;
                        }
                        let mut end = boundary.byte_end.min(text.len());
                        while end > start && !text.is_char_boundary(end) {
                            end -= 1;
                        }
                        text[start..end].trim().to_string()
                    } else {
                        String::new()
                    };

                    // Filter tables for this page
                    let page_tables: Vec<Arc<Table>> = arc_tables
                        .iter()
                        .filter(|t| t.page_number == page_num)
                        .cloned()
                        .collect();

                    // Filter images for this page
                    let page_images: Vec<Arc<ExtractedImage>> = arc_images
                        .iter()
                        .filter(|i| i.page_number == Some(page_num))
                        .cloned()
                        .collect();

                    let is_blank = page_text.chars().filter(|c| !c.is_whitespace()).count() < 3
                        && page_tables.is_empty()
                        && page_images.is_empty();

                    pages.push(PageContent {
                        page_number: page_num,
                        content: page_text,
                        tables: page_tables,
                        images: page_images,
                        hierarchy: None,
                        is_blank: Some(is_blank),
                    });
                }
                Some(pages)
            } else {
                // Single page fallback
                Some(vec![PageContent {
                    page_number: 1,
                    content: text.clone(),
                    tables: arc_tables,
                    images: arc_images,
                    hierarchy: None,
                    is_blank: Some(text.chars().filter(|c| !c.is_whitespace()).count() < 3),
                }])
            }
        };

        // Extract typed metadata fields and remove them from additional map to avoid duplication
        let meta_title = metadata_map
            .remove(&Cow::Borrowed("title"))
            .and_then(|v| v.as_str().map(|s| s.to_string()));
        let meta_subject = metadata_map
            .remove(&Cow::Borrowed("subject"))
            .and_then(|v| v.as_str().map(|s| s.to_string()));
        let meta_authors = metadata_map.remove(&Cow::Borrowed("authors")).and_then(|v| {
            v.as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        });
        let meta_created_by = metadata_map
            .remove(&Cow::Borrowed("created_by"))
            .and_then(|v| v.as_str().map(|s| s.to_string()));
        let meta_modified_by = metadata_map
            .remove(&Cow::Borrowed("modified_by"))
            .and_then(|v| v.as_str().map(|s| s.to_string()));
        let meta_created_at = metadata_map
            .remove(&Cow::Borrowed("created_at"))
            .and_then(|v| v.as_str().map(|s| s.to_string()));
        let meta_modified_at = metadata_map
            .remove(&Cow::Borrowed("modified_at"))
            .and_then(|v| v.as_str().map(|s| s.to_string()));
        let meta_language = metadata_map
            .remove(&Cow::Borrowed("language"))
            .and_then(|v| v.as_str().map(|s| s.to_string()));

        Ok(ExtractionResult {
            content: text,
            mime_type: mime_type.to_string().into(),
            metadata: Metadata {
                title: meta_title,
                subject: meta_subject,
                authors: meta_authors,
                keywords: parsed_keywords,
                language: meta_language,
                created_at: meta_created_at,
                modified_at: meta_modified_at,
                created_by: meta_created_by,
                modified_by: meta_modified_by,
                pages: page_structure,
                format: Some(FormatMetadata::Docx(Box::new(DocxMetadata {
                    core_properties: docx_core_properties,
                    app_properties: docx_app_properties,
                    custom_properties: docx_custom_properties,
                }))),
                additional: metadata_map,
                ..Default::default()
            },
            pages: page_contents,
            tables,
            detected_languages: None,
            chunks: None,
            images: if extracted_images.is_empty() {
                None
            } else {
                Some(extracted_images)
            },
            djot_content: None,
            elements: None,
            ocr_elements: None,
            document: doc_structure,
            #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
            extracted_keywords: None,
            quality_score: None,
            processing_warnings: Vec::new(),
            annotations: None,
        })
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/vnd.openxmlformats-officedocument.wordprocessingml.document"]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_docx_extractor_plugin_interface() {
        let extractor = DocxExtractor::new();
        assert_eq!(extractor.name(), "docx-extractor");
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert_eq!(extractor.priority(), 50);
        assert_eq!(extractor.supported_mime_types().len(), 1);
    }

    #[tokio::test]
    async fn test_docx_extractor_supports_docx() {
        let extractor = DocxExtractor::new();
        assert!(
            extractor
                .supported_mime_types()
                .contains(&"application/vnd.openxmlformats-officedocument.wordprocessingml.document")
        );
    }

    #[tokio::test]
    async fn test_docx_extractor_default() {
        let extractor = DocxExtractor;
        assert_eq!(extractor.name(), "docx-extractor");
    }

    #[tokio::test]
    async fn test_docx_extractor_initialize_shutdown() {
        let extractor = DocxExtractor::new();
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[test]
    fn test_convert_docx_table_to_table() {
        use crate::extraction::docx::parser::{Paragraph, Run, Table as DocxTable, TableCell, TableRow};

        let mut table = DocxTable::new();

        let mut header_row = TableRow::default();
        let mut cell1 = TableCell::default();
        let mut para1 = Paragraph::new();
        para1.add_run(Run::new("Name".to_string()));
        cell1.paragraphs.push(para1);
        header_row.cells.push(cell1);

        let mut cell2 = TableCell::default();
        let mut para2 = Paragraph::new();
        para2.add_run(Run::new("Age".to_string()));
        cell2.paragraphs.push(para2);
        header_row.cells.push(cell2);

        table.rows.push(header_row);

        let mut data_row = TableRow::default();
        let mut cell3 = TableCell::default();
        let mut para3 = Paragraph::new();
        para3.add_run(Run::new("Alice".to_string()));
        cell3.paragraphs.push(para3);
        data_row.cells.push(cell3);

        let mut cell4 = TableCell::default();
        let mut para4 = Paragraph::new();
        para4.add_run(Run::new("30".to_string()));
        cell4.paragraphs.push(para4);
        data_row.cells.push(cell4);

        table.rows.push(data_row);

        let result = convert_docx_table_to_table(&table, 0);

        assert_eq!(result.page_number, 1);
        assert_eq!(result.cells.len(), 2);
        assert_eq!(result.cells[0], vec!["Name", "Age"]);
        assert_eq!(result.cells[1], vec!["Alice", "30"]);
        assert!(result.markdown.contains("| Name | Age |"));
        assert!(result.markdown.contains("| Alice | 30 |"));
    }

    /// Helper: build a minimal DOCX ZIP in memory with given document.xml content.
    fn build_test_docx(document_xml: &str) -> Vec<u8> {
        build_test_docx_with_parts(document_xml, None, None, None, None, None)
    }

    /// Helper: build a DOCX ZIP with optional parts.
    fn build_test_docx_with_parts(
        document_xml: &str,
        styles_xml: Option<&str>,
        footnotes_xml: Option<&str>,
        endnotes_xml: Option<&str>,
        header_xml: Option<&str>,
        footer_xml: Option<&str>,
    ) -> Vec<u8> {
        use std::io::Write;
        let buf = Vec::new();
        let cursor = std::io::Cursor::new(buf);
        let mut zip = zip::ZipWriter::new(cursor);
        let options: zip::write::FileOptions<()> = zip::write::FileOptions::default();

        // Content types
        let content_types = r#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#;
        zip.start_file("[Content_Types].xml", options).unwrap();
        zip.write_all(content_types.as_bytes()).unwrap();

        // Document
        zip.start_file("word/document.xml", options).unwrap();
        zip.write_all(document_xml.as_bytes()).unwrap();

        // Styles
        if let Some(styles) = styles_xml {
            zip.start_file("word/styles.xml", options).unwrap();
            zip.write_all(styles.as_bytes()).unwrap();
        }

        // Footnotes
        if let Some(fn_xml) = footnotes_xml {
            zip.start_file("word/footnotes.xml", options).unwrap();
            zip.write_all(fn_xml.as_bytes()).unwrap();
        }

        // Endnotes
        if let Some(en_xml) = endnotes_xml {
            zip.start_file("word/endnotes.xml", options).unwrap();
            zip.write_all(en_xml.as_bytes()).unwrap();
        }

        // Header
        if let Some(h_xml) = header_xml {
            zip.start_file("word/header1.xml", options).unwrap();
            zip.write_all(h_xml.as_bytes()).unwrap();
        }

        // Footer
        if let Some(f_xml) = footer_xml {
            zip.start_file("word/footer1.xml", options).unwrap();
            zip.write_all(f_xml.as_bytes()).unwrap();
        }

        zip.finish().unwrap().into_inner()
    }

    #[tokio::test]
    async fn test_full_extraction_with_headings_paragraphs() {
        let document_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:pPr><w:pStyle w:val="Title"/></w:pPr><w:r><w:t>Document Title</w:t></w:r></w:p>
    <w:p><w:r><w:t>First paragraph content.</w:t></w:r></w:p>
    <w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>Section One</w:t></w:r></w:p>
    <w:p><w:r><w:t>Section one body text.</w:t></w:r></w:p>
  </w:body>
</w:document>"#;

        let data = build_test_docx(document_xml);
        let extractor = DocxExtractor::new();
        let config = ExtractionConfig {
            output_format: crate::core::config::OutputFormat::Markdown,
            ..Default::default()
        };
        let result = extractor
            .extract_bytes(
                &data,
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                &config,
            )
            .await
            .unwrap();

        // Verify markdown content
        assert!(
            result.content.contains("# Document Title"),
            "Title should be h1: {}",
            result.content
        );
        assert!(
            result.content.contains("## Section One"),
            "Heading1 should be h2: {}",
            result.content
        );
        assert!(result.content.contains("First paragraph content."));
        assert!(result.content.contains("Section one body text."));

        // Verify pages are populated
        assert!(result.pages.is_some(), "Pages should be populated");
        let pages = result.pages.unwrap();
        assert!(!pages.is_empty(), "Should have at least one page");
    }

    #[tokio::test]
    async fn test_full_extraction_with_formatting() {
        let document_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:r><w:rPr><w:b/></w:rPr><w:t>Bold text</w:t></w:r>
      <w:r><w:t> and </w:t></w:r>
      <w:r><w:rPr><w:i/></w:rPr><w:t>italic text</w:t></w:r>
      <w:r><w:t> and </w:t></w:r>
      <w:r><w:rPr><w:u/></w:rPr><w:t>underlined text</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;

        let data = build_test_docx(document_xml);
        let extractor = DocxExtractor::new();
        let config = ExtractionConfig {
            output_format: crate::core::config::OutputFormat::Markdown,
            ..Default::default()
        };
        let result = extractor
            .extract_bytes(
                &data,
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                &config,
            )
            .await
            .unwrap();

        assert!(result.content.contains("**Bold text**"), "Bold: {}", result.content);
        assert!(result.content.contains("*italic text*"), "Italic: {}", result.content);
        assert!(
            result.content.contains("<u>underlined text</u>"),
            "Underline: {}",
            result.content
        );
    }

    #[tokio::test]
    async fn test_full_extraction_with_headers_footers() {
        let document_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>Body content here.</w:t></w:r></w:p>
  </w:body>
</w:document>"#;

        let header_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:hdr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:p><w:r><w:t>Page Header</w:t></w:r></w:p>
</w:hdr>"#;

        let footer_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:ftr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:p><w:r><w:t>Page Footer</w:t></w:r></w:p>
</w:ftr>"#;

        let data = build_test_docx_with_parts(document_xml, None, None, None, Some(header_xml), Some(footer_xml));
        let extractor = DocxExtractor::new();
        let config = ExtractionConfig {
            output_format: crate::core::config::OutputFormat::Markdown,
            ..Default::default()
        };
        let result = extractor
            .extract_bytes(
                &data,
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                &config,
            )
            .await
            .unwrap();

        // Headers/footers should NOT appear in text output
        assert!(
            !result.content.contains("Page Header"),
            "Header should not be in output: {}",
            result.content
        );
        assert!(
            result.content.contains("Body content here."),
            "Body: {}",
            result.content
        );
        assert!(
            !result.content.contains("Page Footer"),
            "Footer should not be in output: {}",
            result.content
        );
    }

    #[tokio::test]
    async fn test_full_extraction_with_footnotes() {
        let document_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:r><w:t>Text with note</w:t></w:r>
      <w:r><w:footnoteReference w:id="2"/></w:r>
    </w:p>
  </w:body>
</w:document>"#;

        let footnotes_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:footnotes xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:footnote w:id="0"><w:p><w:r><w:t>separator</w:t></w:r></w:p></w:footnote>
  <w:footnote w:id="1"><w:p><w:r><w:t>continuation</w:t></w:r></w:p></w:footnote>
  <w:footnote w:id="2"><w:p><w:r><w:t>This is the footnote content.</w:t></w:r></w:p></w:footnote>
</w:footnotes>"#;

        let data = build_test_docx_with_parts(document_xml, None, Some(footnotes_xml), None, None, None);
        let extractor = DocxExtractor::new();
        let config = ExtractionConfig {
            output_format: crate::core::config::OutputFormat::Markdown,
            ..Default::default()
        };
        let result = extractor
            .extract_bytes(
                &data,
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                &config,
            )
            .await
            .unwrap();

        // Should have inline reference
        assert!(
            result.content.contains("[^2]"),
            "Should have footnote ref: {}",
            result.content
        );
        // Should have footnote definition
        assert!(
            result.content.contains("[^2]: This is the footnote content."),
            "Should have footnote def: {}",
            result.content
        );
        // Separator footnotes should be excluded
        assert!(!result.content.contains("separator"), "Separator should be filtered");
        assert!(
            !result.content.contains("continuation"),
            "Continuation should be filtered"
        );
    }

    #[tokio::test]
    async fn test_full_extraction_with_style_based_headings() {
        let document_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:pPr><w:pStyle w:val="CustomTitle"/></w:pPr><w:r><w:t>Custom Title</w:t></w:r></w:p>
    <w:p><w:r><w:t>Body text.</w:t></w:r></w:p>
  </w:body>
</w:document>"#;

        let styles_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:style w:type="paragraph" w:styleId="CustomTitle">
    <w:name w:val="Custom Title"/>
    <w:pPr><w:outlineLvl w:val="0"/></w:pPr>
  </w:style>
</w:styles>"#;

        let data = build_test_docx_with_parts(document_xml, Some(styles_xml), None, None, None, None);
        let extractor = DocxExtractor::new();
        let config = ExtractionConfig {
            output_format: crate::core::config::OutputFormat::Markdown,
            ..Default::default()
        };
        let result = extractor
            .extract_bytes(
                &data,
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                &config,
            )
            .await
            .unwrap();

        // outline_level 0 = h1
        assert!(
            result.content.contains("# Custom Title"),
            "Style-based heading: {}",
            result.content
        );
    }

    #[tokio::test]
    async fn test_document_structure_generation() {
        let document_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:pPr><w:pStyle w:val="Title"/></w:pPr><w:r><w:t>Doc Title</w:t></w:r></w:p>
    <w:p><w:r><w:t>A paragraph.</w:t></w:r></w:p>
    <w:tbl>
      <w:tr><w:tc><w:p><w:r><w:t>Cell 1</w:t></w:r></w:p></w:tc></w:tr>
    </w:tbl>
  </w:body>
</w:document>"#;

        let header_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:hdr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:p><w:r><w:t>Header</w:t></w:r></w:p>
</w:hdr>"#;

        let data = build_test_docx_with_parts(document_xml, None, None, None, Some(header_xml), None);
        let extractor = DocxExtractor::new();
        let config = ExtractionConfig {
            include_document_structure: true,
            ..Default::default()
        };
        let result = extractor
            .extract_bytes(
                &data,
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                &config,
            )
            .await
            .unwrap();

        // DocumentStructure should be populated
        assert!(result.document.is_some(), "DocumentStructure should be populated");
        let doc = result.document.unwrap();

        // Should have nodes
        assert!(!doc.nodes.is_empty(), "Should have document nodes");

        // Validate the structure
        assert!(doc.validate().is_ok(), "DocumentStructure should be valid");

        // Check for heading group
        use crate::types::NodeContent;
        let headings: Vec<_> = doc
            .nodes
            .iter()
            .filter(|n| matches!(n.content, NodeContent::Heading { .. }))
            .collect();
        assert!(!headings.is_empty(), "Should have heading nodes");

        // Check for paragraph
        let paragraphs: Vec<_> = doc
            .nodes
            .iter()
            .filter(|n| matches!(n.content, NodeContent::Paragraph { .. }))
            .collect();
        assert!(!paragraphs.is_empty(), "Should have paragraph nodes");

        // Check for table
        let tables: Vec<_> = doc
            .nodes
            .iter()
            .filter(|n| matches!(n.content, NodeContent::Table { .. }))
            .collect();
        assert!(!tables.is_empty(), "Should have table nodes");

        // Check for header content layer
        use crate::types::ContentLayer;
        let headers: Vec<_> = doc
            .nodes
            .iter()
            .filter(|n| n.content_layer == ContentLayer::Header)
            .collect();
        assert!(!headers.is_empty(), "Should have header nodes");
    }

    #[tokio::test]
    async fn test_pages_populated_single_page() {
        let document_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>Simple single page document.</w:t></w:r></w:p>
  </w:body>
</w:document>"#;

        let data = build_test_docx(document_xml);
        let extractor = DocxExtractor::new();
        let config = ExtractionConfig {
            output_format: crate::core::config::OutputFormat::Markdown,
            ..Default::default()
        };
        let result = extractor
            .extract_bytes(
                &data,
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                &config,
            )
            .await
            .unwrap();

        assert!(result.pages.is_some(), "Pages should be populated");
        let pages = result.pages.unwrap();
        assert_eq!(pages.len(), 1, "Single page document should have 1 page");
        assert_eq!(pages[0].page_number, 1);
        assert!(pages[0].content.contains("Simple single page document."));
    }

    #[test]
    fn test_build_document_structure_from_parsed_doc() {
        use crate::extraction::docx::parser::{
            Document, DocumentElement, HeaderFooter, Note, NoteType, Paragraph, Run, Table as DocxTable, TableCell,
            TableRow,
        };

        let mut doc = Document::new();

        // Add a heading paragraph
        let mut heading = Paragraph::new();
        heading.style = Some("Title".to_string());
        heading.add_run(Run::new("Test Title".to_string()));
        let h_idx = doc.paragraphs.len();
        doc.paragraphs.push(heading);
        doc.elements.push(DocumentElement::Paragraph(h_idx));

        // Add a body paragraph
        let mut body = Paragraph::new();
        body.add_run(Run::new("Body text.".to_string()));
        let b_idx = doc.paragraphs.len();
        doc.paragraphs.push(body);
        doc.elements.push(DocumentElement::Paragraph(b_idx));

        // Add a table
        let mut table = DocxTable::new();
        let mut row = TableRow::default();
        let mut cell = TableCell::default();
        let mut cell_para = Paragraph::new();
        cell_para.add_run(Run::new("Cell data".to_string()));
        cell.paragraphs.push(cell_para);
        row.cells.push(cell);
        table.rows.push(row);
        let t_idx = doc.tables.len();
        doc.tables.push(table);
        doc.elements.push(DocumentElement::Table(t_idx));

        // Add a header
        let mut header = HeaderFooter::default();
        let mut h_para = Paragraph::new();
        h_para.add_run(Run::new("Header content".to_string()));
        header.paragraphs.push(h_para);
        doc.headers.push(header);

        // Add a footnote
        doc.footnotes.push(Note {
            id: "2".to_string(),
            note_type: NoteType::Footnote,
            paragraphs: vec![{
                let mut p = Paragraph::new();
                p.add_run(Run::new("Footnote text".to_string()));
                p
            }],
        });

        let structure = build_document_structure(&doc);

        // Validate
        assert!(structure.validate().is_ok(), "Should be valid");
        assert!(!structure.nodes.is_empty(), "Should have nodes");

        // Check content layers
        use crate::types::ContentLayer;
        let body_nodes: Vec<_> = structure
            .nodes
            .iter()
            .filter(|n| n.content_layer == ContentLayer::Body)
            .collect();
        assert!(!body_nodes.is_empty(), "Should have body nodes");

        let header_nodes: Vec<_> = structure
            .nodes
            .iter()
            .filter(|n| n.content_layer == ContentLayer::Header)
            .collect();
        assert!(!header_nodes.is_empty(), "Should have header nodes");

        let footnote_nodes: Vec<_> = structure
            .nodes
            .iter()
            .filter(|n| n.content_layer == ContentLayer::Footnote)
            .collect();
        assert!(!footnote_nodes.is_empty(), "Should have footnote nodes");
    }

    #[tokio::test]
    async fn test_full_extraction_with_endnotes() {
        let document_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:r><w:t>Text with endnote</w:t></w:r>
      <w:r><w:endnoteReference w:id="2"/></w:r>
    </w:p>
  </w:body>
</w:document>"#;

        let endnotes_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:endnotes xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:endnote w:id="0"><w:p><w:r><w:t>separator</w:t></w:r></w:p></w:endnote>
  <w:endnote w:id="1"><w:p><w:r><w:t>continuation</w:t></w:r></w:p></w:endnote>
  <w:endnote w:id="2"><w:p><w:r><w:t>This is the endnote.</w:t></w:r></w:p></w:endnote>
</w:endnotes>"#;

        let data = build_test_docx_with_parts(document_xml, None, None, Some(endnotes_xml), None, None);
        let extractor = DocxExtractor::new();
        let config = ExtractionConfig {
            output_format: crate::core::config::OutputFormat::Markdown,
            ..Default::default()
        };
        let result = extractor
            .extract_bytes(
                &data,
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                &config,
            )
            .await
            .unwrap();

        // Should have inline reference
        assert!(
            result.content.contains("[^2]"),
            "Should have endnote ref: {}",
            result.content
        );
        // Should have endnote definition
        assert!(
            result.content.contains("[^2]: This is the endnote."),
            "Should have endnote def: {}",
            result.content
        );
        // Separator endnotes should be excluded
        assert!(!result.content.contains("separator"), "Separator should be filtered");
    }

    #[tokio::test]
    async fn test_typed_metadata_fields_populated() {
        use std::io::Write;
        let buf = Vec::new();
        let cursor = std::io::Cursor::new(buf);
        let mut zip = zip::ZipWriter::new(cursor);
        let options: zip::write::FileOptions<()> = zip::write::FileOptions::default();

        // Content types
        zip.start_file("[Content_Types].xml", options).unwrap();
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#).unwrap();

        // Document
        zip.start_file("word/document.xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body><w:p><w:r><w:t>Content</w:t></w:r></w:p></w:body>
</w:document>"#,
        )
        .unwrap();

        // Core properties with title, creator, subject, dates
        zip.start_file("docProps/core.xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties"
                   xmlns:dc="http://purl.org/dc/elements/1.1/"
                   xmlns:dcterms="http://purl.org/dc/terms/">
  <dc:title>My Document</dc:title>
  <dc:creator>Jane Doe</dc:creator>
  <dc:subject>Test Subject</dc:subject>
  <cp:lastModifiedBy>John Smith</cp:lastModifiedBy>
  <dcterms:created>2024-01-15T10:30:00Z</dcterms:created>
  <dcterms:modified>2024-02-20T14:45:00Z</dcterms:modified>
  <dc:language>en-US</dc:language>
</cp:coreProperties>"#,
        )
        .unwrap();

        let data = zip.finish().unwrap().into_inner();

        let extractor = DocxExtractor::new();
        let config = ExtractionConfig {
            output_format: crate::core::config::OutputFormat::Markdown,
            ..Default::default()
        };
        let result = extractor
            .extract_bytes(
                &data,
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                &config,
            )
            .await
            .unwrap();

        // Verify typed metadata fields
        assert_eq!(result.metadata.title.as_deref(), Some("My Document"));
        assert_eq!(result.metadata.subject.as_deref(), Some("Test Subject"));
        assert_eq!(result.metadata.authors, Some(vec!["Jane Doe".to_string()]));
        assert_eq!(result.metadata.created_by.as_deref(), Some("Jane Doe"));
        assert_eq!(result.metadata.modified_by.as_deref(), Some("John Smith"));
        assert_eq!(result.metadata.created_at.as_deref(), Some("2024-01-15T10:30:00Z"));
        assert_eq!(result.metadata.modified_at.as_deref(), Some("2024-02-20T14:45:00Z"));
        assert_eq!(result.metadata.language.as_deref(), Some("en-US"));

        // Verify these are NOT duplicated in additional map
        assert!(
            result.metadata.additional.get("title").is_none(),
            "title should not be in additional"
        );
        assert!(
            result.metadata.additional.get("created_by").is_none(),
            "created_by should not be in additional"
        );
    }

    #[tokio::test]
    async fn test_images_none_when_extraction_disabled() {
        let document_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body><w:p><w:r><w:t>No images.</w:t></w:r></w:p></w:body>
</w:document>"#;

        let data = build_test_docx(document_xml);
        let extractor = DocxExtractor::new();
        let config = ExtractionConfig::default(); // images not enabled by default
        let result = extractor
            .extract_bytes(
                &data,
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                &config,
            )
            .await
            .unwrap();

        assert!(
            result.images.is_none(),
            "Images should be None when extraction is disabled"
        );
    }

    #[test]
    fn test_vertical_merge_renders_empty_cells() {
        use crate::extraction::docx::parser::{Paragraph, Run, Table as DocxTable, TableCell, TableRow};
        use crate::extraction::docx::table::{CellProperties, RowProperties, VerticalMerge};

        let mut table = DocxTable::new();

        // Row 1: header with "Name" | "Score" (v_merge Restart)
        let mut row1 = TableRow {
            properties: Some(RowProperties {
                is_header: true,
                ..Default::default()
            }),
            ..Default::default()
        };

        let mut cell1 = TableCell::default();
        let mut p1 = Paragraph::new();
        p1.add_run(Run::new("Name".to_string()));
        cell1.paragraphs.push(p1);
        row1.cells.push(cell1);

        let mut cell2 = TableCell {
            properties: Some(CellProperties {
                v_merge: Some(VerticalMerge::Restart),
                ..Default::default()
            }),
            ..Default::default()
        };
        let mut p2 = Paragraph::new();
        p2.add_run(Run::new("Score".to_string()));
        cell2.paragraphs.push(p2);
        row1.cells.push(cell2);
        table.rows.push(row1);

        // Row 2: "Alice" | v_merge Continue (should render empty)
        let mut row2 = TableRow::default();
        let mut cell3 = TableCell::default();
        let mut p3 = Paragraph::new();
        p3.add_run(Run::new("Alice".to_string()));
        cell3.paragraphs.push(p3);
        row2.cells.push(cell3);

        let mut cell4 = TableCell {
            properties: Some(CellProperties {
                v_merge: Some(VerticalMerge::Continue),
                ..Default::default()
            }),
            ..Default::default()
        };
        let mut p4 = Paragraph::new();
        p4.add_run(Run::new("Should be hidden".to_string()));
        cell4.paragraphs.push(p4);
        row2.cells.push(cell4);
        table.rows.push(row2);

        let md = table.to_markdown();
        // The v_merge=Continue cell should be empty
        assert!(md.contains("Score"), "Restart cell should show content");
        assert!(
            !md.contains("Should be hidden"),
            "Continue cell should be empty: {}",
            md
        );
        assert!(md.contains("Alice"), "Normal cell should show content");
    }

    #[tokio::test]
    async fn test_drawing_image_placeholder_in_markdown() {
        use crate::extraction::docx::drawing::{DocProperties, Drawing, DrawingType};
        use crate::extraction::docx::parser::{Document, DocumentElement, Paragraph, Run};

        let mut doc = Document::new();

        // Add a paragraph
        let mut para = Paragraph::new();
        para.add_run(Run::new("Before image.".to_string()));
        let p_idx = doc.paragraphs.len();
        doc.paragraphs.push(para);
        doc.elements.push(DocumentElement::Paragraph(p_idx));

        // Add a drawing with description
        let drawing = Drawing {
            drawing_type: DrawingType::Inline,
            extent: None,
            doc_properties: Some(DocProperties {
                id: Some("1".to_string()),
                name: Some("Picture 1".to_string()),
                description: Some("A test image".to_string()),
            }),
            image_ref: None,
        };
        let d_idx = doc.drawings.len();
        doc.drawings.push(drawing);
        doc.elements.push(DocumentElement::Drawing(d_idx));

        // Add another paragraph
        let mut para2 = Paragraph::new();
        para2.add_run(Run::new("After image.".to_string()));
        let p2_idx = doc.paragraphs.len();
        doc.paragraphs.push(para2);
        doc.elements.push(DocumentElement::Paragraph(p2_idx));

        let md = doc.to_markdown();
        assert!(
            md.contains("![A test image](image)"),
            "Should have image placeholder: {}",
            md
        );
        assert!(md.contains("Before image."), "Should have text before");
        assert!(md.contains("After image."), "Should have text after");
    }

    #[tokio::test]
    async fn test_docx_metadata_format_field() {
        use std::io::Write;
        let buf = Vec::new();
        let cursor = std::io::Cursor::new(buf);
        let mut zip = zip::ZipWriter::new(cursor);
        let options: zip::write::FileOptions<()> = zip::write::FileOptions::default();

        zip.start_file("[Content_Types].xml", options).unwrap();
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#).unwrap();

        zip.start_file("word/document.xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body><w:p><w:r><w:t>Content</w:t></w:r></w:p></w:body>
</w:document>"#,
        )
        .unwrap();

        zip.start_file("docProps/core.xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties"
                   xmlns:dc="http://purl.org/dc/elements/1.1/">
  <dc:title>Format Test</dc:title>
</cp:coreProperties>"#,
        )
        .unwrap();

        zip.start_file("docProps/app.xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties">
  <Pages>3</Pages>
  <Words>500</Words>
</Properties>"#,
        )
        .unwrap();

        let data = zip.finish().unwrap().into_inner();

        let extractor = DocxExtractor::new();
        let config = ExtractionConfig {
            output_format: crate::core::config::OutputFormat::Markdown,
            ..Default::default()
        };
        let result = extractor
            .extract_bytes(
                &data,
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                &config,
            )
            .await
            .unwrap();

        // Verify DocxMetadata format field
        assert!(result.metadata.format.is_some(), "Format should be populated");
        match result.metadata.format.as_ref().unwrap() {
            FormatMetadata::Docx(docx_meta) => {
                assert!(docx_meta.core_properties.is_some(), "Core properties should be present");
                let core = docx_meta.core_properties.as_ref().unwrap();
                assert_eq!(core.title.as_deref(), Some("Format Test"));

                assert!(docx_meta.app_properties.is_some(), "App properties should be present");
                let app = docx_meta.app_properties.as_ref().unwrap();
                assert_eq!(app.pages, Some(3));
                assert_eq!(app.words, Some(500));
            }
            _ => panic!("Expected FormatMetadata::Docx"),
        }
    }
}
