//! Types for extraction result diffs.
//!
//! `DiffLine` and `CellChange` are canonical definitions live in
//! `crate::types::revisions` so that `RevisionDelta` can reference them
//! unconditionally without the `diff` feature gate. They are re-exported
//! here so the `crate::diff::DiffLine` path continues to work for callers
//! who import them through the `diff` feature.

use serde::{Deserialize, Serialize};

use crate::types::{extraction::ArchiveEntry, tables::Table};

// Re-export from the unconditional types module so the `diff` feature's
// public path (`kreuzberg::diff::DiffLine`, `kreuzberg::diff::CellChange`)
// remains stable. The canonical definitions are in `crate::types::revisions`.
pub use crate::types::revisions::{CellChange, DiffLine};

/// Options controlling how two `ExtractionResult` values are compared.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct DiffOptions {
    /// Include metadata changes in the diff. Default: `true`.
    pub include_metadata: bool,
    /// Include embedded-children changes in the diff. Default: `true`.
    pub include_embedded: bool,
    /// Truncate content to this many characters before diffing.
    ///
    /// Useful for very large documents where only the first N characters matter.
    /// `None` means no truncation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_content_chars: Option<usize>,
}

impl Default for DiffOptions {
    fn default() -> Self {
        Self {
            include_metadata: true,
            include_embedded: true,
            max_content_chars: None,
        }
    }
}

/// The complete diff between two `ExtractionResult` values.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct ExtractionDiff {
    /// Unified-diff hunks for the `content` field.
    ///
    /// Empty when the content is identical.
    pub content_diff: Vec<DiffHunk>,

    /// Tables present in `b` but not in `a` (by index position, excess right-side tables).
    pub tables_added: Vec<Table>,

    /// Tables present in `a` but not in `b` (by index position, excess left-side tables).
    pub tables_removed: Vec<Table>,

    /// Cell-level changes for table pairs that share the same index and dimensions.
    pub tables_changed: Vec<TableDiff>,

    /// Metadata changes in a simplified add/remove/change map.
    ///
    /// Shape: `{ "added": {key: value, ...}, "removed": {key: value, ...},
    ///           "changed": {key: {from: v1, to: v2}, ...} }`.
    ///
    /// Approximates RFC 6902 JSON Patch semantics without pulling in an extra crate.
    pub metadata_changed: serde_json::Value,

    /// Changes to embedded archive children.
    pub embedded_changes: EmbeddedChanges,
}

/// A single contiguous hunk in a unified diff.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct DiffHunk {
    /// Starting line number in the old content (0-indexed).
    pub from_line: usize,
    /// Number of lines from the old content in this hunk.
    pub from_count: usize,
    /// Starting line number in the new content (0-indexed).
    pub to_line: usize,
    /// Number of lines from the new content in this hunk.
    pub to_count: usize,
    /// Lines that make up this hunk.
    pub lines: Vec<DiffLine>,
}

/// Cell-level changes for a pair of tables that share the same index.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct TableDiff {
    /// Zero-based index of the table in both `a.tables` and `b.tables`.
    pub from_index: usize,
    /// Zero-based index in `b.tables` (equal to `from_index` for same-dimension tables).
    pub to_index: usize,
    /// Cell-level changes within the table.
    pub cell_changes: Vec<CellChange>,
}

/// Changes to embedded archive children between two results.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct EmbeddedChanges {
    /// Children present in `b` but not in `a` (matched by `path`).
    pub added: Vec<ArchiveEntry>,
    /// Children present in `a` but not in `b` (matched by `path`).
    pub removed: Vec<ArchiveEntry>,
    /// Children present in both but with differing content (matched by `path`).
    ///
    /// Each entry holds the diff of the nested `ExtractionResult`.
    pub changed: Vec<EmbeddedDiff>,
}

/// Diff for a single embedded archive entry that appears in both results.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct EmbeddedDiff {
    /// Archive-relative path identifying this entry.
    pub path: String,
    /// The recursive diff of the entry's extraction result.
    pub diff: Box<ExtractionDiff>,
}
