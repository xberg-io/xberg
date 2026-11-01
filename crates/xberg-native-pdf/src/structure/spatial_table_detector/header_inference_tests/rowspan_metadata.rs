use super::multi_tier::{ROWS, Style, detect_fixture, paths, source};
use super::{COLUMN_WIDTH, PITCH, assert_headerless};
use crate::elements::PathContent;
use crate::geometry::Rect;
use crate::structure::table_extractor::Table;

fn actual_spanning_cell() -> Table {
    let mut source = source(Style::Regular);
    assert_eq!(source[2].text, "Product");
    source.remove(2);
    assert_eq!(source.len(), 13);
    let mut paths = paths();
    let boundary = ROWS - 1;
    let y = boundary as f32 * PITCH;
    assert_eq!(paths[boundary].bbox, Rect::new(0.0, y, 3.0 * COLUMN_WIDTH, 0.0));
    paths[boundary] = PathContent::line(COLUMN_WIDTH, y, 3.0 * COLUMN_WIDTH, y);
    assert_eq!(
        paths[boundary].bbox,
        Rect::new(COLUMN_WIDTH, y, 2.0 * COLUMN_WIDTH, 0.0)
    );
    let table = detect_fixture(&source, &paths);
    assert_headerless(&table);
    assert_eq!(
        table
            .rows
            .iter()
            .flat_map(|row| &row.cells)
            .filter(|cell| !cell.text.is_empty())
            .count(),
        13
    );
    assert_eq!(
        table.rows[1]
            .cells
            .iter()
            .filter(|cell| !cell.text.is_empty())
            .map(|cell| cell.text.as_str())
            .collect::<Vec<_>>(),
        vec!["Current", "Previous"]
    );
    assert_eq!(
        table.rows[2..].iter().map(|row| row.cells.len()).collect::<Vec<_>>(),
        vec![3, 3, 3]
    );
    assert_eq!(table.rows[0].cells[0].text, "Category");
    assert_eq!(table.rows[0].cells[0].colspan, 1);
    assert_eq!(table.rows[0].cells[0].spans.len(), 1);
    assert!(
        table
            .rows
            .iter()
            .flat_map(|row| &row.cells)
            .all(|cell| cell.bbox.is_some())
    );
    table
}

#[test]
fn intersection_missing_internal_horizontal_border_retains_rowspan_2332() {
    let table = actual_spanning_cell();
    assert_eq!(
        table.rows[0].cells[0].rowspan, 2,
        "the drawn Category cell crosses both upper source row intervals"
    );
    assert_eq!(
        table.rows[1].cells.len(),
        2,
        "a covered lower slot must not survive as a separate empty cell"
    );
    assert_eq!(table.rows.iter().map(|row| row.cells.len()).sum::<usize>(), 13);
}

mod source_controls {
    use super::super::super::{
        IntersectionCell, TableDetectionConfig, build_grid_from_lines, detect_tables_from_intersections,
        detect_tables_with_lines, extract_cell_text,
    };
    use super::super::multi_tier::{ROWS, Style, paths, source};
    use super::super::{COLUMN_WIDTH, PITCH, assert_headerless, span};
    use crate::elements::PathContent;
    use crate::geometry::Rect;
    use crate::layout::TextSpan;
    use crate::structure::table_extractor::Table;

    fn physical_cells(paths: &[PathContent]) -> Vec<IntersectionCell> {
        let config = TableDetectionConfig {
            min_table_cells: 6,
            ..TableDetectionConfig::strict()
        };
        let (groups, _, intersections) = build_grid_from_lines(paths, &config);
        assert!(intersections, "real drawn crossings must supply the cell geometry");
        assert_eq!(groups.len(), 1);
        let (cells, xs, ys, columns) = groups.into_iter().next().unwrap();
        assert_eq!(columns, 3);
        assert_eq!(xs, vec![0.0, 100.0, 200.0, 300.0]);
        assert_eq!(ys, vec![0.0, 30.0, 60.0, 90.0, 120.0, 150.0]);
        cells
    }

    fn cell_rect(cell: &IntersectionCell) -> Rect {
        Rect::new(cell.x1, cell.y1, cell.x2 - cell.x1, cell.y2 - cell.y1)
    }

    fn assert_witnesses(table: &Table, source: &[TextSpan], count: usize) {
        assert_eq!(source.len(), count);
        assert!(source.iter().all(|span| !span.text.is_empty()));
        let mut source_order: Vec<_> = source.iter().collect();
        source_order.sort_by_key(|span| span.sequence);
        assert!(source_order.windows(2).all(|pair| pair[0].sequence != pair[1].sequence));
        let mut retained: Vec<_> = table
            .rows
            .iter()
            .flat_map(|row| &row.cells)
            .flat_map(|cell| &cell.spans)
            .collect();
        assert_eq!(retained.len(), count, "no lost or duplicated source witness");
        // Physical-cell multiline assembly can legitimately change flattened row-major order. ~keep
        retained.sort_by_key(|span| span.sequence);
        assert_eq!(
            format!("{retained:?}"),
            format!("{source_order:?}"),
            "all original witness fields survive exactly once"
        );
    }

    fn actual_table(source: &[TextSpan], paths: &[PathContent], count: usize) -> Table {
        let before = format!("{source:?}");
        let paths_before = format!("{paths:?}");
        let config = TableDetectionConfig {
            min_table_cells: 6,
            ..TableDetectionConfig::strict()
        };
        let intersections = detect_tables_from_intersections(source, paths, &config);
        let main = detect_tables_with_lines(source, paths, &config);
        assert_eq!(intersections.len(), 1, "actual intersection producer must fire");
        assert_eq!(main.len(), 1, "main detector must publish the same real table");
        assert_eq!(format!("{main:?}"), format!("{intersections:?}"));
        assert_eq!(format!("{source:?}"), before);
        assert_eq!(format!("{paths:?}"), paths_before);
        let table = main.into_iter().next().unwrap();
        assert_eq!((table.rows.len(), table.col_count), (ROWS, 3));
        assert_eq!(
            table.bbox,
            Some(Rect::new(0.0, 0.0, 3.0 * COLUMN_WIDTH, ROWS as f32 * PITCH))
        );
        assert_headerless(&table);
        assert_witnesses(&table, source, count);
        table
    }

    #[test]
    fn ordinary_intersection_empty_cell_is_not_rowspan_2332() {
        let mut source = source(Style::Regular);
        assert_eq!(source[2].text, "Product");
        source.remove(2);
        let paths = paths();
        assert_eq!(paths[ROWS - 1].bbox, Rect::new(0.0, 120.0, 300.0, 0.0));
        let cells = physical_cells(&paths);
        assert_eq!(cells.len(), 14);
        assert!(
            cells
                .iter()
                .any(|cell| cell_rect(cell) == Rect::new(0.0, 120.0, 100.0, 30.0))
        );
        assert!(
            cells
                .iter()
                .any(|cell| cell_rect(cell) == Rect::new(0.0, 90.0, 100.0, 30.0))
        );
        let table = actual_table(&source, &paths, 13);
        assert_eq!(table.rows[0].cells[0].rowspan, 1);
        assert_eq!(table.rows[0].cells[0].text, "Category");
        assert_eq!(table.rows[0].cells[0].bbox, Some(Rect::new(0.0, 120.0, 100.0, 30.0)));
        let lower = &table.rows[1].cells;
        assert_eq!(lower.len(), 3, "a genuinely separate ordinary empty slot must remain");
        assert_eq!(
            lower.iter().map(|cell| cell.text.as_str()).collect::<Vec<_>>(),
            vec!["", "Current", "Previous"]
        );
        assert!(lower[0].spans.is_empty());
        assert_eq!(lower[0].bbox, Some(Rect::new(0.0, 90.0, 100.0, 30.0)));
        for (column, cell) in lower.iter().enumerate() {
            assert_eq!((cell.colspan, cell.rowspan), (1, 1));
            assert_eq!(cell.bbox.unwrap().x, column as f32 * COLUMN_WIDTH);
        }
    }

    fn multiline_source() -> Vec<TextSpan> {
        let cells = [
            ["Category", "Measures", "Upper"],
            ["detailA", "detailB", "Lower"],
            ["Alpha", "12", "15"],
            ["Beta", "18", "21"],
            ["Gamma", "24", "27"],
        ];
        cells
            .iter()
            .enumerate()
            .flat_map(|(row, cells)| {
                cells
                    .iter()
                    .enumerate()
                    .map(move |(column, text)| span(text, row, column, ROWS, false))
            })
            .collect()
    }

    fn multiline_paths() -> Vec<PathContent> {
        let height = ROWS as f32 * PITCH;
        let mut paths: Vec<_> = (0..=ROWS)
            .map(|row| {
                let y = row as f32 * PITCH;
                let left = if row == ROWS - 1 { 2.0 * COLUMN_WIDTH } else { 0.0 };
                PathContent::line(left, y, 3.0 * COLUMN_WIDTH, y)
            })
            .collect();
        paths.extend((0..=3).map(|column| {
            let x = column as f32 * COLUMN_WIDTH;
            PathContent::line(x, 0.0, x, height)
        }));
        paths
    }

    #[test]
    fn intersection_multiline_spanning_cells_keep_witnesses_and_order_2332() {
        let source = multiline_source();
        assert_eq!(
            source.iter().map(|span| span.sequence).collect::<Vec<_>>(),
            (0..15).collect::<Vec<_>>()
        );
        let paths = multiline_paths();
        assert_eq!(paths[ROWS - 1].bbox, Rect::new(200.0, 120.0, 100.0, 0.0));
        let cells = physical_cells(&paths);
        assert_eq!(cells.len(), 13);
        for column in 0..2 {
            let rectangle = Rect::new(column as f32 * COLUMN_WIDTH, 90.0, COLUMN_WIDTH, 60.0);
            assert!(
                cells.iter().any(|cell| cell_rect(cell) == rectangle),
                "missing actual spanning footprint {rectangle:?}"
            );
        }
        let expected = [extract_cell_text(&[0, 3], &source), extract_cell_text(&[1, 4], &source)];
        assert_eq!(expected, ["Category\ndetailA", "Measures\ndetailB"]);
        let table = actual_table(&source, &paths, 15);
        for (column, text) in expected.iter().enumerate() {
            let anchor = &table.rows[0].cells[column];
            assert_eq!(
                anchor.rowspan, 2,
                "both drawn multiline cells cross the two upper source bands"
            );
            assert_eq!(anchor.colspan, 1);
            assert_eq!(anchor.text, *text);
            assert_eq!(
                anchor.bbox,
                Some(Rect::new(column as f32 * COLUMN_WIDTH, 90.0, COLUMN_WIDTH, 60.0))
            );
            assert_eq!(
                anchor.spans.iter().map(|span| span.sequence).collect::<Vec<_>>(),
                vec![column, column + 3]
            );
        }
        assert_eq!(table.rows[0].cells.len(), 3);
        assert_eq!(
            table.rows[1].cells.len(),
            1,
            "only the genuine right-column lower cell survives"
        );
        assert_eq!(table.rows[1].cells[0].text, "Lower");
        assert_eq!(table.rows[1].cells[0].bbox, Some(Rect::new(200.0, 90.0, 100.0, 30.0)));
        assert_eq!(table.rows[1].cells[0].spans[0].sequence, 5);
        assert!(table.rows[2..].iter().all(|row| row.cells.len() == 3));
        assert_witnesses(&table, &source, 15);
    }
}
