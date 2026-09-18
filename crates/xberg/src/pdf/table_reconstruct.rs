//! Table reconstruction from PDF segments (no OCR dependency).
//!
//! This module provides table reconstruction utilities that work with any
//! source of word-level text data (PDF native text, OCR output, etc.).
//! It re-exports core types from `table_core` and adds PDF-specific
//! conversion helpers.

pub(crate) use crate::table_core::{HocrWord, reconstruct_table, table_to_markdown};

const DENSE_NUMERIC_MIN_DATA_ROWS: usize = 6;
const DENSE_NUMERIC_MIN_COLUMNS: usize = 6;
const DENSE_NUMERIC_MIN_CELL_PERCENT: usize = 75;
/// Minimum non-empty data cells for the short-numeric-table exemption. Below
/// this there is too little evidence to call a grid a genuine table.
const SHORT_NUMERIC_MIN_DATA_CELLS: usize = 4;
/// Data-cell numeric fraction at or above which a short, wide single-word grid
/// is a genuine numeric table (invoice line items, small metric tables) rather
/// than shredded multi-column prose. Prose columns are alphabetic, so this bar
/// is unreachable for the misparses the ≥5-column guard targets — it recovers
/// the short borderless tables the corrected preprocessing loses (#1316)
/// without reopening the #36 fabrication hole.
const SHORT_NUMERIC_MIN_CELL_PERCENT: usize = 60;
const SHORT_NUMERIC_MIN_ROW_OCCUPANCY_PERCENT: usize = 85;
/// Short, wide grids have too few rows to establish stable column structure,
/// so require slightly denser evidence than the general table validator.
const SHORT_WIDE_MAX_DATA_ROWS: usize = 2;
const SHORT_WIDE_MIN_COLUMNS: usize = 6;
const SHORT_WIDE_MAX_EMPTY_CELL_PERCENT: usize = 35;
const LARGE_TABLE_MIN_COLUMNS: usize = 6;
const DEFAULT_MIN_DATA_ROW_DIGIT_CELLS: usize = 3;
const REPEATED_DATA_ROW_COUNT: usize = 3;
const ROW_SHAPE_MIN_OVERLAP_PERCENT: usize = 80;
const DENSE_SCALAR_MIN_DATA_ROWS: usize = 20;
const DENSE_SCALAR_MIN_COLUMNS: usize = 6;
const DENSE_SCALAR_MIN_FILLED_PERCENT: usize = 75;
const DENSE_SCALAR_MIN_COMPACT_PERCENT: usize = 90;
const DENSE_SCALAR_MIN_DIGIT_PERCENT: usize = 25;
const DENSE_SCALAR_MAX_CELL_CHARS: usize = 24;
const SPURIOUS_COLUMN_MIN_DATA_ROWS: usize = 20;
const SPURIOUS_COLUMN_MIN_COLUMNS: usize = 6;
const SPURIOUS_COLUMN_MIN_RETAINED_DENSITY_PERCENT: usize = 75;
const FOOTER_MIN_ALPHA_PERCENT: usize = 70;
/// Minimum percentage of a data column's non-ambiguous cells that must parse as a bare
/// numeric literal (after dash-glyph normalisation) for the column to receive
/// `normalize_data_cell`'s dash/exponent rewriting (xberg-io/xberg#1582).
const NUMERIC_COLUMN_MIN_NUMERIC_PERCENT: usize = 60;

#[cfg(feature = "pdf")]
use super::hierarchy::SegmentData;
#[cfg(feature = "pdf")]
use super::structure::lines::segments_are_touching;

/// Convert a PDF `SegmentData` to an `HocrWord` for table reconstruction, adding
/// `advance_offset`/`top_offset` to the segment's upright-frame position before
/// clamping to the unsigned `HocrWord` fields.
///
/// `SegmentData` uses PDF coordinates (y=0 at bottom, increases upward).
/// `HocrWord` uses image coordinates (y=0 at top, increases downward).
///
/// For a segment drawn on a rotated text matrix (GH#1358: a sideways table),
/// `seg.x`/`seg.y` are the run's *page-space* origin, but the run's own row
/// axis and column axis are rotated relative to the page — using raw
/// page-space x/y here would group a rotated table's cells into rows and
/// columns along the wrong axes. [`SegmentData::upright_origin`] rotates the
/// origin back into the segment's own reading frame (identity for the
/// unrotated case, matching the plain x/y this replaced) so the row/column
/// clustering downstream in `native::table::cluster_words_into_vertical_regions`
/// operates on the table's actual advance/cross axes instead of the page's.
///
/// Standalone calls (no sibling segments) always pass `(0.0, 0.0)` offsets: a
/// single segment can never be lifted relative to a run it isn't part of.
/// Only [`segments_to_words`] computes a real offset, from the whole page.
///
/// GH#1358: a rotated run's advance/cross can be negative for every word on
/// the page (e.g. `-90` gives `advance == -y`; `180` gives `advance == -x` and
/// `cross == -y`), and a per-word `.max(0.0)` clamp saturates every one of
/// them onto the same `0`, collapsing a table's columns and/or rows and
/// making it single-column/single-row — which downstream clustering then
/// rejects outright. `rotation_lifts_for_page` computes, once per rotation
/// group sharing a page, the amount needed to lift that group's *minimum*
/// advance/top to zero; adding it here before the per-word clamp preserves
/// the run's relative geometry instead of destroying it. Unrotated content
/// and any rotation whose minimum is already non-negative gets `(0.0, 0.0)`
/// here, leaving this identical to the unlifted computation.
#[cfg(feature = "pdf")]
fn segment_to_hocr_word_lifted(seg: &SegmentData, page_height: f32, advance_offset: f32, top_offset: f32) -> HocrWord {
    let (advance, cross) = seg.upright_origin();
    let top_image = (page_height - (cross + seg.height) + top_offset).round().max(0.0) as u32;
    HocrWord {
        text: seg.text.clone(),
        left: (advance + advance_offset).round().max(0.0) as u32,
        top: top_image,
        width: seg.width.round().max(0.0) as u32,
        height: seg.height.round().max(0.0) as u32,
        confidence: 95.0,
    }
}

/// Split a `SegmentData` into word-level `HocrWord`s for table reconstruction.
///
/// Pdfium segments can contain multiple whitespace-separated words (merged by
/// shared baseline + font). For table cell matching, each word needs its own
/// bounding box so it can be assigned to the correct column/cell.
///
/// Single-word segments use `segment_to_hocr_word_lifted` directly (fast path).
/// Multi-word segments get proportional bbox estimation per word based on
/// byte offset within the segment text.
///
/// See [`segment_to_hocr_word_lifted`] for why a standalone call always lifts by
/// `(0.0, 0.0)`.
#[cfg(feature = "pdf")]
pub(crate) fn split_segment_to_words(seg: &SegmentData, page_height: f32) -> Vec<HocrWord> {
    split_segment_to_words_lifted(seg, page_height, 0.0, 0.0)
}

/// Like [`split_segment_to_words`], with the same `advance_offset`/
/// `top_offset` lift documented on [`segment_to_hocr_word_lifted`].
#[cfg(feature = "pdf")]
fn split_segment_to_words_lifted(
    seg: &SegmentData,
    page_height: f32,
    advance_offset: f32,
    top_offset: f32,
) -> Vec<HocrWord> {
    let trimmed = seg.text.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    if !trimmed.contains(char::is_whitespace) {
        return vec![segment_to_hocr_word_lifted(
            seg,
            page_height,
            advance_offset,
            top_offset,
        )];
    }

    let text = &seg.text;
    let total_bytes = text.len() as f32;
    if total_bytes <= 0.0 {
        return Vec::new();
    }

    // See `segment_to_hocr_word_lifted` for why the segment's own upright frame
    // (rather than raw page-space x/y) is used here: `advance` is the
    // position along the run's reading axis, which per-word interpolation
    // below advances along via `frac_start * seg.width` — consistent
    // whether or not the run is rotated.
    let (advance, cross) = seg.upright_origin();
    let top_image = (page_height - (cross + seg.height) + top_offset).round().max(0.0) as u32;
    let seg_height = seg.height.round().max(0.0) as u32;

    let mut words = Vec::new();
    let mut search_start = 0;
    for word in text.split_whitespace() {
        let byte_offset = text[search_start..].find(word).map(|pos| search_start + pos);
        let Some(offset) = byte_offset else {
            continue;
        };
        search_start = offset + word.len();

        let frac_start = offset as f32 / total_bytes;
        let frac_width = word.len() as f32 / total_bytes;

        words.push(HocrWord {
            text: word.to_string(),
            left: (advance + advance_offset + frac_start * seg.width).round().max(0.0) as u32,
            top: top_image,
            width: (frac_width * seg.width).round().max(1.0) as u32,
            height: seg_height,
            confidence: 95.0,
        });
    }

    words
}

/// Per-rotation lift computed once for a page's segments (GH#1358).
///
/// Holds, for one `rotation_degrees` value, how far that group's advance and
/// top must be translated so its minimum lands at `0.0` — the "lift the whole
/// frame" fix: computing the minimum over the *run* first (rather than
/// clamping each word independently) preserves relative spacing between
/// words instead of collapsing every negative value onto the same floor.
#[cfg(feature = "pdf")]
struct RotationLift {
    rotation_degrees: f32,
    advance_offset: f32,
    top_offset: f32,
}

/// Computes [`RotationLift`]s for every distinct rotation present in
/// `segments`, restricted to rotated content.
///
/// Unrotated segments are deliberately excluded: their advance/cross already
/// equal ordinary page-space x/y, so lifting them would translate normal,
/// already-correct page content by a constant offset instead of fixing
/// anything. A rotation's lift is `0.0` on an axis whose minimum is already
/// non-negative, so a page whose rotated content never goes negative (the
/// `+90` case is non-negative on both axes for real page content) gets
/// `(0.0, 0.0)` here and this function is a no-op for it — unchanged
/// behavior from before this fix.
#[cfg(feature = "pdf")]
fn rotation_lifts_for_page(segments: &[SegmentData], page_height: f32) -> Vec<RotationLift> {
    let mut lifts: Vec<RotationLift> = Vec::new();
    for seg in segments {
        if seg.is_unrotated() {
            continue;
        }
        let (advance, cross) = seg.upright_origin();
        let top = page_height - (cross + seg.height);
        match lifts
            .iter_mut()
            .find(|lift| (lift.rotation_degrees - seg.rotation_degrees).abs() <= f32::EPSILON)
        {
            Some(lift) => {
                lift.advance_offset = lift.advance_offset.min(advance);
                lift.top_offset = lift.top_offset.min(top);
            }
            None => lifts.push(RotationLift {
                rotation_degrees: seg.rotation_degrees,
                advance_offset: advance,
                top_offset: top,
            }),
        }
    }
    for lift in &mut lifts {
        lift.advance_offset = (-lift.advance_offset).max(0.0);
        lift.top_offset = (-lift.top_offset).max(0.0);
    }
    lifts
}

#[cfg(feature = "pdf")]
fn lift_for_rotation(lifts: &[RotationLift], rotation_degrees: f32) -> (f32, f32) {
    lifts
        .iter()
        .find(|lift| (lift.rotation_degrees - rotation_degrees).abs() <= f32::EPSILON)
        .map(|lift| (lift.advance_offset, lift.top_offset))
        .unwrap_or((0.0, 0.0))
}

/// Whether any rotated group on this page needed [`rotation_lifts_for_page`]
/// to translate it into non-negative image space.
///
/// `HocrWord` carries no rotation field, so once words are built there is no
/// way to tell, from a table region alone, whether its geometry came from a
/// translated rotated frame. Callers that compute a page-space bounding box
/// from `HocrWord`s (`native::table::region_bounding_box`) must ask this
/// *before* that conversion and, if true, not trust that bounding box as
/// page-space: `region_bounding_box`'s inversion only cancels out to real
/// page coordinates for the unrotated identity mapping, so a lifted frame's
/// `left`/`top` fed through it back-projects to a rectangle in no page
/// coordinate system at all — which would then be compared against real
/// `SegmentData::x`/`y` by `filter_segments_by_table_bboxes` and could delete
/// unrelated prose it spuriously overlaps. See GH#1358.
#[cfg(feature = "pdf")]
pub(crate) fn page_has_lifted_rotation_frame(segments: &[SegmentData], page_height: f32) -> bool {
    rotation_lifts_for_page(segments, page_height)
        .iter()
        .any(|lift| lift.advance_offset > 0.0 || lift.top_offset > 0.0)
}

/// Convert a page's segments to word-level `HocrWord`s for table extraction.
///
/// Splits multi-word segments into individual words with proportional bounding
/// boxes, ensuring each word can be independently matched to table cells.
///
/// GH#1358: before per-segment conversion, computes each rotation's page-wide
/// lift (see [`rotation_lifts_for_page`]) so a rotated table's columns/rows
/// survive into `HocrWord`'s unsigned fields instead of collapsing onto a
/// shared clamp floor.
#[cfg(feature = "pdf")]
pub(crate) fn segments_to_words(segments: &[SegmentData], page_height: f32) -> Vec<HocrWord> {
    let lifts = rotation_lifts_for_page(segments, page_height);
    let per_segment_words: Vec<Vec<HocrWord>> = segments
        .iter()
        .map(|seg| {
            let (advance_offset, top_offset) = lift_for_rotation(&lifts, seg.rotation_degrees);
            split_segment_to_words_lifted(seg, page_height, advance_offset, top_offset)
        })
        .collect();
    merge_touching_segment_boundaries(segments, per_segment_words)
}

/// Merges a touching word split across two adjacent segments (xberg-io/xberg#1566) into a
/// single `HocrWord` before table-cell assignment, so `assign_words_to_cells`'s
/// `cell_words.join(" ")` never re-inserts the space that `split_segment_to_words_lifted`
/// dropped by construction. `HocrWord` is `u32`-rounded and carries no font size or baseline,
/// so a sub-point gap like the reported case (0.069 pt) is not representable once words exist
/// — the check must run here, on `SegmentData`, one segment boundary at a time.
///
/// Only the last word of one segment's group and the first word of the next segment's group
/// can ever be a split-word boundary, since a single segment's own words were already produced
/// by whitespace-splitting its own text.
#[cfg(feature = "pdf")]
fn merge_touching_segment_boundaries(
    segments: &[SegmentData],
    mut per_segment_words: Vec<Vec<HocrWord>>,
) -> Vec<HocrWord> {
    for boundary in 0..per_segment_words.len().saturating_sub(1) {
        let is_touching = match (
            per_segment_words[boundary].last(),
            per_segment_words[boundary + 1].first(),
        ) {
            (Some(prev_word), Some(next_word)) => segments_are_touching(
                &segments[boundary],
                &prev_word.text,
                &segments[boundary + 1],
                &next_word.text,
            ),
            _ => false,
        };
        if !is_touching {
            continue;
        }
        let prev_word = per_segment_words[boundary].pop().expect("checked Some above");
        let next_word = per_segment_words[boundary + 1].remove(0);
        per_segment_words[boundary + 1].insert(0, merge_hocr_words(prev_word, next_word));
    }
    per_segment_words.into_iter().flatten().collect()
}

/// Combines two `HocrWord`s that are one split word into a single word: text concatenated
/// with no separator, bounding box the union of both, confidence the lower of the two.
#[cfg(feature = "pdf")]
fn merge_hocr_words(prev: HocrWord, next: HocrWord) -> HocrWord {
    let left = prev.left.min(next.left);
    let top = prev.top.min(next.top);
    let right = (prev.left + prev.width).max(next.left + next.width);
    let bottom = (prev.top + prev.height).max(next.top + next.height);

    let mut text = prev.text;
    text.push_str(&next.text);

    HocrWord {
        text,
        left,
        top,
        width: right - left,
        height: bottom - top,
        confidence: prev.confidence.min(next.confidence),
    }
}

/// Column-wise merge of several table rows into a single logical row.
///
/// Each output column's text is the space-joined concatenation of that
/// column's non-empty cells across `rows`, in row order, truncated to
/// `column_count` columns. Used to collapse a fragment's word-wrapped header
/// sub-lines into one header row here, and reused by
/// [`super::structure::pipeline`]'s table-continuation stitching to collapse
/// a whole table fragment (whose rows are word-wrapped sub-lines of a single
/// logical row, once `native::table`'s row-gap clustering has split one
/// physical table into several fragments) into one row when the fragments are
/// stitched back together.
pub(crate) fn merge_rows_columnwise(rows: &[Vec<String>], column_count: usize) -> Vec<String> {
    let mut merged = vec![String::new(); column_count];
    for row in rows {
        for (idx, cell) in row.iter().enumerate().take(column_count) {
            let trimmed = cell.trim();
            if trimmed.is_empty() {
                continue;
            }
            if !merged[idx].is_empty() {
                merged[idx].push(' ');
            }
            merged[idx].push_str(trimmed);
        }
    }
    merged
}

/// Post-process a raw table grid to validate structure and clean up.
///
/// Returns `None` if the table fails structural validation.
///
/// When `layout_guided` is true, the layout model already confirmed this is
/// a table, so validation thresholds are relaxed:
/// - Minimum columns: 3 → 2
/// - Column sparsity: 75% → 95%
/// - Overall density: 40% → 15%
/// - Prose detection: reject if >70% cells >100 chars (vs >50% >60 chars)
/// - Prose detection: reject if avg cell >80 chars (vs >50 chars)
/// - Single-word cell: reject if >85% single-word (vs >70%)
/// - Content asymmetry: reject if one col >92% of text (vs >85%)
/// - Column-text-flow: applied equally (reject if >60% rows flow through)
pub(crate) fn post_process_table(
    table: Vec<Vec<String>>,
    layout_guided: bool,
    allow_single_column: bool,
) -> Option<Vec<Vec<String>>> {
    let min_columns = min_columns_for(layout_guided, allow_single_column);
    post_process_table_inner(table, min_columns, layout_guided, None)
}

/// Like [`post_process_table`], but also keeps `column_positions` — the column
/// x-positions [`crate::table_core::reconstruct_table_with_columns`] returned alongside
/// the grid — in lockstep with any column this cleanup removes.
///
/// `merge_header_only_column` and `prune_spurious_interior_column` both drop a column
/// from the table (a header-only track with no supporting data, or a single stray
/// footer word). Without this, a caller that hands `column_positions` to a
/// column-index-keyed consumer *after* post-processing — e.g.
/// `is_well_formed_borderless_table`'s straddled-boundary check — keeps testing a
/// boundary for a column that no longer exists in the cleaned grid. That boundary
/// reads as clean whitespace (nothing was ever assigned to the column that got
/// dropped), which dilutes the straddle ratio and can let a borderless candidate pass
/// a gate it should have failed (xberg-io/xberg#863).
pub(crate) fn post_process_table_with_columns(
    table: Vec<Vec<String>>,
    layout_guided: bool,
    allow_single_column: bool,
    column_positions: &mut Vec<u32>,
) -> Option<Vec<Vec<String>>> {
    let min_columns = min_columns_for(layout_guided, allow_single_column);
    post_process_table_inner(table, min_columns, layout_guided, Some(column_positions))
}

fn min_columns_for(layout_guided: bool, allow_single_column: bool) -> usize {
    if allow_single_column {
        1
    } else if layout_guided {
        2
    } else {
        3
    }
}

/// Whether `text` is a bare ordered/bulleted list marker: a run of digits or a single
/// lowercase letter followed by `.` or `)` (`"1."`, `"12)"`, `"a."`, `"b)"`), or a lone
/// bullet glyph (`•`, `-`, `–`, `*`). Hand-rolled rather than pulling in `regex` for three
/// fixed shapes this small (xberg-io/xberg#1570).
fn is_list_marker_cell(text: &str) -> bool {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) if first.is_ascii_digit() => {
            let mut rest = chars.as_str();
            while let Some(next_char) = rest.chars().next() {
                if !next_char.is_ascii_digit() {
                    break;
                }
                rest = &rest[next_char.len_utf8()..];
            }
            rest == "." || rest == ")"
        }
        Some(first) if first.is_ascii_lowercase() => {
            let rest = chars.as_str();
            rest == "." || rest == ")"
        }
        Some('•' | '-' | '–' | '*') => chars.as_str().is_empty(),
        _ => false,
    }
}

/// Whether every whitespace-separated token in `text` is a list marker. Column 0 of a
/// reconstructed list region is not always one marker per cell: `merge_rows_columnwise`
/// collapses a multi-row header into a single cell, so the header of a four-item list can
/// read `"1. 2."`. Testing token-wise sees that as marker content while still rejecting a
/// genuine header label like `"Line"` or `"Item #"` (xberg-io/xberg#1570). ~keep
fn is_list_marker_content(text: &str) -> bool {
    let mut tokens = text.split_whitespace().peekable();
    tokens.peek().is_some() && tokens.all(is_list_marker_cell)
}

fn post_process_table_inner(
    mut table: Vec<Vec<String>>,
    min_columns: usize,
    layout_guided: bool,
    mut column_positions: Option<&mut Vec<u32>>,
) -> Option<Vec<Vec<String>>> {
    table.retain(|row| row.iter().any(|cell| !cell.trim().is_empty()));
    if table.is_empty() {
        tracing::debug!(
            target: "xberg::table_reconstruct",
            reason = "empty_after_retain",
            rows = 0,
            cols = 0,
            "post_process_table_inner: rejected table"
        );
        return None;
    }

    let rejection_rows = table.len();
    let rejection_cols = table.first().map_or(0, Vec::len);

    let mut non_empty = 0usize;
    let mut long_cells = 0usize;
    let mut total_chars = 0usize;
    for row in &table {
        for cell in row {
            let trimmed = cell.trim();
            if trimmed.is_empty() {
                continue;
            }
            let char_count = trimmed.chars().count();
            non_empty += 1;
            total_chars += char_count;
            if char_count > 60 {
                long_cells += 1;
            }
        }
    }

    if non_empty > 0 {
        if layout_guided {
            if long_cells > 0 {
                let long_cells_100 = table
                    .iter()
                    .flat_map(|row| row.iter())
                    .filter(|cell| {
                        let trimmed = cell.trim();
                        !trimmed.is_empty() && trimmed.chars().count() > 100
                    })
                    .count();
                if long_cells_100 * 10 > non_empty * 7 {
                    tracing::debug!(
                        target: "xberg::table_reconstruct",
                        reason = "prose_long_cells_100_ratio",
                        long_cells_100,
                        non_empty,
                        limit_numerator = 7,
                        limit_denominator = 10,
                        rows = rejection_rows,
                        cols = rejection_cols,
                        "post_process_table_inner: rejected table"
                    );
                    return None;
                }
            }
            if total_chars / non_empty > 80 {
                tracing::debug!(
                    target: "xberg::table_reconstruct",
                    reason = "prose_avg_chars_layout_guided",
                    avg_chars = total_chars / non_empty,
                    limit = 80,
                    rows = rejection_rows,
                    cols = rejection_cols,
                    "post_process_table_inner: rejected table"
                );
                return None;
            }
        } else {
            if long_cells * 2 > non_empty {
                tracing::debug!(
                    target: "xberg::table_reconstruct",
                    reason = "prose_long_cells_ratio",
                    long_cells,
                    non_empty,
                    rows = rejection_rows,
                    cols = rejection_cols,
                    "post_process_table_inner: rejected table"
                );
                return None;
            }
            if total_chars / non_empty > 50 {
                tracing::debug!(
                    target: "xberg::table_reconstruct",
                    reason = "prose_avg_chars",
                    avg_chars = total_chars / non_empty,
                    limit = 50,
                    rows = rejection_rows,
                    cols = rejection_cols,
                    "post_process_table_inner: rejected table"
                );
                return None;
            }
        }
    }

    let col_count = table.first().map_or(0, Vec::len);
    if col_count < min_columns {
        tracing::debug!(
            target: "xberg::table_reconstruct",
            reason = "min_columns",
            col_count,
            min_columns,
            rows = rejection_rows,
            cols = col_count,
            "post_process_table_inner: rejected table"
        );
        return None;
    }

    let data_start = find_data_start(&table, layout_guided);

    let mut header_rows = if data_start > 0 {
        table[..data_start].to_vec()
    } else {
        Vec::new()
    };
    let mut data_rows = table[data_start..].to_vec();

    if header_rows.len() > 2 {
        // Keep the established two-row header cap, but do not discard an
        // unusually long prefix inferred by `find_data_start`. Earlier rows
        // are still table content; demote them to data in their original
        // order while retaining the two rows closest to the detected data
        // boundary as the header. ~keep
        let surplus_header_rows = header_rows.len() - 2;
        let mut demoted_rows: Vec<Vec<String>> = header_rows.drain(..surplus_header_rows).collect();
        demoted_rows.append(&mut data_rows);
        data_rows = demoted_rows;
    }

    if header_rows.is_empty() {
        if data_rows.len() < 2 {
            tracing::debug!(
                target: "xberg::table_reconstruct",
                reason = "insufficient_data_rows_no_header",
                data_row_count = data_rows.len(),
                min_required = 2,
                rows = data_rows.len(),
                cols = col_count,
                "post_process_table_inner: rejected table"
            );
            return None;
        }
        header_rows.push(data_rows[0].clone());
        data_rows = data_rows[1..].to_vec();
    }

    let column_count = header_rows.first().or_else(|| data_rows.first()).map_or(0, Vec::len);

    if column_count == 0 {
        tracing::debug!(
            target: "xberg::table_reconstruct",
            reason = "zero_column_count",
            rows = header_rows.len() + data_rows.len(),
            cols = column_count,
            "post_process_table_inner: rejected table"
        );
        return None;
    }

    let header = merge_rows_columnwise(&header_rows, column_count);

    let mut processed = Vec::new();
    processed.push(header);
    processed.extend(data_rows);

    if processed.len() <= 1 {
        tracing::debug!(
            target: "xberg::table_reconstruct",
            reason = "processed_too_short",
            rows = processed.len(),
            cols = processed.first().map_or(0, Vec::len),
            min_required_rows = 2,
            "post_process_table_inner: rejected table"
        );
        return None;
    }

    let mut col = 0;
    while col < processed[0].len() {
        let header_text = processed[0][col].trim().to_string();
        let data_empty = processed[1..]
            .iter()
            .all(|row| row.get(col).is_none_or(|cell| cell.trim().is_empty()));

        if data_empty {
            merge_header_only_column(&mut processed, col, header_text, column_positions.as_deref_mut());
        } else {
            col += 1;
        }

        if processed.is_empty() || processed[0].is_empty() {
            tracing::debug!(
                target: "xberg::table_reconstruct",
                reason = "processed_emptied_during_merge",
                rows = processed.len(),
                cols = processed.first().map_or(0, Vec::len),
                "post_process_table_inner: rejected table"
            );
            return None;
        }
    }

    if processed[0].len() < 2 || processed.len() <= 1 {
        tracing::debug!(
            target: "xberg::table_reconstruct",
            reason = "insufficient_columns_or_rows_after_merge",
            rows = processed.len(),
            cols = processed[0].len(),
            "post_process_table_inner: rejected table"
        );
        return None;
    }

    prune_spurious_interior_column(&mut processed, layout_guided, column_positions);

    let data_row_count = processed.len() - 1;
    if data_row_count > 0 {
        for c in 0..processed[0].len() {
            let empty_count = processed[1..]
                .iter()
                .filter(|row| row.get(c).is_none_or(|cell| cell.trim().is_empty()))
                .count();
            let too_sparse = if layout_guided {
                empty_count * 20 > data_row_count * 19
            } else {
                empty_count * 4 > data_row_count * 3
            };
            // A column with its own non-empty header label (e.g. a bank statement's "DEPOSIT",
            // populated on only a minority of transaction rows) is a legitimate, intentionally
            // sparse column, not the noise this density gate targets -- mirrors
            // `prune_spurious_interior_column`'s established rule elsewhere in this file
            // (`header[column].trim().is_empty()` gates eligibility there too), rather than
            // introducing a new signal (xberg-io/xberg#1649). ~keep
            let column_has_own_header = processed[0].get(c).is_some_and(|cell| !cell.trim().is_empty());
            if too_sparse && !column_has_own_header {
                tracing::debug!(
                    target: "xberg::table_reconstruct",
                    reason = "column_sparsity",
                    col = c,
                    empty_count,
                    data_row_count,
                    empty_ratio = empty_count as f64 / data_row_count as f64,
                    limit = if layout_guided { 19.0 / 20.0 } else { 3.0 / 4.0 },
                    rows = processed.len(),
                    cols = processed[0].len(),
                    "post_process_table_inner: rejected table"
                );
                return None;
            }
        }
    }

    {
        let total_data_cells = data_row_count * processed[0].len();
        if total_data_cells > 0 {
            let filled = processed[1..]
                .iter()
                .flat_map(|row| row.iter())
                .filter(|cell| !cell.trim().is_empty())
                .count();
            let too_sparse = if layout_guided {
                filled * 20 < total_data_cells * 3
            } else {
                filled * 5 < total_data_cells * 2
            };
            if too_sparse {
                tracing::debug!(
                    target: "xberg::table_reconstruct",
                    reason = "overall_density",
                    filled,
                    total_data_cells,
                    density = filled as f64 / total_data_cells as f64,
                    limit = if layout_guided { 3.0 / 20.0 } else { 2.0 / 5.0 },
                    rows = processed.len(),
                    cols = processed[0].len(),
                    "post_process_table_inner: rejected table"
                );
                return None;
            }
        }
    }

    // A candidate whose column 0 is bare list markers ("1.", "a)", "•") end to end — the
    // header row included — is an ordered/bulleted list, not a table: the marker is
    // line-numbering supplied by the source layout, not a discrete data value
    // (xberg-io/xberg#1570). Including row 0 is what separates the two lookalikes. A
    // genuine numbered parts or invoice table carries a header label above its numbers
    // ("Line", "#", "Item"), so its column 0 is not markers end to end and this guard
    // leaves it alone; a fabricated list region has a list item in row 0 like every other
    // row. Scoped to `!layout_guided`: a layout-guided region already has ML confirmation
    // it is a real table, and both routes that produced the fabricated-table bug
    // (Tesseract TSV clustering and PaddleOCR word clustering) call this validator with
    // `layout_guided=false`. ~keep
    if !layout_guided {
        let all_col0: Vec<&str> = processed
            .iter()
            .filter_map(|row| row.first().map(|cell| cell.trim()))
            .filter(|cell| !cell.is_empty())
            .collect();
        if !all_col0.is_empty() && all_col0.iter().all(|cell| is_list_marker_content(cell)) {
            tracing::debug!(
                target: "xberg::table_reconstruct",
                reason = "list_marker_first_column",
                marker_rows = all_col0.len(),
                rows = processed.len(),
                cols = processed[0].len(),
                "post_process_table_inner: rejected table"
            );
            return None;
        }
    }

    let dense_numeric_grid = is_dense_numeric_grid(&processed);

    if processed[0].len() >= 5 {
        let mut single_word_cells = 0usize;
        let mut non_empty_cells = 0usize;
        for row in processed.iter().skip(1) {
            for cell in row {
                let trimmed = cell.trim();
                if trimmed.is_empty() {
                    continue;
                }
                non_empty_cells += 1;
                let word_count = trimmed.split_whitespace().count();
                if word_count <= 2 {
                    single_word_cells += 1;
                }
            }
        }
        let threshold = if layout_guided { 85 } else { 70 };
        let dense_scalar_grid = layout_guided && is_dense_scalar_grid(&processed);
        if !dense_numeric_grid
            && !dense_scalar_grid
            && !is_predominantly_numeric_short_grid(&processed)
            && non_empty_cells >= 6
            && single_word_cells * 100 > non_empty_cells * threshold
        {
            tracing::debug!(
                target: "xberg::table_reconstruct",
                reason = "single_word_cell_ratio",
                single_word_cells,
                non_empty_cells,
                ratio = single_word_cells as f64 / non_empty_cells as f64,
                threshold,
                rows = processed.len(),
                cols = processed[0].len(),
                "post_process_table_inner: rejected table"
            );
            return None;
        }
    }

    if processed[0].len() >= 2 {
        // The marker relaxation below applies only when column 0 has no header label of its
        // own. A header cell like "Line" or "#" means the punctuated numbers beneath it are
        // row-number *data* in a real table, not list markers (#1570). ~keep
        let header_col0_is_marker = processed[0]
            .first()
            .map(|cell| cell.trim())
            .is_some_and(is_list_marker_content);
        let mut flow_rows = 0usize;
        let mut eligible_rows = 0usize;
        for row in processed.iter().skip(1) {
            let col0 = row.first().map(|s| s.trim()).unwrap_or("");
            let col1 = row.get(1).map(|s| s.trim()).unwrap_or("");
            if col0.is_empty() || col1.is_empty() {
                continue;
            }
            eligible_rows += 1;
            let col0_is_list_marker = is_list_marker_content(col0);
            // A list marker's own trailing `.`/`)` is punctuation supplied by the marker
            // convention, not a sentence-final period — it must not exempt the row from
            // the flow signal below the way real prose punctuation does (#1570).
            let ends_without_punct = col0_is_list_marker
                || (!col0.ends_with('.') && !col0.ends_with('?') && !col0.ends_with('!') && !col0.ends_with(':'));
            let starts_lowercase = col1.chars().next().is_some_and(|c| c.is_lowercase());
            // A real list item's second field is typically a new capitalized clause, not a
            // lowercase sentence continuation, so `starts_lowercase` is the wrong signal
            // once col0 is known to be a marker — any non-empty col1 already means this
            // marker is not standing alone as a discrete column value. Gated on
            // `header_col0_is_marker` so a headed row-number column keeps the strict
            // signal, and on `!layout_guided` because a layout-guided region is
            // ML-confirmed as a real table (#1570). ~keep
            let flows = if col0_is_list_marker && header_col0_is_marker && !layout_guided {
                ends_without_punct
            } else {
                ends_without_punct && starts_lowercase
            };
            if flows {
                flow_rows += 1;
            }
        }
        if eligible_rows >= 3 && flow_rows * 10 > eligible_rows * 6 {
            tracing::debug!(
                target: "xberg::table_reconstruct",
                reason = "column_text_flow",
                flow_rows,
                eligible_rows,
                ratio = flow_rows as f64 / eligible_rows as f64,
                limit = 0.6,
                rows = processed.len(),
                cols = processed[0].len(),
                "post_process_table_inner: rejected table"
            );
            return None;
        }
    }

    {
        let num_cols = processed[0].len();
        let col_char_counts: Vec<usize> = (0..num_cols)
            .map(|c| {
                processed[1..]
                    .iter()
                    .map(|row| row.get(c).map_or(0, |cell| cell.trim().len()))
                    .sum()
            })
            .collect();
        let total_chars_asym: usize = col_char_counts.iter().sum();

        if total_chars_asym > 0 {
            let max_col_share = col_char_counts
                .iter()
                .map(|&cc| cc as f64 / total_chars_asym as f64)
                .fold(0.0_f64, f64::max);
            let dominant_threshold = if layout_guided { 0.92 } else { 0.85 };
            if max_col_share > dominant_threshold {
                tracing::debug!(
                    target: "xberg::table_reconstruct",
                    reason = "content_asymmetry_dominant_column",
                    max_col_share,
                    dominant_threshold,
                    rows = processed.len(),
                    cols = processed[0].len(),
                    "post_process_table_inner: rejected table"
                );
                return None;
            }

            if !layout_guided {
                for (c, &col_chars) in col_char_counts.iter().enumerate() {
                    let char_share = col_chars as f64 / total_chars_asym as f64;
                    let empty_in_col = processed[1..]
                        .iter()
                        .filter(|row| row.get(c).is_none_or(|cell| cell.trim().is_empty()))
                        .count();
                    let empty_ratio = empty_in_col as f64 / data_row_count as f64;

                    // A column with its own non-empty header label (e.g. a bank statement's
                    // "DEPOSIT", populated on only a minority of transaction rows) is a
                    // legitimate, intentionally sparse column, not a stray annotation/footnote
                    // fragment split from prose -- mirrors `prune_spurious_interior_column`'s
                    // established rule elsewhere in this file (`header[column].trim().is_empty()`
                    // gates eligibility there too), rather than introducing a new signal
                    // (xberg-io/xberg#1649). ~keep
                    let column_has_own_header = processed
                        .first()
                        .and_then(|header| header.get(c))
                        .is_some_and(|cell| !cell.trim().is_empty());
                    if char_share < 0.15 && empty_ratio > 0.5 && !column_has_own_header {
                        tracing::debug!(
                            target: "xberg::table_reconstruct",
                            reason = "content_asymmetry_sparse_column",
                            col = c,
                            char_share,
                            empty_ratio,
                            char_share_limit = 0.15,
                            empty_ratio_limit = 0.5,
                            rows = processed.len(),
                            cols = processed[0].len(),
                            "post_process_table_inner: rejected table"
                        );
                        return None;
                    }
                }
            }
        }
    }

    if processed.len() > 3 && processed[0].len() >= 2 {
        let last_col = processed[0].len() - 1;
        let mut continuation_count = 0usize;
        let mut eligible_transitions = 0usize;
        for pair in processed[1..].windows(2) {
            let prev_last = pair[0].get(last_col).map(|s| s.trim()).unwrap_or("");
            let next_first = pair[1].first().map(|s| s.trim()).unwrap_or("");
            if prev_last.is_empty() || next_first.is_empty() {
                continue;
            }
            eligible_transitions += 1;
            let ends_without_punct = !prev_last.ends_with('.')
                && !prev_last.ends_with('?')
                && !prev_last.ends_with('!')
                && !prev_last.ends_with(':')
                && !prev_last.ends_with(';');
            let starts_lowercase = next_first.chars().next().is_some_and(|c| c.is_lowercase());
            if ends_without_punct && starts_lowercase {
                continuation_count += 1;
            }
        }
        if eligible_transitions >= 3 && continuation_count * 10 > eligible_transitions * 4 {
            tracing::debug!(
                target: "xberg::table_reconstruct",
                reason = "row_continuation_flow",
                continuation_count,
                eligible_transitions,
                ratio = continuation_count as f64 / eligible_transitions as f64,
                limit = 0.4,
                rows = processed.len(),
                cols = processed[0].len(),
                "post_process_table_inner: rejected table"
            );
            return None;
        }
    }

    {
        let num_cols = processed[0].len();
        let num_data_rows = processed.len() - 1;
        if num_data_rows > 20 && num_cols <= 3 {
            let total_data_cells = num_data_rows * num_cols;
            let filled_cells = processed[1..]
                .iter()
                .flat_map(|row| row.iter())
                .filter(|cell| !cell.trim().is_empty())
                .count();
            if total_data_cells > 0
                && filled_cells * 100 > total_data_cells * 80
                && looks_like_prose_in_columns(&processed[1..], num_cols)
            {
                tracing::debug!(
                    target: "xberg::table_reconstruct",
                    reason = "prose_in_columns_dense",
                    filled_cells,
                    total_data_cells,
                    fill_ratio = filled_cells as f64 / total_data_cells as f64,
                    limit = 0.8,
                    rows = processed.len(),
                    cols = num_cols,
                    "post_process_table_inner: rejected table"
                );
                return None;
            }
        }
    }

    {
        let num_cols = processed[0].len();
        let num_data_rows = processed.len() - 1;
        if (3..=5).contains(&num_cols) && num_data_rows >= 5 {
            let col_avg_lengths: Vec<f64> = (0..num_cols)
                .map(|c| {
                    let mut total_len = 0usize;
                    let mut count = 0usize;
                    for row in processed.iter().skip(1) {
                        let cell = row.get(c).map(|s| s.trim()).unwrap_or("");
                        if !cell.is_empty() {
                            total_len += cell.len();
                            count += 1;
                        }
                    }
                    if count > 0 {
                        total_len as f64 / count as f64
                    } else {
                        0.0
                    }
                })
                .collect();

            let text_col_avgs: Vec<f64> = col_avg_lengths.iter().copied().filter(|&avg| avg > 15.0).collect();

            if text_col_avgs.len() >= 3 {
                let min_avg = text_col_avgs.iter().copied().fold(f64::INFINITY, f64::min);
                let max_avg = text_col_avgs.iter().copied().fold(0.0_f64, f64::max);

                if min_avg > 0.0 && max_avg <= min_avg * 2.0 {
                    let total_data_cells = num_data_rows * num_cols;
                    let filled_cells = processed[1..]
                        .iter()
                        .flat_map(|row| row.iter())
                        .filter(|cell| !cell.trim().is_empty())
                        .count();
                    let fill_rate = filled_cells as f64 / total_data_cells as f64;
                    if fill_rate > 0.75 {
                        tracing::debug!(
                            target: "xberg::table_reconstruct",
                            reason = "uniform_column_prose",
                            min_avg,
                            max_avg,
                            fill_rate,
                            fill_rate_limit = 0.75,
                            rows = processed.len(),
                            cols = num_cols,
                            "post_process_table_inner: rejected table"
                        );
                        return None;
                    }
                }
            }
        }
    }

    for cell in &mut processed[0] {
        let text = cell.trim().replace("  ", " ");
        *cell = text;
    }

    // Gated per column, not applied to every data cell end to end: `normalize_data_cell`'s
    // dash/exponent rewriting is correct for a financial column (an em-dash cell means nil,
    // `1.5E-05` is scientific notation) and corrupts a prose column (`Functionaliteit—12`, a
    // part code `HRE - HReco`). Row 0 already never reaches this loop, kept above (xberg-io/
    // xberg#1582). ~keep
    let numeric_columns: Vec<bool> = (0..processed[0].len())
        .map(|col| column_is_numeric_for_normalization(&processed, col))
        .collect();

    for row in processed.iter_mut().skip(1) {
        for (col, cell) in row.iter_mut().enumerate() {
            if numeric_columns.get(col).copied().unwrap_or(false) {
                normalize_data_cell(cell);
            } else {
                let trimmed = cell.trim().to_string();
                *cell = trimmed;
            }
        }
    }

    tracing::debug!(
        target: "xberg::table_reconstruct",
        rows = processed.len(),
        cols = processed[0].len(),
        "post_process_table_inner: accepted table"
    );

    Some(processed)
}

fn find_data_start(table: &[Vec<String>], layout_guided: bool) -> usize {
    // A first row that is fully populated and holds no digit at all is unambiguously a text
    // header label row (e.g. "DATE | DESCRIPTION | WITHDRAWAL | DEPOSIT | BALANCE") -- trust it
    // outright rather than scanning forward for enough numeric cells to declare data started.
    // Without this, a sparse first DATA row (e.g. a bank statement's opening-balance entry,
    // which has neither a withdrawal nor a deposit, so only 2 of 5 cells are numeric) can fall
    // short of `DEFAULT_MIN_DATA_ROW_DIGIT_CELLS` and get folded into a bogus multi-row header
    // merge with the row after it (xberg-io/xberg#1649).
    //
    // Scoped to `!layout_guided`: a layout-guided (ML-confirmed) table already has the more
    // deliberate `looks_like_multiline_numeric_header`/repeated-row-shape logic below to decide
    // whether a digit-bearing second row is a units-annotation header continuation or real data,
    // and this early return must not preempt that.
    //
    // Also gated on row 1 looking like data (holding at least one digit): a genuine two-row text
    // header (e.g. "Region | Sales Amount | Growth Rate" over "Area Code | Dollars | Percent")
    // also has a fully populated, digit-free first row, and without this guard the shortcut
    // stops one row too early, folding the second header row into the data (xberg-io/xberg#1649
    // review follow-up). ~keep
    if !layout_guided
        && let Some(first_row) = table.first()
        && !first_row.is_empty()
        && first_row.iter().all(|cell| !cell.trim().is_empty())
        && digit_cell_count(first_row) == 0
        && table.get(1).is_some_and(|row| digit_cell_count(row) > 0)
    {
        return 1;
    }

    let first_numeric_row = table
        .iter()
        .position(|row| digit_cell_count(row) >= DEFAULT_MIN_DATA_ROW_DIGIT_CELLS)
        .unwrap_or(0);
    let column_count = table.first().map_or(0, Vec::len);
    if !layout_guided || column_count < LARGE_TABLE_MIN_COLUMNS || table.len() < REPEATED_DATA_ROW_COUNT {
        return first_numeric_row;
    }

    let repeated_start = table.windows(REPEATED_DATA_ROW_COUNT).position(|rows| {
        rows.iter()
            .all(|row| digit_cell_count(row) >= DEFAULT_MIN_DATA_ROW_DIGIT_CELLS)
            && rows.windows(2).all(|pair| row_shapes_match(&pair[0], &pair[1]))
    });
    repeated_start
        .filter(|&start| {
            start == first_numeric_row
                || looks_like_multiline_numeric_header(&table[first_numeric_row], &table[first_numeric_row + 1..start])
        })
        .unwrap_or(first_numeric_row)
}

fn looks_like_multiline_numeric_header(header: &[String], continuation_rows: &[Vec<String>]) -> bool {
    let filled_header_cells = header.iter().filter(|cell| !cell.trim().is_empty()).count();
    let multiword_labels = header
        .iter()
        .filter(|cell| {
            let text = cell.trim();
            text.split_whitespace().count() >= 2 && text.chars().any(char::is_alphabetic)
        })
        .count();
    let continuation_cells: Vec<&str> = continuation_rows
        .iter()
        .flat_map(|row| row.iter())
        .map(|cell| cell.trim())
        .filter(|cell| !cell.is_empty())
        .collect();
    let has_parenthesized_unit = continuation_cells
        .iter()
        .any(|cell| cell.starts_with('(') && cell.contains(')'));

    !continuation_rows.is_empty()
        && multiword_labels >= 2
        && continuation_cells.len() < filled_header_cells
        && has_parenthesized_unit
}

fn digit_cell_count(row: &[String]) -> usize {
    row.iter()
        .filter(|cell| cell.chars().any(|character| character.is_ascii_digit()))
        .count()
}

fn row_shapes_match(left: &[String], right: &[String]) -> bool {
    let column_count = left.len().max(right.len());
    let mut occupied_union = 0usize;
    let mut occupied_intersection = 0usize;
    for column in 0..column_count {
        let left_filled = left.get(column).is_some_and(|cell| !cell.trim().is_empty());
        let right_filled = right.get(column).is_some_and(|cell| !cell.trim().is_empty());
        occupied_union += usize::from(left_filled || right_filled);
        occupied_intersection += usize::from(left_filled && right_filled);
    }
    occupied_union > 0
        && occupied_intersection.saturating_mul(100) >= occupied_union.saturating_mul(ROW_SHAPE_MIN_OVERLAP_PERCENT)
}

/// Remove one empty-header interior track that only catches a stray word in a
/// large, otherwise dense layout-guided table. Such tracks arise when a footer
/// word has an x-position that does not occur in the table body.
fn prune_spurious_interior_column(
    table: &mut [Vec<String>],
    layout_guided: bool,
    column_positions: Option<&mut Vec<u32>>,
) -> bool {
    let Some(header) = table.first() else {
        return false;
    };
    let column_count = header.len();
    let data_row_count = table.len().saturating_sub(1);
    if !layout_guided || column_count < SPURIOUS_COLUMN_MIN_COLUMNS || data_row_count < SPURIOUS_COLUMN_MIN_DATA_ROWS {
        return false;
    }

    let candidates: Vec<usize> = (1..column_count - 1)
        .filter(|&column| header[column].trim().is_empty())
        .filter(|&column| {
            let populated_rows: Vec<usize> = table[1..]
                .iter()
                .enumerate()
                .filter_map(|(index, row)| {
                    row.get(column)
                        .is_some_and(|cell| !cell.trim().is_empty())
                        .then_some(index)
                })
                .collect();
            populated_rows.as_slice() == [data_row_count - 1]
                && table.last().is_some_and(|row| looks_like_footer_row(row))
        })
        .collect();
    let [column] = candidates.as_slice() else {
        return false;
    };

    let retained_cells = data_row_count.saturating_mul(column_count - 1);
    let retained_filled = table[1..]
        .iter()
        .flat_map(|row| row.iter().enumerate())
        .filter(|(index, cell)| *index != *column && !cell.trim().is_empty())
        .count();
    if retained_cells == 0
        || retained_filled.saturating_mul(100)
            < retained_cells.saturating_mul(SPURIOUS_COLUMN_MIN_RETAINED_DENSITY_PERCENT)
    {
        return false;
    }

    merge_interior_column(table, *column);
    drop_column_position(column_positions, *column);
    true
}

fn looks_like_footer_row(row: &[String]) -> bool {
    let non_empty: Vec<&str> = row
        .iter()
        .map(|cell| cell.trim())
        .filter(|cell| !cell.is_empty())
        .collect();
    if non_empty.len() < 2 || !non_empty.iter().any(|cell| cell.split_whitespace().count() >= 2) {
        return false;
    }
    let text = non_empty.join(" ");
    let alphanumeric = text.chars().filter(|character| character.is_alphanumeric()).count();
    let alphabetic = text.chars().filter(|character| character.is_alphabetic()).count();
    alphanumeric > 0 && alphabetic.saturating_mul(100) >= alphanumeric.saturating_mul(FOOTER_MIN_ALPHA_PERCENT)
}

fn merge_interior_column(table: &mut [Vec<String>], column: usize) {
    let left_occupancy = table[1..]
        .iter()
        .filter(|row| row.get(column - 1).is_some_and(|cell| !cell.trim().is_empty()))
        .count();
    let right_occupancy = table[1..]
        .iter()
        .filter(|row| row.get(column + 1).is_some_and(|cell| !cell.trim().is_empty()))
        .count();
    let merge_right = right_occupancy >= left_occupancy;

    for row in table {
        let text = row.remove(column).trim().to_string();
        if text.is_empty() {
            continue;
        }
        let target = if merge_right { column } else { column - 1 };
        let existing = row[target].trim();
        row[target] = if existing.is_empty() {
            text
        } else if merge_right {
            format!("{text} {existing}")
        } else {
            format!("{existing} {text}")
        };
    }
}

/// Minimum non-empty cells for [`looks_like_shredded_prose_row`] to consider a
/// row "densely filled" rather than a sparse real table row.
const SHREDDED_PROSE_MIN_FILLED_CELLS: usize = 4;
/// A shredded-prose cell averages this many words or fewer (unlike
/// `PROSE_WORDS_PER_CELL`'s phrase-per-cell prose, single-word cells here are
/// the row-shredding signal).
const SHREDDED_PROSE_MAX_AVG_WORDS_PER_CELL: f64 = 2.5;
/// Minimum concatenated row text length for [`looks_like_shredded_prose_row`]
/// to consider a row substantial enough to be a real clause rather than a
/// handful of short table values.
const SHREDDED_PROSE_MIN_ROW_TEXT_LEN: usize = 30;

/// Decide whether a single data row reads as one clause of a word-shredded,
/// semicolon-delimited prose list rather than genuine table data: most of the
/// row's columns are filled (a real table row from a word-wrapped table
/// fragment leaves many columns empty; a shredded sentence naturally
/// populates almost every column), the cells average few words each (mirrors
/// the one-word-per-cell splitting), the row reads as a substantial run of
/// text, and it ends on clause-terminal punctuation.
fn looks_like_shredded_prose_row(row: &[String], num_cols: usize) -> bool {
    let cells: Vec<&str> = row.iter().map(|c| c.trim()).filter(|c| !c.is_empty()).collect();
    if cells.len() < SHREDDED_PROSE_MIN_FILLED_CELLS {
        return false;
    }
    if num_cols == 0 || (cells.len() as f64) <= num_cols as f64 * 0.5 {
        return false;
    }

    let concatenated_len: usize = cells.iter().map(|c| c.len()).sum();
    if concatenated_len < SHREDDED_PROSE_MIN_ROW_TEXT_LEN {
        return false;
    }

    let total_words: usize = cells.iter().map(|c| c.split_whitespace().count()).sum();
    let avg_words = total_words as f64 / cells.len() as f64;
    if avg_words > SHREDDED_PROSE_MAX_AVG_WORDS_PER_CELL {
        return false;
    }

    cells
        .last()
        .is_some_and(|last| matches!(last.chars().last(), Some(';' | ':' | '.' | ',')))
}

/// Decide whether a dense grid of data rows is prose laid out in columns rather
/// than a real table. The signal is words-per-cell: a table cell holds a value (a
/// number, a code, a short label), while columned prose (a two-column article, a
/// wrapped paragraph) fills each cell with a phrase. This gates the density guard
/// so that a dense numeric ledger (Account | Amount | Note, 30+ rows) is not cut by
/// row-count alone; genuinely alphabetic prose is still caught downstream by the
/// alpha-ratio row-coherence check in `is_well_formed_table` (xberg-io/xberg#1223).
fn looks_like_prose_in_columns(data_rows: &[Vec<String>], num_cols: usize) -> bool {
    /// A cell averaging this many words or more reads as a phrase, not a value.
    const PROSE_WORDS_PER_CELL: f64 = 4.0;

    if num_cols < 2 {
        return false;
    }
    let mut prose_rows = 0usize;
    let mut eligible_rows = 0usize;
    for row in data_rows {
        let cells: Vec<&str> = row.iter().map(|c| c.trim()).filter(|c| !c.is_empty()).collect();
        if cells.len() < 2 {
            continue;
        }
        let total_len: usize = cells.iter().map(|c| c.len()).sum();
        if total_len < 15 {
            continue;
        }
        eligible_rows += 1;
        let total_words: usize = cells.iter().map(|c| c.split_whitespace().count()).sum();
        let avg_words = total_words as f64 / cells.len() as f64;
        if avg_words >= PROSE_WORDS_PER_CELL {
            prose_rows += 1;
        }
    }
    eligible_rows >= 3 && prose_rows * 2 > eligible_rows
}

/// A cell containing at least one alphabetic run but not itself a numeric value.
/// Word cells are the signal of wrapped prose (as opposed to numeric table data)
/// when the grid's cells are too thin to average four words.
fn is_word_cell(cell: &str) -> bool {
    !is_numeric_value_cell(cell) && cell.chars().any(|c| c.is_alphabetic())
}

/// Decide whether a 1–2 data-row grid is really a wrapped-prose passage split
/// across columns rather than a genuine short table. This closes the short-grid
/// hole where the ≥3-row alpha guard, the ≥4-row uniformity/vocabulary guards,
/// and the shredded-prose branch (which demands *every* row end on clause-
/// terminal punctuation) all miss it, so it reaches `return true` and is
/// fabricated as a table (xberg-io/xberg#36).
///
/// Two prose shapes are detected, both applied per row:
/// - **phrase-per-cell** — cells average ≥ `PROSE_WORDS_PER_CELL` words and the
///   row is alphabetic (`alpha_ratio > 0.8`): columns of full phrases (a 2–5
///   column reflow of body text).
/// - **wide-shredded** — a wide row (≥ `MIN_SHREDDED_WORD_CELLS` filled cells)
///   of thin cells (≤ 2.5 words each) that are mostly word cells: a single
///   prose line chopped into one-or-two-word columns (the multi-column academic
///   misparse, e.g. arxiv 0903.1810).
///
/// Genuine short tables survive via a numeric-**fraction** exemption: a real
/// numeric table is mostly value cells, whereas prose that merely contains an
/// equation or a stray number is not. Requiring *every* eligible row to read as
/// prose is deliberately conservative — at 1–2 rows there is no cross-row
/// evidence to average over.
fn looks_like_short_columned_prose(data_rows: &[Vec<String>], num_cols: usize) -> bool {
    /// A cell averaging this many words or more reads as a phrase, not a value.
    /// Mirrors `PROSE_WORDS_PER_CELL` in [`looks_like_prose_in_columns`].
    const SHORT_PROSE_WORDS_PER_CELL: f64 = 4.0;
    /// Above this alphabetic+whitespace fraction a phrase row reads as prose.
    /// Mirrors the alpha-ratio cutoff in [`is_well_formed_table`].
    const SHORT_PROSE_ALPHA_RATIO: f64 = 0.8;
    /// Minimum concatenated row text length to be eligible. Mirrors the 15-char
    /// floor in [`looks_like_prose_in_columns`].
    const SHORT_PROSE_MIN_CONCAT_LEN: usize = 15;
    /// A numeric-value cell fraction at or above this keeps the grid: a genuine
    /// short table is mostly values; prose with an incidental number is not.
    const SHORT_PROSE_NUMERIC_EXEMPT_PERCENT: usize = 30;
    /// A shredded row needs at least this many filled cells — narrow grids are
    /// left to the phrase-per-cell shape so 2-column key/value stays a table.
    const MIN_SHREDDED_WORD_CELLS: usize = 4;
    /// A shredded row's cells average at most this many words (one-or-two-word
    /// fragments). Mirrors `SHREDDED_PROSE_MAX_AVG_WORDS_PER_CELL`.
    const SHREDDED_MAX_AVG_WORDS: f64 = 2.5;
    /// At least this fraction of a shredded row's filled cells must be word
    /// cells (not numbers) for it to read as prose rather than a numeric row.
    const SHREDDED_MIN_WORD_CELL_FRACTION: f64 = 0.6;

    if num_cols < 2 {
        return false;
    }

    let mut filled_cells = 0usize;
    let mut numeric_value_cells = 0usize;
    for row in data_rows {
        for cell in row {
            let trimmed = cell.trim();
            if trimmed.is_empty() {
                continue;
            }
            filled_cells += 1;
            if is_numeric_value_cell(trimmed) {
                numeric_value_cells += 1;
            }
        }
    }
    if filled_cells == 0 {
        return false;
    }
    if numeric_value_cells * 100 >= filled_cells * SHORT_PROSE_NUMERIC_EXEMPT_PERCENT {
        return false;
    }

    let mut eligible_rows = 0usize;
    let mut prose_rows = 0usize;
    for row in data_rows {
        let cells: Vec<&str> = row.iter().map(|c| c.trim()).filter(|c| !c.is_empty()).collect();
        if cells.len() < 2 {
            continue;
        }
        let concatenated = cells.join(" ");
        if concatenated.len() < SHORT_PROSE_MIN_CONCAT_LEN {
            continue;
        }
        eligible_rows += 1;

        let total_words: usize = cells.iter().map(|c| c.split_whitespace().count()).sum();
        let avg_words = total_words as f64 / cells.len() as f64;
        let alpha_ratio = {
            let alpha = concatenated
                .chars()
                .filter(|c| c.is_alphabetic() || c.is_whitespace())
                .count();
            alpha as f64 / concatenated.len() as f64
        };
        let is_phrase_prose = avg_words >= SHORT_PROSE_WORDS_PER_CELL && alpha_ratio > SHORT_PROSE_ALPHA_RATIO;

        let word_cells = cells.iter().filter(|c| is_word_cell(c)).count();
        let is_shredded_prose = cells.len() >= MIN_SHREDDED_WORD_CELLS
            && avg_words <= SHREDDED_MAX_AVG_WORDS
            && word_cells as f64 >= cells.len() as f64 * SHREDDED_MIN_WORD_CELL_FRACTION;

        if is_phrase_prose || is_shredded_prose {
            prose_rows += 1;
        }
    }

    eligible_rows >= 1 && prose_rows * 2 > eligible_rows
}

/// Validate whether a reconstructed table grid represents a well-formed table
/// rather than multi-column prose or a repeated page element.
///
/// Returns `true` if the grid looks like a real table, `false` if it should be
/// rejected and its content emitted as paragraph text instead.
///
/// The checks catch cases the layout model misidentifies as tables:
/// - Multi-column prose split into a grid (detected via row coherence and column uniformity)
/// - Repeated page elements (headers/footers detected as tables on every page)
/// - Low-vocabulary repetitive content (same few words in every row)
pub(crate) fn is_well_formed_table(grid: &[Vec<String>]) -> bool {
    is_well_formed_table_core(grid, false)
}

/// Share of (column boundary, row) pairs whose boundary has a word running
/// across it, above which a rule-less candidate is treated as prose (#1399).
///
/// Deliberately NOT the ~30% the issue proposed. Measured over 147 genuine
/// ruled-table regions from `test_documents/pdf/`, each scored on its own
/// bounding box (the reconstructor only ever sees a pre-segmented region, so
/// scoring whole pages mixes in surrounding prose and inflates the ratio):
/// min 11.9%, median 33.0%, p90 45.6%, max 74.7%. A 30% cut rejects 94 of
/// those 147 real tables; 50% rejects 8; 60% rejects 4. The #1399 prose region
/// scores 65.0%, and every prose page in that document scores 47.8-75.5%, so
/// 60% separates the reported defect from real tables with the least collateral
/// damage this signal can achieve on its own. It is deliberately a weak gate:
/// the ruling-line check below is the load-bearing one. ~keep
const MAX_STRADDLED_BOUNDARY_RATIO: f64 = 0.60;

/// Row-grouping tolerance as a multiple of median word height, matching the
/// value `reconstruct_table` itself uses so both see the same rows.
const STRADDLE_ROW_THRESHOLD_RATIO: f64 = 0.5;

/// Fraction of (column boundary, row) pairs crossed by a word's bounding box.
///
/// A column boundary is the *start* of the next column, not the midpoint
/// between two column positions. [`crate::table_core::detect_columns`] returns
/// each column's **median left edge**, so a midpoint between two such medians
/// falls inside the left column's own text rather than in the gutter, and any
/// word wider than half the column pitch straddles it — flagging legitimate
/// tables that merely contain one long word.
///
/// Measured per row rather than over the whole region: the issue's definition
/// is that a column boundary is a vertical band of whitespace which holds on
/// every row, so text running across it *on most rows* is what disqualifies it.
/// A single long word on one row is not evidence of prose.
pub(crate) fn straddled_boundary_ratio(region: &[HocrWord], column_positions: &[u32]) -> f64 {
    if column_positions.len() < 2 || region.is_empty() {
        return 0.0;
    }

    let row_positions = crate::table_core::detect_rows(region, STRADDLE_ROW_THRESHOLD_RATIO);
    if row_positions.is_empty() {
        return 0.0;
    }

    let mut rows: Vec<Vec<&HocrWord>> = vec![Vec::new(); row_positions.len()];
    for word in region {
        let y_center = word.y_center() as u32;
        let Some((index, _)) = row_positions
            .iter()
            .enumerate()
            .min_by_key(|&(_, row_y)| row_y.abs_diff(y_center))
        else {
            continue;
        };
        rows[index].push(word);
    }

    let mut total = 0usize;
    let mut straddled = 0usize;
    for &boundary in &column_positions[1..] {
        for row in &rows {
            total += 1;
            if row
                .iter()
                .any(|word| word.left < boundary && word.left.saturating_add(word.width) > boundary)
            {
                straddled += 1;
            }
        }
    }

    if total == 0 {
        return 0.0;
    }
    straddled as f64 / total as f64
}

/// Well-formedness gate for the borderless heuristic path, which is the only
/// caller holding raw word geometry and the page's drawn-rule count.
///
/// Implements the two-signal admission test from xberg-io/xberg#1399:
///
/// 1. **Drawn ruling lines (positive).** A page whose region carries horizontal
///    rules had a producer that drew a table, so the candidate is admitted. The
///    reporter's survey found no real table lacking rules, and this is the
///    strong signal — see [`MAX_STRADDLED_BOUNDARY_RATIO`] for why the
///    geometric signal alone cannot carry the decision.
/// 2. **Are the column boundaries actually whitespace (fallback)?** With no
///    rules to go on, test the definition of a column. Continuous prose that
///    merely aligns into column-like x-buckets has words running across those
///    boundaries on most rows; a real borderless table does not, because its
///    cells do not overlap.
///
/// This only ever narrows acceptance relative to [`is_well_formed_table`].
pub(crate) fn is_well_formed_borderless_table(
    grid: &[Vec<String>],
    region: &[HocrWord],
    column_positions: &[u32],
    horizontal_rules: usize,
) -> bool {
    if !is_well_formed_table(grid) {
        return false;
    }
    if horizontal_rules > 0 {
        return true;
    }
    straddled_boundary_ratio(region, column_positions) < MAX_STRADDLED_BOUNDARY_RATIO
}

/// Core well-formedness check. `skip_columnar_prose_guard` drops only the
/// uniform-column-length prose heuristic, for callers that have already vetted
/// the region's columnar structure geometrically (the #1319 text-heavy geometric
/// fallback). A genuine key-value grid has regular, short column lengths that
/// this heuristic mistakes for wrapped columnar prose; every other structural
/// guard (empty-cell fraction, shredded-row, alpha-ratio, unique-word, and
/// header-duplication checks) still applies.
pub(crate) fn is_well_formed_table_core(grid: &[Vec<String>], skip_columnar_prose_guard: bool) -> bool {
    if grid.len() < 2 {
        return false;
    }
    let num_cols = grid[0].len();
    if num_cols < 2 {
        return false;
    }
    let dense_numeric_grid = is_dense_numeric_grid(grid);

    const DEFAULT_MAX_EMPTY_CELL_PERCENT: usize = 40;
    let data_row_count = grid.len().saturating_sub(1);
    let max_empty_cell_percent =
        if data_row_count <= SHORT_WIDE_MAX_DATA_ROWS && num_cols >= SHORT_WIDE_MIN_COLUMNS && !dense_numeric_grid {
            SHORT_WIDE_MAX_EMPTY_CELL_PERCENT
        } else {
            DEFAULT_MAX_EMPTY_CELL_PERCENT
        };
    let max_cols = grid.iter().map(|r| r.len()).max().unwrap_or(0);
    let total_cells = grid.len() * max_cols;
    if total_cells > 0 {
        let empty_cells = grid.len() * max_cols
            - grid
                .iter()
                .flat_map(|row| row.iter())
                .filter(|cell| !cell.trim().is_empty())
                .count();
        if empty_cells * 100 > total_cells * max_empty_cell_percent {
            return false;
        }
    }

    let data_rows = &grid[1..];

    if (1..3).contains(&data_rows.len()) && num_cols >= LARGE_TABLE_MIN_COLUMNS && !dense_numeric_grid {
        let shredded_rows = data_rows
            .iter()
            .filter(|row| looks_like_shredded_prose_row(row, num_cols))
            .count();
        if shredded_rows == data_rows.len() {
            return false;
        }
    }

    if !data_rows.is_empty()
        && num_cols >= 2
        && !dense_numeric_grid
        && looks_like_short_columned_prose(data_rows, num_cols)
    {
        return false;
    }

    if data_rows.len() >= 3 && num_cols >= 2 {
        let mut prose_like_rows = 0usize;
        let mut eligible_rows = 0usize;

        for row in data_rows {
            let concatenated: String = row
                .iter()
                .map(|c| c.trim())
                .filter(|c| !c.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            if concatenated.len() < 15 {
                continue;
            }
            eligible_rows += 1;

            let alpha_ratio = {
                let alpha = concatenated
                    .chars()
                    .filter(|c| c.is_alphabetic() || c.is_whitespace())
                    .count();
                alpha as f64 / concatenated.len() as f64
            };
            if alpha_ratio > 0.8 {
                prose_like_rows += 1;
            }
        }

        if eligible_rows >= 3 && prose_like_rows * 2 > eligible_rows {
            return false;
        }
    }

    if num_cols >= 3 && data_rows.len() >= 4 {
        let col_stats: Vec<(f64, f64)> = (0..num_cols)
            .map(|c| {
                let lengths: Vec<f64> = data_rows
                    .iter()
                    .filter_map(|row| {
                        let cell = row.get(c).map(|s| s.trim()).unwrap_or("");
                        if cell.is_empty() { None } else { Some(cell.len() as f64) }
                    })
                    .collect();
                if lengths.is_empty() {
                    return (0.0, 0.0);
                }
                let mean = lengths.iter().sum::<f64>() / lengths.len() as f64;
                let variance = lengths.iter().map(|l| (l - mean).powi(2)).sum::<f64>() / lengths.len() as f64;
                let stddev = variance.sqrt();
                (mean, stddev)
            })
            .collect();

        let meaningful: Vec<(f64, f64)> = col_stats.iter().copied().filter(|(m, _)| *m > 3.0).collect();

        if meaningful.len() >= 3 {
            let means: Vec<f64> = meaningful.iter().map(|(m, _)| *m).collect();
            let min_mean = means.iter().copied().fold(f64::INFINITY, f64::min);
            let max_mean = means.iter().copied().fold(0.0_f64, f64::max);

            let columns_uniform = min_mean > 0.0 && max_mean <= min_mean * 2.0;

            let low_variance = meaningful
                .iter()
                .all(|(mean, stddev)| *mean > 0.0 && *stddev / *mean < 0.3);

            if !skip_columnar_prose_guard && !dense_numeric_grid && columns_uniform && low_variance {
                return false;
            }
        }
    }

    if num_cols >= 3 {
        let mut unique_words: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for row in data_rows {
            for cell in row {
                for word in cell.split_whitespace() {
                    unique_words.insert(word);
                }
            }
        }
        let row_count = data_rows.len();
        if !dense_numeric_grid && row_count >= 3 && unique_words.len() < row_count * 2 {
            return false;
        }
    }

    if !grid.is_empty() {
        let header = &grid[0];
        let header_matches = data_rows
            .iter()
            .filter(|row| row.len() == header.len() && row.iter().zip(header.iter()).all(|(a, b)| a.trim() == b.trim()))
            .count();
        if header_matches >= 2 {
            return false;
        }
    }

    true
}

fn is_dense_numeric_grid(grid: &[Vec<String>]) -> bool {
    let Some(header) = grid.first() else {
        return false;
    };
    if header.len() < DENSE_NUMERIC_MIN_COLUMNS || grid.len() <= DENSE_NUMERIC_MIN_DATA_ROWS {
        return false;
    }

    let mut non_empty_cells = 0usize;
    let mut numeric_cells = 0usize;
    for cell in grid.iter().skip(1).flat_map(|row| row.iter()) {
        let trimmed = cell.trim();
        if trimmed.is_empty() {
            continue;
        }
        non_empty_cells += 1;
        if is_numeric_value_cell(trimmed) {
            numeric_cells += 1;
        }
    }

    non_empty_cells > 0
        && numeric_cells.saturating_mul(100) >= non_empty_cells.saturating_mul(DENSE_NUMERIC_MIN_CELL_PERCENT)
}

/// Whether the grid's data cells are overwhelmingly numeric values, with no
/// row/column-count floor (unlike [`is_dense_numeric_grid`], which is calibrated
/// for large 6×6+ tables). A short, wide grid of one-or-two-word cells that is
/// this numeric is genuine tabular data — a borderless invoice/line-item table —
/// not shredded prose, which is alphabetic. Used only to exempt such grids from
/// the ≥5-column single-word prose guard (xberg-io/xberg#1316).
fn is_predominantly_numeric_short_grid(grid: &[Vec<String>]) -> bool {
    // Measure the numeric fraction two ways and accept if either clears the bar:
    // over every data cell, and over the substantially-populated data rows only. A
    // borderless line-item table can carry a sparse continuation row — a wrapped
    // description with the remaining columns blank (xberg-io/xberg#1333). That row
    // is all-text and, pooled with the populated rows, drags the numeric fraction
    // below the bar. Requiring substantial rather than total occupancy also
    // tolerates a small number of inferred empty columns (xberg-io/xberg#1342).
    // This selective pass only grants the exemption, so it cannot demote a grid
    // the pooled pass already accepts.
    let width = grid.first().map_or(0, Vec::len);
    short_grid_numeric_ratio_meets_bar(grid, false) || (width > 0 && short_grid_numeric_ratio_meets_bar(grid, true))
}

/// Whether the numeric fraction of a short grid's data cells clears the
/// [`SHORT_NUMERIC_MIN_CELL_PERCENT`] bar with at least
/// [`SHORT_NUMERIC_MIN_DATA_CELLS`] cells of evidence. When
/// `substantially_populated_rows_only` is set, only rows meeting
/// [`SHORT_NUMERIC_MIN_ROW_OCCUPANCY_PERCENT`] contribute, so sparse continuation
/// rows and a few inferred empty columns do not distort the measurement (see
/// [`is_predominantly_numeric_short_grid`]).
fn short_grid_numeric_ratio_meets_bar(grid: &[Vec<String>], substantially_populated_rows_only: bool) -> bool {
    let width = grid.first().map_or(0, Vec::len);
    let mut non_empty_cells = 0usize;
    let mut numeric_cells = 0usize;
    for row in grid.iter().skip(1) {
        if substantially_populated_rows_only && !is_substantially_populated_data_row(row, width) {
            continue;
        }
        for cell in row {
            let trimmed = cell.trim();
            if trimmed.is_empty() {
                continue;
            }
            non_empty_cells += 1;
            if is_numeric_value_cell(trimmed) {
                numeric_cells += 1;
            }
        }
    }
    non_empty_cells >= SHORT_NUMERIC_MIN_DATA_CELLS
        && numeric_cells.saturating_mul(100) >= non_empty_cells.saturating_mul(SHORT_NUMERIC_MIN_CELL_PERCENT)
}

/// Whether the row fills enough of the inferred grid to be self-contained.
fn is_substantially_populated_data_row(row: &[String], width: usize) -> bool {
    if width == 0 {
        return false;
    }
    let populated = row.iter().take(width).filter(|cell| !cell.trim().is_empty()).count();
    populated.saturating_mul(100) >= width.saturating_mul(SHORT_NUMERIC_MIN_ROW_OCCUPANCY_PERCENT)
}

fn is_dense_scalar_grid(grid: &[Vec<String>]) -> bool {
    let Some(header) = grid.first() else {
        return false;
    };
    let data_rows = grid.len().saturating_sub(1);
    if header.len() < DENSE_SCALAR_MIN_COLUMNS || data_rows < DENSE_SCALAR_MIN_DATA_ROWS {
        return false;
    }

    let total_cells = data_rows.saturating_mul(header.len());
    let mut filled_cells = 0usize;
    let mut compact_cells = 0usize;
    let mut digit_cells = 0usize;
    for cell in grid.iter().skip(1).flat_map(|row| row.iter()) {
        let trimmed = cell.trim();
        if trimmed.is_empty() {
            continue;
        }
        filled_cells += 1;
        if trimmed.chars().count() <= DENSE_SCALAR_MAX_CELL_CHARS && trimmed.split_whitespace().count() <= 2 {
            compact_cells += 1;
        }
        if trimmed.chars().any(|c| c.is_ascii_digit()) {
            digit_cells += 1;
        }
    }

    total_cells > 0
        && filled_cells.saturating_mul(100) >= total_cells.saturating_mul(DENSE_SCALAR_MIN_FILLED_PERCENT)
        && compact_cells.saturating_mul(100) >= filled_cells.saturating_mul(DENSE_SCALAR_MIN_COMPACT_PERCENT)
        && digit_cells.saturating_mul(100) >= filled_cells.saturating_mul(DENSE_SCALAR_MIN_DIGIT_PERCENT)
}

fn is_numeric_value_cell(cell: &str) -> bool {
    let digit_count = cell.chars().filter(char::is_ascii_digit).count();
    if digit_count == 0 {
        return false;
    }
    let alphanumeric_count = cell.chars().filter(|c| c.is_alphanumeric()).count();
    digit_count.saturating_mul(2) >= alphanumeric_count
}

/// Minimum fraction of non-empty table cells that must contain curly braces
/// (`{` or `}`) for the region to be classified as a code listing rather than
/// a table. At 0.20, one brace-containing cell per five non-empty cells is
/// enough to trigger the guard.
///
/// A separate hard-reject fires when any non-empty cell is *exactly* `{` or `}`:
/// isolated braces appear only in code block delimiters, never in real table data.
const CODE_BRACE_CELL_FRACTION: f64 = 0.20;

/// Returns `true` if the reconstructed table grid looks like a code listing
/// rather than genuine tabular data.
///
/// The layout model and text-edge heuristic occasionally misclassify code blocks
/// (especially C-family language listings with curly-brace syntax) as table
/// regions, because monospace character spacing creates apparent column positions.
///
/// Three signals are checked:
/// 1. **Hard reject**: any non-empty cell whose entire trimmed text is `{` or
///    `}` (an isolated brace cannot appear in real table content).
/// 2. **Fraction check**: if ≥ [`CODE_BRACE_CELL_FRACTION`] of non-empty cells
///    contain `{` or `}`, the region is likely code with inline block syntax.
/// 3. **Declaration grid**: a lone, unterminated C-family function declaration
///    head followed by pointer-bearing, comma-delimited parameter rows. A
///    terminal `);` or comma termination on every parameter row is required to
///    avoid rejecting API-reference tables with incidental code punctuation.
///
/// Python, Ruby, and other brace-free languages are not caught by this check;
/// those rarely produce false-positive tables at the heuristic tier.
pub(crate) fn looks_like_code_listing(table_cells: &[Vec<String>]) -> bool {
    let non_empty: Vec<&str> = table_cells
        .iter()
        .flat_map(|row| row.iter())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    if non_empty.is_empty() {
        return false;
    }

    if non_empty.iter().any(|&cell| cell == "{" || cell == "}") {
        return true;
    }

    let brace_count = non_empty
        .iter()
        .filter(|&&cell| cell.contains('{') || cell.contains('}'))
        .count();
    (brace_count as f64) / (non_empty.len() as f64) >= CODE_BRACE_CELL_FRACTION
        || looks_like_declaration_grid(table_cells)
}

fn looks_like_declaration_grid(table_cells: &[Vec<String>]) -> bool {
    let Some(first_row) = table_cells.first() else {
        return false;
    };
    let mut first_cells = first_row.iter().map(|cell| cell.trim()).filter(|cell| !cell.is_empty());
    let Some(head) = first_cells.next() else {
        return false;
    };
    if first_cells.next().is_some() || !looks_like_declaration_head(head) {
        return false;
    }

    let continuation_rows: Vec<&[String]> = table_cells
        .iter()
        .skip(1)
        .filter(|row| row.iter().any(|cell| !cell.trim().is_empty()))
        .map(Vec::as_slice)
        .collect();
    let evidence: Vec<ParameterRowEvidence> = continuation_rows
        .iter()
        .filter_map(|row| parameter_row_evidence(row))
        .collect();
    if evidence.len() < 2 || evidence.len() != continuation_rows.len() {
        return false;
    }

    let has_pointer = evidence.iter().any(|row| row.has_pointer);
    let has_closing_declaration = evidence.iter().any(|row| row.closes_declaration);
    let all_truncated_parameters = evidence.iter().all(|row| row.ends_with_comma);
    has_pointer && (has_closing_declaration || all_truncated_parameters)
}

#[derive(Clone, Copy)]
struct ParameterRowEvidence {
    ends_with_comma: bool,
    closes_declaration: bool,
    has_pointer: bool,
}

fn parameter_row_evidence(row: &[String]) -> Option<ParameterRowEvidence> {
    let cells: Vec<&str> = row
        .iter()
        .map(|cell| cell.trim())
        .filter(|cell| !cell.is_empty())
        .collect();
    if cells.len() < 2 {
        return None;
    }
    let last = cells.last()?;
    let (parameter_name, ends_with_comma, closes_declaration) = if let Some(name) = last.strip_suffix(',') {
        (name, true, false)
    } else if let Some(name) = last.strip_suffix(");") {
        (name, false, true)
    } else {
        return None;
    };
    if !looks_like_parameter_name(parameter_name) {
        return None;
    }

    Some(ParameterRowEvidence {
        ends_with_comma,
        closes_declaration,
        has_pointer: cells.iter().any(|cell| cell.contains('*')),
    })
}

fn looks_like_parameter_name(name: &str) -> bool {
    let name = name.trim().trim_start_matches('*');
    !name.is_empty()
        && name.chars().any(|character| character.is_alphabetic())
        && name
            .chars()
            .all(|character| character.is_alphanumeric() || matches!(character, '_' | '[' | ']'))
}

fn looks_like_declaration_head(head: &str) -> bool {
    let Some(prefix) = head.strip_suffix('(') else {
        return false;
    };
    let identifiers = prefix
        .split_whitespace()
        .filter(|token| token.chars().any(|character| character.is_alphabetic()))
        .count();
    identifiers >= 2
}

fn merge_header_only_column(
    table: &mut [Vec<String>],
    col: usize,
    header_text: String,
    column_positions: Option<&mut Vec<u32>>,
) {
    if table.is_empty() || table[0].is_empty() {
        return;
    }

    let trimmed = header_text.trim();
    if trimmed.is_empty() && table.len() > 1 {
        for row in table.iter_mut() {
            row.remove(col);
        }
        drop_column_position(column_positions, col);
        return;
    }

    if !trimmed.is_empty() {
        if col > 0 {
            let mut target = col - 1;
            while target > 0 && table[0][target].trim().is_empty() {
                target -= 1;
            }
            if !table[0][target].trim().is_empty() || target == 0 {
                if !table[0][target].is_empty() {
                    table[0][target].push(' ');
                }
                table[0][target].push_str(trimmed);
                for row in table.iter_mut() {
                    row.remove(col);
                }
                drop_column_position(column_positions, col);
                return;
            }
        }

        if col + 1 < table[0].len() {
            if table[0][col + 1].trim().is_empty() {
                table[0][col + 1] = trimmed.to_string();
            } else {
                let mut updated = trimmed.to_string();
                updated.push(' ');
                updated.push_str(table[0][col + 1].trim());
                table[0][col + 1] = updated;
            }
            for row in table.iter_mut() {
                row.remove(col);
            }
            drop_column_position(column_positions, col);
            return;
        }
    }

    for row in table.iter_mut() {
        row.remove(col);
    }
    drop_column_position(column_positions, col);
}

/// Remove the column-position entry for a column that just got dropped from the table
/// grid, keeping the two in lockstep for callers that index `column_positions` by
/// column after post-processing (xberg-io/xberg#863). A no-op when the caller isn't
/// tracking positions, or `col` is already out of bounds.
fn drop_column_position(column_positions: Option<&mut Vec<u32>>, col: usize) {
    if let Some(positions) = column_positions
        && col < positions.len()
    {
        positions.remove(col);
    }
}

fn normalize_data_cell(cell: &mut String) {
    let trimmed = cell.trim();
    if trimmed.is_empty() {
        cell.clear();
        return;
    }

    let mut text = normalize_dash_glyphs_and_spacing(trimmed);
    text = text.replace("E-", "e-").replace("E+", "e+");

    if text == "-" {
        text.clear();
    }

    *cell = text;
}

/// Rewrites em-dash, en-dash and minus-sign glyphs to an ASCII hyphen and collapses the
/// whitespace `normalize_data_cell` expects around a leading or embedded hyphen (`"- 3"` ->
/// `"-3"`), without the exponent lowercasing or lone-dash clearing that follow it. Shared
/// with [`column_is_numeric_for_normalization`], which needs the same dash-normalised
/// preview to decide whether a cell is numeric *before* `normalize_data_cell` runs on it —
/// testing the raw, unnormalised text would miss `"- 3"`, which only reads as a number once
/// this rewrite has run (xberg-io/xberg#1582). ~keep
fn normalize_dash_glyphs_and_spacing(text: &str) -> String {
    let mut text = text.to_string();
    for ch in ['\u{2014}', '\u{2013}', '\u{2212}'] {
        text = text.replace(ch, "-");
    }

    if text.starts_with("- ") {
        text = format!("-{}", text[2..].trim_start());
    }

    text = text.replace("- ", "-");
    text = text.replace(" -", "-");
    text
}

/// Whether column `col`'s data rows (everything but the header) are predominantly bare
/// numeric literals once dash glyphs are normalised — the gate that keeps
/// `normalize_data_cell` off a prose column. A cell that is nothing but a dash is
/// nil-or-N/A and cannot decide the question on its own, so it is excluded from the vote
/// and left to the column's other cells (xberg-io/xberg#1582).
fn column_is_numeric_for_normalization(table: &[Vec<String>], col: usize) -> bool {
    let mut evidence = 0usize;
    let mut numeric = 0usize;
    for row in table.iter().skip(1) {
        let Some(cell) = row.get(col) else { continue };
        let trimmed = cell.trim();
        if trimmed.is_empty() || is_lone_dash_cell(trimmed) {
            continue;
        }
        evidence += 1;
        if looks_like_numeric_literal(&normalize_dash_glyphs_and_spacing(trimmed)) {
            numeric += 1;
        }
    }
    evidence > 0 && numeric.saturating_mul(100) >= evidence.saturating_mul(NUMERIC_COLUMN_MIN_NUMERIC_PERCENT)
}

/// Whether `text` is nothing but one dash glyph (em, en, minus sign or ASCII hyphen) —
/// ambiguous nil-or-N/A content that carries no evidence either way for
/// [`column_is_numeric_for_normalization`].
fn is_lone_dash_cell(text: &str) -> bool {
    matches!(text, "-" | "\u{2014}" | "\u{2013}" | "\u{2212}")
}

/// Whether `text` — already run through [`normalize_dash_glyphs_and_spacing`] — is a bare
/// numeric literal: an optional leading `-`, one or more digits with at most one `.`, and
/// an optional exponent (`e`/`E`, optional sign, one or more digits). Anything containing a
/// letter outside that exponent marker, or no digits at all, is not a number (xberg-io/
/// xberg#1582).
fn looks_like_numeric_literal(text: &str) -> bool {
    let text = text.strip_prefix('-').unwrap_or(text);
    let (mantissa, exponent) = match text.find(['e', 'E']) {
        Some(index) => (&text[..index], Some(&text[index + 1..])),
        None => (text, None),
    };
    if !is_numeric_mantissa(mantissa) {
        return false;
    }
    exponent.is_none_or(|exp| {
        let digits = exp.strip_prefix(['-', '+']).unwrap_or(exp);
        !digits.is_empty() && digits.chars().all(|character| character.is_ascii_digit())
    })
}

/// Whether `text` is one or more ASCII digits with at most one `.` separator.
fn is_numeric_mantissa(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }
    let mut seen_dot = false;
    let mut seen_digit = false;
    for character in text.chars() {
        match character {
            '0'..='9' => seen_digit = true,
            '.' if !seen_dot => seen_dot = true,
            _ => return false,
        }
    }
    seen_digit
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "pdf")]
    fn make_seg(text: &str, x: f32, y: f32, width: f32, height: f32) -> SegmentData {
        SegmentData {
            text: text.to_string(),
            x,
            y,
            width,
            height,
            font_size: height,
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            baseline_y: y,
            rotation_degrees: 0.0,
            assigned_role: None,
        }
    }

    #[cfg(feature = "pdf")]
    #[test]
    fn test_split_single_word() {
        let seg = make_seg("Hello", 100.0, 500.0, 50.0, 12.0);
        let words = split_segment_to_words(&seg, 800.0);
        assert_eq!(words.len(), 1);
        assert_eq!(words[0].text, "Hello");
        assert_eq!(words[0].left, 100);
    }

    #[cfg(feature = "pdf")]
    #[test]
    fn test_split_two_words() {
        let seg = make_seg("Col A", 100.0, 500.0, 100.0, 12.0);
        let words = split_segment_to_words(&seg, 800.0);
        assert_eq!(words.len(), 2);
        assert_eq!(words[0].text, "Col");
        assert_eq!(words[1].text, "A");
        assert_eq!(words[1].left, 180);
    }

    #[cfg(feature = "pdf")]
    #[test]
    fn test_split_empty_segment() {
        let seg = make_seg("   ", 100.0, 500.0, 50.0, 12.0);
        let words = split_segment_to_words(&seg, 800.0);
        assert!(words.is_empty());
    }

    #[cfg(feature = "pdf")]
    #[test]
    fn test_split_many_words() {
        let seg = make_seg("a b c d", 0.0, 0.0, 700.0, 12.0);
        let words = split_segment_to_words(&seg, 800.0);
        assert_eq!(words.len(), 4);
        assert_eq!(words[0].text, "a");
        assert_eq!(words[1].text, "b");
        assert_eq!(words[2].text, "c");
        assert_eq!(words[3].text, "d");
        assert!(words[1].left > words[0].left);
        assert!(words[2].left > words[1].left);
        assert!(words[3].left > words[2].left);
    }

    #[cfg(feature = "pdf")]
    #[test]
    fn test_split_y_coordinate_conversion() {
        let seg = make_seg("word", 100.0, 500.0, 50.0, 12.0);
        let words = split_segment_to_words(&seg, 800.0);
        assert_eq!(words[0].top, 288);
        assert_eq!(words[0].height, 12);
    }

    #[cfg(feature = "pdf")]
    #[test]
    fn test_segments_to_words_multiple() {
        let segs = vec![
            make_seg("Hello", 10.0, 700.0, 40.0, 12.0),
            make_seg("World", 55.0, 700.0, 40.0, 12.0),
        ];
        let words = segments_to_words(&segs, 800.0);
        assert_eq!(words.len(), 2);
        assert_eq!(words[0].text, "Hello");
        assert_eq!(words[1].text, "World");
    }

    #[cfg(feature = "pdf")]
    #[test]
    fn issue_1566_touching_table_segments_merge_into_one_word() {
        let seg_a = make_seg("2 per ketel, pri", 287.864, 331.641, 46.631, 8.999996);
        let mut seg_b = make_seg("js per meter", 334.564, 331.641, 41.161, 8.999996);
        seg_b.is_bold = true;

        let words = segments_to_words(&[seg_a, seg_b], 800.0);
        let texts: Vec<&str> = words.iter().map(|word| word.text.as_str()).collect();
        assert_eq!(texts, vec!["2", "per", "ketel,", "prijs", "per", "meter"]);
    }

    /// FINDING 1 (adversarial review): determines the exact gutter, in points at a 9pt
    /// font, at which `segments_are_touching` starts fusing two *different table columns*
    /// (a "100" cell followed by a "5" cell) rather than a genuine split word. The
    /// analytic threshold is `font_size * TOUCHING_SPAN_GAP_EM_RATIO` = `9.0 * 0.025` =
    /// 0.225pt (confirmed in f32 arithmetic separately). This test pins that boundary
    /// empirically through the real `segments_to_words` path, not just the predicate.
    #[cfg(feature = "pdf")]
    #[test]
    fn digit_column_fusion_boundary_for_9pt_font() {
        let font_size = 9.0_f32;
        let build = |gap: f32| {
            let seg_a = make_seg("100", 100.0, 500.0, 15.0, font_size);
            let seg_b = make_seg("5", 100.0 + 15.0 + gap, 500.0, 5.0, font_size);
            segments_to_words(&[seg_a, seg_b], 800.0)
        };

        // Just under the 0.225pt threshold: the guard still treats this as one
        // split word and fuses "100" + "5" into "1005".
        let texts: Vec<String> = build(0.20).into_iter().map(|w| w.text).collect();
        assert_eq!(
            texts,
            vec!["1005".to_string()],
            "expected fusion just below the 0.225pt threshold"
        );

        // At/just over the 0.225pt threshold: the two columns stay separate words.
        let texts: Vec<String> = build(0.25).into_iter().map(|w| w.text).collect();
        assert_eq!(
            texts,
            vec!["100".to_string(), "5".to_string()],
            "expected no fusion at/above the 0.225pt threshold"
        );
    }

    /// FINDING 1 (adversarial review), continued: is a sub-quarter-point gutter
    /// physically achievable in a real ruled table? A hairline rule stroke is
    /// commonly ~0.5pt and cell text needs a non-zero clearance from that rule on
    /// each side to avoid visually touching it (a documents-in-the-wild minimum is
    /// on the order of 0.5-1pt per side). This test uses a deliberately tight but
    /// still physically real gutter (1.5pt: a 0.5pt rule plus 0.5pt padding on each
    /// side) between two adjacent numeric-column segments and asserts they do NOT
    /// fuse — guarding the currently-unmodified behavior rather than a hypothetical.
    #[cfg(feature = "pdf")]
    #[test]
    fn ruled_table_realistic_gutter_does_not_fuse_adjacent_numeric_columns() {
        let seg_a = make_seg("100", 287.864, 331.641, 15.0, 9.0);
        let seg_b = make_seg("5", 287.864 + 15.0 + 1.5, 331.641, 5.0, 9.0);
        let words = segments_to_words(&[seg_a, seg_b], 800.0);
        let texts: Vec<&str> = words.iter().map(|word| word.text.as_str()).collect();
        assert_eq!(texts, vec!["100", "5"]);
    }

    #[test]
    fn test_post_process_rejects_prose_as_table() {
        let table = vec![
            vec![
                "Foreword".into(),
                "".into(),
                "".into(),
                "".into(),
                "".into(),
                "ISO 21111-10:2021(E)".into(),
                "".into(),
                "".into(),
            ],
            vec![
                "ISO".into(),
                "(the".into(),
                "International".into(),
                "Organization".into(),
                "for".into(),
                "Standardization)is".into(),
                "a".into(),
                "worldwide".into(),
            ],
            vec![
                "bodies".into(),
                "(ISO".into(),
                "member".into(),
                "bodies).The".into(),
                "work".into(),
                "of".into(),
                "preparing".into(),
                "International".into(),
            ],
            vec![
                "through".into(),
                "ISO".into(),
                "technical".into(),
                "committees.Each".into(),
                "member".into(),
                "body".into(),
                "interested".into(),
                "in".into(),
            ],
        ];
        let result = post_process_table(table, false, false);
        assert!(result.is_none(), "Prose-like table should be rejected");
    }

    #[test]
    fn test_post_process_accepts_real_table() {
        let table = vec![
            vec!["Name".into(), "Department".into(), "Annual Salary".into()],
            vec!["John Smith".into(), "Engineering Dept".into(), "$95,000".into()],
            vec!["Jane Doe".into(), "Marketing Team".into(), "$88,500".into()],
            vec!["Bob Johnson".into(), "Sales Division".into(), "$92,000".into()],
            vec!["Alice Williams".into(), "Human Resources".into(), "$85,000".into()],
        ];
        let result = post_process_table(table, false, false);
        assert!(result.is_some(), "Real table should be accepted");
    }

    #[test]
    fn dense_numeric_matrix_survives_anti_prose_guards() {
        let mut table = vec![
            (0..DENSE_NUMERIC_MIN_COLUMNS)
                .map(|col| format!("Column {col}"))
                .collect(),
        ];
        for row in 0..DENSE_NUMERIC_MIN_DATA_ROWS {
            table.push(
                (0..DENSE_NUMERIC_MIN_COLUMNS)
                    .map(|col| {
                        if col == 0 {
                            format!("{:03}", row)
                        } else {
                            "1.000".to_string()
                        }
                    })
                    .collect(),
            );
        }

        let processed = post_process_table(table, true, false).expect("dense numeric matrix should be retained");
        assert!(is_well_formed_table(&processed));
    }

    #[test]
    fn compact_numeric_boundary_does_not_bypass_anti_prose_guards() {
        for columns in [3, 5] {
            let mut table = vec![(0..columns).map(|col| format!("Column {col}")).collect()];
            table.extend((0..5).map(|_| vec!["1.000".to_string(); columns]));

            let accepted =
                post_process_table(table, true, false).is_some_and(|processed| is_well_formed_table(&processed));
            assert!(!accepted, "repetitive {columns}-column compact grid must be rejected");
        }
    }

    fn dense_grid_with_columns(columns: usize, rows: usize) -> Vec<Vec<String>> {
        let mut table = vec![(0..columns).map(|column| format!("Column {column}")).collect()];
        table.extend((0..rows).map(|row| (0..columns).map(|column| format!("{}.{column}", row + 1)).collect()));
        table
    }

    #[test]
    fn prunes_one_empty_header_interior_track_and_preserves_lone_text() {
        let mut table = dense_grid_with_columns(7, SPURIOUS_COLUMN_MIN_DATA_ROWS);
        table[0][3].clear();
        for row in table.iter_mut().skip(1) {
            row[3].clear();
        }
        *table.last_mut().expect("data row") = vec![
            "footer note".into(),
            "continues here".into(),
            "with text".into(),
            "sustained".into(),
            "near table".into(),
            "boundary words".into(),
            "end".into(),
        ];

        assert!(prune_spurious_interior_column(&mut table, true, None));
        assert_eq!(table[0].len(), 6);
        assert!(
            table
                .last()
                .expect("data row")
                .iter()
                .any(|cell| cell.contains("sustained"))
        );
    }

    #[test]
    fn preserves_legitimate_named_sparse_column() {
        let mut table = dense_grid_with_columns(7, SPURIOUS_COLUMN_MIN_DATA_ROWS);
        table[0][3] = "Optional flag".into();
        for row in table.iter_mut().skip(1) {
            row[3].clear();
        }
        table.last_mut().expect("data row")[3] = "Y".into();

        assert!(!prune_spurious_interior_column(&mut table, true, None));
        assert_eq!(table[0].len(), 7);
        assert_eq!(table[0][3], "Optional flag");
    }

    #[test]
    fn preserves_unnamed_sparse_column_populated_in_table_body() {
        let mut table = dense_grid_with_columns(7, SPURIOUS_COLUMN_MIN_DATA_ROWS);
        table[0][3].clear();
        for row in table.iter_mut().skip(1) {
            row[3].clear();
        }
        let middle = table.len() / 2;
        table[middle] = vec![
            "boundary note".into(),
            "continues here".into(),
            "with text".into(),
            "sustained".into(),
            "inside table".into(),
            "body words".into(),
            "end".into(),
        ];

        assert!(!prune_spurious_interior_column(&mut table, true, None));
        assert_eq!(table[0].len(), 7);
        assert_eq!(table[middle][3], "sustained");
    }

    #[test]
    fn preserves_multiple_sparse_interior_columns() {
        let mut table = dense_grid_with_columns(8, SPURIOUS_COLUMN_MIN_DATA_ROWS);
        for column in [2, 5] {
            table[0][column].clear();
            for row in table.iter_mut().skip(1) {
                row[column].clear();
            }
        }

        assert!(!prune_spurious_interior_column(&mut table, true, None));
        assert_eq!(table[0].len(), 8);
    }

    #[test]
    fn sparse_track_does_not_turn_prose_into_table() {
        let mut table = vec![vec![String::new(); 7]];
        table.extend((0..SPURIOUS_COLUMN_MIN_DATA_ROWS).map(|row| {
            vec![
                format!("section {row}"),
                format!("page {row}"),
                "quick".into(),
                String::new(),
                "brown".into(),
                "fox".into(),
                "continues".into(),
            ]
        }));

        let accepted = post_process_table(table, true, false).is_some_and(|processed| is_well_formed_table(&processed));
        assert!(!accepted);
    }

    #[test]
    fn repeated_row_shape_finds_three_numeric_fields_after_two_row_header() {
        let table = vec![
            vec![
                "Report 2024".into(),
                "Patient status".into(),
                "Metric 70".into(),
                "Treatment group".into(),
                "Metric 91".into(),
                "Final outcome".into(),
            ],
            vec![
                "".into(),
                "".into(),
                "(score)".into(),
                "".into(),
                "(years)".into(),
                "".into(),
            ],
            vec![
                "R1".into(),
                "active".into(),
                "4.36".into(),
                "A".into(),
                "52".into(),
                "SVR".into(),
            ],
            vec![
                "R2".into(),
                "active".into(),
                "6.37".into(),
                "B".into(),
                "35".into(),
                "SVR".into(),
            ],
            vec![
                "R3".into(),
                "active".into(),
                "7.84".into(),
                "A".into(),
                "46".into(),
                "SVR".into(),
            ],
        ];

        assert_eq!(find_data_start(&table, true), 2);
        assert_eq!(find_data_start(&table, false), 0);
    }

    #[test]
    fn issue_1558_surplus_inferred_header_rows_are_demoted_not_dropped() {
        let table: Vec<Vec<String>> = (1..=18)
            .map(|row| {
                vec![
                    format!("{row} 8000{row:02}"),
                    "Fastening screw".into(),
                    format!("{row},10"),
                    if row == 7 {
                        "package of 30".into()
                    } else {
                        "available".into()
                    },
                ]
            })
            .collect();

        assert_eq!(find_data_start(&table, true), 6);
        let processed = post_process_table(table, true, false).expect("dense parts table should remain valid");

        // Six inferred header rows become one merged header plus four demoted
        // data rows. No source row may disappear.
        assert_eq!(processed.len(), 17);
        let flattened = processed.iter().flatten().cloned().collect::<Vec<_>>().join(" ");
        for row in 1..=18 {
            let article = format!("8000{row:02}");
            assert_eq!(
                flattened.matches(&article).count(),
                1,
                "{article} must survive exactly once"
            );
        }
        assert!(
            processed[0]
                .iter()
                .any(|cell| cell.contains("800005") && cell.contains("800006"))
        );
        assert!(processed[1].iter().any(|cell| cell.contains("800001")));
        assert!(processed[4].iter().any(|cell| cell.contains("800004")));
        assert!(processed[5].iter().any(|cell| cell.contains("800007")));
    }

    #[test]
    fn categorical_subtotal_does_not_hide_leading_numeric_data_row() {
        let table = vec![
            vec![
                "R1".into(),
                "New York".into(),
                "4.36".into(),
                "needs review".into(),
                "52".into(),
                "SVR".into(),
            ],
            vec![
                "Subtotal for region".into(),
                "".into(),
                "".into(),
                "".into(),
                "".into(),
                "".into(),
            ],
            vec![
                "R2".into(),
                "active".into(),
                "6.37".into(),
                "B".into(),
                "35".into(),
                "SVR".into(),
            ],
            vec![
                "R3".into(),
                "active".into(),
                "7.84".into(),
                "A".into(),
                "46".into(),
                "SVR".into(),
            ],
            vec![
                "R4".into(),
                "active".into(),
                "5.12".into(),
                "B".into(),
                "41".into(),
                "SVR".into(),
            ],
        ];

        assert_eq!(find_data_start(&table, true), 0);
    }

    #[test]
    fn repeated_shape_does_not_skip_numeric_rows_without_header_gap() {
        let table = vec![
            vec!["1".into(), "2".into(), "3".into(), "".into(), "5".into(), "".into()],
            vec!["1".into(), "2".into(), "".into(), "4".into(), "5".into(), "".into()],
            vec!["1".into(), "2".into(), "3".into(), "4".into(), "".into(), "".into()],
            vec!["1".into(), "2".into(), "3".into(), "4".into(), "".into(), "".into()],
            vec!["1".into(), "2".into(), "3".into(), "4".into(), "".into(), "".into()],
        ];

        assert_eq!(find_data_start(&table, true), 0);
    }

    #[test]
    fn retains_large_scalar_table_with_numeric_multiline_header() {
        let mut table = vec![
            vec![
                "".into(),
                "".into(),
                "".into(),
                "".into(),
                "".into(),
                "".into(),
                "Core amino".into(),
                "acid".into(),
                "".into(),
                "".into(),
                "".into(),
            ],
            vec![
                "Patient".into(),
                "Genotype".into(),
                "Viral load".into(),
                "".into(),
                "Sex".into(),
                "Age".into(),
                "70".into(),
                "91".into(),
                "rs12979860".into(),
                "End of treatment".into(),
                "".into(),
            ],
            vec![
                "no".into(),
                "".into(),
                "(10 IU/ml) 6".into(),
                "".into(),
                "".into(),
                "(years)".into(),
                "".into(),
                "".into(),
                "".into(),
                "response".into(),
                "a".into(),
            ],
        ];
        for row in 1..=SPURIOUS_COLUMN_MIN_DATA_ROWS {
            table.push(vec![
                format!("R{row}"),
                "1a".into(),
                format!("{}.36", row + 3),
                String::new(),
                if row % 2 == 0 { "F".into() } else { "M".into() },
                format!("{}.6", row + 30),
                "R".into(),
                "C".into(),
                if row % 2 == 0 { "CT".into() } else { "CC".into() },
                "SVR".into(),
                String::new(),
            ]);
        }
        table.push(vec![
            "a SVR, sustained".into(),
            "virologic response;".into(),
            "non-SVR, no".into(),
            "sustained".into(),
            "virologic".into(),
            "response".into(),
            "".into(),
            "".into(),
            "".into(),
            "".into(),
            "".into(),
        ]);

        let processed = post_process_table(table, true, false).expect("large scalar table should be retained");
        assert_eq!(processed[0].len(), 9);
        assert!(processed[0][0].contains("Patient"));
        assert!(is_well_formed_table(&processed));
    }

    #[test]
    fn test_column_text_flow_rejects_multicolumn_prose() {
        let table = vec![
            vec!["Header Left".into(), "Header Right".into()],
            vec![
                "The results of this experiment show that the proposed method".into(),
                "significantly outperforms the baseline in all metrics tested".into(),
            ],
            vec![
                "across multiple datasets including the standard benchmark".into(),
                "suite commonly used in the literature for evaluation of".into(),
            ],
            vec![
                "natural language processing tasks and related problems".into(),
                "involving text classification and information extraction".into(),
            ],
            vec![
                "methods that rely on deep learning architectures with".into(),
                "attention mechanisms and transformer-based embeddings".into(),
            ],
        ];
        let result_unsupervised = post_process_table(table.clone(), false, false);
        assert!(
            result_unsupervised.is_none(),
            "Multi-column prose should be rejected in unsupervised mode"
        );
        let result_guided = post_process_table(table, true, false);
        assert!(
            result_guided.is_none(),
            "Multi-column prose should be rejected in layout-guided mode"
        );
    }

    #[test]
    fn test_column_text_flow_accepts_real_two_column_table() {
        let table = vec![
            vec!["Feature".into(), "Description".into()],
            vec!["Authentication.".into(), "OAuth 2.0 with JWT tokens.".into()],
            vec!["Rate Limiting.".into(), "100 requests per minute.".into()],
            vec!["Caching.".into(), "Redis-backed with TTL.".into()],
            vec!["Monitoring.".into(), "Prometheus metrics endpoint.".into()],
        ];
        let result = post_process_table(table, true, false);
        assert!(
            result.is_some(),
            "Real 2-column table with proper sentence endings should be accepted"
        );
    }

    #[test]
    fn test_column_text_flow_not_triggered_with_few_rows() {
        let table = vec![
            vec!["Left".into(), "Right".into()],
            vec![
                "some text without ending punct".into(),
                "continues here in lowercase".into(),
            ],
            vec!["another partial sentence".into(), "flowing into next column".into()],
        ];
        let _ = post_process_table(table, true, false);
    }

    #[test]
    fn test_layout_guided_rejects_prose_with_long_cells() {
        let long_cell = "a".repeat(120);
        let table = vec![
            vec!["Header A".into(), "Header B".into()],
            vec![long_cell.clone(), long_cell.clone()],
            vec![long_cell.clone(), long_cell.clone()],
            vec![long_cell.clone(), long_cell.clone()],
            vec![long_cell.clone(), long_cell.clone()],
        ];
        let result = post_process_table(table, true, false);
        assert!(
            result.is_none(),
            "Layout-guided should reject tables with overwhelmingly long cells"
        );
    }

    #[test]
    fn test_layout_guided_accepts_table_with_some_long_cells() {
        let table = vec![
            vec!["Feature Name".into(), "Description".into()],
            vec![
                "User Authentication Module".into(),
                "Handles login, logout, and session management for users.".into(),
            ],
            vec![
                "Rate Limiting Service".into(),
                "Controls API request rates per client and endpoint.".into(),
            ],
            vec!["Cache Layer".into(), "Short desc.".into()],
            vec![
                "Monitoring Dashboard".into(),
                "Displays real-time metrics and alerting configuration.".into(),
            ],
        ];
        let result = post_process_table(table, true, false);
        assert!(
            result.is_some(),
            "Layout-guided table with some long cells should be accepted"
        );
    }

    #[test]
    fn test_layout_guided_rejects_dominant_column() {
        let table = vec![
            vec!["Tag".into(), "Content".into()],
            vec!["x".into(), "This is a very long paragraph of text that contains almost all content in the table and dwarfs the tag column.".into()],
            vec!["y".into(), "Another massive block of text that makes the first column insignificant by comparison in terms of character count.".into()],
            vec!["z".into(), "Yet more extensive content that further skews the distribution of characters heavily toward this second column here.".into()],
        ];
        let result = post_process_table(table, true, false);
        assert!(
            result.is_none(),
            "Layout-guided should reject tables with >92% text in one column"
        );
    }

    #[test]
    fn test_layout_guided_single_word_prose_rejected() {
        let table = vec![
            vec!["A".into(), "B".into(), "C".into(), "D".into(), "E".into(), "F".into()],
            vec![
                "The".into(),
                "quick".into(),
                "brown".into(),
                "fox".into(),
                "jumps".into(),
                "over".into(),
            ],
            vec![
                "the".into(),
                "lazy".into(),
                "dog".into(),
                "and".into(),
                "runs".into(),
                "away".into(),
            ],
            vec![
                "from".into(),
                "the".into(),
                "big".into(),
                "bad".into(),
                "wolf".into(),
                "today".into(),
            ],
            vec![
                "who".into(),
                "was".into(),
                "very".into(),
                "mean".into(),
                "and".into(),
                "scary".into(),
            ],
            vec![
                "but".into(),
                "the".into(),
                "fox".into(),
                "was".into(),
                "too".into(),
                "fast".into(),
            ],
            vec![
                "for".into(),
                "the".into(),
                "wolf".into(),
                "to".into(),
                "ever".into(),
                "catch".into(),
            ],
        ];
        let result = post_process_table(table, true, false);
        assert!(
            result.is_none(),
            "Layout-guided should reject tables with >85% single-word cells"
        );
    }

    #[test]
    fn test_row_continuation_rejects_prose_flowing_across_rows() {
        let mut table = vec![vec!["Left Column".into(), "Right Column".into()]];
        let prose_pairs = vec![
            ("The experiment was conducted", "over several weeks and the"),
            ("results clearly demonstrate", "that the proposed method is"),
            ("superior to existing approaches", "because it leverages novel"),
            ("techniques developed in our", "laboratory during the past"),
            ("decade of intensive research", "on machine learning systems"),
        ];
        for (left, right) in prose_pairs {
            table.push(vec![left.into(), right.into()]);
        }
        let result = post_process_table(table.clone(), false, false);
        assert!(
            result.is_none(),
            "Row-continuation prose should be rejected in unsupervised mode"
        );
        let result_guided = post_process_table(table, true, false);
        assert!(
            result_guided.is_none(),
            "Row-continuation prose should be rejected in layout-guided mode"
        );
    }

    #[test]
    fn test_row_continuation_accepts_table_with_sentence_endings() {
        let table = vec![
            vec!["Parameter".into(), "Value".into()],
            vec!["Max connections.".into(), "100 per host.".into()],
            vec!["Timeout.".into(), "30 seconds.".into()],
            vec!["Retry policy.".into(), "Exponential backoff.".into()],
            vec!["Cache TTL.".into(), "3600 seconds.".into()],
            vec!["Rate limit.".into(), "1000 req/min.".into()],
        ];
        let result = post_process_table(table, true, false);
        assert!(
            result.is_some(),
            "Table with proper sentence endings should not be rejected by row-continuation check"
        );
    }

    #[test]
    fn test_high_row_low_column_rejects_prose() {
        let mut table = vec![vec!["Column A".into(), "Column B".into()]];
        for i in 0..25 {
            table.push(vec![
                format!("Content block {} left side text", i),
                format!("Content block {} right side text", i),
            ]);
        }
        let result = post_process_table(table.clone(), false, false);
        assert!(
            result.is_none(),
            "High-row low-column fully-filled table should be rejected (unsupervised)"
        );
        let result_guided = post_process_table(table, true, false);
        assert!(
            result_guided.is_none(),
            "High-row low-column fully-filled table should be rejected (layout-guided)"
        );
    }

    #[test]
    fn test_high_row_low_column_accepts_sparse_table() {
        let mut table = vec![vec!["Date".into(), "Event".into()]];
        for i in 0..25 {
            if i % 3 == 0 {
                table.push(vec![format!("2024-01-{:02}", i + 1), "Holiday.".into()]);
            } else {
                table.push(vec![format!("2024-01-{:02}", i + 1), String::new()]);
            }
        }
        let result = post_process_table(table, true, false);
        let _ = result;
    }

    #[test]
    fn test_high_row_low_column_allows_four_plus_columns() {
        let mut table = vec![vec!["ID".into(), "Name".into(), "Dept".into(), "Salary".into()]];
        for i in 0..25 {
            table.push(vec![
                format!("{}", i + 1),
                format!("Employee {}", i),
                "Engineering".into(),
                format!("${},000", 80 + i),
            ]);
        }
        let result = post_process_table(table, false, false);
        assert!(
            result.is_some(),
            "4-column table with many rows should not be rejected by high-row-low-column check"
        );
    }

    #[test]
    fn test_uniform_column_width_rejects_prose() {
        let mut table = vec![vec!["Col A".into(), "Col B".into(), "Col C".into()]];
        for _ in 0..8 {
            table.push(vec![
                "The quick brown fox jumps over".into(),
                "the lazy dog and runs through".into(),
                "the forest at remarkable speed".into(),
            ]);
        }
        let result = post_process_table(table.clone(), false, false);
        assert!(
            result.is_none(),
            "Uniform column width prose should be rejected (unsupervised)"
        );
        let result_guided = post_process_table(table, true, false);
        assert!(
            result_guided.is_none(),
            "Uniform column width prose should be rejected (layout-guided)"
        );
    }

    #[test]
    fn test_uniform_column_width_accepts_varied_columns() {
        let table = vec![
            vec!["ID".into(), "Product Name".into(), "Short Note".into()],
            vec![
                "1001".into(),
                "Industrial Premium Widget Alpha Series".into(),
                "High durability rating.".into(),
            ],
            vec![
                "1002".into(),
                "Advanced Sensor Gadget Beta Model".into(),
                "Wireless connectivity.".into(),
            ],
            vec![
                "1003".into(),
                "Professional Ergonomic Tool Gamma".into(),
                "Titanium blade.".into(),
            ],
            vec![
                "1004".into(),
                "Main Assembly Replacement Part Delta".into(),
                "Production line seven.".into(),
            ],
            vec![
                "1005".into(),
                "Standard Inventory Item Epsilon Unit".into(),
                "Daily operations use.".into(),
            ],
        ];
        let result = post_process_table(table, false, false);
        assert!(result.is_some(), "Table with varied column widths should be accepted");
    }

    #[test]
    fn test_well_formed_rejects_single_row() {
        let grid = vec![vec!["Header".into(), "Value".into()]];
        assert!(!is_well_formed_table(&grid), "Single-row grid should be rejected");
    }

    #[test]
    fn test_well_formed_rejects_single_column() {
        let grid = vec![vec!["Header".into()], vec!["Row 1".into()], vec!["Row 2".into()]];
        assert!(!is_well_formed_table(&grid), "Single-column grid should be rejected");
    }

    #[test]
    fn test_well_formed_accepts_real_table() {
        let grid = vec![
            vec!["Name".into(), "Department".into(), "Salary".into()],
            vec!["John Smith".into(), "Engineering".into(), "$95,000".into()],
            vec!["Jane Doe".into(), "Marketing".into(), "$88,500".into()],
            vec!["Bob Johnson".into(), "Sales".into(), "$92,000".into()],
            vec!["Alice Williams".into(), "HR".into(), "$85,000".into()],
        ];
        assert!(
            is_well_formed_table(&grid),
            "Real table with varied columns should be accepted"
        );
    }

    /// A genuine text-heavy key-value grid (#1319 invoice header) has regular,
    /// short column lengths, so the global uniform-column prose heuristic rejects
    /// it — but a geometrically pre-vetted caller passing
    /// `skip_columnar_prose_guard = true` must accept it while every other
    /// structural guard still applies.
    #[test]
    fn test_key_value_grid_gated_by_columnar_prose_guard_only() {
        let grid: Vec<Vec<String>> = vec![
            vec![
                "EXAMPLE COMPANY".into(),
                "Customer number".into(),
                "CUST-86241057".into(),
            ],
            vec![
                "Attn. SYNTH RECIPIENT".into(),
                "Invoice number".into(),
                "INV-709381624".into(),
            ],
            vec!["SAMPLE ROAD 14".into(), "Invoice date".into(), "15 January 2030".into()],
            vec!["45123 DEMO CITY".into(), "Order number".into(), "ORDER-58260419".into()],
            vec!["SYNTH COUNTRY".into(), "Order date".into(), "15 January 2030".into()],
            vec![
                "Tax ID SYNTH-TAX-918274635".into(),
                "Delivery date".into(),
                "15 January 2030".into(),
            ],
        ];
        assert!(
            !is_well_formed_table_core(&grid, false),
            "uniform-column prose heuristic rejects the key-value grid without the skip"
        );
        assert!(
            is_well_formed_table_core(&grid, true),
            "pre-vetted key-value grid must pass every other structural guard"
        );
    }

    #[test]
    fn test_well_formed_rejects_sparse_form_grid() {
        let grid: Vec<Vec<String>> = vec![
            vec!["".into(), "Tender".into(), "No.".into(), "".into()],
            vec!["41(01)/2019/PROM".into(), "".into(), "".into(), "".into()],
            vec!["Dated:".into(), "".into(), "11/09/2020".into(), "".into()],
            vec!["CPP".into(), "Portal".into(), "Tender".into(), "ID:".into()],
            vec!["2020_TBI_582964_1".into(), "".into(), "".into(), "".into()],
        ];
        assert!(
            !is_well_formed_table(&grid),
            "Sparse form-like grid (>40% empty cells) should be rejected"
        );
    }

    #[test]
    fn test_well_formed_rejects_repetitive_content() {
        let grid = vec![
            vec!["Bookmark".into(), "File PDF".into(), "Year 4".into()],
            vec!["Bookmark".into(), "File PDF".into(), "Year 4".into()],
            vec!["Bookmark".into(), "File PDF".into(), "Year 4".into()],
            vec!["Bookmark".into(), "File PDF".into(), "Year 4".into()],
            vec!["Bookmark".into(), "File PDF".into(), "Year 4".into()],
        ];
        assert!(
            !is_well_formed_table(&grid),
            "Repetitive content (same words every row) should be rejected"
        );
    }

    #[test]
    fn test_well_formed_rejects_repeated_header_in_data() {
        let grid = vec![
            vec!["Title".into(), "Author".into(), "Page".into()],
            vec!["Chapter 1".into(), "Smith".into(), "10".into()],
            vec!["Title".into(), "Author".into(), "Page".into()],
            vec!["Chapter 2".into(), "Doe".into(), "25".into()],
            vec!["Title".into(), "Author".into(), "Page".into()],
        ];
        assert!(
            !is_well_formed_table(&grid),
            "Table with header repeated in data rows should be rejected"
        );
    }

    #[test]
    fn test_well_formed_rejects_prose_rows() {
        let grid = vec![
            vec!["Column A".into(), "Column B".into(), "Column C".into()],
            vec![
                "The experiment was conducted over".into(),
                "several weeks and the results clearly".into(),
                "demonstrate that the proposed method is".into(),
            ],
            vec![
                "superior to existing approaches because".into(),
                "it leverages novel techniques developed".into(),
                "in our laboratory during the past decade".into(),
            ],
            vec![
                "of intensive research on machine learning".into(),
                "systems and their applications to natural".into(),
                "language processing and text extraction".into(),
            ],
            vec![
                "from documents in various formats including".into(),
                "portable document format and hypertext markup".into(),
                "language as well as office document formats".into(),
            ],
        ];
        assert!(
            !is_well_formed_table(&grid),
            "Multi-column prose should be rejected by row coherence check"
        );
    }

    #[test]
    fn test_well_formed_rejects_uniform_columns() {
        let grid = vec![
            vec!["Col A".into(), "Col B".into(), "Col C".into()],
            vec!["twelve chars".into(), "twelve char2".into(), "twelve char3".into()],
            vec!["twelve char4".into(), "twelve char5".into(), "twelve char6".into()],
            vec!["twelve char7".into(), "twelve char8".into(), "twelve char9".into()],
            vec!["twelve charA".into(), "twelve charB".into(), "twelve charC".into()],
        ];
        assert!(
            !is_well_formed_table(&grid),
            "Table with uniform column widths and low variance should be rejected"
        );
    }

    #[test]
    fn test_well_formed_accepts_varied_columns() {
        let grid = vec![
            vec!["ID".into(), "Product Name".into(), "Price".into()],
            vec!["1".into(), "Widget Alpha Premium".into(), "$29.99".into()],
            vec!["2".into(), "Gadget Beta Standard".into(), "$149.50".into()],
            vec!["3".into(), "Tool Gamma Deluxe Ed".into(), "$7.25".into()],
            vec!["4".into(), "Part Delta Industrial".into(), "$1,299.00".into()],
        ];
        assert!(
            is_well_formed_table(&grid),
            "Table with varied column types should be accepted"
        );
    }

    #[test]
    fn test_well_formed_rejects_multicolumn_prose_short_cells() {
        let grid = vec![
            vec!["Bookmark".into(), "File PDF".into(), "Year 4".into()],
            vec!["Numeracy".into(), "Essment".into(), "Test".into()],
            vec![
                "Papers is universally".into(),
                "And Answers compatible".into(),
                "with any".into(),
            ],
            vec!["devices".into(), "to read".into(), "".into()],
            vec!["Year 4 Maths".into(), "Lesson".into(), "Uk The".into()],
            vec!["Maths Guy".into(), "ninety fail".into(), "Can you".into()],
            vec!["pass a GRADE".into(), "four Math".into(), "Test here".into()],
            vec!["Quick Learnerz".into(), "Year".into(), "four Termly".into()],
            vec!["Maths Assessment".into(), "Can".into(), "You Pass".into()],
            vec!["".into(), "Page five".into(), "".into()],
        ];
        assert!(
            !is_well_formed_table(&grid),
            "3-column prose with short cells (nougat_008 pattern) should be rejected"
        );
    }

    #[test]
    fn test_well_formed_rejects_two_row_columned_prose() {
        let grid = vec![
            vec!["Column A".into(), "Column B".into(), "Column C".into()],
            vec![
                "The experiment was conducted over".into(),
                "several weeks and the results clearly".into(),
                "demonstrate that the proposed method is".into(),
            ],
            vec![
                "superior to existing approaches because".into(),
                "it leverages novel techniques developed".into(),
                "in our laboratory during the past decade".into(),
            ],
        ];
        assert!(
            !is_well_formed_table(&grid),
            "Two-row column-aligned prose should be demoted (issue #36)"
        );
    }

    #[test]
    fn test_well_formed_rejects_two_col_two_row_prose() {
        let grid = vec![
            vec!["Column A".into(), "Column B".into()],
            vec![
                "The experiment was conducted over".into(),
                "several weeks and the results clearly".into(),
            ],
            vec![
                "demonstrate that the proposed method".into(),
                "is superior to existing approaches here".into(),
            ],
        ];
        assert!(
            !is_well_formed_table(&grid),
            "Two-column, two-row prose should be demoted (issue #36)"
        );
    }

    #[test]
    fn test_well_formed_rejects_single_data_row_prose() {
        let grid = vec![
            vec!["Column A".into(), "Column B".into(), "Column C".into()],
            vec![
                "The experiment was conducted over".into(),
                "several weeks and the results clearly".into(),
                "demonstrate that the proposed method is".into(),
            ],
        ];
        assert!(
            !is_well_formed_table(&grid),
            "Single-data-row column-aligned prose should be demoted (issue #36)"
        );
    }

    #[test]
    fn test_well_formed_rejects_five_col_short_prose() {
        let grid = vec![
            vec!["A".into(), "B".into(), "C".into(), "D".into(), "E".into()],
            vec![
                "conducted over several weeks".into(),
                "and the results clearly show".into(),
                "that the proposed method here".into(),
                "is superior to existing work".into(),
                "because of novel techniques used".into(),
            ],
            vec![
                "developed in our laboratory over".into(),
                "the past decade of intensive".into(),
                "research on machine learning here".into(),
                "and its applications to natural".into(),
                "language processing of documents".into(),
            ],
        ];
        assert!(
            !is_well_formed_table(&grid),
            "Five-column short prose (upper boundary) should be demoted (issue #36)"
        );
    }

    #[test]
    fn test_sparse_continuation_row_keeps_numeric_line_item_table() {
        // A five-column borderless line-item table whose second row is a wrapped
        // description continuation (trailing columns blank). Pooled over all
        // cells the numeric fraction is 4/8 = 50%, below the 60% bar, but the
        // complete item row alone is 4/5 = 80% numeric. The continuation row
        // must not erase the table (issue #1333).
        let table = vec![
            vec![
                "Item".into(),
                "Qty".into(),
                "Price".into(),
                "VAT".into(),
                "Total".into(),
            ],
            vec![
                "SYNTH PRODUCT".into(),
                "1".into(),
                "120.40".into(),
                "19%".into(),
                "120.40".into(),
            ],
            vec![
                "WITH FEE".into(),
                "each".into(),
                "$".into(),
                String::new(),
                String::new(),
            ],
        ];
        let result = post_process_table(table.clone(), true, false);
        assert!(
            result.is_some(),
            "Numeric line-item table with a sparse continuation row must survive (layout-guided)"
        );
        let result_unsupervised = post_process_table(table, false, false);
        assert!(
            result_unsupervised.is_some(),
            "Numeric line-item table with a sparse continuation row must survive (unsupervised)"
        );
    }

    #[test]
    fn test_inferred_columns_keep_sparse_numeric_line_item_table() {
        // The visible table has five columns, but reconstruction can infer two
        // extra tracks. The principal row still supplies 6/7 occupied cells and
        // four numeric values; the sparse fee row must not dilute that evidence.
        let table = vec![
            vec![
                "Item".into(),
                "Quantity".into(),
                "Price".into(),
                "VAT".into(),
                "Total".into(),
                String::new(),
                String::new(),
            ],
            vec![
                "SYNTH PRODUCT".into(),
                "1".into(),
                "120.40".into(),
                "19%".into(),
                "120.40".into(),
                "split".into(),
                String::new(),
            ],
            vec![
                "INCLUDING SYNTHETIC DEVICE FEE".into(),
                "1".into(),
                "3.40".into(),
                String::new(),
                String::new(),
                "split".into(),
                "tail".into(),
            ],
        ];

        assert!(
            post_process_table(table.clone(), true, false).is_some(),
            "numeric line-item table must survive a small inferred-column overrun"
        );
        assert!(
            post_process_table(table, false, false).is_some(),
            "the inferred-column recovery must not depend on layout guidance"
        );
    }

    #[test]
    fn test_sparse_numeric_prose_does_not_bypass_short_grid_guard() {
        let table = vec![
            vec![
                "A".into(),
                "B".into(),
                "C".into(),
                "D".into(),
                "E".into(),
                "F".into(),
                "G".into(),
            ],
            vec![
                "alpha".into(),
                "1".into(),
                "2".into(),
                "3".into(),
                String::new(),
                String::new(),
                String::new(),
            ],
            vec![
                "beta".into(),
                String::new(),
                String::new(),
                String::new(),
                "x".into(),
                "4".into(),
                "tail".into(),
            ],
        ];

        assert!(
            post_process_table(table.clone(), true, false).is_none(),
            "sparse numeric prose must remain rejected when no row fills 85% of the inferred grid"
        );
        assert!(
            post_process_table(table, false, false).is_none(),
            "the issue #36 guard must remain active without layout guidance"
        );
    }

    #[test]
    fn test_well_formed_rejects_short_wide_sparse_contact_block() {
        let grid = vec![
            vec![
                String::new(),
                "30B5".into(),
                "Stevenson".into(),
                "Drive·".into(),
                "Suite".into(),
                "301. Springfield,".into(),
                String::new(),
                "IL 62703".into(),
            ],
            vec![
                "Telephone".into(),
                String::new(),
                "(217)".into(),
                "585-2370'".into(),
                "(888)".into(),
                "547-8473·".into(),
                "Fax (217)".into(),
                "585-2372".into(),
            ],
            vec![
                String::new(),
                "a-mail:".into(),
                String::new(),
                "suaa@suaa.org·website:WWN.su88.oro".into(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ],
        ];

        assert!(
            !is_well_formed_table(&grid),
            "a sparse three-line contact block must not be promoted to a table"
        );
    }

    #[test]
    fn test_well_formed_keeps_two_row_numeric_table() {
        let grid = vec![
            vec!["Q1".into(), "Q2".into(), "Q3".into()],
            vec!["12".into(), "8".into(), "20".into()],
            vec!["15".into(), "9".into(), "24".into()],
        ];
        assert!(
            is_well_formed_table(&grid),
            "Two-row numeric table must survive the short-prose guard"
        );
    }

    #[test]
    fn test_well_formed_keeps_key_value_numeric() {
        let grid = vec![
            vec!["Metric".into(), "Value".into()],
            vec!["Total".into(), "$1,299.00".into()],
        ];
        assert!(
            is_well_formed_table(&grid),
            "Key/value pair with a numeric value must survive the short-prose guard"
        );
    }

    #[test]
    fn test_well_formed_keeps_unit_rows() {
        let grid = vec![
            vec!["Property".into(), "Measurement".into()],
            vec!["Length".into(), "45 mm".into()],
            vec!["Voltage".into(), "3.3 V".into()],
        ];
        assert!(
            is_well_formed_table(&grid),
            "Unit-bearing rows must survive the short-prose guard (digit-bearing exemption)"
        );
    }

    #[test]
    fn test_well_formed_keeps_short_label_key_value() {
        let grid = vec![
            vec!["Field".into(), "Entry".into()],
            vec!["Status".into(), "Active".into()],
            vec!["Country".into(), "France".into()],
        ];
        assert!(
            is_well_formed_table(&grid),
            "Short-label key/value (< 4 words/cell) must survive the short-prose guard"
        );
    }

    #[test]
    fn test_well_formed_rejects_wide_two_row_shredded_prose() {
        let grid = vec![
            vec!["A".into(), "B".into(), "C".into(), "D".into(), "E".into(), "F".into()],
            vec![
                "the above equation by".into(),
                "the factor applied to".into(),
                "the initial density field".into(),
                "yields a cloud radius".into(),
                "of roughly ten to the".into(),
                "seventeen centimeters here".into(),
            ],
            vec![
                "which is approximately equal".into(),
                "to point zero three parsec".into(),
                "measured for all of the".into(),
                "models considered throughout".into(),
                "the present numerical study".into(),
                "of collapsing molecular clouds".into(),
            ],
        ];
        assert!(
            !is_well_formed_table(&grid),
            "Wide two-row phrase-per-cell prose should be demoted (issue #36, wide variant)"
        );
    }

    #[test]
    fn test_well_formed_rejects_wide_shredded_prose_with_incidental_numbers() {
        let grid = vec![
            vec![
                "oblate range".into(),
                "clouds".into(),
                "have are used".into(),
                "ra = to add".into(),
                "rb = noise".into(),
                "R and to".into(),
                "rc = initial".into(),
                "density".into(),
                "R . Random".into(),
                "distributions".into(),
                "numbers".into(),
                "by multiplying".into(),
                "( x, y,".into(),
                "z )) in".into(),
                "the from".into(),
            ],
            vec![
                "the".into(),
                "above".into(),
                "equation".into(),
                "by".into(),
                "the factor".into(),
                "0 . 1 ran".into(),
                "( x, y,".into(),
                "z )].".into(),
                "The".into(),
                "cloud radius".into(),
                "is".into(),
                "R =".into(),
                "1 . 0".into(),
                "10 17".into(),
                "cm".into(),
            ],
        ];
        assert!(
            !is_well_formed_table(&grid),
            "Wide shredded prose with incidental numbers should be demoted (issue #36)"
        );
    }

    #[test]
    fn test_well_formed_keeps_wide_short_value_grid() {
        let grid = vec![
            vec![
                "Q1".into(),
                "Q2".into(),
                "Q3".into(),
                "Q4".into(),
                "FY".into(),
                "YoY".into(),
            ],
            vec![
                "12".into(),
                "8".into(),
                "20".into(),
                "15".into(),
                "55".into(),
                "+4%".into(),
            ],
            vec![
                "14".into(),
                "9".into(),
                "22".into(),
                "17".into(),
                "62".into(),
                "+7%".into(),
            ],
        ];
        assert!(
            is_well_formed_table(&grid),
            "Wide numeric short-value grid must survive the widened short-prose guard"
        );
    }

    #[test]
    fn test_looks_like_short_columned_prose_signal() {
        let prose = vec![
            vec![
                "The experiment was conducted over".into(),
                "several weeks and the results clearly".into(),
                "demonstrate that the proposed method is".into(),
            ],
            vec![
                "superior to existing approaches because".into(),
                "it leverages novel techniques developed".into(),
                "in our laboratory during the past decade".into(),
            ],
        ];
        assert!(
            looks_like_short_columned_prose(&prose, 3),
            "phrase-per-cell prose rows read as prose"
        );

        let numeric = vec![vec!["12".into(), "8".into(), "20".into()]];
        assert!(!looks_like_short_columned_prose(&numeric, 3), "numeric rows are exempt");

        let short_labels = vec![vec!["Status".into(), "Active".into()]];
        assert!(
            !looks_like_short_columned_prose(&short_labels, 2),
            "short-label rows (< 4 words/cell) are not prose"
        );
    }

    #[test]
    fn declaration_shaped_code_grids_are_rejected() {
        let fill_string = vec![
            vec!["void FillString(".into(), "".into()],
            vec!["TCHAR*".into(), "buf,".into()],
            vec!["size_t".into(), "cchBuf,".into()],
        ];
        let get_file_version = vec![
            vec!["BOOL GetFileVersion(".into(), "".into(), "".into()],
            vec!["LPCWSTR".into(), "lpsFile,".into(), "".into()],
            vec!["__out".into(), "FILE_VERSION".into(), "*pVersion);".into()],
        ];
        let encode_stream = vec![
            vec!["size_t EncodeStream(".into(), "".into(), "".into()],
            vec!["__in".into(), "HANDLE".into(), "hStream,".into()],
            vec!["__inout".into(), "STREAM".into(), "*pStream);".into()],
        ];

        for grid in [&fill_string, &get_file_version, &encode_stream] {
            assert!(looks_like_code_listing(grid));
        }
    }

    #[test]
    fn api_reference_grid_with_code_punctuation_is_not_rejected() {
        let grid = vec![
            vec!["Function".into(), "Signature".into(), "Description".into()],
            vec![
                "allocate()".into(),
                "void* allocate(size_t);".into(),
                "Allocates a buffer, or returns null".into(),
            ],
            vec![
                "release(ptr)".into(),
                "void release(void*);".into(),
                "Releases the supplied buffer".into(),
            ],
        ];

        assert!(!looks_like_code_listing(&grid));
    }

    #[test]
    fn merged_api_title_and_parameter_descriptions_are_not_rejected() {
        let grid = vec![
            vec!["Function Parameters (".into(), "".into(), "".into()],
            vec!["Type".into(), "Name".into(), "Description".into()],
            vec![
                "char *".into(),
                "buffer".into(),
                "Destination pointer, must be writable".into(),
            ],
            vec!["size_t".into(), "length".into(), "Bytes, excluding terminator;".into()],
        ];

        assert!(!looks_like_code_listing(&grid));
    }

    #[test]
    fn required_field_pointer_footnote_is_not_rejected() {
        let grid = vec![
            vec!["Required Fields (".into(), "".into()],
            vec!["Name*".into(), "Primary contact,".into()],
            vec!["Owner".into(), "Responsible team,".into()],
            vec!["".into(), "* Required field".into()],
        ];

        assert!(!looks_like_code_listing(&grid));
    }

    #[test]
    fn post_processed_declaration_grid_is_rejected_as_code() {
        let grid = vec![
            vec!["BOOL GetFileVersion(".into(), "".into(), "".into()],
            vec!["LPCWSTR".into(), "lpsFile,".into(), "".into()],
            vec!["__out".into(), "FILE_VERSION".into(), "*pVersion);".into()],
        ];
        let cleaned = post_process_table(grid, true, false).expect("declaration grid should survive table cleanup");

        assert!(looks_like_code_listing(&cleaned));
    }

    #[test]
    fn numeric_grid_is_not_rejected_as_code() {
        let grid = vec![
            vec!["Year".into(), "Revenue".into(), "Margin".into()],
            vec!["2024".into(), "1,250".into(), "18.5%".into()],
            vec!["2025".into(), "1,420".into(), "20.1%".into()],
        ];

        assert!(!looks_like_code_listing(&grid));
    }

    /// Regression test for xberg-io/xberg#1301 (mode b): a colon-introduced,
    /// semicolon-delimited 2-item list whose clauses were word-per-cell
    /// reconstructed into a 10-column, 2-data-row grid. The existing
    /// row-coherence guards all require >= 3 or >= 4 data rows and never fire
    /// on this shape; `looks_like_shredded_prose_row` must reject it directly.
    #[test]
    fn short_word_shredded_prose_grid_is_rejected() {
        let grid = vec![
            vec![
                "to exclude".into(),
                "fractional".into(),
                "amounts".into(),
                "from the".into(),
                "shareholders'".into(),
                "".into(),
                "subscription".into(),
                "right;".into(),
                "".into(),
                "".into(),
            ],
            vec![
                "where".into(),
                "the new shares".into(),
                "are issued".into(),
                "against".into(),
                "cash".into(),
                "contributions".into(),
                "".into(),
                "at market price;".into(),
                "".into(),
                "".into(),
            ],
            vec![
                "where".into(),
                "the capital".into(),
                "is increased".into(),
                "against".into(),
                "contributions".into(),
                "".into(),
                "in kind".into(),
                "for the purpose".into(),
                "of merging".into(),
                "companies;".into(),
            ],
        ];

        assert!(
            !is_well_formed_table(&grid),
            "short word-shredded prose run must be rejected as a table"
        );
    }

    /// A single short-row prose fragment (1 data row) must also be caught —
    /// the guard must not require >= 2 data rows either.
    #[test]
    fn single_row_word_shredded_prose_grid_is_rejected() {
        let grid = vec![
            vec![
                "to exclude".into(),
                "fractional".into(),
                "amounts".into(),
                "from the".into(),
                "shareholders'".into(),
                "".into(),
                "subscription".into(),
                "right;".into(),
                "".into(),
                "".into(),
            ],
            vec![
                "where".into(),
                "the new shares".into(),
                "are issued".into(),
                "against".into(),
                "cash".into(),
                "contributions".into(),
                "".into(),
                "at market price;".into(),
                "".into(),
                "".into(),
            ],
        ];

        assert!(!is_well_formed_table(&grid));
    }

    /// A short, wide, but genuinely tabular grid (sparse per-row fill, no
    /// clause-terminal punctuation) must survive: the guard is scoped to
    /// dense, sentence-shaped rows, not merely "few rows and many columns".
    #[test]
    fn short_wide_sparse_numeric_grid_is_not_rejected_as_shredded_prose() {
        let grid = vec![
            vec![
                "NAME".into(),
                "ADDRESS".into(),
                "PCT".into(),
                "CLASS".into(),
                "COMMIT".into(),
                "TOTAL".into(),
            ],
            vec![
                "Northern Pension Trust".into(),
                "1 Lake Road, Zurich".into(),
                "15.20%".into(),
                "Limited Partner".into(),
                "45,040,000.00".into(),
                "45,233,052.00".into(),
            ],
        ];

        assert!(
            is_well_formed_table(&grid),
            "a real numeric/name table row must not be mistaken for shredded prose"
        );
        assert!(!looks_like_shredded_prose_row(&grid[1], grid[0].len()));
    }

    /// Single-word `HocrWord` at a given position, for the #1399 geometry tests.
    fn geometry_word(text: &str, left: u32, top: u32, width: u32) -> HocrWord {
        HocrWord {
            text: text.to_string(),
            left,
            top,
            width,
            height: 20,
            confidence: 95.0,
        }
    }

    /// GH#1399: prose whose lines run across the inferred column boundaries on
    /// every row must score as almost entirely straddled. Three rows of words
    /// each spanning both boundaries — the shape the reported page has.
    #[test]
    fn straddled_ratio_is_near_total_for_prose_running_across_every_boundary() {
        let mut region = Vec::new();
        for (row, top) in [0u32, 40, 80].iter().enumerate() {
            // Each word starts inside one column and ends inside the next.
            region.push(geometry_word(&format!("a{row}"), 0, *top, 130));
            region.push(geometry_word(&format!("b{row}"), 140, *top, 130));
        }
        let columns = vec![0u32, 100, 200];

        let ratio = straddled_boundary_ratio(&region, &columns);
        assert_eq!(
            ratio, 1.0,
            "every boundary is crossed on every row, so the ratio must be exactly 1.0; got {ratio}"
        );
        assert!(
            !is_well_formed_borderless_table(
                &[
                    vec!["a0".to_string(), "b0".to_string(), String::new()],
                    vec!["a1".to_string(), "b1".to_string(), String::new()],
                ],
                &region,
                &columns,
                0,
            ),
            "a rule-less region straddled on every row must be rejected as prose"
        );
    }

    /// The false-positive guard. A legitimate table whose first column holds
    /// one long word must NOT be rejected. An earlier attempt at this gate put
    /// the boundary at the midpoint between two column *medians* and used
    /// `any()` rather than a per-row proportion, so this exact shape — one wide
    /// word, every other cell clean — was misread as bridging.
    #[test]
    fn long_word_in_a_wide_column_does_not_read_as_a_straddled_boundary() {
        let region = vec![
            geometry_word("Department", 0, 0, 80),
            geometry_word("Head", 200, 0, 30),
            geometry_word("Telecommunications", 0, 40, 150),
            geometry_word("Alice", 200, 40, 40),
            geometry_word("Finance", 0, 80, 60),
            geometry_word("Bob", 200, 80, 30),
        ];
        let columns = vec![0u32, 200];

        let ratio = straddled_boundary_ratio(&region, &columns);
        assert_eq!(
            ratio, 0.0,
            "no word reaches column 1's start at x=200, so nothing straddles; got {ratio}"
        );
    }

    /// Signal 1 outranks Signal 2: a producer that drew ruling lines gets the
    /// benefit of the doubt even when the geometry looks prose-like, because
    /// the geometric signal alone is too weak to overrule drawn structure.
    #[test]
    fn drawn_ruling_lines_admit_a_region_the_geometric_gate_would_reject() {
        let mut region = Vec::new();
        for (row, top) in [0u32, 40, 80].iter().enumerate() {
            region.push(geometry_word(&format!("a{row}"), 0, *top, 130));
            region.push(geometry_word(&format!("b{row}"), 140, *top, 130));
        }
        let columns = vec![0u32, 100, 200];
        let grid = vec![
            vec!["Name".to_string(), "Role".to_string()],
            vec!["Alice".to_string(), "Engineer".to_string()],
            vec!["Bob".to_string(), "Designer".to_string()],
        ];

        assert_eq!(
            straddled_boundary_ratio(&region, &columns),
            1.0,
            "precondition: this geometry is fully straddled"
        );
        assert!(
            is_well_formed_borderless_table(&grid, &region, &columns, 3),
            "3 horizontal rules must admit the candidate despite the straddled geometry"
        );
        assert!(
            !is_well_formed_borderless_table(&grid, &region, &columns, 0),
            "the same candidate with no rules must fall through to the geometric gate and be rejected"
        );
    }

    /// Fewer than two detected columns means there is no boundary to straddle.
    #[test]
    fn straddled_ratio_is_zero_when_there_is_no_column_boundary() {
        let region = vec![geometry_word("only", 0, 0, 50)];
        assert_eq!(straddled_boundary_ratio(&region, &[0]), 0.0);
        assert_eq!(straddled_boundary_ratio(&region, &[]), 0.0);
        assert_eq!(straddled_boundary_ratio(&[], &[0, 100]), 0.0);
    }

    #[test]
    fn test_is_list_marker_cell_matches_the_three_shapes() {
        assert!(is_list_marker_cell("1."));
        assert!(is_list_marker_cell("12)"));
        assert!(is_list_marker_cell("a."));
        assert!(is_list_marker_cell("b)"));
        assert!(is_list_marker_cell("•"));
        assert!(is_list_marker_cell("-"));
        assert!(is_list_marker_cell("–"));
        assert!(is_list_marker_cell("*"));
    }

    #[test]
    fn test_is_list_marker_cell_rejects_non_marker_shapes() {
        assert!(
            !is_list_marker_cell("1"),
            "a bare digit run with no trailing punctuation is not a marker"
        );
        assert!(
            !is_list_marker_cell("ab."),
            "multi-letter prefix is not a single-letter ordinal"
        );
        assert!(
            !is_list_marker_cell("A."),
            "spec covers lowercase ordinals only, not uppercase"
        );
        assert!(!is_list_marker_cell("Feature"));
        assert!(!is_list_marker_cell(""));
        assert!(!is_list_marker_cell("10"));
        assert!(
            !is_list_marker_cell("$4.25"),
            "currency is not a marker even though it contains digits and a dot"
        );
    }

    /// Word-geometry fixture for a one-page scanned PDF: a title, a heading, and a
    /// four-item numbered list, laid out the way the reported #1570 page actually OCRs —
    /// each list line's words fall into three x-clusters (marker / early phrase / late
    /// phrase) separated by gaps well above `CELL_MERGE_GAP_HEIGHT_RATIO * median height`,
    /// so `merge_words_into_cell_tokens` collapses each line into ~3 dense tokens and
    /// `detect_columns` mints exactly 3 columns from them — reproducing the "spurious
    /// 3-column table" the issue describes. Built directly with `HocrWord`/geometry and
    /// run through the real `cluster_words_into_table_regions` / `reconstruct_table` /
    /// `post_process_table` pipeline, matching the Tesseract route's own call shape
    /// (`ocr::processor::execution`, `table_column_threshold: 50`,
    /// `table_row_threshold_ratio: 0.5`, `post_process_table(table, false, false)`). ~keep
    #[cfg(feature = "ocr")]
    fn numbered_list_page_words() -> Vec<HocrWord> {
        fn hocr_word(text: &str, left: u32, top: u32, width: u32, height: u32) -> HocrWord {
            HocrWord {
                text: text.to_string(),
                left,
                top,
                width,
                height,
                confidence: 95.0,
            }
        }

        vec![
            // Title line (isolated by a large vertical gap from everything below it).
            hocr_word("Engine", 100, 88, 60, 24),
            hocr_word("Oil", 165, 88, 30, 24),
            hocr_word("Change", 200, 88, 65, 24),
            hocr_word("Procedure", 270, 88, 85, 24),
            // Heading line (isolated the same way).
            hocr_word("Required", 100, 288, 75, 24),
            hocr_word("Steps", 180, 288, 45, 24),
            // "1. Drain old oil from engine"
            hocr_word("1.", 100, 488, 20, 24),
            hocr_word("Drain", 180, 488, 50, 24),
            hocr_word("old", 234, 488, 30, 24),
            hocr_word("oil", 400, 488, 25, 24),
            hocr_word("from", 429, 488, 35, 24),
            hocr_word("engine", 468, 488, 55, 24),
            // "2. Replace oil filter"
            hocr_word("2.", 100, 528, 20, 24),
            hocr_word("Replace", 180, 528, 65, 24),
            hocr_word("oil", 400, 528, 25, 24),
            hocr_word("filter", 429, 528, 45, 24),
            // "3. Add 5.5 quarts of synthetic 5W-30 oil"
            hocr_word("3.", 100, 568, 20, 24),
            hocr_word("Add", 180, 568, 35, 24),
            hocr_word("5.5", 219, 568, 30, 24),
            hocr_word("quarts", 400, 568, 55, 24),
            hocr_word("of", 459, 568, 20, 24),
            hocr_word("synthetic", 483, 568, 80, 24),
            hocr_word("5W-30", 567, 568, 50, 24),
            hocr_word("oil", 621, 568, 25, 24),
            // "4. Check oil level with dipstick"
            hocr_word("4.", 100, 608, 20, 24),
            hocr_word("Check", 180, 608, 50, 24),
            hocr_word("oil", 234, 608, 25, 24),
            hocr_word("level", 400, 608, 40, 24),
            hocr_word("with", 444, 608, 35, 24),
            hocr_word("dipstick", 483, 608, 65, 24),
        ]
    }

    /// #1570: reconstructing the numbered-list region through the real pipeline must no
    /// longer produce an accepted table. Asserts the FIXED behavior (`None`) — this is the
    /// TDD-red assertion: it fails against the pre-fix validator (which returns
    /// `Some(3-column grid)`, fabricating a table out of prose and, worse, deleting that
    /// prose from the surrounding page per #1571's centre-in-bbox rule) and passes once
    /// the list-marker guards land.
    #[cfg(feature = "ocr")]
    #[test]
    fn test_numbered_list_region_is_not_reconstructed_as_a_table() {
        let words = numbered_list_page_words();
        let regions = crate::table_core::cluster_words_into_table_regions(&words);

        let list_region = regions
            .into_iter()
            .find(|region| region.len() >= crate::table_core::MIN_TABLE_CANDIDATE_WORDS)
            .expect("the numbered list must cluster into its own table-candidate region");
        assert_eq!(
            list_region.len(),
            24,
            "the list region must isolate all 24 list words from the title/heading"
        );

        let table = reconstruct_table(&list_region, 50, 0.5);
        assert!(
            !table.is_empty(),
            "precondition: the list must reconstruct into a non-empty grid"
        );
        assert_eq!(
            table[0].len(),
            3,
            "precondition: the list reconstructs into 3 columns, matching the bug report"
        );

        let result = post_process_table(table, false, false);
        assert!(
            result.is_none(),
            "a numbered list rendered as a 3-column grid must be rejected, not accepted as a fabricated table"
        );
    }

    /// Precision regression: a genuine layout-guided table (ML-confirmed region) whose
    /// first column happens to be numeric-and-punctuated ("1.", "2.", ...) must still be
    /// accepted. The new list-marker guard is scoped to `!layout_guided`, so this path
    /// never reaches it at all.
    #[test]
    fn test_layout_guided_numeric_first_column_table_is_still_accepted() {
        let table = vec![
            vec![
                "Line".to_string(),
                "Part Description".to_string(),
                "Unit Price".to_string(),
            ],
            vec![
                "1.".to_string(),
                "Stainless Steel Bolt M8x40".to_string(),
                "$4.25".to_string(),
            ],
            vec![
                "2.".to_string(),
                "Anodized Aluminum Bracket".to_string(),
                "$12.90".to_string(),
            ],
            vec![
                "3.".to_string(),
                "Rubber Grommet Assembly".to_string(),
                "$1.15".to_string(),
            ],
            vec![
                "4.".to_string(),
                "Tempered Glass Panel".to_string(),
                "$38.00".to_string(),
            ],
        ];
        let result = post_process_table(table, true, false);
        assert!(
            result.is_some(),
            "a layout-guided table with a punctuated numeric first column must not be eaten by the #1570 fix"
        );
    }

    /// Precision regression, non-layout-guided: the SAME genuine numeric-first-column
    /// table, reconstructed WITHOUT ML layout confirmation, must still be accepted. Its
    /// "Line" header is the signal that separates it from a numbered list — a list has a
    /// marker in every column-0 cell including the first, this table does not (#1570).
    #[test]
    fn test_headed_numeric_first_column_table_survives_the_list_marker_guard() {
        let table = vec![
            vec![
                "Line".to_string(),
                "Part Description".to_string(),
                "Unit Price".to_string(),
            ],
            vec![
                "1.".to_string(),
                "Stainless Steel Bolt M8x40".to_string(),
                "$4.25".to_string(),
            ],
            vec![
                "2.".to_string(),
                "Anodized Aluminum Bracket".to_string(),
                "$12.90".to_string(),
            ],
            vec![
                "3.".to_string(),
                "Rubber Grommet Assembly".to_string(),
                "$1.15".to_string(),
            ],
            vec![
                "4.".to_string(),
                "Tempered Glass Panel".to_string(),
                "$38.00".to_string(),
            ],
        ];
        let result = post_process_table(table, false, false);
        assert!(
            result.is_some(),
            "a headed numeric-first-column table must survive the #1570 list-marker guard without ML confirmation"
        );
    }

    /// A list whose markers did not all survive OCR ("Note" where "3." should be) no
    /// longer satisfies the end-to-end guard, so rejection has to come from the relaxed
    /// `column_text_flow` signal instead. Proves that relaxation is live, not dead code
    /// shadowed by the guard above it (#1570).
    #[test]
    fn test_partially_ocred_list_markers_are_still_rejected_by_text_flow() {
        let table = vec![
            vec!["1.".to_string(), "Drain old".to_string(), "oil from engine".to_string()],
            vec![
                "2.".to_string(),
                "Replace oil".to_string(),
                "filter and gasket".to_string(),
            ],
            vec![
                "Note".to_string(),
                "Add 5.5".to_string(),
                "quarts of synthetic".to_string(),
            ],
            vec![
                "4.".to_string(),
                "Check oil".to_string(),
                "level with dipstick".to_string(),
            ],
            vec![
                "5.".to_string(),
                "Reset the".to_string(),
                "service indicator".to_string(),
            ],
        ];
        let result = post_process_table(table, false, false);
        assert!(
            result.is_none(),
            "a list with one mis-OCRed marker must still be rejected as prose flow"
        );
    }

    /// A prose column's em-dash must survive table normalisation: `normalize_data_cell`'s
    /// dash rewriting is correct for a financial column but not for a title welded to an
    /// em-dash leader (xberg-io/xberg#1582).
    #[test]
    fn test_prose_column_em_dash_survives_table_normalization() {
        let table = vec![
            vec![
                "Line".to_string(),
                "Part Description".to_string(),
                "Unit Price".to_string(),
            ],
            vec![
                "1.".to_string(),
                "Functionaliteit\u{2014}12".to_string(),
                "$4.25".to_string(),
            ],
            vec![
                "2.".to_string(),
                "Anodized Aluminum Bracket".to_string(),
                "$12.90".to_string(),
            ],
            vec![
                "3.".to_string(),
                "Rubber Grommet Assembly".to_string(),
                "$1.15".to_string(),
            ],
            vec![
                "4.".to_string(),
                "Tempered Glass Panel".to_string(),
                "$38.00".to_string(),
            ],
        ];
        let processed = post_process_table(table, true, false).expect("headed prose+price table must be accepted");
        assert_eq!(
            processed[1][1], "Functionaliteit\u{2014}12",
            "a prose cell's em-dash must not be rewritten to an ASCII hyphen"
        );
    }

    /// A part code split by a spaced hyphen (`"HRE - HReco"`) must not be corrupted by the
    /// numeric normaliser's `E-` -> `e-` rewrite, which is only correct inside a scientific
    /// notation exponent (xberg-io/xberg#1582).
    #[test]
    fn test_prose_column_part_code_survives_table_normalization() {
        let table = vec![
            vec![
                "Line".to_string(),
                "Part Description".to_string(),
                "Unit Price".to_string(),
            ],
            vec![
                "1.".to_string(),
                "Montagebeugel HRE - HReco".to_string(),
                "$4.25".to_string(),
            ],
            vec![
                "2.".to_string(),
                "Anodized Aluminum Bracket".to_string(),
                "$12.90".to_string(),
            ],
            vec![
                "3.".to_string(),
                "Rubber Grommet Assembly".to_string(),
                "$1.15".to_string(),
            ],
            vec![
                "4.".to_string(),
                "Tempered Glass Panel".to_string(),
                "$38.00".to_string(),
            ],
        ];
        let processed = post_process_table(table, true, false).expect("headed prose+price table must be accepted");
        assert_eq!(
            processed[1][1], "Montagebeugel HRE - HReco",
            "a part code must not be lowercased or have its hyphen spacing collapsed"
        );
    }

    /// A prose cell whose entire content is a single em-dash means something in a document
    /// (an unfilled field, "not applicable") and must not be silently emptied the way a nil
    /// marker in a numeric column is (xberg-io/xberg#1582).
    #[test]
    fn test_prose_column_lone_em_dash_cell_is_not_emptied() {
        let table = vec![
            vec![
                "Line".to_string(),
                "Part Description".to_string(),
                "Unit Price".to_string(),
            ],
            vec!["1.".to_string(), "\u{2014}".to_string(), "$4.25".to_string()],
            vec![
                "2.".to_string(),
                "Anodized Aluminum Bracket".to_string(),
                "$12.90".to_string(),
            ],
            vec![
                "3.".to_string(),
                "Rubber Grommet Assembly".to_string(),
                "$1.15".to_string(),
            ],
            vec![
                "4.".to_string(),
                "Tempered Glass Panel".to_string(),
                "$38.00".to_string(),
            ],
        ];
        let processed = post_process_table(table, true, false).expect("headed prose+price table must be accepted");
        assert_eq!(
            processed[1][1], "\u{2014}",
            "a lone em-dash in a prose column must not be cleared to an empty cell"
        );
    }

    /// Regression: a genuine numeric/financial table must keep getting the full
    /// normalisation — an em-dash nil cell emptied, `"- 3"` joined to `"-3"`, and a
    /// scientific-notation exponent lowercased — exactly as before #1582.
    #[test]
    fn test_numeric_column_normalization_is_unchanged_by_prose_gate() {
        let table = vec![
            vec!["Item".to_string(), "2024".to_string(), "2023".to_string()],
            vec!["Omzet".to_string(), "1234".to_string(), "1100".to_string()],
            vec![
                "Bijzondere baten".to_string(),
                "\u{2014}".to_string(),
                "- 3".to_string(),
            ],
            vec!["Meetfout".to_string(), "1.5E-05".to_string(), "2.0E-06".to_string()],
            vec!["Afschrijving".to_string(), "-12".to_string(), "-9".to_string()],
        ];
        let processed = post_process_table(table, true, false).expect("financial table must be accepted");
        assert_eq!(
            processed[2],
            vec!["Bijzondere baten".to_string(), String::new(), "-3".to_string()]
        );
        assert_eq!(
            processed[3],
            vec!["Meetfout".to_string(), "1.5e-05".to_string(), "2.0e-06".to_string()]
        );
        assert_eq!(
            processed[4],
            vec!["Afschrijving".to_string(), "-12".to_string(), "-9".to_string()]
        );
    }

    /// Build the reported fixture's transaction table content directly (xberg-io/xberg#1649):
    /// a header row followed by nine rows where WITHDRAWAL/DEPOSIT are mutually exclusive, so
    /// each is empty on a majority of rows and DEPOSIT in particular carries little text overall.
    fn bank_statement_transaction_table() -> Vec<Vec<String>> {
        [
            ["DATE", "DESCRIPTION", "WITHDRAWAL", "DEPOSIT", "BALANCE"],
            ["2026-01-02", "Opening Balance", "", "", "$10,500.00"],
            [
                "2026-01-05",
                "ACH Deposit - EMPLOYER INC",
                "",
                "$2,500.00",
                "$13,000.00",
            ],
            ["2026-01-08", "Check #1042", "$1,250.00", "", "$11,750.00"],
            [
                "2026-01-12",
                "Debit Card Purchase - GROCERY",
                "$87.32",
                "",
                "$11,662.68",
            ],
            ["2026-01-15", "Wire Transfer (Outgoing)", "$3,000.00", "", "$8,662.68"],
            ["2026-01-18", "ATM Withdrawal", "$500.00", "", "$8,162.68"],
            ["2026-01-22", "ACH Deposit - CONSULTING", "", "$5,000.00", "$13,162.68"],
            ["2026-01-25", "Monthly Service Fee", "$15.00", "", "$13,147.68"],
            [
                "2026-01-28",
                "Debit Card Purchase - UTILITIES",
                "$300.00",
                "",
                "$12,847.68",
            ],
        ]
        .into_iter()
        .map(|row| row.into_iter().map(str::to_string).collect())
        .collect()
    }

    /// xberg-io/xberg#1649: a real transaction table's sparse but independently-headed DEPOSIT
    /// column, and its sparse first data row (an opening-balance entry with neither a withdrawal
    /// nor a deposit), must both survive `post_process_table` intact.
    #[test]
    fn issue_1649_sparse_named_deposit_column_and_sparse_first_row_survive_post_process() {
        let table = bank_statement_transaction_table();

        let processed = post_process_table(table, false, false)
            .expect("a real financial table with a sparse, independently-headed column must be accepted");

        assert_eq!(
            processed.len(),
            10,
            "the header plus all nine transaction rows must survive"
        );
        assert_eq!(
            processed[0],
            vec!["DATE", "DESCRIPTION", "WITHDRAWAL", "DEPOSIT", "BALANCE"]
        );
        assert_eq!(
            processed[1],
            vec!["2026-01-02", "Opening Balance", "", "", "$10,500.00"],
            "the sparse first data row must not be folded into a bogus multi-row header merge"
        );
        assert_eq!(
            processed[2],
            vec![
                "2026-01-05",
                "ACH Deposit - EMPLOYER INC",
                "",
                "$2,500.00",
                "$13,000.00"
            ]
        );
    }

    /// Negative control for xberg-io/xberg#1649's `column_sparsity`/`content_asymmetry_sparse_column`
    /// fix: a column that is just as sparse but carries no header label of its own is exactly the
    /// noise those gates exist to catch, and must still be rejected.
    #[test]
    fn issue_1649_unnamed_sparse_column_is_still_rejected() {
        let mut table = bank_statement_transaction_table();
        table[0][3] = String::new();

        assert!(
            post_process_table(table, false, false).is_none(),
            "an unnamed, mostly-empty column must still be treated as noise, not preserved"
        );
    }

    /// Negative control for xberg-io/xberg#1649's `find_data_start` fix: when the first row is
    /// NOT fully populated, the original numeric-density scan must still run unmodified.
    #[test]
    fn issue_1649_find_data_start_leaves_a_genuinely_incomplete_first_row_alone() {
        let mut table = bank_statement_transaction_table();
        table[0][4].clear();

        assert_eq!(
            find_data_start(&table, false),
            2,
            "a first row with an empty cell must not trigger the fully-populated-header shortcut"
        );
    }

    /// Regression for the fully-populated-header shortcut over-firing on a genuine two-row text
    /// header (`!layout_guided`, e.g. Tesseract/PaddleOCR): row 0 is fully populated and
    /// digit-free, but row 1 is a non-numeric header continuation (units/labels), not data. The
    /// shortcut must not stop at row 1 in that case -- it must fall through to the digit-density
    /// scan and land on the first genuinely numeric row.
    #[test]
    fn issue_1649_two_row_text_header_is_not_truncated_by_the_header_shortcut() {
        let table: Vec<Vec<String>> = vec![
            vec!["Region".into(), "Sales Amount".into(), "Growth Rate".into()],
            vec!["Area Code".into(), "Dollars".into(), "Percent".into()],
            vec!["R1".into(), "120".into(), "5".into()],
            vec!["R2".into(), "98".into(), "3".into()],
        ];

        assert_eq!(
            find_data_start(&table, false),
            2,
            "a non-numeric second header row must not be mistaken for data"
        );
    }
}
