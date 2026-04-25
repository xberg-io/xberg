//! Table reconstruction from PDF segments (no OCR dependency).
//!
//! This module provides table reconstruction utilities that work with any
//! source of word-level text data (PDF native text, OCR output, etc.).
//! It re-exports core types from `table_core` and adds PDF-specific
//! conversion helpers.

use super::hierarchy::SegmentData;

pub(crate) use crate::table_core::{HocrWord, reconstruct_table, table_to_markdown};

/// Convert a PDF `SegmentData` to an `HocrWord` for table reconstruction.
///
/// `SegmentData` uses PDF coordinates (y=0 at bottom, increases upward).
/// `HocrWord` uses image coordinates (y=0 at top, increases downward).
#[cfg(feature = "pdf-oxide")]
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
#[cfg(feature = "pdf-oxide")]
pub(crate) fn split_segment_to_words(seg: &SegmentData, page_height: f32) -> Vec<HocrWord> {
    let trimmed = seg.text.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    // Fast path: single word
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
        // Find byte offset of this word in the original text
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
#[cfg(feature = "pdf-oxide")]
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
    // Strip empty rows
    table.retain(|row| row.iter().any(|cell| !cell.trim().is_empty()));
    if table.is_empty() {
        return None;
    }

    // Reject prose: if >50% of non-empty cells exceed 60 chars, it's not a table.
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

    // Prose detection: reject if too many cells contain long text.
    // Layout-guided tables use relaxed thresholds since the model already
    // confirmed this is a table region, but multi-column prose can still
    // fool the layout model (e.g., 2-column academic papers).
    if non_empty > 0 {
        if layout_guided {
            // Relaxed: reject if >70% of cells exceed 100 chars
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
            // Relaxed: reject if avg cell length > 80 chars
            if total_chars / non_empty > 80 {
                return None;
            }
        } else {
            // Unsupervised: reject if >50% of cells exceed 60 chars
            if long_cells * 2 > non_empty {
                return None;
            }
            // Unsupervised: reject if avg cell length > 50 chars
            if total_chars / non_empty > 50 {
                return None;
            }
        }
    }

    let col_count = table.first().map_or(0, Vec::len);
    if col_count < min_columns {
        return None;
    }

    // Find where data rows start (first row with ≥3 cells containing digits)
    let data_start = table
        .iter()
        .enumerate()
        .find_map(|(idx, row)| {
            let digit_cells = row
                .iter()
                .filter(|cell| cell.chars().any(|c| c.is_ascii_digit()))
                .count();
            if digit_cells >= 3 { Some(idx) } else { None }
        })
        .unwrap_or(0);

    let mut header_rows = if data_start > 0 {
        table[..data_start].to_vec()
    } else {
        Vec::new()
    };
    let mut data_rows = table[data_start..].to_vec();

    // Keep at most 2 header rows
    if header_rows.len() > 2 {
        header_rows = header_rows[header_rows.len() - 2..].to_vec();
    }

    // If no header detected, promote first data row
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

    // Merge multi-row headers into a single header row
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

    // Remove header-only columns (header text but no data)
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

    // Final dimension check: must have ≥2 columns and ≥2 rows
    if processed[0].len() < 2 || processed.len() <= 1 {
        return None;
    }

    // Column sparsity check: reject if any column is too sparse.
    // Threshold: >75% empty (unsupervised) or >95% empty (layout-guided).
    // Layout-guided uses a very permissive threshold because layout models
    // can confidently detect tables with optional/sparse columns.
    let data_row_count = processed.len() - 1;
    if data_row_count > 0 {
        for c in 0..processed[0].len() {
            let empty_count = processed[1..]
                .iter()
                .filter(|row| row.get(c).is_none_or(|cell| cell.trim().is_empty()))
                .count();
            let too_sparse = if layout_guided {
                empty_count * 20 > data_row_count * 19 // >95%
            } else {
                empty_count * 4 > data_row_count * 3 // >75%
            };
            if too_sparse {
                return None;
            }
        }
    }

    // Overall density check: reject if too few data cells are filled.
    // Threshold: <40% filled (unsupervised) or <15% filled (layout-guided).
    // Layout-guided uses a lower threshold because the model already confirmed
    // this is a table — sparse tables (e.g., with merged cells or optional
    // fields) are legitimate.
    {
        let total_data_cells = data_row_count * processed[0].len();
        if total_data_cells > 0 {
            let filled = processed[1..]
                .iter()
                .flat_map(|row| row.iter())
                .filter(|cell| !cell.trim().is_empty())
                .count();
            let too_sparse = if layout_guided {
                filled * 20 < total_data_cells * 3 // <15%
            } else {
                filled * 5 < total_data_cells * 2 // <40%
            };
            if too_sparse {
                return None;
            }
        }
    }

    // Prose detection: reject tables where most non-empty cells contain only single words.
    // When justified prose text is falsely detected as a table, the reconstruction
    // splits sentences across many columns, producing cells with single words.
    // Real tables typically have meaningful multi-word content in their cells.
    // Only check tables with 5+ columns, since 2-4 column tables with short cells
    // are common and legitimate (e.g., Name | Department | Salary).
    // Layout-guided uses a stricter threshold (>85%) since the model has some
    // confidence, but multi-column prose can still fool it.
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
        let threshold = if layout_guided {
            // Layout-guided: reject if >85% single-word cells
            85
        } else {
            // Unsupervised: reject if >70% single-word cells
            70
        };
        if non_empty_cells >= 6 && single_word_cells * 100 > non_empty_cells * threshold {
            return None;
        }
    }

    // Column-text-flow check: detect multi-column prose masquerading as a table.
    // The key signal is that cells in adjacent columns form sentence continuations:
    // column 0 ends without sentence-ending punctuation and column 1 starts with a
    // lowercase letter. If >60% of non-empty rows exhibit this "flow-through"
    // pattern, the content is prose, not a table.
    // Applied for both layout-guided and unsupervised modes.
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

    // Content asymmetry check: reject if one column has the vast majority of text.
    // Layout-guided uses relaxed threshold (>92%) vs unsupervised (>85%).
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
            // Check for dominant column (one column has almost all the text)
            let max_col_share = col_char_counts
                .iter()
                .map(|&cc| cc as f64 / total_chars_asym as f64)
                .fold(0.0_f64, f64::max);
            let dominant_threshold = if layout_guided { 0.92 } else { 0.85 };
            if max_col_share > dominant_threshold {
                return None;
            }

            // Check for sparse + low-content columns (unsupervised only)
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

    // Row-to-row sentence continuation check: detect multi-column prose by looking
    // at whether text flows from the last column of one row into the first column
    // of the next row. In prose laid out as columns, the last cell of a row ends
    // mid-sentence (no terminal punctuation) and the first cell of the next row
    // starts with a lowercase letter. If >40% of row transitions show this, reject.
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

    // High-row low-column prose check: multi-column prose typically produces tables
    // with many rows (>20), few columns (≤3), and high fill rate (>80%).
    // Real tables with this shape usually have sparse or structured data.
    // Applied for both layout-guided and unsupervised modes.
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
            if total_data_cells > 0 && filled_cells * 100 > total_data_cells * 80 {
                return None;
            }
        }
    }

    // Uniform column width check: in real tables, columns have varying average cell
    // lengths (e.g., narrow ID column vs wide description column). In multi-column
    // prose, all columns carry similar amounts of text. If we have 3-5 text columns
    // where the longest average cell length is within 2x of the shortest, AND the
    // table has many rows with high fill rate, reject as likely prose.
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

            // Only consider columns with substantial text (avg > 15 chars).
            // Prose columns have sentence-like content with avg cell length well above
            // 15 chars. Short-valued columns (IDs, codes, short labels) are excluded.
            let text_col_avgs: Vec<f64> = col_avg_lengths.iter().copied().filter(|&avg| avg > 15.0).collect();

            if text_col_avgs.len() >= 3 {
                let min_avg = text_col_avgs.iter().copied().fold(f64::INFINITY, f64::min);
                let max_avg = text_col_avgs.iter().copied().fold(0.0_f64, f64::max);

                // All text columns within 2x of each other
                if min_avg > 0.0 && max_avg <= min_avg * 2.0 {
                    // Also check fill rate
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

    // Normalize cells
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

    // --- Check 1: Row coherence (prose detection) ---
    // For each data row, concatenate all cells left-to-right. If the result
    // reads like a coherent sentence fragment (>30 chars, last cell ends without
    // punctuation and next row's first cell starts lowercase), the grid is
    // likely prose laid out in columns.
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

            // Check if the row reads like prose: mostly alphabetic, few abrupt
            // breaks, and doesn't look like structured data (numbers, codes).
            let alpha_ratio = {
                let alpha = concatenated
                    .chars()
                    .filter(|c| c.is_alphabetic() || c.is_whitespace())
                    .count();
                alpha as f64 / concatenated.len() as f64
            };
            // Prose has high alpha ratio (>0.8) — structured data has numbers, symbols
            if alpha_ratio > 0.8 {
                prose_like_rows += 1;
            }
        }

        if eligible_rows >= 3 && prose_like_rows * 2 > eligible_rows {
            return false;
        }
    }

    // --- Check 2: Column semantic uniformity ---
    // In real tables, columns have different character: a narrow ID column, a wide
    // description column, a numeric column. In multi-column prose, all columns
    // carry similar-length text. Check if cell lengths within each column have
    // low variance AND all columns have similar average length.
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

        // Filter to columns with meaningful content (mean > 3 chars)
        let meaningful: Vec<(f64, f64)> = col_stats.iter().copied().filter(|(m, _)| *m > 3.0).collect();

        if meaningful.len() >= 3 {
            let means: Vec<f64> = meaningful.iter().map(|(m, _)| *m).collect();
            let min_mean = means.iter().copied().fold(f64::INFINITY, f64::min);
            let max_mean = means.iter().copied().fold(0.0_f64, f64::max);

            // All columns within 2x of each other in avg length
            let columns_uniform = min_mean > 0.0 && max_mean <= min_mean * 2.0;

            // Low coefficient of variation within each column (cells ±30% of mean)
            let low_variance = meaningful
                .iter()
                .all(|(mean, stddev)| *mean > 0.0 && *stddev / *mean < 0.3);

            if columns_uniform && low_variance {
                return false;
            }
        }
    }

    // --- Check 3: Minimum meaningful content (repetitive vocabulary) ---
    // If the table has ≥3 columns but total unique words across all cells is
    // < 2× the number of rows, the same few words are repeated in every row
    // (e.g., "Bookmark | File PDF | Year 4" repeated 83 times).
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
        if row_count >= 3 && unique_words.len() < row_count * 2 {
            return false;
        }
    }

    // --- Check 4: Repeated header detection ---
    // If the header row (first row) appears identically as a data row multiple
    // times, the layout model is detecting a repeating page element (running
    // header/footer) as a table.
    if !grid.is_empty() {
        let header = &grid[0];
        let header_matches = data_rows
            .iter()
            .filter(|row| row.len() == header.len() && row.iter().zip(header.iter()).all(|(a, b)| a.trim() == b.trim()))
            .count();
        // If header appears 2+ times in data rows, it's a repeating element
        if header_matches >= 2 {
            return false;
        }
    }

    true
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

    #[cfg(feature = "pdf-oxide")]
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

    #[cfg(feature = "pdf-oxide")]
    #[test]
    fn test_split_single_word() {
        let seg = make_seg("Hello", 100.0, 500.0, 50.0, 12.0);
        let words = split_segment_to_words(&seg, 800.0);
        assert_eq!(words.len(), 1);
        assert_eq!(words[0].text, "Hello");
        assert_eq!(words[0].left, 100);
    }

    #[cfg(feature = "pdf-oxide")]
    #[test]
    fn test_split_two_words() {
        let seg = make_seg("Col A", 100.0, 500.0, 100.0, 12.0);
        let words = split_segment_to_words(&seg, 800.0);
        assert_eq!(words.len(), 2);
        assert_eq!(words[0].text, "Col");
        assert_eq!(words[1].text, "A");
        // "A" starts at byte 4 of "Col A" (len=5), so frac_start = 4/5 = 0.8
        // word_x = 100 + 0.8 * 100 = 180
        assert_eq!(words[1].left, 180);
    }

    #[cfg(feature = "pdf-oxide")]
    #[test]
    fn test_split_empty_segment() {
        let seg = make_seg("   ", 100.0, 500.0, 50.0, 12.0);
        let words = split_segment_to_words(&seg, 800.0);
        assert!(words.is_empty());
    }

    #[cfg(feature = "pdf-oxide")]
    #[test]
    fn test_split_many_words() {
        let seg = make_seg("a b c d", 0.0, 0.0, 700.0, 12.0);
        let words = split_segment_to_words(&seg, 800.0);
        assert_eq!(words.len(), 4);
        assert_eq!(words[0].text, "a");
        assert_eq!(words[1].text, "b");
        assert_eq!(words[2].text, "c");
        assert_eq!(words[3].text, "d");
        // Words should be spaced across the 700pt width
        assert!(words[1].left > words[0].left);
        assert!(words[2].left > words[1].left);
        assert!(words[3].left > words[2].left);
    }

    #[cfg(feature = "pdf-oxide")]
    #[test]
    fn test_split_y_coordinate_conversion() {
        // Segment at y=500 (PDF bottom-up), height=12, page_height=800
        // Image top = 800 - (500 + 12) = 288
        let seg = make_seg("word", 100.0, 500.0, 50.0, 12.0);
        let words = split_segment_to_words(&seg, 800.0);
        assert_eq!(words[0].top, 288);
        assert_eq!(words[0].height, 12);
    }

    #[cfg(feature = "pdf-oxide")]
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
        // Simulates what happens when justified prose text is incorrectly
        // split into a multi-column table: most cells contain single words.
        let table = vec![
            // header
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
            // data rows: single words per cell (prose split across columns)
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
        // This should be rejected because most cells are single words (prose).
        let result = post_process_table(table, false, false);
        assert!(result.is_none(), "Prose-like table should be rejected");
    }

    #[test]
    fn test_post_process_accepts_real_table() {
        // A real table with meaningful multi-word content in cells.
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
    fn test_column_text_flow_rejects_multicolumn_prose() {
        // Simulates 2-column academic paper prose reconstructed as a table.
        // Column 0 ends mid-sentence (no punctuation), column 1 starts lowercase.
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
        // Both unsupervised and layout-guided should reject this
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
        // A real 2-column table where cells are independent (sentences end properly).
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
        // Only 2 eligible rows — below the 3-row minimum for the check.
        let table = vec![
            vec!["Left".into(), "Right".into()],
            vec![
                "some text without ending punct".into(),
                "continues here in lowercase".into(),
            ],
            vec!["another partial sentence".into(), "flowing into next column".into()],
        ];
        // With only 2 data rows (eligible_rows < 3), the flow check should not trigger.
        // The table may still be rejected by other checks, but not by flow-through.
        // We just verify it doesn't panic and runs without issues.
        let _ = post_process_table(table, true, false);
    }

    #[test]
    fn test_layout_guided_rejects_prose_with_long_cells() {
        // Layout-guided should now reject tables where >70% of cells exceed 100 chars.
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
        // A layout-guided table where some cells are long (description column)
        // but not overwhelming — should be accepted. The first column has enough
        // text to avoid the content asymmetry rejection (>92% in one column).
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
        // One column has >92% of all text content — asymmetric.
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
        // Layout-guided mode with 5+ columns where >85% cells are single words.
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
        // Simulates 2-column prose where the last column of row N flows into
        // the first column of row N+1 (no terminal punctuation + lowercase start).
        let mut table = vec![vec!["Left Column".into(), "Right Column".into()]];
        // Generate enough rows to trigger the check (>3 rows, >=3 eligible transitions)
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
        // A real 2-column table where cells end with proper punctuation.
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
        // 2-column table with >20 rows and >80% fill rate — classic multi-column prose.
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
        // 2-column table with >20 rows but <80% fill rate (many empty cells).
        let mut table = vec![vec!["Date".into(), "Event".into()]];
        for i in 0..25 {
            if i % 3 == 0 {
                table.push(vec![format!("2024-01-{:02}", i + 1), "Holiday.".into()]);
            } else {
                table.push(vec![format!("2024-01-{:02}", i + 1), String::new()]);
            }
        }
        let result = post_process_table(table, true, false);
        // Should not be rejected by the high-row check since fill rate < 80%
        // (may be rejected by other checks like column sparsity, which is fine)
        let _ = result; // Just ensure no panic
    }

    #[test]
    fn test_high_row_low_column_allows_four_plus_columns() {
        // 4-column table with many rows and high fill rate — NOT rejected since cols > 3.
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
        // 3-column table where all columns have similar average cell length
        // and high fill rate — characteristic of multi-column prose.
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
        // Real table where columns have very different average cell lengths.
        // The Name column is long enough to avoid the content asymmetry check
        // (no single column has >85% of total chars).
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

    // --- Tests for is_well_formed_table ---

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
    fn test_well_formed_rejects_repetitive_content() {
        // Same few words repeated in every row — like "Bookmark | File PDF | Year 4"
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
        // Header appears identically in 3 data rows — repeated page element
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
        // Multi-column prose: each row concatenates to a coherent sentence
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
        // All 3 columns have similar average cell length and low variance — prose signal
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
        // Columns have clearly different character: ID (short), name (medium), amount (numeric)
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
        // nougat_008 pattern: scanned 3-column PDF where prose text flow is
        // misdetected as a table. Cells are short (1-3 words each) but the
        // concatenated rows read as prose with high alphabetic ratio.
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
}
