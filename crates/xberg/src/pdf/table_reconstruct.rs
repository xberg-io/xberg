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

#[cfg(feature = "pdf")]
use super::hierarchy::SegmentData;

/// Convert a PDF `SegmentData` to an `HocrWord` for table reconstruction.
///
/// `SegmentData` uses PDF coordinates (y=0 at bottom, increases upward).
/// `HocrWord` uses image coordinates (y=0 at top, increases downward).
#[cfg(feature = "pdf")]
pub(crate) fn segment_to_hocr_word(seg: &SegmentData, page_height: f32) -> HocrWord {
    let top_image = (page_height - (seg.y + seg.height)).round().max(0.0) as u32;
    HocrWord {
        text: seg.text.clone(),
        left: seg.x.round().max(0.0) as u32,
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
/// Single-word segments use `segment_to_hocr_word` directly (fast path).
/// Multi-word segments get proportional bbox estimation per word based on
/// byte offset within the segment text.
#[cfg(feature = "pdf")]
pub(crate) fn split_segment_to_words(seg: &SegmentData, page_height: f32) -> Vec<HocrWord> {
    let trimmed = seg.text.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    if !trimmed.contains(char::is_whitespace) {
        return vec![segment_to_hocr_word(seg, page_height)];
    }

    let text = &seg.text;
    let total_bytes = text.len() as f32;
    if total_bytes <= 0.0 {
        return Vec::new();
    }

    let top_image = (page_height - (seg.y + seg.height)).round().max(0.0) as u32;
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
            left: (seg.x + frac_start * seg.width).round().max(0.0) as u32,
            top: top_image,
            width: (frac_width * seg.width).round().max(1.0) as u32,
            height: seg_height,
            confidence: 95.0,
        });
    }

    words
}

/// Convert a page's segments to word-level `HocrWord`s for table extraction.
///
/// Splits multi-word segments into individual words with proportional bounding
/// boxes, ensuring each word can be independently matched to table cells.
#[cfg(feature = "pdf")]
pub(crate) fn segments_to_words(segments: &[SegmentData], page_height: f32) -> Vec<HocrWord> {
    segments
        .iter()
        .flat_map(|seg| split_segment_to_words(seg, page_height))
        .collect()
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
    let min_columns = if allow_single_column {
        1
    } else if layout_guided {
        2
    } else {
        3
    };
    post_process_table_inner(table, min_columns, layout_guided)
}

fn post_process_table_inner(
    mut table: Vec<Vec<String>>,
    min_columns: usize,
    layout_guided: bool,
) -> Option<Vec<Vec<String>>> {
    table.retain(|row| row.iter().any(|cell| !cell.trim().is_empty()));
    if table.is_empty() {
        return None;
    }

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
                    return None;
                }
            }
            if total_chars / non_empty > 80 {
                return None;
            }
        } else {
            if long_cells * 2 > non_empty {
                return None;
            }
            if total_chars / non_empty > 50 {
                return None;
            }
        }
    }

    let col_count = table.first().map_or(0, Vec::len);
    if col_count < min_columns {
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
        header_rows = header_rows[header_rows.len() - 2..].to_vec();
    }

    if header_rows.is_empty() {
        if data_rows.len() < 2 {
            return None;
        }
        header_rows.push(data_rows[0].clone());
        data_rows = data_rows[1..].to_vec();
    }

    let column_count = header_rows.first().or_else(|| data_rows.first()).map_or(0, Vec::len);

    if column_count == 0 {
        return None;
    }

    let mut header = vec![String::new(); column_count];
    for row in &header_rows {
        for (idx, cell) in row.iter().enumerate() {
            let trimmed = cell.trim();
            if trimmed.is_empty() {
                continue;
            }
            if !header[idx].is_empty() {
                header[idx].push(' ');
            }
            header[idx].push_str(trimmed);
        }
    }

    let mut processed = Vec::new();
    processed.push(header);
    processed.extend(data_rows);

    if processed.len() <= 1 {
        return None;
    }

    let mut col = 0;
    while col < processed[0].len() {
        let header_text = processed[0][col].trim().to_string();
        let data_empty = processed[1..]
            .iter()
            .all(|row| row.get(col).is_none_or(|cell| cell.trim().is_empty()));

        if data_empty {
            merge_header_only_column(&mut processed, col, header_text);
        } else {
            col += 1;
        }

        if processed.is_empty() || processed[0].is_empty() {
            return None;
        }
    }

    if processed[0].len() < 2 || processed.len() <= 1 {
        return None;
    }

    prune_spurious_interior_column(&mut processed, layout_guided);

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
            if too_sparse {
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
                return None;
            }
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
            && non_empty_cells >= 6
            && single_word_cells * 100 > non_empty_cells * threshold
        {
            return None;
        }
    }

    if processed[0].len() >= 2 {
        let mut flow_rows = 0usize;
        let mut eligible_rows = 0usize;
        for row in processed.iter().skip(1) {
            let col0 = row.first().map(|s| s.trim()).unwrap_or("");
            let col1 = row.get(1).map(|s| s.trim()).unwrap_or("");
            if col0.is_empty() || col1.is_empty() {
                continue;
            }
            eligible_rows += 1;
            let ends_without_punct =
                !col0.ends_with('.') && !col0.ends_with('?') && !col0.ends_with('!') && !col0.ends_with(':');
            let starts_lowercase = col1.chars().next().is_some_and(|c| c.is_lowercase());
            if ends_without_punct && starts_lowercase {
                flow_rows += 1;
            }
        }
        if eligible_rows >= 3 && flow_rows * 10 > eligible_rows * 6 {
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

                    if char_share < 0.15 && empty_ratio > 0.5 {
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

    for row in processed.iter_mut().skip(1) {
        for cell in row.iter_mut() {
            normalize_data_cell(cell);
        }
    }

    Some(processed)
}

fn find_data_start(table: &[Vec<String>], layout_guided: bool) -> usize {
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
fn prune_spurious_interior_column(table: &mut [Vec<String>], layout_guided: bool) -> bool {
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
    if grid.len() < 2 {
        return false;
    }
    let num_cols = grid[0].len();
    if num_cols < 2 {
        return false;
    }
    let dense_numeric_grid = is_dense_numeric_grid(grid);

    const MAX_EMPTY_CELL_FRACTION_PERCENT: usize = 40;
    let max_cols = grid.iter().map(|r| r.len()).max().unwrap_or(0);
    let total_cells = grid.len() * max_cols;
    if total_cells > 0 {
        let empty_cells = grid.len() * max_cols
            - grid
                .iter()
                .flat_map(|row| row.iter())
                .filter(|cell| !cell.trim().is_empty())
                .count();
        if empty_cells * 100 > total_cells * MAX_EMPTY_CELL_FRACTION_PERCENT {
            return false;
        }
    }

    let data_rows = &grid[1..];
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

            if !dense_numeric_grid && columns_uniform && low_variance {
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

fn merge_header_only_column(table: &mut [Vec<String>], col: usize, header_text: String) {
    if table.is_empty() || table[0].is_empty() {
        return;
    }

    let trimmed = header_text.trim();
    if trimmed.is_empty() && table.len() > 1 {
        for row in table.iter_mut() {
            row.remove(col);
        }
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
            return;
        }
    }

    for row in table.iter_mut() {
        row.remove(col);
    }
}

fn normalize_data_cell(cell: &mut String) {
    let mut text = cell.trim().to_string();
    if text.is_empty() {
        cell.clear();
        return;
    }

    for ch in ['\u{2014}', '\u{2013}', '\u{2212}'] {
        text = text.replace(ch, "-");
    }

    if text.starts_with("- ") {
        text = format!("-{}", text[2..].trim_start());
    }

    text = text.replace("- ", "-");
    text = text.replace(" -", "-");
    text = text.replace("E-", "e-").replace("E+", "e+");

    if text == "-" {
        text.clear();
    }

    *cell = text;
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

        assert!(prune_spurious_interior_column(&mut table, true));
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

        assert!(!prune_spurious_interior_column(&mut table, true));
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

        assert!(!prune_spurious_interior_column(&mut table, true));
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

        assert!(!prune_spurious_interior_column(&mut table, true));
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
}
