//! Document revision types for track-changes metadata.
//!
//! These types surface the change-tracking metadata embedded in document formats
//! such as DOCX (`w:ins`/`w:del`/`w:rPrChange`), ODT (`text:change-*`), and
//! PDF xref chains. They are part of the unconditional public surface — no feature
//! gate required.
//!
//! `DiffLine` and `CellChange` are defined here so `RevisionDelta` can reference
//! them without depending on the `diff` Cargo feature. The `diff` module
//! re-exports them from this module so the `xberg::diff::DiffLine` path
//! continues to work.

use serde::{Deserialize, Serialize};

/// A single line in a unified-diff hunk.
///
/// Defined here (rather than only in `crate::diff`) so `RevisionDelta` can
/// reference it unconditionally, without requiring the `diff` Cargo feature.
/// `crate::diff` re-exports this type verbatim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[serde(tag = "kind", content = "text", rename_all = "snake_case")]
pub enum DiffLine {
    /// Unchanged context line.
    Context(String),
    /// Line added in the "after" version.
    Added(String),
    /// Line removed from the "before" version.
    Removed(String),
}

impl Default for DiffLine {
    /// Returns an empty context line — carries no semantic change, the safest neutral value.
    fn default() -> Self {
        Self::Context(String::new())
    }
}

/// A single changed cell within a table.
///
/// Defined here (rather than only in `crate::diff`) so `RevisionDelta` can
/// reference it unconditionally, without requiring the `diff` Cargo feature.
/// `crate::diff` re-exports this type verbatim.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct CellChange {
    /// Zero-based row index.
    pub row: usize,
    /// Zero-based column index.
    pub col: usize,
    /// Value before the change.
    pub from: String,
    /// Value after the change.
    pub to: String,
}

/// A single run-level or style-level property change.
///
/// Used for revisions that change formatting rather than text content. `from`
/// and `to` store normalized property values when the source format exposes
/// them; either side may be absent when the format only records one side of the
/// change.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct PropertyChange {
    /// Property name, such as `"bold"`, `"italic"`, `"font_size"`, or `"font_color"`.
    pub name: String,
    /// Value before the change, when available.
    pub from: Option<String>,
    /// Value after the change, when available.
    pub to: Option<String>,
}

/// A single tracked change embedded in a document.
///
/// Populated by per-format extractors that understand change-tracking metadata
/// (DOCX `w:ins`/`w:del`/`w:rPrChange`, ODT `text:change-*`, …). Every
/// extractor defaults to `ExtractedDocument.revisions = None` until a
/// format-specific implementation is added.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct DocumentRevision {
    /// Format-specific revision identifier.
    ///
    /// For DOCX this is the `w:id` attribute value on the change element
    /// (e.g. `"42"`). When the attribute is absent a synthetic fallback is
    /// generated (`"docx-ins-0"`, `"docx-del-3"`, …).
    pub revision_id: String,

    /// Display name of the author who made this change, when available.
    pub author: Option<String>,

    /// ISO-8601 timestamp of the change, when available.
    ///
    /// Stored as a plain string so this type remains FFI-friendly and
    /// unconditionally available without the `chrono` optional dep.
    /// DOCX populates this from the `w:date` attribute (e.g.
    /// `"2024-03-15T10:30:00Z"`).
    pub timestamp: Option<String>,

    /// Semantic kind of this revision.
    pub kind: RevisionKind,

    /// Best-effort document location for this revision.
    ///
    /// Resolution is format-dependent and may be `None` when the location
    /// cannot be determined (e.g. changes inside table cells before
    /// table-cell anchor support is added).
    pub anchor: Option<RevisionAnchor>,

    /// The content changes that make up this revision.
    pub delta: RevisionDelta,
}

/// Semantic classification of a tracked change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum RevisionKind {
    /// Text or content was inserted.
    Insertion,
    /// Text or content was deleted.
    Deletion,
    /// Run-level formatting (font, size, colour, …) was changed.
    FormatChange,
    /// A reviewer comment or annotation.
    Comment,
}

/// Best-effort document location for a revision.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum RevisionAnchor {
    /// Body paragraph, identified by its zero-based index in the document flow.
    Paragraph {
        /// Zero-based index of the paragraph in document order.
        index: usize,
    },
    /// Cell inside a table.
    TableCell {
        /// Zero-based row index within the table.
        row: usize,
        /// Zero-based column index within the table.
        col: usize,
        /// Zero-based index of the table in document order.
        table_index: usize,
    },
    /// Page, identified by its zero-based index.
    Page {
        /// Zero-based page index.
        index: usize,
    },
    /// Presentation slide, identified by its zero-based index.
    Slide {
        /// Zero-based slide index.
        index: usize,
    },
    /// Spreadsheet cell or range, identified by sheet index and optional name.
    Sheet {
        /// Zero-based sheet index.
        index: usize,
        /// Sheet display name when available.
        name: Option<String>,
    },
}

impl Default for RevisionAnchor {
    /// Returns `Paragraph { index: 0 }` — the most neutral, zero-cost anchor variant.
    fn default() -> Self {
        Self::Paragraph { index: 0 }
    }
}

/// The content changes that make up a single revision.
///
/// For insertions and deletions the `content` field carries the added/removed
/// lines as `DiffLine::Added` / `DiffLine::Removed` entries. For format
/// changes, `property_changes` carries normalized before/after formatting
/// values when the source document exposes them.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct RevisionDelta {
    /// Line-level content changes for this revision.
    pub content: Vec<DiffLine>,
    /// Cell-level table changes for this revision.
    pub table_changes: Vec<CellChange>,
    /// Formatting or metadata property changes for this revision.
    #[serde(default)]
    pub property_changes: Vec<PropertyChange>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_round_trip_document_revision_with_all_fields() {
        let revision = DocumentRevision {
            revision_id: "42".to_string(),
            author: Some("Alice".to_string()),
            timestamp: Some("2024-03-15T10:30:00Z".to_string()),
            kind: RevisionKind::Insertion,
            anchor: Some(RevisionAnchor::Paragraph { index: 3 }),
            delta: RevisionDelta {
                content: vec![DiffLine::Added("hello world".to_string())],
                ..Default::default()
            },
        };

        let json = serde_json::to_string(&revision).expect("serialization must succeed");
        let deserialized: DocumentRevision = serde_json::from_str(&json).expect("deserialization must succeed");

        assert_eq!(deserialized.revision_id, "42");
        assert_eq!(deserialized.author.as_deref(), Some("Alice"));
        assert_eq!(deserialized.timestamp.as_deref(), Some("2024-03-15T10:30:00Z"));
        assert!(matches!(deserialized.kind, RevisionKind::Insertion));
        assert_eq!(deserialized.delta.content.len(), 1);
        assert!(matches!(&deserialized.delta.content[0], DiffLine::Added(t) if t == "hello world"));
    }

    #[test]
    fn should_round_trip_document_revision_with_minimal_fields() {
        let revision = DocumentRevision {
            revision_id: "docx-del-0".to_string(),
            author: None,
            timestamp: None,
            kind: RevisionKind::Deletion,
            anchor: None,
            delta: RevisionDelta {
                content: vec![DiffLine::Removed("old text".to_string())],
                ..Default::default()
            },
        };

        let json = serde_json::to_string(&revision).expect("serialization must succeed");
        let deserialized: DocumentRevision = serde_json::from_str(&json).expect("deserialization must succeed");

        assert_eq!(deserialized.revision_id, "docx-del-0");
        assert!(deserialized.author.is_none());
        assert!(deserialized.timestamp.is_none());
        assert!(matches!(deserialized.kind, RevisionKind::Deletion));
        assert!(matches!(&deserialized.delta.content[0], DiffLine::Removed(t) if t == "old text"));
    }

    #[test]
    fn should_round_trip_format_change_revision_with_empty_delta() {
        let revision = DocumentRevision {
            revision_id: "docx-fmt-5".to_string(),
            author: Some("Bob".to_string()),
            timestamp: None,
            kind: RevisionKind::FormatChange,
            anchor: Some(RevisionAnchor::Paragraph { index: 0 }),
            delta: RevisionDelta::default(),
        };

        let json = serde_json::to_string(&revision).expect("serialization must succeed");
        let deserialized: DocumentRevision = serde_json::from_str(&json).expect("deserialization must succeed");

        assert!(matches!(deserialized.kind, RevisionKind::FormatChange));
        assert!(deserialized.delta.content.is_empty());
        assert!(deserialized.delta.table_changes.is_empty());
        assert!(deserialized.delta.property_changes.is_empty());
    }

    #[test]
    fn should_round_trip_all_revision_kinds() {
        for kind in [
            RevisionKind::Insertion,
            RevisionKind::Deletion,
            RevisionKind::FormatChange,
            RevisionKind::Comment,
        ] {
            let revision = DocumentRevision {
                revision_id: "test".to_string(),
                author: None,
                timestamp: None,
                kind,
                anchor: None,
                delta: RevisionDelta::default(),
            };
            let json = serde_json::to_string(&revision).expect("serialization must succeed");
            let back: DocumentRevision = serde_json::from_str(&json).expect("deserialization must succeed");
            assert_eq!(back.kind, kind);
        }
    }

    #[test]
    fn should_round_trip_all_revision_anchors() {
        let anchors = vec![
            RevisionAnchor::Paragraph { index: 2 },
            RevisionAnchor::TableCell {
                row: 1,
                col: 3,
                table_index: 0,
            },
            RevisionAnchor::Page { index: 5 },
            RevisionAnchor::Slide { index: 7 },
            RevisionAnchor::Sheet {
                index: 2,
                name: Some("Q1".to_string()),
            },
        ];

        for anchor in anchors {
            let revision = DocumentRevision {
                revision_id: "test".to_string(),
                author: None,
                timestamp: None,
                kind: RevisionKind::Insertion,
                anchor: Some(anchor),
                delta: RevisionDelta::default(),
            };
            let json = serde_json::to_string(&revision).expect("serialization must succeed");
            let back: DocumentRevision = serde_json::from_str(&json).expect("deserialization must succeed");
            assert!(back.anchor.is_some());
        }
    }

    #[test]
    fn should_round_trip_cell_change_in_revision_delta() {
        let revision = DocumentRevision {
            revision_id: "tbl-1".to_string(),
            author: None,
            timestamp: None,
            kind: RevisionKind::Insertion,
            anchor: Some(RevisionAnchor::TableCell {
                row: 0,
                col: 1,
                table_index: 0,
            }),
            delta: RevisionDelta {
                content: vec![],
                table_changes: vec![CellChange {
                    row: 0,
                    col: 1,
                    from: "old".to_string(),
                    to: "new".to_string(),
                }],
                property_changes: vec![],
            },
        };

        let json = serde_json::to_string(&revision).expect("serialization must succeed");
        let back: DocumentRevision = serde_json::from_str(&json).expect("deserialization must succeed");
        assert_eq!(back.delta.table_changes.len(), 1);
        assert_eq!(back.delta.table_changes[0].from, "old");
        assert_eq!(back.delta.table_changes[0].to, "new");
    }

    #[test]
    fn should_round_trip_property_change_in_revision_delta() {
        let revision = DocumentRevision {
            revision_id: "fmt-1".to_string(),
            author: None,
            timestamp: None,
            kind: RevisionKind::FormatChange,
            anchor: Some(RevisionAnchor::Paragraph { index: 0 }),
            delta: RevisionDelta {
                property_changes: vec![PropertyChange {
                    name: "bold".to_string(),
                    from: Some("true".to_string()),
                    to: Some("false".to_string()),
                }],
                ..Default::default()
            },
        };

        let json = serde_json::to_string(&revision).expect("serialization must succeed");
        let back: DocumentRevision = serde_json::from_str(&json).expect("deserialization must succeed");
        assert_eq!(back.delta.property_changes.len(), 1);
        assert_eq!(back.delta.property_changes[0].name, "bold");
        assert_eq!(back.delta.property_changes[0].from.as_deref(), Some("true"));
        assert_eq!(back.delta.property_changes[0].to.as_deref(), Some("false"));
    }
}
