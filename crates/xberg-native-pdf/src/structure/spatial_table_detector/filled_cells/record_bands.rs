use super::{
    MAX_PATHS, MAX_WORDS, PathContent, Rect, Table, TableDetectionConfig, TextSpan, Work, axis_aligned_fill, finite,
    maximal_cells, owns_word, same, tiled_rows, unique_owner,
};
use crate::structure::table_extractor::TableRow;

pub(in super::super) fn preserve(
    rows: &[TableRow],
    assigned: &[Vec<Vec<usize>>],
    spans: &[TextSpan],
    paths: &[PathContent],
    config: &TableDetectionConfig,
) -> Option<Table> {
    if paths.len() > MAX_PATHS
        || spans.len() > MAX_WORDS
        || rows.len() < 5
        || !rows.iter().take(5).enumerate().all(|(index, row)| {
            let populated = row.cells.iter().any(|cell| !cell.spans.is_empty());
            populated == (index % 2 == 0)
        })
        || !config.row_tolerance.is_finite()
        || config.row_tolerance < 0.0
        || !paths.iter().all(axis_aligned_fill)
    {
        return None;
    }

    let mut work = Work::new();
    let cells = maximal_cells(paths, &mut work)?;
    let columns = tiled_rows(&cells, config, &mut work)?;
    if cells.len() != rows.len().checked_mul(columns)? || !top_border_insets_only(paths, &cells, columns, &mut work)? {
        return None;
    }
    let words = source_indices(spans, &cells, config.row_tolerance, &mut work)?;
    if !assigned.iter().map(Vec::as_slice).eq(words.chunks_exact(columns)) {
        return None;
    }
    let records = record_rows(&cells, columns, &words)?;
    let table = build_table(rows, &cells, columns, records, config)?;
    let bounds = table.bbox?;
    let mut owned = vec![false; spans.len()];
    for index in words.iter().flatten() {
        owned[*index] = true;
    }
    spans
        .iter()
        .zip(owned)
        .all(|(span, owned)| owned || span.text.trim().is_empty() || !span.bbox.intersects(&bounds))
        .then_some(table)
}

fn top_border_insets_only(
    paths: &[PathContent],
    cells: &[&PathContent],
    columns: usize,
    work: &mut Work,
) -> Option<bool> {
    for path in paths {
        let mut valid = false;
        for (index, cell) in cells.iter().enumerate() {
            work.step()?;
            if !same(path.bbox.left(), cell.bbox.left())
                || !same(path.bbox.right(), cell.bbox.right())
                || !same(path.bbox.bottom(), cell.bbox.bottom())
            {
                continue;
            }
            let same_tile = same(path.bbox.top(), cell.bbox.top()) && path.fill_color == cell.fill_color;
            let top_inset = index < columns
                && path.bbox.top() > cell.bbox.top()
                && path.is_horizontal_line(super::super::BAND_RULE_SPAN_TOL);
            valid = same_tile || top_inset;
            if valid {
                break;
            }
        }
        if !valid {
            return Some(false);
        }
    }
    Some(true)
}

fn source_indices(
    spans: &[TextSpan],
    cells: &[&PathContent],
    tolerance: f32,
    work: &mut Work,
) -> Option<Vec<Vec<usize>>> {
    let region = cells.first()?.bbox.union(&cells.last()?.bbox);
    let mut owners = vec![Vec::new(); cells.len()];
    for (index, span) in spans.iter().enumerate() {
        work.spend(span.text.len().checked_add(1)?)?;
        if span.text.trim().is_empty() {
            continue;
        }
        if !finite(span.bbox) {
            return None;
        }
        if !span.bbox.intersects(&region) {
            continue;
        }
        if span.rotation_degrees != 0.0 || span.wmode != 0 || span.page_rotation_applied != 0 || span.mirrored {
            return None;
        }
        let owner = unique_owner(cells, work, |cell| owns_word(cell, span.bbox, tolerance))?;
        owners[owner].push(index);
    }
    Some(owners)
}

fn record_rows(cells: &[&PathContent], columns: usize, words: &[Vec<usize>]) -> Option<usize> {
    let occupied = |band: &[Vec<usize>]| band.iter().map(|cell| !cell.is_empty()).collect::<Vec<_>>();
    let active = occupied(&words[..columns]);
    if active.iter().filter(|&&used| used).count() < 3 {
        return None;
    }
    let last = words
        .chunks_exact(columns)
        .rposition(|band| band.iter().any(|cell| !cell.is_empty()))?;
    let last_visible = last + 1;
    if last_visible < 5 || last % 2 != 0 || cells.len() / columns - last_visible > 2 {
        return None;
    }

    let record_color = cells[0].fill_color?;
    let spacer_color = cells[columns].fill_color?;
    if spacer_color == record_color {
        return None;
    }
    for (index, band) in words.chunks_exact(columns).enumerate() {
        let cell = cells[index * columns];
        let is_record = index % 2 == 0;
        if cell.fill_color != Some(if is_record { record_color } else { spacer_color }) {
            return None;
        }
        if !record_tracks_match(index, last, &active, &occupied(band)) {
            return None;
        }
        if !is_record
            && (cell.bbox.height >= cells[(index - 1) * columns].bbox.height
                || cell.bbox.height >= cells.get((index + 1) * columns)?.bbox.height)
        {
            return None;
        }
    }
    Some(last_visible)
}

fn record_tracks_match(index: usize, last: usize, active: &[bool], occupied: &[bool]) -> bool {
    if index == last {
        return occupied.iter().filter(|&&has_text| has_text).count() >= 2
            && occupied
                .iter()
                .zip(active)
                .all(|(&has_text, &track)| !has_text || track);
    }
    if index < last && index.is_multiple_of(2) {
        return occupied == active;
    }
    !occupied.contains(&true)
}

fn build_table(
    original: &[TableRow],
    paint: &[&PathContent],
    columns: usize,
    records: usize,
    config: &TableDetectionConfig,
) -> Option<Table> {
    let mut rows = Vec::new();
    let mut bounds: Option<Rect> = None;
    let mut previous_bottom = f32::INFINITY;
    for index in (0..records).step_by(2) {
        let mut row = original[index].clone();
        let tiles = &paint[index * columns..(index + 1) * columns];
        let region = build_row_bounds(&mut row, tiles, index + 1 == records)?;
        if region.bottom() > previous_bottom {
            return None;
        }
        previous_bottom = region.top();
        bounds = Some(bounds.map_or(region, |prior| prior.union(&region)));
        rows.push(row);
    }
    let table = Table {
        rows,
        has_header: false,
        col_count: columns,
        bbox: bounds,
    };
    let filled = table
        .rows
        .iter()
        .flat_map(|row| &row.cells)
        .filter(|cell| !cell.text.is_empty())
        .count();
    (filled >= config.min_table_cells
        && table.is_real_grid()
        && super::super::is_valid_table(&table)
        && !crate::document::looks_like_prose_table(&table))
    .then_some(table)
}

fn build_row_bounds(row: &mut TableRow, paint: &[&PathContent], last_record: bool) -> Option<Rect> {
    if row.is_header || row.cells.len() != paint.len() {
        return None;
    }
    let outer_bottom = row
        .cells
        .iter()
        .flat_map(|cell| &cell.spans)
        .map(|span| span.bbox.top())
        .reduce(f32::min)?;
    let mut bounds: Option<Rect> = None;
    for (cell, tile) in row.cells.iter_mut().zip(paint) {
        if cell.colspan != 1 || cell.rowspan != 1 || cell.is_header {
            return None;
        }
        let mut cell_bounds = tile.bbox;
        if last_record {
            cell_bounds.height = cell_bounds.bottom() - outer_bottom;
            cell_bounds.y = outer_bottom;
        }
        for span in &cell.spans {
            cell_bounds = cell_bounds.union(&span.bbox);
        }
        cell.bbox = Some(cell_bounds);
        bounds = Some(bounds.map_or(cell_bounds, |prior| prior.union(&cell_bounds)));
    }
    bounds
}
