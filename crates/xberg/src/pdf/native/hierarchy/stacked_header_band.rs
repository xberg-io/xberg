//! Table header bands whose cells stack a different number of label lines, each cell centred
//! on the band: "UW Tier / Group" beside "Minimum / Vehicle / Miles" beside "BI". The cells'
//! lines interleave into rows closer together than any line of text, so a row-by-row read
//! scatters every cell's label across the band (pb_sample_page_16). Such a band is read cell
//! by cell: the spans are grouped into columns by horizontal overlap, the columns are read left
//! to right and each column top to bottom.

use xberg_native_pdf::layout::TextSpan;

/// Largest baseline step between two rows of a band, as a fraction of the rows' glyph height.
/// Lines of text step at least their glyph height; the stacked band measured 0.63 to 0.67.
const MAX_BAND_ROW_STEP_FRACTION: f32 = 0.7;
/// Baseline difference, as a fraction of the glyph height, within which consecutive spans share a row.
const SAME_ROW_FRACTION: f32 = 0.2;
/// Fewest rows in a band.
const MIN_BAND_ROWS: usize = 3;
/// Fewest spans in each row of a band.
const MIN_ROW_SPANS: usize = 2;
/// Fewest columns of two or more stacked spans in a band.
const MIN_STACKED_COLUMNS: usize = 2;
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
        if last + 1 - first >= MIN_BAND_ROWS {
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

fn is_ink(span: &TextSpan) -> bool {
    !span.text.trim().is_empty()
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
        if !is_ink(span)
            && let Some(row) = rows.last_mut()
        {
            row.end = index + 1;
            continue;
        }
        let joins = row_anchor.is_some_and(|anchor| {
            let anchor = &spans[anchor];
            is_ink(span)
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
        row_anchor = is_ink(span).then_some(index);
    }
    rows
}

fn ink_spans<'a>(spans: &'a [TextSpan], row: &Row) -> Vec<&'a TextSpan> {
    spans[row.start..row.end].iter().filter(|span| is_ink(span)).collect()
}

/// True when `lower` is the next row of a stacked band that `upper` belongs to: both rows hold
/// several upright spans of one glyph height and `lower` sits less than a line below `upper`.
fn rows_stack(spans: &[TextSpan], upper: &Row, lower: &Row) -> bool {
    let (upper, lower) = (ink_spans(spans, upper), ink_spans(spans, lower));
    if upper.len() < MIN_ROW_SPANS || lower.len() < MIN_ROW_SPANS {
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
    step > 0.0 && step < MAX_BAND_ROW_STEP_FRACTION * shortest
}

/// Read `band` column by column: spans whose horizontal extents overlap share a column, the
/// columns go left to right and each column top to bottom.
fn reorder_band(band: &mut [TextSpan]) {
    let extent = |span: &TextSpan| {
        let width = if is_ink(span) { span.bbox.width.max(0.0) } else { 0.0 };
        (span.bbox.x, span.bbox.x + width)
    };
    let mut order: Vec<usize> = (0..band.len()).collect();
    order.sort_by(|&left, &right| extent(&band[left]).0.total_cmp(&extent(&band[right]).0));
    let mut columns: Vec<(Vec<usize>, f32)> = Vec::new();
    for index in order {
        let (left, right) = extent(&band[index]);
        if let Some((members, column_right)) = columns.last_mut()
            && left < *column_right
        {
            members.push(index);
            *column_right = column_right.max(right);
            continue;
        }
        columns.push((vec![index], right));
    }
    let stacked = columns
        .iter()
        .filter(|(members, _)| members.iter().filter(|&&index| is_ink(&band[index])).count() >= 2)
        .count();
    if stacked < MIN_STACKED_COLUMNS {
        return;
    }
    let mut reading: Vec<usize> = Vec::with_capacity(band.len());
    for (mut members, _) in columns {
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

    const GLYPH_HEIGHT: f32 = 4.3;
    const BAND_STEP: f32 = 2.8;
    const BAND_TOP: f32 = 704.5;

    fn span(text: &str, x: f32, y: f32, width: f32) -> TextSpan {
        TextSpan {
            text: text.to_string(),
            bbox: Rect::new(x, y, width, GLYPH_HEIGHT),
            font_size: GLYPH_HEIGHT,
            ..TextSpan::default()
        }
    }

    /// Top to bottom, then left to right: the order the extractor hands over.
    fn rows(mut spans: Vec<TextSpan>) -> Vec<TextSpan> {
        spans.sort_by(|left, right| {
            right
                .bbox
                .y
                .total_cmp(&left.bbox.y)
                .then(left.bbox.x.total_cmp(&right.bbox.x))
        });
        spans
    }

    fn texts(spans: &[TextSpan]) -> Vec<&str> {
        spans.iter().map(|span| span.text.as_str()).collect()
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
}
