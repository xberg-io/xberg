//! Title lines printed above a borderless table (xberg-io/xberg#3445).
//!
//! A table's name is set on one or two centred lines over the table, in the same type as the
//! body. Left in the region they are read as table rows: the line nearest the columns joins
//! the header band (its words land in the header cells) and the other becomes a data row.
//! Lines centred over the columns, spanning most of the table's width and set off from the
//! first column header by more than the table's own row pitch, are the table's title and go
//! back to the page as prose. A region with ruling lines is a ruled table whose spanning
//! cells are part of it, so it is left as it is.

use crate::pdf::table_reconstruct::HocrWord;

use super::titled_sections::{is_title_words, row_runs, split_titled_region};

/// Most lines a title may span.
const MAX_TITLE_ROWS: usize = 3;

/// Fewest rows the table below the title must keep: a header and three body rows.
const MIN_TABLE_ROWS: usize = 4;

/// Fewest column runs a row needs to be a header band.
const MIN_HEADER_RUNS: usize = 2;

/// Rows below the title in which the header band is looked for: a spanning header may sit
/// over it.
const HEADER_SEARCH_ROWS: usize = 2;

/// Farthest, in median word heights, the centre of a title line may sit from the centre of
/// the table below it. Two heights is a word's width of slack for the uneven word spacing of
/// a centred line, while a left- or right-aligned caption sits many heights away.
const CENTRE_TOLERANCE_HEIGHTS: u32 = 2;

/// Least share of the table's width, in percent, each title line spans. A name is set across
/// the table; a few words over its first columns are a header label, and half the width is
/// the line between the two.
const MIN_TITLE_WIDTH_PERCENT: u32 = 50;

/// Smallest gap between the last title line and the header, as a percentage of the table's
/// median row pitch. A title sits visibly apart: at one pitch it is just another row, and a
/// quarter more is the smallest separation the pages checked (a 30 pt gap over 19 pt rows) and
/// the rows they must not split (tightly stacked header lines) tell apart.
const MIN_GAP_PITCH_PERCENT: u32 = 125;

/// Gaps at most this many word heights wide lie inside one text run: a space of justified
/// text is under one height, a column gutter several.
const RUN_GAP_HEIGHTS: u32 = 2;

/// Smallest row-grouping tolerance, in the units of the words, so that a region of tiny words
/// does not split one line into several rows.
const MIN_ROW_TOLERANCE: u32 = 3;

/// The region without the words of its leading title lines, or `None` when it has none.
///
/// A region that [`split_titled_region`] splits into titled tables keeps its titles for that
/// path. Pass `ruled` for a region with ruling lines: it is never trimmed.
pub(super) fn without_leading_titles(region: &[HocrWord], page_height: f32, ruled: bool) -> Option<Vec<HocrWord>> {
    if ruled {
        return None;
    }
    let median_height = super::median_region_word_height(region).max(1);
    let rows = super::numeric_rows(region, (median_height / 2).max(MIN_ROW_TOLERANCE));
    if rows.len() < MIN_TABLE_ROWS + 1 {
        return None;
    }
    let gap_limit = median_height.saturating_mul(RUN_GAP_HEIGHTS).max(MIN_ROW_TOLERANCE);
    let mut title_rows = rows
        .iter()
        .take(MAX_TITLE_ROWS)
        .take_while(|row| is_title_words(row) && row_runs(row, gap_limit).len() == 1)
        .count();
    // A last line that lies within the extent of the row below it is a spanning header
    // ("Fiscal Year 2024" over its columns), part of the table, not a title.
    while title_rows > 0 && lies_within(&rows[title_rows - 1], &rows[title_rows]) {
        title_rows -= 1;
    }
    if title_rows == 0 || rows.len() - title_rows < MIN_TABLE_ROWS {
        return None;
    }
    let (titles, table) = rows.split_at(title_rows);
    if !table
        .iter()
        .take(HEADER_SEARCH_ROWS)
        .any(|row| row_runs(row, gap_limit).len() >= MIN_HEADER_RUNS)
        || !table
            .iter()
            .any(|row| row.iter().filter(|word| super::is_numeric_word(&word.text)).count() >= 2)
    {
        return None;
    }
    let (left, right) = extent(table.iter().flatten().copied())?;
    let table_centre = (left + right) / 2;
    let table_width = right.saturating_sub(left);
    let centred = titles.iter().all(|row| {
        extent(row.iter().copied()).is_some_and(|(start, end)| {
            ((start + end) / 2).abs_diff(table_centre) <= median_height.saturating_mul(CENTRE_TOLERANCE_HEIGHTS)
                && end.saturating_sub(start).saturating_mul(100) >= table_width.saturating_mul(MIN_TITLE_WIDTH_PERCENT)
        })
    });
    if !centred || !title_stands_apart(titles, table) || split_titled_region(region, page_height).is_some() {
        return None;
    }
    let cutoff = titles.iter().flatten().map(|word| word.top + word.height / 2).max()?;
    Some(
        region
            .iter()
            .filter(|word| word.top + word.height / 2 > cutoff)
            .cloned()
            .collect(),
    )
}

/// Whether the words of `line` all lie between the leftmost and rightmost edge of `other`.
fn lies_within(line: &[&HocrWord], other: &[&HocrWord]) -> bool {
    match (extent(line.iter().copied()), extent(other.iter().copied())) {
        (Some((start, end)), Some((left, right))) => start >= left && end <= right,
        _ => false,
    }
}

/// The leftmost and rightmost edge of `words`.
fn extent<'a>(words: impl Iterator<Item = &'a HocrWord> + Clone) -> Option<(u32, u32)> {
    Some((
        words.clone().map(|word| word.left).min()?,
        words.map(HocrWord::right).max()?,
    ))
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
        let rest = without_leading_titles(&log_table(), 792.0, false).expect("both title lines are found");
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
        assert!(without_leading_titles(&words, 792.0, false).is_none());
    }

    #[test]
    fn a_few_words_over_the_first_columns_are_a_header_label_not_a_title() {
        let mut words = log_table();
        words.retain(|word| word.top >= 140);
        words.push(word("Common", 150, 127, 40));
        words.push(word("label", 194, 127, 30));
        assert!(without_leading_titles(&words, 792.0, false).is_none());
    }

    #[test]
    fn a_left_aligned_line_above_the_table_is_not_a_title() {
        let mut words = log_table();
        for word in words.iter_mut().filter(|word| word.top < 140) {
            word.left -= 60;
        }
        assert!(without_leading_titles(&words, 792.0, false).is_none());
    }

    #[test]
    fn a_ruled_region_is_never_trimmed() {
        assert!(without_leading_titles(&log_table(), 792.0, true).is_none());
    }

    #[test]
    fn a_spanning_header_over_its_own_columns_stays_in_the_table() {
        let mut words = log_table();
        words.retain(|word| word.top >= 150);
        // Centred over the whole header row, which it spans, one pitch-and-a-half above it.
        words.push(word("Fiscal", 150, 120, 36));
        words.push(word("Year", 190, 120, 28));
        words.push(word("Tables", 222, 120, 40));
        assert!(without_leading_titles(&words, 792.0, false).is_none());
    }

    #[test]
    fn a_three_line_centred_title_leaves_the_table_region() {
        let mut words = log_table();
        words.push(word("Natural", 150, 89, 40));
        words.push(word("Logarithms", 194, 89, 60));
        words.push(word("Table", 258, 89, 40));
        let rest = without_leading_titles(&words, 792.0, false).expect("three lines are found");
        assert_eq!(rest.len(), log_table().len() - 13);
    }

    #[test]
    fn a_wide_title_over_a_spanning_header_keeps_only_the_header_in_the_table() {
        let mut words = log_table();
        words.retain(|word| word.top >= 150);
        for (text, left, top, width) in [
            ("Annual", 63, 89, 55),
            ("Financial", 122, 89, 75),
            ("Results", 201, 89, 65),
            ("Summary", 270, 89, 99),
        ] {
            words.push(word(text, left, top, width));
        }
        words.push(word("Fiscal", 150, 120, 36));
        words.push(word("Year", 190, 120, 28));
        words.push(word("Tables", 222, 120, 40));
        let rest = without_leading_titles(&words, 792.0, false).expect("the wide title is found");
        let texts: Vec<&str> = rest.iter().map(|word| word.text.as_str()).collect();
        assert!(!texts.contains(&"Annual") && !texts.contains(&"Summary"), "{texts:?}");
        assert!(texts.contains(&"Fiscal") && texts.contains(&"Tables"), "{texts:?}");
    }
}
