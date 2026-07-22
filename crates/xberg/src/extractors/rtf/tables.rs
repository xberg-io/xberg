//! Table extraction and state management for RTF documents.

use crate::extraction::{cells_to_markdown, cells_to_text};
use crate::types::Table;

/// State machine for tracking table construction during RTF parsing.
pub struct TableState {
    pub rows: Vec<Vec<String>>,
    pub current_row: Vec<String>,
    pub current_cell: String,
    pub in_row: bool,
    /// Set after `\row` to indicate that another `\trowd` may follow
    /// for the same table. Prevents premature finalization when stray
    /// whitespace or formatting control words appear between rows.
    pub expecting_next_row: bool,
}

impl TableState {
    /// Create a new empty table state.
    pub(crate) fn new() -> Self {
        Self {
            rows: Vec::new(),
            current_row: Vec::new(),
            current_cell: String::new(),
            in_row: false,
            expecting_next_row: false,
        }
    }

    /// Push the current cell content to the current row.
    pub(crate) fn push_cell(&mut self) {
        let cell = self.current_cell.trim().to_string();
        self.current_row.push(cell);
        self.current_cell.clear();
    }

    /// Push the current row to the rows collection.
    pub(crate) fn push_row(&mut self) {
        if !self.current_cell.is_empty() {
            self.push_cell();
        }
        self.in_row = false;
        self.expecting_next_row = true;
        if !self.current_row.is_empty() {
            self.rows.push(self.current_row.clone());
            self.current_row.clear();
        }
    }

    /// Start a new table row.
    pub(crate) fn start_row(&mut self) {
        if self.in_row {
            self.push_row();
        }
        self.in_row = true;
        self.expecting_next_row = false;
        self.current_cell.clear();
        self.current_row.clear();
    }

    /// Finalize the table with format control. If `plain` is true, the table
    /// text representation uses tab-separated format instead of markdown pipes.
    pub(crate) fn finalize_with_format(mut self, plain: bool) -> Option<Table> {
        if self.in_row || !self.current_cell.is_empty() || !self.current_row.is_empty() {
            self.push_row();
        }

        if self.rows.is_empty() {
            return None;
        }

        let markdown = if plain {
            cells_to_text(&self.rows)
        } else {
            cells_to_markdown(&self.rows)
        };
        Some(Table {
            cells: self.rows,
            markdown,
            page_number: 1,
            bounding_box: None,
            ..Default::default()
        })
    }
}

impl Default for TableState {
    fn default() -> Self {
        Self::new()
    }
}
