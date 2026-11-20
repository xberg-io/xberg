//! Span-aware flattening of table grids to `Vec<Vec<String>>`.
//!
//! Several extractors (HTML, DOCX, PPTX, email) receive tables as a stream of
//! cells carrying `row_span` / `col_span`. Flattening them by trusting a naive
//! per-row column index shifts every cell under a `rowspan` left into the
//! spanning column, misaligning the data against its headers
//! (xberg-io/xberg#1223). This helper places cells on a grid that reserves the
//! columns still covered by a rowspan started in an earlier row, so merged-cell
//! tables keep their column alignment across every format.
//!
//! Both `col_span` and `row_span` are clamped ([`MAX_COL_SPAN`], [`MAX_ROW_SPAN`])
//! before they can size anything. This is deliberately the single choke point for
//! that clamp: `resolve_span_grid` has two callers with different amounts of
//! trust in their input — `html::structure`'s own attribute parser, and
//! `html::converter`, which forwards `col_span`/`row_span` values produced by the
//! vendored `html_to_markdown_rs` crate. That crate clamps spans on its
//! Markdown-rendering path but *not* on the `document_structure`/`TableGrid`
//! path `converter.rs` actually consumes (`types::structure_builder`, as of
//! 3.11.3), so an untrusted `colspan`/`rowspan` can reach here unclamped from
//! upstream even though our own parser is fixed. Clamping here catches both.

use crate::types::{GridCell, TableGrid};

/// Upper bound on `colspan` accepted by [`resolve_span_grid`].
///
/// This is the HTML Living Standard's own ceiling ("If \[colspan\] is greater
/// than 1000, then let colspan be 1000."), not a value we invented: no
/// spec-conforming browser renders a wider span, so clamping to it cannot
/// mangle a legitimate document. It also caps the cost of the occupancy
/// vector `resolve_span_grid` grows per cell to `MAX_COL_SPAN * 4` bytes
/// (~4 KiB), closing the unbounded-allocation path where an untrusted
/// `colspan` (e.g. `4294967295`) previously drove a ~17 GB `Vec<u32>::resize`.
pub(crate) const MAX_COL_SPAN: u32 = 1000;

/// Upper bound on `rowspan` accepted by [`resolve_span_grid`].
///
/// Also the HTML Living Standard's own ceiling ("If \[rowspan\] is greater
/// than 65534, then let rowspan be 65534."). A rowspan does not itself index
/// an allocation here, but it feeds `row_idx + row_span` when computing
/// `end_row`; clamping keeps that addition far from `u32::MAX` regardless of
/// how many real rows a (bounded-size) document can contain, so it cannot
/// overflow.
pub(crate) const MAX_ROW_SPAN: u32 = 65534;

/// A table cell with its span, in document order within its row.
#[cfg(any(feature = "pdf", test))]
pub(crate) struct SpanCell {
    pub content: String,
    pub row_span: u32,
    pub col_span: u32,
}

#[cfg(any(feature = "pdf", test))]
impl SpanCell {
    pub(crate) fn new(content: impl Into<String>, row_span: u32, col_span: u32) -> Self {
        Self {
            content: content.into(),
            row_span,
            col_span,
        }
    }
}

/// Resolve each span-carrying cell's `(row, col)` on an occupancy grid that
/// reserves the columns still covered by a rowspan from an earlier row, and
/// return the total column count.
///
/// Cells are visited row-major, in document order within each row; `place` is
/// called once per cell with its row index, resolved column, and the cell
/// itself. This is the single home of the merged-cell placement algorithm —
/// both the flattening helpers here and the `DocumentStructure` table grid
/// builder route through it so the geometry can never drift between them
/// (xberg-io/xberg#1223).
pub(crate) fn resolve_span_grid<C>(
    rows: &[Vec<C>],
    col_span: impl Fn(&C) -> u32,
    row_span: impl Fn(&C) -> u32,
    mut place: impl FnMut(u32, u32, &C),
) -> u32 {
    let mut occupied_until: Vec<u32> = Vec::new();
    for (row_idx, row) in rows.iter().enumerate() {
        let row_idx = row_idx as u32;
        let mut col = 0usize;
        for cell in row {
            while col < occupied_until.len() && occupied_until[col] > row_idx {
                col += 1;
            }
            let end_row = row_idx.saturating_add(row_span(cell).clamp(1, MAX_ROW_SPAN));
            let span = col_span(cell).clamp(1, MAX_COL_SPAN) as usize;
            for c in col..col + span {
                if c >= occupied_until.len() {
                    occupied_until.resize(c + 1, 0);
                }
                occupied_until[c] = end_row;
            }
            place(row_idx, col as u32, cell);
            col += span;
        }
    }
    occupied_until.len() as u32
}

/// Flatten rows of span-carrying cells into a dense `Vec<Vec<String>>`.
///
/// The origin cell of a span holds the value; the columns/rows it covers are
/// left empty. The output is rectangular: every row is padded to the widest
/// column reached.
#[cfg(any(feature = "pdf", test))]
pub(crate) fn flatten_spanned_rows(rows: &[Vec<SpanCell>]) -> Vec<Vec<String>> {
    let mut placed: Vec<(u32, u32, String)> = Vec::new();
    let num_cols = resolve_span_grid(
        rows,
        |c| c.col_span,
        |c| c.row_span,
        |row_idx, col, cell| placed.push((row_idx, col, cell.content.clone())),
    ) as usize;

    let num_rows = rows.len();
    let mut grid = vec![vec![String::new(); num_cols]; num_rows];
    for (r, c, content) in placed {
        if (r as usize) < num_rows && (c as usize) < num_cols {
            grid[r as usize][c as usize] = content;
        }
    }
    grid
}

/// A source cell before its true grid position is known.
struct PositionedSourceCell {
    content: String,
    row_span: u32,
    col_span: u32,
    is_header: bool,
}

/// Flatten a stream of span-carrying cells whose per-row order is known but
/// whose column index is naive (does not reserve rowspan columns) — the shape
/// `html_to_markdown_rs` grids arrive in. Groups by row, then re-derives true
/// positions with [`flatten_spanned_rows`].
///
/// `cells` yields `(row, row_span, col_span, content)` in document order.
pub(crate) fn flatten_positioned_cells(
    num_rows: usize,
    cells: impl Iterator<Item = (u32, u32, u32, String)>,
) -> Vec<Vec<String>> {
    flatten_positioned_cells_with_roles(
        num_rows,
        cells.map(|(row, row_span, col_span, content)| (row, row_span, col_span, false, content)),
    )
    .0
}

/// Flatten a stream of span-carrying, role-carrying cells into both the dense
/// `Vec<Vec<String>>` grid and the [`TableGrid`] that records where each source
/// cell sits and what it is.
///
/// The dense grid loses cell roles and merge geometry: a `<th>` becomes
/// indistinguishable from a `<td>`, and the columns a `colspan` covers become
/// empty strings a renderer cannot tell apart from genuinely empty cells. The
/// retained grid keeps them, so an HTML table reaches HTML output with its
/// header cells and `rowspan`/`colspan` intact instead of being re-inferred
/// from bare text (xberg-io/xberg#2332).
///
/// `cells` yields `(row, row_span, col_span, is_header, content)` in document
/// order. Positions are re-derived by [`resolve_span_grid`], so a caller whose
/// own column index ignores rowspan columns still gets true positions. Spans
/// are clamped to the grid's own bounds, which is what a browser does with a
/// `rowspan` that outruns the table: the retained grid must stay a valid
/// occupancy of the published cells or it is discarded downstream.
pub(crate) fn flatten_positioned_cells_with_roles(
    num_rows: usize,
    cells: impl Iterator<Item = (u32, u32, u32, bool, String)>,
) -> (Vec<Vec<String>>, TableGrid) {
    let mut rows: Vec<Vec<PositionedSourceCell>> = (0..num_rows.max(1)).map(|_| Vec::new()).collect();
    for (row, row_span, col_span, is_header, content) in cells {
        let r = row as usize;
        // A cell whose reported row index falls outside the caller's declared
        // `num_rows` used to be clamped onto the last row, scrambling it in as
        // extra columns under the wrong header (xberg-io/xberg#234). Grow the
        // grid instead so the cell keeps its own row.
        if r >= rows.len() {
            rows.resize_with(r + 1, Vec::new);
        }
        rows[r].push(PositionedSourceCell {
            content,
            row_span,
            col_span,
            is_header,
        });
    }

    let num_rows = rows.len();
    let mut placed: Vec<(u32, u32)> = Vec::new();
    let num_cols = resolve_span_grid(
        &rows,
        |c| c.col_span,
        |c| c.row_span,
        |row_idx, col, _| placed.push((row_idx, col)),
    ) as usize;

    // `resolve_span_grid` visits rows in order and each row's cells in order, so this
    // iterator hands back the same cell that produced each placement. ~keep
    let mut source_cells = rows.iter().flatten();
    let mut grid_cells = Vec::with_capacity(placed.len());
    let mut dense = vec![vec![String::new(); num_cols]; num_rows];
    for (row, col) in placed {
        let cell = source_cells
            .next()
            .expect("resolve_span_grid places every source cell exactly once");
        dense[row as usize][col as usize] = cell.content.clone();
        grid_cells.push(GridCell {
            content: cell.content.clone(),
            row,
            col,
            row_span: cell.row_span.clamp(1, MAX_ROW_SPAN).min(num_rows as u32 - row),
            col_span: cell.col_span.clamp(1, MAX_COL_SPAN).min(num_cols as u32 - col),
            is_header: cell.is_header,
            bbox: None,
            heading_level: None,
            style_name: None,
        });
    }

    let grid = TableGrid {
        rows: num_rows as u32,
        cols: num_cols as u32,
        cells: grid_cells,
    };
    (dense, grid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rowspan_reserves_column() {
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

    /// Positive control at 3 columns, in addition to `colspan_widens_grid`'s 2, so
    /// the fix below is proven against more than one coincidental width.
    #[test]
    fn colspan_three_widens_grid_exactly() {
        let rows = vec![
            vec![SpanCell::new("Header", 1, 3)],
            vec![
                SpanCell::new("a", 1, 1),
                SpanCell::new("b", 1, 1),
                SpanCell::new("c", 1, 1),
            ],
        ];
        let grid = flatten_spanned_rows(&rows);
        assert_eq!(grid[0], vec!["Header", "", ""]);
        assert_eq!(grid[1], vec!["a", "b", "c"]);
    }

    /// The HTML-spec ceiling itself must pass through untouched: a `colspan`
    /// exactly at `MAX_COL_SPAN` is a legal value, so the following cell must land
    /// right after it, not clamped down further.
    #[test]
    fn colspan_at_cap_is_unaffected() {
        let rows = vec![vec![
            SpanCell::new("wide", 1, MAX_COL_SPAN),
            SpanCell::new("next", 1, 1),
        ]];
        let mut placed = Vec::new();
        let cols = resolve_span_grid(
            &rows,
            |c| c.col_span,
            |c| c.row_span,
            |row_idx, col, cell| placed.push((row_idx, col, cell.content.clone())),
        );
        assert_eq!(cols, MAX_COL_SPAN + 1);
        assert_eq!(placed[1], (0, MAX_COL_SPAN, "next".to_string()));
    }

    /// One past the cap must clamp down to exactly `MAX_COL_SPAN`, not to some
    /// smaller value and not left unclamped.
    #[test]
    fn colspan_one_over_cap_is_clamped_to_cap() {
        let rows = vec![vec![
            SpanCell::new("wide", 1, MAX_COL_SPAN + 1),
            SpanCell::new("next", 1, 1),
        ]];
        let mut placed = Vec::new();
        let cols = resolve_span_grid(
            &rows,
            |c| c.col_span,
            |c| c.row_span,
            |row_idx, col, cell| placed.push((row_idx, col, cell.content.clone())),
        );
        assert_eq!(
            cols,
            MAX_COL_SPAN + 1,
            "one over cap must clamp down to exactly MAX_COL_SPAN columns"
        );
        assert_eq!(placed[1], (0, MAX_COL_SPAN, "next".to_string()));
    }

    /// A hostile `colspan="4294967295"` must not attempt the ~17 GB `Vec<u32>`
    /// resize this used to drive in `resolve_span_grid`. Against the unfixed code
    /// (`col_span(cell).max(1)` with no upper bound) this scenario does not fail an
    /// assertion — the process hangs or is OOM-killed trying to grow
    /// `occupied_until` to ~4.29 billion entries before any `assert_eq!` runs. The
    /// fixed code must instead come back immediately with the span clamped to
    /// `MAX_COL_SPAN`.
    #[test]
    fn colspan_bomb_is_clamped_not_allocated() {
        let rows = vec![vec![SpanCell::new("bomb", 1, u32::MAX), SpanCell::new("next", 1, 1)]];
        let mut placed = Vec::new();
        let cols = resolve_span_grid(
            &rows,
            |c| c.col_span,
            |c| c.row_span,
            |row_idx, col, cell| placed.push((row_idx, col, cell.content.clone())),
        );
        assert_eq!(
            cols,
            MAX_COL_SPAN + 1,
            "bomb cell clamps to MAX_COL_SPAN columns, plus the trailing cell"
        );
        assert_eq!(placed[0], (0, 0, "bomb".to_string()));
        assert_eq!(
            placed[1],
            (0, MAX_COL_SPAN, "next".to_string()),
            "the following cell must land right after the clamped span, not after the raw u32::MAX request"
        );
    }

    /// `row_span` doesn't itself size an allocation the way `col_span` does, but an
    /// unclamped value overflows the `row_idx + row_span` addition used to compute
    /// `end_row` once `row_idx` gets close enough to `u32::MAX - row_span`. Under
    /// `cargo test`'s overflow checks that addition panics rather than failing an
    /// assertion. Clamping to `MAX_ROW_SPAN` keeps the sum far from that boundary
    /// regardless of the raw attribute value, and the column the rowspan claims
    /// must still read as reserved on the following row.
    #[test]
    fn rowspan_bomb_does_not_panic_or_overflow() {
        let rows = vec![
            vec![SpanCell::new("bomb", u32::MAX, 1)],
            vec![SpanCell::new("still-reserved", 1, 1)],
        ];
        let mut placed = Vec::new();
        resolve_span_grid(
            &rows,
            |c| c.col_span,
            |c| c.row_span,
            |row_idx, col, cell| placed.push((row_idx, col, cell.content.clone())),
        );
        assert_eq!(
            placed[1],
            (1, 1, "still-reserved".to_string()),
            "row 1's column 0 is still reserved by the clamped rowspan, so the second cell must be pushed to column 1"
        );
    }

    #[test]
    fn overflow_row_grows_grid_instead_of_clamping() {
        // num_rows under-reports the true row count (2 instead of 3); the
        // second cell's row index (2) is out of bounds for that declared size.
        let cells = vec![
            (0u32, 1u32, 1u32, "Header".to_string()),
            (2u32, 1u32, 1u32, "Orphan".to_string()),
        ];
        let grid = flatten_positioned_cells(1, cells.into_iter());
        assert_eq!(grid.len(), 3, "overflow row must grow the grid, not clamp into row 0");
        assert_eq!(grid[0], vec!["Header"]);
        assert_eq!(grid[1], vec![""]);
        assert_eq!(grid[2], vec!["Orphan"]);
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

    /// A `<th>` first row and a `rowspan` in the data below it must reach the
    /// retained grid at their true positions, so HTML output can re-emit them
    /// (GH#2332).
    #[test]
    fn positioned_roles_keep_header_cells_and_spans() {
        let cells = vec![
            (0u32, 1u32, 1u32, true, "Group".to_string()),
            (0u32, 1u32, 1u32, true, "Name".to_string()),
            (1u32, 2u32, 1u32, false, "Alpha".to_string()),
            (1u32, 1u32, 1u32, false, "Alice".to_string()),
            (2u32, 1u32, 1u32, false, "Bob".to_string()),
        ];
        let (dense, grid) = flatten_positioned_cells_with_roles(3, cells.into_iter());

        assert_eq!(
            dense,
            vec![vec!["Group", "Name"], vec!["Alpha", "Alice"], vec!["", "Bob"]]
        );
        assert_eq!((grid.rows, grid.cols), (3, 2));
        let roles: Vec<_> = grid
            .cells
            .iter()
            .map(|cell| {
                (
                    cell.row,
                    cell.col,
                    cell.row_span,
                    cell.col_span,
                    cell.is_header,
                    cell.content.as_str(),
                )
            })
            .collect();
        assert_eq!(
            roles,
            vec![
                (0, 0, 1, 1, true, "Group"),
                (0, 1, 1, 1, true, "Name"),
                (1, 0, 2, 1, false, "Alpha"),
                (1, 1, 1, 1, false, "Alice"),
                (2, 1, 1, 1, false, "Bob"),
            ]
        );
    }

    /// A rowspan that outruns the table must clamp to the rows actually published,
    /// otherwise the retained grid is rejected as an invalid occupancy and every
    /// role and span it carries is lost. (A colspan cannot outrun the grid: it is
    /// what establishes the table's width.)
    #[test]
    fn outrunning_rowspan_clamps_to_the_published_grid() {
        let cells = vec![
            (0u32, 5u32, 1u32, false, "tall".to_string()),
            (0u32, 1u32, 1u32, false, "next".to_string()),
            (1u32, 1u32, 1u32, false, "last".to_string()),
        ];
        let (dense, grid) = flatten_positioned_cells_with_roles(2, cells.into_iter());

        assert_eq!(dense, vec![vec!["tall", "next"], vec!["", "last"]]);
        assert_eq!((grid.rows, grid.cols), (2, 2));
        assert_eq!(grid.cells[0].row_span, 2, "two rows are all the table has");
    }

    /// The role-aware path and the plain one must place cells identically; the
    /// only difference is the metadata carried alongside.
    #[test]
    fn role_aware_flattening_matches_the_plain_flattening() {
        let cells = vec![
            (0u32, 1u32, 2u32, false, "Fuse".to_string()),
            (0u32, 1u32, 1u32, false, "Circuit".to_string()),
            (1u32, 1u32, 1u32, false, "101".to_string()),
            (1u32, 1u32, 1u32, false, "40A".to_string()),
            (1u32, 1u32, 1u32, false, "Blower".to_string()),
        ];
        let (dense, _) = flatten_positioned_cells_with_roles(2, cells.clone().into_iter());
        let plain = flatten_positioned_cells(
            2,
            cells
                .into_iter()
                .map(|(row, row_span, col_span, _, content)| (row, row_span, col_span, content)),
        );
        assert_eq!(dense, plain);
    }
}
