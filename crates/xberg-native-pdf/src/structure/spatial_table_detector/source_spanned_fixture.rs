use crate::elements::PathContent;
use crate::geometry::Rect;
use crate::layout::TextSpan;
use crate::structure::table_extractor::{Table, TableCell, TableRow};

pub(crate) const COLUMNS: usize = 16;
const CELL: f32 = 20.0;
const MERGED_WIDTHS: [usize; 4] = [1, 8, 6, 1];

pub(crate) struct Grid {
    pub table: Table,
    pub spans: Vec<TextSpan>,
    pub paths: Vec<PathContent>,
}

fn cell(text: &str, x: f32, y: f32, columns: usize, sequence: usize) -> (TableCell, TextSpan) {
    let bbox = Rect::new(x, y, CELL * columns as f32, CELL);
    let span = TextSpan {
        text: text.to_string(),
        bbox: Rect::new(x + 2.0, y + 2.0, 8.0, 8.0),
        sequence,
        ..Default::default()
    };
    let mut cell = TableCell::new(text.to_string(), false).with_colspan(columns as u32);
    cell.bbox = Some(bbox);
    cell.spans = vec![span.clone()];
    (cell, span)
}

/// Three ruled rows of sixteen tracks whose middle row merges them as 1, 8, 6 and 1, with
/// every rule painted and every word claimed by exactly one cell.
pub(crate) fn grid() -> Grid {
    let mut spans = Vec::new();
    let mut table = Table::new();
    table.col_count = COLUMNS;
    table.bbox = Some(Rect::new(0.0, 0.0, CELL * COLUMNS as f32, CELL * 3.0));
    let mut paths: Vec<PathContent> = (0..=3)
        .map(|row| PathContent::line(0.0, row as f32 * CELL, CELL * COLUMNS as f32, row as f32 * CELL))
        .collect();
    for row in 0..3 {
        let widths: &[usize] = if row == 1 { &MERGED_WIDTHS } else { &[1; COLUMNS] };
        let mut track = 0;
        let y = (2 - row) as f32 * CELL;
        let mut cells = TableRow::new(false);
        for width in widths {
            let (cell, span) = cell("W", track as f32 * CELL, y, *width, spans.len());
            spans.push(span);
            cells.add_cell(cell);
            paths.push(PathContent::line(track as f32 * CELL, y, track as f32 * CELL, y + CELL));
            track += width;
        }
        paths.push(PathContent::line(track as f32 * CELL, y, track as f32 * CELL, y + CELL));
        table.rows.push(cells);
    }
    Grid { table, spans, paths }
}
