//! Conservative normalization for native PDF table grids.

use crate::types::Table;

const MIN_REPEATED_DATA_ROWS: usize = 8;
const MIN_WIDE_TABLE_COLUMNS: usize = 16;

/// Reject a page letterhead emitted as a table.
///
/// A two-column letterhead (company name, plan line, place) is an unruled,
/// three-row 2x2-ish block: at most one row fills both columns, one row has
/// only the left cell and another only the right cell. A real two-column
/// table repeats both columns across its rows and carries values or at least
/// more than one full row. ~keep
pub(crate) fn is_page_letterhead_table(table: &Table) -> bool {
    const MAX_LETTERHEAD_ROWS: usize = 3;
    const MAX_LETTERHEAD_CELLS: usize = 5;
    if table.cells.len() > MAX_LETTERHEAD_ROWS || table.cells.iter().any(|row| row.len() != 2) {
        return false;
    }
    let mut filled = 0usize;
    let mut full_rows = 0usize;
    let mut left_only = false;
    let mut right_only = false;
    let mut has_numeric = false;
    let mut has_multiword_cell = false;
    for row in &table.cells {
        let left = row[0].trim();
        let right = row[1].trim();
        filled += usize::from(!left.is_empty()) + usize::from(!right.is_empty());
        match (left.is_empty(), right.is_empty()) {
            (false, false) => full_rows += 1,
            (false, true) => left_only = true,
            (true, false) => right_only = true,
            (true, true) => {}
        }
        for cell in [left, right] {
            has_numeric |= cell.chars().any(|character| character.is_ascii_digit());
            has_multiword_cell |= cell.split_whitespace().count() > 1;
        }
    }
    filled <= MAX_LETTERHEAD_CELLS && full_rows <= 1 && left_only && right_only && !has_numeric && has_multiword_cell
}

/// Repairs one column boundary that a native detector consistently collapsed.
///
/// Returns `true` when the table was changed.
pub(crate) fn repair_consistently_merged_numeric_column(table: &mut Table) -> bool {
    // TODO(https://github.com/yfedoseev/xberg_native_pdf/issues/975): remove this repair after upstream preserves
    // adjacent numeric columns. ~keep
    let Some(header) = table.cells.first() else {
        return false;
    };
    let column_count = header.len();
    let Some(data_rows) = table.cells.get(1..) else {
        return false;
    };
    if data_rows.len() < MIN_REPEATED_DATA_ROWS
        || column_count < MIN_WIDE_TABLE_COLUMNS
        || data_rows.iter().any(|row| row.len() != column_count)
        || table
            .columns
            .as_ref()
            .is_some_and(|columns| columns.len() != column_count)
    {
        return false;
    }

    let candidates: Vec<usize> = (1..column_count - 1)
        .filter(|&column| {
            let candidate_header = header[column].trim();
            !candidate_header.is_empty()
                && candidate_header == header[column + 1].trim()
                && data_rows.iter().all(|row| {
                    split_merged_numeric_cell(&row[column]).is_some_and(|(left, right)| left == right)
                        && is_numeric_atom(&row[column - 1])
                        && is_numeric_atom(&row[column + 1])
                })
        })
        .collect();
    let [column] = candidates.as_slice() else {
        return false;
    };

    table.cells[0].insert(*column, String::new());
    for row in &mut table.cells[1..] {
        let Some((left, right)) = split_merged_numeric_cell(&row[*column]) else {
            return false;
        };
        let left = left.to_string();
        let right = right.to_string();
        row[*column] = left;
        row.insert(*column + 1, right);
    }
    if let Some(columns) = table.columns.as_mut()
        && columns.len() == column_count
    {
        columns.insert(*column, String::new());
    }
    table.markdown = crate::pdf::table_reconstruct::table_to_markdown(&table.cells);
    true
}

fn split_merged_numeric_cell(cell: &str) -> Option<(&str, &str)> {
    let bytes = cell.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if !bytes[index].is_ascii_whitespace() {
            index += 1;
            continue;
        }
        let start = index;
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if index - start < 2 {
            continue;
        }
        let left = cell[..start].trim();
        let right = cell[index..].trim();
        if is_numeric_atom(left) && is_numeric_atom(right) {
            return Some((left, right));
        }
    }
    None
}

fn is_numeric_atom(cell: &str) -> bool {
    let mut value = cell.trim();
    if value.starts_with('(') && value.ends_with(')') {
        value = &value[1..value.len() - 1];
    }
    if let Some(stripped) = value.strip_prefix(['$', '€', '£', '¥']) {
        value = stripped;
    }
    if let Some(stripped) = value.strip_suffix('%') {
        value = stripped;
    }
    if value.is_empty() {
        return false;
    }
    let normalized = value.replace(',', "");
    normalized.parse::<f64>().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table_with_rows(rows: Vec<Vec<&str>>) -> Table {
        Table {
            cells: rows
                .into_iter()
                .map(|row| row.into_iter().map(str::to_string).collect())
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn rejects_two_column_page_letterhead() {
        let table = table_with_rows(vec![
            vec!["WASHINGTON", "NATIONAL INSURANCE COMPANY"],
            vec!["Medicare Select -- Plan A,B, C, & D", ""],
            vec!["", "Texas Experience"],
        ]);

        assert!(is_page_letterhead_table(&table));
    }

    #[test]
    fn keeps_a_real_two_column_table() {
        let table = table_with_rows(vec![
            vec!["Plan", "Premium"],
            vec!["Plan A", "120.00"],
            vec!["Plan B", "145.00"],
            vec!["Plan C", "170.00"],
        ]);

        assert!(!is_page_letterhead_table(&table));
    }

    #[test]
    fn keeps_a_short_two_column_table_with_more_than_one_full_row() {
        let table = table_with_rows(vec![
            vec!["Name", "Amount"],
            vec!["Alpha", ""],
            vec!["", "Beta"],
            vec!["Gamma", "Delta"],
        ]);

        assert!(!is_page_letterhead_table(&table));
    }

    fn repeated_rows(candidate: &str, count: usize) -> Vec<Vec<&str>> {
        std::iter::repeat_n(
            vec![
                "002", "AO", "1000", "1.000", "1.000", candidate, "1.000", "1.000", "1.000", "1.000", "1.000", "1.000",
                "1.000", "1.000", "1.000", "1.000",
            ],
            count,
        )
        .collect()
    }

    #[test]
    fn should_split_consistently_merged_numeric_column() {
        const DETECTED_COLUMNS: usize = 23;
        const DATA_ROWS: usize = 105;
        const MERGED_COLUMN: usize = 17;
        let mut header = vec![String::new(); DETECTED_COLUMNS];
        header[MERGED_COLUMN] = "Loan /Lease".to_string();
        header[MERGED_COLUMN + 1] = "Loan /Lease".to_string();
        let mut cells = vec![header];
        for _ in 0..DATA_ROWS {
            let mut row = vec!["1.000".to_string(); DETECTED_COLUMNS];
            row[0] = "002".to_string();
            row[1] = "AO".to_string();
            row[2] = "1000".to_string();
            row[MERGED_COLUMN] = "1.000  1.000".to_string();
            cells.push(row);
        }
        let mut table = Table {
            cells,
            columns: Some(vec![String::new(); DETECTED_COLUMNS]),
            ..Default::default()
        };

        assert!(repair_consistently_merged_numeric_column(&mut table));
        assert!(table.cells.iter().all(|row| row.len() == 24));
        assert_eq!(table.cells[0][MERGED_COLUMN], "");
        assert_eq!(table.cells[0][MERGED_COLUMN + 1], "Loan /Lease");
        assert_eq!(&table.cells[1][MERGED_COLUMN..=MERGED_COLUMN + 1], ["1.000", "1.000"]);
        assert_eq!(table.columns.as_ref().map(Vec::len), Some(24));
        assert!(table.markdown.contains("| 1.000 | 1.000 |"));
    }

    #[test]
    fn should_not_split_when_data_rows_are_irregular() {
        let mut header = vec![""; MIN_WIDE_TABLE_COLUMNS];
        header[5] = "Value";
        header[6] = "Value";
        let mut rows = vec![header];
        rows.extend(repeated_rows("1.000  1.000", MIN_REPEATED_DATA_ROWS));
        rows[4][5] = "1.000";
        let mut table = table_with_rows(rows);

        assert!(!repair_consistently_merged_numeric_column(&mut table));
    }

    #[test]
    fn should_not_split_prose_cells() {
        let mut header = vec![""; MIN_WIDE_TABLE_COLUMNS];
        header[5] = "Value";
        header[6] = "Value";
        let mut rows = vec![header];
        rows.extend(repeated_rows("alpha  beta", MIN_REPEATED_DATA_ROWS));
        let mut table = table_with_rows(rows);

        assert!(!repair_consistently_merged_numeric_column(&mut table));
    }

    #[test]
    fn should_not_split_legitimate_multi_number_cells() {
        const COLUMNS: usize = MIN_WIDE_TABLE_COLUMNS;
        const CANDIDATE: usize = 8;
        let mut header = vec![String::new(); COLUMNS];
        header[CANDIDATE] = "Range".to_string();
        header[CANDIDATE + 1] = "Range".to_string();
        let mut cells = vec![header];
        for _ in 0..MIN_REPEATED_DATA_ROWS {
            let mut row = vec!["1".to_string(); COLUMNS];
            row[CANDIDATE] = "10  20".to_string();
            cells.push(row);
        }
        let mut table = Table {
            cells,
            ..Default::default()
        };

        assert!(!repair_consistently_merged_numeric_column(&mut table));
    }

    #[test]
    fn should_not_split_when_columns_metadata_is_stale() {
        let mut header = vec![""; MIN_WIDE_TABLE_COLUMNS];
        header[5] = "Value";
        header[6] = "Value";
        let mut rows = vec![header];
        let data_row = vec![
            "1",
            "1",
            "1",
            "1",
            "1",
            "1.000  1.000",
            "1",
            "1",
            "1",
            "1",
            "1",
            "1",
            "1",
            "1",
            "1",
            "1",
        ];
        rows.extend(std::iter::repeat_n(data_row, MIN_REPEATED_DATA_ROWS));
        let mut table = table_with_rows(rows);
        table.columns = Some(vec!["stale".to_string()]);

        assert!(!repair_consistently_merged_numeric_column(&mut table));
        assert!(table.cells.iter().all(|row| row.len() == MIN_WIDE_TABLE_COLUMNS));
    }
}
