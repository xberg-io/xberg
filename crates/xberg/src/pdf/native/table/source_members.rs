//! Private membership evidence bound to the exact table that renders each retained cell.

use crate::pdf::hierarchy::SegmentData;
use crate::pdf::native::hierarchy::script_evidence::SegmentKey;
use crate::types::{BoundingBox, Table};
use xberg_native_pdf::layout::TextSpan;
use xberg_native_pdf::structure::spatial_table_detector::PaintedCellOwnership;
use xberg_native_pdf::structure::table_extractor::{Table as NativeTable, TableCell};

mod projection;
pub(crate) use projection::{CertifiedRuns, MAX_PAGE_SEGMENTS, MAX_PROOF_WORK, project};

pub(super) const MAX_CAPTURE_TABLES: usize = 64;

const MAX_TABLE_CELLS: usize = 4_096;
const MAX_TABLE_RUNS: usize = 4_096;
const MAX_TABLE_TEXT_BYTES: usize = 65_536;

#[derive(Debug)]
pub(crate) struct TableSources {
    page: u32,
    bbox: [u64; 4],
    cells: Vec<Vec<String>>,
    markdown: String,
    identity_work: usize,
    members: Vec<Vec<Run>>,
    paint_overhangs: Vec<Run>,
    recovered_lines: Vec<Run>,
    redistributed: Vec<Vec<SegmentKey>>,
    recovered_tracks: Vec<(f32, f32)>,
    replaces_empty_ruled_route: bool,
}

// ~keep Native cell spans may be synthetic; their sequence/offset fields cannot identify original source members.
#[derive(Debug, Clone)]
struct Run {
    text: String,
    bounds: Edges,
    font_size: u32,
    style: [bool; 3],
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Edges {
    left: f32,
    bottom: f32,
    right: f32,
    top: f32,
}

impl Edges {
    fn new(x: f32, y: f32, width: f32, height: f32) -> Option<Self> {
        let result = Self {
            left: x,
            bottom: y,
            right: x + width,
            top: y + height,
        };
        (width >= 0.0 && height >= 0.0 && result.values().iter().all(|value| value.is_finite())).then_some(result)
    }

    fn values(self) -> [f32; 4] {
        [self.left, self.bottom, self.right, self.top]
    }

    fn contains(self, other: Self) -> bool {
        self.left <= other.left && self.bottom <= other.bottom && self.right >= other.right && self.top >= other.top
    }

    fn intersects(self, other: Self) -> bool {
        let horizontal = if other.left == other.right {
            other.left >= self.left && other.left <= self.right
        } else {
            other.left < self.right && other.right > self.left
        };
        let vertical = if other.bottom == other.top {
            other.bottom >= self.bottom && other.bottom <= self.top
        } else {
            other.bottom < self.top && other.top > self.bottom
        };
        horizontal && vertical
    }

    fn union(self, other: Self) -> Self {
        Self {
            left: self.left.min(other.left),
            bottom: self.bottom.min(other.bottom),
            right: self.right.max(other.right),
            top: self.top.max(other.top),
        }
    }

    fn exact(self, other: Self) -> bool {
        self.values().map(f32::to_bits) == other.values().map(f32::to_bits)
    }
}

fn table_bounds(bbox: Option<BoundingBox>) -> Option<[u64; 4]> {
    let bbox = bbox?;
    let edges = [bbox.x0, bbox.y0, bbox.x1, bbox.y1];
    (edges.iter().all(|edge| edge.is_finite()) && bbox.x0 < bbox.x1 && bbox.y0 < bbox.y1)
        .then(|| edges.map(f64::to_bits))
}

fn table_text_size(table: &Table) -> Option<usize> {
    if table.cells.len() > MAX_TABLE_CELLS {
        return None;
    }
    let mut bytes = table.markdown.len();
    let mut cells = 0usize;
    for row in &table.cells {
        cells = cells.checked_add(row.len())?;
        if cells > MAX_TABLE_CELLS {
            return None;
        }
        for cell in row {
            bytes = bytes.checked_add(cell.len())?;
        }
        if bytes > MAX_TABLE_TEXT_BYTES {
            return None;
        }
    }
    (bytes <= MAX_TABLE_TEXT_BYTES).then_some(bytes)
}

fn source_size(table: &NativeTable) -> Option<()> {
    let (mut runs, mut bytes, mut cells, mut columns) = (0usize, 0usize, 0usize, 0usize);
    if table.rows.len() > MAX_TABLE_CELLS {
        return None;
    }
    for row in &table.rows {
        if row.cells.len() > MAX_TABLE_CELLS {
            return None;
        }
        for cell in &row.cells {
            cells = cells.checked_add(1)?;
            columns =
                columns.checked_add(cell.colspan.clamp(1, crate::extraction::grid_flatten::MAX_COL_SPAN) as usize)?;
            if cells > MAX_TABLE_CELLS || columns > MAX_TABLE_CELLS {
                return None;
            }
            runs = runs.checked_add(cell.spans.len())?;
            if runs > MAX_TABLE_RUNS {
                return None;
            }
            for run in &cell.spans {
                bytes = bytes.checked_add(run.text.len())?;
            }
            if bytes > MAX_TABLE_TEXT_BYTES {
                return None;
            }
        }
    }
    Some(())
}

impl Run {
    fn capture(span: &TextSpan) -> Option<Self> {
        let text = span.text.trim();
        if text.is_empty()
            || text.chars().any(char::is_whitespace)
            || span.rotation_degrees != 0.0
            || span.text_rise != 0.0
            || span.wmode != 0
            || !span.font_size.is_finite()
            || span.font_size <= 0.0
        {
            return None;
        }
        let bounds = Edges::new(span.bbox.x, span.bbox.y, span.bbox.width, span.bbox.height)?;
        (bounds.left < bounds.right && bounds.bottom < bounds.top).then(|| Self {
            text: text.into(),
            bounds,
            font_size: span.font_size.to_bits(),
            style: [
                span.font_weight == xberg_native_pdf::layout::FontWeight::Bold,
                span.is_italic,
                span.is_monospace,
            ],
        })
    }

    fn matches(&self, segment: &SegmentData, whole_line: bool) -> bool {
        segment.font_size.to_bits() == self.font_size
            && segment.y.to_bits() == self.bounds.bottom.to_bits()
            && segment.rotation_degrees == 0.0
            && segment.baseline_y.to_bits() == segment.y.to_bits()
            && [segment.is_bold, segment.is_italic, segment.is_monospace] == self.style
            && (!whole_line || segment.assigned_role.is_none())
            && (whole_line || !segment.text.chars().any(char::is_whitespace))
            && segment.width > 0.0
            && segment.height > 0.0
    }
}

fn cell_members(cell: &TableCell, text: &str) -> Option<Vec<Run>> {
    if cell.rowspan != 1
        || cell.colspan != 1
        || cell.spans.is_empty()
        || super::cell_text_in_reading_order(cell) != text
    {
        return None;
    }
    let bounds = cell.bbox?;
    let bounds = Edges::new(bounds.x, bounds.y, bounds.width, bounds.height)?;
    super::sort_spans_in_reading_order(cell.spans.iter().collect())
        .into_iter()
        .map(|span| {
            let run = Run::capture(span)?;
            bounds.contains(run.bounds).then_some(run)
        })
        .collect()
}

fn exact_decimal_member(cell: &TableCell, text: &str) -> Option<Run> {
    let [span] = cell.spans.as_slice() else { return None };
    let (whole, fraction) = text.split_once('.')?;
    if whole.is_empty()
        || fraction.is_empty()
        || !whole.bytes().chain(fraction.bytes()).all(|byte| byte.is_ascii_digit())
        || span.text.trim() != text
        || span.char_x_offsets.len() != text.len()
        || !span.char_x_offsets.windows(2).all(|pair| pair[0] < pair[1])
        || !span
            .char_x_offsets
            .iter()
            .all(|x| (span.bbox.left()..span.bbox.right()).contains(x))
    {
        return None;
    }
    Run::capture(span)
}

// Match the whole prose span to exact glyph-bearing cells on one baseline. A
// textual match against concatenated table cells alone can swallow short prose. ~keep
fn joined_decimal_source_keys(members: &[(u32, u32, Run)], source: &[TextSpan]) -> Vec<Vec<SegmentKey>> {
    if members.len() < 2 || source.len() > MAX_PAGE_SEGMENTS {
        return Vec::new();
    }
    let groups = members.chunk_by(|(row_a, col_a, a), (row_b, col_b, b)| {
        row_a == row_b
            && *col_a + 1 == *col_b
            && a.bounds.right < b.bounds.left
            && a.bounds.bottom == b.bounds.bottom
            && a.bounds.top == b.bounds.top
            && a.font_size == b.font_size
            && a.style == b.style
    });
    let mut sources = ahash::AHashMap::new();
    for span in source.iter().filter(|span| span.mcid.is_none()) {
        if let Some(bounds) = Edges::new(span.bbox.x, span.bbox.y, span.bbox.width, span.bbox.height) {
            sources
                .entry(bounds.values().map(f32::to_bits))
                .and_modify(|entry| *entry = None)
                .or_insert(Some(span));
        }
    }
    groups
        .filter_map(|group| {
            let [(_, _, first), .., (_, _, last)] = group else {
                return None;
            };
            let bounds = first.bounds.union(last.bounds).values().map(f32::to_bits);
            let span = sources.get(&bounds)?.as_ref()?;
            let text = group.iter().map(|(_, _, run)| run.text.as_str()).collect::<String>();
            if text != span.text {
                return None;
            }
            let painted = Run::capture(span)?;
            (first.font_size == painted.font_size && first.style == painted.style)
                .then(|| vec![redistributed_key(span)])
        })
        .collect()
}

fn normalized_spaces(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn recovered_cell_lines(cell: &TableCell, text: &str) -> Option<Vec<Run>> {
    if cell.rowspan != 1 || cell.colspan != 1 || cell.spans.is_empty() {
        return None;
    }
    let bounds = cell.bbox?;
    let bounds = Edges::new(bounds.x, bounds.y, bounds.width, bounds.height)?;
    let sorted = super::sort_spans_in_reading_order(cell.spans.iter().collect());
    let mut lines = Vec::new();
    for line in sorted.chunk_by(|a, b| a.bbox.y.to_bits() == b.bbox.y.to_bits()) {
        let mut combined = Run::capture(line[0])?;
        combined.text.clone_from(&line[0].text);
        for span in &line[1..] {
            let next = Run::capture(span)?;
            if next.style != combined.style || next.font_size != combined.font_size {
                return None;
            }
            combined.bounds = combined.bounds.union(next.bounds);
            combined.text.push_str(&span.text);
        }
        if !bounds.contains(combined.bounds) {
            return None;
        }
        combined.text = normalized_spaces(&combined.text);
        lines.push(combined);
    }
    let emitted = lines
        .iter()
        .map(|line| line.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    (emitted == text).then_some(lines)
}

fn recovered_track_edges(table: &NativeTable) -> Vec<(f32, f32)> {
    let Some(row) = table.rows.first() else {
        return Vec::new();
    };
    if row.cells.len() != table.col_count
        || row
            .cells
            .windows(2)
            .any(|pair| !pair[0].spans.is_empty() && !pair[1].spans.is_empty())
    {
        return Vec::new();
    }
    row.cells
        .iter()
        .filter(|cell| !cell.spans.is_empty())
        .map(|cell| {
            let bbox = cell.bbox?;
            (cell.colspan == 1 && bbox.left().is_finite() && bbox.right().is_finite() && bbox.left() < bbox.right())
                .then_some((bbox.left(), bbox.right()))
        })
        .collect::<Option<Vec<_>>>()
        .unwrap_or_default()
}

fn paint_overhang(
    cell: &TableCell,
    text: &str,
    ownership: &PaintedCellOwnership,
    row: usize,
    column: usize,
) -> Option<Run> {
    let [span] = cell.spans.as_slice() else { return None };
    if cell.rowspan != 1 || cell.colspan != 1 || span.text != text || !ownership.certifies_word(row, column, span.bbox)
    {
        return None;
    }
    Run::capture(span)
}

fn redistributed_keys(original: &TextSpan, source: &[TextSpan]) -> Option<Vec<SegmentKey>> {
    if let Some(exact) = source.iter().find(|span| {
        span.sequence == original.sequence
            && span.bbox == original.bbox
            && span.text == original.text
            && span.mcid == original.mcid
            && span.mcid_scope == original.mcid_scope
    }) {
        return Some(vec![redistributed_key(exact)]);
    }
    let mut spans: Vec<_> = source
        .iter()
        .filter(|span| {
            (span.bbox.y - original.bbox.y).abs() <= super::DECIMAL_ROW_TOLERANCE
                && span.bbox.x >= original.bbox.x - super::DECIMAL_ROW_TOLERANCE
                && span.bbox.right() <= original.bbox.right() + super::DECIMAL_ROW_TOLERANCE
        })
        .collect();
    spans.sort_by(|left, right| left.bbox.x.total_cmp(&right.bbox.x));
    if (spans.first()?.bbox.x - original.bbox.x).abs() > super::DECIMAL_ROW_TOLERANCE
        || (spans.last()?.bbox.right() - original.bbox.right()).abs() > super::DECIMAL_ROW_TOLERANCE
        || spans.windows(2).any(|pair| pair[0].bbox.x >= pair[1].bbox.x)
        || spans
            .iter()
            .any(|span| span.rotation_degrees != 0.0 || span.mcid != original.mcid)
        || spans.iter().map(|span| span.text.as_str()).collect::<String>() != original.text
    {
        return None;
    }
    Some(spans.into_iter().map(redistributed_key).collect())
}

fn redistributed_key(span: &TextSpan) -> SegmentKey {
    SegmentKey::from(&SegmentData {
        text: span.text.clone(),
        x: span.bbox.x,
        y: span.bbox.y,
        width: span.bbox.width,
        height: span.bbox.height,
        font_size: span.font_size,
        is_bold: span.font_weight == xberg_native_pdf::layout::FontWeight::Bold,
        is_italic: span.is_italic,
        is_monospace: span.is_monospace,
        baseline_y: span.bbox.y,
        rotation_degrees: span.rotation_degrees,
        assigned_role: None,
        ocr_heading_spans: None,
    })
}

#[cfg(test)]
pub(super) fn capture(
    native: &NativeTable,
    table: &Table,
    ownership: Option<&PaintedCellOwnership>,
) -> Option<TableSources> {
    capture_with_redistributed_spans(native, table, ownership, &[], &Default::default(), &[])
}

pub(super) fn capture_with_redistributed_spans(
    native: &NativeTable,
    table: &Table,
    ownership: Option<&PaintedCellOwnership>,
    repaired: &[TextSpan],
    source: &super::scripts::PageSpans,
    recovered: &[Vec<TextSpan>],
) -> Option<TableSources> {
    let mut redistributed: Vec<_> = repaired
        .iter()
        .filter_map(|span| redistributed_keys(span, &source.spans))
        .chain(recovered.iter().filter_map(|group| {
            group
                .iter()
                .map(|span| redistributed_keys(span, &source.spans))
                .collect::<Option<Vec<_>>>()
                .map(|keys| keys.into_iter().flatten().collect())
        }))
        .collect();
    let text_bytes = table_text_size(table)?;
    let identity_work = text_bytes + table.cells.len() + table.cells.iter().map(Vec::len).sum::<usize>();
    source_size(native)?;
    let bbox = table_bounds(table.bounding_box)?;
    let rows: Vec<Vec<_>> = native.rows.iter().map(|row| row.cells.iter().collect()).collect();
    let mut members = Vec::new();
    let mut decimal_members = Vec::new();
    let mut paint_overhangs = Vec::new();
    let mut recovered_lines = Vec::new();
    let recovered = native
        .bbox
        .is_some_and(|bounds| ownership.is_some_and(|ownership| ownership.certifies_recovered_rows(bounds)));
    crate::extraction::grid_flatten::resolve_span_grid(
        &rows,
        |cell| cell.colspan,
        |cell| cell.rowspan,
        |row, col, cell| {
            let text = table.cells.get(row as usize).and_then(|row| row.get(col as usize));
            let Some(text) = text else { return };
            if recovered {
                if let Some(lines) = recovered_cell_lines(cell, text) {
                    recovered_lines.extend(lines);
                }
            } else if let Some(runs) = cell_members(cell, text) {
                if let Some(decimal) = exact_decimal_member(cell, text) {
                    decimal_members.push((row, col, decimal));
                }
                members.push(runs);
            } else if let Some(run) =
                ownership.and_then(|ownership| paint_overhang(cell, text, ownership, row as usize, col as usize))
            {
                paint_overhangs.push(run);
            }
        },
    );
    redistributed.extend(joined_decimal_source_keys(&decimal_members, &source.spans));
    (!members.is_empty() || !paint_overhangs.is_empty() || !recovered_lines.is_empty() || !redistributed.is_empty())
        .then(|| TableSources {
            page: table.page_number,
            bbox,
            cells: table.cells.clone(),
            markdown: table.markdown.clone(),
            identity_work,
            members,
            paint_overhangs,
            recovered_lines,
            redistributed,
            replaces_empty_ruled_route: native
                .bbox
                .is_some_and(|bounds| ownership.is_some_and(|evidence| evidence.certifies_new_ruled_table(bounds))),
            recovered_tracks: if recovered {
                recovered_track_edges(native)
            } else {
                Vec::new()
            },
        })
}

#[cfg(test)]
#[path = "source_members/tests.rs"]
mod tests;
