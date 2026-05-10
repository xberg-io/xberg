//! Native table detection using the pdf_oxide backend.
//!
//! Uses pdf_oxide's built-in `extract_tables_with_config` API with strict mode
//! to detect tables with high precision, replacing the heuristic word-based
//! approach that caused false positives on paragraph text.

use super::OxideDocument;
use crate::pdf::error::{PdfError, Result};
use crate::types::{BoundingBox, Table};

/// Extract tables from all pages using pdf_oxide's native table detection.
///
/// Uses `TableDetectionConfig::strict()` to reduce false positives from
/// paragraph text being misidentified as tables.
///
/// # Arguments
///
/// * `doc` - Mutable reference to the oxide document
///
/// # Returns
///
/// A `Vec<Table>` containing all detected tables with cells, markdown, and bounding boxes.
pub(crate) fn extract_tables_native(doc: &mut OxideDocument) -> Result<Vec<Table>> {
    let page_count = doc
        .doc
        .page_count()
        .map_err(|e| PdfError::MetadataExtractionFailed(format!("pdf_oxide: failed to get page count: {e}")))?;

    let config = pdf_oxide::structure::spatial_table_detector::TableDetectionConfig::strict();
    let mut all_tables = Vec::new();

    for page_idx in 0..page_count {
        let extracted = match doc.doc.extract_tables_with_config(page_idx, config.clone()) {
            Ok(tables) => tables,
            Err(e) => {
                tracing::debug!(page = page_idx, "pdf_oxide extract_tables failed: {e}");
                continue;
            }
        };

        let page_number = page_idx + 1; // Kreuzberg uses 1-indexed page numbers

        for extracted_table in extracted {
            if extracted_table.rows.is_empty() || extracted_table.col_count == 0 {
                continue;
            }

            let (cells, markdown) = convert_extracted_table(&extracted_table);

            // Skip tables that produced no meaningful content
            if cells.is_empty() || markdown.trim().is_empty() {
                continue;
            }

            // Guard: require minimum 2 rows and 2 columns for a valid table.
            // Single-column tables and single-row tables are typically not real tables.
            // This filters out Google Docs paragraph borders and other styling artifacts.
            if cells.len() < 2 || cells.iter().all(|row| row.len() < 2) {
                tracing::debug!(
                    page = page_idx,
                    rows = cells.len(),
                    cols = cells.first().map(|r| r.len()).unwrap_or(0),
                    "Skipping table below minimum dimensions (need ≥2 rows and ≥2 cols)"
                );
                continue;
            }

            let bounding_box = extracted_table.bbox.map(|rect| BoundingBox {
                x0: rect.x as f64,
                y0: rect.y as f64,
                x1: (rect.x + rect.width) as f64,
                y1: (rect.y + rect.height) as f64,
            });

            all_tables.push(Table {
                cells,
                markdown,
                page_number,
                bounding_box,
            });
        }
    }

    Ok(all_tables)
}

/// Reconstruct cell text from span positions in reading order.
///
/// PDF coordinates place y=0 at the bottom of the page, so larger Y values are
/// higher on the page (visually earlier in reading order). Within a row, X
/// increases left-to-right. We sort by Y descending (top-to-bottom) then X
/// ascending (left-to-right) to recover natural reading order regardless of
/// the order in which pdf_oxide yields spans.
///
/// Embedded newlines inside span text are collapsed to spaces to produce
/// clean single-line cell strings.
fn cell_text_in_reading_order(cell: &pdf_oxide::structure::table_extractor::TableCell) -> String {
    if cell.spans.is_empty() {
        // No positional span data — fall back to the pre-joined text field.
        return cell.text.trim().replace('\n', " ").to_string();
    }

    // Collect (y, x, text) for each span, then sort by y descending, x ascending.
    let mut sorted: Vec<(f32, f32, &str)> = cell
        .spans
        .iter()
        .map(|span| (span.bbox.y, span.bbox.x, span.text.as_str()))
        .collect();
    sorted.sort_by(|a, b| {
        // Primary: Y descending (larger Y = higher on page = earlier in reading order)
        b.0.total_cmp(&a.0).then_with(|| {
            // Secondary: X ascending (left before right)
            a.1.total_cmp(&b.1)
        })
    });

    let joined: String = sorted
        .iter()
        .map(|(_, _, text)| text.trim().replace('\n', " "))
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");

    joined
}

/// Convert a pdf_oxide `ExtractedTable` to kreuzberg's cell grid and markdown.
///
/// Maps rows/cells from the native table structure to a 2D `Vec<Vec<String>>`
/// grid and builds a markdown representation with proper header separators.
///
/// Cell text is reconstructed from span positions in reading order
/// (Y descending, X ascending) when span data is available.
fn convert_extracted_table(table: &pdf_oxide::structure::table_extractor::Table) -> (Vec<Vec<String>>, String) {
    let mut cells: Vec<Vec<String>> = Vec::with_capacity(table.rows.len());
    let mut markdown = String::new();
    let mut found_header = false;

    for (row_idx, row) in table.rows.iter().enumerate() {
        let row_cells: Vec<String> = row.cells.iter().map(cell_text_in_reading_order).collect();

        // Build markdown row
        markdown.push('|');
        for cell in &row_cells {
            markdown.push(' ');
            markdown.push_str(cell);
            markdown.push_str(" |");
        }
        markdown.push('\n');

        // Insert header separator after the first header row
        if row.is_header && !found_header {
            found_header = true;
            markdown.push('|');
            for _ in &row_cells {
                markdown.push_str(" --- |");
            }
            markdown.push('\n');
        } else if row_idx == 0 && !found_header {
            // If no explicit header, treat first row as header for markdown formatting
            found_header = true;
            markdown.push('|');
            for _ in &row_cells {
                markdown.push_str(" --- |");
            }
            markdown.push('\n');
        }

        cells.push(row_cells);
    }

    (cells, markdown)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_extracted_table_basic() {
        use pdf_oxide::structure::table_extractor::{Table as ExtractedTable, TableCell, TableRow};

        let table = ExtractedTable {
            rows: vec![
                TableRow {
                    cells: vec![
                        TableCell {
                            text: "Name".to_string(),
                            colspan: 1,
                            rowspan: 1,
                            mcids: vec![],
                            spans: vec![],
                            bbox: None,
                            is_header: true,
                        },
                        TableCell {
                            text: "Age".to_string(),
                            colspan: 1,
                            rowspan: 1,
                            mcids: vec![],
                            spans: vec![],
                            bbox: None,
                            is_header: true,
                        },
                    ],
                    is_header: true,
                },
                TableRow {
                    cells: vec![
                        TableCell {
                            text: "Alice".to_string(),
                            colspan: 1,
                            rowspan: 1,
                            mcids: vec![],
                            spans: vec![],
                            bbox: None,
                            is_header: false,
                        },
                        TableCell {
                            text: "30".to_string(),
                            colspan: 1,
                            rowspan: 1,
                            mcids: vec![],
                            spans: vec![],
                            bbox: None,
                            is_header: false,
                        },
                    ],
                    is_header: false,
                },
            ],
            has_header: true,
            col_count: 2,
            bbox: None,
        };

        let (cells, markdown) = convert_extracted_table(&table);
        assert_eq!(cells.len(), 2);
        assert_eq!(cells[0], vec!["Name", "Age"]);
        assert_eq!(cells[1], vec!["Alice", "30"]);
        assert!(markdown.contains("| Name | Age |"));
        assert!(markdown.contains("| --- | --- |"));
        assert!(markdown.contains("| Alice | 30 |"));
    }

    #[test]
    fn test_convert_extracted_table_no_header() {
        use pdf_oxide::structure::table_extractor::{Table as ExtractedTable, TableCell, TableRow};

        let table = ExtractedTable {
            rows: vec![
                TableRow {
                    cells: vec![TableCell {
                        text: "A".to_string(),
                        colspan: 1,
                        rowspan: 1,
                        mcids: vec![],
                        spans: vec![],
                        bbox: None,
                        is_header: false,
                    }],
                    is_header: false,
                },
                TableRow {
                    cells: vec![TableCell {
                        text: "B".to_string(),
                        colspan: 1,
                        rowspan: 1,
                        mcids: vec![],
                        spans: vec![],
                        bbox: None,
                        is_header: false,
                    }],
                    is_header: false,
                },
            ],
            has_header: false,
            col_count: 1,
            bbox: None,
        };

        let (cells, markdown) = convert_extracted_table(&table);
        assert_eq!(cells.len(), 2);
        // Even without explicit header, first row gets separator for valid markdown
        assert!(markdown.contains("| --- |"));
    }

    #[test]
    fn test_convert_extracted_table_empty() {
        use pdf_oxide::structure::table_extractor::Table as ExtractedTable;

        let table = ExtractedTable {
            rows: vec![],
            has_header: false,
            col_count: 0,
            bbox: None,
        };

        let (cells, markdown) = convert_extracted_table(&table);
        assert!(cells.is_empty());
        assert!(markdown.is_empty());
    }

    // ── W2.E: cell reading-order reconciliation ──

    /// Build a synthetic TextSpan for position-order tests.
    ///
    /// `x` and `y` are the span's PDF-coordinate origin (y=0 at bottom of page).
    fn make_span(text: &str, x: f32, y: f32) -> pdf_oxide::layout::TextSpan {
        pdf_oxide::layout::TextSpan {
            text: text.to_string(),
            bbox: pdf_oxide::geometry::Rect { x, y, width: 50.0, height: 10.0 },
            font_name: "Helvetica".to_string(),
            font_size: 10.0,
            font_weight: pdf_oxide::layout::FontWeight::Normal,
            is_italic: false,
            is_monospace: false,
            color: pdf_oxide::layout::Color::default(),
            mcid: None,
            sequence: 0,
            split_boundary_before: false,
            offset_semantic: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 1.0,
            primary_detected: false,
            artifact_type: None,
            char_widths: vec![],
        }
    }

    /// Spans delivered in reverse Y order (bottom span first, top span last).
    /// Reading order must place the top span (higher Y in PDF coords) first.
    #[test]
    fn test_cell_text_in_reading_order_sorts_by_y_descending() {
        use pdf_oxide::structure::table_extractor::TableCell;

        let cell = TableCell {
            text: "wrong order".to_string(),
            colspan: 1,
            rowspan: 1,
            mcids: vec![],
            // Spans intentionally out of reading order: lower Y (bottom of page) first.
            spans: vec![
                make_span("second", 10.0, 100.0), // y=100 — lower on page (appears later)
                make_span("first", 10.0, 200.0),  // y=200 — higher on page (appears first)
            ],
            bbox: None,
            is_header: false,
        };

        let text = cell_text_in_reading_order(&cell);
        assert_eq!(
            text, "first second",
            "span with higher Y (top of page) must come before span with lower Y; got: {text:?}"
        );
    }

    /// Within the same Y row, spans must be ordered left-to-right (X ascending).
    #[test]
    fn test_cell_text_in_reading_order_sorts_same_y_by_x_ascending() {
        use pdf_oxide::structure::table_extractor::TableCell;

        let cell = TableCell {
            text: "wrong order".to_string(),
            colspan: 1,
            rowspan: 1,
            mcids: vec![],
            // Same Y — right column (x=200) delivered before left column (x=10).
            spans: vec![
                make_span("right", 200.0, 150.0),
                make_span("left", 10.0, 150.0),
            ],
            bbox: None,
            is_header: false,
        };

        let text = cell_text_in_reading_order(&cell);
        assert_eq!(
            text, "left right",
            "same-row spans must be ordered left-to-right (X ascending); got: {text:?}"
        );
    }

    /// When spans is empty, fall back to cell.text (trimmed, newlines collapsed).
    #[test]
    fn test_cell_text_in_reading_order_fallback_to_cell_text() {
        use pdf_oxide::structure::table_extractor::TableCell;

        let cell = TableCell {
            text: "  hello\nworld  ".to_string(),
            colspan: 1,
            rowspan: 1,
            mcids: vec![],
            spans: vec![],
            bbox: None,
            is_header: false,
        };

        let text = cell_text_in_reading_order(&cell);
        assert_eq!(text, "hello world", "fallback must trim and collapse newlines; got: {text:?}");
    }
}
