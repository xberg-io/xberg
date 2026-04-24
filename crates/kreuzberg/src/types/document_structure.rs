//! Structured document model types.
//!
//! This module provides a hierarchical, tree-based representation of document content.
//! It uses a flat `Vec<DocumentNode>` with index-based parent/child references for
//! efficient traversal and compact serialization.
//!
//! # Design
//!
//! - **Flat array storage**: All nodes stored in `Vec<DocumentNode>` in reading order
//! - **Index-based references**: `NodeIndex(u32)` for parent/child links
//! - **Tagged enum content**: `NodeContent` with `#[serde(tag = "node_type")]`
//! - **Content layer classification**: Each node tagged as Body, Header, Footer, or Footnote
//! - **Deterministic IDs**: `NodeId` generated from content hash for diffing/caching

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::extraction::BoundingBox;

// ============================================================================
// Index and ID Types
// ============================================================================

/// Newtype for node indices into the `DocumentStructure::nodes` array.
///
/// Uses `u32` for cross-platform consistency (WASM is 32-bit) and to avoid
/// confusion with page numbers or other `usize` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "api", schema(value_type = u32))]
pub struct NodeIndex(pub u32);

/// Deterministic node identifier.
///
/// Generated from a hash of `node_type + text + page`. The same document
/// always produces the same IDs, making them useful for diffing, caching,
/// and external references.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "api", schema(value_type = String))]
pub struct NodeId(String);

impl NodeId {
    /// Create a `NodeId` from a pre-computed hash string.
    pub(crate) fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Generate a deterministic `NodeId` from node content.
    ///
    /// Uses wrapping multiplication hashing on the node type discriminant,
    /// text content, page number, and node index to produce a stable hex identifier.
    /// The index parameter ensures uniqueness for duplicate content on the same page.
    ///
    /// # Parameters
    ///
    /// - `node_type`: The node type discriminant (e.g., "paragraph", "heading")
    /// - `text`: The text content of the node
    /// - `page`: The page number (None becomes u64::MAX for hashing)
    /// - `index`: The position of this node in the document's nodes array
    pub(crate) fn generate(node_type: &str, text: &str, page: Option<u32>, index: u32) -> Self {
        let type_hash = node_type
            .bytes()
            .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));

        let text_hash = text
            .bytes()
            .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));

        // Use u64::MAX as sentinel for None to distinguish from Some(0)
        let page_hash = page.map(|p| p as u64).unwrap_or(u64::MAX);

        let combined = type_hash
            .wrapping_mul(65599)
            .wrapping_add(text_hash)
            .wrapping_mul(65599)
            .wrapping_add(page_hash)
            .wrapping_mul(65599)
            .wrapping_add(index as u64);

        Self(format!("node-{:x}", combined))
    }
}

impl AsRef<str> for NodeId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ============================================================================
// Document Structure
// ============================================================================

/// Top-level structured document representation.
///
/// A flat array of nodes with index-based parent/child references forming a tree.
/// Root-level nodes have `parent: None`. Use `body_roots()` and `furniture_roots()`
/// to iterate over top-level content by layer.
///
/// # Validation
///
/// Call `validate()` after construction to verify all node indices are in bounds
/// and parent-child relationships are bidirectionally consistent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "api", schema(no_recursion))]
pub struct DocumentStructure {
    /// All nodes in document/reading order.
    pub nodes: Vec<DocumentNode>,

    /// Origin format identifier (e.g. "docx", "pptx", "html", "pdf").
    ///
    /// Allows renderers to apply format-aware heuristics when converting
    /// the document tree to output formats.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_format: Option<String>,

    /// Resolved relationships between nodes (footnote refs, citations, anchor links, etc.).
    ///
    /// Populated during derivation from the internal document representation.
    /// Empty when no relationships are detected.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub relationships: Vec<DocumentRelationship>,
}

/// A resolved relationship between two nodes in the document tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct DocumentRelationship {
    /// Source node index (the referencing node).
    pub source: NodeIndex,
    /// Target node index (the referenced node).
    pub target: NodeIndex,
    /// Semantic kind of the relationship.
    pub kind: RelationshipKind,
}

/// Semantic kind of a relationship between document elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum RelationshipKind {
    /// Footnote marker -> footnote definition.
    FootnoteReference,
    /// Citation marker -> bibliography entry.
    CitationReference,
    /// Internal anchor link (`#id`) -> target heading/element.
    InternalLink,
    /// Caption paragraph -> figure/table it describes.
    Caption,
    /// Label -> labeled element (HTML `<label for>`, LaTeX `\label{}`).
    Label,
    /// TOC entry -> target section.
    TocEntry,
    /// Cross-reference (LaTeX `\ref{}`, DOCX cross-reference field).
    CrossReference,
}

/// A single node in the document tree.
///
/// Each node has deterministic `id`, typed `content`, optional `parent`/`children`
/// for tree structure, and metadata like page number, bounding box, and content layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct DocumentNode {
    /// Deterministic identifier (hash of content + position).
    pub id: NodeId,

    /// Node content — tagged enum, type-specific data only.
    pub content: NodeContent,

    /// Parent node index (`None` = root-level node).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<NodeIndex>,

    /// Child node indices in reading order.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub children: Vec<NodeIndex>,

    /// Content layer classification.
    #[serde(default, skip_serializing_if = "ContentLayer::is_default")]
    pub content_layer: ContentLayer,

    /// Page number where this node starts (1-indexed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,

    /// Page number where this node ends (for multi-page tables/sections).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_end: Option<u32>,

    /// Bounding box in document coordinates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bbox: Option<BoundingBox>,

    /// Inline annotations (formatting, links) on this node's text content.
    ///
    /// Only meaningful for text-carrying nodes; empty for containers.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub annotations: Vec<TextAnnotation>,

    /// Format-specific key-value attributes.
    ///
    /// Extensible bag for data that doesn't warrant a typed field: CSS classes,
    /// LaTeX environment names, Excel cell formulas, slide layout names, etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<HashMap<String, String>>,
}

// ============================================================================
// Content Layer
// ============================================================================

/// Content layer classification for document nodes.
///
/// Replaces separate body/furniture arrays with per-node granularity.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum ContentLayer {
    /// Main document body content.
    #[default]
    Body,
    /// Page/section header (running header).
    Header,
    /// Page/section footer (running footer).
    Footer,
    /// Footnote content.
    Footnote,
}

impl ContentLayer {
    /// Check if this is the default content layer (Body).
    pub(crate) fn is_default(&self) -> bool {
        *self == ContentLayer::Body
    }
}

// ============================================================================
// Node Content (Tagged Enum)
// ============================================================================

/// Tagged enum for node content. Each variant carries only type-specific data.
///
/// Uses `#[serde(tag = "node_type")]` to avoid "type" keyword collision in
/// Go/Java/TypeScript bindings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[serde(tag = "node_type", rename_all = "snake_case")]
pub enum NodeContent {
    /// Document title.
    Title { text: String },

    /// Section heading with level (1-6).
    Heading { level: u8, text: String },

    /// Body text paragraph.
    Paragraph { text: String },

    /// List container — children are `ListItem` nodes.
    List { ordered: bool },

    /// Individual list item.
    ListItem { text: String },

    /// Table with structured cell grid.
    Table { grid: TableGrid },

    /// Image reference.
    Image {
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        image_index: Option<u32>,
        /// Source URL or path of the image (from `<img src="...">` or `![](src)`).
        #[serde(skip_serializing_if = "Option::is_none")]
        src: Option<String>,
    },

    /// Code block.
    Code {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        language: Option<String>,
    },

    /// Block quote — container, children carry the quoted content.
    Quote,

    /// Mathematical formula / equation.
    Formula { text: String },

    /// Footnote reference content.
    Footnote { text: String },

    /// Logical grouping container (section, key-value area).
    ///
    /// `heading_level` + `heading_text` capture the section heading directly
    /// rather than relying on a first-child positional convention.
    Group {
        #[serde(skip_serializing_if = "Option::is_none")]
        label: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        heading_level: Option<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        heading_text: Option<String>,
    },

    /// Page break marker.
    PageBreak,

    /// Presentation slide container — children are the slide's content nodes.
    Slide {
        /// 1-indexed slide number.
        number: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
    },

    /// Definition list container — children are `DefinitionItem` nodes.
    DefinitionList,

    /// Individual definition list entry with term and definition.
    DefinitionItem { term: String, definition: String },

    /// Citation or bibliographic reference.
    Citation { key: String, text: String },

    /// Admonition / callout container (note, warning, tip, etc.).
    ///
    /// Children carry the admonition body content.
    Admonition {
        /// Kind of admonition (e.g. "note", "warning", "tip", "danger").
        kind: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
    },

    /// Raw block preserved verbatim from the source format.
    ///
    /// Used for content that cannot be mapped to a semantic node type
    /// (e.g. JSX in MDX, raw LaTeX in markdown, embedded HTML).
    RawBlock {
        /// Source format identifier (e.g. "html", "latex", "jsx").
        format: String,
        content: String,
    },

    /// Structured metadata block (email headers, YAML frontmatter, etc.).
    MetadataBlock { entries: Vec<(String, String)> },
}

// ============================================================================
// Table Grid
// ============================================================================

/// Structured table grid with cell-level metadata.
///
/// Stores row/column dimensions and a flat list of cells with position info.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct TableGrid {
    /// Number of rows in the table.
    pub rows: u32,
    /// Number of columns in the table.
    pub cols: u32,
    /// All cells in row-major order.
    pub cells: Vec<GridCell>,
}

/// Individual grid cell with position and span metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct GridCell {
    /// Cell text content.
    pub content: String,
    /// Zero-indexed row position.
    pub row: u32,
    /// Zero-indexed column position.
    pub col: u32,
    /// Number of rows this cell spans.
    #[serde(default = "default_span")]
    pub row_span: u32,
    /// Number of columns this cell spans.
    #[serde(default = "default_span")]
    pub col_span: u32,
    /// Whether this is a header cell.
    #[serde(default)]
    pub is_header: bool,
    /// Bounding box for this cell (if available).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bbox: Option<BoundingBox>,
}

fn default_span() -> u32 {
    1
}

// ============================================================================
// Text Annotations
// ============================================================================

/// Inline text annotation — byte-range based formatting and links.
///
/// Annotations reference byte offsets into the node's text content,
/// enabling precise identification of formatted regions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct TextAnnotation {
    /// Start byte offset in the node's text content (inclusive).
    pub start: u32,
    /// End byte offset in the node's text content (exclusive).
    pub end: u32,
    /// Annotation type.
    pub kind: AnnotationKind,
}

/// Types of inline text annotations.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[serde(tag = "annotation_type", rename_all = "snake_case")]
pub enum AnnotationKind {
    #[default]
    Bold,
    Italic,
    Underline,
    Strikethrough,
    Code,
    Subscript,
    Superscript,
    Link {
        url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
    },
    /// Highlighted text (PDF highlights, HTML `<mark>`).
    Highlight,
    /// Text color (CSS-compatible value, e.g. "#ff0000", "red").
    Color {
        value: String,
    },
    /// Font size with units (e.g. "12pt", "1.2em", "16px").
    FontSize {
        value: String,
    },
    /// Extensible annotation for format-specific styling.
    Custom {
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<String>,
    },
}

// ============================================================================
// BoundingBox Conversions
// ============================================================================

/// Convert PDF hierarchy's `(f32, f32, f32, f32)` bounding box to canonical `BoundingBox`.
///
/// The tuple order is `(left, top, right, bottom)` matching the PDF coordinate convention.
impl From<(f32, f32, f32, f32)> for BoundingBox {
    fn from((left, top, right, bottom): (f32, f32, f32, f32)) -> Self {
        BoundingBox {
            x0: left as f64,
            y0: top as f64,
            x1: right as f64,
            y1: bottom as f64,
        }
    }
}

impl Default for NodeContent {
    fn default() -> Self {
        NodeContent::Paragraph { text: String::new() }
    }
}

// ============================================================================
// NodeContent Helpers
// ============================================================================

impl NodeContent {
    /// Get the primary text content of this node, if it carries text.
    ///
    /// Text-carrying nodes: `Title`, `Heading`, `Paragraph`, `ListItem`, `Code`,
    /// `Formula`, `Footnote`, `Citation` (returns text), `RawBlock` (returns content),
    /// `DefinitionItem` (returns term only, not definition).
    ///
    /// Container/marker nodes return `None`: `List`, `Quote`, `Group`, `PageBreak`,
    /// `Slide`, `DefinitionList`, `Admonition`, `MetadataBlock`.
    pub(crate) fn text(&self) -> Option<&str> {
        match self {
            NodeContent::Title { text }
            | NodeContent::Heading { text, .. }
            | NodeContent::Paragraph { text }
            | NodeContent::ListItem { text }
            | NodeContent::Code { text, .. }
            | NodeContent::Formula { text }
            | NodeContent::Footnote { text }
            | NodeContent::Citation { text, .. }
            | NodeContent::RawBlock { content: text, .. } => Some(text),
            NodeContent::DefinitionItem { term, .. } => Some(term),
            NodeContent::Table { .. }
            | NodeContent::Image { .. }
            | NodeContent::List { .. }
            | NodeContent::Quote
            | NodeContent::Group { .. }
            | NodeContent::PageBreak
            | NodeContent::Slide { .. }
            | NodeContent::DefinitionList
            | NodeContent::Admonition { .. }
            | NodeContent::MetadataBlock { .. } => None,
        }
    }

    /// Get the serde tag discriminant string for this variant.
    pub(crate) fn node_type_str(&self) -> &'static str {
        match self {
            NodeContent::Title { .. } => "title",
            NodeContent::Heading { .. } => "heading",
            NodeContent::Paragraph { .. } => "paragraph",
            NodeContent::List { .. } => "list",
            NodeContent::ListItem { .. } => "list_item",
            NodeContent::Table { .. } => "table",
            NodeContent::Image { .. } => "image",
            NodeContent::Code { .. } => "code",
            NodeContent::Quote => "quote",
            NodeContent::Formula { .. } => "formula",
            NodeContent::Footnote { .. } => "footnote",
            NodeContent::Group { .. } => "group",
            NodeContent::PageBreak => "page_break",
            NodeContent::Slide { .. } => "slide",
            NodeContent::DefinitionList => "definition_list",
            NodeContent::DefinitionItem { .. } => "definition_item",
            NodeContent::Citation { .. } => "citation",
            NodeContent::Admonition { .. } => "admonition",
            NodeContent::RawBlock { .. } => "raw_block",
            NodeContent::MetadataBlock { .. } => "metadata_block",
        }
    }
}

// ============================================================================
// DocumentStructure Methods
// ============================================================================

impl DocumentStructure {
    /// Create an empty `DocumentStructure`.
    pub(crate) fn new() -> Self {
        Self {
            nodes: Vec::new(),
            source_format: None,
            relationships: Vec::new(),
        }
    }

    /// Create a `DocumentStructure` with pre-allocated capacity.
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            nodes: Vec::with_capacity(capacity),
            source_format: None,
            relationships: Vec::new(),
        }
    }

    /// Push a node and return its `NodeIndex`.
    pub(crate) fn push_node(&mut self, node: DocumentNode) -> NodeIndex {
        let idx = NodeIndex(self.nodes.len() as u32);
        self.nodes.push(node);
        idx
    }

    /// Add a child to an existing parent node.
    ///
    /// Updates both the parent's `children` list and the child's `parent` field.
    ///
    /// # Panics
    ///
    /// Panics if either index is out of bounds.
    pub(crate) fn add_child(&mut self, parent: NodeIndex, child: NodeIndex) {
        self.nodes[parent.0 as usize].children.push(child);
        self.nodes[child.0 as usize].parent = Some(parent);
    }

    /// Validate all node indices are in bounds and parent-child relationships
    /// are bidirectionally consistent.
    ///
    /// # Errors
    ///
    /// Returns a descriptive error string if validation fails.
    pub(crate) fn validate(&self) -> std::result::Result<(), String> {
        let len = self.nodes.len() as u32;

        for (i, node) in self.nodes.iter().enumerate() {
            let idx = i as u32;

            // Validate parent reference
            if let Some(parent) = node.parent {
                if parent.0 >= len {
                    return Err(format!(
                        "Node {} has parent index {} which is out of bounds (len={})",
                        idx, parent.0, len
                    ));
                }
                if !self.nodes[parent.0 as usize].children.contains(&NodeIndex(idx)) {
                    return Err(format!(
                        "Node {} claims parent {}, but parent's children list does not contain {}",
                        idx, parent.0, idx
                    ));
                }
            }

            // Validate child references
            for child in &node.children {
                if child.0 >= len {
                    return Err(format!(
                        "Node {} has child index {} which is out of bounds (len={})",
                        idx, child.0, len
                    ));
                }
                if self.nodes[child.0 as usize].parent != Some(NodeIndex(idx)) {
                    return Err(format!(
                        "Node {} lists child {}, but child's parent is {:?} instead of {}",
                        idx, child.0, self.nodes[child.0 as usize].parent, idx
                    ));
                }
            }
        }

        Ok(())
    }

    /// Iterate over root-level body nodes (content_layer == Body, parent == None).
    #[cfg(test)]
    pub(crate) fn body_roots(&self) -> impl Iterator<Item = (NodeIndex, &DocumentNode)> {
        self.nodes.iter().enumerate().filter_map(|(i, node)| {
            if node.parent.is_none() && node.content_layer == ContentLayer::Body {
                Some((NodeIndex(i as u32), node))
            } else {
                None
            }
        })
    }

    /// Iterate over root-level furniture nodes (non-Body content_layer, parent == None).
    #[cfg(test)]
    pub(crate) fn furniture_roots(&self) -> impl Iterator<Item = (NodeIndex, &DocumentNode)> {
        self.nodes.iter().enumerate().filter_map(|(i, node)| {
            if node.parent.is_none() && node.content_layer != ContentLayer::Body {
                Some((NodeIndex(i as u32), node))
            } else {
                None
            }
        })
    }

    /// Get a node by index.
    pub(crate) fn get(&self, index: NodeIndex) -> Option<&DocumentNode> {
        self.nodes.get(index.0 as usize)
    }

    /// Get the total number of nodes.
    pub(crate) fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Check if the document structure is empty.
    pub(crate) fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

impl Default for DocumentStructure {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_paragraph(text: &str, page: Option<u32>, index: u32) -> DocumentNode {
        let content = NodeContent::Paragraph { text: text.to_string() };
        DocumentNode {
            id: NodeId::generate(content.node_type_str(), text, page, index),
            content,
            parent: None,
            children: vec![],
            content_layer: ContentLayer::Body,
            page,
            page_end: None,
            bbox: None,
            annotations: vec![],
            attributes: None,
        }
    }

    #[test]
    fn test_empty_document_validates() {
        let doc = DocumentStructure::new();
        assert!(doc.validate().is_ok());
        assert!(doc.is_empty());
        assert_eq!(doc.len(), 0);
    }

    #[test]
    fn test_single_node_validates() {
        let mut doc = DocumentStructure::new();
        doc.push_node(make_paragraph("Hello world", Some(1), 0));
        assert!(doc.validate().is_ok());
        assert_eq!(doc.len(), 1);
    }

    #[test]
    fn test_parent_child_relationship() {
        let mut doc = DocumentStructure::new();

        let group_content = NodeContent::Group {
            label: None,
            heading_level: Some(1),
            heading_text: Some("Section 1".to_string()),
        };
        let group = DocumentNode {
            id: NodeId::generate("group", "Section 1", Some(1), 0),
            content: group_content,
            parent: None,
            children: vec![],
            content_layer: ContentLayer::Body,
            page: Some(1),
            page_end: None,
            bbox: None,
            annotations: vec![],
            attributes: None,
        };
        let group_idx = doc.push_node(group);

        let child = make_paragraph("Child paragraph", Some(1), 1);
        let child_idx = doc.push_node(child);

        doc.add_child(group_idx, child_idx);

        assert!(doc.validate().is_ok());
        assert_eq!(doc.nodes[0].children.len(), 1);
        assert_eq!(doc.nodes[1].parent, Some(NodeIndex(0)));
    }

    #[test]
    fn test_validation_catches_bad_parent() {
        let mut doc = DocumentStructure::new();
        let mut node = make_paragraph("Bad parent", Some(1), 0);
        node.parent = Some(NodeIndex(99)); // Out of bounds
        doc.push_node(node);

        assert!(doc.validate().is_err());
    }

    #[test]
    fn test_validation_catches_inconsistent_parent_child() {
        let mut doc = DocumentStructure::new();

        // Parent node with no children listed
        let parent = DocumentNode {
            id: NodeId::generate("group", "", Some(1), 0),
            content: NodeContent::Group {
                label: None,
                heading_level: None,
                heading_text: None,
            },
            parent: None,
            children: vec![], // No children listed
            content_layer: ContentLayer::Body,
            page: Some(1),
            page_end: None,
            bbox: None,
            annotations: vec![],
            attributes: None,
        };
        doc.push_node(parent);

        // Child claims parent, but parent doesn't list it
        let mut child = make_paragraph("Orphan child", Some(1), 1);
        child.parent = Some(NodeIndex(0));
        doc.push_node(child);

        assert!(doc.validate().is_err());
    }

    #[test]
    fn test_validation_catches_bad_child() {
        let mut doc = DocumentStructure::new();

        let parent = DocumentNode {
            id: NodeId::generate("group", "", Some(1), 0),
            content: NodeContent::Group {
                label: None,
                heading_level: None,
                heading_text: None,
            },
            parent: None,
            children: vec![NodeIndex(99)], // Out of bounds child
            content_layer: ContentLayer::Body,
            page: Some(1),
            page_end: None,
            bbox: None,
            annotations: vec![],
            attributes: None,
        };
        doc.push_node(parent);

        assert!(doc.validate().is_err());
    }

    #[test]
    fn test_body_and_furniture_roots() {
        let mut doc = DocumentStructure::new();

        // Body node
        doc.push_node(make_paragraph("Body content", Some(1), 0));

        // Header node
        let mut header = make_paragraph("Page header", Some(1), 1);
        header.content_layer = ContentLayer::Header;
        doc.push_node(header);

        // Footer node
        let mut footer = make_paragraph("Page footer", Some(1), 2);
        footer.content_layer = ContentLayer::Footer;
        doc.push_node(footer);

        assert!(doc.validate().is_ok());

        let body: Vec<_> = doc.body_roots().collect();
        assert_eq!(body.len(), 1);

        let furniture: Vec<_> = doc.furniture_roots().collect();
        assert_eq!(furniture.len(), 2);
    }

    #[test]
    fn test_node_id_deterministic() {
        let id1 = NodeId::generate("paragraph", "Hello world", Some(1), 0);
        let id2 = NodeId::generate("paragraph", "Hello world", Some(1), 0);
        assert_eq!(id1, id2);

        let id3 = NodeId::generate("paragraph", "Different text", Some(1), 0);
        assert_ne!(id1, id3);

        let id4 = NodeId::generate("paragraph", "Hello world", Some(2), 0);
        assert_ne!(id1, id4);

        let id5 = NodeId::generate("heading", "Hello world", Some(1), 0);
        assert_ne!(id1, id5);

        // Same content, different index → different ID (ensures uniqueness)
        let id6 = NodeId::generate("paragraph", "Hello world", Some(1), 1);
        assert_ne!(id1, id6);

        // None vs Some(0) should produce different IDs (sentinel value)
        let id_none = NodeId::generate("paragraph", "Hello world", None, 0);
        let id_some_0 = NodeId::generate("paragraph", "Hello world", Some(0), 0);
        assert_ne!(id_none, id_some_0);
    }

    #[test]
    fn test_node_content_text() {
        assert_eq!(
            NodeContent::Paragraph {
                text: "Hello".to_string()
            }
            .text(),
            Some("Hello")
        );
        assert_eq!(
            NodeContent::Title {
                text: "Title".to_string()
            }
            .text(),
            Some("Title")
        );
        assert_eq!(
            NodeContent::Heading {
                level: 1,
                text: "H1".to_string()
            }
            .text(),
            Some("H1")
        );
        assert_eq!(NodeContent::PageBreak.text(), None);
        assert_eq!(NodeContent::Quote.text(), None);
        assert_eq!(
            NodeContent::Group {
                label: None,
                heading_level: None,
                heading_text: None
            }
            .text(),
            None
        );

        // New variants
        assert_eq!(
            NodeContent::Slide {
                number: 1,
                title: Some("Slide".to_string())
            }
            .text(),
            None
        );
        assert_eq!(NodeContent::DefinitionList.text(), None);
        assert_eq!(
            NodeContent::DefinitionItem {
                term: "Term".to_string(),
                definition: "Def".to_string()
            }
            .text(),
            Some("Term")
        );
        assert_eq!(
            NodeContent::Citation {
                key: "k".to_string(),
                text: "Text".to_string()
            }
            .text(),
            Some("Text")
        );
        assert_eq!(
            NodeContent::Admonition {
                kind: "note".to_string(),
                title: None
            }
            .text(),
            None
        );
        assert_eq!(
            NodeContent::RawBlock {
                format: "html".to_string(),
                content: "<b>hi</b>".to_string()
            }
            .text(),
            Some("<b>hi</b>")
        );
        assert_eq!(
            NodeContent::MetadataBlock {
                entries: vec![("k".to_string(), "v".to_string())]
            }
            .text(),
            None
        );
    }

    #[test]
    fn test_new_node_type_str() {
        assert_eq!(NodeContent::Slide { number: 1, title: None }.node_type_str(), "slide");
        assert_eq!(NodeContent::DefinitionList.node_type_str(), "definition_list");
        assert_eq!(
            NodeContent::DefinitionItem {
                term: "t".to_string(),
                definition: "d".to_string()
            }
            .node_type_str(),
            "definition_item"
        );
        assert_eq!(
            NodeContent::Citation {
                key: "k".to_string(),
                text: "t".to_string()
            }
            .node_type_str(),
            "citation"
        );
        assert_eq!(
            NodeContent::Admonition {
                kind: "note".to_string(),
                title: None
            }
            .node_type_str(),
            "admonition"
        );
        assert_eq!(
            NodeContent::RawBlock {
                format: "html".to_string(),
                content: "x".to_string()
            }
            .node_type_str(),
            "raw_block"
        );
        assert_eq!(
            NodeContent::MetadataBlock { entries: vec![] }.node_type_str(),
            "metadata_block"
        );
    }

    #[test]
    fn test_new_annotation_serde_roundtrip() {
        // Highlight
        let ann = TextAnnotation {
            start: 0,
            end: 5,
            kind: AnnotationKind::Highlight,
        };
        let json = serde_json::to_string(&ann).expect("serialize");
        let de: TextAnnotation = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(de.kind, AnnotationKind::Highlight);

        // Color
        let ann = TextAnnotation {
            start: 0,
            end: 5,
            kind: AnnotationKind::Color {
                value: "#ff0000".to_string(),
            },
        };
        let json = serde_json::to_string(&ann).expect("serialize");
        let de: TextAnnotation = serde_json::from_str(&json).expect("deserialize");
        match &de.kind {
            AnnotationKind::Color { value } => assert_eq!(value, "#ff0000"),
            _ => panic!("Expected Color"),
        }

        // FontSize
        let ann = TextAnnotation {
            start: 0,
            end: 5,
            kind: AnnotationKind::FontSize {
                value: "12pt".to_string(),
            },
        };
        let json = serde_json::to_string(&ann).expect("serialize");
        let de: TextAnnotation = serde_json::from_str(&json).expect("deserialize");
        match &de.kind {
            AnnotationKind::FontSize { value } => assert_eq!(value, "12pt"),
            _ => panic!("Expected FontSize"),
        }

        // Custom
        let ann = TextAnnotation {
            start: 0,
            end: 5,
            kind: AnnotationKind::Custom {
                name: "bg-color".to_string(),
                value: Some("yellow".to_string()),
            },
        };
        let json = serde_json::to_string(&ann).expect("serialize");
        let de: TextAnnotation = serde_json::from_str(&json).expect("deserialize");
        match &de.kind {
            AnnotationKind::Custom { name, value } => {
                assert_eq!(name, "bg-color");
                assert_eq!(value.as_deref(), Some("yellow"));
            }
            _ => panic!("Expected Custom"),
        }
    }

    #[test]
    fn test_new_node_content_serde_roundtrip() {
        // Slide
        let content = NodeContent::Slide {
            number: 3,
            title: Some("My Slide".to_string()),
        };
        let json = serde_json::to_value(&content).expect("serialize");
        assert_eq!(json.get("node_type").unwrap(), "slide");
        assert_eq!(json.get("number").unwrap(), 3);
        assert_eq!(json.get("title").unwrap(), "My Slide");

        // Citation
        let content = NodeContent::Citation {
            key: "doe2024".to_string(),
            text: "Doe (2024)".to_string(),
        };
        let json = serde_json::to_value(&content).expect("serialize");
        assert_eq!(json.get("node_type").unwrap(), "citation");
        assert_eq!(json.get("key").unwrap(), "doe2024");

        // MetadataBlock
        let content = NodeContent::MetadataBlock {
            entries: vec![
                ("From".to_string(), "alice@example.com".to_string()),
                ("Subject".to_string(), "Hello".to_string()),
            ],
        };
        let json = serde_json::to_value(&content).expect("serialize");
        assert_eq!(json.get("node_type").unwrap(), "metadata_block");
        let entries = json.get("entries").unwrap().as_array().unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_serde_roundtrip() {
        let mut doc = DocumentStructure::new();

        let group_content = NodeContent::Group {
            label: Some("section".to_string()),
            heading_level: Some(1),
            heading_text: Some("Introduction".to_string()),
        };
        let group = DocumentNode {
            id: NodeId::generate("group", "Introduction", Some(1), 0),
            content: group_content,
            parent: None,
            children: vec![],
            content_layer: ContentLayer::Body,
            page: Some(1),
            page_end: None,
            bbox: Some(BoundingBox {
                x0: 10.0,
                y0: 20.0,
                x1: 500.0,
                y1: 50.0,
            }),
            annotations: vec![],
            attributes: None,
        };
        let group_idx = doc.push_node(group);

        let para_content = NodeContent::Paragraph {
            text: "Hello world".to_string(),
        };
        let para = DocumentNode {
            id: NodeId::generate("paragraph", "Hello world", Some(1), 1),
            content: para_content,
            parent: None,
            children: vec![],
            content_layer: ContentLayer::Body,
            page: Some(1),
            page_end: None,
            bbox: None,
            annotations: vec![TextAnnotation {
                start: 0,
                end: 5,
                kind: AnnotationKind::Bold,
            }],
            attributes: None,
        };
        let para_idx = doc.push_node(para);
        doc.add_child(group_idx, para_idx);

        assert!(doc.validate().is_ok());

        let json = serde_json::to_string(&doc).expect("serialize");
        let deserialized: DocumentStructure = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(deserialized.len(), 2);
        assert!(deserialized.validate().is_ok());
        assert_eq!(deserialized.nodes[0].children.len(), 1);
        assert_eq!(deserialized.nodes[1].parent, Some(NodeIndex(0)));
    }

    #[test]
    fn test_serde_node_type_tag() {
        let content = NodeContent::Heading {
            level: 2,
            text: "My Heading".to_string(),
        };
        let json = serde_json::to_value(&content).expect("serialize");

        assert_eq!(json.get("node_type").unwrap(), "heading");
        assert_eq!(json.get("level").unwrap(), 2);
        assert_eq!(json.get("text").unwrap(), "My Heading");
    }

    #[test]
    fn test_serde_annotation_roundtrip() {
        let annotation = TextAnnotation {
            start: 10,
            end: 20,
            kind: AnnotationKind::Link {
                url: "https://example.com".to_string(),
                title: Some("Example".to_string()),
            },
        };

        let json = serde_json::to_string(&annotation).expect("serialize");
        let deserialized: TextAnnotation = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(deserialized.start, 10);
        assert_eq!(deserialized.end, 20);
        match &deserialized.kind {
            AnnotationKind::Link { url, title } => {
                assert_eq!(url, "https://example.com");
                assert_eq!(title.as_deref(), Some("Example"));
            }
            _ => panic!("Expected Link annotation"),
        }
    }

    #[test]
    fn test_table_grid_serde() {
        let grid = TableGrid {
            rows: 2,
            cols: 3,
            cells: vec![
                GridCell {
                    content: "Header 1".to_string(),
                    row: 0,
                    col: 0,
                    row_span: 1,
                    col_span: 1,
                    is_header: true,
                    bbox: None,
                },
                GridCell {
                    content: "Cell 1".to_string(),
                    row: 1,
                    col: 0,
                    row_span: 1,
                    col_span: 1,
                    is_header: false,
                    bbox: None,
                },
            ],
        };

        let json = serde_json::to_string(&grid).expect("serialize");
        let deserialized: TableGrid = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(deserialized.rows, 2);
        assert_eq!(deserialized.cols, 3);
        assert_eq!(deserialized.cells.len(), 2);
        assert!(deserialized.cells[0].is_header);
        assert!(!deserialized.cells[1].is_header);
    }

    #[test]
    fn test_content_layer_default() {
        let layer: ContentLayer = Default::default();
        assert_eq!(layer, ContentLayer::Body);
    }

    #[test]
    fn test_bounding_box_from_f32_tuple() {
        let bbox: BoundingBox = (10.5f32, 20.5f32, 100.5f32, 200.5f32).into();
        assert!((bbox.x0 - 10.5).abs() < f64::EPSILON);
        assert!((bbox.y0 - 20.5).abs() < f64::EPSILON);
        assert!((bbox.x1 - 100.5).abs() < f64::EPSILON);
        assert!((bbox.y1 - 200.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_skip_serializing_empty_fields() {
        let node = make_paragraph("Simple", Some(1), 0);
        let json = serde_json::to_value(&node).expect("serialize");

        // These should be skipped when empty/None
        assert!(json.get("parent").is_none());
        assert!(json.get("children").is_none());
        assert!(json.get("page_end").is_none());
        assert!(json.get("bbox").is_none());
        assert!(json.get("annotations").is_none());
        assert!(json.get("attributes").is_none());

        // These should be present
        assert!(json.get("id").is_some());
        assert!(json.get("content").is_some());
        assert!(json.get("page").is_some());
    }
}
