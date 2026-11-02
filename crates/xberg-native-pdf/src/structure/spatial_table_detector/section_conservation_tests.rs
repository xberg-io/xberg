use super::*;
use crate::elements::PathContent;
use crate::geometry::Rect;
use crate::layout::TextSpan;
use crate::structure::table_extractor::TableCell;

fn source_span(text: String, x: f32, y: f32) -> TextSpan {
    TextSpan {
        text,
        bbox: Rect::new(x, y, 20.0, 8.0),
        font_name: "SectionControl".into(),
        font_size: 12.0,
        ..Default::default()
    }
}

fn drawn_sections() -> (Vec<TextSpan>, Vec<PathContent>) {
    let mut lines: Vec<_> = (0..=9)
        .map(|row| PathContent::line(10.0, 10.0 + row as f32 * 10.0, 100.0, 10.0 + row as f32 * 10.0))
        .collect();
    for start in [10.0, 40.0, 70.0] {
        for x in [10.0, 40.0, 70.0, 100.0] {
            lines.push(PathContent::line(x, start, x, start + 30.0));
        }
    }
    let mut spans = Vec::new();
    for row in 0..9 {
        for col in 0..3 {
            spans.push(source_span(
                format!("S{}-R{}-C{}", row / 3 + 1, row % 3 + 1, col + 1),
                15.0 + col as f32 * 30.0,
                15.0 + row as f32 * 10.0,
            ));
        }
    }
    (spans, lines)
}

#[test]
fn valid_drawn_sections_still_split_with_every_source_cell_exactly_once() {
    let (spans, lines) = drawn_sections();
    let config = TableDetectionConfig {
        horizontal_strategy: TableStrategy::Lines,
        vertical_strategy: TableStrategy::Lines,
        ..Default::default()
    };
    let (tables, events) = header_source_diagnostic::capture(|| detect_tables_with_lines(&spans, &lines, &config));
    let mut assigned: Vec<_> = events
        .iter()
        .filter(|event| event["stage"] == "intersection_assigned")
        .flat_map(|event| event["payload"]["source_span_ids"].as_array().unwrap())
        .flat_map(|row| row.as_array().unwrap())
        .flat_map(|cell| cell.as_array().unwrap())
        .map(|id| id.as_u64().unwrap())
        .collect();
    assigned.sort_unstable();
    assert_eq!(
        assigned,
        (0..27).collect::<Vec<_>>(),
        "actual producer assignment must contain each source span once"
    );
    let mut expected: Vec<_> = spans.iter().map(|span| span.text.as_str()).collect();
    expected.sort_unstable();
    let mut actual: Vec<_> = tables
        .iter()
        .flat_map(|table| &table.rows)
        .flat_map(|row| &row.cells)
        .filter(|cell| !cell.text.is_empty())
        .map(|cell| cell.text.as_str())
        .collect();
    actual.sort_unstable();
    assert_eq!(tables.len(), 3, "valid sections must still split");
    assert!(tables.iter().all(|table| table.col_count == 3));
    assert_eq!(actual, expected, "all 27 source-cell payloads survive exactly once");
}

fn admitted_table() -> Table {
    let mut table = Table::new();
    table.col_count = 3;
    table.has_header = true;
    table.bbox = Some(Rect::new(0.0, 0.0, 90.0, 40.0));
    for row in 0..4 {
        let mut source_row = TableRow::new(row == 0);
        for col in 0..3 {
            let id = u32::try_from(row * 3 + col).unwrap();
            let text = format!("R{row}C{col}");
            let mut cell = TableCell::new(text.clone(), row == 0);
            cell.bbox = Some(Rect::new(col as f32 * 30.0, row as f32 * 10.0, 30.0, 10.0));
            cell.mcids = vec![id];
            let mut span = source_span(text, col as f32 * 30.0 + 2.0, row as f32 * 10.0 + 1.0);
            span.mcid = Some(id);
            cell.spans.push(span);
            source_row.add_cell(cell);
        }
        table.rows.push(source_row);
    }
    table
}

#[test]
fn undersized_section_preserves_the_whole_admitted_table_and_metadata() {
    let table = admitted_table();
    let config = TableDetectionConfig::strict();
    assert!(table.is_real_grid());
    assert_eq!(
        table
            .rows
            .iter()
            .flat_map(|row| &row.cells)
            .filter(|cell| !cell.text.is_empty())
            .count(),
        12
    );
    assert!(table.rows[0].cells.len() < config.min_table_cells);
    assert!(table.rows[1..].iter().map(|row| row.cells.len()).sum::<usize>() >= config.min_table_cells);
    let expected = header_source_diagnostic::tables(std::slice::from_ref(&table));
    let divider = Edge {
        coord: 10.0,
        start: 0.0,
        end: 90.0,
    };
    let actual = split_table_at_section_dividers(table, &[divider], &[], &config);
    assert_eq!(
        header_source_diagnostic::tables(&actual),
        expected,
        "an unretainable nonempty partition must not delete rows or metadata"
    );
}
