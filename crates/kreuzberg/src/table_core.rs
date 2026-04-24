//! Core table reconstruction types and algorithms.
//!
//! This module provides the `HocrWord` type and table reconstruction functions
//! that are shared between the OCR and PDF modules. The algorithms detect
//! column/row structure from word bounding boxes and reconstruct tabular layouts.
//!
//! Originally adapted from the `hocr` module of `html-to-markdown-rs` (removed in v3).

/// Represents a word extracted from hOCR (or any source) with position and confidence information.
#[derive(Debug, Clone)]
pub struct HocrWord {
    pub text: String,
    pub left: u32,
    pub top: u32,
    pub width: u32,
    pub height: u32,
    pub confidence: f64,
}

impl HocrWord {
    /// Get the right edge position.
    #[cfg(test)]
    #[inline]
    pub(crate) fn right(&self) -> u32 {
        self.left + self.width
    }

    /// Get the bottom edge position.
    #[cfg(test)]
    #[inline]
    pub(crate) fn bottom(&self) -> u32 {
        self.top + self.height
    }

    /// Get the vertical center position.
    #[inline]
    pub(crate) fn y_center(&self) -> f64 {
        self.top as f64 + (self.height as f64 / 2.0)
    }

    /// Get the horizontal center position.
    #[cfg(test)]
    #[inline]
    pub(crate) fn x_center(&self) -> f64 {
        self.left as f64 + (self.width as f64 / 2.0)
    }
}

/// Detect column positions from word x-coordinates.
///
/// Groups words by approximate x-position (within `column_threshold` pixels)
/// and returns the median x-position for each detected column, sorted left to right.
pub(crate) fn detect_columns(words: &[HocrWord], column_threshold: u32) -> Vec<u32> {
    if words.is_empty() {
        return Vec::new();
    }

    // Group words by approximate x-position
    let mut position_groups: Vec<Vec<u32>> = Vec::new();

    for word in words {
        let x_pos = word.left;

        // Find existing group within threshold
        let mut found_group = false;
        for group in &mut position_groups {
            if let Some(&first_pos) = group.first()
                && x_pos.abs_diff(first_pos) <= column_threshold
            {
                group.push(x_pos);
                found_group = true;
                break;
            }
        }

        // Create new group if not found
        if !found_group {
            position_groups.push(vec![x_pos]);
        }
    }

    // Calculate median for each group
    let mut columns: Vec<u32> = position_groups
        .iter()
        .filter(|group| !group.is_empty())
        .map(|group| {
            let mut sorted = group.clone();
            sorted.sort_unstable();
            let mid = sorted.len() / 2;
            sorted[mid]
        })
        .collect();

    // Sort columns left to right
    columns.sort_unstable();
    columns
}

/// Detect row positions from word y-coordinates.
///
/// Groups words by their vertical center position and returns the median
/// y-position for each detected row. The `row_threshold_ratio` is multiplied
/// by the median word height to determine the grouping threshold.
pub(crate) fn detect_rows(words: &[HocrWord], row_threshold_ratio: f64) -> Vec<u32> {
    if words.is_empty() {
        return Vec::new();
    }

    // Calculate median height for threshold
    let mut heights: Vec<u32> = words.iter().map(|w| w.height).collect();
    heights.sort_unstable();
    let median_height = heights[heights.len() / 2];
    let row_threshold = (median_height as f64 * row_threshold_ratio) as u32;

    // Group words by approximate y-center
    let mut position_groups: Vec<Vec<f64>> = Vec::new();

    for word in words {
        let y_center = word.y_center();

        // Find existing group within threshold
        let mut found_group = false;
        for group in &mut position_groups {
            if let Some(&first_pos) = group.first()
                && (y_center - first_pos).abs() <= row_threshold as f64
            {
                group.push(y_center);
                found_group = true;
                break;
            }
        }

        // Create new group if not found
        if !found_group {
            position_groups.push(vec![y_center]);
        }
    }

    // Calculate median for each group
    let mut rows: Vec<u32> = position_groups
        .iter()
        .filter(|group| !group.is_empty())
        .map(|group| {
            let mut sorted = group.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let mid = sorted.len() / 2;
            sorted[mid] as u32
        })
        .collect();

    // Sort rows top to bottom
    rows.sort_unstable();
    rows
}

/// Find which row a word belongs to based on its y-center.
fn find_row_index(row_positions: &[u32], word: &HocrWord) -> Option<usize> {
    let y_center = word.y_center() as u32;

    row_positions
        .iter()
        .enumerate()
        .min_by_key(|&(_, row_y)| row_y.abs_diff(y_center))
        .map(|(idx, _)| idx)
}

/// Find which column a word belongs to based on its x-position.
fn find_column_index(col_positions: &[u32], word: &HocrWord) -> Option<usize> {
    let x_pos = word.left;

    col_positions
        .iter()
        .enumerate()
        .min_by_key(|&(_, col_x)| col_x.abs_diff(x_pos))
        .map(|(idx, _)| idx)
}

/// Remove empty rows and columns from a table grid.
fn remove_empty_rows_and_columns(table: Vec<Vec<String>>) -> Vec<Vec<String>> {
    if table.is_empty() {
        return table;
    }

    // Find non-empty columns
    let num_cols = table[0].len();
    let mut non_empty_cols: Vec<bool> = vec![false; num_cols];

    for row in &table {
        for (col_idx, cell) in row.iter().enumerate() {
            if !cell.trim().is_empty() {
                non_empty_cols[col_idx] = true;
            }
        }
    }

    // Filter rows and columns
    table
        .into_iter()
        .filter(|row| row.iter().any(|cell| !cell.trim().is_empty()))
        .map(|row| {
            row.into_iter()
                .enumerate()
                .filter(|(idx, _)| non_empty_cols[*idx])
                .map(|(_, cell)| cell)
                .collect()
        })
        .collect()
}

/// Reconstruct a table grid from words with bounding box positions.
///
/// Takes detected words and reconstructs a 2D table by:
/// 1. Detecting column positions (grouping by x-coordinate within `column_threshold`)
/// 2. Detecting row positions (grouping by y-center within `row_threshold_ratio` * median height)
/// 3. Assigning words to cells based on closest row/column
/// 4. Combining words within the same cell
///
/// Returns a `Vec<Vec<String>>` where each inner `Vec` is a row of cell texts.
pub(crate) fn reconstruct_table(
    words: &[HocrWord],
    column_threshold: u32,
    row_threshold_ratio: f64,
) -> Vec<Vec<String>> {
    if words.is_empty() {
        return Vec::new();
    }

    // Detect table structure
    let col_positions = detect_columns(words, column_threshold);
    let row_positions = detect_rows(words, row_threshold_ratio);

    if col_positions.is_empty() || row_positions.is_empty() {
        return Vec::new();
    }

    // Initialize table grid
    let num_rows = row_positions.len();
    let num_cols = col_positions.len();
    let mut table: Vec<Vec<Vec<String>>> = vec![vec![vec![]; num_cols]; num_rows];

    // Assign words to cells
    for word in words {
        if let (Some(r), Some(c)) = (
            find_row_index(&row_positions, word),
            find_column_index(&col_positions, word),
        ) && r < num_rows
            && c < num_cols
        {
            table[r][c].push(word.text.clone());
        }
    }

    // Combine words within cells
    let result: Vec<Vec<String>> = table
        .into_iter()
        .map(|row| {
            row.into_iter()
                .map(|cell_words| {
                    if cell_words.is_empty() {
                        String::new()
                    } else {
                        cell_words.join(" ")
                    }
                })
                .collect()
        })
        .collect();

    // Remove empty rows and columns
    remove_empty_rows_and_columns(result)
}

/// Convert a table grid to markdown format.
///
/// The first row is treated as the header row, with a separator line added after it.
/// Pipe characters in cell content are escaped.
pub(crate) fn table_to_markdown(table: &[Vec<String>]) -> String {
    if table.is_empty() {
        return String::new();
    }

    let num_cols = table[0].len();
    if num_cols == 0 {
        return String::new();
    }

    let mut markdown = String::new();

    // Add rows
    for (row_idx, row) in table.iter().enumerate() {
        markdown.push('|');
        for cell in row {
            markdown.push(' ');
            // Escape pipes in cell content
            markdown.push_str(&cell.replace('|', "\\|"));
            markdown.push_str(" |");
        }
        markdown.push('\n');

        // Add header separator after first row
        if row_idx == 0 {
            markdown.push('|');
            for _ in 0..num_cols {
                markdown.push_str(" --- |");
            }
            markdown.push('\n');
        }
    }

    markdown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hocr_word_methods() {
        let word = HocrWord {
            text: "Hello".to_string(),
            left: 100,
            top: 50,
            width: 80,
            height: 30,
            confidence: 95.5,
        };

        assert_eq!(word.right(), 180);
        assert_eq!(word.bottom(), 80);
        assert_eq!(word.y_center(), 65.0);
        assert_eq!(word.x_center(), 140.0);
    }

    #[test]
    fn test_detect_columns() {
        let words = vec![
            HocrWord {
                text: "A".to_string(),
                left: 100,
                top: 50,
                width: 20,
                height: 30,
                confidence: 95.0,
            },
            HocrWord {
                text: "B".to_string(),
                left: 300,
                top: 50,
                width: 20,
                height: 30,
                confidence: 95.0,
            },
            HocrWord {
                text: "C".to_string(),
                left: 105,
                top: 100,
                width: 20,
                height: 30,
                confidence: 95.0,
            },
            HocrWord {
                text: "D".to_string(),
                left: 295,
                top: 100,
                width: 20,
                height: 30,
                confidence: 95.0,
            },
        ];

        let cols = detect_columns(&words, 20);
        assert_eq!(cols.len(), 2);
    }

    #[test]
    fn test_detect_rows() {
        let words = vec![
            HocrWord {
                text: "A".to_string(),
                left: 100,
                top: 50,
                width: 20,
                height: 30,
                confidence: 95.0,
            },
            HocrWord {
                text: "B".to_string(),
                left: 200,
                top: 52,
                width: 20,
                height: 30,
                confidence: 95.0,
            },
            HocrWord {
                text: "C".to_string(),
                left: 100,
                top: 100,
                width: 20,
                height: 30,
                confidence: 95.0,
            },
        ];

        let rows = detect_rows(&words, 0.5);
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn test_reconstruct_table_basic() {
        let words = vec![
            HocrWord {
                text: "Name".to_string(),
                left: 100,
                top: 50,
                width: 40,
                height: 20,
                confidence: 95.0,
            },
            HocrWord {
                text: "Value".to_string(),
                left: 300,
                top: 50,
                width: 40,
                height: 20,
                confidence: 95.0,
            },
            HocrWord {
                text: "Alice".to_string(),
                left: 100,
                top: 100,
                width: 40,
                height: 20,
                confidence: 95.0,
            },
            HocrWord {
                text: "42".to_string(),
                left: 300,
                top: 100,
                width: 20,
                height: 20,
                confidence: 95.0,
            },
        ];

        let table = reconstruct_table(&words, 20, 0.5);
        assert_eq!(table.len(), 2);
        assert_eq!(table[0].len(), 2);
        assert_eq!(table[0][0], "Name");
        assert_eq!(table[0][1], "Value");
        assert_eq!(table[1][0], "Alice");
        assert_eq!(table[1][1], "42");
    }

    #[test]
    fn test_table_to_markdown_basic() {
        let table = vec![
            vec!["Name".to_string(), "Value".to_string()],
            vec!["Alice".to_string(), "42".to_string()],
        ];

        let md = table_to_markdown(&table);
        assert!(md.contains("| Name | Value |"));
        assert!(md.contains("| --- | --- |"));
        assert!(md.contains("| Alice | 42 |"));
    }

    #[test]
    fn test_table_to_markdown_empty() {
        assert_eq!(table_to_markdown(&[]), String::new());
    }

    #[test]
    fn test_table_to_markdown_escapes_pipes() {
        let table = vec![vec!["Header".to_string()], vec!["a|b".to_string()]];

        let md = table_to_markdown(&table);
        assert!(md.contains("a\\|b"));
    }
}
