//! Native table detection using the pdf_oxide backend.
//!
//! Three entry points:
//!
//! - [`extract_tables_native`] — pdf_oxide's spatial detector in strict mode
//!   (via the stroke-aware wrapper [`extract_tables_stroke_aware`]). High
//!   precision, requires explicit table grid with 3+ columns.
//! - [`extract_tables_bordered`] — relaxed Lines-strategy pass for bordered tables
//!   with 2+ columns. Catches 2-column data tables (label/value) whose cells are
//!   delimited by stroke lines that `strict()` skips due to `min_table_columns=3`.
//! - [`extract_tables_heuristic`] — text-layer fallback that reuses the same
//!   spatial reconstruction the layout-detection path uses, but without
//!   requiring layout hints. Catches tables that pdf_oxide's grid detector
//!   misses (e.g. invoice line items, financial tables without ruling lines).
//!
//! The default extraction flow in `extractors::pdf::extraction` runs all three
//! per-page in priority order (native → bordered → heuristic), where each tier
//! only runs on pages where the previous tier found nothing.

use super::OxideDocument;
use crate::pdf::error::{PdfError, Result};
use crate::types::{BoundingBox, Table};
use std::collections::HashSet;

/// Cap on candidate vertical regions per page. Real tables fit comfortably
/// under this; prose-heavy pages can otherwise generate dozens of small
/// regions that each go through `reconstruct_table` + `post_process_table`,
/// burning CPU on guaranteed rejections. The cap is generous on purpose —
/// the validation chain is fast.
const MAX_REGIONS_PER_PAGE: usize = 20;

/// Geometry floor of `PathContent::is_table_primitive`: both its thin-line
/// branches require the long dimension to exceed 5pt, and its box branch
/// requires both dimensions to. A path whose longest dimension is at or below
/// this can never register as a table primitive on geometry alone — its only
/// possible rendered extent lives in the stroke width.
const PRIMITIVE_GEOMETRY_FLOOR: f32 = 5.0;

/// Expand a stroke-width-encoded rule's bounding box to its rendered geometry.
///
/// INTERIM WORKAROUND — remove once yfedoseev/pdf_oxide#812 lands (PR #814).
/// This reimplements part of pdf_oxide's table pipeline to inject a bbox
/// expansion; once pdf_oxide's path bounding boxes account for stroke width,
/// `extract_tables_with_config` detects these rules natively and this module
/// reverts to calling it directly. Kept on a dedicated branch, not merged,
/// precisely because it duplicates upstream internals.
///
/// Some print-era PDF generators draw vertical table rules as a ~1pt
/// horizontal segment stroked with a line width equal to the table height
/// (`430 w` + `0 0 m 1 0 l S` renders as a 1×430pt vertical bar). pdf_oxide's
/// `bbox` covers the path geometry only, so such a rule is a 1×0pt speck that
/// fails `is_table_primitive` — the Lines-strategy detector then sees no
/// vertical rulings and misses the whole table (xberg-io/xberg#1213).
///
/// Stroking widens a segment by half the line width on each side
/// perpendicular to its direction; mirror that on the minor axis so the
/// detector sees what the reader sees. Expansion is deliberately limited to
/// geometric *specks* — paths whose longest dimension is under the
/// [`PRIMITIVE_GEOMETRY_FLOOR`] and which therefore can never pass the
/// primitive filter on geometry. Such paths contribute nothing to detection
/// today, so expanding them can only add rulings, never perturb a grid that
/// already works (a long thin separator with a thick stroke — e.g. the
/// 0×507pt/7.2pt column rules in pdfplumber's la-precinct fixture — is
/// already a valid primitive and must keep its geometric bbox).
fn expand_hairline_stroke(path: &mut pdf_oxide::elements::PathContent) {
    if path.stroke_color.is_none() || !path.stroke_width.is_finite() || path.stroke_width <= PRIMITIVE_GEOMETRY_FLOOR {
        return;
    }
    let width = path.bbox.width.abs();
    let height = path.bbox.height.abs();
    if !(width.is_finite() && height.is_finite()) {
        return; // degenerate glyph geometry (see #1198): don't grow NaN boxes
    }
    if width.max(height) > PRIMITIVE_GEOMETRY_FLOOR {
        return; // real geometric extent: the detector already sees this path
    }
    let stroke = path.stroke_width;
    if height <= width {
        // Horizontal-ish segment: the stroke extends above and below.
        path.bbox.y -= stroke / 2.0;
        path.bbox.height += stroke;
    } else {
        // Vertical-ish segment: the stroke extends left and right.
        path.bbox.x -= stroke / 2.0;
        path.bbox.width += stroke;
    }
}

/// Detect tables on one page with stroke-rendered rule geometry.
///
/// Mirrors `pdf_oxide::PdfDocument::extract_tables_with_config` (words →
/// spans, paths → table primitives → `detect_tables_with_lines`), inserting
/// [`expand_hairline_stroke`] between path extraction and the
/// `is_table_primitive` filter so stroke-width-rendered rules survive it.
fn extract_tables_stroke_aware(
    doc: &pdf_oxide::PdfDocument,
    page_idx: usize,
    config: &pdf_oxide::structure::spatial_table_detector::TableDetectionConfig,
) -> std::result::Result<Vec<pdf_oxide::structure::table_extractor::Table>, pdf_oxide::Error> {
    use pdf_oxide::layout::{FontWeight, TextSpan};
    use pdf_oxide::structure::spatial_table_detector::detect_tables_with_lines;

    let words = doc.extract_words(page_idx)?;
    let lines: Vec<_> = doc
        .extract_paths(page_idx)?
        .into_iter()
        .map(|mut p| {
            expand_hairline_stroke(&mut p);
            p
        })
        .filter(|p| p.is_table_primitive())
        .collect();

    // Word → TextSpan conversion matching extract_tables_with_config: words
    // (not spans) so space-separated strings split into their own columns.
    let spans: Vec<TextSpan> = words
        .into_iter()
        .map(|w| TextSpan {
            text: w.text,
            bbox: w.bbox,
            font_name: w.dominant_font,
            font_size: w.avg_font_size,
            font_weight: if w.is_bold {
                FontWeight::Bold
            } else {
                FontWeight::Normal
            },
            is_italic: w.is_italic,
            mcid: w.mcid,
            horizontal_scaling: 1.0,
            ..TextSpan::default()
        })
        .collect();

    Ok(detect_tables_with_lines(&spans, &lines, config))
}

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
        let extracted = match extract_tables_stroke_aware(&doc.doc, page_idx, &config) {
            Ok(tables) => tables,
            Err(e) => {
                tracing::warn!(page = page_idx, "pdf_oxide extract_tables failed: {e}");
                continue;
            }
        };

        let page_number = (page_idx + 1) as u32; // Xberg uses 1-indexed page numbers

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

/// Extract bordered tables from all pages using a relaxed Lines-strategy config.
///
/// Targets 2-column tables whose cells are delimited by stroke/fill rectangles
/// (the `re` PDF operator or explicit `m/l` paths) that `extract_tables_native`
/// misses because `strict()` requires ≥ 3 columns.
///
/// Uses `TableStrategy::Lines` for both axes — path primitives only, no text-
/// edge heuristics — so this pass does not introduce new text-based false positives.
/// The relaxed thresholds:
///
/// - `min_table_columns = 2` (vs 3 in strict)
/// - `min_table_cells = 4` (vs 6 in strict)
/// - `regular_row_ratio = 0.5` (vs 0.8 in strict)
///
/// `skip_pages` (1-indexed) suppresses this pass on pages where a higher-priority
/// detector already produced a result. Pass an empty set to run on every page.
pub(crate) fn extract_tables_bordered(doc: &mut OxideDocument, skip_pages: &HashSet<u32>) -> Result<Vec<Table>> {
    use pdf_oxide::structure::spatial_table_detector::{TableDetectionConfig, TableStrategy};

    let page_count = doc
        .doc
        .page_count()
        .map_err(|e| PdfError::MetadataExtractionFailed(format!("pdf_oxide: failed to get page count: {e}")))?;

    let config = TableDetectionConfig {
        enabled: true,
        horizontal_strategy: TableStrategy::Lines,
        vertical_strategy: TableStrategy::Lines,
        column_tolerance: 3.0,
        row_tolerance: 2.0,
        min_table_cells: 4,
        min_table_columns: 2,
        regular_row_ratio: 0.5,
        max_table_columns: 15,
        column_merge_threshold: 12.0,
        v_split_gap: 4.0,
        text_fallback: false,
    };

    let mut all_tables = Vec::new();

    for page_idx in 0..page_count {
        let page_number = (page_idx + 1) as u32;
        if skip_pages.contains(&page_number) {
            continue;
        }

        let extracted = match extract_tables_stroke_aware(&doc.doc, page_idx, &config) {
            Ok(tables) => tables,
            Err(e) => {
                tracing::warn!(page = page_idx, "pdf_oxide bordered extract_tables failed: {e}");
                continue;
            }
        };

        for extracted_table in extracted {
            if extracted_table.rows.is_empty() || extracted_table.col_count == 0 {
                continue;
            }

            let (cells, markdown) = convert_extracted_table(&extracted_table);

            if cells.is_empty() || markdown.trim().is_empty() {
                continue;
            }

            if cells.len() < 2 || cells.iter().all(|row| row.len() < 2) {
                tracing::debug!(
                    page = page_idx,
                    rows = cells.len(),
                    cols = cells.first().map(|r| r.len()).unwrap_or(0),
                    "Skipping bordered table below minimum dimensions"
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

/// Heuristic table reconstruction for text-layer PDFs without layout hints.
///
/// Pulls per-page text segments via the same hierarchy extractor used by the
/// structured pipeline, projects each segment into an `HocrWord`, clusters
/// words into vertically-contiguous regions by abnormally-large row gaps,
/// then runs the shared `reconstruct_table` → `post_process_table` →
/// `is_well_formed_table` chain (the same one consumed by the OCR pipeline
/// and the layout-detection path) per region.
///
/// The clustering step does NOT validate that a region is table-shaped — it
/// merely separates vertically-isolated word slabs. Multi-column prose on a
/// single page will pass clustering as one wide region; `is_well_formed_table`
/// is the actual prose-rejection guard. We pass `layout_guided = true` to
/// `post_process_table` so that "we already pre-segmented the page" earns
/// the relaxed text-density thresholds, but the prose-detection check in
/// `is_well_formed_table` (row coherence + column semantic uniformity) is
/// what catches columned articles and similar false positives.
///
/// `skip_pages` (1-indexed, matching `Table.page_number`) lets the caller
/// suppress heuristic work on pages where pdf_oxide's native grid detector
/// already produced results — those tables are higher precision than
/// anything the heuristic can derive, so we keep native and only fill in
/// the gaps. Pass an empty set to run on every page.
///
/// This is invoked by `extractors::pdf::extraction` as a per-page fallback
/// alongside [`extract_tables_native`] — typically for text-layer PDFs
/// whose tables aren't drawn with explicit rule lines that pdf_oxide's grid
/// detector can lock onto.
///
/// Returns an empty vec on any extraction failure (the caller treats this as
/// "no heuristic tables found" and keeps going).
pub(crate) fn extract_tables_heuristic(
    doc: &mut OxideDocument,
    allow_single_column: bool,
    skip_pages: &HashSet<u32>,
) -> Result<Vec<Table>> {
    use crate::pdf::table_reconstruct::{HocrWord, segments_to_words};

    let (per_page_segments, _used_structure_tree) =
        crate::pdf::oxide::hierarchy::extract_all_segments(doc).map_err(|e| {
            PdfError::TextExtractionFailed(format!(
                "pdf_oxide hierarchy extraction failed for heuristic tables: {e}"
            ))
        })?;

    let page_count = doc
        .doc
        .page_count()
        .map_err(|e| PdfError::MetadataExtractionFailed(format!("pdf_oxide: failed to get page count: {e}")))?;

    let mut tables = Vec::new();

    for page_idx in 0..page_count {
        let page_number = (page_idx + 1) as u32;
        if skip_pages.contains(&page_number) {
            continue;
        }

        let Some(segments) = per_page_segments.get(page_idx) else {
            continue;
        };
        if segments.is_empty() {
            continue;
        }

        // Page-height approximation matches the structure pipeline:
        // max of segment y+height with letter-size fallback.
        let page_height = segments
            .iter()
            .map(|s| s.y + s.height)
            .fold(0.0_f32, f32::max)
            .max(792.0);

        let words: Vec<HocrWord> = segments_to_words(segments, page_height);
        if words.len() < 4 {
            continue;
        }

        let mut regions = cluster_words_into_vertical_regions(&words);
        if regions.len() > MAX_REGIONS_PER_PAGE {
            tracing::debug!(
                page = page_number,
                regions = regions.len(),
                cap = MAX_REGIONS_PER_PAGE,
                "heuristic table extraction: capping candidate regions on this page",
            );
            regions.truncate(MAX_REGIONS_PER_PAGE);
        }

        for region in regions {
            if let Some(table) = reconstruct_region_table(&region, page_height, page_number, allow_single_column) {
                tables.push(table);
            }
        }
    }

    Ok(tables)
}

/// Cluster words on a single page into vertically-contiguous regions.
///
/// Splits the page on row gaps that are abnormally large compared to the
/// median word height. Each region is a slab of words that may form a table;
/// the caller validates with `reconstruct_table` + `post_process_table`.
fn cluster_words_into_vertical_regions(
    words: &[crate::pdf::table_reconstruct::HocrWord],
) -> Vec<Vec<crate::pdf::table_reconstruct::HocrWord>> {
    if words.len() < 4 {
        return Vec::new();
    }

    let mut heights: Vec<u32> = words.iter().map(|w| w.height).collect();
    heights.sort_unstable();
    let median_height = heights[heights.len() / 2].max(1);
    let row_tolerance = (median_height / 2).max(3);
    let row_gap_split = (median_height as f32 * 1.8) as u32;

    let mut sorted = words.to_vec();
    sorted.sort_by_key(|w| w.top + w.height / 2);

    let mut regions: Vec<Vec<crate::pdf::table_reconstruct::HocrWord>> = Vec::new();
    let mut current: Vec<crate::pdf::table_reconstruct::HocrWord> = Vec::new();
    let mut last_row_yc: Option<u32> = None;

    let mut idx = 0;
    while idx < sorted.len() {
        let row_yc = sorted[idx].top + sorted[idx].height / 2;
        let mut end = idx + 1;
        while end < sorted.len() {
            let yc = sorted[end].top + sorted[end].height / 2;
            if yc.abs_diff(row_yc) <= row_tolerance {
                end += 1;
            } else {
                break;
            }
        }

        if let Some(prev_yc) = last_row_yc
            && row_yc > prev_yc
            && row_yc - prev_yc > row_gap_split
            && !current.is_empty()
        {
            regions.push(std::mem::take(&mut current));
        }
        current.extend(sorted[idx..end].iter().cloned());
        last_row_yc = Some(row_yc);
        idx = end;
    }
    if !current.is_empty() {
        regions.push(current);
    }

    regions.retain(|r| {
        if r.len() < 4 {
            return false;
        }
        let mut row_ycs: Vec<u32> = r.iter().map(|w| w.top + w.height / 2).collect();
        row_ycs.sort_unstable();
        row_ycs.dedup_by(|a, b| a.abs_diff(*b) <= row_tolerance);
        if row_ycs.len() < 3 {
            return false;
        }
        let mut xs: Vec<u32> = r.iter().map(|w| w.left).collect();
        xs.sort_unstable();
        xs.dedup_by(|a, b| a.abs_diff(*b) <= 8);
        xs.len() >= 2
    });

    regions
}

/// Reconstruct a single region's words into a `Table`, applying the same
/// validation chain the layout-detection path uses (`layout_guided = true`).
fn reconstruct_region_table(
    region: &[crate::pdf::table_reconstruct::HocrWord],
    page_height: f32,
    page_number: u32,
    allow_single_column: bool,
) -> Option<Table> {
    use crate::pdf::table_reconstruct::{
        is_well_formed_table, looks_like_code_listing, post_process_table, reconstruct_table, table_to_markdown,
    };

    let region_left = region.iter().map(|w| w.left).min().unwrap_or(0);
    let region_right = region.iter().map(|w| w.left + w.width).max().unwrap_or(0);
    let region_width = region_right.saturating_sub(region_left) as f32;
    let col_gap = crate::pdf::structure::regions::tables::compute_adaptive_column_gap(region, region_width);

    let grid = reconstruct_table(region, col_gap, 0.5);
    if grid.is_empty() || grid[0].is_empty() {
        return None;
    }

    // `layout_guided = true`: the region-clustering step separated vertically
    // isolated slabs of words, which is similar to (but weaker than) a layout
    // model's Table region hint. We use the relaxed text-density thresholds;
    // `is_well_formed_table` below is the actual prose guard.
    let cleaned = post_process_table(grid, true, allow_single_column)?;
    if cleaned.len() <= 1 {
        return None;
    }

    // Reject reconstructed grids that look like code listings rather than tables.
    // Monospace code blocks (especially C-family syntax with curly braces) can
    // pass the text-edge clustering step because their fixed-width character
    // spacing creates column-like positions. Isolated `{` or `}` cells are an
    // unambiguous signal: those characters never appear as standalone table cells
    // in real tabular data.
    if looks_like_code_listing(&cleaned) {
        tracing::trace!(
            page = page_number,
            rows = cleaned.len(),
            cols = cleaned.first().map_or(0, |r| r.len()),
            "heuristic table region looks like a code listing — skipping false-positive"
        );
        return None;
    }

    if !is_well_formed_table(&cleaned) {
        return None;
    }

    let img_left = region.iter().map(|w| w.left as f64).fold(f64::INFINITY, f64::min);
    let img_top = region.iter().map(|w| w.top as f64).fold(f64::INFINITY, f64::min);
    let img_right = region.iter().map(|w| (w.left + w.width) as f64).fold(0.0_f64, f64::max);
    let img_bottom = region.iter().map(|w| (w.top + w.height) as f64).fold(0.0_f64, f64::max);
    let bounding_box = if img_right > img_left && img_bottom > img_top {
        Some(BoundingBox {
            x0: img_left,
            y0: page_height as f64 - img_bottom,
            x1: img_right,
            y1: page_height as f64 - img_top,
        })
    } else {
        None
    };

    let markdown = table_to_markdown(&cleaned);
    if markdown.trim().is_empty() {
        return None;
    }

    Some(Table {
        cells: cleaned,
        markdown,
        page_number,
        bounding_box,
    })
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

/// Convert a pdf_oxide `ExtractedTable` to xberg's cell grid and markdown.
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

    fn stroked_line(x: f32, y: f32, w: f32, h: f32, stroke_width: f32) -> pdf_oxide::elements::PathContent {
        let mut path = pdf_oxide::elements::PathContent::new(pdf_oxide::geometry::Rect::new(x, y, w, h));
        path.stroke_color = Some(pdf_oxide::layout::Color::black());
        path.stroke_width = stroke_width;
        path
    }

    /// A ~1pt horizontal segment with a table-height stroke width must expand
    /// vertically into a full-height bar (the #1213 vertical-rule pattern).
    #[test]
    fn expand_hairline_stroke_expands_horizontal_speck_vertically() {
        let mut path = stroked_line(236.0, 402.0, 1.0, 0.0, 430.0);
        expand_hairline_stroke(&mut path);
        assert_eq!(path.bbox.width, 1.0);
        assert_eq!(path.bbox.height, 430.0);
        assert_eq!(path.bbox.y, 402.0 - 215.0);
        assert!(
            path.is_table_primitive(),
            "expanded rule must classify as a table primitive"
        );
    }

    /// The mirrored trick — a short vertical segment with a huge stroke width —
    /// must expand horizontally.
    #[test]
    fn expand_hairline_stroke_expands_vertical_speck_horizontally() {
        let mut path = stroked_line(100.0, 300.0, 0.0, 1.0, 322.0);
        expand_hairline_stroke(&mut path);
        assert_eq!(path.bbox.height, 1.0);
        assert_eq!(path.bbox.width, 322.0);
        assert_eq!(path.bbox.x, 100.0 - 161.0);
        assert!(path.is_table_primitive());
    }

    /// Ordinary hairline rules (stroke ≤ 5pt) must pass through unchanged —
    /// this is every normal ruled table in the wild.
    #[test]
    fn expand_hairline_stroke_leaves_ordinary_rules_alone() {
        let mut path = stroked_line(50.0, 700.0, 322.0, 0.0, 1.0);
        expand_hairline_stroke(&mut path);
        assert_eq!((path.bbox.width, path.bbox.height, path.bbox.y), (322.0, 0.0, 700.0));
    }

    /// A rule with real geometric extent must keep its bbox even when its
    /// stroke is thick: it is already a valid primitive, and widening it
    /// perturbs a grid that already works. Regression test for the
    /// la-precinct fixture (0×507pt column separators stroked 7.2pt wide),
    /// where expansion flipped a correctly detected 47-row results table
    /// into a false-positive header table.
    #[test]
    fn expand_hairline_stroke_leaves_long_thick_stroked_rules_alone() {
        let mut path = stroked_line(212.4, 28.8, 0.0, 507.6, 7.2);
        expand_hairline_stroke(&mut path);
        assert_eq!(
            (path.bbox.x, path.bbox.width, path.bbox.height),
            (212.4, 0.0, 507.6),
            "long stroked rules keep their geometric bbox"
        );
        assert!(path.is_table_primitive(), "still a primitive on geometry alone");
    }

    /// Non-finite geometry or stroke width (NaN glyph pathology, #1198) must
    /// never be expanded — growing a NaN bbox propagates NaN into the
    /// detector's sort keys.
    #[test]
    fn expand_hairline_stroke_skips_non_finite_geometry() {
        let mut nan_box = stroked_line(10.0, 10.0, f32::NAN, 0.0, 430.0);
        expand_hairline_stroke(&mut nan_box);
        assert!(nan_box.bbox.width.is_nan(), "bbox untouched");
        assert_eq!(nan_box.bbox.height, 0.0, "bbox untouched");

        let mut nan_stroke = stroked_line(10.0, 10.0, 1.0, 0.0, f32::NAN);
        expand_hairline_stroke(&mut nan_stroke);
        assert_eq!((nan_stroke.bbox.width, nan_stroke.bbox.height), (1.0, 0.0));
    }

    /// Non-line-like shapes (both dimensions above hairline) and unstroked
    /// paths must pass through unchanged regardless of stroke width.
    #[test]
    fn expand_hairline_stroke_leaves_boxes_and_fills_alone() {
        let mut boxy = stroked_line(50.0, 550.0, 350.0, 200.0, 8.0);
        expand_hairline_stroke(&mut boxy);
        assert_eq!((boxy.bbox.width, boxy.bbox.height), (350.0, 200.0));

        // Fill-only path (the extractor sets stroke_color: None for `f`-painted
        // paths): stroke width is inert without a stroke.
        let mut filled = pdf_oxide::elements::PathContent::new(pdf_oxide::geometry::Rect::new(0.0, 0.0, 1.0, 0.0));
        filled.stroke_color = None;
        filled.fill_color = Some(pdf_oxide::layout::Color::black());
        filled.stroke_width = 430.0;
        expand_hairline_stroke(&mut filled);
        assert_eq!((filled.bbox.width, filled.bbox.height), (1.0, 0.0));
    }

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
            bbox: pdf_oxide::geometry::Rect {
                x,
                y,
                width: 50.0,
                height: 10.0,
            },
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
            heading_level: None,
            rotation_degrees: 0.0,
            ..pdf_oxide::layout::TextSpan::default()
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
            spans: vec![make_span("right", 200.0, 150.0), make_span("left", 10.0, 150.0)],
            bbox: None,
            is_header: false,
        };

        let text = cell_text_in_reading_order(&cell);
        assert_eq!(
            text, "left right",
            "same-row spans must be ordered left-to-right (X ascending); got: {text:?}"
        );
    }

    // ── Heuristic table extraction tests ──

    /// Build a synthetic `HocrWord` for the clustering tests.
    fn make_word(text: &str, left: u32, top: u32, width: u32) -> crate::pdf::table_reconstruct::HocrWord {
        crate::pdf::table_reconstruct::HocrWord {
            text: text.to_string(),
            left,
            top,
            width,
            height: 10,
            confidence: 95.0,
        }
    }

    /// Heuristic table extraction must not panic on an empty / minimal PDF.
    #[test]
    fn test_extract_tables_heuristic_minimal_pdf_no_panic() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/pdf/tiny.pdf");
        if !path.exists() {
            return;
        }
        let bytes = std::fs::read(&path).expect("read tiny.pdf");
        let mut doc = OxideDocument::open_bytes(&bytes).expect("open tiny.pdf");
        let skip = HashSet::new();
        let tables = extract_tables_heuristic(&mut doc, false, &skip).expect("heuristic must not error on minimal PDF");
        assert!(tables.is_empty(), "expected no tables on minimal PDF, got: {tables:?}");
    }

    /// On a real text-layer PDF that contains a table, the heuristic should
    /// produce at least one well-formed `Table` (rows≥2, cols≥2) with a
    /// non-empty markdown rendering. Regression test for #897.
    #[test]
    fn test_extract_tables_heuristic_recovers_table_document() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/pdf/table_document.pdf");
        if !path.exists() {
            return;
        }
        let bytes = std::fs::read(&path).expect("read table_document.pdf");
        let mut doc = OxideDocument::open_bytes(&bytes).expect("open table_document.pdf");

        let skip = HashSet::new();
        let tables = extract_tables_heuristic(&mut doc, false, &skip).expect("heuristic must not error");

        if tables.is_empty() {
            eprintln!(
                "extract_tables_heuristic returned 0 tables on table_document.pdf — \
                 fixture may be borderline for the prose filter; verify manually"
            );
            return;
        }

        for t in &tables {
            assert!(t.cells.len() >= 2, "table has fewer than 2 rows: {t:?}");
            assert!(t.cells.iter().any(|r| r.len() >= 2), "no row has 2+ columns: {t:?}");
            assert!(!t.markdown.trim().is_empty(), "markdown is empty: {t:?}");
            assert!(t.page_number >= 1, "page_number must be 1-indexed: {t:?}");
            // Markdown must contain a header-separator row, which is what makes
            // the output a valid GFM table the rest of the pipeline can consume.
            assert!(
                t.markdown.contains("| --- |") || t.markdown.contains("|---|"),
                "table markdown is missing the header separator row: {:?}",
                t.markdown
            );
            // Bounding box: PDF coords with y0 = bottom edge, y1 = top edge.
            // Catches coordinate inversions; only checked when present.
            if let Some(bbox) = &t.bounding_box {
                assert!(
                    bbox.y0 < bbox.y1,
                    "bbox y0 must be less than y1 (PDF coords: bottom < top): {bbox:?}"
                );
                assert!(bbox.x0 < bbox.x1, "bbox x0 must be less than x1: {bbox:?}");
            }
        }
    }

    /// `skip_pages` (1-indexed) suppresses the heuristic on the listed pages.
    /// This is the per-page composition contract: native finds page N, we
    /// pass `{N}` to the heuristic and it must not emit any tables for page N.
    #[test]
    fn test_extract_tables_heuristic_skip_pages_honored() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/pdf/table_document.pdf");
        if !path.exists() {
            return;
        }
        let bytes = std::fs::read(&path).expect("read table_document.pdf");
        let mut doc = OxideDocument::open_bytes(&bytes).expect("open table_document.pdf");

        let baseline = extract_tables_heuristic(&mut doc, false, &HashSet::new()).expect("baseline heuristic");
        if baseline.is_empty() {
            return; // fixture didn't produce anything to skip — see prior test
        }
        let pages_baseline_touched: HashSet<u32> = baseline.iter().map(|t| t.page_number).collect();

        let skip = pages_baseline_touched.clone();
        let suppressed = extract_tables_heuristic(&mut doc, false, &skip).expect("skip-pages heuristic");

        for t in &suppressed {
            assert!(
                !skip.contains(&t.page_number),
                "page {} appeared in skip set but heuristic still emitted a table: {:?}",
                t.page_number,
                t
            );
        }
    }

    /// Vertical-gap clustering: words all within one band become one region.
    #[test]
    fn test_cluster_words_single_region_not_split() {
        // 6 words in 3 rows, all within ~30px vertically.
        // height=10, row_gap_split = 1.8 * 10 = 18px → 12px row spacing stays in one region.
        let words = vec![
            make_word("a1", 100, 100, 20),
            make_word("a2", 200, 100, 20),
            make_word("b1", 100, 112, 20),
            make_word("b2", 200, 112, 20),
            make_word("c1", 100, 124, 20),
            make_word("c2", 200, 124, 20),
        ];
        let regions = cluster_words_into_vertical_regions(&words);
        assert_eq!(regions.len(), 1, "expected one region, got {regions:?}");
        assert_eq!(regions[0].len(), 6);
    }

    /// Vertical-gap clustering: words separated by a >1.8× median-height gap
    /// must split into two regions. Both tables sized to survive the retain
    /// filter (≥3 distinct rows, ≥2 distinct x-positions, ≥4 words).
    #[test]
    fn test_cluster_words_two_tables_separated_by_large_gap() {
        // First table at y≈100-124 (3 rows), second at y≈300-324 (3 rows).
        // Gap between table 1's last row and table 2's first row = 300-124 = 176px ≫ 18px split.
        let words = vec![
            // Table 1: rows at y=100, y=112, y=124
            make_word("a1", 100, 100, 20),
            make_word("a2", 200, 100, 20),
            make_word("b1", 100, 112, 20),
            make_word("b2", 200, 112, 20),
            make_word("c1", 100, 124, 20),
            make_word("c2", 200, 124, 20),
            // Table 2: rows at y=300, y=312, y=324
            make_word("d1", 100, 300, 20),
            make_word("d2", 200, 300, 20),
            make_word("e1", 100, 312, 20),
            make_word("e2", 200, 312, 20),
            make_word("f1", 100, 324, 20),
            make_word("f2", 200, 324, 20),
        ];
        let regions = cluster_words_into_vertical_regions(&words);
        assert_eq!(regions.len(), 2, "expected two regions, got {regions:?}");
        for r in &regions {
            assert_eq!(r.len(), 6, "each region should have 6 words: {r:?}");
        }
        // First region must precede second (image-coord top, smaller-y first).
        let first_top = regions[0].iter().map(|w| w.top).min().unwrap();
        let second_top = regions[1].iter().map(|w| w.top).min().unwrap();
        assert!(first_top < second_top, "regions should be ordered top-to-bottom");
    }

    /// Clustering must reject single-column (only one distinct x-position)
    /// runs — those are lists, not tables.
    #[test]
    fn test_cluster_words_rejects_single_column_run() {
        let words = vec![
            make_word("item1", 100, 100, 40),
            make_word("item2", 100, 112, 40),
            make_word("item3", 100, 124, 40),
            make_word("item4", 100, 136, 40),
        ];
        let regions = cluster_words_into_vertical_regions(&words);
        assert!(
            regions.is_empty(),
            "single-column word run must not survive clustering, got {regions:?}"
        );
    }

    /// Clustering must reject runs with fewer than 3 distinct rows — too small
    /// to be a real table.
    #[test]
    fn test_cluster_words_rejects_two_row_run() {
        let words = vec![
            make_word("a", 100, 100, 20),
            make_word("b", 200, 100, 20),
            make_word("c", 100, 112, 20),
            make_word("d", 200, 112, 20),
        ];
        let regions = cluster_words_into_vertical_regions(&words);
        assert!(
            regions.is_empty(),
            "two-row run must not survive clustering, got {regions:?}"
        );
    }

    /// Clustering returns an empty vec when fewer than 4 input words are given,
    /// since no realistic table can be reconstructed from that.
    #[test]
    fn test_cluster_words_below_minimum_returns_empty() {
        let words = vec![
            make_word("a", 0, 0, 10),
            make_word("b", 0, 12, 10),
            make_word("c", 0, 24, 10),
        ];
        let regions = cluster_words_into_vertical_regions(&words);
        assert!(regions.is_empty());
    }

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
        assert_eq!(
            text, "hello world",
            "fallback must trim and collapse newlines; got: {text:?}"
        );
    }

    /// Build a minimal single-page PDF with a 2-column stroke-bordered table.
    ///
    /// 5 rows × 2 columns. Every cell boundary is drawn with stroke_rect/stroke_line
    /// so pdf_oxide's detect_tables_with_lines can lock onto the paths.
    fn build_two_column_bordered_table_pdf() -> Vec<u8> {
        use pdf_oxide::geometry::Rect;
        use pdf_oxide::writer::{DocumentBuilder, LineStyle, TextAlign};

        let style = LineStyle::new(1.0, 0.0, 0.0, 0.0);

        // A4: 595 × 842 pt. Table: y=550..750 (PDF y-up coords).
        // 2 columns: left x=50..200, right x=200..400. 5 rows of 40pt each.
        let mut doc = DocumentBuilder::new();
        doc.a4_page()
            .stroke_rect(50.0, 550.0, 350.0, 200.0, style.clone())
            .stroke_line(200.0, 550.0, 200.0, 750.0, style.clone())
            .stroke_line(50.0, 710.0, 400.0, 710.0, style.clone())
            .stroke_line(50.0, 670.0, 400.0, 670.0, style.clone())
            .stroke_line(50.0, 630.0, 400.0, 630.0, style.clone())
            .stroke_line(50.0, 590.0, 400.0, 590.0, style.clone())
            .text_in_rect(Rect::new(50.0, 710.0, 150.0, 40.0), "Item", TextAlign::Left)
            .text_in_rect(Rect::new(200.0, 710.0, 200.0, 40.0), "Status", TextAlign::Left)
            .text_in_rect(Rect::new(50.0, 670.0, 150.0, 40.0), "8", TextAlign::Left)
            .text_in_rect(Rect::new(200.0, 670.0, 200.0, 40.0), "Not correct", TextAlign::Left)
            .text_in_rect(Rect::new(50.0, 630.0, 150.0, 40.0), "27", TextAlign::Left)
            .text_in_rect(Rect::new(200.0, 630.0, 200.0, 40.0), "Incomplete", TextAlign::Left)
            .text_in_rect(Rect::new(50.0, 590.0, 150.0, 40.0), "29,30", TextAlign::Left)
            .text_in_rect(Rect::new(200.0, 590.0, 200.0, 40.0), "Missing data", TextAlign::Left)
            .text_in_rect(Rect::new(50.0, 550.0, 150.0, 40.0), "45", TextAlign::Left)
            .text_in_rect(Rect::new(200.0, 550.0, 200.0, 40.0), "Fixed", TextAlign::Left)
            .done();
        doc.build().expect("DocumentBuilder must produce valid PDF bytes")
    }

    /// `extract_tables_native` (strict, min_table_columns=3) must NOT detect a
    /// 2-column bordered table. Regression baseline for issue #964.
    #[test]
    fn extract_tables_native_misses_two_column_bordered_table() {
        let bytes = build_two_column_bordered_table_pdf();
        let mut doc = OxideDocument::open_bytes(&bytes).expect("open synthetic PDF");
        let tables = extract_tables_native(&mut doc).expect("extract_tables_native must not error");
        assert!(
            tables.is_empty(),
            "extract_tables_native (strict, min 3 cols) must not detect a 2-column table; got: {tables:?}"
        );
    }

    /// `extract_tables_bordered` (relaxed, min_table_columns=2) must detect the
    /// 2-column stroke-bordered table that `extract_tables_native` skips.
    #[test]
    fn extract_tables_bordered_detects_two_column_bordered_table() {
        let bytes = build_two_column_bordered_table_pdf();
        let mut doc = OxideDocument::open_bytes(&bytes).expect("open synthetic PDF");
        let skip = HashSet::new();
        let tables = extract_tables_bordered(&mut doc, &skip).expect("extract_tables_bordered must not error");
        assert!(
            !tables.is_empty(),
            "extract_tables_bordered must detect the 2-column stroke-bordered table"
        );
        let table = &tables[0];
        assert_eq!(table.cells.len(), 5, "expected 5 rows, got {}", table.cells.len());
        assert!(
            table.cells.iter().all(|row| row.len() == 2),
            "all rows must have 2 columns; rows: {:?}",
            table.cells.iter().map(|r| r.len()).collect::<Vec<_>>()
        );
        assert_eq!(table.page_number, 1);
        assert!(!table.markdown.trim().is_empty(), "must produce non-empty markdown");
    }

    /// `extract_tables_bordered` must skip pages listed in `skip_pages`.
    #[test]
    fn extract_tables_bordered_skip_pages_honored() {
        let bytes = build_two_column_bordered_table_pdf();
        let mut doc = OxideDocument::open_bytes(&bytes).expect("open synthetic PDF");
        let mut skip = HashSet::new();
        skip.insert(1u32);
        let tables = extract_tables_bordered(&mut doc, &skip).expect("extract_tables_bordered must not error");
        assert!(
            tables.is_empty(),
            "skip_pages={{1}} must suppress the only page; got: {tables:?}"
        );
    }

    /// Build a 3-column bordered table whose vertical rules are drawn the way
    /// some print-era PDF generators emit them: a ~1pt horizontal segment
    /// stroked with a line width equal to the table height, which renders as a
    /// full-height vertical bar. Geometrically the path is a speck (bbox
    /// 1×0pt), so a stroke-unaware reader sees no vertical rulings at all.
    /// Mirrors the fuse-chart tables in xberg-io/xberg#1213.
    fn build_stroke_width_vertical_rules_table_pdf() -> Vec<u8> {
        use pdf_oxide::geometry::Rect;
        use pdf_oxide::writer::{DocumentBuilder, LineStyle, TextAlign};

        let thin = LineStyle::new(1.0, 0.0, 0.0, 0.0);
        // Table: x=50..400, y=510..750 (PDF y-up), 6 rows of 40pt.
        // Column edges at x = 50, 150, 250, 400.
        let rows: [[&str; 3]; 6] = [
            ["Location", "Rating", "Circuit"],
            ["6", "15A*", "Alternator regulator"],
            ["7", "30A*", "PCM relay feed"],
            ["11", "15A*", "A/C clutch relay feed"],
            ["24", "10A*", "Heated mirrors"],
            ["101", "40A**", "Blower relay feed"],
        ];

        let mut doc = DocumentBuilder::new();
        let mut page = doc.a4_page();
        // Horizontal rules: ordinary 1pt strokes at every row boundary.
        for i in 0..=6u32 {
            let y = 510.0 + 40.0 * i as f32;
            page = page.stroke_line(50.0, y, 400.0, y, thin.clone());
        }
        // Vertical rules: 1pt-long horizontal segments at the table's vertical
        // midpoint, stroked with the full table height (240pt).
        for x in [50.0_f32, 150.0, 250.0, 400.0] {
            page = page.stroke_line(x - 0.5, 630.0, x + 0.5, 630.0, LineStyle::new(240.0, 0.0, 0.0, 0.0));
        }
        // Cell text, top row first (row i occupies y = 750-40*(i+1) .. 750-40*i).
        let col_x = [50.0_f32, 150.0, 250.0];
        let col_w = [100.0_f32, 100.0, 150.0];
        for (i, row) in rows.iter().enumerate() {
            let y = 750.0 - 40.0 * (i as f32 + 1.0);
            for (c, text) in row.iter().enumerate() {
                page = page.text_in_rect(Rect::new(col_x[c], y, col_w[c], 40.0), text, TextAlign::Left);
            }
        }
        page.done();
        doc.build().expect("DocumentBuilder must produce valid PDF bytes")
    }

    /// A 3-column grid whose vertical rules are stroke-width-rendered must be
    /// detected by `extract_tables_native` (strict tier) with its rows intact.
    /// Regression test for xberg-io/xberg#1213: previously the vertical rules'
    /// geometry-only bboxes (1×0pt) failed `is_table_primitive`, the detector
    /// saw no vertical rulings, and the table text flowed out column-major.
    #[test]
    fn extract_tables_native_detects_stroke_width_vertical_rules_table() {
        let bytes = build_stroke_width_vertical_rules_table_pdf();
        let mut doc = OxideDocument::open_bytes(&bytes).expect("open synthetic PDF");
        let tables = extract_tables_native(&mut doc).expect("extract_tables_native must not error");
        assert!(
            !tables.is_empty(),
            "extract_tables_native must detect the stroke-width-ruled 3-column table"
        );
        let table = &tables[0];
        assert_eq!(table.page_number, 1);
        assert!(
            table.cells.iter().all(|row| row.len() == 3),
            "all rows must have 3 columns; rows: {:?}",
            table.cells.iter().map(|r| r.len()).collect::<Vec<_>>()
        );
        let fuse_row: Vec<&str> = vec!["101", "40A**", "Blower relay feed"];
        assert!(
            table
                .cells
                .iter()
                .any(|row| row.iter().map(String::as_str).collect::<Vec<_>>() == fuse_row),
            "row association must survive: expected a row {fuse_row:?}; got cells: {:?}",
            table.cells
        );
    }
}
