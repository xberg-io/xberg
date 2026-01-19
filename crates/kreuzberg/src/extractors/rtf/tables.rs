//! Table extraction and state management for RTF documents.

use crate::extraction::cells_to_markdown;
use crate::types::Table;

/// State machine for tracking table construction during RTF parsing.
pub struct TableState {
    pub rows: Vec<Vec<String>>,
    pub current_row: Vec<String>,
    pub current_cell: String,
    pub in_row: bool,
}

impl TableState {
    /// Create a new empty table state.
    pub fn new() -> Self {
        Self {
            rows: Vec::new(),
            current_row: Vec::new(),
            current_cell: String::new(),
            in_row: false,
        }
    }

    /// Push the current cell content to the current row.
    pub fn push_cell(&mut self) {
        let cell = self.current_cell.trim().to_string();
        self.current_row.push(cell);
        self.current_cell.clear();
    }

    /// Push the current row to the rows collection.
    pub fn push_row(&mut self) {
        if self.in_row || !self.current_cell.is_empty() {
            self.push_cell();
            self.in_row = false;
        }
        if !self.current_row.is_empty() {
            self.rows.push(self.current_row.clone());
            self.current_row.clear();
        }
    }

    /// Start a new table row.
    pub fn start_row(&mut self) {
        if self.in_row {
            self.push_row();
        }
        self.in_row = true;
        self.current_cell.clear();
        self.current_row.clear();
    }

    /// Check if this table has any content.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty() && self.current_row.is_empty() && self.current_cell.is_empty()
    }

    /// Finalize the table and convert it to a Table struct.
    pub fn finalize(mut self) -> Option<Table> {
        if self.in_row || !self.current_cell.is_empty() || !self.current_row.is_empty() {
            self.push_row();
        }

        if self.rows.is_empty() {
            return None;
        }

        let markdown = cells_to_markdown(&self.rows);
        Some(Table {
            cells: self.rows,
            markdown,
            page_number: 1,
        })
    }
}

impl Default for TableState {
    fn default() -> Self {
        Self::new()
    }
}
