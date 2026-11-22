//! Borderless two-column definition grids.
//!
//! A definition grid is a small table whose left column is a short text label
//! and whose right column is that label's value or word: `Credit Tier | x`,
//! `Mean Bad Debt | XX`, `B | 0.4017`. Print-era factor schedules pack several
//! of them side by side on one page band, so the region clusterer sees one wide
//! slab rather than eight tables.
//!
//! The shape is easy to confuse with two-column prose, so admission needs both
//! signals: **repeated row structure** (each inferred column holds a word on
//! the same row band, over several consecutive rows) and **cell semantics**
//! (left cells are short labels, the right column is values or single words).
//! Either signal alone is not enough — geometry alone accepts a narrow prose
//! column, text alone accepts a key/value paragraph.
//!
//! Grids this module produced are tagged at the call site, and only they are
//! re-joined when the row-gap splitter cuts one table into several regions; an
//! ordinary or ruled table is never a stitching candidate.

use crate::pdf::table_reconstruct::{HocrWord, table_to_markdown};
use crate::types::Table;

/// Widest definition grid this module will claim. The schedules this targets
/// carry a label column plus one or two grouping columns and a value column.
const MAX_COLUMNS: usize = 3;

/// Fewest data rows a grid needs. Two rows of `Label | x` are indistinguishable
/// from two lines of incidental text; three begin to show repeated structure.
const MIN_DATA_ROWS: usize = 3;

/// Share of data rows that must populate every column.
const MIN_COMPLETE_ROW_PERCENT: usize = 60;

/// Share of a three-column grid's last column that must hold values. A
/// three-column definition grid splits the label into grouping columns
/// (`FDL | Full Coverage | Factor`), so the right column stays a value column.
/// A three-column roster of words is indistinguishable from columned prose.
const MIN_VALUE_ROW_PERCENT: usize = 60;

/// Share of the region's row bands that must align on every column track.
const MIN_ALIGNED_ROW_PERCENT: usize = 70;

/// Left-edge cluster width for one column track.
const TRACK_TOLERANCE: u32 = 12;

/// A label longer than this reads as a prose cell, not a column label.
const MAX_LABEL_WORDS: usize = 4;
const MAX_LABEL_CHARS: usize = 40;

/// A value/word cell longer than this reads as prose.
const MAX_VALUE_CHARS: usize = 20;

/// A value made only of symbols (`=`, `—`, `%`) may still be a cell value; it
/// cannot carry more than this many characters.
const MAX_SYMBOL_VALUE_CHARS: usize = 3;

/// A left cell that is a short token (`0`, `1+`, `D0`, `A`) may still be a
/// label; anything longer must carry letters to qualify.
const MAX_NUMERIC_LABEL_CHARS: usize = 4;

/// Most tracks considered for one region's partition.
const MAX_REGION_TRACKS: usize = 24;

/// Most tracks one grid may group. A column contributes more than one track
/// when its header wraps onto a second line (`Additional / Driver`) or its
/// labels vary in width, so a two- or three-column grid can span five.
const MAX_GROUP_TRACKS: usize = 5;

/// Minimum rows a candidate group must produce to stand on its own.
const MIN_GROUP_ROWS: usize = MIN_DATA_ROWS + 1;

/// Two vertically adjacent slices of one definition grid are rejoined when the
/// gap is this many median row pitches or less.
const MAX_STITCH_GAP_ROWS: u32 = 3;

/// Horizontal slack when deciding two grid slices share column tracks.
const STITCH_TRACK_TOLERANCE: f64 = 4.0;

/// Page-level context every grid in one region is reconstructed under.
#[derive(Clone, Copy)]
pub(super) struct GridFrame<'a> {
    pub page_height: f32,
    pub page_number: u32,
    pub allow_single_column: bool,
    /// Page paths, so each group can be credited with the rules that span its
    /// own bbox rather than the whole fused region's.
    pub paths: &'a [super::PathContent],
}

#[derive(Clone, Copy)]
struct Track {
    left: u32,
    right: u32,
    words: usize,
}

/// Whether a reconstructed grid has the definition shape: short labels on the
/// left, a value or word on the right, populated on most rows.
///
/// A grid sliced from the middle of a stacked band has no header row of its
/// own — its first row is data. Accepting that form is what lets
/// [`stitch_stacked`] rejoin a grid the row-gap clusterer cut in two; every
/// data row must still satisfy the label/value pattern either way.
pub(super) fn has_definition_shape(grid: &[Vec<String>]) -> bool {
    let Some(first) = grid.first() else {
        return false;
    };
    let width = first.len();
    if !(2..=MAX_COLUMNS).contains(&width) || grid.len() < MIN_GROUP_ROWS {
        return false;
    }

    let data_rows = if has_header_row(first) { &grid[1..] } else { grid };
    if data_rows.len() < MIN_DATA_ROWS {
        return false;
    }

    let mut complete_rows = 0usize;
    for row in data_rows {
        if row.len() != width {
            return false;
        }
        let Some(value) = row.last() else {
            return false;
        };
        let value = value.trim();
        if value.is_empty() || !is_value_or_word(value) {
            return false;
        }
        if row.iter().all(|cell| !cell.trim().is_empty()) {
            complete_rows += 1;
        }
        for label in &row[..width - 1] {
            let label = label.trim();
            if !label.is_empty() && !is_short_label(label) {
                return false;
            }
        }
    }

    if width == MAX_COLUMNS {
        let value_rows = data_rows
            .iter()
            .filter(|row| {
                row.last()
                    .is_some_and(|cell| crate::table_core::is_cell_value_text(cell.trim()))
            })
            .count();
        if value_rows * 100 < data_rows.len() * MIN_VALUE_ROW_PERCENT {
            return false;
        }
    }

    complete_rows >= 2 && complete_rows * 100 >= data_rows.len() * MIN_COMPLETE_ROW_PERCENT
}

/// Shortest cell that can name a column rather than hold a value. A definition
/// grid's values are short tokens: a single letter (`x`), a placeholder (`XX`),
/// a symbol (`=`) or a number (`0.3848`). A header names its column with a word.
/// This is the evidence that keeps a headerless continuation (`D | x`) from
/// reading as an independent table's header (`Plan | Factor`).
const MIN_COLUMN_NAME_CHARS: usize = 3;

/// Whether a row reads as a column-label header.
///
/// Every populated cell must be a short label, and the row must name a column:
/// its first cell is a multi-character word (or empty, as a corner cell), so a
/// headerless continuation row whose first cell is a data label (`D`) does not
/// qualify. Genuinely headed tables keep their differing-header rejection.
fn has_header_row(row: &[String]) -> bool {
    let Some(first) = row.first() else {
        return false;
    };
    let first = first.trim();
    if !first.is_empty() && !is_short_label(first) {
        return false;
    }
    if !row.iter().skip(1).all(|cell| {
        let text = cell.trim();
        text.is_empty() || is_short_label(text)
    }) {
        return false;
    }
    if !first.is_empty() {
        return names_a_column(first);
    }
    row.iter().skip(1).any(|cell| names_a_column(cell.trim()))
}

fn names_a_column(text: &str) -> bool {
    text.chars().count() >= MIN_COLUMN_NAME_CHARS && text.chars().any(char::is_alphabetic)
}

/// Collapse a header that the producer printed on two lines.
///
/// A column-label header can land one line below the value column's header:
/// row 1 is `("", "", "Factor")` and row 2 is `("FDL", "Full Coverage", "")`.
/// Left alone, the first row reads as a headerless data row with empty labels
/// and the grid is rejected. The two rows are one header when the first holds
/// only the value label and the second holds only the column labels.
///
/// Returns whether a collapse happened; a collapsed header is explicit evidence
/// that the grid is a table, which the caller may use instead of a single-row
/// alignment test.
pub(super) fn collapse_wrapped_header(grid: &mut Vec<Vec<String>>) -> bool {
    if grid.len() < MIN_GROUP_ROWS {
        return false;
    }
    let width = grid[0].len();
    if grid[1].len() != width || width < 2 {
        return false;
    }
    let first = &grid[0];
    let second = &grid[1];
    let first_labels_empty = first[..width - 1].iter().all(|cell| cell.trim().is_empty());
    let first_value_present = !first[width - 1].trim().is_empty();
    let second_labels_present = second[..width - 1].iter().any(|cell| !cell.trim().is_empty());
    let second_value_empty = second[width - 1].trim().is_empty();
    if !(first_labels_empty && first_value_present && second_labels_present && second_value_empty) {
        return false;
    }
    // The collapsed grid must still be a definition grid: without this, a
    // value-less row could vanish into the header and the remaining rows would
    // be admitted on the strength of the row that disappeared.
    let mut collapsed = grid.clone();
    let merged = crate::pdf::table_reconstruct::merge_rows_columnwise(&collapsed[..2], width);
    collapsed.drain(1..2);
    collapsed[0] = merged;
    if has_definition_shape(&collapsed) {
        *grid = collapsed;
        return true;
    }
    false
}

fn is_short_label(text: &str) -> bool {
    let trimmed = text.trim();
    !trimmed.is_empty()
        && trimmed.split_whitespace().count() <= MAX_LABEL_WORDS
        && trimmed.chars().count() <= MAX_LABEL_CHARS
        && (trimmed.chars().any(char::is_alphabetic) || trimmed.chars().count() <= MAX_NUMERIC_LABEL_CHARS)
}

fn is_value_or_word(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed.chars().count() <= MAX_VALUE_CHARS
        && !trimmed.contains(char::is_whitespace)
        && (crate::table_core::is_cell_value_text(trimmed)
            || trimmed.chars().any(char::is_alphabetic)
            || is_symbol_value(trimmed))
}

/// A lone symbol (`=`, `—`, `%`) is a value the way [`crate::table_core::is_cell_value_text`]
/// reads a nil dash; `=` is outside that list but ends a factor schedule with no digit in it.
fn is_symbol_value(text: &str) -> bool {
    !text.is_empty()
        && text.chars().count() <= MAX_SYMBOL_VALUE_CHARS
        && text.chars().all(|character| !character.is_alphanumeric())
}

/// Whether the raw geometry backs [`has_definition_shape`]: every inferred
/// column track carries a word on the same row band, on most data rows.
///
/// Word *centres* are compared to the boundaries between column positions, so
/// right-aligned values stay in their own column.
pub(super) fn tracks_align_across_rows(region: &[HocrWord], column_positions: &[u32]) -> bool {
    let width = column_positions.len();
    if !(2..=MAX_COLUMNS).contains(&width) || region.len() < width * MIN_DATA_ROWS {
        return false;
    }
    let rows = crate::table_core::detect_rows(region, 0.5);
    if rows.len() < MIN_GROUP_ROWS {
        return false;
    }

    let boundaries: Vec<u32> = column_positions
        .windows(2)
        .map(|pair| pair[0] + pair[1].saturating_sub(pair[0]) / 2)
        .collect();
    let mut row_words: Vec<Vec<&HocrWord>> = vec![Vec::new(); rows.len()];
    for word in region {
        let center_y = word.top.saturating_add(word.height / 2);
        let Some((index, _)) = rows
            .iter()
            .enumerate()
            .min_by_key(|&(_, row_y)| row_y.abs_diff(center_y))
        else {
            continue;
        };
        row_words[index].push(word);
    }

    // The top row is the header and may name only its leftmost column.
    let data_rows = row_words.len().saturating_sub(1);
    if data_rows < MIN_DATA_ROWS {
        return false;
    }
    let aligned = row_words
        .iter()
        .skip(1)
        .filter(|words| {
            (0..width).all(|column| {
                let low = column.checked_sub(1).map_or(0, |previous| boundaries[previous]);
                let high = boundaries.get(column).copied().unwrap_or(u32::MAX);
                words.iter().any(|word| {
                    let center = word.left.saturating_add(word.width / 2);
                    center >= low && center < high
                })
            })
        })
        .count();
    aligned * 100 >= data_rows * MIN_ALIGNED_ROW_PERCENT
}

/// Split a fused region into its side-by-side definition grids.
///
/// The leftmost tracks must open a valid grid; interior tracks that open none
/// are skipped and their words stay in the page text, so a stray note or an
/// unreconstructable wrapped header cannot shift the grids beside it. A region
/// holding a single definition grid is also returned directly: no ruling
/// reconstruction accepted it, so the grid is the region's only table.
pub(super) fn segment_region(region: &[HocrWord], frame: GridFrame<'_>) -> Option<Vec<Table>> {
    let tracks = region_tracks(region);
    if tracks.len() < 2 || tracks.len() > MAX_REGION_TRACKS {
        return None;
    }

    let groups = partition(&tracks, region, frame)?;
    if groups.is_empty() {
        return None;
    }

    let mut tables = Vec::with_capacity(groups.len());
    for (start, end) in groups {
        let table = reconstruct_grid(region, &tracks, start, end, frame)?;
        tables.push(table);
    }
    Some(tables)
}

/// Reconstruct one group, taking the tighter column clustering the group's
/// tracks allow, then the spacing between tracks themselves.
///
/// A wrapped label whose first word starts on a track of its own (`Additional`
/// at one x, its data labels a few points right) needs the wider threshold to
/// read as one column; a genuine three-column grid needs the tighter one. The
/// definition shape decides — a grid that fails it is not a definition grid at
/// that clustering, so the next one is tried.
fn reconstruct_grid(
    region: &[HocrWord],
    tracks: &[Track],
    start: usize,
    end: usize,
    frame: GridFrame<'_>,
) -> Option<Table> {
    let words = group_words(region, tracks, start, end);
    // Rules belong to the grid whose bbox they span, not to the fused band: a
    // rule under one grid must not supply rule evidence to its neighbour.
    let horizontal_rules = super::horizontal_rules_for_region(frame.paths, &words, frame.page_height);
    let spacing = tracks[start..end]
        .windows(2)
        .map(|pair| pair[1].left.saturating_sub(pair[0].left))
        .min()
        .unwrap_or(1)
        .max(1);
    let thresholds = [(spacing / 2).max(1), spacing];
    for (index, threshold) in thresholds.into_iter().enumerate() {
        if index == 1 && threshold == thresholds[0] {
            break;
        }
        let Some((table, column_positions, wrapped_header)) = super::reconstruct_definition_grid(
            &words,
            frame.page_height,
            frame.page_number,
            frame.allow_single_column,
            horizontal_rules,
            threshold,
        ) else {
            continue;
        };
        // The fallback reconstructs from its own column clustering, so the same
        // alignment threshold the heuristic path applies has to be re-tested
        // here. Shape alone admits a grid whose labels populate only 60% of the
        // rows; the columns only align on those rows (xberg-io/xberg#3351). A
        // two-line header collapsed during reconstruction is explicit table
        // evidence: its value label sits one row above the column labels, so the
        // raw words can never align on every track in one row.
        if has_definition_shape(&table.cells)
            && table.cells.len() >= MIN_GROUP_ROWS
            && (wrapped_header || tracks_align_across_rows(&words, &column_positions))
        {
            return Some(table);
        }
    }
    None
}

fn region_tracks(region: &[HocrWord]) -> Vec<Track> {
    let mut ordered: Vec<&HocrWord> = region.iter().collect();
    ordered.sort_by_key(|word| word.left);
    let mut tracks: Vec<Track> = Vec::new();
    for word in ordered {
        match tracks.last_mut() {
            Some(track) if word.left.abs_diff(track.left) <= TRACK_TOLERANCE => {
                track.right = track.right.max(word.left.saturating_add(word.width));
                track.words += 1;
            }
            _ => tracks.push(Track {
                left: word.left,
                right: word.left.saturating_add(word.width),
                words: 1,
            }),
        }
    }
    // A header word centred over its column forms a one-word track of its own;
    // it neither starts nor ends a grid. Data tracks have several rows of words.
    tracks.retain(|track| track.words >= 2);
    tracks
}

/// Whether the group's left cut runs through a word, so the group would take
/// the right half of a cell and leave the left half to another table.
///
/// A wrapped label like `Additional / Driver` puts its first word on a track of
/// its own; a group starting at the second word would read `Driver` as a label
/// and hide the rest of the cell. Only the left cut is checked: the right cut
/// falls between a value column and the next table's label, and a wide value
/// may legitimately reach past it.
fn starts_mid_word(region: &[HocrWord], tracks: &[Track], index: usize) -> bool {
    if index == 0 {
        return false;
    }
    cut_crosses_word(region, cut_between(tracks, index))
}

/// Whether the cut at `end` runs through a word that the group ending there
/// would leave behind. The cut only matters when another track follows it; the
/// region's right edge is not a table boundary.
fn mid_word_cut(region: &[HocrWord], tracks: &[Track], end: usize) -> bool {
    end < tracks.len() && cut_crosses_word(region, cut_between(tracks, end))
}

fn cut_crosses_word(region: &[HocrWord], cut: u32) -> bool {
    region.iter().any(|word| {
        let center = word.left.saturating_add(word.width / 2);
        word.left < cut && center >= cut
    })
}

/// Tracks a partition claims, used to break row-count ties toward the reading
/// order's leftmost table.
fn covered_tracks(groups: &[(usize, usize)]) -> usize {
    groups.iter().map(|(start, end)| end - start).sum()
}

/// Cut position between two adjacent track groups. Words are attributed by
/// centre, so header words centred over a column join that column.
fn cut_between(tracks: &[Track], group_end: usize) -> u32 {
    let left = tracks[group_end - 1];
    let right = tracks[group_end];
    let left_center = left.left + (left.right - left.left) / 2;
    let right_center = right.left + (right.right - right.left) / 2;
    left_center + right_center.saturating_sub(left_center) / 2
}

fn group_words(region: &[HocrWord], tracks: &[Track], start: usize, end: usize) -> Vec<HocrWord> {
    let low = if start == 0 { 0 } else { cut_between(tracks, start) };
    let high = if end >= tracks.len() {
        u32::MAX
    } else {
        cut_between(tracks, end)
    };
    region
        .iter()
        .filter(|word| {
            let center = word.left.saturating_add(word.width / 2);
            center >= low && center < high
        })
        .cloned()
        .collect()
}

/// Best cover of the track list by valid grids: most rows first, then the most
/// tracks claimed, then the fewest grids.
fn partition(tracks: &[Track], region: &[HocrWord], frame: GridFrame<'_>) -> Option<Vec<(usize, usize)>> {
    type Best = (usize, Vec<(usize, usize)>);

    fn best_from(
        index: usize,
        tracks: &[Track],
        region: &[HocrWord],
        frame: GridFrame<'_>,
        memo: &mut [Option<Option<Best>>],
    ) -> Option<Best> {
        if index == tracks.len() {
            return Some((0, Vec::new()));
        }
        if let Some(cached) = &memo[index] {
            return cached.clone();
        }
        // A track that starts no valid grid may belong to a table this module
        // cannot reconstruct (a wrapped header centred off its data column, a
        // stray note). Skipping it leaves its words unclaimed, so they stay in
        // the page text rather than being misattributed to a neighbour. Only
        // *interior* tracks may be skipped: a partition that leaves the
        // region's leftmost columns unclaimed has shifted every grid one table
        // to the right, which is how a three-column roster becomes a two-column
        // "City | Role" definition grid; one that leaves the final track
        // unclaimed has dropped the roster's last column. Either way the region
        // is rejected rather than truncating the table.
        let mut best: Option<(usize, Vec<(usize, usize)>)> = if index > 0 && index + 1 < tracks.len() {
            best_from(index + 1, tracks, region, frame, memo)
        } else {
            None
        };
        for size in 2..=MAX_GROUP_TRACKS.min(tracks.len() - index) {
            let end = index + size;
            // Neither edge may cut through a word: a group starting inside a
            // cell loses its left part, and one ending mid-cell leaves its last
            // column truncated (the word is handed to the neighbour). Both are
            // how prose fragments get read as short table rows.
            if starts_mid_word(region, tracks, index) || mid_word_cut(region, tracks, end) {
                continue;
            }
            let Some(table) = reconstruct_grid(region, tracks, index, end, frame) else {
                continue;
            };
            let Some((rest_rows, mut rest)) = best_from(end, tracks, region, frame, memo) else {
                continue;
            };
            let rows = table.cells.len() + rest_rows;
            let mut groups = vec![(index, end)];
            groups.append(&mut rest);
            // Cover the leftmost tracks on a tie. A table's first column can be
            // claimed by a group that starts at its neighbour, leaving the
            // label behind; both readings can yield the same row count.
            let better = best.as_ref().is_none_or(|(best_rows, best_groups)| {
                rows > *best_rows
                    || (rows == *best_rows
                        && (covered_tracks(&groups) > covered_tracks(best_groups)
                            || (covered_tracks(&groups) == covered_tracks(best_groups)
                                && groups.len() < best_groups.len())))
            });
            if better {
                best = Some((rows, groups));
            }
        }
        memo[index] = Some(best.clone());
        best
    }

    let mut memo = vec![None; tracks.len()];
    let (_, groups) = best_from(0, tracks, region, frame, &mut memo)?;
    Some(groups)
}

/// Rejoin slices of one definition grid that the row-gap clusterer cut into
/// separate regions.
///
/// The row-gap splitter cuts on vertical whitespace, so a factor schedule with
/// one wide row gap arrives as two regions: the band with the headers and the
/// band below it. Both reconstruct, but only the first has the header. Two
/// tables join when both still have the definition shape, share column count
/// and horizontal extent, and sit within a few row pitches of each other, with
/// the slices taken in reading order. A repeated header row is dropped from the
/// continuation.
///
/// Restricted to definition-shaped tables on purpose: a ruled table is never
/// re-segmented here, and two ordinary tables that happen to share a width are
/// left alone.
///
/// Only the tables the definition-grid path produced are candidates, and a
/// slice that opens with its own header joins only when that header repeats the
/// grid's. Two independent tables of the same width therefore stay separate
/// instead of one's header becoming a data row of the other.
pub(super) fn stitch_tagged<T>(rows: Vec<(Table, T)>, is_definition: Vec<bool>) -> Vec<(Table, T)> {
    if rows.len() != is_definition.len() || !is_definition.iter().any(|&tagged| tagged) {
        return rows;
    }
    let mut definition = Vec::new();
    let mut slots = Vec::with_capacity(rows.len());
    for (row, tagged) in rows.into_iter().zip(is_definition) {
        if tagged {
            slots.push(None);
            definition.push(row);
        } else {
            slots.push(Some(row));
        }
    }
    let mut stitched = stitch_stacked(definition).into_iter();
    let mut rows = Vec::with_capacity(slots.len());
    for slot in slots {
        match slot {
            Some(row) => rows.push(row),
            None => {
                if let Some(row) = stitched.next() {
                    rows.push(row);
                }
            }
        }
    }
    rows.extend(stitched);
    rows
}

fn stitch_stacked<T>(mut rows: Vec<(Table, T)>) -> Vec<(Table, T)> {
    if !rows.iter().any(|(table, _)| has_definition_shape(&table.cells)) {
        return rows;
    }
    rows.sort_by(|left, right| {
        let top = |table: &Table| table.bounding_box.map_or(f64::NEG_INFINITY, |bounds| bounds.y1);
        let left_edge = |table: &Table| table.bounding_box.map_or(f64::INFINITY, |bounds| bounds.x0);
        top(&right.0)
            .total_cmp(&top(&left.0))
            .then_with(|| left_edge(&left.0).total_cmp(&left_edge(&right.0)))
    });

    let mut stitched: Vec<(Table, T)> = Vec::with_capacity(rows.len());
    for row in rows {
        let previous = stitched
            .iter_mut()
            .rev()
            .find(|(candidate, _)| stitches_onto(candidate, &row.0));
        let Some((previous, _)) = previous else {
            stitched.push(row);
            continue;
        };
        let table = row.0;
        let repeated_header = table.cells.first().is_some_and(|row| {
            has_header_row(row)
                && previous
                    .cells
                    .first()
                    .is_some_and(|header| row.len() == header.len() && row.iter().zip(header).all(|(a, b)| a == b))
        });
        let skip = usize::from(repeated_header);
        previous.cells.extend(table.cells.into_iter().skip(skip));
        if let (Some(previous_bounds), Some(table_bounds)) = (previous.bounding_box.as_mut(), table.bounding_box) {
            previous_bounds.y0 = previous_bounds.y0.min(table_bounds.y0);
            previous_bounds.y1 = previous_bounds.y1.max(table_bounds.y1);
            previous_bounds.x0 = previous_bounds.x0.min(table_bounds.x0);
            previous_bounds.x1 = previous_bounds.x1.max(table_bounds.x1);
        }
        previous.markdown = table_to_markdown(&previous.cells);
    }
    stitched
}

fn stitches_onto(previous: &Table, next: &Table) -> bool {
    let (Some(previous_bounds), Some(next_bounds)) = (previous.bounding_box, next.bounding_box) else {
        return false;
    };
    let columns = previous.cells.first().map_or(0, Vec::len);
    if columns != next.cells.first().map_or(0, Vec::len) || !(2..=MAX_COLUMNS).contains(&columns) {
        return false;
    }
    if !has_definition_shape(&previous.cells) || !has_definition_shape(&next.cells) {
        return false;
    }
    if let Some(next_header) = next.cells.first().filter(|row| has_header_row(row)) {
        let repeats_previous = previous.cells.first().is_some_and(|header| {
            header.len() == next_header.len() && header.iter().zip(next_header).all(|(a, b)| a == b)
        });
        if !repeats_previous {
            return false;
        }
    }
    if (previous_bounds.x0 - next_bounds.x0).abs() > STITCH_TRACK_TOLERANCE
        || (previous_bounds.x1 - next_bounds.x1).abs() > STITCH_TRACK_TOLERANCE
    {
        return false;
    }
    let gap = previous_bounds.y0 - next_bounds.y1;
    let pitch = row_pitch(previous).max(row_pitch(next)).max(1.0);
    gap >= 0.0 && gap <= pitch * f64::from(MAX_STITCH_GAP_ROWS)
}

fn row_pitch(table: &Table) -> f64 {
    let Some(bounds) = table.bounding_box else {
        return 0.0;
    };
    let rows = table.cells.len().max(1) as f64;
    (bounds.y1 - bounds.y0) / rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::table_reconstruct::HocrWord;
    use xberg_native_pdf::elements::PathContent;
    use xberg_native_pdf::elements::PathOperation;
    use xberg_native_pdf::layout::Color;

    const PAGE_HEIGHT: f32 = 792.0;
    const ROW_PITCH: u32 = 12;
    const WORD_HEIGHT: u32 = 9;

    fn frame(paths: &[PathContent]) -> GridFrame<'_> {
        GridFrame {
            page_height: PAGE_HEIGHT,
            page_number: 1,
            allow_single_column: false,
            paths,
        }
    }

    fn word(text: &str, left: u32, top: u32, width: u32) -> HocrWord {
        HocrWord {
            text: text.to_string(),
            left,
            top,
            width,
            height: WORD_HEIGHT,
            confidence: 95.0,
        }
    }

    /// Lay out a cell the way the text extractor does: one word per token, one
    /// point apart, so the cell merge has to put them back together.
    fn cell(text: &str, left: u32, top: u32) -> Vec<HocrWord> {
        let mut x = left;
        let mut words = Vec::new();
        for token in text.split_whitespace() {
            let width = token.len() as u32 * 4;
            words.push(word(token, x, top, width));
            x += width + 1;
        }
        words
    }

    /// A two-column definition grid: a header pair and its data rows.
    fn grid(label_x: u32, value_x: u32, header: (&str, &str), rows: &[(&str, &str)], top: u32) -> Vec<HocrWord> {
        let mut words = cell(header.0, label_x, top);
        words.extend(cell(header.1, value_x, top));
        for (index, (label, value)) in rows.iter().enumerate() {
            let y = top + ROW_PITCH * (index as u32 + 1);
            words.extend(cell(label, label_x, y));
            words.extend(cell(value, value_x, y));
        }
        words
    }

    fn ruled_bar(top: f32, left: f32, right: f32) -> PathContent {
        PathContent::from_operations(vec![PathOperation::Rectangle(left, top, right - left, 1.0)])
            .with_fill(Color::black())
            .with_stroke_width(0.0)
    }

    #[test]
    fn segments_side_by_side_definition_grids_from_one_fused_band() {
        let mut words = Vec::new();
        words.extend(grid(
            40,
            120,
            ("Credit Tier", "Factor"),
            &[("A", "0.3848"), ("B", "0.4017"), ("C", "0.4365"), ("D", "0.4995")],
            400,
        ));
        words.extend(grid(
            200,
            270,
            ("ISI", "Factor"),
            &[("1_BEST", "0.5732"), ("2_GOOD", "0.7460"), ("3_AVER", "0.9370")],
            400,
        ));
        words.extend(grid(
            360,
            430,
            ("UW Tier", "Factor"),
            &[("1", "0.3435"), ("2", "0.9169"), ("3", "0.4146")],
            400,
        ));

        let tables = segment_region(&words, frame(&[])).expect("three definition grids");

        let headers: Vec<&str> = tables.iter().map(|table| table.cells[0][0].as_str()).collect();
        assert_eq!(
            headers,
            ["Credit Tier", "ISI", "UW Tier"],
            "each side-by-side grid is its own table; got {tables:?}"
        );
        assert_eq!(tables[0].cells.len(), 5);
        assert_eq!(tables[0].cells[1], ["A", "0.3848"]);
        assert_eq!(tables[1].cells.len(), 4);
        assert_eq!(tables[2].cells.len(), 4);
    }

    #[test]
    fn keeps_a_lone_definition_grid_whose_values_are_single_letters() {
        // This is the shape the borderless reconstruction loses: a lowercase
        // value reads as a sentence continuation and the value `=` is not a
        // number (ParseBench record 0580ae4eb20f312e0b8b). Labels are multi-word
        // cells exactly as the extractor produces them.
        let mut rows: Vec<(&str, &str)> = (0..7).map(|_| ("Credit Tier", "x")).collect();
        rows[0] = ("Mean Bad Debt", "XX");
        rows[3] = ("Additional Driver Count Group", "x");
        rows[6] = ("Bad Debt Factor", "=");
        let words = grid(40, 190, ("Factor", ""), &rows, 200);

        let tables = segment_region(&words, frame(&[])).expect("one definition grid");

        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].cells.len(), 8);
        assert_eq!(tables[0].cells[0], ["Factor", ""]);
        assert_eq!(tables[0].cells[1], ["Mean Bad Debt", "XX"]);
        assert_eq!(tables[0].cells[4], ["Additional Driver Count Group", "x"]);
        assert_eq!(tables[0].cells[7], ["Bad Debt Factor", "="]);
    }

    #[test]
    fn two_column_prose_is_not_a_definition_grid() {
        // Running text that happens to align into two x-buckets: long cells,
        // a lowercase continuation on most rows, no repeated label/value pairs.
        let prose = [
            ("The rates above do not include a one time", "policy fee and apply"),
            ("A discount factor of 0.93 is applied for", "married applicants in"),
            ("the state of Texas and the surrounding", "counties shown below"),
            ("for policyholders who elect the standard", "plan with no riders"),
        ];
        let mut words = Vec::new();
        for (index, (left, right)) in prose.iter().enumerate() {
            let y = 200 + ROW_PITCH * index as u32;
            words.extend(cell(left, 40, y));
            words.extend(cell(right, 320, y));
        }

        assert!(
            segment_region(&words, frame(&[])).is_none(),
            "prose must not be segmented into a definition grid"
        );
    }

    #[test]
    fn both_group_cuts_are_checked_for_a_split_word() {
        // Track centres come from the tracks' own extents, so a word that
        // starts at its track's left edge cannot cross that group's right cut;
        // the check guards the remaining shape, a word that starts inside the
        // span and is centred past the cut.
        let tracks = vec![
            Track {
                left: 40,
                right: 200,
                words: 4,
            },
            Track {
                left: 150,
                right: 160,
                words: 2,
            },
        ];
        let region = vec![word("continuation", 100, 200, 80), word("tail", 150, 212, 10)];

        assert!(
            mid_word_cut(&region, &tracks, 1) || mid_word_cut(&region, &tracks, 2),
            "a word centred past the cut is a split word"
        );
        assert!(
            !mid_word_cut(&region, &tracks, 2),
            "the region's right edge is not a table boundary"
        );

        // The left edge uses the same predicate.
        assert!(starts_mid_word(&region, &tracks, 1), "45..125 crosses the cut at 95");
    }

    #[test]
    fn a_rule_belongs_only_to_the_grid_it_spans() {
        let left = grid(
            40,
            120,
            ("Credit Tier", "Factor"),
            &[("A", "0.3848"), ("B", "0.4017"), ("C", "0.4365")],
            400,
        );
        let right = grid(
            300,
            380,
            ("ISI", "Factor"),
            &[("1_BEST", "0.5732"), ("2_GOOD", "0.7460"), ("3_AVER", "0.9370")],
            400,
        );
        // Rules under the left grid only, in PDF (bottom-up) space. The words
        // are in image (top-down) space, which the attribution converts.
        let paths = vec![ruled_bar(360.0, 40.0, 150.0), ruled_bar(380.0, 40.0, 150.0)];

        assert_eq!(
            super::super::horizontal_rules_for_region(&paths, &left, PAGE_HEIGHT),
            2,
            "both rules span the left grid"
        );
        assert_eq!(
            super::super::horizontal_rules_for_region(&paths, &right, PAGE_HEIGHT),
            0,
            "the left grid's rules do not span the right grid"
        );
    }

    #[test]
    fn stitches_a_continuation_slice_below_its_header_slice() {
        let header = Table {
            cells: vec![
                vec!["Credit Tier".into(), "Factor".into()],
                vec!["A".into(), "0.3848".into()],
                vec!["B".into(), "0.4017".into()],
                vec!["C".into(), "0.4365".into()],
            ],
            bounding_box: Some(crate::types::BoundingBox {
                x0: 40.0,
                y0: 300.0,
                x1: 140.0,
                y1: 330.0,
            }),
            ..Default::default()
        };
        let continuation = Table {
            cells: vec![
                vec!["D".into(), "0.4995".into()],
                vec!["E".into(), "0.5475".into()],
                vec!["F".into(), "0.5840".into()],
                vec!["G".into(), "0.6143".into()],
            ],
            bounding_box: Some(crate::types::BoundingBox {
                x0: 40.0,
                y0: 282.0,
                x1: 142.0,
                y1: 298.0,
            }),
            ..Default::default()
        };

        let stitched = stitch_tagged(vec![(continuation, ()), (header, ())], vec![true, true]);

        assert_eq!(stitched.len(), 1);
        assert_eq!(stitched[0].0.cells.len(), 8, "header stays first, continuation follows");
        assert_eq!(stitched[0].0.cells[0], ["Credit Tier", "Factor"]);
        assert_eq!(stitched[0].0.cells[7], ["G", "0.6143"]);
    }

    #[test]
    fn a_slice_with_its_own_header_is_a_separate_table() {
        let upper = Table {
            cells: vec![
                vec!["Credit Tier".into(), "Factor".into()],
                vec!["A".into(), "0.3848".into()],
                vec!["B".into(), "0.4017".into()],
                vec!["C".into(), "0.4365".into()],
            ],
            bounding_box: Some(crate::types::BoundingBox {
                x0: 40.0,
                y0: 300.0,
                x1: 140.0,
                y1: 330.0,
            }),
            ..Default::default()
        };
        let lower = Table {
            cells: vec![
                vec!["ISI".into(), "Factor".into()],
                vec!["1_BEST".into(), "0.5732".into()],
                vec!["2_GOOD".into(), "0.7460".into()],
                vec!["3_AVER".into(), "0.9370".into()],
            ],
            bounding_box: Some(crate::types::BoundingBox {
                x0: 40.0,
                y0: 282.0,
                x1: 142.0,
                y1: 298.0,
            }),
            ..Default::default()
        };

        let stitched = stitch_tagged(vec![(lower, ()), (upper, ())], vec![true, true]);

        assert_eq!(stitched.len(), 2, "a differing header must not become a data row");
    }

    #[test]
    fn a_three_track_roster_is_not_truncated_to_two_columns() {
        // `Name | City | Role` where the last column is a word, not a value: the
        // three-column candidate fails the value-text threshold, and the
        // two-column `Name | City` candidate would otherwise pass. Dropping
        // `Role` loses a column of the roster, so the region must be rejected.
        let mut words = cell("Name", 40, 200);
        words.extend(cell("City", 120, 200));
        words.extend(cell("Role", 200, 200));
        let rows = [
            ("Alice", "Boston", "Manager"),
            ("Bob", "Denver", "Analyst"),
            ("Carol", "Austin", "Clerk"),
            ("Dave", "Seattle", "Engineer"),
        ];
        for (index, (name, city, role)) in rows.iter().enumerate() {
            let y = 200 + ROW_PITCH * (index as u32 + 1);
            words.extend(cell(name, 40, y));
            words.extend(cell(city, 120, y));
            words.extend(cell(role, 200, y));
        }

        assert!(
            segment_region(&words, frame(&[])).is_none(),
            "a partition that leaves the final track unclaimed must not return a truncated roster"
        );
    }

    #[test]
    fn labels_on_six_of_ten_rows_do_not_align_enough() {
        // The fallback used to accept on shape alone: 6 of 10 data rows are
        // complete (60%, the shape threshold) but the label column carries a
        // word on only 6 rows, below the 70% alignment threshold the heuristic
        // path already applies.
        let mut words = cell("Tier", 40, 200);
        words.extend(cell("Factor", 120, 200));
        for index in 0..10u32 {
            let y = 200 + ROW_PITCH * (index + 1);
            if index < 6 {
                words.extend(cell("Label", 40, y));
            }
            words.extend(cell("x", 120, y));
        }

        assert!(
            segment_region(&words, frame(&[])).is_none(),
            "a grid whose columns align on only 60% of the rows must be rejected"
        );
    }

    #[test]
    fn independently_headed_tables_do_not_merge() {
        let table = |header_label: &str, rows: &[(&str, &str)], y0: f64| Table {
            cells: std::iter::once(vec![header_label.to_string(), "Factor".into()])
                .chain(rows.iter().map(|(a, b)| vec![(*a).to_string(), (*b).to_string()]))
                .collect(),
            bounding_box: Some(crate::types::BoundingBox {
                x0: 40.0,
                y0,
                x1: 140.0,
                y1: y0 + 15.0,
            }),
            ..Default::default()
        };
        let upper = table(
            "Credit Tier",
            &[("A", "0.3848"), ("B", "0.4017"), ("C", "0.4365")],
            300.0,
        );
        let lower = table("Plan", &[("D", "0.4995"), ("E", "0.5475"), ("F", "0.5840")], 282.0);
        let stitched = stitch_tagged(vec![(lower, ()), (upper, ())], vec![true, true]);

        assert_eq!(
            stitched.len(),
            2,
            "two independently headed tables must not become one table with a header row: {:?}",
            stitched
                .iter()
                .map(|(table, _)| table.cells.clone())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_corner_empty_header_keeps_its_table_separate() {
        // A header row can leave its corner cell empty. It is still a header,
        // and it still must not be inserted into the table above as a data row.
        let upper = Table {
            cells: vec![
                vec!["Credit Tier".into(), "Factor".into()],
                vec!["A".into(), "0.3848".into()],
                vec!["B".into(), "0.4017".into()],
                vec!["C".into(), "0.4365".into()],
            ],
            bounding_box: Some(crate::types::BoundingBox {
                x0: 40.0,
                y0: 300.0,
                x1: 140.0,
                y1: 330.0,
            }),
            ..Default::default()
        };
        let lower = Table {
            cells: vec![
                vec![String::new(), "Factor".into()],
                vec!["Gold".into(), "Y".into()],
                vec!["Silver".into(), "N".into()],
                vec!["Bronze".into(), "Y".into()],
            ],
            bounding_box: Some(crate::types::BoundingBox {
                x0: 40.0,
                y0: 282.0,
                x1: 140.0,
                y1: 298.0,
            }),
            ..Default::default()
        };
        let stitched = stitch_tagged(vec![(lower, ()), (upper, ())], vec![true, true]);

        assert_eq!(stitched.len(), 2, "a corner-empty header is still a header");
    }

    #[test]
    fn a_headerless_short_value_continuation_stitches() {
        let upper = Table {
            cells: vec![
                vec!["Credit Tier".into(), "Factor".into()],
                vec!["A".into(), "0.3848".into()],
                vec!["B".into(), "0.4017".into()],
                vec!["C".into(), "0.4365".into()],
            ],
            bounding_box: Some(crate::types::BoundingBox {
                x0: 40.0,
                y0: 300.0,
                x1: 140.0,
                y1: 330.0,
            }),
            ..Default::default()
        };
        let lower = Table {
            cells: vec![
                vec!["D".into(), "x".into()],
                vec!["E".into(), "x".into()],
                vec!["F".into(), "x".into()],
                vec!["G".into(), "x".into()],
            ],
            bounding_box: Some(crate::types::BoundingBox {
                x0: 40.0,
                y0: 282.0,
                x1: 140.0,
                y1: 298.0,
            }),
            ..Default::default()
        };
        let stitched = stitch_tagged(vec![(lower, ()), (upper, ())], vec![true, true]);

        assert_eq!(stitched.len(), 1, "`D | x` is a data row, not a new header");
        assert_eq!(stitched[0].0.cells.len(), 8);
        assert_eq!(stitched[0].0.cells[4], ["D", "x"]);
    }

    #[test]
    fn a_repeated_headerless_first_row_is_kept() {
        // Two slices whose first rows coincide but whose value `0.4995` is not a
        // column name: the row is data, so dropping it as a repeated header
        // loses a row.
        let table = |rows: &[(&str, &str)], y0: f64| Table {
            cells: rows
                .iter()
                .map(|(a, b)| vec![(*a).to_string(), (*b).to_string()])
                .collect(),
            bounding_box: Some(crate::types::BoundingBox {
                x0: 40.0,
                y0,
                x1: 140.0,
                y1: y0 + 15.0,
            }),
            ..Default::default()
        };
        let upper = table(
            &[("A", "0.4995"), ("B", "0.5475"), ("C", "0.5840"), ("D", "0.6143")],
            300.0,
        );
        let lower = table(
            &[("A", "0.4995"), ("E", "0.6702"), ("F", "0.7152"), ("G", "0.8190")],
            282.0,
        );
        let stitched = stitch_tagged(vec![(lower, ()), (upper, ())], vec![true, true]);

        assert_eq!(stitched.len(), 1);
        assert_eq!(
            stitched[0].0.cells.len(),
            8,
            "a repeated data row must not be dropped as a header"
        );
        assert_eq!(stitched[0].0.cells[4], ["A", "0.4995"]);
    }

    #[test]
    fn ordinary_tables_are_never_stitched() {
        let cells = |row: &str| vec![vec![row.to_string(), "1".to_string()]; 4];
        let bounds = |y: f64| {
            Some(crate::types::BoundingBox {
                x0: 40.0,
                y0: y,
                x1: 140.0,
                y1: y + 15.0,
            })
        };
        let first = Table {
            cells: cells("Label"),
            bounding_box: bounds(300.0),
            ..Default::default()
        };
        let second = Table {
            cells: cells("Label"),
            bounding_box: bounds(282.0),
            ..Default::default()
        };

        let stitched = stitch_tagged(vec![(first, ()), (second, ())], vec![false, false]);

        assert_eq!(stitched.len(), 2);
    }
}
