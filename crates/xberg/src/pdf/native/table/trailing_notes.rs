use crate::pdf::table_reconstruct::HocrWord;

const NOTE_MARKERS: [&str; 3] = ["(1)", "(2)", "(3)"];
const MIN_NOTE_WORDS: usize = 8;
const SOURCE_ALIGNMENT_TOLERANCE: u32 = 2;

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

// The grid supplies the proposed suffix. Source spacing must independently
// confirm hanging-indent paragraphs without evidence of a value track. ~keep
pub(super) fn certified_prefix(region: &[HocrWord], cells: &[Vec<String>]) -> Option<(usize, Vec<HocrWord>)> {
    let start = note_start(cells)?;
    let rows = super::numeric_rows(region, (super::median_region_word_height(region) / 2).max(3));
    let cutoff = source_note_cutoff(rows)?;
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

fn source_note_cutoff(mut rows: Vec<Vec<&HocrWord>>) -> Option<u32> {
    for row in &mut rows {
        row.sort_by_key(|word| word.left);
    }
    let start = rows.iter().position(|row| row[0].text == NOTE_MARKERS[0])?;
    let suffix = &rows[start..];
    let [first, ..] = suffix else { return None };
    if suffix.len() != NOTE_MARKERS.len() * 2 {
        return None;
    }
    let margin = first[0].left;
    let height = first[0].height;
    let indent = suffix[1][0].left.checked_sub(margin)?;
    if height == 0 || indent < height / 2 || indent > height.saturating_mul(2) {
        return None;
    }
    let cutoff = first.iter().map(|word| word.top).min()?;
    let legend_bottom = rows
        .get(start.checked_sub(1)?)?
        .iter()
        .map(|word| word.top.saturating_add(word.height))
        .max()?;
    let preceding_gap = cutoff.checked_sub(legend_bottom)?;
    let mut numeric_ends = Vec::new();
    for end in suffix.iter().filter_map(|row| row.last()) {
        if !crate::table_core::text_is_value(&end.text) {
            continue;
        }
        if numeric_ends
            .iter()
            .any(|&edge| end.left.abs_diff(edge) <= SOURCE_ALIGNMENT_TOLERANCE)
        {
            return None;
        }
        numeric_ends.push(end.left);
    }
    for ([opening, continuation], marker) in suffix.as_chunks::<2>().0.iter().zip(NOTE_MARKERS) {
        let opening_bottom = opening.iter().map(|word| word.top.saturating_add(word.height)).max()?;
        let continuation_top = continuation.iter().map(|word| word.top).min()?;
        if opening[0].text != marker
            || opening[0].left.abs_diff(margin) > SOURCE_ALIGNMENT_TOLERANCE
            || continuation[0].left.abs_diff(margin + indent) > SOURCE_ALIGNMENT_TOLERANCE
            || continuation_top.checked_sub(opening_bottom)? >= preceding_gap
            || !continuous_line(opening)
            || !continuous_line(continuation)
        {
            return None;
        }
    }
    Some(cutoff)
}

fn continuous_line(row: &[&HocrWord]) -> bool {
    row.len() >= MIN_NOTE_WORDS
        && row.windows(2).all(|pair| {
            let [left, right] = pair else { return false };
            right.left.saturating_sub(left.left.saturating_add(left.width)) <= left.height.max(right.height)
        })
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
        for marker in ["(1)", "", "(2)", "", "(3)", ""] {
            cells.push(vec![format!("{marker} {label}").trim().to_owned(), String::new()]);
        }
        let source_words = |cells: &[Vec<String>]| -> Vec<HocrWord> {
            cells
                .iter()
                .zip([0, 16, 32, 48, 57, 72, 81, 96, 105])
                .flat_map(|(row, top)| {
                    row.iter()
                        .zip([if [57, 81, 105].contains(&top) { 18 } else { 10 }, 450])
                        .flat_map(move |(cell, left)| {
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
    #[test]
    fn source_geometry_rejects_an_aligned_status_and_a_merged_numeric_value() {
        let word = |text: &str, left, top| HocrWord {
            text: text.into(),
            left,
            top,
            width: 6,
            height: 8,
            confidence: 100.0,
        };
        let mut rows = vec![vec![word("N.M. = not meaningful", 10, 0)]];
        for (marker, top) in NOTE_MARKERS.into_iter().zip([16, 40, 64]) {
            let mut opening = vec![word(marker, 10, top)];
            opening.extend((1..9).map(|index| word("text", 10 + index * 8, top)));
            rows.push(opening);
            rows.push((0..9).map(|index| word("text", 18 + index * 8, top + 9)).collect());
        }
        let cells: Vec<Vec<String>> = rows
            .iter()
            .map(|row| {
                vec![
                    row.iter().map(|word| word.text.as_str()).collect::<Vec<_>>().join(" "),
                    String::new(),
                ]
            })
            .collect();
        let flatten = |rows: &[Vec<HocrWord>]| rows.iter().flatten().cloned().collect::<Vec<_>>();
        assert_eq!(
            certified_prefix(&flatten(&rows), &cells).map(|(start, _)| start),
            Some(1)
        );
        for (value, column, first_line) in [("Pending", 1, 1), ("300.00", 0, 1), ("300.00", 0, 2)] {
            let mut records = rows.clone();
            let mut grid = cells.clone();
            for (row, cells) in records.iter_mut().zip(&mut grid).skip(first_line).step_by(2) {
                let top = row[0].top;
                row.push(word(
                    value,
                    if column == 0 { 76 + 8 * first_line as u32 } else { 108 },
                    top,
                ));
                if column == 0 {
                    cells[column].push(' ');
                }
                cells[column].push_str(value);
            }
            if column == 0 {
                let (reconstructed, _) = crate::table_core::reconstruct_table_with_columns(&flatten(&records), 20, 0.5);
                assert_eq!(reconstructed.len(), grid.len());
                assert!(
                    reconstructed
                        .iter()
                        .skip(first_line)
                        .step_by(2)
                        .all(|row| row[0].ends_with(value))
                );
            }
            assert_eq!(note_start(&grid), Some(1), "{value} passes the cell-only predicate");
            assert!(
                certified_prefix(&flatten(&records), &grid).is_none(),
                "{value} on line {first_line}"
            );
        }
    }
}
