//! Diff two [`ExtractedDocument`] values.
//!
//! This module is gated behind the `diff` Cargo feature. Enable it by adding
//! `xberg = { features = ["diff"] }` to your `Cargo.toml`.
//!
//! # Example
//!
//! ```rust,no_run
//! use xberg::{ExtractedDocument, diff::{compare, DiffOptions}};
//!
//! # fn main() {
//! let a = ExtractedDocument::default();
//! let b = ExtractedDocument::default();
//! let opts = DiffOptions::default();
//! let result = compare(&a, &b, &opts);
//! assert!(result.content_diff.is_empty());
//! # }
//! ```

pub mod types;

pub use types::{
    CellChange, DiffHunk, DiffLine, DiffOptions, EmbeddedChanges, EmbeddedDiff, ExtractionDiff, TableDiff,
};

use similar::{ChangeTag, DiffOp, TextDiff};

use crate::types::extraction::{ArchiveEntry, ExtractedDocument};
use crate::types::tables::Table;

/// Default number of context lines on each side of a changed region.
const CONTEXT_LINES: usize = 3;

/// Compare two extraction results and return a structured diff.
///
/// The comparison is purely structural — no I/O, no side effects. All fields
/// of [`ExtractionDiff`] are populated according to the provided [`DiffOptions`].
///
/// # Arguments
///
/// * `a` — the "before" extraction result
/// * `b` — the "after" extraction result
/// * `opts` — controls which sections are compared and optional truncation
///
/// # Example
///
/// ```rust,no_run
/// use xberg::{ExtractedDocument, diff::{compare, DiffOptions}};
///
/// # fn main() {
/// let mut a = ExtractedDocument::default();
/// let mut b = ExtractedDocument::default();
/// a.content = "Hello world".to_string();
/// b.content = "Hello Rust".to_string();
///
/// let diff = compare(&a, &b, &DiffOptions::default());
/// assert_eq!(diff.content_diff.len(), 1);
/// # }
/// ```
pub fn compare(a: &ExtractedDocument, b: &ExtractedDocument, opts: &DiffOptions) -> ExtractionDiff {
    let content_diff = diff_content(&a.content, &b.content, opts);
    let (tables_added, tables_removed, tables_changed) = diff_tables(&a.tables, &b.tables);
    let metadata_changed = if opts.include_metadata {
        diff_metadata(&a.metadata, &b.metadata)
    } else {
        serde_json::Value::Null
    };
    let embedded_changes = if opts.include_embedded {
        diff_embedded(a.children.as_deref(), b.children.as_deref(), opts)
    } else {
        EmbeddedChanges {
            added: vec![],
            removed: vec![],
            changed: vec![],
        }
    };

    ExtractionDiff {
        content_diff,
        tables_added,
        tables_removed,
        tables_changed,
        metadata_changed,
        embedded_changes,
    }
}

fn diff_content(a: &str, b: &str, opts: &DiffOptions) -> Vec<DiffHunk> {
    let a_text = apply_truncation(a, opts.max_content_chars);
    let b_text = apply_truncation(b, opts.max_content_chars);
    let a_ref: &str = a_text.as_deref().unwrap_or(a);
    let b_ref: &str = b_text.as_deref().unwrap_or(b);

    let text_diff = TextDiff::from_lines(a_ref, b_ref);

    if text_diff.ratio() == 1.0 {
        return vec![];
    }

    let mut hunks = Vec::new();
    for group in text_diff.grouped_ops(CONTEXT_LINES) {
        let hunk_from_line = hunk_old_start(&group);
        let hunk_to_line = hunk_new_start(&group);
        let hunk_from_count = hunk_old_len(&group);
        let hunk_to_count = hunk_new_len(&group);
        let mut lines = Vec::new();

        for op in &group {
            for change in text_diff.iter_changes(op) {
                let text = change.value().trim_end_matches('\n').to_string();
                let line = match change.tag() {
                    ChangeTag::Equal => DiffLine::Context(text),
                    ChangeTag::Insert => DiffLine::Added(text),
                    ChangeTag::Delete => DiffLine::Removed(text),
                };
                lines.push(line);
            }
        }

        if !lines.is_empty() {
            hunks.push(DiffHunk {
                from_line: hunk_from_line,
                from_count: hunk_from_count,
                to_line: hunk_to_line,
                to_count: hunk_to_count,
                lines,
            });
        }
    }
    hunks
}

fn hunk_old_start(ops: &[DiffOp]) -> usize {
    ops.first().map_or(0, |op| op.old_range().start)
}

fn hunk_new_start(ops: &[DiffOp]) -> usize {
    ops.first().map_or(0, |op| op.new_range().start)
}

fn hunk_old_len(ops: &[DiffOp]) -> usize {
    let start = ops.first().map_or(0, |op| op.old_range().start);
    let end = ops.last().map_or(0, |op| op.old_range().end);
    end.saturating_sub(start)
}

fn hunk_new_len(ops: &[DiffOp]) -> usize {
    let start = ops.first().map_or(0, |op| op.new_range().start);
    let end = ops.last().map_or(0, |op| op.new_range().end);
    end.saturating_sub(start)
}

fn apply_truncation(text: &str, limit: Option<usize>) -> Option<String> {
    limit.map(|n| {
        let mut boundary = n.min(text.len());
        while !text.is_char_boundary(boundary) {
            boundary -= 1;
        }
        text[..boundary].to_string()
    })
}

fn diff_tables(a_tables: &[Table], b_tables: &[Table]) -> (Vec<Table>, Vec<Table>, Vec<TableDiff>) {
    let min_len = a_tables.len().min(b_tables.len());
    let mut tables_changed = Vec::new();
    let mut tables_removed = Vec::new();
    let mut tables_added = Vec::new();

    for idx in 0..min_len {
        let a_t = &a_tables[idx];
        let b_t = &b_tables[idx];

        if tables_same_shape(a_t, b_t) {
            let cell_changes = diff_cells(a_t, b_t);
            if !cell_changes.is_empty() {
                tables_changed.push(TableDiff {
                    from_index: idx,
                    to_index: idx,
                    cell_changes,
                });
            }
        } else {
            tables_removed.push(a_t.clone());
            tables_added.push(b_t.clone());
        }
    }

    if a_tables.len() > b_tables.len() {
        tables_removed.extend(a_tables[min_len..].iter().cloned());
    } else if b_tables.len() > a_tables.len() {
        tables_added.extend(b_tables[min_len..].iter().cloned());
    }

    (tables_added, tables_removed, tables_changed)
}

/// Two tables are considered the same shape if and only if their row and column counts match.
///
/// Header content is NOT compared — column reordering with the same dimensions will produce
/// per-cell `CellChange` entries for every cell whose value differs, not a structural replacement.
fn tables_same_shape(a: &Table, b: &Table) -> bool {
    if a.cells.len() != b.cells.len() {
        return false;
    }
    let a_cols = a.cells.first().map_or(0, Vec::len);
    let b_cols = b.cells.first().map_or(0, Vec::len);
    a_cols == b_cols
}

fn diff_cells(a: &Table, b: &Table) -> Vec<CellChange> {
    let mut changes = Vec::new();
    for (row_idx, (a_row, b_row)) in a.cells.iter().zip(b.cells.iter()).enumerate() {
        for (col_idx, (a_cell, b_cell)) in a_row.iter().zip(b_row.iter()).enumerate() {
            if a_cell != b_cell {
                changes.push(CellChange {
                    row: row_idx,
                    col: col_idx,
                    from: a_cell.clone(),
                    to: b_cell.clone(),
                });
            }
        }
    }
    changes
}

fn diff_metadata(a: &crate::types::metadata::Metadata, b: &crate::types::metadata::Metadata) -> serde_json::Value {
    let a_val = serde_json::to_value(a).unwrap_or(serde_json::Value::Null);
    let b_val = serde_json::to_value(b).unwrap_or(serde_json::Value::Null);

    let a_obj = a_val.as_object().cloned().unwrap_or_default();
    let b_obj = b_val.as_object().cloned().unwrap_or_default();

    let mut added = serde_json::Map::new();
    let mut removed = serde_json::Map::new();
    let mut changed = serde_json::Map::new();

    for (key, b_value) in &b_obj {
        match a_obj.get(key) {
            None => {
                added.insert(key.clone(), b_value.clone());
            }
            Some(a_value) if a_value != b_value => {
                changed.insert(key.clone(), serde_json::json!({ "from": a_value, "to": b_value }));
            }
            _ => {}
        }
    }
    for (key, a_value) in &a_obj {
        if !b_obj.contains_key(key) {
            removed.insert(key.clone(), a_value.clone());
        }
    }

    serde_json::json!({ "added": added, "removed": removed, "changed": changed })
}

fn diff_embedded(
    a_children: Option<&[ArchiveEntry]>,
    b_children: Option<&[ArchiveEntry]>,
    opts: &DiffOptions,
) -> EmbeddedChanges {
    let a_entries = a_children.unwrap_or(&[]);
    let b_entries = b_children.unwrap_or(&[]);

    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut changed = Vec::new();

    for b_entry in b_entries {
        match a_entries.iter().find(|e| e.path == b_entry.path) {
            None => added.push(b_entry.clone()),
            Some(a_entry) => {
                let child_diff = compare(&a_entry.result, &b_entry.result, opts);
                if is_nonempty_diff(&child_diff) {
                    changed.push(EmbeddedDiff {
                        path: b_entry.path.clone(),
                        diff: Box::new(child_diff),
                    });
                }
            }
        }
    }
    for a_entry in a_entries {
        if !b_entries.iter().any(|e| e.path == a_entry.path) {
            removed.push(a_entry.clone());
        }
    }

    EmbeddedChanges {
        added,
        removed,
        changed,
    }
}

fn is_nonempty_diff(diff: &ExtractionDiff) -> bool {
    !diff.content_diff.is_empty()
        || !diff.tables_added.is_empty()
        || !diff.tables_removed.is_empty()
        || !diff.tables_changed.is_empty()
        || !diff.embedded_changes.added.is_empty()
        || !diff.embedded_changes.removed.is_empty()
        || !diff.embedded_changes.changed.is_empty()
        || is_nonempty_metadata_diff(&diff.metadata_changed)
}

fn is_nonempty_metadata_diff(val: &serde_json::Value) -> bool {
    if val.is_null() {
        return false;
    }
    let empty_obj = serde_json::json!({ "added": {}, "removed": {}, "changed": {} });
    val != &empty_obj
}

#[cfg(all(test, feature = "diff"))]
mod tests {
    use super::*;
    use crate::types::{extraction::ExtractedDocument, tables::Table};

    fn empty_result() -> ExtractedDocument {
        ExtractedDocument::default()
    }

    fn result_with_content(content: &str) -> ExtractedDocument {
        ExtractedDocument {
            content: content.to_string(),
            ..Default::default()
        }
    }

    fn result_with_tables(tables: Vec<Table>) -> ExtractedDocument {
        ExtractedDocument {
            tables,
            ..Default::default()
        }
    }

    fn simple_table(cells: Vec<Vec<&str>>) -> Table {
        Table {
            cells: cells
                .into_iter()
                .map(|row| row.into_iter().map(str::to_string).collect())
                .collect(),
            markdown: String::new(),
            page_number: 1,
            bounding_box: None,
        }
    }

    #[test]
    fn should_produce_empty_diff_for_identical_inputs() {
        let a = empty_result();
        let b = empty_result();
        let diff = compare(&a, &b, &DiffOptions::default());

        assert!(diff.content_diff.is_empty());
        assert!(diff.tables_added.is_empty());
        assert!(diff.tables_removed.is_empty());
        assert!(diff.tables_changed.is_empty());
        assert!(diff.embedded_changes.added.is_empty());
        assert!(diff.embedded_changes.removed.is_empty());
        assert!(diff.embedded_changes.changed.is_empty());
    }

    #[test]
    fn should_produce_empty_diff_for_both_empty_results() {
        let diff = compare(
            &ExtractedDocument::default(),
            &ExtractedDocument::default(),
            &DiffOptions::default(),
        );
        assert!(!is_nonempty_diff(&diff));
    }

    #[test]
    fn should_produce_one_hunk_for_single_line_change() {
        let a = result_with_content("Hello world");
        let b = result_with_content("Hello Rust");
        let diff = compare(&a, &b, &DiffOptions::default());

        assert_eq!(diff.content_diff.len(), 1, "expected exactly one hunk");
        let hunk = &diff.content_diff[0];
        let has_removed = hunk
            .lines
            .iter()
            .any(|l| matches!(l, DiffLine::Removed(t) if t == "Hello world"));
        let has_added = hunk
            .lines
            .iter()
            .any(|l| matches!(l, DiffLine::Added(t) if t == "Hello Rust"));
        assert!(has_removed, "expected 'Hello world' as Removed line");
        assert!(has_added, "expected 'Hello Rust' as Added line");
    }

    #[test]
    fn should_report_correct_line_numbers_for_single_line_change() {
        let a = result_with_content("line one\nline two\nline three");
        let b = result_with_content("line one\nline TWO\nline three");
        let diff = compare(&a, &b, &DiffOptions::default());

        assert_eq!(diff.content_diff.len(), 1);
        let hunk = &diff.content_diff[0];
        assert_eq!(hunk.from_line, 0);
        assert_eq!(hunk.to_line, 0);
        assert_eq!(hunk.from_count, 3);
        assert_eq!(hunk.to_count, 3);
        let has_removed = hunk
            .lines
            .iter()
            .any(|l| matches!(l, DiffLine::Removed(t) if t == "line two"));
        let has_added = hunk
            .lines
            .iter()
            .any(|l| matches!(l, DiffLine::Added(t) if t == "line TWO"));
        assert!(has_removed, "expected 'line two' as Removed line");
        assert!(has_added, "expected 'line TWO' as Added line");
    }

    #[test]
    fn should_produce_empty_content_diff_when_content_identical_but_tables_differ() {
        let mut a = result_with_tables(vec![simple_table(vec![vec!["A", "B"]])]);
        a.content = "same text".to_string();
        let mut b = result_with_tables(vec![simple_table(vec![vec!["A", "C"]])]);
        b.content = "same text".to_string();

        let diff = compare(&a, &b, &DiffOptions::default());
        assert!(
            diff.content_diff.is_empty(),
            "content is identical; no content hunks expected"
        );
        assert!(!diff.tables_changed.is_empty(), "table change expected");
    }

    /// Regression for #1223: a table that changes shape (gains a column) at the
    /// same index must be reported as removed + added, not as an information-free
    /// empty `tables_changed` entry.
    #[test]
    fn shape_change_reports_removed_and_added_not_empty_change() {
        let a = result_with_tables(vec![simple_table(vec![vec!["A", "B"], vec!["1", "2"]])]);
        let b = result_with_tables(vec![simple_table(vec![vec!["A", "B", "C"], vec!["1", "2", "3"]])]);
        let diff = compare(&a, &b, &DiffOptions::default());

        assert!(
            diff.tables_changed.is_empty(),
            "a shape change must not produce an empty 'changed' entry; got: {:?}",
            diff.tables_changed
        );
        assert_eq!(diff.tables_removed.len(), 1, "old-shape table must be reported removed");
        assert_eq!(diff.tables_added.len(), 1, "new-shape table must be reported added");
        assert_eq!(diff.tables_removed[0].cells[0].len(), 2);
        assert_eq!(diff.tables_added[0].cells[0].len(), 3);
    }

    #[test]
    fn should_detect_single_cell_change_in_same_table() {
        let a = result_with_tables(vec![simple_table(vec![vec!["A", "B"], vec!["C", "D"]])]);
        let b = result_with_tables(vec![simple_table(vec![vec!["A", "B"], vec!["C", "X"]])]);
        let diff = compare(&a, &b, &DiffOptions::default());

        assert_eq!(diff.tables_changed.len(), 1);
        let table_diff = &diff.tables_changed[0];
        assert_eq!(table_diff.cell_changes.len(), 1);
        let change = &table_diff.cell_changes[0];
        assert_eq!(change.row, 1);
        assert_eq!(change.col, 1);
        assert_eq!(change.from, "D");
        assert_eq!(change.to, "X");
    }

    #[test]
    fn should_put_extra_table_in_tables_added() {
        let a = result_with_tables(vec![simple_table(vec![vec!["A"]])]);
        let b = result_with_tables(vec![simple_table(vec![vec!["A"]]), simple_table(vec![vec!["NEW"]])]);
        let diff = compare(&a, &b, &DiffOptions::default());

        assert_eq!(diff.tables_added.len(), 1);
        assert_eq!(diff.tables_added[0].cells[0][0], "NEW");
        assert!(diff.tables_removed.is_empty());
    }

    #[test]
    fn should_put_missing_table_in_tables_removed() {
        let a = result_with_tables(vec![simple_table(vec![vec!["A"]]), simple_table(vec![vec!["OLD"]])]);
        let b = result_with_tables(vec![simple_table(vec![vec!["A"]])]);
        let diff = compare(&a, &b, &DiffOptions::default());

        assert_eq!(diff.tables_removed.len(), 1);
        assert_eq!(diff.tables_removed[0].cells[0][0], "OLD");
        assert!(diff.tables_added.is_empty());
    }

    #[test]
    fn should_detect_added_embedded_child() {
        let a = empty_result();
        let mut b = empty_result();
        b.children = Some(vec![ArchiveEntry {
            path: "doc.txt".to_string(),
            mime_type: "text/plain".to_string(),
            result: Box::new(result_with_content("hello")),
        }]);

        let diff = compare(&a, &b, &DiffOptions::default());
        assert_eq!(diff.embedded_changes.added.len(), 1);
        assert_eq!(diff.embedded_changes.added[0].path, "doc.txt");
        assert!(diff.embedded_changes.removed.is_empty());
    }

    #[test]
    fn should_detect_removed_embedded_child() {
        let mut a = empty_result();
        a.children = Some(vec![ArchiveEntry {
            path: "old.txt".to_string(),
            mime_type: "text/plain".to_string(),
            result: Box::new(result_with_content("old")),
        }]);
        let b = empty_result();

        let diff = compare(&a, &b, &DiffOptions::default());
        assert_eq!(diff.embedded_changes.removed.len(), 1);
        assert_eq!(diff.embedded_changes.removed[0].path, "old.txt");
        assert!(diff.embedded_changes.added.is_empty());
    }
}
