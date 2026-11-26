//! Table header bands whose cells stack a different number of label lines, each cell centred
//! on the band: "UW Tier / Group" beside "Minimum / Vehicle / Miles" beside "BI". The cells'
//! lines interleave into rows closer together than any line of text, so a row-by-row read
//! scatters every cell's label across the band (pb_sample_page_16). Such a band is read cell
//! by cell: the spans are grouped into columns by horizontal overlap, the columns are read left
//! to right and each column top to bottom. Table reconstruction reads the same band as one
//! header row by the same rule (`table_core::overlap`).

use xberg_native_pdf::layout::TextSpan;

use super::super::text::span_has_ink;
use crate::table_core::overlap::{
    STACKED_BAND_MAX_ROW_STEP, STACKED_BAND_MIN_ROW_WORDS, STACKED_BAND_MIN_ROWS, STACKED_BAND_MIN_STACKED_COLUMNS,
    extents_overlap, overlap_columns,
};

/// Baseline difference, as a fraction of the glyph height, within which consecutive spans share a row.
const SAME_ROW_FRACTION: f32 = 0.2;
/// Largest ratio between the tallest and the shortest glyph height in a band; raised and
/// lowered script marks are smaller than their line.
const MAX_BAND_HEIGHT_RATIO: f32 = 1.2;

/// Reorder every stacked header band in `spans` cell by cell, leaving all other spans in place.
pub(super) fn read_stacked_header_bands(spans: &mut [TextSpan]) {
    let rows = sequence_rows(spans);
    let mut first = 0;
    while first < rows.len() {
        let mut last = first;
        while last + 1 < rows.len() && rows_stack(spans, &rows[last], &rows[last + 1]) {
            last += 1;
        }
        if last + 1 - first >= STACKED_BAND_MIN_ROWS {
            let start = rows[first].start;
            let end = rows[last].end;
            reorder_band(&mut spans[start..end]);
        }
        first = last + 1;
    }
}

/// A run of consecutive spans on one baseline.
struct Row {
    start: usize,
    end: usize,
}

fn glyph_height(span: &TextSpan) -> f32 {
    span.bbox.height
}

fn is_upright(span: &TextSpan) -> bool {
    span.rotation_degrees == 0.0
        && span.bbox.y.is_finite()
        && span.bbox.x.is_finite()
        && span.bbox.width.is_finite()
        && glyph_height(span).is_finite()
        && glyph_height(span) > 0.0
}

/// Split `spans` into runs of consecutive ink spans on one baseline; a whitespace span joins
/// the row it follows.
fn sequence_rows(spans: &[TextSpan]) -> Vec<Row> {
    let mut rows: Vec<Row> = Vec::new();
    let mut row_anchor: Option<usize> = None;
    for (index, span) in spans.iter().enumerate() {
        if !span_has_ink(span)
            && let Some(row) = rows.last_mut()
        {
            row.end = index + 1;
            continue;
        }
        let joins = row_anchor.is_some_and(|anchor| {
            let anchor = &spans[anchor];
            span_has_ink(span)
                && (span.bbox.y - anchor.bbox.y).abs()
                    <= SAME_ROW_FRACTION * glyph_height(anchor).min(glyph_height(span))
        });
        if joins && let Some(row) = rows.last_mut() {
            row.end = index + 1;
            continue;
        }
        rows.push(Row {
            start: index,
            end: index + 1,
        });
        row_anchor = span_has_ink(span).then_some(index);
    }
    rows
}

fn ink_spans<'a>(spans: &'a [TextSpan], row: &Row) -> Vec<&'a TextSpan> {
    spans[row.start..row.end]
        .iter()
        .filter(|span| span_has_ink(span))
        .collect()
}

/// True when `lower` is the next row of a stacked band that `upper` belongs to: both rows hold
/// several upright spans of one glyph height, `lower` sits less than a line below `upper`, and
/// no span of one row overlaps a span of the other horizontally, as the lines of one column
/// that close together would.
fn rows_stack(spans: &[TextSpan], upper: &Row, lower: &Row) -> bool {
    let (upper, lower) = (ink_spans(spans, upper), ink_spans(spans, lower));
    if upper.len() < STACKED_BAND_MIN_ROW_WORDS || lower.len() < STACKED_BAND_MIN_ROW_WORDS {
        return false;
    }
    if !upper.iter().chain(&lower).all(|span| is_upright(span)) {
        return false;
    }
    let heights = upper.iter().chain(&lower).map(|span| glyph_height(span));
    let (shortest, tallest) = heights.fold((f32::INFINITY, 0.0_f32), |(low, high), height| {
        (low.min(height), high.max(height))
    });
    if tallest > MAX_BAND_HEIGHT_RATIO * shortest {
        return false;
    }
    let step = upper[0].bbox.y - lower[0].bbox.y;
    let extent = |span: &&TextSpan| (span.bbox.x, span.bbox.x + span.bbox.width);
    step > 0.0
        && step < STACKED_BAND_MAX_ROW_STEP * shortest
        && !extents_overlap(upper.iter().map(extent), lower.iter().map(extent))
}

/// Read `band` column by column: spans whose horizontal extents overlap share a column, the
/// columns go left to right and each column top to bottom.
fn reorder_band(band: &mut [TextSpan]) {
    let extents: Vec<(f32, f32)> = band
        .iter()
        .map(|span| {
            let width = if span_has_ink(span) {
                span.bbox.width.max(0.0)
            } else {
                0.0
            };
            (span.bbox.x, span.bbox.x + width)
        })
        .collect();
    let columns = overlap_columns(&extents);
    let stacked = columns
        .iter()
        .filter(|column| {
            column
                .members
                .iter()
                .filter(|&&index| span_has_ink(&band[index]))
                .count()
                >= 2
        })
        .count();
    if stacked < STACKED_BAND_MIN_STACKED_COLUMNS {
        return;
    }
    let mut reading: Vec<usize> = Vec::with_capacity(band.len());
    for column in columns {
        let mut members = column.members;
        members.sort_by(|&upper, &lower| {
            band[lower]
                .bbox
                .y
                .total_cmp(&band[upper].bbox.y)
                .then(band[upper].bbox.x.total_cmp(&band[lower].bbox.x))
        });
        reading.extend(members);
    }
    let mut taken: Vec<TextSpan> = band.iter_mut().map(std::mem::take).collect();
    for (slot, index) in band.iter_mut().zip(reading) {
        *slot = std::mem::take(&mut taken[index]);
    }
}

#[cfg(test)]
mod tests {
    use xberg_native_pdf::geometry::Rect;

    use super::*;
    use crate::pdf::native::hierarchy::test_spans::{rows, texts};

    const GLYPH_HEIGHT: f32 = 4.3;
    const BAND_STEP: f32 = 2.8;
    const BAND_TOP: f32 = 704.5;

    fn span(text: &str, x: f32, y: f32, width: f32) -> TextSpan {
        super::super::test_spans::span(text, x, y, width, GLYPH_HEIGHT)
    }

    /// The pb_sample_page_16 header band: two-line, three-line and one-line cells, each centred
    /// on the band, above a data row. Every band row holds at least two cells' lines.
    fn header_band_page() -> Vec<TextSpan> {
        let row = |line: usize| BAND_TOP - line as f32 * BAND_STEP;
        vec![
            span("Rating Exhibit", 200.0, BAND_TOP + 20.0, 60.0),
            span("UW Tier", 25.4, row(1), 14.4),
            span("Group", 27.1, row(3), 11.2),
            span("Minimum", 71.5, row(0), 17.2),
            span("Vehicle", 73.8, row(2), 12.7),
            span("Miles", 75.4, row(4), 9.4),
            span("Maximum", 94.9, row(0), 17.9),
            span("Vehicle", 97.6, row(2), 12.7),
            span("Miles", 99.1, row(4), 9.5),
            span("BI", 125.9, row(2), 3.5),
            span("Comp Auto", 150.5, row(1), 19.8),
            span("Loan /Lease", 149.9, row(3), 20.8),
            span("002", 29.5, row(4) - 7.5, 6.5),
            span("1000", 76.0, row(4) - 7.5, 8.6),
            span("1.000", 122.4, row(4) - 7.5, 10.8),
            span("1.000", 155.0, row(4) - 7.5, 10.8),
        ]
    }

    #[test]
    fn a_stacked_header_band_is_read_cell_by_cell() {
        let mut spans = rows(header_band_page());

        read_stacked_header_bands(&mut spans);

        assert_eq!(
            texts(&spans),
            [
                "Rating Exhibit",
                "UW Tier",
                "Group",
                "Minimum",
                "Vehicle",
                "Miles",
                "Maximum",
                "Vehicle",
                "Miles",
                "BI",
                "Comp Auto",
                "Loan /Lease",
                "002",
                "1000",
                "1.000",
                "1.000",
            ]
        );
    }

    #[test]
    fn lines_a_full_line_apart_keep_their_order() {
        let mut spans = Vec::new();
        for line in 0..4 {
            let y = BAND_TOP - line as f32 * GLYPH_HEIGHT * 1.2;
            spans.push(span(&format!("label {line}"), 20.0, y, 30.0));
            spans.push(span(&format!("value {line}"), 80.0, y, 30.0));
        }
        let before: Vec<String> = spans.iter().map(|span| span.text.clone()).collect();

        read_stacked_header_bands(&mut spans);

        assert_eq!(texts(&spans), before);
    }

    #[test]
    fn raised_and_lowered_marks_around_a_line_keep_their_order() {
        let mut spans = rows(vec![
            span("x", 20.0, BAND_TOP, 4.0),
            span("+ y", 40.0, BAND_TOP, 8.0),
            TextSpan {
                bbox: Rect::new(24.0, BAND_TOP + 1.5, 2.0, GLYPH_HEIGHT * 0.6),
                ..span("2", 0.0, 0.0, 0.0)
            },
            TextSpan {
                bbox: Rect::new(48.0, BAND_TOP + 1.5, 2.0, GLYPH_HEIGHT * 0.6),
                ..span("2", 0.0, 0.0, 0.0)
            },
            TextSpan {
                bbox: Rect::new(24.0, BAND_TOP - 1.5, 2.0, GLYPH_HEIGHT * 0.6),
                ..span("i", 0.0, 0.0, 0.0)
            },
            TextSpan {
                bbox: Rect::new(48.0, BAND_TOP - 1.5, 2.0, GLYPH_HEIGHT * 0.6),
                ..span("j", 0.0, 0.0, 0.0)
            },
        ]);
        let before: Vec<String> = spans.iter().map(|span| span.text.clone()).collect();

        read_stacked_header_bands(&mut spans);

        assert_eq!(texts(&spans), before);
    }

    #[test]
    fn a_band_without_stacked_cells_keeps_its_order() {
        let row = |line: usize| BAND_TOP - line as f32 * BAND_STEP;
        let mut spans = rows(vec![
            span("one", 20.0, row(0), 10.0),
            span("two", 40.0, row(0), 10.0),
            span("three", 60.0, row(1), 10.0),
            span("four", 80.0, row(1), 10.0),
            span("five", 100.0, row(2), 10.0),
            span("six", 120.0, row(2), 10.0),
        ]);
        let before: Vec<String> = spans.iter().map(|span| span.text.clone()).collect();

        read_stacked_header_bands(&mut spans);

        assert_eq!(texts(&spans), before);
    }

    /// Tightly set rows of one two-column grid: each row's spans overlap the next row's, so
    /// they are lines of shared columns, not a band of interleaved cell labels.
    #[test]
    fn close_rows_sharing_columns_keep_their_order() {
        let row = |line: usize| BAND_TOP - line as f32 * BAND_STEP;
        let mut spans = rows(
            (0..4)
                .flat_map(|line| {
                    [
                        span(&format!("label {line}"), 20.0, row(line), 30.0),
                        span(&format!("value {line}"), 80.0, row(line), 30.0),
                    ]
                })
                .collect(),
        );
        let before: Vec<String> = spans.iter().map(|span| span.text.clone()).collect();

        read_stacked_header_bands(&mut spans);

        assert_eq!(texts(&spans), before);
    }
}
