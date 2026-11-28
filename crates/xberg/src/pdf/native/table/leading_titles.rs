//! Title lines printed above a borderless table (xberg-io/xberg#3445).
//!
//! A table's name is set on one or two centred lines over the table, in the same type as the
//! body. Left in the region they are read as table rows: the line nearest the columns joins
//! the header band (its words land in the header cells) and the other becomes a data row.
//! Lines that stand apart from the columns, centred over them and a gap above the first
//! column header wider than the table's own row pitch, are the table's title and go back to
//! the page as prose.

use crate::pdf::table_reconstruct::HocrWord;

use super::titled_sections::{is_title_words, row_runs, split_titled_region};

/// Most lines a title may span.
const MAX_TITLE_ROWS: usize = 3;

/// Fewest rows the table below the title must keep.
const MIN_TABLE_ROWS: usize = 4;

/// Fewest column runs the table's first row needs to be a header band.
const MIN_HEADER_RUNS: usize = 2;

/// Farthest, in median word heights, the centre of a title line may sit from the centre of
/// the table below it.
const CENTRE_TOLERANCE_HEIGHTS: u32 = 2;

/// Least share of the table's width, in percent, each title line spans: a name is set across the
/// table, where a few words over its first columns are a header label.
const MIN_TITLE_WIDTH_PERCENT: u32 = 50;

/// Smallest gap between the last title line and the header, as a percentage of the table's
/// median row pitch.
const MIN_GAP_PITCH_PERCENT: u32 = 125;

/// Gaps at most this many word heights wide lie inside one text run.
const RUN_GAP_HEIGHTS: u32 = 2;

/// The region without the words of its leading title lines, or `None` when it has none.
///
/// A region that [`split_titled_region`] splits into titled tables keeps its titles for that
/// path.
pub(super) fn without_leading_titles(region: &[HocrWord], page_height: f32) -> Option<Vec<HocrWord>> {
    let median_height = super::median_region_word_height(region).max(1);
    let rows = super::numeric_rows(region, (median_height / 2).max(3));
    if rows.len() < MIN_TABLE_ROWS + 1 {
        return None;
    }
    let gap_limit = median_height.saturating_mul(RUN_GAP_HEIGHTS).max(3);
    let title_rows = rows
        .iter()
        .take(MAX_TITLE_ROWS)
        .take_while(|row| is_title_words(row) && row_runs(row, gap_limit).len() == 1)
        .count();
    if title_rows == 0 || rows.len() - title_rows < MIN_TABLE_ROWS {
        return None;
    }
    let (titles, table) = rows.split_at(title_rows);
    if row_runs(&table[0], gap_limit).len() < MIN_HEADER_RUNS
        || !table
            .iter()
            .any(|row| row.iter().filter(|word| super::is_numeric_word(&word.text)).count() >= 2)
    {
        return None;
    }
    let left = table.iter().flatten().map(|word| word.left).min()?;
    let right = table.iter().flatten().map(|word| word.right()).max()?;
    let table_centre = (left + right) / 2;
    let table_width = right.saturating_sub(left);
    let centred = titles.iter().all(|row| {
        let (start, end) = (
            row.iter().map(|word| word.left).min().unwrap_or(0),
            row.iter().map(|word| word.right()).max().unwrap_or(0),
        );
        ((start + end) / 2).abs_diff(table_centre) <= median_height.saturating_mul(CENTRE_TOLERANCE_HEIGHTS)
            && end.saturating_sub(start).saturating_mul(100) >= table_width.saturating_mul(MIN_TITLE_WIDTH_PERCENT)
    });
    if !centred || !title_stands_apart(titles, table) || split_titled_region(region, page_height).is_some() {
        return None;
    }
    let titled: Vec<*const HocrWord> = titles.iter().flatten().map(|word| std::ptr::from_ref(*word)).collect();
    Some(
        region
            .iter()
            .filter(|word| !titled.contains(&std::ptr::from_ref(*word)))
            .cloned()
            .collect(),
    )
}

/// Whether the gap from the last title line to the header is wider than the table's row pitch.
fn title_stands_apart(titles: &[Vec<&HocrWord>], table: &[Vec<&HocrWord>]) -> bool {
    let centre = |row: &[&HocrWord]| {
        let sum: u64 = row.iter().map(|word| u64::from(word.top + word.height / 2)).sum();
        (sum / row.len().max(1) as u64) as u32
    };
    let mut pitches: Vec<u32> = table
        .windows(2)
        .map(|pair| centre(&pair[1]).saturating_sub(centre(&pair[0])))
        .collect();
    pitches.sort_unstable();
    let Some(pitch) = pitches.get(pitches.len() / 2).copied().filter(|pitch| *pitch > 0) else {
        return false;
    };
    let Some(last) = titles.last() else { return false };
    let gap = centre(&table[0]).saturating_sub(centre(last));
    gap.saturating_mul(100) >= pitch.saturating_mul(MIN_GAP_PITCH_PERCENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(text: &str, left: u32, top: u32, width: u32) -> HocrWord {
        HocrWord {
            text: text.to_string(),
            left,
            top,
            width,
            height: 12,
            confidence: 100.0,
        }
    }

    /// The log table of gutenberg-33283 p155 (points, top down): two centred title lines, a
    /// 31 pt gap, a header of four columns and body rows 19 pt apart.
    fn log_table() -> Vec<HocrWord> {
        let mut words = vec![
            word("A", 85, 108, 10),
            word("Useful", 99, 108, 42),
            word("Table", 145, 108, 35),
            word("of", 185, 108, 14),
            word("\u{201c}Naperian", 203, 108, 63),
            word("Logarithms\u{201d}", 270, 108, 77),
            word("(Also", 63, 127, 27),
            word("called", 94, 127, 29),
            word("Natural", 127, 127, 39),
            word("Logarithms", 170, 127, 58),
            word("or", 232, 127, 10),
            word("Hyperbolic", 247, 127, 56),
            word("Logarithms)", 306, 127, 63),
            word("Number", 107, 158, 35),
            word("log", 161, 157, 15),
            word("Number", 255, 158, 35),
            word("log", 292, 157, 15),
        ];
        for (index, (a, b, c, d)) in [("1", "0.0000", "6", "1.7918"), ("1.1", "0.0953", "7", "1.9459")]
            .into_iter()
            .chain([("1.2", "0.1823", "8", "2.0794"), ("1.5", "0.4055", "9", "2.1972")])
            .enumerate()
        {
            let top = 178 + 19 * index as u32;
            words.push(word(a, 111, top, 12));
            words.push(word(b, 156, top, 33));
            words.push(word(c, 261, top, 6));
            words.push(word(d, 292, top, 33));
        }
        words
    }

    #[test]
    fn centred_title_lines_above_the_header_leave_the_table_region() {
        let rest = without_leading_titles(&log_table(), 792.0).expect("both title lines are found");
        let texts: Vec<&str> = rest.iter().map(|word| word.text.as_str()).collect();
        assert_eq!(rest.len(), log_table().len() - 13);
        assert!(
            !texts.contains(&"Useful") && !texts.contains(&"Hyperbolic"),
            "{texts:?}"
        );
        assert_eq!(texts.iter().filter(|text| **text == "Number").count(), 2);
    }

    #[test]
    fn a_header_row_as_close_as_the_body_rows_is_not_a_title() {
        let mut words = log_table();
        for word in words.iter_mut().filter(|word| word.top >= 157 && word.top < 170) {
            word.top -= 20;
        }
        assert!(without_leading_titles(&words, 792.0).is_none());
    }

    #[test]
    fn a_few_words_over_the_first_columns_are_a_header_label_not_a_title() {
        let mut words = log_table();
        words.retain(|word| word.top >= 140);
        words.push(word("Common", 150, 127, 40));
        words.push(word("label", 194, 127, 30));
        assert!(without_leading_titles(&words, 792.0).is_none());
    }

    #[test]
    fn a_left_aligned_line_above_the_table_is_not_a_title() {
        let mut words = log_table();
        for word in words.iter_mut().filter(|word| word.top < 140) {
            word.left -= 60;
        }
        assert!(without_leading_titles(&words, 792.0).is_none());
    }
}
