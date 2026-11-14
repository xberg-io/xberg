use crate::pdf::table_reconstruct::HocrWord;

const NOTE_MARKERS: [&str; 3] = ["(1)", "(2)", "(3)"];
const MIN_NOTE_WORDS: usize = 8;

fn marker(row: &[String]) -> Option<&str> {
    row.first()?.split_whitespace().next()
}

fn note_start(cells: &[Vec<String>]) -> Option<usize> {
    let start = cells.iter().position(|row| marker(row) == Some(NOTE_MARKERS[0]))?;
    let legend = cells.get(start.checked_sub(1)?)?;
    if !legend.first()?.split_whitespace().any(|word| word == "=")
        || legend.iter().skip(1).any(|cell| !cell.trim().is_empty())
    {
        return None;
    }
    let tail = &cells[start..];
    if tail.iter().any(|row| {
        row.iter()
            .skip(1)
            .any(|cell| crate::table_core::is_cell_value_text(cell))
            || row
                .iter()
                .flat_map(|cell| cell.split_whitespace())
                .filter(|word| word.chars().any(char::is_alphabetic))
                .count()
                < MIN_NOTE_WORDS
    }) {
        return None;
    }
    tail.iter()
        .filter_map(|row| marker(row).filter(|word| word.starts_with('(')))
        .eq(NOTE_MARKERS)
        .then_some(start)
}

// A standalone value on a reconstructed column track can belong to a record even
// when the label reads like prose. Keep the original grid when ownership is uncertain. ~keep
pub(super) fn certified_prefix(region: &[HocrWord], cells: &[Vec<String>]) -> Option<(usize, Vec<HocrWord>)> {
    let start = note_start(cells)?;
    let rows = super::numeric_rows(region, (super::median_region_word_height(region) / 2).max(3));
    let mut marker_rows = rows.iter().filter(|row| {
        row.len() >= MIN_NOTE_WORDS
            && row
                .iter()
                .min_by_key(|word| word.left)
                .is_some_and(|word| word.text.trim() == NOTE_MARKERS[0])
    });
    let first = marker_rows.next()?;
    if marker_rows.next().is_some() {
        return None;
    }
    let cutoff = first.iter().map(|word| word.top).min()?;
    let mut prefix = Vec::new();
    for word in region {
        if word.top < cutoff {
            if word.top.checked_add(word.height)? > cutoff {
                return None;
            }
            prefix.push(word.clone());
        }
    }
    Some((start, prefix))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_numbered_prose_respects_legend_order_and_rejects_records() {
        let rows = |texts: &[&str]| {
            texts
                .iter()
                .map(|text| vec![(*text).to_owned(), String::new()])
                .collect::<Vec<_>>()
        };
        let note = "These rates include the corresponding expenses and other recurring adjustments";
        let mut cells = rows(&["Account", "N.M. = not meaningful"]);
        for marker in NOTE_MARKERS {
            cells.push(vec![format!("{marker} {note}"), String::new()]);
        }
        assert_eq!(note_start(&cells), Some(2));
        let mut no_legend = cells.clone();
        no_legend[1][0] = "Operating Revenue".into();
        assert_eq!(note_start(&no_legend), None);
        let mut reversed = cells.clone();
        reversed.swap(3, 4);
        assert_eq!(note_start(&reversed), None);
        cells.push(vec!["Expenses".into(), "120.0".into()]);
        assert_eq!(note_start(&cells), None);
    }
    #[test]
    fn certified_prefix_preserves_long_records_on_the_numeric_track() {
        let label = "These rates include the corresponding expenses and other recurring adjustments";
        let mut cells = vec![
            vec!["Revenue".into(), "100.00".into()],
            vec!["Expenses".into(), "200.00".into()],
            vec!["N.M. = not meaningful".into(), String::new()],
        ];
        for marker in ["(1)", "", "(2)", "(3)"] {
            cells.push(vec![format!("{marker} {label}").trim().to_owned(), String::new()]);
        }
        let source_words = |cells: &[Vec<String>]| -> Vec<HocrWord> {
            cells
                .iter()
                .zip((0..).step_by(16))
                .flat_map(|(row, top)| {
                    row.iter().zip([10, 450]).flat_map(move |(cell, left)| {
                        cell.split_whitespace()
                            .zip((left..).step_by(35))
                            .map(move |(text, left)| HocrWord {
                                text: text.to_owned(),
                                left,
                                top,
                                width: 31,
                                height: 8,
                                confidence: 100.0,
                            })
                    })
                })
                .collect()
        };
        assert_eq!(
            certified_prefix(&source_words(&cells), &cells).map(|(start, _)| start),
            Some(3),
        );
        for row_index in [3, 4] {
            let mut records = cells.clone();
            records[row_index][1] = "300.00".into();
            let words = source_words(&records);
            let (grid, tracks) = crate::table_core::reconstruct_table_with_columns(&words, 20, 0.5);
            assert_eq!(tracks, [10, 450]);
            assert_eq!(grid, records);
            assert!(certified_prefix(&words, &grid).is_none(), "row {row_index}");
        }
    }
}
