use super::*;
use crate::geometry::Rect;
use crate::structure::spatial_table_detector::source_spanned_fixture as fixture;

fn line_config() -> TableDetectionConfig {
    TableDetectionConfig {
        horizontal_strategy: TableStrategy::Lines,
        vertical_strategy: TableStrategy::Lines,
        max_table_columns: fixture::COLUMNS,
        ..Default::default()
    }
}

fn admitted(page: Rect) -> bool {
    let grid = fixture::grid();
    wide_ruled_ownership(&grid.table, &line_config(), &grid.spans, &grid.paths, || Some(page)).is_some()
}

#[test]
fn wide_table_inside_the_page_is_admitted() {
    assert!(admitted(Rect::new(0.0, 0.0, 612.0, 792.0)));
}

#[test]
fn wide_table_outside_the_page_is_declined() {
    assert!(!admitted(Rect::new(400.0, 0.0, 212.0, 792.0)));
}

#[test]
fn wide_table_on_a_page_of_unknown_bounds_is_declined() {
    let grid = fixture::grid();
    assert!(wide_ruled_ownership(&grid.table, &line_config(), &grid.spans, &grid.paths, || None).is_none());
}
