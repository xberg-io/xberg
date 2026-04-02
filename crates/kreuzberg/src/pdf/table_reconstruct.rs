//! Table reconstruction from PDF segments (no OCR dependency).
//!
//! This module provides table reconstruction utilities that work with any
//! source of word-level text data (PDF native text, OCR output, etc.).
//! It re-exports core types from `table_core` and adds PDF-specific
//! conversion helpers.

use super::hierarchy::SegmentData;

pub use crate::table_core::{HocrWord, reconstruct_table, table_to_markdown};

/// Convert a PDF `SegmentData` to an `HocrWord` for table reconstruction.
///
/// `SegmentData` uses PDF coordinates (y=0 at bottom, increases upward).
/// `HocrWord` uses image coordinates (y=0 at top, increases downward).
pub fn segment_to_hocr_word(seg: &SegmentData, page_height: f32) -> HocrWord {
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
pub fn split_segment_to_words(seg: &SegmentData, page_height: f32) -> Vec<HocrWord> {
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
pub fn segments_to_words(segments: &[SegmentData], page_height: f32) -> Vec<HocrWord> {
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
pub fn post_process_table(
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
        }
    }

    #[test]
    fn test_split_single_word() {
        let seg = make_seg("Hello", 100.0, 500.0, 50.0, 12.0);
        let words = split_segment_to_words(&seg, 800.0);
        assert_eq!(words.len(), 1);
        assert_eq!(words[0].text, "Hello");
        assert_eq!(words[0].left, 100);
    }

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

    #[test]
    fn test_split_empty_segment() {
        let seg = make_seg("   ", 100.0, 500.0, 50.0, 12.0);
        let words = split_segment_to_words(&seg, 800.0);
        assert!(words.is_empty());
    }

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

    #[test]
    fn test_split_y_coordinate_conversion() {
        // Segment at y=500 (PDF bottom-up), height=12, page_height=800
        // Image top = 800 - (500 + 12) = 288
        let seg = make_seg("word", 100.0, 500.0, 50.0, 12.0);
        let words = split_segment_to_words(&seg, 800.0);
        assert_eq!(words[0].top, 288);
        assert_eq!(words[0].height, 12);
    }

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
        // but not overwhelming — should be accepted.
        let table = vec![
            vec!["ID".into(), "Description".into()],
            vec![
                "1".into(),
                "A moderately long description that explains the feature in detail.".into(),
            ],
            vec![
                "2".into(),
                "Another description of similar length for a different item.".into(),
            ],
            vec!["3".into(), "Short desc.".into()],
            vec![
                "4".into(),
                "Yet another reasonably sized description for testing.".into(),
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
}
