//! Private membership evidence bound to the exact table that renders each retained cell.

use crate::pdf::hierarchy::SegmentData;
use crate::pdf::native::hierarchy::script_evidence::SegmentKey;
use crate::types::{BoundingBox, Table};
use xberg_native_pdf::layout::TextSpan;
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

    fn matches(&self, segment: &SegmentData) -> bool {
        segment.font_size.to_bits() == self.font_size
            && segment.y.to_bits() == self.bounds.bottom.to_bits()
            && segment.rotation_degrees == 0.0
            && segment.baseline_y.to_bits() == segment.y.to_bits()
            && [segment.is_bold, segment.is_italic, segment.is_monospace] == self.style
            && !segment.text.chars().any(char::is_whitespace)
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

pub(super) fn capture(native: &NativeTable, table: &Table) -> Option<TableSources> {
    let text_bytes = table_text_size(table)?;
    let identity_work = text_bytes + table.cells.len() + table.cells.iter().map(Vec::len).sum::<usize>();
    source_size(native)?;
    let bbox = table_bounds(table.bounding_box)?;
    let rows: Vec<Vec<_>> = native.rows.iter().map(|row| row.cells.iter().collect()).collect();
    let mut members = Vec::new();
    crate::extraction::grid_flatten::resolve_span_grid(
        &rows,
        |cell| cell.colspan,
        |cell| cell.rowspan,
        |row, col, cell| {
            let text = table.cells.get(row as usize).and_then(|row| row.get(col as usize));
            if let Some(text) = text
                && let Some(runs) = cell_members(cell, text)
            {
                members.push(runs);
            }
        },
    );
    (!members.is_empty()).then(|| TableSources {
        page: table.page_number,
        bbox,
        cells: table.cells.clone(),
        markdown: table.markdown.clone(),
        identity_work,
        members,
    })
}

#[cfg(test)]
#[path = "source_members/tests.rs"]
mod tests;
