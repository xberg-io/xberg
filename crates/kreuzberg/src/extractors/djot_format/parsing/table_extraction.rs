//! Table extraction from Djot events.
//!
//! Parses table events and extracts table data.

use crate::types::Table;
use jotdown::{Container, Event};

/// Extract tables from Djot events.
///
/// Parses table events and extracts table data as a Vec<Vec<String>>,
/// converting each table to markdown representation for storage.
pub fn extract_tables_from_events(events: &[Event]) -> Vec<Table> {
    let mut tables = Vec::new();
    let mut current_table: Option<(Vec<Vec<String>>, usize)> = None;
    let mut current_row: Vec<String> = Vec::new();
    let mut current_cell = String::new();
    let mut in_table_cell = false;
    let mut table_index = 0;

    for event in events {
        match event {
            Event::Start(Container::Table, _) => {
                current_table = Some((Vec::new(), table_index));
            }
            Event::Start(Container::TableRow { .. }, _) => {
                current_row = Vec::new();
            }
            Event::Start(Container::TableCell { .. }, _) => {
                current_cell = String::new();
                in_table_cell = true;
            }
            Event::Str(s) if in_table_cell => {
                current_cell.push_str(s.as_ref());
            }
            Event::End(Container::TableCell { .. }) => {
                if in_table_cell {
                    current_row.push(current_cell.trim().to_string());
                    current_cell = String::new();
                    in_table_cell = false;
                }
            }
            Event::End(Container::TableRow { .. }) => {
                if !current_row.is_empty()
                    && let Some((ref mut rows, _)) = current_table
                {
                    rows.push(current_row.clone());
                }
                current_row = Vec::new();
            }
            Event::End(Container::Table) => {
                if let Some((cells, idx)) = current_table.take()
                    && !cells.is_empty()
                {
                    let markdown = crate::extractors::frontmatter_utils::cells_to_markdown(&cells);
                    tables.push(Table {
                        cells,
                        markdown,
                        page_number: idx + 1,
                    });
                    table_index += 1;
                }
            }
            _ => {}
        }
    }

    tables
}
