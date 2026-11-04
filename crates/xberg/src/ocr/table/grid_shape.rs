//! The rule that separates a table from text that only sits in a staircase.
//!
//! OCR table reconstruction groups words by their pixel columns and rows. A cover page, a map
//! or a diagram has labels at scattered positions, and those labels also fall into columns.
//! Both the `pdf` and the non-`pdf` build apply this rule, and so does the PaddleOCR producer.

/// A cell with this many words or more reads as a sentence, not as a label or a value.
const PROSE_WORDS: usize = 4;
/// A block needs a header row and two body rows that fill the same columns.
const BLOCK_ROWS: usize = 3;

fn filled(cell: &str) -> bool {
    !cell.trim().is_empty()
}

fn is_prose(cell: &str) -> bool {
    cell.split_whitespace().count() >= PROSE_WORDS
}

/// A number, an amount or a date: digits with only number punctuation around them.
fn is_value(cell: &str) -> bool {
    let cell = cell.trim();
    cell.chars().any(|c| c.is_ascii_digit())
        && cell
            .chars()
            .all(|c| c.is_ascii_digit() || c.is_whitespace() || ",.%/:-()$€£+".contains(c))
}

fn columns_with_relation(grid: &[Vec<String>]) -> usize {
    let column_count = grid.iter().map(Vec::len).max().unwrap_or(0);
    (0..column_count)
        .filter(|&column| {
            grid.iter()
                .filter(|row| row.get(column).is_some_and(|cell| filled(cell)))
                .count()
                >= 2
        })
        .count()
}

/// Whether `grid` has the shape of a table.
///
/// A grid is a table when two cells relate by row and by column in one of three ways. Rows
/// relate cells: at least two rows fill two or more cells, and those rows are at least half of
/// all rows. A column of values relates cells: a column holds two or more numbers that are most
/// of its cells, as in a ledger or a receipt where each row carries one value. A block relates
/// cells: three rows in a row fill the same two or more columns, as the table at the end of a
/// page region that also holds lines of prose. A staircase of labels, one per row, has none of
/// these. ~keep
pub(crate) fn has_table_grid(grid: &[Vec<String>]) -> bool {
    columns_with_relation(grid) >= 2 && (rows_relate(grid) || has_value_column(grid) || has_uniform_block(grid))
}

fn rows_relate(grid: &[Vec<String>]) -> bool {
    let rows = grid
        .iter()
        .filter(|row| row.iter().filter(|c| filled(c)).count() >= 2)
        .count();
    rows >= 2 && rows * 2 >= grid.len()
}

fn has_value_column(grid: &[Vec<String>]) -> bool {
    let column_count = grid.iter().map(Vec::len).max().unwrap_or(0);
    (0..column_count).any(|column| {
        let cells: Vec<&String> = grid
            .iter()
            .filter_map(|row| row.get(column))
            .filter(|c| filled(c))
            .collect();
        let values = cells.iter().filter(|c| is_value(c)).count();
        values >= 2 && values * 2 >= cells.len()
    })
}

fn has_uniform_block(grid: &[Vec<String>]) -> bool {
    let mut run = 0;
    let mut previous: Option<Vec<usize>> = None;
    for row in grid {
        let columns: Vec<usize> = row
            .iter()
            .enumerate()
            .filter(|(_, cell)| filled(cell) && !is_prose(cell))
            .map(|(index, _)| index)
            .collect();
        run = if columns.len() >= 2 && previous.as_ref() == Some(&columns) {
            run + 1
        } else if columns.len() >= 2 {
            1
        } else {
            0
        };
        if run >= BLOCK_ROWS {
            return true;
        }
        previous = Some(columns);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(rows: &[&[&str]]) -> Vec<Vec<String>> {
        rows.iter()
            .map(|row| row.iter().map(|cell| (*cell).to_string()).collect())
            .collect()
    }

    #[test]
    fn dense_grid_is_a_table() {
        let table = grid(&[
            &["Item", "Qty", "Price"],
            &["Anvil", "12", "50.00"],
            &["Bucket", "40", "3.20"],
        ]);
        assert!(has_table_grid(&table));
    }

    #[test]
    fn sparse_but_aligned_grid_is_a_table() {
        let table = grid(&[&["Item", "Qty", ""], &["Anvil", "12", "x"], &["Bucket", "", "y"]]);
        assert!(has_table_grid(&table));
    }

    #[test]
    fn one_label_per_row_is_not_a_table() {
        let staircase = grid(&[
            &["Title", ""],
            &["", "Edition"],
            &["Prepared", ""],
            &["", "Office"],
            &["Volume", ""],
            &["", "Year"],
        ]);
        assert!(!has_table_grid(&staircase));
    }

    #[test]
    fn staircase_with_two_word_titles_is_not_a_table() {
        let cover = grid(&[
            &["HARBOR", "CITY"],
            &["Annual", "Review"],
            &["", "Seventh Edition"],
            &["Prepared for the council", ""],
            &["Office of Planning", ""],
            &["Spring Volume Two", ""],
        ]);
        assert!(!has_table_grid(&cover));
    }

    #[test]
    fn labels_split_across_neighbouring_columns_are_not_a_table() {
        let split = grid(&[
            &["Pine Ridge", "", "", ""],
            &["North", "Harbor", "", ""],
            &["", "", "Stone", "Bridge"],
        ]);
        assert!(!has_table_grid(&split));
    }

    #[test]
    fn single_row_and_empty_grids_are_not_tables() {
        assert!(!has_table_grid(&grid(&[&[
            "North Harbor",
            "Pine Ridge",
            "Stone Bridge"
        ]])));
        assert!(!has_table_grid(&[]));
        assert!(!has_table_grid(&grid(&[&["", ""], &["", ""]])));
    }

    #[test]
    fn ledger_with_one_value_per_row_is_a_table() {
        let ledger = grid(&[
            &["Debit", "Credit"],
            &["100", ""],
            &["", "200"],
            &["300", ""],
            &["", "400"],
        ]);
        assert!(has_table_grid(&ledger));
    }

    #[test]
    fn receipt_with_amounts_on_their_own_lines_is_a_table() {
        let receipt = grid(&[
            &["Anvil", ""],
            &["", "1,000"],
            &["SUBTOTAL", ""],
            &["", "11,000"],
            &["TOTAL", ""],
            &["", "11,000"],
        ]);
        assert!(has_table_grid(&receipt));
    }

    #[test]
    fn labels_beside_a_lone_number_are_not_a_table() {
        let staircase = grid(&[&["Title", ""], &["", "Edition"], &["Volume", ""], &["", "2"]]);
        assert!(!has_table_grid(&staircase));
    }

    #[test]
    fn table_after_lines_of_prose_keeps_the_region() {
        let region = grid(&[
            &["Lorem ipsum dolor sit amet", "", ""],
            &["consectetur adipiscing elit sed", "", ""],
            &["eiusmod tempor incididunt ut", "", ""],
            &["labore et dolore magna", "", ""],
            &["Chose", "", "Truc"],
            &["Chose 1", "", "Truc 1"],
            &["Chose 2", "", "Truc 2"],
        ]);
        assert!(!rows_relate(&region));
        assert!(has_table_grid(&region));
    }

    #[test]
    fn column_where_numbers_are_a_minority_is_not_a_table() {
        let staircase = grid(&[
            &["Title", ""],
            &["", "Edition"],
            &["Volume", ""],
            &["", "2024"],
            &["Prepared", ""],
            &["", "2025"],
            &["Office", ""],
            &["", "Council"],
            &["Review", ""],
            &["", "Seventh"],
        ]);
        assert!(columns_with_relation(&staircase) >= 2);
        assert!(!rows_relate(&staircase));
        assert!(!has_table_grid(&staircase));
    }

    #[test]
    fn rows_that_start_with_a_sentence_are_not_a_block() {
        let region = grid(&[
            &["Lorem ipsum dolor sit amet", "", ""],
            &["consectetur adipiscing elit sed", "", ""],
            &["eiusmod tempor incididunt ut", "", ""],
            &["labore et dolore magna", "", ""],
            &["Aenean commodo ligula eget", "", "Note"],
            &["Donec quam felis ultricies", "", "Memo"],
            &["Nulla consequat massa quis", "", "Item"],
        ]);
        assert!(columns_with_relation(&region) >= 2);
        assert!(!rows_relate(&region));
        assert!(!has_table_grid(&region));
    }

    #[test]
    fn one_related_row_does_not_relate_rows() {
        assert!(!rows_relate(&grid(&[&["North Harbor", "Pine Ridge"]])));
        assert!(!rows_relate(&grid(&[
            &["North Harbor", "Pine Ridge"],
            &["Stone Bridge", ""]
        ])));
        assert!(rows_relate(&grid(&[&["Item", "Qty"], &["Anvil", "12"]])));
    }

    #[test]
    fn two_aligned_rows_among_prose_are_not_a_block() {
        let region = grid(&[
            &["Lorem ipsum dolor sit amet", "", ""],
            &["Chose", "", "Truc"],
            &["Chose 1", "", "Truc 1"],
            &["consectetur adipiscing elit sed", "", ""],
            &["Aenean commodo ligula eget", "", ""],
            &["Donec quam felis ultricies", "", ""],
        ]);
        assert!(!has_table_grid(&region));
    }
}
