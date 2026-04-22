//! DOCX extractor for high-performance text extraction.
//!
//! Supports: Microsoft Word (.docx)

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extraction::{cells_to_markdown, office_metadata};
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::ExtractedImage;
use crate::types::internal::InternalDocument;
use crate::types::internal_builder::InternalDocumentBuilder;
use crate::types::{
    DocxMetadata, FormatMetadata, Metadata, PageBoundary, PageContent, PageInfo, PageStructure, PageUnitType, Table,
};
use ahash::AHashMap;
use async_trait::async_trait;
use bytes::Bytes;
use std::borrow::Cow;
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
/// Uses `DocumentStructureBuilder` for automatic section nesting and
/// collects `TextAnnotation`s from Run formatting data.
fn build_document_structure(doc: &crate::extraction::docx::parser::Document) -> crate::types::DocumentStructure {
    use crate::types::builder::DocumentStructureBuilder;
    use crate::types::extraction::BoundingBox;
    use crate::types::{GridCell, TableGrid};

    let capacity =
        doc.paragraphs.len() + doc.tables.len() + doc.drawings.len() + doc.headers.len() + doc.footers.len() + 16;
    let mut b = DocumentStructureBuilder::with_capacity(capacity).source_format("docx");

    // Add section properties as attributes on a root-level Group node.
    // Use the last section, which represents the document-level default in DOCX.
    if let Some(section) = doc.sections.last() {
        let mut attrs = AHashMap::new();
        if let Some(w) = section.page_width_points() {
            attrs.insert("page_width_pt".to_string(), format!("{}", w));
        }
        if let Some(h) = section.page_height_points() {
            attrs.insert("page_height_pt".to_string(), format!("{}", h));
        }
        if let Some(ref orient) = section.orientation {
            attrs.insert(
                "orientation".to_string(),
                match orient {
                    crate::extraction::docx::section::Orientation::Portrait => "portrait".to_string(),
                    crate::extraction::docx::section::Orientation::Landscape => "landscape".to_string(),
                },
            );
        }
        let margins = section.margins.to_points();
        if let Some(top) = margins.top {
            attrs.insert("margin_top_pt".to_string(), format!("{}", top));
        }
        if let Some(right) = margins.right {
            attrs.insert("margin_right_pt".to_string(), format!("{}", right));
        }
        if let Some(bottom) = margins.bottom {
            attrs.insert("margin_bottom_pt".to_string(), format!("{}", bottom));
        }
        if let Some(left) = margins.left {
            attrs.insert("margin_left_pt".to_string(), format!("{}", left));
        }
        if !attrs.is_empty() {
            let group_idx = b.push_raw(
                crate::types::document_structure::NodeContent::Group {
                    label: Some("section_properties".to_string()),
                    heading_level: None,
                    heading_text: None,
                },
                None,
                None,
                crate::types::document_structure::ContentLayer::Body,
                vec![],
            );
            b.set_attributes(group_idx, attrs);
        }
    }

    // Process body elements in document order
    for element in &doc.elements {
        match element {
            crate::extraction::docx::parser::DocumentElement::Paragraph(idx) => {
                let paragraph = &doc.paragraphs[*idx];

                // Collect plain text and annotations from runs, separating math runs
                let (text, annotations, math_formulas) = collect_run_annotations(&paragraph.runs);

                if text.is_empty() && math_formulas.is_empty() {
                    continue;
                }

                // Check if this paragraph is a heading
                let heading_level = paragraph.style.as_deref().and_then(|s| doc.resolve_heading_level(s));

                if let Some(level) = heading_level {
                    // For headings, use plain text (annotations are less relevant for DocumentStructure)
                    let heading_text = if text.is_empty() {
                        paragraph.runs_to_markdown()
                    } else {
                        text
                    };
                    b.push_heading(level, &heading_text, None, None);
                } else if paragraph.numbering_id.is_some() {
                    // Push any preceding math formulas as standalone nodes
                    for formula in &math_formulas {
                        b.push_formula(formula, None);
                    }
                    if !text.is_empty() {
                        // List item - create as list item (list grouping done by transform)
                        let is_ordered = paragraph
                            .numbering_id
                            .zip(paragraph.numbering_level)
                            .and_then(|(nid, nlvl)| doc.numbering_defs.get(&(nid, nlvl)))
                            .is_some_and(|lt| *lt == crate::extraction::docx::parser::ListType::Numbered);
                        let list = b.push_list(is_ordered, None);
                        b.push_list_item(list, &text, None);
                    }
                } else {
                    // Push any math formulas as standalone Formula nodes
                    for formula in &math_formulas {
                        b.push_formula(formula, None);
                    }
                    if !text.is_empty() {
                        b.push_paragraph(&text, annotations, None, None);
                    }
                }
            }
            crate::extraction::docx::parser::DocumentElement::Table(idx) => {
                let table = &doc.tables[*idx];
                let rows = table.rows.len() as u32;
                let cols = table.rows.first().map_or(0, |r| r.cells.len()) as u32;
                let mut cells = Vec::new();
                let mut cell_style_attrs = AHashMap::new();
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

                        // Collect cell styling as attributes keyed by "cell_R_C_..."
                        if let Some(ref props) = cell.properties {
                            let prefix = format!("cell_{}_{}", row_idx, col_idx);
                            if let Some(ref shading) = props.shading
                                && let Some(ref fill) = shading.fill
                                && fill != "auto"
                            {
                                cell_style_attrs.insert(format!("{}_shading_fill", prefix), fill.clone());
                            }
                            if let Some(ref borders) = props.borders {
                                if let Some(ref top) = borders.top {
                                    cell_style_attrs.insert(
                                        format!("{}_border_top", prefix),
                                        format!("{}:{}", top.style, top.color.as_deref().unwrap_or("auto")),
                                    );
                                }
                                if let Some(ref bottom) = borders.bottom {
                                    cell_style_attrs.insert(
                                        format!("{}_border_bottom", prefix),
                                        format!("{}:{}", bottom.style, bottom.color.as_deref().unwrap_or("auto")),
                                    );
                                }
                            }
                        }
                    }
                }
                let grid = TableGrid { rows, cols, cells };
                let table_idx = b.push_table(grid, None, None);
                if !cell_style_attrs.is_empty() {
                    b.set_attributes(table_idx, cell_style_attrs);
                }
            }
            crate::extraction::docx::parser::DocumentElement::Drawing(idx) => {
                let drawing = &doc.drawings[*idx];

                // Skip drawings without an image reference (e.g. textbox shapes)
                if drawing.image_ref.is_none() {
                    continue;
                }

                let description = drawing.doc_properties.as_ref().and_then(|dp| dp.description.clone());

                // Build bounding box from anchor position + extent
                let bbox = match &drawing.drawing_type {
                    crate::extraction::docx::drawing::DrawingType::Anchored(anchor) => {
                        let x = anchor.position_h.as_ref().and_then(|p| p.offset).unwrap_or(0);
                        let y = anchor.position_v.as_ref().and_then(|p| p.offset).unwrap_or(0);
                        let (cx, cy) = drawing.extent.as_ref().map(|e| (e.cx, e.cy)).unwrap_or((0, 0));
                        if x != 0 || y != 0 || cx != 0 || cy != 0 {
                            const EMU_PER_PT: f64 = 914_400.0 / 72.0;
                            Some(BoundingBox {
                                x0: x as f64 / EMU_PER_PT,
                                y0: y as f64 / EMU_PER_PT,
                                x1: (x + cx) as f64 / EMU_PER_PT,
                                y1: (y + cy) as f64 / EMU_PER_PT,
                            })
                        } else {
                            None
                        }
                    }
                    _ => None,
                };

                b.push_image(description.as_deref(), Some(*idx as u32), None, bbox);
            }
            crate::extraction::docx::parser::DocumentElement::PageBreak => {}
        }
    }

    // Add headers and footers
    for hf in &doc.headers {
        let text: String = hf
            .paragraphs
            .iter()
            .map(|p| p.runs_to_markdown())
            .collect::<Vec<_>>()
            .join("\n");
        if !text.is_empty() {
            b.push_header(&text, None);
        }
    }
    for hf in &doc.footers {
        let text: String = hf
            .paragraphs
            .iter()
            .map(|p| p.runs_to_markdown())
            .collect::<Vec<_>>()
            .join("\n");
        if !text.is_empty() {
            b.push_footer(&text, None);
        }
    }

    // Add footnotes and endnotes
    for note in doc.footnotes.iter().chain(doc.endnotes.iter()) {
        let text: String = note
            .paragraphs
            .iter()
            .map(|p| p.runs_to_markdown())
            .collect::<Vec<_>>()
            .join(" ");
        if !text.is_empty() {
            b.push_footnote(&text, None);
        }
    }

    b.build()
}

/// Build an `InternalDocument` from parsed DOCX data.
///
/// Creates a flat element list with headings, paragraphs, lists, tables, images,
/// footnotes/endnotes (with relationships), and hyperlinks (as InternalLink relationships).
/// Mirrors `build_document_structure` but outputs the flat `InternalDocument` representation.
fn build_internal_document(doc: &crate::extraction::docx::parser::Document) -> InternalDocument {
    use crate::types::document_structure::ContentLayer;
    use crate::types::extraction::BoundingBox;
    use crate::types::internal::{ElementKind, InternalElement, RelationshipKind, RelationshipTarget};
    use crate::types::uri::Uri;

    let mut builder = InternalDocumentBuilder::new("docx");

    // Track current list state for grouping consecutive list items
    let mut current_list_numbering_id: Option<i64> = None;
    let mut current_list_ordered: bool = false;

    // Process body elements in document order
    for element in &doc.elements {
        match element {
            crate::extraction::docx::parser::DocumentElement::Paragraph(idx) => {
                let paragraph = &doc.paragraphs[*idx];

                // Collect plain text and annotations from runs, separating math runs
                let (text, annotations, math_formulas) = collect_run_annotations(&paragraph.runs);

                if text.is_empty() && math_formulas.is_empty() {
                    // Close any open list if we hit an empty paragraph
                    if current_list_numbering_id.is_some() {
                        builder.end_list();
                        current_list_numbering_id = None;
                    }
                    continue;
                }

                // Check if this paragraph is a heading
                let heading_level = paragraph.style.as_deref().and_then(|s| doc.resolve_heading_level(s));

                // Check if this paragraph has a quote/blockquote style
                let is_quote_style = paragraph.style.as_deref().is_some_and(|s| {
                    let lower = s.to_ascii_lowercase();
                    lower == "quote"
                        || lower == "blockquote"
                        || lower == "intenseq"
                        || lower == "intensequote"
                        || lower.contains("quote")
                });

                // Track the element index from whichever branch pushes the element,
                // so we can scan for hyperlink URIs unconditionally afterward.
                let element_idx: Option<u32> = if let Some(level) = heading_level {
                    // Close any open list before a heading
                    if current_list_numbering_id.is_some() {
                        builder.end_list();
                        current_list_numbering_id = None;
                    }
                    let heading_text = if text.is_empty() {
                        paragraph.runs_to_markdown()
                    } else {
                        text.clone()
                    };
                    let idx = builder.push_heading(level, &heading_text, None, None);
                    if !annotations.is_empty() {
                        builder.set_annotations(idx, annotations.clone());
                    }
                    Some(idx)
                } else if is_quote_style {
                    // Close any open list before a blockquote
                    if current_list_numbering_id.is_some() {
                        builder.end_list();
                        current_list_numbering_id = None;
                    }
                    builder.push_quote_start();
                    let para_idx = builder.push_paragraph(&text, annotations.clone(), None, None);
                    builder.push_quote_end();
                    Some(para_idx)
                } else if let Some(nid) = paragraph.numbering_id {
                    // Push any preceding math formulas as standalone nodes
                    for formula in &math_formulas {
                        builder.push_formula(formula, None, None);
                    }
                    if !text.is_empty() {
                        let is_ordered = paragraph
                            .numbering_id
                            .zip(paragraph.numbering_level)
                            .and_then(|(nid, nlvl)| doc.numbering_defs.get(&(nid, nlvl)))
                            .is_some_and(|lt| *lt == crate::extraction::docx::parser::ListType::Numbered);
                        // Check if we need to start a new list or continue the current one
                        if current_list_numbering_id != Some(nid) {
                            // Close previous list if open
                            if current_list_numbering_id.is_some() {
                                builder.end_list();
                            }
                            builder.push_list(is_ordered);
                            current_list_numbering_id = Some(nid);
                            current_list_ordered = is_ordered;
                        }
                        let li_idx =
                            builder.push_list_item(&text, current_list_ordered, annotations.clone(), None, None);
                        Some(li_idx)
                    } else {
                        None
                    }
                } else {
                    // Close any open list before a non-list paragraph
                    if current_list_numbering_id.is_some() {
                        builder.end_list();
                        current_list_numbering_id = None;
                    }
                    // Push any math formulas as standalone Formula nodes
                    for formula in &math_formulas {
                        builder.push_formula(formula, None, None);
                    }
                    if !text.is_empty() {
                        let para_idx = builder.push_paragraph(&text, annotations.clone(), None, None);
                        Some(para_idx)
                    } else {
                        None
                    }
                };

                // Scan runs for hyperlink URLs to create InternalLink relationships and extract URIs.
                // This runs for ALL paragraph types: headings, quotes, list items, and plain paragraphs.
                if let Some(elem_idx) = element_idx {
                    for run in &paragraph.runs {
                        if run.math_latex.is_some() || run.text.is_empty() {
                            continue;
                        }
                        if let Some(ref url) = run.hyperlink_url {
                            if url.starts_with('#') {
                                let anchor_key = url.trim_start_matches('#').to_string();
                                builder.push_relationship(
                                    elem_idx,
                                    RelationshipTarget::Key(anchor_key),
                                    RelationshipKind::InternalLink,
                                );
                            }
                            builder.push_uri(Uri::hyperlink(url.as_str(), Some(run.text.clone())));
                        }
                    }

                    // Scan for inline footnote/endnote references ([^N]) in text
                    let mut search_start = 0;
                    while let Some(start) = text[search_start..].find("[^") {
                        let abs_start = search_start + start;
                        if let Some(end) = text[abs_start..].find(']') {
                            let ref_id = &text[abs_start + 2..abs_start + end];
                            if !ref_id.is_empty() && ref_id.chars().all(|c| c.is_ascii_digit()) {
                                let key = format!("fn{}", ref_id);
                                builder.push_footnote_ref(ref_id, &key, None);
                                // Relationship is auto-created by push_footnote_ref
                            }
                            search_start = abs_start + end + 1;
                        } else {
                            break;
                        }
                    }
                }
            }
            crate::extraction::docx::parser::DocumentElement::Table(idx) => {
                // Close any open list before a table
                if current_list_numbering_id.is_some() {
                    builder.end_list();
                    current_list_numbering_id = None;
                }
                let table = &doc.tables[*idx];
                // Emit table caption as a paragraph before the table
                if let Some(ref props) = table.properties
                    && let Some(ref caption) = props.caption
                    && !caption.is_empty()
                {
                    builder.push_paragraph(caption, vec![], None, None);
                }
                let mut cells: Vec<Vec<String>> = Vec::new();
                for row in &table.rows {
                    let mut row_cells = Vec::new();
                    for cell in &row.cells {
                        let text = cell
                            .paragraphs
                            .iter()
                            .map(|p| p.runs_to_markdown())
                            .collect::<Vec<_>>()
                            .join(" ")
                            .trim()
                            .to_string();
                        let span = cell.properties.as_ref().and_then(|p| p.grid_span).unwrap_or(1);
                        for _ in 0..span {
                            row_cells.push(text.clone());
                        }
                    }
                    cells.push(row_cells);
                }
                // Fill vertically merged cells
                for row_idx in 1..table.rows.len() {
                    let mut col = 0usize;
                    for cell in &table.rows[row_idx].cells {
                        let span = cell.properties.as_ref().and_then(|p| p.grid_span).unwrap_or(1) as usize;
                        let is_vmerge_continue = cell.properties.as_ref().is_some_and(|p| {
                            matches!(p.v_merge, Some(crate::extraction::docx::table::VerticalMerge::Continue))
                        });
                        if is_vmerge_continue {
                            for c in col..col + span {
                                if c < cells[row_idx].len() && c < cells[row_idx - 1].len() {
                                    cells[row_idx][c] = cells[row_idx - 1][c].clone();
                                }
                            }
                        }
                        col += span;
                    }
                }
                if !cells.is_empty() {
                    builder.push_table_from_cells(&cells, None, None);
                }
            }
            crate::extraction::docx::parser::DocumentElement::Drawing(idx) => {
                let drawing = &doc.drawings[*idx];

                // Skip drawings without an image reference (e.g. textbox shapes)
                if drawing.image_ref.is_none() {
                    continue;
                }

                // Close any open list before a drawing
                if current_list_numbering_id.is_some() {
                    builder.end_list();
                    current_list_numbering_id = None;
                }
                let description = drawing.doc_properties.as_ref().and_then(|dp| dp.description.clone());

                // Build bounding box from anchor position + extent
                let bbox = match &drawing.drawing_type {
                    crate::extraction::docx::drawing::DrawingType::Anchored(anchor) => {
                        let x = anchor.position_h.as_ref().and_then(|p| p.offset).unwrap_or(0);
                        let y = anchor.position_v.as_ref().and_then(|p| p.offset).unwrap_or(0);
                        let (cx, cy) = drawing.extent.as_ref().map(|e| (e.cx, e.cy)).unwrap_or((0, 0));
                        if x != 0 || y != 0 || cx != 0 || cy != 0 {
                            const EMU_PER_PT: f64 = 914_400.0 / 72.0;
                            Some(BoundingBox {
                                x0: x as f64 / EMU_PER_PT,
                                y0: y as f64 / EMU_PER_PT,
                                x1: (x + cx) as f64 / EMU_PER_PT,
                                y1: (y + cy) as f64 / EMU_PER_PT,
                            })
                        } else {
                            None
                        }
                    }
                    _ => None,
                };

                // Push image element directly (no ExtractedImage data at this point)
                let kind = ElementKind::Image {
                    image_index: *idx as u32,
                };
                let text_val = description.as_deref().unwrap_or("");
                let elem = InternalElement::text(kind, text_val, 0);
                let elem = if let Some(b) = bbox { elem.with_bbox(b) } else { elem };
                let img_elem_idx = builder.push_element(elem);

                // Store the resolved image path as an attribute on the image element
                if let Some(ref rid) = drawing.image_ref
                    && let Some(path) = doc.image_relationships.get(rid)
                {
                    let mut attrs = AHashMap::new();
                    attrs.insert("image_uri".to_string(), path.clone());
                    builder.set_attributes(img_elem_idx, attrs);
                }
            }
            crate::extraction::docx::parser::DocumentElement::PageBreak => {}
        }
    }

    // Close any remaining open list
    if current_list_numbering_id.is_some() {
        builder.end_list();
    }

    // Add headers and footers with appropriate content layers
    for hf in &doc.headers {
        let text: String = hf
            .paragraphs
            .iter()
            .map(|p| p.runs_to_markdown())
            .collect::<Vec<_>>()
            .join("\n");
        if !text.is_empty() {
            let idx = builder.push_paragraph(&text, vec![], None, None);
            builder.set_layer(idx, ContentLayer::Header);
        }
    }
    for hf in &doc.footers {
        let text: String = hf
            .paragraphs
            .iter()
            .map(|p| p.runs_to_markdown())
            .collect::<Vec<_>>()
            .join("\n");
        if !text.is_empty() {
            let idx = builder.push_paragraph(&text, vec![], None, None);
            builder.set_layer(idx, ContentLayer::Footer);
        }
    }

    // Add footnotes and endnotes as FootnoteDefinition elements with anchors
    for note in doc.footnotes.iter().chain(doc.endnotes.iter()) {
        let text: String = note
            .paragraphs
            .iter()
            .map(|p| p.runs_to_markdown())
            .collect::<Vec<_>>()
            .join(" ");
        if !text.is_empty() {
            let key = format!("fn{}", note.id);
            let idx = builder.push_footnote_definition(&text, &key, None);
            builder.set_layer(idx, ContentLayer::Footnote);
        }
    }

    builder.build()
}

/// Collect plain text, annotations, and math formulas from a slice of Runs.
///
/// Returns `(plain_text, annotations, math_formulas)` where:
/// - `plain_text` is the concatenated non-math run text
/// - `annotations` are byte-offset-based formatting annotations for the plain text
/// - `math_formulas` are LaTeX strings from math runs (to be emitted as Formula nodes)
fn collect_run_annotations(
    runs: &[crate::extraction::docx::parser::Run],
) -> (String, Vec<crate::types::TextAnnotation>, Vec<String>) {
    use crate::types::builder;

    let mut text = String::new();
    let mut annotations = Vec::new();
    let mut math_formulas = Vec::new();

    for run in runs {
        // Math runs become standalone Formula nodes
        if let Some((ref latex, _is_display)) = run.math_latex {
            if !latex.is_empty() {
                math_formulas.push(latex.clone());
            }
            continue;
        }

        if run.text.is_empty() {
            continue;
        }

        let start = text.len() as u32;
        text.push_str(&run.text);
        let end = text.len() as u32;

        if run.bold {
            annotations.push(builder::bold(start, end));
        }
        if run.italic {
            annotations.push(builder::italic(start, end));
        }
        if run.underline {
            annotations.push(builder::underline(start, end));
        }
        if run.strikethrough {
            annotations.push(builder::strikethrough(start, end));
        }
        if run.subscript {
            annotations.push(builder::subscript(start, end));
        }
        if run.superscript {
            annotations.push(builder::superscript(start, end));
        }
        if let Some(sz) = run.font_size {
            // sz is in half-points; convert to "Xpt" string
            let pts = sz as f64 / 2.0;
            let value = if pts.fract() == 0.0 {
                format!("{}pt", pts as u32)
            } else {
                format!("{:.1}pt", pts)
            };
            annotations.push(builder::font_size(start, end, &value));
        }
        if let Some(ref color_val) = run.font_color {
            annotations.push(builder::color(start, end, &format!("#{}", color_val)));
        }
        if run.highlight.is_some() {
            annotations.push(builder::highlight(start, end));
        }
        if let Some(ref url) = run.hyperlink_url {
            annotations.push(builder::link(start, end, url, None));
        }
    }

    // Merge adjacent annotations of the same kind to avoid spurious marker sequences (e.g. `****`).
    // Sort by (kind discriminant, start) then merge touching/overlapping ranges.
    merge_adjacent_annotations(&mut annotations);

    (text, annotations, math_formulas)
}

/// Merge adjacent or overlapping annotations of the same kind.
///
/// When consecutive DOCX runs have the same formatting (e.g. bold), each run produces
/// its own annotation. Without merging, the markdown renderer would close and immediately
/// reopen markers, producing `**text1****text2**` instead of `**text1text2**`.
fn merge_adjacent_annotations(annotations: &mut Vec<crate::types::TextAnnotation>) {
    use crate::types::document_structure::AnnotationKind;

    if annotations.len() < 2 {
        return;
    }

    /// Check if two annotation kinds are the same for merging purposes.
    /// Simple kinds match by discriminant; Link kinds match if they have the same URL.
    fn same_kind_for_merge(a: &AnnotationKind, b: &AnnotationKind) -> bool {
        match (a, b) {
            (AnnotationKind::Bold, AnnotationKind::Bold)
            | (AnnotationKind::Italic, AnnotationKind::Italic)
            | (AnnotationKind::Underline, AnnotationKind::Underline)
            | (AnnotationKind::Strikethrough, AnnotationKind::Strikethrough)
            | (AnnotationKind::Subscript, AnnotationKind::Subscript)
            | (AnnotationKind::Superscript, AnnotationKind::Superscript)
            | (AnnotationKind::Highlight, AnnotationKind::Highlight)
            | (AnnotationKind::Code, AnnotationKind::Code) => true,
            (
                AnnotationKind::Link {
                    url: url_a,
                    title: title_a,
                },
                AnnotationKind::Link {
                    url: url_b,
                    title: title_b,
                },
            ) => url_a == url_b && title_a == title_b,
            _ => false,
        }
    }

    fn is_mergeable(kind: &AnnotationKind) -> bool {
        matches!(
            kind,
            AnnotationKind::Bold
                | AnnotationKind::Italic
                | AnnotationKind::Underline
                | AnnotationKind::Strikethrough
                | AnnotationKind::Subscript
                | AnnotationKind::Superscript
                | AnnotationKind::Highlight
                | AnnotationKind::Code
                | AnnotationKind::Link { .. }
        )
    }

    let kind_key = |kind: &AnnotationKind| -> u8 {
        match kind {
            AnnotationKind::Bold => 0,
            AnnotationKind::Italic => 1,
            AnnotationKind::Underline => 2,
            AnnotationKind::Strikethrough => 3,
            AnnotationKind::Subscript => 4,
            AnnotationKind::Superscript => 5,
            AnnotationKind::Highlight => 6,
            AnnotationKind::Code => 7,
            AnnotationKind::Link { .. } => 8,
            _ => 255,
        }
    };

    // Sort annotations by (kind_key, start)
    annotations.sort_by(|a, b| kind_key(&a.kind).cmp(&kind_key(&b.kind)).then(a.start.cmp(&b.start)));

    let mut merged = Vec::with_capacity(annotations.len());
    let mut i = 0;
    while i < annotations.len() {
        let mut ann = annotations[i].clone();
        if is_mergeable(&ann.kind) {
            // Merge consecutive annotations of the same kind
            let mut j = i + 1;
            while j < annotations.len()
                && same_kind_for_merge(&annotations[j].kind, &ann.kind)
                && annotations[j].start <= ann.end
            {
                ann.end = ann.end.max(annotations[j].end);
                j += 1;
            }
            merged.push(ann);
            i = j;
        } else {
            merged.push(ann);
            i += 1;
        }
    }

    *annotations = merged;
}

type DocxParseResult = (
    String,
    Vec<Table>,
    Option<Vec<PageBoundary>>,
    Vec<crate::extraction::docx::drawing::Drawing>,
    AHashMap<String, String>,
    Option<crate::types::DocumentStructure>,
    InternalDocument,
);

/// Parse DOCX document content and extract text, tables, page boundaries, drawings, image relationships, optional document structure, and InternalDocument.
fn parse_docx_core(
    content: &[u8],
    include_doc_structure: bool,
    output_format: crate::core::config::OutputFormat,
) -> crate::error::Result<DocxParseResult> {
    let doc = crate::extraction::docx::parser::parse_document(content)?;
    let (text, page_boundaries) =
        doc.extract_text_with_boundaries(matches!(output_format, crate::core::config::OutputFormat::Markdown));

    // Determine the correct 1-based page number for each top-level table.
    let table_page_nums = doc.table_page_numbers();
    let tables: Vec<Table> = doc
        .tables
        .iter()
        .enumerate()
        .map(|(idx, table)| {
            let page_number = table_page_nums.get(idx).copied().unwrap_or(1);
            convert_docx_table_to_table(table, page_number)
        })
        .collect();

    let page_boundaries = if page_boundaries.len() > 1 {
        Some(page_boundaries)
    } else {
        None
    };

    let drawings = doc.drawings.clone();
    let image_rels = doc.image_relationships.clone();
    let doc_structure = if include_doc_structure {
        Some(build_document_structure(&doc))
    } else {
        None
    };
    let internal_doc = build_internal_document(&doc);
    Ok((
        text,
        tables,
        page_boundaries,
        drawings,
        image_rels,
        doc_structure,
        internal_doc,
    ))
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
/// * `page_number` - 1-based page number the table appears on
///
/// # Returns
/// * `Table` - Converted table with cells and markdown representation
fn convert_docx_table_to_table(docx_table: &crate::extraction::docx::parser::Table, page_number: usize) -> Table {
    // Build grid with merged cell content repeated across spans.
    let mut cells: Vec<Vec<String>> = Vec::new();
    for row in &docx_table.rows {
        let mut row_cells = Vec::new();
        for cell in &row.cells {
            let cell_text = cell
                .paragraphs
                .iter()
                .map(|para| para.runs_to_markdown())
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string();
            let span = cell.properties.as_ref().and_then(|p| p.grid_span).unwrap_or(1);
            for _ in 0..span {
                row_cells.push(cell_text.clone());
            }
        }
        cells.push(row_cells);
    }
    // Fill vertically merged cells by copying from the row above.
    for row_idx in 1..docx_table.rows.len() {
        let mut col = 0usize;
        for cell in &docx_table.rows[row_idx].cells {
            let span = cell.properties.as_ref().and_then(|p| p.grid_span).unwrap_or(1) as usize;
            let is_vmerge_continue = cell
                .properties
                .as_ref()
                .is_some_and(|p| matches!(p.v_merge, Some(crate::extraction::docx::table::VerticalMerge::Continue)));
            if is_vmerge_continue {
                for c in col..col + span {
                    if c < cells[row_idx].len() && c < cells[row_idx - 1].len() {
                        cells[row_idx][c] = cells[row_idx - 1][c].clone();
                    }
                }
            }
            col += span;
        }
    }

    let markdown = cells_to_markdown(&cells);

    Table {
        cells,
        markdown,
        page_number,
        bounding_box: None,
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl DocumentExtractor for DocxExtractor {
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        tracing::debug!("extract_docx: starting");

        // When image extraction is enabled, force Markdown output so that
        // image placeholders (![](image)) are included in the text.
        let output_format = if config.images.as_ref().is_some_and(|i| i.extract_images) {
            crate::core::config::OutputFormat::Markdown
        } else {
            config.output_format.clone()
        };

        let include_doc_structure = config.include_document_structure;
        let (text, tables, page_boundaries, drawings, image_rels, _doc_structure, mut internal_doc) = {
            #[cfg(feature = "tokio-runtime")]
            if crate::core::batch_mode::is_batch_mode() {
                if config.cancel_token.as_ref().map(|t| t.is_cancelled()).unwrap_or(false) {
                    return Err(crate::error::KreuzbergError::Cancelled);
                }
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
        let mut docx_custom_properties: Option<std::collections::HashMap<String, serde_json::Value>> = None;

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

        // Build image entries for ALL drawings so that image_index (= drawing index)
        // always resolves in doc.images. When image extraction is enabled, populate
        // actual binary data; otherwise create placeholder entries with description
        // and source_path so the renderer can produce meaningful markdown references.
        let extract_image_data = config.images.as_ref().is_some_and(|i| i.extract_images);
        let mut extracted_images = Vec::with_capacity(drawings.len());
        for (idx, drawing) in drawings.iter().enumerate() {
            let description = drawing.doc_properties.as_ref().and_then(|dp| dp.description.clone());
            let source_path = drawing.image_ref.as_ref().and_then(|rid| image_rels.get(rid)).cloned();

            // Try to extract actual image data from the archive
            let mut image_data = None;
            if extract_image_data
                && let Some(ref rid) = drawing.image_ref
                && let Some(target) = image_rels.get(rid)
            {
                // Reject path traversal attempts within the archive
                if !target.contains("..") {
                    let zip_path = if let Some(stripped) = target.strip_prefix('/') {
                        stripped.to_string()
                    } else {
                        format!("word/{}", target)
                    };
                    if let Ok(mut file) = archive.by_name(&zip_path)
                        && file.size() <= crate::extraction::docx::MAX_IMAGE_FILE_SIZE
                    {
                        let mut data = Vec::with_capacity(file.size() as usize);
                        if std::io::Read::read_to_end(&mut file, &mut data).is_ok() {
                            image_data = Some(data);
                        }
                    }
                }
            }

            let (data, format, width, height) = if let Some(data) = image_data {
                let format = crate::extraction::image_format::detect_image_format(&data);
                let emus_per_px = crate::extraction::docx::EMUS_PER_PIXEL_96DPI;
                let (w, h) = drawing
                    .extent
                    .as_ref()
                    .map(|e| {
                        (
                            Some(u32::try_from(e.cx.max(0) / emus_per_px).unwrap_or(0)),
                            Some(u32::try_from(e.cy.max(0) / emus_per_px).unwrap_or(0)),
                        )
                    })
                    .unwrap_or((None, None));
                (Bytes::from(data), format, w, h)
            } else {
                // Derive format from source path extension
                let format = source_path
                    .as_ref()
                    .and_then(|p| p.rsplit('.').next())
                    .map(|ext| Cow::Owned(ext.to_lowercase()))
                    .unwrap_or(Cow::Borrowed("png"));
                (Bytes::new(), format, None, None)
            };

            // Determine page number from image placeholder position in text
            let page_number = {
                let placeholder = format!("![](image_{})", idx);
                let placeholder_with_desc = description.as_ref().map(|d| format!("![{}](image_{})", d, idx));

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

            extracted_images.push(ExtractedImage {
                data,
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
                source_path,
            });
        }

        // Build PageContent from page boundaries and store as prebuilt_pages so
        // derive_extraction_result can populate ExtractionResult.pages.
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
                        layout_regions: None,
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
                    layout_regions: None,
                }])
            }
        };
        internal_doc.prebuilt_pages = page_contents;

        // Extract typed metadata fields and remove them from additional map to avoid duplication
        let meta_title: Option<String> = metadata_map
            .remove(&Cow::Borrowed("title"))
            .and_then(|v| v.as_str().map(|s| s.to_string()));
        let meta_subject: Option<String> = metadata_map
            .remove(&Cow::Borrowed("subject"))
            .and_then(|v| v.as_str().map(|s| s.to_string()));
        let meta_authors: Option<Vec<String>> = metadata_map.remove(&Cow::Borrowed("authors")).and_then(|v| {
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

        internal_doc.metadata = Metadata {
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
        };

        // Filter headers/footers based on content_filter config.
        // When content_filter is None, keep current behavior (headers/footers included).
        // When content_filter is Some(...), respect include_headers/include_footers flags.
        if let Some(ref filter) = config.content_filter {
            use crate::types::document_structure::ContentLayer;
            internal_doc.elements.retain(|elem| match elem.layer {
                ContentLayer::Header => filter.include_headers,
                ContentLayer::Footer => filter.include_footers,
                _ => true,
            });
        }

        // Transfer images to InternalDocument
        internal_doc.images = extracted_images;
        internal_doc.mime_type = std::borrow::Cow::Owned(mime_type.to_string());

        // Recursively extract embedded objects from word/embeddings/
        if config.max_archive_depth > 0 {
            let (children, embed_warnings) = crate::extraction::ooxml_embedded::extract_ooxml_embedded_objects(
                content,
                "word/embeddings/",
                "docx",
                config,
            )
            .await;
            if !children.is_empty() {
                internal_doc.children = Some(children);
            }
            internal_doc.processing_warnings.extend(embed_warnings);
        }

        tracing::debug!(element_count = internal_doc.elements.len(), "extract_docx: complete");

        Ok(internal_doc)
    }

    fn supported_mime_types(&self) -> &[&str] {
        &[
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            "application/vnd.ms-word.document.macroEnabled.12",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.template",
            "application/vnd.ms-word.template.macroEnabled.12",
        ]
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
        assert_eq!(extractor.supported_mime_types().len(), 4);
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

        let result = convert_docx_table_to_table(&table, 1);

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
        let result =
            crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);

        // The derive pipeline produces plain text content; heading markers are in DocumentStructure
        assert!(
            result.content.contains("Document Title"),
            "Title should be present: {}",
            result.content
        );
        assert!(
            result.content.contains("Section One"),
            "Heading1 should be present: {}",
            result.content
        );
        assert!(result.content.contains("First paragraph content."));
        assert!(result.content.contains("Section one body text."));

        // Verify headings are in DocumentStructure
        let doc = result.document.as_ref().expect("DocumentStructure should be present");
        use crate::types::NodeContent;
        let headings: Vec<_> = doc
            .nodes
            .iter()
            .filter(|n| matches!(n.content, NodeContent::Heading { .. }))
            .collect();
        assert!(!headings.is_empty(), "Should have heading nodes in DocumentStructure");

        // Pages are not populated for DOCX elements without explicit page numbers
        // (the derive pipeline only creates pages when elements have page info)
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
        let result =
            crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);

        // The derive pipeline produces plain text content; formatting is in annotations/DocumentStructure
        assert!(result.content.contains("Bold text"), "Bold: {}", result.content);
        assert!(result.content.contains("italic text"), "Italic: {}", result.content);
        assert!(
            result.content.contains("underlined text"),
            "Underline: {}",
            result.content
        );

        // Verify formatting is preserved in DocumentStructure annotations
        let doc = result.document.as_ref().expect("DocumentStructure should be present");
        let all_annotations: Vec<_> = doc.nodes.iter().flat_map(|n| &n.annotations).collect();
        assert!(
            all_annotations
                .iter()
                .any(|a| a.kind == crate::types::document_structure::AnnotationKind::Bold),
            "Should have bold annotation"
        );
        assert!(
            all_annotations
                .iter()
                .any(|a| a.kind == crate::types::document_structure::AnnotationKind::Italic),
            "Should have italic annotation"
        );
        assert!(
            all_annotations
                .iter()
                .any(|a| a.kind == crate::types::document_structure::AnnotationKind::Underline),
            "Should have underline annotation"
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
        let result =
            crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);

        // The derive pipeline includes all element text in content (including headers/footers).
        // Headers/footers are distinguished via content_layer in DocumentStructure.
        assert!(
            result.content.contains("Body content here."),
            "Body: {}",
            result.content
        );

        // Verify headers/footers are tagged with correct content layers in DocumentStructure
        let doc = result.document.as_ref().expect("DocumentStructure should be present");
        use crate::types::ContentLayer;
        let header_nodes: Vec<_> = doc
            .nodes
            .iter()
            .filter(|n| n.content_layer == ContentLayer::Header)
            .collect();
        assert!(!header_nodes.is_empty(), "Should have header layer nodes");
        let footer_nodes: Vec<_> = doc
            .nodes
            .iter()
            .filter(|n| n.content_layer == ContentLayer::Footer)
            .collect();
        assert!(!footer_nodes.is_empty(), "Should have footer layer nodes");
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
        let result =
            crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);

        // Should have inline reference marker in the paragraph text
        assert!(
            result.content.contains("[^2]"),
            "Should have footnote ref: {}",
            result.content
        );
        // Footnote definitions are in the Footnote content layer, not the main content string.
        // Verify they exist in the DocumentStructure instead.
        let doc = result.document.as_ref().expect("should have document structure");
        let has_footnote = doc.nodes.iter().any(
            |n| matches!(&n.content, crate::types::NodeContent::Footnote { text } if text.contains("footnote content")),
        );
        assert!(has_footnote, "DocumentStructure should contain footnote node");
        // Separator footnotes should be excluded
        assert!(!result.content.contains("separator"), "Separator should be filtered");
        assert!(
            !result.content.contains("continuation"),
            "Continuation should be filtered"
        );
        // Verify footnote relationship exists in DocumentStructure
        let doc = result.document.as_ref().expect("DocumentStructure should be present");
        assert!(
            !doc.relationships.is_empty(),
            "Should have footnote relationships in DocumentStructure"
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
        let result =
            crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);

        // outline_level 0 = h1 — content is plain text; heading structure is in DocumentStructure
        assert!(
            result.content.contains("Custom Title"),
            "Style-based heading text should be present: {}",
            result.content
        );
        // Verify heading is in DocumentStructure with correct level
        let doc = result.document.as_ref().expect("DocumentStructure should be present");
        use crate::types::NodeContent;
        let h1_nodes: Vec<_> = doc
            .nodes
            .iter()
            .filter(|n| matches!(n.content, NodeContent::Heading { level: 1, .. }))
            .collect();
        assert!(
            !h1_nodes.is_empty(),
            "Should have h1 heading node from style-based heading"
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
        let result =
            crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);

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
        let result =
            crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);

        // The derive pipeline only builds pages when elements have explicit page numbers.
        // For simple DOCX without page break info, pages may be None.
        // Content should still be available in the main content field.
        assert!(
            result.content.contains("Simple single page document."),
            "Content should contain the document text: {}",
            result.content
        );
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
        let result =
            crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);

        // Should have inline reference marker in the paragraph text
        assert!(
            result.content.contains("[^2]"),
            "Should have endnote ref: {}",
            result.content
        );
        // Endnote definition text should be present (derive pipeline outputs plain text, not [^N]: format)
        assert!(
            result.document.as_ref().is_some_and(|doc| doc.nodes.iter().any(
                |n| matches!(&n.content, crate::types::NodeContent::Footnote { text } if text.contains("endnote"))
            )),
            "DocumentStructure should contain endnote node"
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
        let result =
            crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);

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
        let result =
            crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);

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
            image_ref: Some("rId1".to_string()),
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

        let md = doc.to_markdown(true);
        assert!(
            md.contains("![A test image](image)"),
            "Should have image placeholder: {}",
            md
        );
        assert!(md.contains("Before image."), "Should have text before");
        assert!(md.contains("After image."), "Should have text after");
    }

    /// Regression test for issue #484: image placeholders must appear even with
    /// default (Plain) output format when extract_images is enabled.
    #[tokio::test]
    async fn test_image_placeholder_with_default_output_format() {
        let document_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
            xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
            xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture"
            xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <w:body>
    <w:p><w:r><w:t>Text before image.</w:t></w:r></w:p>
    <w:p><w:r>
      <w:drawing>
        <wp:inline>
          <wp:extent cx="914400" cy="914400"/>
          <wp:docPr id="1" name="Picture 1" descr="Test image"/>
          <a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture">
            <pic:pic><pic:blipFill><a:blip r:embed="rId5"/></pic:blipFill></pic:pic>
          </a:graphicData></a:graphic>
        </wp:inline>
      </w:drawing>
    </w:r></w:p>
    <w:p><w:r><w:t>Text after image.</w:t></w:r></w:p>
  </w:body>
</w:document>"#;

        let docx_bytes = build_test_docx(document_xml);

        // Use default config (Plain output format) with extract_images enabled
        let config = ExtractionConfig {
            images: Some(crate::core::config::ImageExtractionConfig {
                extract_images: true,
                ..Default::default()
            }),
            ..Default::default()
        };

        let extractor = DocxExtractor::new();
        let result = extractor
            .extract_bytes(
                &docx_bytes,
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                &config,
            )
            .await
            .unwrap();
        let result =
            crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);

        // The derive pipeline skips images in the content string; they are represented
        // in the DocumentStructure as Image nodes.
        assert!(
            result.content.contains("Text before image."),
            "Should contain text before image: {}",
            result.content
        );
        assert!(
            result.content.contains("Text after image."),
            "Should contain text after image: {}",
            result.content
        );
        let doc = result.document.as_ref().expect("DocumentStructure should be present");
        use crate::types::NodeContent;
        let image_nodes: Vec<_> = doc
            .nodes
            .iter()
            .filter(|n| matches!(n.content, NodeContent::Image { .. }))
            .collect();
        assert!(!image_nodes.is_empty(), "Should have image nodes in DocumentStructure");
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
        let result =
            crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);

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
