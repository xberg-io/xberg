//! Unruled value columns set beside a ruled table.
//!
//! A rate table can rule its main grid and set a narrow block of value columns to its right
//! without rules (pb_SERFF_TX_random_pages_1_page1075: `2-10yr BP` and `LifeBP` multipliers beside
//! a fifteen-column ruled grid). The ruled detector ends the table at its last rule, so each row's
//! values were left as a paragraph ahead of the table. Spans right of the table that sit in its
//! rows and form value tracks are read as further columns of it.

use xberg_native_pdf::elements::PathContent;
use xberg_native_pdf::geometry::Rect;
use xberg_native_pdf::layout::TextSpan;
use xberg_native_pdf::structure::table_extractor::{Table, TableCell, TableRow};

use super::is_numeric_word;

/// Most value columns read beside a table; a wider block is a table of its own.
const MAX_OPEN_COLUMNS: usize = 4;
/// Fewest rows that must hold an open value.
const MIN_VALUE_ROWS: usize = 3;
/// Share of the value rows that must fill every open column.
const MIN_FULL_ROW_PERCENT: usize = 80;
/// Thickness below which a path is a rule.
const RULE_TOLERANCE: f32 = 2.0;

/// A row's vertical extent in PDF space: bottom and top.
type Band = (f32, f32);
/// A value column's horizontal extent: left and right.
type Track = (f32, f32);

/// Append the unruled value columns right of `table` that share its rows, or leave it unchanged.
///
/// Every span between the table's right edge and the last value column, within the table's
/// height, must land in a row and a column: the table's new bounds suppress the text they cover,
/// so a span left out would be lost.
pub(super) fn restore_open_trailing_columns(table: &mut Table, spans: &[TextSpan], paths: &[PathContent]) {
    let Some(bounds) = table.bbox else { return };
    if table.col_count == 0 || table.rows.is_empty() {
        return;
    }
    let Some(bands) = table.rows.iter().map(row_band).collect::<Option<Vec<_>>>() else {
        return;
    };
    let candidates: Vec<&TextSpan> = spans
        .iter()
        .filter(|span| {
            span.rotation_degrees == 0.0
                && !span.text.trim().is_empty()
                && span.bbox.x >= bounds.right()
                && (bounds.y..=bounds.y + bounds.height).contains(&center_y(span))
        })
        .collect();
    let tracks = value_tracks(&candidates);
    if tracks.is_empty() || tracks.len() > MAX_OPEN_COLUMNS {
        return;
    }
    let column_width = bounds.width / table.col_count as f32;
    let mut previous_right = bounds.right();
    for &(left, right) in &tracks {
        if left - previous_right > column_width {
            return;
        }
        previous_right = right;
    }
    let first_left = tracks[0].0;
    if candidates
        .iter()
        .any(|span| span.bbox.x > previous_right || span.bbox.right() < first_left - column_width)
    {
        return;
    }
    let right = candidates
        .iter()
        .map(|span| span.bbox.right())
        .fold(previous_right, f32::max);
    if has_vertical_rule(paths, bounds, right) {
        return;
    }

    let mut cells: Vec<Vec<Vec<&TextSpan>>> = vec![vec![Vec::new(); tracks.len()]; table.rows.len()];
    for &span in &candidates {
        let Some(row) = containing_band(&bands, center_y(span)) else {
            return;
        };
        cells[row][nearest_track(&tracks, span)].push(span);
    }
    if !fills_value_rows(&cells, tracks.len(), table.rows.len()) {
        return;
    }

    for ((row, &band), row_cells) in table.rows.iter_mut().zip(&bands).zip(cells) {
        for (&track, members) in tracks.iter().zip(row_cells) {
            let cell = open_cell(row.is_header, band, track, members);
            row.cells.push(cell);
        }
    }
    table.col_count += tracks.len();
    table.bbox = Some(Rect::new(bounds.x, bounds.y, right - bounds.x, bounds.height));
}

fn center_y(span: &TextSpan) -> f32 {
    span.bbox.y + span.bbox.height / 2.0
}

fn is_value(span: &TextSpan) -> bool {
    is_numeric_word(span.text.trim())
}

/// The vertical extent of a row's cells, or `None` when no cell has bounds.
fn row_band(row: &TableRow) -> Option<Band> {
    row.cells
        .iter()
        .filter_map(|cell| cell.bbox)
        .map(|bbox| (bbox.y, bbox.y + bbox.height))
        .reduce(|(bottom, top), (cell_bottom, cell_top)| (bottom.min(cell_bottom), top.max(cell_top)))
}

/// The columns the value spans form: spans whose horizontal extents overlap share a column.
fn value_tracks(candidates: &[&TextSpan]) -> Vec<Track> {
    let mut values: Vec<Track> = candidates
        .iter()
        .filter(|span| is_value(span))
        .map(|span| (span.bbox.x, span.bbox.right()))
        .collect();
    values.sort_by(|left, right| left.0.total_cmp(&right.0));
    let mut tracks: Vec<Track> = Vec::new();
    for (left, right) in values {
        match tracks.last_mut() {
            Some(track) if left < track.1 => track.1 = track.1.max(right),
            _ => tracks.push((left, right)),
        }
    }
    tracks
}

fn containing_band(bands: &[Band], y: f32) -> Option<usize> {
    bands
        .iter()
        .enumerate()
        .filter(|(_, (bottom, top))| (*bottom..=*top).contains(&y))
        .min_by(|(_, left), (_, right)| {
            let distance = |(bottom, top): &Band| ((bottom + top) / 2.0 - y).abs();
            distance(left).total_cmp(&distance(right))
        })
        .map(|(index, _)| index)
}

fn nearest_track(tracks: &[Track], span: &TextSpan) -> usize {
    let center = span.bbox.x + span.bbox.width / 2.0;
    tracks
        .iter()
        .enumerate()
        .min_by(|(_, left), (_, right)| {
            let distance = |(start, end): &Track| ((start + end) / 2.0 - center).abs();
            distance(left).total_cmp(&distance(right))
        })
        .map_or(0, |(index, _)| index)
}

/// Whether the open values read as columns of the table's rows: at most one value per cell, in
/// enough of the rows, with most value rows filling every column.
fn fills_value_rows(cells: &[Vec<Vec<&TextSpan>>], columns: usize, table_rows: usize) -> bool {
    let mut value_rows = 0;
    let mut full_rows = 0;
    for row in cells {
        if row
            .iter()
            .any(|cell| cell.iter().filter(|span| is_value(span)).count() > 1)
        {
            return false;
        }
        let values = row.iter().filter(|cell| cell.iter().any(|span| is_value(span))).count();
        if values == 0 {
            continue;
        }
        value_rows += 1;
        full_rows += usize::from(values == columns);
    }
    value_rows >= MIN_VALUE_ROWS && value_rows * 2 >= table_rows && full_rows * 100 >= value_rows * MIN_FULL_ROW_PERCENT
}

/// Whether a vertical rule crosses the table's height right of its edge, so the text there is
/// ruled off from it rather than open.
fn has_vertical_rule(paths: &[PathContent], bounds: Rect, right: f32) -> bool {
    paths.iter().any(|path| {
        let rule = path.rendered_bbox();
        path.is_vertical_line(RULE_TOLERANCE)
            && rule.x > bounds.right() + RULE_TOLERANCE
            && rule.x <= right
            && rule.y < bounds.y + bounds.height
            && rule.y + rule.height > bounds.y
    })
}

fn open_cell(is_header: bool, (bottom, top): Band, (left, right): Track, mut members: Vec<&TextSpan>) -> TableCell {
    members.sort_by(|upper, lower| {
        lower
            .bbox
            .y
            .total_cmp(&upper.bbox.y)
            .then(upper.bbox.x.total_cmp(&lower.bbox.x))
    });
    TableCell {
        text: members
            .iter()
            .map(|span| span.text.split_whitespace().collect::<Vec<_>>().join(" "))
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join(" "),
        mcids: members.iter().filter_map(|span| span.mcid).collect(),
        spans: members.into_iter().cloned().collect(),
        colspan: 1,
        rowspan: 1,
        bbox: Some(Rect::new(left, bottom, right - left, top - bottom)),
        is_header,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROW_HEIGHT: f32 = 12.0;
    const TABLE_TOP: f32 = 700.0;

    fn span(text: &str, x: f32, baseline: f32, width: f32) -> TextSpan {
        TextSpan {
            text: text.to_string(),
            bbox: Rect::new(x, baseline, width, 8.0),
            font_size: 8.0,
            ..TextSpan::default()
        }
    }

    fn baseline(row: usize) -> f32 {
        TABLE_TOP - (row as f32 + 1.0) * ROW_HEIGHT + 2.0
    }

    /// A ruled two-column table: a header row, then `rows` value rows.
    fn ruled_table(rows: usize) -> Table {
        let cell = |text: &str, x: f32, row: usize| TableCell {
            text: text.to_string(),
            spans: vec![span(text, x + 2.0, baseline(row), 20.0)],
            colspan: 1,
            rowspan: 1,
            mcids: Vec::new(),
            bbox: Some(Rect::new(
                x,
                TABLE_TOP - (row as f32 + 1.0) * ROW_HEIGHT,
                50.0,
                ROW_HEIGHT,
            )),
            is_header: row == 0,
        };
        let rows: Vec<TableRow> = (0..=rows)
            .map(|row| {
                let (label, value) = if row == 0 {
                    ("Age".to_string(), "Rate".to_string())
                } else {
                    (format!("{}", 30 + row), format!("{}.00", 10 + row))
                };
                TableRow {
                    cells: vec![cell(&label, 100.0, row), cell(&value, 150.0, row)],
                    is_header: row == 0,
                }
            })
            .collect();
        let height = rows.len() as f32 * ROW_HEIGHT;
        Table {
            rows,
            has_header: true,
            col_count: 2,
            bbox: Some(Rect::new(100.0, TABLE_TOP - height, 100.0, height)),
        }
    }

    fn open_spans(rows: usize) -> Vec<TextSpan> {
        let mut spans = vec![
            span("Factor", 222.0, baseline(0), 22.0),
            span("LifeBP", 262.0, baseline(0), 22.0),
        ];
        for row in 1..=rows {
            spans.push(span(&format!("1.{row:02}"), 225.0, baseline(row), 16.0));
            spans.push(span("1.00", 265.0, baseline(row), 16.0));
        }
        spans
    }

    fn texts(table: &Table) -> Vec<Vec<&str>> {
        table
            .rows
            .iter()
            .map(|row| row.cells.iter().map(|cell| cell.text.as_str()).collect())
            .collect()
    }

    #[test]
    fn unruled_value_columns_beside_a_ruled_table_become_its_columns() {
        let mut table = ruled_table(4);

        restore_open_trailing_columns(&mut table, &open_spans(4), &[]);

        assert_eq!(table.col_count, 4);
        assert_eq!(
            texts(&table),
            [
                ["Age", "Rate", "Factor", "LifeBP"],
                ["31", "11.00", "1.01", "1.00"],
                ["32", "12.00", "1.02", "1.00"],
                ["33", "13.00", "1.03", "1.00"],
                ["34", "14.00", "1.04", "1.00"],
            ]
        );
        let bounds = table.bbox.expect("bounds");
        assert_eq!(bounds.x, 100.0);
        assert_eq!(bounds.right(), 284.0);
    }

    #[test]
    fn a_vertical_rule_keeps_the_values_out_of_the_table() {
        let mut table = ruled_table(4);
        let rule = PathContent::line(215.0, TABLE_TOP - 60.0, 215.0, TABLE_TOP);
        assert!(rule.is_vertical_line(RULE_TOLERANCE), "fixture must be a rule");

        restore_open_trailing_columns(&mut table, &open_spans(4), &[rule]);

        assert_eq!(table.col_count, 2);
    }

    #[test]
    fn values_far_right_of_the_table_are_not_its_columns() {
        let mut table = ruled_table(4);
        let spans: Vec<TextSpan> = open_spans(4)
            .into_iter()
            .map(|mut span| {
                span.bbox.x += 200.0;
                span
            })
            .collect();

        restore_open_trailing_columns(&mut table, &spans, &[]);

        assert_eq!(table.col_count, 2);
    }

    #[test]
    fn a_stray_value_between_the_columns_keeps_the_table_unchanged() {
        let mut table = ruled_table(4);
        let mut spans = open_spans(4);
        spans.push(span("1.50", 245.0, baseline(2), 16.0));

        restore_open_trailing_columns(&mut table, &spans, &[]);

        assert_eq!(table.col_count, 2);
    }
}
