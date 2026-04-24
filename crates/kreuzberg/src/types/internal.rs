//! Internal flat document representation.
//!
//! This module provides the internal DTO that all extractors output. It is a flat,
//! append-only structure optimized for extraction performance. The public
//! [`DocumentStructure`](super::document_structure::DocumentStructure) tree and
//! relationship graph are derived from this in a post-processing step.
//!
//! # Design
//!
//! - **Flat `Vec<InternalElement>`**: Cache-friendly, append-only during extraction
//! - **Relationships stored separately**: Keeps element iteration compact
//! - **Optional container markers**: `ListStart`/`ListEnd` etc. improve tree derivation
//!   when present; depth-based heuristics used as fallback
//! - **OCR elements unified**: OCR text is just another element kind, not a parallel structure
//! - **Blake3 IDs**: Deterministic, collision-resistant identifiers

use std::borrow::Cow;
use std::fmt;

use ahash::AHashMap;

use super::document_structure::{ContentLayer, TextAnnotation};
use super::extraction::BoundingBox;
use super::metadata::Metadata;
use super::ocr_elements::{OcrBoundingGeometry, OcrConfidence, OcrElementLevel, OcrRotation};
use super::tables::Table;
use crate::types::ExtractedImage;

// ============================================================================
// ID Type
// ============================================================================

/// Deterministic element identifier, generated via blake3 hashing.
///
/// Format: `"ie-{12 hex chars}"` (48 bits from blake3, ~281 trillion address space).
/// Same input always produces the same ID, enabling diffing and caching.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InternalElementId([u8; 15]);

impl InternalElementId {
    /// Generate a deterministic ID from element content.
    ///
    /// Hashes the element kind discriminant, text content, page number, and
    /// positional index using blake3. Takes 48 bits (6 bytes) of the hash.
    pub(crate) fn generate(kind_discriminant: &str, text: &str, page: Option<u32>, index: u32) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(kind_discriminant.as_bytes());
        hasher.update(text.as_bytes());
        hasher.update(&page.unwrap_or(u32::MAX).to_le_bytes());
        hasher.update(&index.to_le_bytes());
        let hash = hasher.finalize();
        let bytes = &hash.as_bytes()[..6];
        let mut buf = [0u8; 15];
        buf[0] = b'i';
        buf[1] = b'e';
        buf[2] = b'-';
        hex::encode_to_slice(bytes, &mut buf[3..]).expect("fixed size");
        Self(buf)
    }

    /// Create from a pre-computed ID string.
    ///
    /// The input must be exactly 15 bytes in `"ie-{12 hex}"` format.
    /// Panics if the input length is not 15.
    #[allow(dead_code)]
    pub fn new(id: &str) -> Self {
        assert!(
            id.len() == 15,
            "InternalElementId must be exactly 15 bytes, got {}",
            id.len()
        );
        let mut buf = [0u8; 15];
        buf.copy_from_slice(id.as_bytes());
        Self(buf)
    }

    /// Get the ID as a string slice.
    pub(crate) fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).unwrap()
    }
}

impl fmt::Display for InternalElementId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for InternalElementId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

// ============================================================================
// Internal Document
// ============================================================================

/// The internal flat document representation.
///
/// All extractors output this structure. It is converted to the public
/// [`ExtractionResult`](super::extraction::ExtractionResult) and
/// [`DocumentStructure`](super::document_structure::DocumentStructure) in the pipeline.
#[derive(Debug, Clone)]
pub struct InternalDocument {
    /// All elements in reading order. Append-only during extraction.
    pub elements: Vec<InternalElement>,

    /// Relationships between elements (source index → target).
    /// Stored separately from elements for cache-friendly iteration.
    pub relationships: Vec<Relationship>,

    /// Source format identifier (e.g., "pdf", "docx", "html", "markdown").
    pub source_format: Cow<'static, str>,

    /// Document-level metadata (title, author, dates, etc.).
    pub metadata: Metadata,

    /// Extracted images (binary data). Referenced by index from `ElementKind::Image`.
    pub images: Vec<ExtractedImage>,

    /// Extracted tables (structured data). Referenced by index from `ElementKind::Table`.
    pub tables: Vec<Table>,

    /// URIs/links discovered during extraction (hyperlinks, image refs, citations, etc.).
    pub uris: Vec<super::uri::Uri>,

    /// Archive children: fully-extracted results for files within an archive.
    ///
    /// Only populated by archive extractors (ZIP, TAR, 7z, GZIP) when recursive
    /// extraction is enabled. Each entry contains the full `ExtractionResult` for
    /// a child file that was extracted through the public pipeline.
    pub children: Option<Vec<crate::types::ArchiveEntry>>,

    /// MIME type of the source document (e.g., "application/pdf", "text/html").
    pub mime_type: Cow<'static, str>,

    /// Non-fatal warnings collected during extraction.
    pub processing_warnings: Vec<crate::types::ProcessingWarning>,

    /// PDF annotations (links, highlights, notes).
    pub annotations: Option<Vec<crate::types::annotations::PdfAnnotation>>,

    /// Pre-built per-page content (set by extractors that track page boundaries natively).
    ///
    /// When populated, `derive_extraction_result` uses this directly instead of
    /// attempting to reconstruct pages from element-level page numbers.
    pub prebuilt_pages: Option<Vec<crate::types::PageContent>>,

    /// Pre-rendered formatted content produced by the extractor itself.
    ///
    /// When an extractor has direct access to high-quality formatted output (e.g.,
    /// html-to-markdown produces GFM markdown), it can store that here to bypass
    /// the lossy InternalDocument → renderer round-trip. `derive_extraction_result`
    /// will use this directly when the requested output format matches
    /// `metadata.output_format`.
    pub pre_rendered_content: Option<String>,

    /// Pre-built OCR element list (set by extractors that have direct access to
    /// bounding-box element data alongside a separately produced coherent text).
    ///
    /// When populated, `derive_extraction_result` uses this directly instead of
    /// reconstructing `OcrElement`s from `OcrText` `InternalElement`s. This lets
    /// the image extractor carry Tesseract/paddle-ocr bounding-box metadata without
    /// injecting raw word tokens into the element list (which would otherwise corrupt
    /// `render_plain` and page content — issue #706).
    pub prebuilt_ocr_elements: Option<Vec<crate::types::ocr_elements::OcrElement>>,

    /// LLM usage records accumulated during extraction (e.g., VLM OCR per page).
    ///
    /// Populated by extractors that call LLM-backed backends (VLM OCR).
    /// `derive_extraction_result` transfers this to `ExtractionResult.llm_usage`.
    pub llm_usage: Option<Vec<crate::types::LlmUsage>>,
}

impl InternalDocument {
    /// Create a new empty document with the given source format.
    pub fn new(source_format: impl Into<Cow<'static, str>>) -> Self {
        Self {
            elements: Vec::new(),
            relationships: Vec::new(),
            source_format: source_format.into(),
            metadata: Metadata::default(),
            images: Vec::new(),
            tables: Vec::new(),
            uris: Vec::new(),
            children: None,
            mime_type: Cow::Borrowed("application/octet-stream"),
            processing_warnings: Vec::new(),
            annotations: None,
            prebuilt_pages: None,
            pre_rendered_content: None,
            prebuilt_ocr_elements: None,
            llm_usage: None,
        }
    }

    /// Push an element and return its index.
    pub fn push_element(&mut self, element: InternalElement) -> u32 {
        // Safety: element count is bounded by available memory; u32::MAX (~4 billion)
        // elements would require hundreds of GB, so truncation cannot occur in practice.
        let idx = self.elements.len() as u32;
        self.elements.push(element);
        idx
    }

    /// Push a relationship.
    pub fn push_relationship(&mut self, relationship: Relationship) {
        self.relationships.push(relationship);
    }

    /// Push a table and return its index (for use in `ElementKind::Table`).
    pub fn push_table(&mut self, table: Table) -> u32 {
        // Safety: table count is bounded by document size; overflow at u32::MAX is
        // practically unreachable (would require ~4 billion tables).
        let idx = self.tables.len() as u32;
        self.tables.push(table);
        idx
    }

    /// Push an image and return its index (for use in `ElementKind::Image`).
    pub fn push_image(&mut self, image: ExtractedImage) -> u32 {
        // Safety: image count is bounded by document size; overflow at u32::MAX is
        // practically unreachable (would require ~4 billion images).
        let idx = self.images.len() as u32;
        self.images.push(image);
        idx
    }

    /// Maximum number of URIs to collect per document (DoS prevention).
    const MAX_URIS: usize = 100_000;

    /// Push a URI discovered during extraction.
    /// Silently drops URIs beyond `MAX_URIS` to prevent unbounded memory growth.
    pub fn push_uri(&mut self, uri: super::uri::Uri) {
        if self.uris.len() < Self::MAX_URIS {
            self.uris.push(uri);
        }
    }

    /// Concatenate all element text into a single string, separated by newlines.
    pub(crate) fn content(&self) -> String {
        self.elements
            .iter()
            .map(|e| e.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

// ============================================================================
// Internal Element
// ============================================================================

/// A single element in the internal flat document.
///
/// Elements are appended in reading order during extraction. The `depth` field
/// and optional container markers enable tree reconstruction in the derivation step.
#[derive(Debug, Clone, PartialEq)]
pub struct InternalElement {
    /// Deterministic identifier.
    pub id: InternalElementId,

    /// What kind of content this element represents.
    pub kind: ElementKind,

    /// Primary text content. Empty for non-text elements (images, page breaks).
    pub text: String,

    /// Nesting depth (0 = root level).
    ///
    /// Extractors set this based on heading level, list indent, blockquote depth, etc.
    /// The tree derivation step uses depth changes to reconstruct parent-child relationships.
    pub depth: u16,

    /// Page number (1-indexed). `None` for non-paginated formats.
    pub page: Option<u32>,

    /// Bounding box in document coordinates.
    pub bbox: Option<BoundingBox>,

    /// Content layer classification (Body, Header, Footer, Footnote).
    pub layer: ContentLayer,

    /// Inline annotations (formatting, links) on this element's text content.
    /// Byte-range based, reuses the existing `TextAnnotation` type.
    pub annotations: Vec<TextAnnotation>,

    /// Format-specific key-value attributes.
    /// Used for CSS classes, LaTeX env names, slide layout names, etc.
    pub attributes: Option<AHashMap<String, String>>,

    /// Optional anchor/key for this element.
    ///
    /// Used by the relationship resolver to match references to targets.
    /// Examples: heading slug `"introduction"`, footnote label `"fn1"`,
    /// citation key `"smith2024"`, figure label `"fig:diagram"`.
    pub anchor: Option<String>,

    // === OCR-specific fields (zero-cost when None) ===
    /// OCR bounding geometry (rectangle or quadrilateral).
    pub ocr_geometry: Option<OcrBoundingGeometry>,

    /// OCR confidence scores (detection + recognition).
    pub ocr_confidence: Option<OcrConfidence>,

    /// OCR rotation metadata.
    pub ocr_rotation: Option<OcrRotation>,
}

impl InternalElement {
    /// Create a simple text element with minimal fields.
    pub fn text(kind: ElementKind, text: impl Into<String>, depth: u16) -> Self {
        let text = text.into();
        let id = InternalElementId::generate(kind.discriminant(), &text, None, 0);
        Self {
            id,
            kind,
            text,
            depth,
            page: None,
            bbox: None,
            layer: ContentLayer::Body,
            annotations: Vec::new(),
            attributes: None,
            anchor: None,
            ocr_geometry: None,
            ocr_confidence: None,
            ocr_rotation: None,
        }
    }

    /// Set the page number.
    pub(crate) fn with_page(mut self, page: u32) -> Self {
        self.page = Some(page);
        self
    }

    /// Set the bounding box.
    pub(crate) fn with_bbox(mut self, bbox: BoundingBox) -> Self {
        self.bbox = Some(bbox);
        self
    }

    /// Set the content layer.
    #[cfg(test)]
    pub(crate) fn with_layer(mut self, layer: ContentLayer) -> Self {
        self.layer = layer;
        self
    }

    /// Set the anchor key.
    pub(crate) fn with_anchor(mut self, anchor: impl Into<String>) -> Self {
        self.anchor = Some(anchor.into());
        self
    }

    /// Set annotations.
    pub(crate) fn with_annotations(mut self, annotations: Vec<TextAnnotation>) -> Self {
        self.annotations = annotations;
        self
    }

    /// Set attributes.
    pub(crate) fn with_attributes(mut self, attributes: AHashMap<String, String>) -> Self {
        self.attributes = Some(attributes);
        self
    }

    /// Regenerate the ID with the correct index (call after pushing to the document).
    pub(crate) fn with_index(mut self, index: u32) -> Self {
        self.id = InternalElementId::generate(self.kind.discriminant(), &self.text, self.page, index);
        self
    }
}

// ============================================================================
// Element Kind
// ============================================================================

/// Semantic role of an internal element.
///
/// Superset of [`NodeContent`](super::document_structure::NodeContent) variants
/// plus OCR and container markers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementKind {
    // --- Text-carrying ---
    /// Document title.
    Title,
    /// Section heading with level (1-6).
    Heading { level: u8 },
    /// Body text paragraph.
    Paragraph,
    /// List item. `ordered` indicates numbered vs bulleted.
    ListItem { ordered: bool },
    /// Code block. Language stored in element attributes.
    Code,
    /// Mathematical formula / equation.
    Formula,
    /// Footnote content (the definition, not the reference marker).
    FootnoteDefinition,
    /// Footnote reference marker in body text.
    FootnoteRef,
    /// Citation or bibliographic reference.
    Citation,
    /// Presentation slide container.
    Slide { number: u32 },
    /// Definition list term.
    DefinitionTerm,
    /// Definition list description.
    DefinitionDescription,
    /// Admonition / callout (note, warning, tip, etc.). Kind stored in attributes.
    Admonition,
    /// Raw block preserved verbatim. Format stored in attributes.
    RawBlock,
    /// Structured metadata block (frontmatter, email headers).
    MetadataBlock,

    // --- Container markers (optional, improve tree precision) ---
    /// Start of a list container.
    ListStart { ordered: bool },
    /// End of a list container.
    ListEnd,
    /// Start of a block quote.
    QuoteStart,
    /// End of a block quote.
    QuoteEnd,
    /// Start of a generic group/section.
    GroupStart,
    /// End of a generic group/section.
    GroupEnd,

    // --- Structural ---
    /// Table reference. `table_index` is an index into `InternalDocument::tables`.
    Table { table_index: u32 },
    /// Image reference. `image_index` is an index into `InternalDocument::images`.
    Image { image_index: u32 },
    /// Page break marker.
    PageBreak,

    // --- OCR ---
    /// OCR-detected text at a given hierarchical level.
    OcrText { level: OcrElementLevel },
}

impl ElementKind {
    /// Get a stable string discriminant for ID generation.
    pub(crate) fn discriminant(&self) -> &'static str {
        match self {
            Self::Title => "title",
            Self::Heading { .. } => "heading",
            Self::Paragraph => "paragraph",
            Self::ListItem { .. } => "list_item",
            Self::Code => "code",
            Self::Formula => "formula",
            Self::FootnoteDefinition => "footnote_definition",
            Self::FootnoteRef => "footnote_ref",
            Self::Citation => "citation",
            Self::Slide { .. } => "slide",
            Self::DefinitionTerm => "definition_term",
            Self::DefinitionDescription => "definition_description",
            Self::Admonition => "admonition",
            Self::RawBlock => "raw_block",
            Self::MetadataBlock => "metadata_block",
            Self::ListStart { .. } => "list_start",
            Self::ListEnd => "list_end",
            Self::QuoteStart => "quote_start",
            Self::QuoteEnd => "quote_end",
            Self::GroupStart => "group_start",
            Self::GroupEnd => "group_end",
            Self::Table { .. } => "table",
            Self::Image { .. } => "image",
            Self::PageBreak => "page_break",
            Self::OcrText { .. } => "ocr_text",
        }
    }

    /// Returns true if this is a container start marker.
    pub(crate) fn is_container_start(&self) -> bool {
        matches!(self, Self::ListStart { .. } | Self::QuoteStart | Self::GroupStart)
    }

    /// Returns true if this is a container end marker.
    pub(crate) fn is_container_end(&self) -> bool {
        matches!(self, Self::ListEnd | Self::QuoteEnd | Self::GroupEnd)
    }

    /// Returns the matching end marker for a container start, if applicable.
    #[cfg(test)]
    pub(crate) fn matching_end(&self) -> Option<ElementKind> {
        match self {
            Self::ListStart { .. } => Some(Self::ListEnd),
            Self::QuoteStart => Some(Self::QuoteEnd),
            Self::GroupStart => Some(Self::GroupEnd),
            _ => None,
        }
    }
}

// ============================================================================
// Relationships
// ============================================================================

/// A relationship between two elements in the document.
///
/// During extraction, targets may be unresolved keys (`RelationshipTarget::Key`).
/// The derivation step resolves these to indices using the element anchor index.
#[derive(Debug, Clone, PartialEq)]
pub struct Relationship {
    /// Index of the source element in `InternalDocument::elements`.
    pub source: u32,

    /// Target of the relationship (resolved index or unresolved key).
    pub target: RelationshipTarget,

    /// Semantic kind of the relationship.
    pub kind: RelationshipKind,
}

/// Target of a relationship — either a resolved element index or an unresolved key.
#[derive(Debug, Clone, PartialEq)]
pub enum RelationshipTarget {
    /// Resolved: index into `InternalDocument::elements`.
    Index(u32),
    /// Unresolved: key to be matched against element anchors during derivation.
    Key(String),
}

// Re-export RelationshipKind from the public API module where it is defined.
pub use super::document_structure::RelationshipKind;

// Compile-time assertions: these types must be Send + Sync for concurrent extraction.
const _: () = {
    #[allow(dead_code)]
    fn assert_send_sync<T: Send + Sync>() {}
    #[allow(dead_code)]
    fn _check() {
        assert_send_sync::<InternalDocument>();
        assert_send_sync::<InternalElement>();
    }
};

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_internal_element_id_deterministic() {
        let id1 = InternalElementId::generate("heading", "Introduction", Some(1), 0);
        let id2 = InternalElementId::generate("heading", "Introduction", Some(1), 0);
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_internal_element_id_differs_by_index() {
        let id1 = InternalElementId::generate("paragraph", "Same text", Some(1), 0);
        let id2 = InternalElementId::generate("paragraph", "Same text", Some(1), 1);
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_internal_element_id_format() {
        let id = InternalElementId::generate("title", "Hello", None, 0);
        assert!(id.as_str().starts_with("ie-"));
        // 12 hex chars = 6 bytes
        assert_eq!(id.as_str().len(), 3 + 12); // "ie-" + 12 hex
    }

    #[test]
    fn test_element_kind_discriminant() {
        assert_eq!(ElementKind::Title.discriminant(), "title");
        assert_eq!(ElementKind::Heading { level: 2 }.discriminant(), "heading");
        assert_eq!(ElementKind::ListStart { ordered: true }.discriminant(), "list_start");
    }

    #[test]
    fn test_container_markers() {
        assert!(ElementKind::ListStart { ordered: false }.is_container_start());
        assert!(ElementKind::ListEnd.is_container_end());
        assert!(!ElementKind::Paragraph.is_container_start());
        assert_eq!(ElementKind::QuoteStart.matching_end(), Some(ElementKind::QuoteEnd));
    }

    #[test]
    fn test_internal_document_push() {
        let mut doc = InternalDocument::new("markdown");
        let elem = InternalElement::text(ElementKind::Paragraph, "Hello world", 0);
        let idx = doc.push_element(elem);
        assert_eq!(idx, 0);
        assert_eq!(doc.elements.len(), 1);
        assert_eq!(doc.elements[0].text, "Hello world");
    }

    #[test]
    fn test_internal_element_builder_pattern() {
        let elem = InternalElement::text(ElementKind::Heading { level: 2 }, "Methods", 1)
            .with_page(3)
            .with_anchor("methods")
            .with_layer(ContentLayer::Body);

        assert_eq!(elem.text, "Methods");
        assert_eq!(elem.page, Some(3));
        assert_eq!(elem.anchor, Some("methods".to_string()));
        assert_eq!(elem.depth, 1);
    }

    #[test]
    fn test_relationship_kind_serde() {
        let kind = RelationshipKind::FootnoteReference;
        let json = serde_json::to_string(&kind).unwrap();
        assert_eq!(json, "\"footnote_reference\"");

        let parsed: RelationshipKind = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, kind);
    }
}
