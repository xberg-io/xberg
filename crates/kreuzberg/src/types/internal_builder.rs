//! Ergonomic builder for constructing `InternalDocument` instances.
//!
//! Mirrors the API of [`DocumentStructureBuilder`](super::builder::DocumentStructureBuilder)
//! but outputs the flat [`InternalDocument`](super::internal::InternalDocument) representation
//! instead of a tree-based `DocumentStructure`.
//!
//! # Example
//!
//! ```rust
//! use kreuzberg::types::internal_builder::InternalDocumentBuilder;
//!
//! let mut b = InternalDocumentBuilder::new("markdown");
//! b.push_heading(1, "Introduction", None, None);
//! b.push_paragraph("First paragraph.", vec![], None, None);
//! b.push_heading(2, "Details", None, None);
//! b.push_paragraph("Sub-section content.", vec![], None, None);
//! let doc = b.build();
//! assert_eq!(doc.elements.len(), 4);
//! ```

use std::borrow::Cow;

use ahash::AHashMap;

use super::document_structure::{ContentLayer, TextAnnotation};
use super::extraction::BoundingBox;
use super::internal::{
    ElementKind, InternalDocument, InternalElement, InternalElementId, Relationship, RelationshipKind,
    RelationshipTarget,
};
use super::metadata::Metadata;
use super::ocr_elements::{OcrBoundingGeometry, OcrConfidence, OcrElementLevel, OcrRotation};
use super::tables::Table;
use crate::types::ExtractedImage;
use crate::types::annotations::PdfAnnotation;
use crate::types::extraction::ProcessingWarning;

/// Builder for constructing `InternalDocument` with an ergonomic push-based API.
///
/// Tracks nesting depth automatically for list and quote containers,
/// and generates deterministic element IDs via blake3 hashing.
pub struct InternalDocumentBuilder {
    doc: InternalDocument,
    depth: u16,
    node_count: u32,
}

impl InternalDocumentBuilder {
    /// Create a new builder for the given source format.
    pub fn new(source_format: impl Into<Cow<'static, str>>) -> Self {
        Self {
            doc: InternalDocument::new(source_format),
            depth: 0,
            node_count: 0,
        }
    }

    /// Set the source format identifier (e.g. "docx", "html", "pptx").
    pub fn source_format(&mut self, format: impl Into<Cow<'static, str>>) {
        self.doc.source_format = format.into();
    }

    /// Set document-level metadata.
    pub fn set_metadata(&mut self, metadata: Metadata) {
        self.doc.metadata = metadata;
    }

    /// Set the MIME type of the source document.
    pub fn set_mime_type(&mut self, mime_type: impl Into<Cow<'static, str>>) {
        self.doc.mime_type = mime_type.into();
    }

    /// Add a non-fatal processing warning.
    pub fn add_warning(&mut self, warning: ProcessingWarning) {
        self.doc.processing_warnings.push(warning);
    }

    /// Set document-level PDF annotations (links, highlights, notes).
    pub fn set_pdf_annotations(&mut self, annotations: Vec<PdfAnnotation>) {
        self.doc.annotations = Some(annotations);
    }

    /// Push a URI discovered during extraction.
    pub fn push_uri(&mut self, uri: super::uri::Uri) {
        self.doc.push_uri(uri);
    }

    /// Consume the builder and return the constructed `InternalDocument`.
    pub fn build(self) -> InternalDocument {
        if self.depth != 0 {
            log::warn!("Unclosed containers: depth is {} at build time", self.depth);
        }
        self.doc
    }

    // ========================================================================
    // Heading
    // ========================================================================

    /// Push a heading element.
    ///
    /// Auto-sets depth from the heading level and generates an anchor slug
    /// from the heading text.
    pub fn push_heading(&mut self, level: u8, text: &str, page: Option<u32>, bbox: Option<BoundingBox>) -> u32 {
        let anchor = slugify(text);
        let kind = ElementKind::Heading { level };
        let depth = std::cmp::max(self.depth, level.saturating_sub(1) as u16);
        let elem = self.make_element(kind, text, depth, page, bbox, Some(&anchor));
        self.doc.push_element(elem)
    }

    // ========================================================================
    // Paragraph
    // ========================================================================

    /// Push a paragraph element.
    pub fn push_paragraph(
        &mut self,
        text: &str,
        annotations: Vec<TextAnnotation>,
        page: Option<u32>,
        bbox: Option<BoundingBox>,
    ) -> u32 {
        self.push_simple(ElementKind::Paragraph, text, page, bbox, annotations, None, None)
    }

    // ========================================================================
    // Lists
    // ========================================================================

    /// Push a `ListStart` marker and increment depth.
    pub fn push_list(&mut self, ordered: bool) {
        self.push_container_start(ElementKind::ListStart { ordered }, None);
    }

    /// Push a `ListEnd` marker and decrement depth.
    pub fn end_list(&mut self) {
        self.push_container_end(ElementKind::ListEnd);
    }

    /// Push a list item element at the current depth.
    pub fn push_list_item(
        &mut self,
        text: &str,
        ordered: bool,
        annotations: Vec<TextAnnotation>,
        page: Option<u32>,
        bbox: Option<BoundingBox>,
    ) -> u32 {
        let kind = ElementKind::ListItem { ordered };
        self.push_simple(kind, text, page, bbox, annotations, None, None)
    }

    // ========================================================================
    // Table
    // ========================================================================

    /// Push a table element. The table data is stored separately in
    /// `InternalDocument::tables` and referenced by index.
    pub fn push_table(&mut self, table: Table, page: Option<u32>, bbox: Option<BoundingBox>) -> u32 {
        let table_index = self.doc.push_table(table);
        let kind = ElementKind::Table { table_index };
        self.push_simple(kind, "", page, bbox, Vec::new(), None, None)
    }

    /// Push a table element from a 2D cell grid, building a `Table` struct automatically.
    pub fn push_table_from_cells(
        &mut self,
        cells: &[Vec<String>],
        page: Option<u32>,
        bbox: Option<BoundingBox>,
    ) -> u32 {
        let markdown = cells_to_markdown(cells);
        let table = Table {
            cells: cells.to_vec(),
            markdown,
            // 0 means "no page information available"; pages are otherwise 1-indexed.
            page_number: page.map(|p| p as usize).unwrap_or(0),
            bounding_box: None,
        };
        self.push_table(table, page, bbox)
    }

    // ========================================================================
    // Image
    // ========================================================================

    /// Push an image element. The image data is stored separately in
    /// `InternalDocument::images` and referenced by index.
    pub fn push_image(
        &mut self,
        description: Option<&str>,
        image: ExtractedImage,
        page: Option<u32>,
        bbox: Option<BoundingBox>,
    ) -> u32 {
        let image_index = self.doc.push_image(image);
        let kind = ElementKind::Image { image_index };
        let text = description.unwrap_or("");
        self.push_simple(kind, text, page, bbox, Vec::new(), None, None)
    }

    // ========================================================================
    // Code
    // ========================================================================

    /// Push a code block element. Language is stored in attributes.
    pub fn push_code(
        &mut self,
        text: &str,
        language: Option<&str>,
        page: Option<u32>,
        bbox: Option<BoundingBox>,
    ) -> u32 {
        let attrs = language.map(|lang| single_attr("language", lang));
        self.push_simple(ElementKind::Code, text, page, bbox, Vec::new(), attrs, None)
    }

    // ========================================================================
    // Formula
    // ========================================================================

    /// Push a math formula element.
    pub fn push_formula(&mut self, text: &str, page: Option<u32>, bbox: Option<BoundingBox>) -> u32 {
        self.push_simple(ElementKind::Formula, text, page, bbox, Vec::new(), None, None)
    }

    // ========================================================================
    // Footnotes
    // ========================================================================

    /// Push a footnote reference marker.
    ///
    /// Creates a `FootnoteRef` element with `anchor = key` and also records
    /// a `Relationship` with `RelationshipTarget::Key(key)` so the derivation
    /// step can resolve it to the definition.
    pub fn push_footnote_ref(&mut self, marker: &str, key: &str, page: Option<u32>) -> u32 {
        let idx = self.push_simple(
            ElementKind::FootnoteRef,
            marker,
            page,
            None,
            Vec::new(),
            None,
            Some(key),
        );
        // Record relationship
        self.doc.push_relationship(Relationship {
            source: idx,
            target: RelationshipTarget::Key(key.to_string()),
            kind: RelationshipKind::FootnoteReference,
        });
        idx
    }

    /// Push a footnote definition element with `anchor = key`.
    pub fn push_footnote_definition(&mut self, text: &str, key: &str, page: Option<u32>) -> u32 {
        self.push_simple(
            ElementKind::FootnoteDefinition,
            text,
            page,
            None,
            Vec::new(),
            None,
            Some(key),
        )
    }

    // ========================================================================
    // Citation
    // ========================================================================

    /// Push a citation / bibliographic reference element.
    pub fn push_citation(&mut self, text: &str, key: &str, page: Option<u32>) -> u32 {
        self.push_simple(ElementKind::Citation, text, page, None, Vec::new(), None, Some(key))
    }

    // ========================================================================
    // Quotes
    // ========================================================================

    /// Push a `QuoteStart` marker and increment depth.
    pub fn push_quote_start(&mut self) {
        self.push_container_start(ElementKind::QuoteStart, None);
    }

    /// Push a `QuoteEnd` marker and decrement depth.
    pub fn push_quote_end(&mut self) {
        self.push_container_end(ElementKind::QuoteEnd);
    }

    // ========================================================================
    // Page Break
    // ========================================================================

    /// Push a page break marker at depth 0.
    pub fn push_page_break(&mut self) {
        let elem = self.make_element(ElementKind::PageBreak, "", 0, None, None, None);
        self.doc.push_element(elem);
    }

    // ========================================================================
    // Slide
    // ========================================================================

    /// Push a slide element.
    pub fn push_slide(&mut self, number: u32, title: Option<&str>, page: Option<u32>) -> u32 {
        let kind = ElementKind::Slide { number };
        let text = title.unwrap_or("");
        let attrs = title.map(|t| single_attr("title", t));
        self.push_simple(kind, text, page, None, Vec::new(), attrs, None)
    }

    // ========================================================================
    // Admonition
    // ========================================================================

    /// Push an admonition / callout element (note, warning, tip, etc.).
    /// Kind and optional title are stored in attributes.
    pub fn push_admonition(&mut self, kind: &str, title: Option<&str>, page: Option<u32>) -> u32 {
        let mut attrs = AHashMap::new();
        attrs.insert("kind".to_string(), kind.to_string());
        if let Some(t) = title {
            attrs.insert("title".to_string(), t.to_string());
        }
        let text = title.unwrap_or(kind);
        self.push_simple(ElementKind::Admonition, text, page, None, Vec::new(), Some(attrs), None)
    }

    // ========================================================================
    // Raw Block
    // ========================================================================

    /// Push a raw block preserved verbatim. Format is stored in attributes.
    pub fn push_raw_block(&mut self, format: &str, content: &str, page: Option<u32>) -> u32 {
        self.push_simple(
            ElementKind::RawBlock,
            content,
            page,
            None,
            Vec::new(),
            Some(single_attr("format", format)),
            None,
        )
    }

    // ========================================================================
    // Metadata Block
    // ========================================================================

    /// Push a structured metadata block (frontmatter, email headers).
    /// Entries are stored in attributes.
    pub fn push_metadata_block(&mut self, entries: &[(String, String)], page: Option<u32>) -> u32 {
        let attrs: AHashMap<String, String> = entries.iter().cloned().collect();
        let text = entries
            .iter()
            .map(|(k, v)| format!("{k}: {v}"))
            .collect::<Vec<_>>()
            .join("\n");
        self.push_simple(
            ElementKind::MetadataBlock,
            &text,
            page,
            None,
            Vec::new(),
            Some(attrs),
            None,
        )
    }

    // ========================================================================
    // Title
    // ========================================================================

    /// Push a title element.
    pub fn push_title(&mut self, text: &str, page: Option<u32>, bbox: Option<BoundingBox>) -> u32 {
        self.push_simple(ElementKind::Title, text, page, bbox, Vec::new(), None, None)
    }

    // ========================================================================
    // Definition Term / Description
    // ========================================================================

    /// Push a definition term element.
    pub fn push_definition_term(&mut self, text: &str, page: Option<u32>) -> u32 {
        self.push_simple(ElementKind::DefinitionTerm, text, page, None, Vec::new(), None, None)
    }

    /// Push a definition description element.
    pub fn push_definition_description(&mut self, text: &str, page: Option<u32>) -> u32 {
        self.push_simple(
            ElementKind::DefinitionDescription,
            text,
            page,
            None,
            Vec::new(),
            None,
            None,
        )
    }

    // ========================================================================
    // OCR Text
    // ========================================================================

    /// Push an OCR text element with OCR-specific fields populated.
    #[allow(clippy::too_many_arguments)]
    pub fn push_ocr_text(
        &mut self,
        text: &str,
        level: OcrElementLevel,
        geometry: OcrBoundingGeometry,
        confidence: OcrConfidence,
        rotation: Option<OcrRotation>,
        page: Option<u32>,
        bbox: Option<BoundingBox>,
    ) -> u32 {
        let kind = ElementKind::OcrText { level };
        let mut elem = self.make_element(kind, text, self.depth, page, bbox, None);
        elem.ocr_geometry = Some(geometry);
        elem.ocr_confidence = Some(confidence);
        elem.ocr_rotation = rotation;
        self.doc.push_element(elem)
    }

    // ========================================================================
    // Groups
    // ========================================================================

    /// Push a `GroupStart` marker and increment depth.
    pub fn push_group_start(&mut self, label: Option<&str>, page: Option<u32>) {
        let attrs = label.map(|l| single_attr("label", l));
        self.push_container_start_with_attrs(ElementKind::GroupStart, page, attrs);
    }

    /// Push a `GroupEnd` marker and decrement depth.
    pub fn push_group_end(&mut self) {
        self.push_container_end(ElementKind::GroupEnd);
    }

    // ========================================================================
    // Relationships
    // ========================================================================

    /// Push a relationship between two elements.
    pub fn push_relationship(&mut self, source: u32, target: RelationshipTarget, kind: RelationshipKind) {
        self.doc.push_relationship(Relationship { source, target, kind });
    }

    /// Set the anchor on an already-pushed element.
    pub fn set_anchor(&mut self, index: u32, anchor: impl Into<String>) {
        if let Some(elem) = self.doc.elements.get_mut(index as usize) {
            elem.anchor = Some(anchor.into());
        }
    }

    /// Set the content layer on an already-pushed element.
    pub fn set_layer(&mut self, index: u32, layer: ContentLayer) {
        if let Some(elem) = self.doc.elements.get_mut(index as usize) {
            elem.layer = layer;
        }
    }

    /// Set attributes on an already-pushed element.
    pub fn set_attributes(&mut self, index: u32, attributes: AHashMap<String, String>) {
        if let Some(elem) = self.doc.elements.get_mut(index as usize) {
            elem.attributes = Some(attributes);
        }
    }

    /// Set annotations on an already-pushed element.
    pub fn set_annotations(&mut self, index: u32, annotations: Vec<TextAnnotation>) {
        if let Some(elem) = self.doc.elements.get_mut(index as usize) {
            elem.annotations = annotations;
        }
    }

    /// Set the text content of an already-pushed element.
    pub fn set_text(&mut self, index: u32, text: &str) {
        if let Some(elem) = self.doc.elements.get_mut(index as usize) {
            elem.text = text.into();
        }
    }

    // ========================================================================
    // Raw Element Push
    // ========================================================================

    /// Push a pre-constructed `InternalElement` directly.
    ///
    /// Useful when the caller needs to construct an element with fields
    /// that the builder's convenience methods don't cover (e.g. an image
    /// element without `ExtractedImage` data).
    pub fn push_element(&mut self, element: InternalElement) -> u32 {
        self.node_count += 1;
        self.doc.push_element(element)
    }

    // ========================================================================
    // Container Helpers (DRY start/end logic)
    // ========================================================================

    /// Push a container start marker and increment depth.
    fn push_container_start(&mut self, kind: ElementKind, page: Option<u32>) {
        self.push_simple(kind, "", page, None, Vec::new(), None, None);
        self.depth += 1;
    }

    /// Push a container start marker with attributes and increment depth.
    fn push_container_start_with_attrs(
        &mut self,
        kind: ElementKind,
        page: Option<u32>,
        attrs: Option<AHashMap<String, String>>,
    ) {
        self.push_simple(kind, "", page, None, Vec::new(), attrs, None);
        self.depth += 1;
    }

    /// Push a container end marker and decrement depth.
    fn push_container_end(&mut self, kind: ElementKind) {
        self.depth = self.depth.saturating_sub(1);
        self.push_simple(kind, "", None, None, Vec::new(), None, None);
    }

    // ========================================================================
    // Internal Helpers
    // ========================================================================

    /// Get the next index and increment the counter.
    fn next_index(&mut self) -> u32 {
        let idx = self.node_count;
        self.node_count += 1;
        idx
    }

    /// Create an `InternalElement` with common fields. All element construction
    /// is funnelled through this single site to reduce duplication.
    fn make_element(
        &mut self,
        kind: ElementKind,
        text: &str,
        depth: u16,
        page: Option<u32>,
        bbox: Option<BoundingBox>,
        anchor: Option<&str>,
    ) -> InternalElement {
        let idx = self.next_index();
        InternalElement {
            id: InternalElementId::generate(kind.discriminant(), text, page, idx),
            kind,
            text: text.to_string(),
            depth,
            page,
            bbox,
            layer: ContentLayer::Body,
            annotations: Vec::new(),
            attributes: None,
            anchor: anchor.map(|s| s.to_string()),
            ocr_geometry: None,
            ocr_confidence: None,
            ocr_rotation: None,
        }
    }

    /// Push a simple element with the current depth.
    #[allow(clippy::too_many_arguments)]
    fn push_simple(
        &mut self,
        kind: ElementKind,
        text: &str,
        page: Option<u32>,
        bbox: Option<BoundingBox>,
        annotations: Vec<TextAnnotation>,
        attributes: Option<AHashMap<String, String>>,
        anchor: Option<&str>,
    ) -> u32 {
        let mut elem = self.make_element(kind, text, self.depth, page, bbox, anchor);
        elem.annotations = annotations;
        elem.attributes = attributes;
        self.doc.push_element(elem)
    }
}

/// Create a `HashMap` with a single key-value pair.
fn single_attr(key: &str, val: &str) -> AHashMap<String, String> {
    let mut m = AHashMap::with_capacity(1);
    m.insert(key.to_string(), val.to_string());
    m
}

/// Convert a 2D cell grid to markdown table format.
fn cells_to_markdown(cells: &[Vec<String>]) -> String {
    if cells.is_empty() {
        return String::new();
    }
    let mut md = String::new();
    for (row_idx, row) in cells.iter().enumerate() {
        md.push('|');
        for cell in row {
            md.push(' ');
            md.push_str(cell);
            md.push_str(" |");
        }
        md.push('\n');
        if row_idx == 0 && cells.len() > 1 {
            md.push('|');
            for _ in row {
                md.push_str(" --- |");
            }
            md.push('\n');
        }
    }
    md
}

/// Generate a URL-friendly anchor slug from heading text.
///
/// Single-pass, single-allocation: lowercases alphanumeric characters,
/// collapses non-alphanumeric runs into single hyphens, and trims
/// leading/trailing hyphens.
fn slugify(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut prev_dash = true; // treat start as dash to avoid leading dash
    for c in text.chars() {
        if c.is_alphanumeric() {
            for lc in c.to_lowercase() {
                result.push(lc);
            }
            prev_dash = false;
        } else if !prev_dash {
            result.push('-');
            prev_dash = true;
        }
    }
    if result.ends_with('-') {
        result.pop();
    }
    result
}

// Compile-time assertion: InternalDocumentBuilder must be Send + Sync for concurrent extraction.
const _: () = {
    #[allow(dead_code)]
    fn assert_send_sync<T: Send + Sync>() {}
    #[allow(dead_code)]
    fn _check() {
        assert_send_sync::<InternalDocumentBuilder>();
    }
};

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use std::borrow::Cow;

    #[test]
    fn test_empty_builder() {
        let b = InternalDocumentBuilder::new("test");
        let doc = b.build();
        assert!(doc.elements.is_empty());
        assert!(doc.relationships.is_empty());
        assert_eq!(doc.source_format, "test");
    }

    #[test]
    fn test_push_heading() {
        let mut b = InternalDocumentBuilder::new("markdown");
        b.push_heading(1, "Introduction", Some(1), None);
        let doc = b.build();

        assert_eq!(doc.elements.len(), 1);
        let elem = &doc.elements[0];
        assert_eq!(elem.text, "Introduction");
        assert_eq!(elem.kind, ElementKind::Heading { level: 1 });
        assert_eq!(elem.depth, 0); // level 1 -> depth 0
        assert_eq!(elem.anchor.as_deref(), Some("introduction"));
        assert_eq!(elem.page, Some(1));
    }

    #[test]
    fn test_heading_depth_from_level() {
        let mut b = InternalDocumentBuilder::new("markdown");
        b.push_heading(1, "H1", None, None);
        b.push_heading(2, "H2", None, None);
        b.push_heading(3, "H3", None, None);
        let doc = b.build();

        assert_eq!(doc.elements[0].depth, 0);
        assert_eq!(doc.elements[1].depth, 1);
        assert_eq!(doc.elements[2].depth, 2);
    }

    #[test]
    fn test_push_paragraph() {
        let mut b = InternalDocumentBuilder::new("html");
        b.push_paragraph("Hello world.", vec![], Some(1), None);
        let doc = b.build();

        assert_eq!(doc.elements.len(), 1);
        assert_eq!(doc.elements[0].text, "Hello world.");
        assert_eq!(doc.elements[0].kind, ElementKind::Paragraph);
        assert_eq!(doc.elements[0].depth, 0);
    }

    #[test]
    fn test_paragraph_with_annotations() {
        let mut b = InternalDocumentBuilder::new("html");
        let ann = vec![TextAnnotation {
            start: 0,
            end: 5,
            kind: super::super::document_structure::AnnotationKind::Bold,
        }];
        b.push_paragraph("Hello world.", ann, None, None);
        let doc = b.build();

        assert_eq!(doc.elements[0].annotations.len(), 1);
        assert_eq!(doc.elements[0].annotations[0].start, 0);
        assert_eq!(doc.elements[0].annotations[0].end, 5);
    }

    #[test]
    fn test_list_depth_management() {
        let mut b = InternalDocumentBuilder::new("markdown");
        assert_eq!(b.depth, 0);
        b.push_list(false);
        assert_eq!(b.depth, 1);
        b.push_list_item("Item 1", false, vec![], Some(1), None);
        b.push_list_item("Item 2", false, vec![], Some(1), None);
        b.end_list();
        assert_eq!(b.depth, 0);

        let doc = b.build();
        // ListStart + 2 items + ListEnd
        assert_eq!(doc.elements.len(), 4);
        assert_eq!(doc.elements[0].kind, ElementKind::ListStart { ordered: false });
        assert_eq!(doc.elements[1].kind, ElementKind::ListItem { ordered: false });
        assert_eq!(doc.elements[1].depth, 1);
        assert_eq!(doc.elements[2].depth, 1);
        assert_eq!(doc.elements[3].kind, ElementKind::ListEnd);
        assert_eq!(doc.elements[3].depth, 0);
    }

    #[test]
    fn test_nested_lists() {
        let mut b = InternalDocumentBuilder::new("markdown");
        b.push_list(false);
        b.push_list_item("Outer", false, vec![], None, None);
        b.push_list(true);
        b.push_list_item("Inner", true, vec![], None, None);
        b.end_list();
        b.end_list();

        let doc = b.build();
        // Outer ListStart(depth=0), item(depth=1), inner ListStart(depth=1),
        // inner item(depth=2), inner ListEnd(depth=1), outer ListEnd(depth=0)
        assert_eq!(doc.elements.len(), 6);
        assert_eq!(doc.elements[0].depth, 0); // outer list start
        assert_eq!(doc.elements[1].depth, 1); // outer item
        assert_eq!(doc.elements[2].depth, 1); // inner list start
        assert_eq!(doc.elements[3].depth, 2); // inner item
        assert_eq!(doc.elements[4].depth, 1); // inner list end
        assert_eq!(doc.elements[5].depth, 0); // outer list end
    }

    #[test]
    fn test_push_table() {
        let mut b = InternalDocumentBuilder::new("html");
        let table = Table {
            cells: vec![vec!["A".to_string(), "B".to_string()]],
            markdown: "| A | B |".to_string(),
            page_number: 1,
            bounding_box: None,
        };
        b.push_table(table, Some(1), None);
        let doc = b.build();

        assert_eq!(doc.elements.len(), 1);
        assert_eq!(doc.elements[0].kind, ElementKind::Table { table_index: 0 });
        assert_eq!(doc.tables.len(), 1);
        assert_eq!(doc.tables[0].cells[0][0], "A");
    }

    #[test]
    fn test_push_image() {
        let mut b = InternalDocumentBuilder::new("pdf");
        let image = ExtractedImage {
            data: Bytes::from_static(&[0xFF, 0xD8]),
            format: Cow::Borrowed("jpeg"),
            image_index: 0,
            page_number: Some(1),
            width: Some(100),
            height: Some(200),
            colorspace: None,
            bits_per_component: None,
            is_mask: false,
            description: Some("A photo".to_string()),
            ocr_result: None,
            bounding_box: None,
            source_path: None,
        };
        b.push_image(Some("A photo"), image, Some(1), None);
        let doc = b.build();

        assert_eq!(doc.elements.len(), 1);
        assert_eq!(doc.elements[0].kind, ElementKind::Image { image_index: 0 });
        assert_eq!(doc.elements[0].text, "A photo");
        assert_eq!(doc.images.len(), 1);
    }

    #[test]
    fn test_push_code() {
        let mut b = InternalDocumentBuilder::new("markdown");
        b.push_code("fn main() {}", Some("rust"), Some(1), None);
        let doc = b.build();

        assert_eq!(doc.elements.len(), 1);
        assert_eq!(doc.elements[0].kind, ElementKind::Code);
        assert_eq!(doc.elements[0].text, "fn main() {}");
        let attrs = doc.elements[0].attributes.as_ref().unwrap();
        assert_eq!(attrs.get("language").unwrap(), "rust");
    }

    #[test]
    fn test_push_code_no_language() {
        let mut b = InternalDocumentBuilder::new("markdown");
        b.push_code("echo hello", None, None, None);
        let doc = b.build();

        assert_eq!(doc.elements[0].kind, ElementKind::Code);
        assert!(doc.elements[0].attributes.is_none());
    }

    #[test]
    fn test_push_formula() {
        let mut b = InternalDocumentBuilder::new("latex");
        b.push_formula("E = mc^2", Some(1), None);
        let doc = b.build();

        assert_eq!(doc.elements.len(), 1);
        assert_eq!(doc.elements[0].kind, ElementKind::Formula);
        assert_eq!(doc.elements[0].text, "E = mc^2");
    }

    #[test]
    fn test_push_footnote_ref() {
        let mut b = InternalDocumentBuilder::new("markdown");
        b.push_footnote_ref("1", "fn1", Some(1));
        let doc = b.build();

        assert_eq!(doc.elements.len(), 1);
        assert_eq!(doc.elements[0].kind, ElementKind::FootnoteRef);
        assert_eq!(doc.elements[0].text, "1");
        assert_eq!(doc.elements[0].anchor.as_deref(), Some("fn1"));
        // Check relationship
        assert_eq!(doc.relationships.len(), 1);
        assert_eq!(doc.relationships[0].source, 0);
        assert_eq!(doc.relationships[0].kind, RelationshipKind::FootnoteReference);
        match &doc.relationships[0].target {
            RelationshipTarget::Key(k) => assert_eq!(k, "fn1"),
            _ => panic!("Expected Key target"),
        }
    }

    #[test]
    fn test_push_footnote_definition() {
        let mut b = InternalDocumentBuilder::new("markdown");
        b.push_footnote_definition("This is a footnote.", "fn1", Some(1));
        let doc = b.build();

        assert_eq!(doc.elements.len(), 1);
        assert_eq!(doc.elements[0].kind, ElementKind::FootnoteDefinition);
        assert_eq!(doc.elements[0].text, "This is a footnote.");
        assert_eq!(doc.elements[0].anchor.as_deref(), Some("fn1"));
    }

    #[test]
    fn test_push_citation() {
        let mut b = InternalDocumentBuilder::new("latex");
        b.push_citation("Smith et al. 2024", "smith2024", Some(3));
        let doc = b.build();

        assert_eq!(doc.elements.len(), 1);
        assert_eq!(doc.elements[0].kind, ElementKind::Citation);
        assert_eq!(doc.elements[0].text, "Smith et al. 2024");
        assert_eq!(doc.elements[0].anchor.as_deref(), Some("smith2024"));
    }

    #[test]
    fn test_quote_start_end() {
        let mut b = InternalDocumentBuilder::new("markdown");
        b.push_quote_start();
        b.push_paragraph("Quoted text.", vec![], None, None);
        b.push_quote_end();
        let doc = b.build();

        assert_eq!(doc.elements.len(), 3);
        assert_eq!(doc.elements[0].kind, ElementKind::QuoteStart);
        assert_eq!(doc.elements[0].depth, 0);
        assert_eq!(doc.elements[1].kind, ElementKind::Paragraph);
        assert_eq!(doc.elements[1].depth, 1);
        assert_eq!(doc.elements[2].kind, ElementKind::QuoteEnd);
        assert_eq!(doc.elements[2].depth, 0);
    }

    #[test]
    fn test_push_page_break() {
        let mut b = InternalDocumentBuilder::new("pdf");
        b.push_list(false);
        b.push_page_break(); // should be depth 0 regardless
        b.end_list();
        let doc = b.build();

        let pb = doc.elements.iter().find(|e| e.kind == ElementKind::PageBreak).unwrap();
        assert_eq!(pb.depth, 0);
    }

    #[test]
    fn test_push_slide() {
        let mut b = InternalDocumentBuilder::new("pptx");
        b.push_slide(1, Some("Title Slide"), Some(1));
        let doc = b.build();

        assert_eq!(doc.elements.len(), 1);
        assert_eq!(doc.elements[0].kind, ElementKind::Slide { number: 1 });
        assert_eq!(doc.elements[0].text, "Title Slide");
        let attrs = doc.elements[0].attributes.as_ref().unwrap();
        assert_eq!(attrs.get("title").unwrap(), "Title Slide");
    }

    #[test]
    fn test_push_slide_no_title() {
        let mut b = InternalDocumentBuilder::new("pptx");
        b.push_slide(2, None, None);
        let doc = b.build();

        assert_eq!(doc.elements[0].kind, ElementKind::Slide { number: 2 });
        assert_eq!(doc.elements[0].text, "");
        assert!(doc.elements[0].attributes.is_none());
    }

    #[test]
    fn test_push_admonition() {
        let mut b = InternalDocumentBuilder::new("markdown");
        b.push_admonition("warning", Some("Be careful"), Some(1));
        let doc = b.build();

        assert_eq!(doc.elements.len(), 1);
        assert_eq!(doc.elements[0].kind, ElementKind::Admonition);
        let attrs = doc.elements[0].attributes.as_ref().unwrap();
        assert_eq!(attrs.get("kind").unwrap(), "warning");
        assert_eq!(attrs.get("title").unwrap(), "Be careful");
    }

    #[test]
    fn test_push_admonition_no_title() {
        let mut b = InternalDocumentBuilder::new("markdown");
        b.push_admonition("note", None, None);
        let doc = b.build();

        let attrs = doc.elements[0].attributes.as_ref().unwrap();
        assert_eq!(attrs.get("kind").unwrap(), "note");
        assert!(!attrs.contains_key("title"));
        assert_eq!(doc.elements[0].text, "note");
    }

    #[test]
    fn test_push_raw_block() {
        let mut b = InternalDocumentBuilder::new("html");
        b.push_raw_block("html", "<div>raw</div>", Some(1));
        let doc = b.build();

        assert_eq!(doc.elements.len(), 1);
        assert_eq!(doc.elements[0].kind, ElementKind::RawBlock);
        assert_eq!(doc.elements[0].text, "<div>raw</div>");
        let attrs = doc.elements[0].attributes.as_ref().unwrap();
        assert_eq!(attrs.get("format").unwrap(), "html");
    }

    #[test]
    fn test_push_metadata_block() {
        let mut b = InternalDocumentBuilder::new("email");
        let entries = vec![
            ("From".to_string(), "alice@example.com".to_string()),
            ("To".to_string(), "bob@example.com".to_string()),
        ];
        b.push_metadata_block(&entries, Some(1));
        let doc = b.build();

        assert_eq!(doc.elements.len(), 1);
        assert_eq!(doc.elements[0].kind, ElementKind::MetadataBlock);
        let attrs = doc.elements[0].attributes.as_ref().unwrap();
        assert_eq!(attrs.get("From").unwrap(), "alice@example.com");
        assert_eq!(attrs.get("To").unwrap(), "bob@example.com");
        assert!(doc.elements[0].text.contains("From: alice@example.com"));
        assert!(doc.elements[0].text.contains("To: bob@example.com"));
    }

    #[test]
    fn test_set_metadata() {
        let mut b = InternalDocumentBuilder::new("pdf");
        b.set_metadata(Metadata {
            title: Some("My Document".to_string()),
            ..Metadata::default()
        });
        let doc = b.build();

        assert_eq!(doc.metadata.title.as_deref(), Some("My Document"));
    }

    #[test]
    fn test_source_format() {
        let mut b = InternalDocumentBuilder::new("pdf");
        b.source_format("docx");
        let doc = b.build();
        assert_eq!(doc.source_format, "docx");
    }

    #[test]
    fn test_node_count_increments() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_paragraph("A", vec![], None, None);
        b.push_paragraph("B", vec![], None, None);
        b.push_paragraph("C", vec![], None, None);
        assert_eq!(b.node_count, 3);
        let doc = b.build();
        assert_eq!(doc.elements.len(), 3);
    }

    #[test]
    fn test_ids_are_deterministic() {
        let build = || {
            let mut b = InternalDocumentBuilder::new("test");
            b.push_heading(1, "Hello", Some(1), None);
            b.push_paragraph("World", vec![], Some(1), None);
            b.build()
        };
        let doc1 = build();
        let doc2 = build();
        assert_eq!(doc1.elements[0].id, doc2.elements[0].id);
        assert_eq!(doc1.elements[1].id, doc2.elements[1].id);
    }

    #[test]
    fn test_ids_differ_by_index() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_paragraph("Same text", vec![], Some(1), None);
        b.push_paragraph("Same text", vec![], Some(1), None);
        let doc = b.build();
        assert_ne!(doc.elements[0].id, doc.elements[1].id);
    }

    #[test]
    fn test_bbox_preserved() {
        let mut b = InternalDocumentBuilder::new("pdf");
        let bbox = BoundingBox {
            x0: 10.0,
            y0: 20.0,
            x1: 100.0,
            y1: 50.0,
        };
        b.push_paragraph("text", vec![], Some(1), Some(bbox));
        let doc = b.build();
        let elem_bbox = doc.elements[0].bbox.unwrap();
        assert_eq!(elem_bbox.x0, 10.0);
        assert_eq!(elem_bbox.y1, 50.0);
    }

    // ====================================================================
    // Slug generation tests
    // ====================================================================

    #[test]
    fn test_slugify_basic() {
        assert_eq!(slugify("Hello World"), "hello-world");
    }

    #[test]
    fn test_slugify_special_chars() {
        assert_eq!(slugify("What's New?"), "what-s-new");
    }

    #[test]
    fn test_slugify_consecutive_specials() {
        assert_eq!(slugify("A -- B"), "a-b");
    }

    #[test]
    fn test_slugify_leading_trailing() {
        assert_eq!(slugify("  Hello  "), "hello");
    }

    #[test]
    fn test_slugify_numbers() {
        assert_eq!(slugify("Section 3.1"), "section-3-1");
    }

    #[test]
    fn test_slugify_unicode() {
        // Unicode alphanumeric chars are preserved
        assert_eq!(slugify("Über Cool"), "über-cool");
    }

    #[test]
    fn test_slugify_empty() {
        assert_eq!(slugify(""), "");
    }

    // ====================================================================
    // Integration-style test
    // ====================================================================

    #[test]
    fn test_full_document_construction() {
        let mut b = InternalDocumentBuilder::new("markdown");

        b.set_metadata(Metadata {
            title: Some("Test Document".to_string()),
            ..Metadata::default()
        });

        b.push_heading(1, "Introduction", Some(1), None);
        b.push_paragraph("Welcome to the document.", vec![], Some(1), None);

        b.push_heading(2, "Details", Some(1), None);
        b.push_paragraph("Some details here.", vec![], Some(1), None);

        b.push_list(false);
        b.push_list_item("First item", false, vec![], Some(1), None);
        b.push_list_item("Second item", false, vec![], Some(1), None);
        b.end_list();

        b.push_code("let x = 1;", Some("rust"), Some(2), None);
        b.push_formula("E = mc^2", Some(2), None);

        b.push_footnote_ref("1", "fn1", Some(2));
        b.push_footnote_definition("A footnote.", "fn1", Some(2));

        b.push_quote_start();
        b.push_paragraph("A famous quote.", vec![], Some(3), None);
        b.push_quote_end();

        b.push_page_break();

        let doc = b.build();

        assert_eq!(doc.metadata.title.as_deref(), Some("Test Document"));
        assert_eq!(doc.source_format, "markdown");
        // Count: 2 headings + 2 paragraphs + ListStart + 2 items + ListEnd +
        //        code + formula + fnref + fndef + QuoteStart + paragraph + QuoteEnd + PageBreak
        assert_eq!(doc.elements.len(), 16);
        assert_eq!(doc.relationships.len(), 1);

        // Verify heading anchors
        assert_eq!(doc.elements[0].anchor.as_deref(), Some("introduction"));
        assert_eq!(doc.elements[2].anchor.as_deref(), Some("details"));
    }
}
