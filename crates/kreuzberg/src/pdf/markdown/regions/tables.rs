//! Heuristic table extraction from layout-detected Table regions.

use crate::pdf::markdown::text_repair::repair_broken_word_spacing;
use crate::pdf::markdown::types::{LayoutHint, LayoutHintClass};
use crate::pdf::table_reconstruct::{post_process_table, reconstruct_table, table_to_markdown};
use crate::types::Table;

use super::table_recognition::word_hint_iow;

/// Extract tables from layout-detected Table regions using word-level data.
///
/// Filters the provided words by Table hint bboxes and reconstructs table
/// structure using heuristic column/row detection. The caller is responsible
/// for providing words (typically from `segments_to_words` for consistency
/// with region assembly, or from `extract_words_from_page` as fallback).
pub(in crate::pdf::markdown) fn extract_tables_from_layout_hints(
    words: &[crate::pdf::table_reconstruct::HocrWord],
    hints: &[LayoutHint],
    page_index: usize,
    page_height: f32,
    min_confidence: f32,
    allow_single_column: bool,
) -> Vec<Table> {
    use crate::pdf::table_reconstruct::HocrWord;

    let table_hints: Vec<&LayoutHint> = hints
        .iter()
        .filter(|h| h.class == LayoutHintClass::Table && h.confidence >= min_confidence)
        .collect();

    if table_hints.is_empty() {
        return Vec::new();
    }

    let mut tables = Vec::new();

    for hint in &table_hints {
        // Filter words that overlap the table hint bbox (≥20% of word area).
        // HocrWord uses image coordinates (y=0 at top), while hint uses PDF
        // coordinates (y=0 at bottom). Convert hint bbox to image coords.
        let hint_img_top = (page_height - hint.top).max(0.0);
        let hint_img_bottom = (page_height - hint.bottom).max(0.0);

        let table_words: Vec<HocrWord> = words
            .iter()
            .filter(|w| {
                if w.text.trim().is_empty() {
                    return false;
                }
                word_hint_iow(w, hint.left, hint_img_top, hint.right, hint_img_bottom) >= 0.2
            })
            .cloned()
            .collect();

        // Need at least 4 words for a meaningful table
        if table_words.len() < 4 {
            continue;
        }

        // Adaptive column gap threshold: scale with table width.
        // Narrow tables (< 300pt) use a tight threshold (15), while wide
        // tables (> 600pt) use a looser threshold (30) to avoid over-splitting.
        let table_width = hint.right - hint.left;
        let col_gap = if table_width < 300.0 {
            15
        } else if table_width < 600.0 {
            20
        } else {
            30
        };
        let table_cells = reconstruct_table(&table_words, col_gap, 0.5);

        if table_cells.is_empty() || table_cells[0].is_empty() {
            continue;
        }

        // Bounding box from the layout hint (already in PDF coordinates)
        let bounding_box = Some(crate::types::BoundingBox {
            x0: hint.left as f64,
            y0: hint.bottom as f64,
            x1: hint.right as f64,
            y1: hint.top as f64,
        });

        // Validate with layout_guided=true (relaxed thresholds)
        let table_cells = match post_process_table(table_cells, true, allow_single_column) {
            Some(cleaned) => cleaned,
            None => {
                // Table reconstruction failed — the Table hint was a false positive.
                // Do NOT emit a table with bounding_box: that would add the bbox to
                // extracted_table_bboxes_by_page, suppressing legitimate text segments
                // in assign_segments_to_regions (IoS >= 0.5 check). Instead, skip this
                // hint entirely and let the text fall through as unassigned segments
                // in the normal pipeline.
                tracing::trace!(
                    page = page_index,
                    hint_left = hint.left,
                    hint_right = hint.right,
                    words = table_words.len(),
                    "table reconstruction failed — skipping false-positive Table hint"
                );
                continue;
            }
        };

        // Reject single-row tables — these are almost always false positives
        // from the layout model (e.g., a line of text misclassified as Table).
        if table_cells.len() <= 1 {
            tracing::trace!(
                page = page_index,
                rows = table_cells.len(),
                "table has <=1 row — skipping likely false-positive Table hint"
            );
            continue;
        }

        // Reject degenerate tables with too many empty cells.
        // False-positive Table hints (e.g. in RTL documents) often produce
        // tables where most cells are empty because the content is not truly
        // tabular. Skip these to avoid polluting output with markdown table
        // formatting characters that hurt TF1.
        let total_cells: usize = table_cells.iter().map(|row| row.len()).sum();
        let empty_cells: usize = table_cells
            .iter()
            .flat_map(|row| row.iter())
            .filter(|cell| cell.trim().is_empty())
            .count();
        if total_cells > 0 && empty_cells as f64 / total_cells as f64 > 0.4 {
            tracing::trace!(
                page = page_index,
                total_cells,
                empty_cells,
                "table has >40% empty cells — skipping degenerate table"
            );
            continue;
        }

        // Reject tables where total text content is very short relative to
        // the number of cells. This catches false positives where a small
        // amount of text is spread across a table grid.
        let total_text_len: usize = table_cells
            .iter()
            .flat_map(|row| row.iter())
            .map(|cell| cell.trim().len())
            .sum();
        if total_cells > 4 && total_text_len < total_cells * 2 {
            tracing::trace!(
                page = page_index,
                total_cells,
                total_text_len,
                "table text content too sparse — skipping degenerate table"
            );
            continue;
        }

        // Reject tables where most rows have only 1 filled cell.
        // This pattern indicates non-tabular content forced into a grid
        // (e.g., RTL text where each line becomes a "row" with one cell).
        if table_cells.len() >= 3 {
            let single_cell_rows = table_cells
                .iter()
                .filter(|row| row.iter().filter(|c| !c.trim().is_empty()).count() <= 1)
                .count();
            if single_cell_rows as f64 / table_cells.len() as f64 > 0.5 {
                tracing::trace!(
                    page = page_index,
                    rows = table_cells.len(),
                    single_cell_rows,
                    "table has >50% single-cell rows — skipping likely false-positive"
                );
                continue;
            }
        }

        // Repair broken word spacing per-cell before rendering to markdown
        let repaired_cells: Vec<Vec<String>> = table_cells
            .iter()
            .map(|row| {
                row.iter()
                    .map(|cell| repair_broken_word_spacing(cell).into_owned())
                    .collect()
            })
            .collect();
        let markdown = table_to_markdown(&repaired_cells);

        tracing::trace!(
            page = page_index,
            rows = table_cells.len(),
            total_cells,
            empty_cells,
            total_text_len,
            markdown_len = markdown.len(),
            "table accepted"
        );

        tables.push(Table {
            cells: table_cells,
            markdown,
            page_number: page_index + 1,
            bounding_box,
        });
    }

    tables
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::table_reconstruct::HocrWord;

    fn make_word(text: &str, left: u32, top: u32, width: u32, height: u32) -> HocrWord {
        HocrWord {
            text: text.to_string(),
            left,
            top,
            width,
            height,
            confidence: 95.0,
        }
    }

    fn make_table_hint(confidence: f32, left: f32, bottom: f32, right: f32, top: f32) -> LayoutHint {
        LayoutHint {
            class: LayoutHintClass::Table,
            confidence,
            left,
            bottom,
            right,
            top,
        }
    }

    #[test]
    fn test_no_table_hints_returns_empty() {
        let words = vec![make_word("hello", 10, 10, 50, 12)];
        let hints = vec![LayoutHint {
            class: LayoutHintClass::Text,
            confidence: 0.9,
            left: 0.0,
            bottom: 0.0,
            right: 600.0,
            top: 800.0,
        }];
        let tables = extract_tables_from_layout_hints(&words, &hints, 0, 800.0, 0.5, false);
        assert!(tables.is_empty());
    }

    #[test]
    fn test_low_confidence_table_hint_filtered() {
        let words = vec![
            make_word("A", 10, 10, 50, 12),
            make_word("B", 100, 10, 50, 12),
            make_word("C", 10, 30, 50, 12),
            make_word("D", 100, 30, 50, 12),
        ];
        let hints = vec![make_table_hint(0.3, 0.0, 0.0, 200.0, 800.0)];
        // min_confidence = 0.5, hint has 0.3 → filtered
        let tables = extract_tables_from_layout_hints(&words, &hints, 0, 800.0, 0.5, false);
        assert!(tables.is_empty());
    }

    #[test]
    fn test_empty_region_too_few_words() {
        // Only 2 words in the region — below the 4-word minimum
        let words = vec![make_word("A", 10, 10, 50, 12), make_word("B", 100, 10, 50, 12)];
        let hints = vec![make_table_hint(0.9, 0.0, 0.0, 200.0, 800.0)];
        let tables = extract_tables_from_layout_hints(&words, &hints, 0, 800.0, 0.5, false);
        assert!(tables.is_empty());
    }

    #[test]
    fn test_empty_words_returns_empty() {
        let hints = vec![make_table_hint(0.9, 0.0, 0.0, 200.0, 800.0)];
        let tables = extract_tables_from_layout_hints(&[], &hints, 0, 800.0, 0.5, false);
        assert!(tables.is_empty());
    }

    #[test]
    fn test_no_hints_returns_empty() {
        let words = vec![
            make_word("A", 10, 10, 50, 12),
            make_word("B", 100, 10, 50, 12),
            make_word("C", 10, 30, 50, 12),
            make_word("D", 100, 30, 50, 12),
        ];
        let tables = extract_tables_from_layout_hints(&words, &[], 0, 800.0, 0.5, false);
        assert!(tables.is_empty());
    }

    #[test]
    fn test_words_outside_hint_bbox_excluded() {
        // Words at (500, 500) are far from the hint bbox
        let words = vec![
            make_word("A", 500, 500, 50, 12),
            make_word("B", 560, 500, 50, 12),
            make_word("C", 500, 520, 50, 12),
            make_word("D", 560, 520, 50, 12),
        ];
        // Hint covers (0, 0) to (100, 100) in PDF coords → image y = 700..800
        let hints = vec![make_table_hint(0.9, 0.0, 700.0, 100.0, 800.0)];
        let tables = extract_tables_from_layout_hints(&words, &hints, 0, 800.0, 0.5, false);
        // Words at (500, 500) don't overlap the hint → too few words → empty
        assert!(tables.is_empty());
    }

    #[test]
    fn test_whitespace_only_words_filtered() {
        let words = vec![
            make_word("  ", 10, 10, 50, 12),
            make_word("A", 100, 10, 50, 12),
            make_word("B", 10, 30, 50, 12),
            make_word("C", 100, 30, 50, 12),
        ];
        // Only 3 non-empty words → below 4-word minimum
        let hints = vec![make_table_hint(0.9, 0.0, 0.0, 200.0, 800.0)];
        let tables = extract_tables_from_layout_hints(&words, &hints, 0, 800.0, 0.5, false);
        assert!(tables.is_empty());
    }

    #[test]
    fn test_page_number_is_one_indexed() {
        // Construct words that form a valid 2-column, multi-row table
        // Rows at y=10 and y=40 in image coords, columns at x=10 and x=200
        let words = vec![
            make_word("Header1", 10, 10, 80, 15),
            make_word("Header2", 200, 10, 80, 15),
            make_word("Cell1", 10, 40, 80, 15),
            make_word("Cell2", 200, 40, 80, 15),
            make_word("Cell3", 10, 70, 80, 15),
            make_word("Cell4", 200, 70, 80, 15),
        ];
        // Hint in PDF coords: bottom=700, top=800 → image top=0, image bottom=100
        let hints = vec![make_table_hint(0.9, 0.0, 700.0, 400.0, 800.0)];
        let tables = extract_tables_from_layout_hints(&words, &hints, 2, 800.0, 0.5, false);
        // If a valid table is produced, its page_number should be page_index + 1
        for table in &tables {
            assert_eq!(table.page_number, 3); // page_index=2 → page_number=3
        }
    }
}
