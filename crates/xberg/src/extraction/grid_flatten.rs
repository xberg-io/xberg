//! Span-aware flattening of table grids to `Vec<Vec<String>>`.
//!
//! Several extractors (HTML, DOCX, PPTX, email) receive tables as a stream of
//! cells carrying `row_span` / `col_span`. Flattening them by trusting a naive
//! per-row column index shifts every cell under a `rowspan` left into the
//! spanning column, misaligning the data against its headers
//! (xberg-io/xberg#1223). This helper places cells on a grid that reserves the
//! columns still covered by a rowspan started in an earlier row, so merged-cell
//! tables keep their column alignment across every format.

/// A table cell with its span, in document order within its row.
pub(crate) struct SpanCell {
    pub content: String,
    pub row_span: u32,
    pub col_span: u32,
}

impl SpanCell {
    pub(crate) fn new(content: impl Into<String>, row_span: u32, col_span: u32) -> Self {
        Self {
            content: content.into(),
            row_span,
            col_span,
        }
    }
}

/// Flatten rows of span-carrying cells into a dense `Vec<Vec<String>>`.
///
/// The origin cell of a span holds the value; the columns/rows it covers are
/// left empty. The output is rectangular: every row is padded to the widest
/// column reached.
pub(crate) fn flatten_spanned_rows(rows: &[Vec<SpanCell>]) -> Vec<Vec<String>> {
    // `occupied_until[col]` is the exclusive row index up to which the column is
    // covered by a rowspan from an earlier row.
    let mut occupied_until: Vec<u32> = Vec::new();
    // Placed cells as (row, col, content); positions are resolved with occupancy.
    let mut placed: Vec<(u32, u32, String)> = Vec::new();

    for (row_idx, row) in rows.iter().enumerate() {
        let row_idx = row_idx as u32;
        let mut col = 0usize;
        for cell in row {
            while col < occupied_until.len() && occupied_until[col] > row_idx {
                col += 1;
            }
            let end_row = row_idx + cell.row_span.max(1);
            let span = cell.col_span.max(1) as usize;
            for c in col..col + span {
                if c >= occupied_until.len() {
                    occupied_until.resize(c + 1, 0);
                }
                occupied_until[c] = end_row;
            }
            placed.push((row_idx, col as u32, cell.content.clone()));
            col += span;
        }
    }

    let num_cols = occupied_until.len();
    let num_rows = rows.len();
    let mut grid = vec![vec![String::new(); num_cols]; num_rows];
    for (r, c, content) in placed {
        if (r as usize) < num_rows && (c as usize) < num_cols {
            grid[r as usize][c as usize] = content;
        }
    }
    grid
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rowspan_reserves_column() {
        // Row 0: [Alpha(rowspan 2), Alice, 10]; Row 1: [Bob, 20].
        let rows = vec![
            vec![
                SpanCell::new("Alpha", 2, 1),
                SpanCell::new("Alice", 1, 1),
                SpanCell::new("10", 1, 1),
            ],
            vec![SpanCell::new("Bob", 1, 1), SpanCell::new("20", 1, 1)],
        ];
        let grid = flatten_spanned_rows(&rows);
        assert_eq!(grid[0], vec!["Alpha", "Alice", "10"]);
        // Bob must land in column 1, not shift into Alpha's column.
        assert_eq!(grid[1], vec!["", "Bob", "20"]);
    }

    #[test]
    fn colspan_widens_grid() {
        let rows = vec![
            vec![SpanCell::new("Fuse", 1, 2), SpanCell::new("Circuit", 1, 1)],
            vec![
                SpanCell::new("101", 1, 1),
                SpanCell::new("40A", 1, 1),
                SpanCell::new("Blower", 1, 1),
            ],
        ];
        let grid = flatten_spanned_rows(&rows);
        assert_eq!(grid[0], vec!["Fuse", "", "Circuit"]);
        assert_eq!(grid[1], vec!["101", "40A", "Blower"]);
    }

    #[test]
    fn no_spans_is_identity() {
        let rows = vec![
            vec![SpanCell::new("a", 1, 1), SpanCell::new("b", 1, 1)],
            vec![SpanCell::new("c", 1, 1), SpanCell::new("d", 1, 1)],
        ];
        let grid = flatten_spanned_rows(&rows);
        assert_eq!(grid, vec![vec!["a", "b"], vec!["c", "d"]]);
    }
}
