//! Core table reconstruction types and algorithms.
//!
//! This module provides the `HocrWord` type and table reconstruction functions
//! that are shared between the OCR and PDF modules. The algorithms detect
//! column/row structure from word bounding boxes and reconstruct tabular layouts.
//!
//! Originally adapted from the `hocr` module of `html-to-markdown-rs` (removed in v3).

/// Represents a word extracted from hOCR (or any source) with position and confidence information.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone)]
pub struct HocrWord {
    /// Recognized word text.
    pub text: String,
    /// Left edge of the word bounding box in pixels.
    pub left: u32,
    /// Top edge of the word bounding box in pixels.
    pub top: u32,
    /// Bounding box width in pixels.
    pub width: u32,
    /// Bounding box height in pixels.
    pub height: u32,
    /// OCR confidence score (0.0–100.0).
    pub confidence: f64,
}

impl HocrWord {
    /// Get the right edge position.
    #[cfg(test)]
    #[inline]
    pub(crate) fn right(&self) -> u32 {
        self.left + self.width
    }

    /// Get the bottom edge position.
    #[cfg(test)]
    #[inline]
    pub(crate) fn bottom(&self) -> u32 {
        self.top + self.height
    }

    /// Get the vertical center position.
    #[inline]
    pub(crate) fn y_center(&self) -> f64 {
        self.top as f64 + (self.height as f64 / 2.0)
    }

    /// Get the horizontal center position.
    #[cfg(test)]
    #[inline]
    pub(crate) fn x_center(&self) -> f64 {
        self.left as f64 + (self.width as f64 / 2.0)
    }
}

/// Detect column positions from word x-coordinates.
///
/// Groups words by approximate x-position (within `column_threshold` pixels)
/// and returns the median x-position for each detected column, sorted left to right.
pub(crate) fn detect_columns(words: &[HocrWord], column_threshold: u32) -> Vec<u32> {
    if words.is_empty() {
        return Vec::new();
    }

    let mut position_groups: Vec<Vec<u32>> = Vec::new();

    for word in words {
        let x_pos = word.left;

        let mut found_group = false;
        for group in &mut position_groups {
            if let Some(&first_pos) = group.first()
                && x_pos.abs_diff(first_pos) <= column_threshold
            {
                group.push(x_pos);
                found_group = true;
                break;
            }
        }

        if !found_group {
            position_groups.push(vec![x_pos]);
        }
    }

    let mut columns: Vec<u32> = position_groups
        .iter()
        .filter(|group| !group.is_empty())
        .map(|group| {
            let mut sorted = group.clone();
            sorted.sort_unstable();
            let mid = sorted.len() / 2;
            sorted[mid]
        })
        .collect();

    columns.sort_unstable();
    columns
}

/// Compute the median word height. Returns 0 for an empty slice.
///
/// Extracted so `detect_rows`'s row-grouping threshold and
/// `merge_words_into_cell_tokens`'s cell-merge threshold are always computed
/// from the same statistic, rather than duplicating the sort-and-index in
/// two places.
fn median_word_height(words: &[HocrWord]) -> u32 {
    if words.is_empty() {
        return 0;
    }
    let mut heights: Vec<u32> = words.iter().map(|w| w.height).collect();
    heights.sort_unstable();
    heights[heights.len() / 2]
}

/// Detect row positions from word y-coordinates.
///
/// Groups words by their vertical center position and returns the median
/// y-position for each detected row. The `row_threshold_ratio` is multiplied
/// by the median word height to determine the grouping threshold.
pub(crate) fn detect_rows(words: &[HocrWord], row_threshold_ratio: f64) -> Vec<u32> {
    if words.is_empty() {
        return Vec::new();
    }

    let median_height = median_word_height(words);
    let row_threshold = (median_height as f64 * row_threshold_ratio) as u32;

    let mut position_groups: Vec<Vec<f64>> = Vec::new();

    for word in words {
        let y_center = word.y_center();

        let mut found_group = false;
        for group in &mut position_groups {
            if let Some(&first_pos) = group.first()
                && (y_center - first_pos).abs() <= row_threshold as f64
            {
                group.push(y_center);
                found_group = true;
                break;
            }
        }

        if !found_group {
            position_groups.push(vec![y_center]);
        }
    }

    let mut rows: Vec<u32> = position_groups
        .iter()
        .filter(|group| !group.is_empty())
        .map(|group| {
            let mut sorted = group.clone();
            sorted.sort_by(|a, b| a.total_cmp(b));
            let mid = sorted.len() / 2;
            sorted[mid] as u32
        })
        .collect();

    rows.sort_unstable();
    rows
}

/// Find which row a word belongs to based on its y-center.
fn find_row_index(row_positions: &[u32], word: &HocrWord) -> Option<usize> {
    let y_center = word.y_center() as u32;

    row_positions
        .iter()
        .enumerate()
        .min_by_key(|&(_, row_y)| row_y.abs_diff(y_center))
        .map(|(idx, _)| idx)
}

/// Find which column a word belongs to based on its x-position.
fn find_column_index(col_positions: &[u32], word: &HocrWord) -> Option<usize> {
    let x_pos = word.left;

    col_positions
        .iter()
        .enumerate()
        .min_by_key(|&(_, col_x)| col_x.abs_diff(x_pos))
        .map(|(idx, _)| idx)
}

/// Determine which columns of a table grid contain at least one non-empty
/// cell. Shared by `remove_empty_rows_and_columns` and
/// `reconstruct_table_with_columns`, which both need to drop the same set of
/// all-empty columns — the latter also has to filter a parallel
/// column-position vector by the identical mask so positions keep lining up
/// 1:1 with the grid's surviving columns.
fn non_empty_column_mask(table: &[Vec<String>]) -> Vec<bool> {
    let num_cols = table.first().map_or(0, Vec::len);
    let mut mask = vec![false; num_cols];

    for row in table {
        for (col_idx, cell) in row.iter().enumerate() {
            if !cell.trim().is_empty() {
                mask[col_idx] = true;
            }
        }
    }

    mask
}

/// Remove empty rows and columns from a table grid.
fn remove_empty_rows_and_columns(table: Vec<Vec<String>>) -> Vec<Vec<String>> {
    if table.is_empty() {
        return table;
    }

    let non_empty_cols = non_empty_column_mask(&table);

    table
        .into_iter()
        .filter(|row| row.iter().any(|cell| !cell.trim().is_empty()))
        .map(|row| {
            row.into_iter()
                .enumerate()
                .filter(|(idx, _)| non_empty_cols[*idx])
                .map(|(_, cell)| cell)
                .collect()
        })
        .collect()
}

/// Fraction of the median word height used as the maximum horizontal gap (in
/// pixels) for merging two horizontally-adjacent words within the same
/// detected row into a single cell token before column detection
/// (xberg-io/xberg#688).
///
/// Without this pre-merge, a table cell containing more than one word (e.g.
/// "ABX Air n") puts that cell's second and third words at x-positions that
/// match no genuine column start, so [`detect_columns`] mints a spurious
/// near-empty column per extra word — exactly what the downstream structural
/// validator then rejects, discarding a genuine table entirely.
///
/// 0.6 was chosen because it recovers the exact ground-truth column count
/// (10) on a 215-word-box fixture captured from a real scanned newspaper
/// stock table (ground truth: 16 rows x 10 columns; two side-by-side
/// sub-tables give true column starts at x = {46, 89, 135, 230, 272} and
/// {340, 386, 428, 526, 567}). Measured sensitivity of this factor on that
/// fixture: 0.4 -> 12 columns, 0.6 -> 10 columns (correct), 0.8 -> 9 columns,
/// 1.0 -> 6 columns.
pub(crate) const CELL_MERGE_GAP_HEIGHT_RATIO: f64 = 0.6;

/// Merge horizontally-adjacent words within the same detected row into
/// single cell tokens, so a multi-word table cell is fed to
/// [`detect_columns`] as one token instead of minting a spurious column per
/// extra word (xberg-io/xberg#688).
///
/// Two words in the same row are merged when the horizontal gap between them
/// (the left word's right edge to the right word's left edge) is at most
/// `CELL_MERGE_GAP_HEIGHT_RATIO * median word height`. Rows are the same
/// `row_positions` the caller already computed via `detect_rows` — this does
/// not invent a second row model.
///
/// The returned tokens are for column detection only. `reconstruct_table`
/// still assigns the *original* words to cells, so cell text is exactly what
/// the pre-merge code would have produced for a correctly detected column
/// (words joined by a single space in left-to-right order).
fn merge_words_into_cell_tokens(words: &[HocrWord], row_positions: &[u32]) -> Vec<HocrWord> {
    if words.len() <= 1 || row_positions.is_empty() {
        return words.to_vec();
    }

    let merge_gap = median_word_height(words) as f64 * CELL_MERGE_GAP_HEIGHT_RATIO;

    let mut rows: Vec<Vec<&HocrWord>> = vec![Vec::new(); row_positions.len()];
    for word in words {
        if let Some(row_index) = find_row_index(row_positions, word) {
            rows[row_index].push(word);
        }
    }

    let mut tokens = Vec::with_capacity(words.len());
    for mut row_words in rows {
        row_words.sort_by_key(|w| w.left);

        let mut current: Option<HocrWord> = None;
        for word in row_words {
            current = Some(match current.take() {
                None => word.clone(),
                Some(mut token) => {
                    let gap = word.left as f64 - (token.left + token.width) as f64;
                    if gap <= merge_gap {
                        let new_right = (word.left + word.width).max(token.left + token.width);
                        let new_bottom = (word.top + word.height).max(token.top + token.height);
                        token.top = token.top.min(word.top);
                        token.width = new_right.saturating_sub(token.left);
                        token.height = new_bottom.saturating_sub(token.top);
                        token.text.push(' ');
                        token.text.push_str(&word.text);
                        token
                    } else {
                        tokens.push(token);
                        word.clone()
                    }
                }
            });
        }
        if let Some(token) = current {
            tokens.push(token);
        }
    }

    tokens
}

/// Reconstruct a table grid from words with bounding box positions.
///
/// Takes detected words and reconstructs a 2D table by:
/// 1. Detecting row positions (grouping by y-center within `row_threshold_ratio` * median height)
/// 2. Merging horizontally-adjacent words within the same row into cell tokens
///    (see [`CELL_MERGE_GAP_HEIGHT_RATIO`]) so a multi-word cell does not mint
///    spurious columns
/// 3. Detecting column positions from those tokens (grouping by x-coordinate
///    within `column_threshold`)
/// 4. Assigning the *original* words to cells based on closest row/column
/// 5. Combining words within the same cell
///
/// Returns a `Vec<Vec<String>>` where each inner `Vec` is a row of cell texts.
///
/// Thin wrapper over [`reconstruct_table_with_columns`] for callers that only
/// need the grid; use that function instead when the column x-positions used
/// to build the grid must stay correlated with it (e.g. a caller that later
/// indexes `column_positions[column]` against `grid[row][column]`).
pub(crate) fn reconstruct_table(
    words: &[HocrWord],
    column_threshold: u32,
    row_threshold_ratio: f64,
) -> Vec<Vec<String>> {
    reconstruct_table_with_columns(words, column_threshold, row_threshold_ratio).0
}

/// Like [`reconstruct_table`], but also returns the column x-positions
/// actually used to build the grid, filtered to the same set of columns that
/// survived empty-column removal — so `result.1[i]` corresponds exactly to
/// `result.0[row][i]` for every row.
///
/// Callers that separately call [`detect_columns`] on the raw input words and
/// then index a returned grid by those positions (e.g. to compare adjacent
/// columns) MUST use this function instead: `reconstruct_table` now detects
/// columns from post-merge cell tokens, not from `words` directly, so a
/// `detect_columns(words, ...)` call sitting next to a plain `reconstruct_table`
/// call no longer corresponds to that grid's column count or positions.
pub(crate) fn reconstruct_table_with_columns(
    words: &[HocrWord],
    column_threshold: u32,
    row_threshold_ratio: f64,
) -> (Vec<Vec<String>>, Vec<u32>) {
    if words.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let row_positions = detect_rows(words, row_threshold_ratio);
    let cell_tokens = merge_words_into_cell_tokens(words, &row_positions);
    let col_positions = detect_columns(&cell_tokens, column_threshold);

    if col_positions.is_empty() || row_positions.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let result = assign_words_to_cells(words, &row_positions, &col_positions);

    let non_empty_cols = non_empty_column_mask(&result);
    let kept_col_positions: Vec<u32> = col_positions
        .iter()
        .zip(non_empty_cols.iter())
        .filter(|&(_, &keep)| keep)
        .map(|(&pos, _)| pos)
        .collect();

    (remove_empty_rows_and_columns(result), kept_col_positions)
}

/// Fraction of a cell's median word height within which two words belong to the same visual line.
///
/// Sized to sit between the two deltas it must tell apart: a sub/superscript is offset from its
/// base's vertical centre by a fraction of its own height (0.6pt against an 11.59pt line on the
/// GH#1628 reproducer), while a wrapped second line is a full line height away. Shares the shape,
/// but not the value, of [`CELL_MERGE_GAP_HEIGHT_RATIO`], which measures a horizontal gap. ~keep
const CELL_LINE_GROUP_HEIGHT_RATIO: f64 = 0.5;

/// Order one cell's words for joining: top-to-bottom by visual line, left-to-right within a line.
///
/// Arrival order is wrong for a cell containing a sub/superscript (xberg-io/xberg#1628). A
/// sub/superscript is drawn as its own content-stream segment sitting a fraction of a point below
/// the line it annotates, so every reading-order sort upstream of here places it after the whole
/// line rather than beside the symbol it belongs to — `eta_S %` joins as `eta % S`.
///
/// Sorting the cell by `left` alone fixes that and introduces something worse: a cell holding two
/// wrapped lines interleaves them (`Hello world` / `again here` becomes `Hello again world here`),
/// which would scramble every wrapped cell in the corpus rather than only cells with scripts.
/// Grouping into visual lines first separates the two cases, because the vertical deltas differ by
/// an order of magnitude — see [`CELL_LINE_GROUP_HEIGHT_RATIO`]. ~keep
fn order_cell_words_in_reading_order(mut cell_words: Vec<&HocrWord>) -> Vec<&HocrWord> {
    if cell_words.len() <= 1 {
        return cell_words;
    }

    let mut heights: Vec<u32> = cell_words.iter().map(|word| word.height).collect();
    heights.sort_unstable();
    let line_gap = heights[heights.len() / 2] as f64 * CELL_LINE_GROUP_HEIGHT_RATIO;

    cell_words.sort_by(|a, b| a.y_center().total_cmp(&b.y_center()).then_with(|| a.left.cmp(&b.left)));

    let mut lines: Vec<Vec<&HocrWord>> = Vec::new();
    for word in cell_words {
        let same_line = lines.last().is_some_and(|line| {
            let centre = line.iter().map(|w| w.y_center()).sum::<f64>() / line.len() as f64;
            (word.y_center() - centre).abs() <= line_gap
        });
        match lines.last_mut() {
            Some(line) if same_line => line.push(word),
            _ => lines.push(vec![word]),
        }
    }

    for line in &mut lines {
        line.sort_by_key(|word| word.left);
    }
    lines.into_iter().flatten().collect()
}

/// Assign each original word to its nearest detected row/column and combine
/// same-cell words into space-joined cell text. `col_positions` may come from
/// merged cell tokens (see [`merge_words_into_cell_tokens`]) rather than from
/// `words` directly — `find_column_index` only needs the candidate x-positions,
/// not the tokens that produced them.
fn assign_words_to_cells(words: &[HocrWord], row_positions: &[u32], col_positions: &[u32]) -> Vec<Vec<String>> {
    let num_rows = row_positions.len();
    let num_cols = col_positions.len();
    let mut table: Vec<Vec<Vec<&HocrWord>>> = vec![vec![vec![]; num_cols]; num_rows];

    for word in words {
        if let (Some(r), Some(c)) = (
            find_row_index(row_positions, word),
            find_column_index(col_positions, word),
        ) && r < num_rows
            && c < num_cols
        {
            table[r][c].push(word);
        }
    }

    table
        .into_iter()
        .map(|row| {
            row.into_iter()
                .map(|cell_words| {
                    // A "word" whose whole segment held no whitespace keeps that segment's own
                    // trailing space (`split_segment_to_words_lifted` returns it unsplit), which
                    // then stacks on the separator below and renders `Q_HE GJ` as `Q  HE GJ`.
                    // Collapse each word's own whitespace so the join alone owns the spacing. ~keep
                    order_cell_words_in_reading_order(cell_words)
                        .into_iter()
                        .flat_map(|word| word.text.split_whitespace())
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .collect()
        })
        .collect()
}

/// Convert a table grid to markdown format.
///
/// The first row is treated as the header row, with a separator line added after it.
///
/// Delegates to the crate's single table renderer
/// ([`crate::rendering::common::render_table_markdown`]) so a reconstructed
/// OCR/PDF table serialises identically to one from any other source
/// (xberg-io/xberg#220). That renderer also pads every row to the widest row,
/// which this reconstruction path did not do — ragged rows used to emit fewer
/// pipe columns than the header, misaligning the table for downstream parsers.
pub(crate) fn table_to_markdown(table: &[Vec<String>]) -> String {
    crate::rendering::common::render_table_markdown(table)
}

/// Multiple of the page's average word height used as the vertical-gap
/// threshold for splitting table candidate words into separate regions
/// (#177). Normal row spacing inside one table rarely exceeds ~1.5x the
/// average word height, so a wider multiple avoids splitting a single
/// table's own row gaps while still separating genuinely distinct tables
/// (or a table from surrounding prose) on the same page.
//
// This module also compiles for `feature = "pdf"` (shared with the native PDF table path), but
// the only real callers of this constant -- `ocr::processor::execution` (gated `feature = "ocr"`)
// and `paddle_ocr::backend` (gated `paddle_ocr`) -- need neither `pdf` nor each other's gate. A
// gate as wide as the module's left this compiled with zero callers on a `pdf`-only leg. ~keep
#[cfg(any(feature = "ocr", paddle_ocr))]
pub(crate) const TABLE_REGION_GAP_HEIGHT_MULTIPLIER: u32 = 3;

/// Minimum number of words for a spatial region to be treated as a table
/// candidate. Mirrors the previous whole-page threshold so a single small
/// table on an otherwise text-only page is not over-fabricated.
//
// Same reasoning as `TABLE_REGION_GAP_HEIGHT_MULTIPLIER` above: real callers are gated `ocr` or
// `paddle_ocr`, not `pdf`. ~keep
#[cfg(any(feature = "ocr", paddle_ocr))]
pub(crate) const MIN_TABLE_CANDIDATE_WORDS: usize = 6;

/// Split table-candidate words into vertically separated regions.
///
/// Tesseract's TSV output has no notion of "this is a separate table from
/// that one" — [`reconstruct_table`] previously ran once over every
/// table-confidence word on the page, producing at most one table whose
/// bounding box spanned the union of all such words, even when the page had
/// several independent tables separated by paragraphs of prose (#177).
///
/// This groups words by contiguous vertical extent: a gap between one row's
/// bottom edge and the next word's top edge wider than
/// `TABLE_REGION_GAP_HEIGHT_MULTIPLIER` times the average word height starts
/// a new region. Each region is reconstructed independently, giving each
/// table its own bounding box.
///
// Same reasoning as `TABLE_REGION_GAP_HEIGHT_MULTIPLIER` above: the two real callers --
// `ocr::processor::execution::perform_ocr`'s table-detection branch (gated `feature = "ocr"`)
// and `PaddleOcrBackend::build_ocr_tables_from_words` (gated `paddle_ocr`) -- never need the
// wider `pdf` gate this module carries. ~keep
#[cfg(any(feature = "ocr", paddle_ocr))]
pub(crate) fn cluster_words_into_table_regions(words: &[HocrWord]) -> Vec<Vec<HocrWord>> {
    if words.is_empty() {
        return Vec::new();
    }

    let mut sorted: Vec<&HocrWord> = words.iter().collect();
    sorted.sort_by(|a, b| a.top.cmp(&b.top).then(a.left.cmp(&b.left)));

    let avg_height: u32 = {
        let total: u32 = sorted.iter().map(|w| w.height).sum();
        (total / sorted.len() as u32).max(1)
    };
    let region_gap_threshold = avg_height * TABLE_REGION_GAP_HEIGHT_MULTIPLIER;

    let mut regions: Vec<Vec<HocrWord>> = Vec::new();
    let mut current_region: Vec<HocrWord> = Vec::new();
    let mut current_bottom: u32 = 0;

    for word in sorted {
        let word_bottom = word.top + word.height;
        let is_new_region =
            !current_region.is_empty() && word.top.saturating_sub(current_bottom) > region_gap_threshold;

        if is_new_region {
            regions.push(std::mem::take(&mut current_region));
            current_bottom = 0;
        }

        current_bottom = current_bottom.max(word_bottom);
        current_region.push(word.clone());
    }
    if !current_region.is_empty() {
        regions.push(current_region);
    }

    regions
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a `HocrWord` with a fixed confidence, for tests that only care
    /// about position/text (matches the `word`/`make_word`/`hocr_word_at`
    /// helper convention used by the other files in this module's blast
    /// radius, e.g. `pdf::native::table` and `paddle_ocr::backend`).
    fn word(text: &str, left: u32, top: u32, width: u32, height: u32) -> HocrWord {
        HocrWord {
            text: text.to_string(),
            left,
            top,
            width,
            height,
            confidence: 95.0,
        }
    }

    #[test]
    fn test_detect_rows_zero_height_words_grouped_into_one_row() {
        let words = vec![
            HocrWord {
                text: "A".to_string(),
                left: 0,
                top: 10,
                width: 5,
                height: 0,
                confidence: 0.0,
            },
            HocrWord {
                text: "B".to_string(),
                left: 0,
                top: 10,
                width: 5,
                height: 0,
                confidence: 0.0,
            },
        ];
        let rows = detect_rows(&words, 0.5);
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn test_nan_safe_sort_does_not_panic() {
        let mut values: Vec<f64> = vec![1.0, f64::NAN, 2.0];
        values.sort_by(|a, b| a.total_cmp(b));
        assert_eq!(values.len(), 3);
        assert!(!values[0].is_nan());
        assert!(!values[1].is_nan());
        assert!(values[2].is_nan(), "NaN sorts last in ascending total_cmp order");
    }

    #[test]
    fn test_hocr_word_methods() {
        let word = HocrWord {
            text: "Hello".to_string(),
            left: 100,
            top: 50,
            width: 80,
            height: 30,
            confidence: 95.5,
        };

        assert_eq!(word.right(), 180);
        assert_eq!(word.bottom(), 80);
        assert_eq!(word.y_center(), 65.0);
        assert_eq!(word.x_center(), 140.0);
    }

    #[test]
    fn test_detect_columns() {
        let words = vec![
            HocrWord {
                text: "A".to_string(),
                left: 100,
                top: 50,
                width: 20,
                height: 30,
                confidence: 95.0,
            },
            HocrWord {
                text: "B".to_string(),
                left: 300,
                top: 50,
                width: 20,
                height: 30,
                confidence: 95.0,
            },
            HocrWord {
                text: "C".to_string(),
                left: 105,
                top: 100,
                width: 20,
                height: 30,
                confidence: 95.0,
            },
            HocrWord {
                text: "D".to_string(),
                left: 295,
                top: 100,
                width: 20,
                height: 30,
                confidence: 95.0,
            },
        ];

        let cols = detect_columns(&words, 20);
        assert_eq!(cols.len(), 2);
    }

    #[test]
    fn test_detect_rows() {
        let words = vec![
            HocrWord {
                text: "A".to_string(),
                left: 100,
                top: 50,
                width: 20,
                height: 30,
                confidence: 95.0,
            },
            HocrWord {
                text: "B".to_string(),
                left: 200,
                top: 52,
                width: 20,
                height: 30,
                confidence: 95.0,
            },
            HocrWord {
                text: "C".to_string(),
                left: 100,
                top: 100,
                width: 20,
                height: 30,
                confidence: 95.0,
            },
        ];

        let rows = detect_rows(&words, 0.5);
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn test_reconstruct_table_basic() {
        let words = vec![
            HocrWord {
                text: "Name".to_string(),
                left: 100,
                top: 50,
                width: 40,
                height: 20,
                confidence: 95.0,
            },
            HocrWord {
                text: "Value".to_string(),
                left: 300,
                top: 50,
                width: 40,
                height: 20,
                confidence: 95.0,
            },
            HocrWord {
                text: "Alice".to_string(),
                left: 100,
                top: 100,
                width: 40,
                height: 20,
                confidence: 95.0,
            },
            HocrWord {
                text: "42".to_string(),
                left: 300,
                top: 100,
                width: 20,
                height: 20,
                confidence: 95.0,
            },
        ];

        let table = reconstruct_table(&words, 20, 0.5);
        assert_eq!(table.len(), 2);
        assert_eq!(table[0].len(), 2);
        assert_eq!(table[0][0], "Name");
        assert_eq!(table[0][1], "Value");
        assert_eq!(table[1][0], "Alice");
        assert_eq!(table[1][1], "42");
    }

    #[test]
    fn test_table_to_markdown_basic() {
        let table = vec![
            vec!["Name".to_string(), "Value".to_string()],
            vec!["Alice".to_string(), "42".to_string()],
        ];

        let md = table_to_markdown(&table);
        assert!(md.contains("| Name | Value |"));
        assert!(md.contains("| --- | --- |"));
        assert!(md.contains("| Alice | 42 |"));
    }

    #[test]
    fn test_table_to_markdown_empty() {
        assert_eq!(table_to_markdown(&[]), String::new());
    }

    #[test]
    fn test_table_to_markdown_escapes_pipes() {
        let table = vec![vec!["Header".to_string()], vec!["a|b".to_string()]];

        let md = table_to_markdown(&table);
        assert!(md.contains("a\\|b"));
    }

    /// Regression test for issue where intra-cell word spacing ("Chose 1")
    /// was incorrectly split into separate columns.
    /// The word "1" should stay in the same cell as "Chose" despite having
    /// a different left position, because they're separated by a small gap.
    #[test]
    fn test_reconstruct_table_intra_cell_word_spacing() {
        let words = vec![
            HocrWord {
                text: "Chose".to_string(),
                left: 57,
                top: 496,
                width: 30,
                height: 12,
                confidence: 95.0,
            },
            HocrWord {
                text: "Truc".to_string(),
                left: 306,
                top: 496,
                width: 23,
                height: 12,
                confidence: 95.0,
            },
            HocrWord {
                text: "Chose".to_string(),
                left: 57,
                top: 510,
                width: 28,
                height: 12,
                confidence: 95.0,
            },
            HocrWord {
                text: "1".to_string(),
                left: 90,
                top: 510,
                width: 6,
                height: 12,
                confidence: 95.0,
            },
            HocrWord {
                text: "Truc".to_string(),
                left: 306,
                top: 510,
                width: 21,
                height: 12,
                confidence: 95.0,
            },
            HocrWord {
                text: "1".to_string(),
                left: 332,
                top: 510,
                width: 5,
                height: 12,
                confidence: 95.0,
            },
            HocrWord {
                text: "Chose".to_string(),
                left: 57,
                top: 524,
                width: 28,
                height: 12,
                confidence: 95.0,
            },
            HocrWord {
                text: "2".to_string(),
                left: 90,
                top: 524,
                width: 6,
                height: 12,
                confidence: 95.0,
            },
            HocrWord {
                text: "Truc".to_string(),
                left: 306,
                top: 524,
                width: 21,
                height: 12,
                confidence: 95.0,
            },
            HocrWord {
                text: "2".to_string(),
                left: 332,
                top: 524,
                width: 5,
                height: 12,
                confidence: 95.0,
            },
        ];

        let table = reconstruct_table(&words, 60, 0.5);

        assert_eq!(table.len(), 3, "Expected 3 rows, got {}", table.len());
        assert_eq!(table[0].len(), 2, "Expected 2 columns in row 0, got {}", table[0].len());

        assert_eq!(table[0][0], "Chose", "Header row 1, col 1");
        assert_eq!(table[0][1], "Truc", "Header row 1, col 2");
        assert_eq!(table[1][0], "Chose 1", "Row 2, col 1 should contain merged text");
        assert_eq!(table[1][1], "Truc 1", "Row 2, col 2 should contain merged text");
        assert_eq!(table[2][0], "Chose 2", "Row 3, col 1 should contain merged text");
        assert_eq!(table[2][1], "Truc 2", "Row 3, col 2 should contain merged text");
    }

    /// Regression/verification test for the claim in xberg-io/xberg#183 that
    /// `reconstruct_table` "drops any word for which `find_row_index` or
    /// `find_column_index` returns `None`".
    ///
    /// That claim does not hold for the current implementation: both
    /// `find_row_index` and `find_column_index` resolve to the *nearest*
    /// row/column via `min_by_key`, which is `Some` for every word whenever
    /// `row_positions`/`col_positions` are non-empty — and `reconstruct_table`
    /// already early-returns before this loop if either is empty. There is no
    /// path through this function that silently discards a word; even a word
    /// wildly outside the detected column/row bands is force-assigned to its
    /// nearest band instead of being dropped. This test proves that with an
    /// outlier word far outside the main table extent: every input word is
    /// still present, exactly once, in the reconstructed table.
    #[test]
    fn test_reconstruct_table_never_drops_words_including_far_outliers() {
        let mut words = vec![
            HocrWord {
                text: "A1".to_string(),
                left: 0,
                top: 0,
                width: 10,
                height: 10,
                confidence: 95.0,
            },
            HocrWord {
                text: "B1".to_string(),
                left: 200,
                top: 0,
                width: 10,
                height: 10,
                confidence: 95.0,
            },
            HocrWord {
                text: "A2".to_string(),
                left: 0,
                top: 200,
                width: 10,
                height: 10,
                confidence: 95.0,
            },
            HocrWord {
                text: "B2".to_string(),
                left: 200,
                top: 200,
                width: 10,
                height: 10,
                confidence: 95.0,
            },
        ];
        // Far outside every detected row/column band and every other word's
        // neighborhood — the scenario #183 claims gets silently dropped.
        words.push(HocrWord {
            text: "Outlier".to_string(),
            left: 50_000,
            top: 50_000,
            width: 10,
            height: 10,
            confidence: 95.0,
        });

        let input_word_count = words.len();
        let table = reconstruct_table(&words, 20, 0.5);

        let output_word_count: usize = table
            .iter()
            .flat_map(|row| row.iter())
            .flat_map(|cell| cell.split_whitespace())
            .count();

        assert_eq!(
            output_word_count, input_word_count,
            "every input word (including the far outlier) must appear exactly once in the output; \
             reconstruct_table's nearest-row/nearest-column assignment never returns None here"
        );

        let all_text: Vec<&str> = table
            .iter()
            .flat_map(|row| row.iter())
            .flat_map(|c| c.split_whitespace())
            .collect();
        assert!(
            all_text.contains(&"Outlier"),
            "the outlier word must not be silently dropped"
        );
    }

    /// Regression test for xberg-io/xberg#688: a table cell containing more
    /// than one word ("Alice Smith") must not mint a spurious extra column
    /// from its second word's x-position.
    ///
    /// TEST HONESTY: without the merge-before-column-detection fix, this
    /// fails at the `table[0].len() == 2` assertion — `detect_columns` sees
    /// "Alice"@100 and "Smith"@145 as two distinct groups (gap 45 > the
    /// column_threshold of 20), producing 3 columns instead of 2. `table[0]`
    /// comes out as `["Name", "", "Value"]` (len 3) instead of `["Name",
    /// "Value"]`.
    #[test]
    fn test_reconstruct_table_multiword_cell_no_spurious_column() {
        let words = vec![
            word("Name", 100, 50, 40, 20),
            word("Value", 300, 50, 40, 20),
            word("Alice", 100, 100, 40, 20),
            word("Smith", 145, 100, 30, 20),
            word("42", 300, 100, 20, 20),
        ];

        let table = reconstruct_table(&words, 20, 0.5);

        assert_eq!(table.len(), 2);
        assert_eq!(
            table[0].len(),
            2,
            "expected 2 columns; without the fix 'Smith' (x=145) mints a spurious 3rd column"
        );
        assert_eq!(table[0], vec!["Name".to_string(), "Value".to_string()]);
        assert_eq!(table[1], vec!["Alice Smith".to_string(), "42".to_string()]);
    }

    /// A genuinely separate, closely-spaced column pair ("Wid"/"Zone" at
    /// x=150 and x=180, gap 15px) must survive intact even once a multi-word
    /// cell elsewhere in the same table ("Foo"+"Bar", gap 5px) gets merged.
    ///
    /// TEST HONESTY: without the fix, "Bar" (x=25) mints its own spurious
    /// column, producing 4 columns total ([0, 25, 150, 180] once sorted).
    /// `table[0]` comes out as `["Name", "", "Wid", "Zone"]` (len 4) instead
    /// of `["Name", "Wid", "Zone"]` (len 3), and `table[1]` as `["Foo",
    /// "Bar", "Val1", "Val2"]` instead of `["Foo Bar", "Val1", "Val2"]`.
    #[test]
    fn test_reconstruct_table_close_but_distinct_columns_not_merged() {
        let words = vec![
            word("Name", 0, 0, 20, 20),
            word("Wid", 150, 0, 15, 20),
            word("Zone", 180, 0, 20, 20),
            word("Foo", 0, 40, 20, 20),
            word("Bar", 25, 40, 15, 20),
            word("Val1", 150, 40, 15, 20),
            word("Val2", 180, 40, 20, 20),
        ];

        let table = reconstruct_table(&words, 10, 0.5);

        assert_eq!(table.len(), 2);
        assert_eq!(
            table[0].len(),
            3,
            "Wid/Zone must stay 2 distinct columns, not collapsed by the Foo/Bar merge fix"
        );
        assert_eq!(
            table[0],
            vec!["Name".to_string(), "Wid".to_string(), "Zone".to_string()]
        );
        assert_eq!(
            table[1],
            vec!["Foo Bar".to_string(), "Val1".to_string(), "Val2".to_string()],
            "Foo+Bar merge into one cell; Val1 and Val2 remain distinct cells, not merged together"
        );
    }

    /// Column count must be stable across different `column_threshold`
    /// values once multi-word cells are merged before column detection.
    ///
    /// TEST HONESTY: without the fix, threshold 10 yields 4 columns
    /// ([0, 15, 30, 300], "A"/"B"/"C" each minting a separate column) while
    /// threshold 30 over-merges the spurious columns into the real ones,
    /// yielding only 2 columns ([15, 300]) — i.e. 4 != 2, so the equality
    /// assertion below fails on unfixed code even though 2 happens to match
    /// one of the two unfixed outputs by coincidence.
    #[test]
    fn test_reconstruct_table_column_count_stable_across_thresholds() {
        let words = vec![
            word("H1", 0, 0, 20, 20),
            word("H2", 300, 0, 30, 20),
            word("A", 0, 40, 10, 20),
            word("B", 15, 40, 10, 20),
            word("C", 30, 40, 10, 20),
            word("Val", 300, 40, 20, 20),
        ];

        let table_tight = reconstruct_table(&words, 10, 0.5);
        let table_loose = reconstruct_table(&words, 30, 0.5);

        assert_eq!(table_tight[0].len(), 2);
        assert_eq!(
            table_tight[0].len(),
            table_loose[0].len(),
            "column count must not depend on column_threshold once cells are pre-merged"
        );
        assert_eq!(table_tight, table_loose);
        assert_eq!(table_tight[0], vec!["H1".to_string(), "H2".to_string()]);
        assert_eq!(table_tight[1], vec!["A B C".to_string(), "Val".to_string()]);
    }

    /// Degenerate-input guard: empty input. This early return predates the
    /// fix and behaves identically before and after it — included for
    /// completeness of degenerate-input coverage, not as a fix-detecting
    /// regression test (there is nothing for the fix to change here).
    #[test]
    fn test_reconstruct_table_empty_words_returns_empty_table() {
        let table = reconstruct_table(&[], 20, 0.5);
        assert!(table.is_empty());
    }

    /// Degenerate-input guard: a single word can never trigger the merge
    /// step (nothing to merge with), so this is identical before and after
    /// the fix. It exists to prove `merge_words_into_cell_tokens`'s
    /// `words.len() <= 1` guard doesn't panic or drop the word.
    #[test]
    fn test_reconstruct_table_single_word_no_panic() {
        let words = vec![word("Solo", 10, 10, 20, 20)];
        let table = reconstruct_table(&words, 20, 0.5);
        assert_eq!(table, vec![vec!["Solo".to_string()]]);
    }

    /// Degenerate-input guard: zero-height words drive the merge-gap
    /// threshold to exactly 0 (`0.6 * 0`), so only touching/overlapping
    /// words (gap <= 0) merge, and the computation must not panic (no
    /// division by a zero median height, no overflow in the saturating
    /// bbox-union arithmetic).
    ///
    /// TEST HONESTY: without the fix, "X" (x=0..10) and "Y" (x=10..20) —
    /// which touch with a zero-pixel gap — are still 2 separate columns
    /// under raw `detect_columns` (their left edges differ by 10, over the
    /// column_threshold of 5), giving 3 columns (`["X", "Y", "Z"]`) instead
    /// of the correct 2 (`["X Y", "Z"]`).
    #[test]
    fn test_reconstruct_table_zero_height_words_merge_on_touching_gap() {
        let words = vec![word("X", 0, 0, 10, 0), word("Y", 10, 0, 10, 0), word("Z", 50, 0, 10, 0)];

        let table = reconstruct_table(&words, 5, 0.5);

        assert_eq!(table.len(), 1);
        assert_eq!(
            table[0].len(),
            2,
            "X and Y touch with a 0px gap and must merge into one cell"
        );
        assert_eq!(table[0], vec!["X Y".to_string(), "Z".to_string()]);
    }

    /// GH#1628: a cell's words must read left-to-right, not in the order they happened to
    /// arrive in `words`.
    ///
    /// A sub/superscript is drawn as its own content-stream segment sitting a fraction of a point
    /// below the line it annotates, so every reading-order sort upstream of here places it after
    /// the whole line rather than beside the symbol it belongs to. By the time words reach this
    /// function the subscript's `left` still says exactly where it goes --- `eta`(206),
    /// `S`(211), `%`(253) --- but slice order says `eta`, `%`, `S`, and the join followed slice
    /// order. The reported output was `E % S` for `eta_S %` and `Q GJ HE` for `Q_HE GJ`.
    ///
    /// Left ordering is the correct rule here regardless of subscripts: every word in one cell
    /// sits in the same row band by construction (`find_row_index` put it there), so `left` is
    /// reading order for that cell.
    #[test]
    fn issue_1628_cell_words_join_in_left_to_right_order() {
        let words = vec![
            word("eta", 206, 100, 7, 12),
            word("%", 253, 100, 6, 12),
            word("S", 211, 104, 3, 7),
        ];

        let table = assign_words_to_cells(&words, &[100], &[206]);

        assert_eq!(
            table,
            vec![vec!["eta S %".to_string()]],
            "a subscript must join between the symbol it annotates and the next unit, not after it"
        );
    }

    /// GH#1628 negative control: a cell holding two visual lines must NOT interleave them.
    ///
    /// This is the failure mode a naive left-only sort introduces, and it is strictly worse than
    /// the bug being fixed: it scrambles every wrapped cell in the corpus, not just cells with
    /// sub/superscripts. Line grouping is what separates the two cases --- a subscript sits a
    /// fraction of its own height off its base's centre, a second line sits a full line height
    /// away.
    #[test]
    fn issue_1628_cell_with_two_visual_lines_does_not_interleave_them() {
        let words = vec![
            word("Hello", 100, 100, 30, 12),
            word("world", 140, 100, 30, 12),
            word("again", 100, 116, 30, 12),
            word("here", 140, 116, 30, 12),
        ];

        let table = assign_words_to_cells(&words, &[106], &[100]);

        assert_eq!(table, vec![vec!["Hello world again here".to_string()]]);
    }

    /// GH#1628: a segment whose text holds no whitespace is returned unsplit, so its word keeps
    /// the segment's own trailing space. Joining those words then stacks that space on the
    /// separator and renders `Q_HE GJ` as `Q  HE GJ`. Each word's own whitespace must collapse so
    /// the join alone owns the spacing.
    #[test]
    fn issue_1628_cell_join_does_not_stack_a_word_s_own_trailing_space() {
        let words = vec![word("Q ", 206, 100, 8, 12), word("HE", 213, 104, 7, 7)];

        let table = assign_words_to_cells(&words, &[100], &[206]);

        assert_eq!(table, vec![vec!["Q HE".to_string()]]);
    }

    /// Degenerate-input guard: every word on one row is a degenerate case
    /// for the per-row bucketing inside `merge_words_into_cell_tokens` (a
    /// single bucket holding every word) — must not panic and must still
    /// merge/split correctly within that one row.
    ///
    /// TEST HONESTY: without the fix, "A" (x=0) and "B" (x=15) are 2
    /// separate columns under raw `detect_columns` (gap 15 > the
    /// column_threshold of 5), giving 3 columns (`["A", "B", "C"]`) instead
    /// of the correct 2 (`["A B", "C"]`).
    #[test]
    fn test_reconstruct_table_all_words_one_row() {
        let words = vec![
            word("A", 0, 0, 10, 10),
            word("B", 15, 0, 10, 10),
            word("C", 100, 0, 10, 10),
        ];

        let table = reconstruct_table(&words, 5, 0.5);

        assert_eq!(table.len(), 1);
        assert_eq!(table[0].len(), 2);
        assert_eq!(table[0], vec!["A B".to_string(), "C".to_string()]);
    }

    /// `reconstruct_table_with_columns` must return column positions that
    /// stay index-correlated with the returned grid: `positions[i]`
    /// corresponds to `grid[row][i]` for every row. Callers like
    /// `pdf::native::table::reconstruct_region_table_with_column_gap` rely on
    /// this to index `column_positions[column]` against `grid[row][column]`
    /// (xberg-io/xberg#688 blast-radius fix: `reconstruct_table` now detects
    /// columns from post-merge cell tokens, so a caller's own separate
    /// `detect_columns(words, ...)` call no longer corresponds to the grid
    /// `reconstruct_table` returns — this sibling function is how such a
    /// caller gets a mutually consistent pair instead).
    ///
    /// Note: row/column assignment always resolves to the nearest position
    /// (xberg-io/xberg#183), and every column detect_columns reports is the
    /// exact position of one of its member tokens, so that token's word
    /// always maps back to its own column at distance 0. In practice this
    /// means a detect_columns-derived column can never end up entirely
    /// empty across all rows, so remove_empty_rows_and_columns cannot
    /// actually strip a column reachable through this path. This test
    /// therefore checks the correspondence invariant directly rather than
    /// depending on triggering that specific removal branch.
    #[test]
    fn test_reconstruct_table_with_columns_positions_correlate_with_grid() {
        let words = vec![
            word("Left", 0, 0, 20, 20),
            word("Right", 300, 0, 20, 20),
            word("L2", 0, 40, 20, 20),
            word("R2", 300, 40, 20, 20),
        ];
        let (grid, positions) = reconstruct_table_with_columns(&words, 20, 0.5);

        assert!(!grid.is_empty());
        assert_eq!(
            positions.len(),
            grid[0].len(),
            "column_positions must have exactly one entry per surviving grid column"
        );
        assert_eq!(positions, vec![0, 300]);
        assert_eq!(grid[0], vec!["Left".to_string(), "Right".to_string()]);
        assert_eq!(grid[1], vec!["L2".to_string(), "R2".to_string()]);
    }
}
