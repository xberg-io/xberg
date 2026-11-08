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

fn default_line_config() -> TableDetectionConfig {
    TableDetectionConfig {
        max_table_columns: DEFAULT_MAX_TABLE_COLUMNS,
        ..line_config()
    }
}

const PAGE: Rect = Rect {
    x: 0.0,
    y: 0.0,
    width: 612.0,
    height: 792.0,
};

#[test]
fn wide_table_on_a_non_ruled_route_is_declined() {
    let grid = fixture::grid();
    for config in [
        TableDetectionConfig {
            horizontal_strategy: TableStrategy::Text,
            ..line_config()
        },
        TableDetectionConfig {
            vertical_strategy: TableStrategy::Text,
            ..line_config()
        },
    ] {
        assert!(wide_ruled_ownership(&grid.table, &config, &grid.spans, &grid.paths, || Some(PAGE)).is_none());
    }
}

#[test]
fn wide_table_over_the_column_cap_is_declined() {
    let grid = fixture::grid();
    let config = TableDetectionConfig {
        max_table_columns: fixture::COLUMNS - 1,
        ..line_config()
    };
    assert!(wide_ruled_ownership(&grid.table, &config, &grid.spans, &grid.paths, || Some(PAGE)).is_none());
}

#[test]
fn prose_shaped_wide_table_is_declined() {
    let mut grid = fixture::grid();
    for cell in grid.table.rows.iter_mut().flat_map(|row| &mut row.cells) {
        cell.text = "word".to_string();
    }
    assert!(looks_like_prose_table(&grid.table));
    assert!(wide_ruled_ownership(&grid.table, &line_config(), &grid.spans, &grid.paths, || Some(PAGE)).is_none());
}

fn recovered(config: &TableDetectionConfig, grid: &fixture::Grid, page: Option<Rect>) -> bool {
    recover_wide_source_table(config, &grid.spans, &grid.paths, || page).is_some()
}

#[test]
fn empty_default_route_recovers_the_wide_table() {
    let grid = fixture::grid();
    let (tables, ownership) =
        recover_wide_source_table(&default_line_config(), &grid.spans, &grid.paths, || Some(PAGE))
            .expect("the fixture is the recoverable shape");
    assert_eq!(tables.len(), 1);
    assert_eq!(tables[0].col_count, fixture::COLUMNS);
    assert!(ownership.is_some_and(|ownership| ownership.replaces_empty_ruled_route));
}

#[test]
fn recovery_declines_a_non_ruled_route() {
    let grid = fixture::grid();
    let config = TableDetectionConfig {
        horizontal_strategy: TableStrategy::Text,
        ..default_line_config()
    };
    assert!(!recovered(&config, &grid, Some(PAGE)));
}

#[test]
fn recovery_declines_a_raised_column_cap() {
    let grid = fixture::grid();
    assert!(!recovered(&line_config(), &grid, Some(PAGE)));
}

#[test]
fn recovery_declines_a_lowered_column_cap() {
    let grid = fixture::grid();
    let config = TableDetectionConfig {
        max_table_columns: 10,
        ..line_config()
    };
    assert!(!recovered(&config, &grid, Some(PAGE)));
}

#[test]
fn recovery_declines_a_page_of_unknown_bounds() {
    let grid = fixture::grid();
    assert!(!recovered(&default_line_config(), &grid, None));
}

#[test]
fn recovery_declines_a_table_outside_the_page() {
    let grid = fixture::grid();
    assert!(!recovered(
        &default_line_config(),
        &grid,
        Some(Rect::new(400.0, 0.0, 212.0, 792.0))
    ));
}

#[test]
fn recovery_declines_a_prose_shaped_table() {
    let mut grid = fixture::grid();
    for span in &mut grid.spans {
        span.text = "word".to_string();
    }
    let mut retry = default_line_config();
    retry.max_table_columns = MAX_SOURCE_SPANNED_COLUMNS;
    let tables = detect_tables_without_painted_recovery(&grid.spans, &grid.paths, &retry);
    let [table] = tables.as_slice() else {
        panic!("the detector must find the one table for the prose check to be what declines");
    };
    assert_eq!(table.col_count, fixture::COLUMNS);
    assert!(looks_like_prose_table(table));
    assert!(!recovered(&default_line_config(), &grid, Some(PAGE)));
}

#[test]
fn recovery_declines_a_page_with_two_wide_tables() {
    let mut grid = fixture::grid();
    let second = fixture::grid_at(200.0, grid.spans.len());
    grid.spans.extend(second.spans);
    grid.paths.extend(second.paths);
    let mut retry = default_line_config();
    retry.max_table_columns = MAX_SOURCE_SPANNED_COLUMNS;
    assert_eq!(
        detect_tables_without_painted_recovery(&grid.spans, &grid.paths, &retry).len(),
        2,
        "the detector must see two tables for the single-table guard to be what declines"
    );
    assert!(!recovered(&default_line_config(), &grid, Some(PAGE)));
}
