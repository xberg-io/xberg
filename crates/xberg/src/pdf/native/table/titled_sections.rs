//! Section titles that start their own table (xberg-io/xberg#3348).
//!
//! Several separately titled tables printed on one page reach the extractors as a
//! single table: a ruled detector joins them because its rules are continuous,
//! and the borderless path sees one slab of aligned columns. The title of each
//! table then survives only as a full-width row inside the merged table, and the
//! repeated header band that follows it as ordinary body rows.
//!
//! A full-width row alone is not evidence of a table boundary: subtotals, notes
//! and other spanning rows sit inside bodies. A section starts only when a
//! full-width row is the table's own title *and* a fresh header band follows it.

use xberg_native_pdf::geometry::Rect;
use xberg_native_pdf::structure::table_extractor::{Table as NativeTable, TableRow};

use crate::pdf::table_reconstruct::HocrWord;
use crate::types::{BoundingBox, Table};

/// A title row must occupy at least this much of the table's width. A ruled
/// detector represents a spanning row as one cell whose colspan covers the
/// remaining columns; anything narrower is a cell, not a title.
const TITLE_MIN_WIDTH_PERCENT: usize = 90;

/// Longest text a title may carry. Longer spanning rows are notes or
/// statements about the table, not its name.
const TITLE_MAX_CHARS: usize = 64;

/// Most words a title may carry, for the same reason.
const TITLE_MAX_WORDS: usize = 12;

/// A title has to read as words rather than as a value.
const TITLE_MIN_ALPHA_PERCENT: usize = 50;

/// A header band needs at least two cells to be a column header rather than a
/// repeated spanning row.
const HEADER_MIN_CELLS: usize = 2;

/// A header band must cover at least this much of the table's width.
const HEADER_MIN_WIDTH_PERCENT: usize = 90;

/// Fraction of a header band's cells that must be labels (not numeric values).
const HEADER_MIN_LABEL_PERCENT: usize = 60;

/// Split a ruled table whose rows stack several separately titled tables.
///
/// Returns the input unchanged when no row carries both a title and a following
/// header band, so a spanning body row or a repeated header band on its own
/// never cuts a table in two.
pub(super) fn split_ruled_sections(table: NativeTable) -> Vec<NativeTable> {
    let starts = section_starts(&table);
    if starts.is_empty() {
        return vec![table];
    }
    let mut boundaries = starts;
    if boundaries[0] > 0 {
        boundaries.insert(0, 0);
    }
    let mut sections = Vec::with_capacity(boundaries.len());
    for (index, start) in boundaries.iter().enumerate() {
        let end = boundaries.get(index + 1).copied().unwrap_or(table.rows.len());
        let rows = table.rows[*start..end].to_vec();
        sections.push(NativeTable {
            has_header: has_spanning_title(&rows),
            rows,
            col_count: table.col_count,
            bbox: section_bounds(&table, *start, end),
        });
    }
    sections
}

/// Row indices where a titled section begins.
fn section_starts(table: &NativeTable) -> Vec<usize> {
    let mut starts = Vec::new();
    let mut index = 1;
    while index < table.rows.len() {
        if !is_title_row(&table.rows[index], table.col_count) {
            index += 1;
            continue;
        }
        let mut band_end = index;
        while band_end + 1 < table.rows.len() && is_full_width_row(&table.rows[band_end + 1], table.col_count) {
            band_end += 1;
        }
        // Consecutive spanning rows above a header band are one title band: a
        // table may print its name over several lines (`DW, MH` under the name
        // in xberg-io/xberg#3348's first record). Only the header band after the
        // whole band proves the rows below are a new table.
        if table
            .rows
            .get(band_end + 1)
            .is_some_and(|row| is_header_band(row, table.col_count))
        {
            starts.push(index);
            index = band_end + 1;
        } else {
            index += 1;
        }
    }
    starts
}

fn is_full_width_row(row: &TableRow, col_count: usize) -> bool {
    row.cells.len() == 1 && covers_width(row.cells[0].colspan as usize, col_count, TITLE_MIN_WIDTH_PERCENT)
}

fn is_title_row(row: &TableRow, col_count: usize) -> bool {
    if !is_full_width_row(row, col_count) {
        return false;
    }
    is_title_text(&row.cells[0].text)
}

fn is_title_text(text: &str) -> bool {
    let text = text.trim();
    if text.is_empty() || text.chars().count() > TITLE_MAX_CHARS {
        return false;
    }
    let words = text.split_whitespace().count();
    if !(2..=TITLE_MAX_WORDS).contains(&words) {
        return false;
    }
    let visible = text.chars().filter(|character| !character.is_whitespace()).count();
    let alphabetic = text.chars().filter(|character| character.is_alphabetic()).count();
    alphabetic > 0 && alphabetic * 100 >= visible * TITLE_MIN_ALPHA_PERCENT
}

fn is_header_band(row: &TableRow, col_count: usize) -> bool {
    if row.cells.len() < HEADER_MIN_CELLS {
        return false;
    }
    let covered: usize = row.cells.iter().map(|cell| cell.colspan as usize).sum();
    if !covers_width(covered, col_count, HEADER_MIN_WIDTH_PERCENT) {
        return false;
    }
    let populated: Vec<&str> = row
        .cells
        .iter()
        .map(|cell| cell.text.trim())
        .filter(|text| !text.is_empty())
        .collect();
    if populated.len() < HEADER_MIN_CELLS {
        return false;
    }
    let labels = populated
        .iter()
        .filter(|text| !super::is_numeric_word(text) && text.chars().any(char::is_alphabetic))
        .count();
    labels * 100 >= populated.len() * HEADER_MIN_LABEL_PERCENT
}

fn covers_width(span: usize, col_count: usize, percent: usize) -> bool {
    col_count > 0 && span * 100 >= col_count * percent
}

fn has_spanning_title(rows: &[TableRow]) -> bool {
    rows.first()
        .is_some_and(|row| row.cells.len() == 1 && row.cells[0].colspan > 1)
}

fn section_bounds(table: &NativeTable, start: usize, end: usize) -> Option<Rect> {
    let mut bounds: Option<(f32, f32, f32, f32)> = None;
    for cell in table.rows[start..end]
        .iter()
        .flat_map(|row| row.cells.iter())
        .filter_map(|cell| cell.bbox)
    {
        let (x0, y0, x1, y1) = (cell.x, cell.y, cell.x + cell.width, cell.y + cell.height);
        bounds = Some(match bounds {
            None => (x0, y0, x1, y1),
            Some((left, top, right, bottom)) => (left.min(x0), top.min(y0), right.max(x1), bottom.max(y1)),
        });
    }
    bounds
        .map(|(left, top, right, bottom)| Rect::from_points(left, top, right, bottom))
        .or(table.bbox)
}

/// One titled section of a borderless region: its own title rows and the words
/// that belong to it.
pub(super) struct RegionSection {
    pub(super) titles: Vec<String>,
    pub(super) words: Vec<HocrWord>,
    pub(super) bounds: Option<BoundingBox>,
}

/// Most runs a title row may have. A header band has more.
const MAX_TITLE_RUNS: usize = 3;

/// Fewest runs a row needs before it can be a column header.
const MIN_HEADER_RUNS: usize = 4;

/// Rows a title may look ahead for the header band that proves it titles a table.
const HEADER_LOOKAHEAD_ROWS: usize = 3;

/// Rows a header band may look ahead for its first numeric body row.
const BODY_LOOKAHEAD_ROWS: usize = 4;

/// A section needs at least this many non-title rows to be a table.
const MIN_SECTION_ROWS: usize = 3;

/// A block split off a side-by-side band needs at least this many columns.
const MIN_BLOCK_COLUMNS: usize = 2;

/// A side-by-side band's rows must carry at least this many runs per block;
/// anything sparser is prose printed beneath the band, not table rows.
const MIN_GRID_RUNS_PER_BLOCK: usize = 3;

/// Gaps at most this many word heights wide are the spaces inside one text run,
/// not a column boundary.
const RUN_GAP_HEIGHT_FACTOR: u32 = 2;

/// Split a borderless region that stacks or prints side by side several tables,
/// each opening with its own title row followed by a header band.
///
/// Returns `None` unless every section is a full table in its own right, so a
/// spanning body row, a repeated header band, or a lone caption never cuts a
/// table. The sections are returned without their title words; the caller
/// reconstructs each table and prepends [`RegionSection::titles`].
pub(super) fn split_titled_region(region: &[HocrWord], page_height: f32) -> Option<Vec<RegionSection>> {
    if region.is_empty() {
        return None;
    }
    let median_height = super::median_region_word_height(region).max(1);
    let row_tolerance = (median_height / 2).max(3);
    let gap_limit = median_height.saturating_mul(RUN_GAP_HEIGHT_FACTOR).max(3);
    let rows = super::numeric_rows(region, row_tolerance);
    let groups = title_groups(&rows, gap_limit);
    match groups.as_slice() {
        [] => None,
        [group] if group.runs.len() >= 2 => horizontal_sections(&rows, group, page_height, gap_limit),
        _ if groups.len() >= 2 => vertical_sections(&rows, &groups, page_height),
        _ => None,
    }
}

struct TitleGroup {
    start: usize,
    end: usize,
    runs: Vec<(u32, u32)>,
}

fn title_groups(rows: &[Vec<&HocrWord>], gap_limit: u32) -> Vec<TitleGroup> {
    let mut groups: Vec<TitleGroup> = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        let Some(runs) = title_runs(row, rows, index, gap_limit) else {
            continue;
        };
        match groups.last_mut() {
            Some(group) if group.end == index => {
                // A band may print its name over several lines; the runs come
                // from the first line, which is the widest.
                group.end = index + 1;
            }
            _ => groups.push(TitleGroup {
                start: index,
                end: index + 1,
                runs,
            }),
        }
    }
    groups
}

fn title_runs(row: &[&HocrWord], rows: &[Vec<&HocrWord>], index: usize, gap_limit: u32) -> Option<Vec<(u32, u32)>> {
    if !is_title_words(row) {
        return None;
    }
    let runs = row_runs(row, gap_limit);
    if runs.is_empty() || runs.len() > MAX_TITLE_RUNS {
        return None;
    }
    let header = rows
        .iter()
        .enumerate()
        .skip(index + 1)
        .take(HEADER_LOOKAHEAD_ROWS)
        .find(|(_, candidate)| is_header_row(candidate, gap_limit))
        .map(|(header_index, _)| header_index)?;
    let body = rows.iter().skip(header + 1).take(BODY_LOOKAHEAD_ROWS).any(|candidate| {
        candidate
            .iter()
            .filter(|word| super::is_numeric_word(&word.text))
            .count()
            >= 2
    });
    body.then_some(runs)
}

/// A title reads as a name, not as values: a short run of words with real
/// letters in it. Values, notes and body rows fail on one of the two.
pub(super) fn is_title_words(row: &[&HocrWord]) -> bool {
    if !(2..=TITLE_MAX_WORDS).contains(&row.len()) {
        return false;
    }
    let alphabetic: usize = row
        .iter()
        .map(|word| word.text.chars().filter(|character| character.is_alphabetic()).count())
        .sum();
    alphabetic >= 3
}

fn vertical_sections(rows: &[Vec<&HocrWord>], groups: &[TitleGroup], page_height: f32) -> Option<Vec<RegionSection>> {
    // A leading untitled table would be lost by splitting at the first title.
    if groups.first()?.start != 0 {
        return None;
    }
    let mut sections = Vec::with_capacity(groups.len());
    for (index, group) in groups.iter().enumerate() {
        let end = groups.get(index + 1).map_or(rows.len(), |next| next.start);
        let words: Vec<HocrWord> = rows[group.end..end]
            .iter()
            .flat_map(|row| row.iter().copied())
            .cloned()
            .collect();
        if !section_is_whole(rows, group.end, end, words.len()) {
            return None;
        }
        let bounds = super::region_bounding_box(
            &rows[group.start..end]
                .iter()
                .flat_map(|row| row.iter().copied())
                .cloned()
                .collect::<Vec<_>>(),
            page_height,
        );
        sections.push(RegionSection {
            titles: title_texts(rows, group),
            words,
            bounds,
        });
    }
    Some(sections)
}

fn horizontal_sections(
    rows: &[Vec<&HocrWord>],
    group: &TitleGroup,
    page_height: f32,
    gap_limit: u32,
) -> Option<Vec<RegionSection>> {
    let runs = &group.runs;
    if runs.len() < 2 {
        return None;
    }
    let others = band_rows(rows, group, gap_limit)?;
    let boundaries = block_boundaries(&others, runs)?;
    let mut sections = Vec::with_capacity(runs.len());
    for (block, run) in runs.iter().enumerate() {
        sections.push(block_section(
            rows,
            group,
            &others,
            (boundaries[block], boundaries[block + 1]),
            *run,
            page_height,
        )?);
    }
    Some(sections)
}

/// The band's own rows: the contiguous run up to the last row that still
/// carries its column structure. Prose below it (notes, footers) is not part of
/// any block; sparse data rows inside the run are kept.
fn band_rows<'a>(rows: &'a [Vec<&'a HocrWord>], group: &TitleGroup, gap_limit: u32) -> Option<Vec<&'a HocrWord>> {
    let grid_rows = MIN_GRID_RUNS_PER_BLOCK * group.runs.len();
    let last_grid = rows
        .iter()
        .rposition(|row| row_runs(row, gap_limit).len() >= grid_rows)?;
    let words: Vec<&HocrWord> = rows
        .iter()
        .enumerate()
        .filter(|(index, _)| (*index < group.start || *index >= group.end) && *index <= last_grid)
        .flat_map(|(_, row)| row.iter().copied())
        .collect();
    (!words.is_empty()).then_some(words)
}

fn block_boundaries(words: &[&HocrWord], runs: &[(u32, u32)]) -> Option<Vec<u32>> {
    let mut boundaries = Vec::with_capacity(runs.len() + 1);
    boundaries.push(0);
    for pair in runs.windows(2) {
        boundaries.push(block_boundary(words, pair[0].1, pair[1].0)?);
    }
    boundaries.push(words.iter().map(|word| word.left.saturating_add(word.width)).max()?);
    Some(boundaries)
}

fn block_section(
    rows: &[Vec<&HocrWord>],
    group: &TitleGroup,
    others: &[&HocrWord],
    (left, right): (u32, u32),
    run: (u32, u32),
    page_height: f32,
) -> Option<RegionSection> {
    if right <= left {
        return None;
    }
    let words: Vec<HocrWord> = others
        .iter()
        .filter(|word| {
            let center = word.left + word.width / 2;
            center >= left && center < right
        })
        .map(|word| (*word).clone())
        .collect();
    if words.len() < MIN_SECTION_ROWS * MIN_BLOCK_COLUMNS
        || !words.iter().any(|word| super::is_numeric_word(&word.text))
    {
        return None;
    }
    let mut title_words = title_run_words(rows, group, run);
    if title_words.is_empty() {
        return None;
    }
    title_words.sort_by_key(|word| word.left);
    let title = title_words
        .iter()
        .map(|word| word.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let bounds = super::region_bounding_box(
        &words.iter().chain(title_words.iter()).cloned().collect::<Vec<_>>(),
        page_height,
    );
    Some(RegionSection {
        titles: vec![title],
        words,
        bounds,
    })
}

fn title_texts(rows: &[Vec<&HocrWord>], group: &TitleGroup) -> Vec<String> {
    rows[group.start..group.end]
        .iter()
        .map(|row| row_words_text(row))
        .collect()
}

fn title_run_words(rows: &[Vec<&HocrWord>], group: &TitleGroup, run: (u32, u32)) -> Vec<HocrWord> {
    rows[group.start..group.end]
        .iter()
        .flat_map(|row| row.iter().copied())
        .filter(|word| {
            let center = word.left + word.width / 2;
            center >= run.0 && center < run.1
        })
        .cloned()
        .collect()
}

/// Centre of a free corridor between two title runs: the table boundary no row
/// crosses. Titles sit over the middle of their own table, so the corridor
/// nearest that midpoint is the one between two tables.
fn block_boundary(words: &[&HocrWord], left: u32, right: u32) -> Option<u32> {
    if right <= left {
        return None;
    }
    let mut intervals: Vec<(u32, u32)> = words
        .iter()
        .map(|word| (word.left, word.left.saturating_add(word.width)))
        .filter(|(start, end)| *end > left && *start < right)
        .map(|(start, end)| (start.max(left), end.min(right)))
        .collect();
    intervals.sort_unstable();
    let mut gaps: Vec<(u32, u32)> = Vec::new();
    let mut cursor = left;
    for (start, end) in intervals {
        if start > cursor {
            gaps.push((cursor, start));
        }
        cursor = cursor.max(end);
    }
    if right > cursor {
        gaps.push((cursor, right));
    }
    let midpoint = left + (right - left) / 2;
    let middle = |(start, end): &(u32, u32)| start + (end - start) / 2;
    let chosen = gaps
        .iter()
        .find(|(start, end)| midpoint >= *start && midpoint <= *end)
        .or_else(|| gaps.iter().min_by_key(|gap| middle(gap).abs_diff(midpoint)))?;
    Some(middle(chosen))
}

fn section_is_whole(rows: &[Vec<&HocrWord>], start: usize, end: usize, word_count: usize) -> bool {
    let data_rows = end.saturating_sub(start);
    data_rows >= MIN_SECTION_ROWS
        && word_count >= MIN_SECTION_ROWS * MIN_BLOCK_COLUMNS
        && rows[start..end]
            .iter()
            .any(|row| row.iter().filter(|word| super::is_numeric_word(&word.text)).count() >= 2)
}

fn is_header_row(row: &[&HocrWord], gap_limit: u32) -> bool {
    let runs = row_runs(row, gap_limit);
    if runs.len() < MIN_HEADER_RUNS {
        return false;
    }
    // A header band names its columns: most of its runs carry words, not
    // values. Body rows are the other way round, and a token like `1` inside
    // `Peril 1` must not count against the label.
    let labelled = runs
        .iter()
        .filter(|(start, end)| {
            row.iter()
                .filter(|word| {
                    let center = word.left + word.width / 2;
                    center >= *start && center < *end
                })
                .any(|word| word.text.chars().any(char::is_alphabetic))
        })
        .count();
    labelled * 100 >= runs.len() * HEADER_MIN_LABEL_PERCENT
}

pub(super) fn row_runs(row: &[&HocrWord], gap_limit: u32) -> Vec<(u32, u32)> {
    let mut intervals: Vec<(u32, u32)> = row
        .iter()
        .map(|word| (word.left, word.left.saturating_add(word.width)))
        .collect();
    intervals.sort_unstable();
    let mut runs: Vec<(u32, u32)> = Vec::new();
    for (start, end) in intervals {
        match runs.last_mut() {
            Some(run) if start.saturating_sub(run.1) <= gap_limit => run.1 = run.1.max(end),
            _ => runs.push((start, end)),
        }
    }
    runs
}

fn row_words_text(row: &[&HocrWord]) -> String {
    let mut words: Vec<&HocrWord> = row.to_vec();
    words.sort_by_key(|word| word.left);
    words
        .iter()
        .map(|word| word.text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Put each section's title rows back above the table reconstructed from the
/// rest of that section. The title occupies the full row, as the source prints
/// it, and is not a column label.
pub(super) fn prepend_titles(table: &mut Table, titles: &[String]) -> bool {
    let width = table.cells.iter().map(Vec::len).max().unwrap_or(0);
    if width == 0 || titles.is_empty() {
        return false;
    }
    for title in titles.iter().rev() {
        let mut row = vec![String::new(); width];
        row[0] = title.clone();
        table.cells.insert(0, row);
    }
    table.markdown = crate::pdf::table_reconstruct::table_to_markdown(&table.cells);
    true
}

/// Extend a split table's bounds over the title rows taken out of the words it
/// was reconstructed from, so the title's text is not emitted as prose too.
pub(super) fn include_bounds(table: &mut Table, bounds: Option<BoundingBox>) {
    let Some(bounds) = bounds else { return };
    table.bounding_box = Some(match table.bounding_box {
        None => bounds,
        Some(current) => BoundingBox {
            x0: current.x0.min(bounds.x0),
            y0: current.y0.min(bounds.y0),
            x1: current.x1.max(bounds.x1),
            y1: current.y1.max(bounds.y1),
        },
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use xberg_native_pdf::structure::table_extractor::TableCell;

    fn cell(text: &str, colspan: u32) -> TableCell {
        TableCell {
            text: text.to_string(),
            spans: Vec::new(),
            colspan,
            rowspan: 1,
            mcids: Vec::new(),
            bbox: None,
            is_header: false,
        }
    }

    fn row(cells: Vec<TableCell>) -> TableRow {
        TableRow {
            cells,
            is_header: false,
        }
    }

    fn header(labels: &[&str]) -> TableRow {
        row(labels.iter().map(|label| cell(label, 1)).collect())
    }

    fn body(values: &[&str]) -> TableRow {
        row(values.iter().map(|value| cell(value, 1)).collect())
    }

    fn spanning(text: &str) -> TableRow {
        row(vec![cell(text, 4)])
    }

    fn table(rows: Vec<TableRow>) -> NativeTable {
        NativeTable {
            rows,
            has_header: false,
            col_count: 4,
            bbox: None,
        }
    }

    #[test]
    fn a_title_followed_by_a_header_band_starts_a_new_table() {
        let merged = table(vec![
            spanning("First Factors"),
            header(&["Product", "One", "Two", "Three"]),
            body(&["Alpha", "1", "2", "3"]),
            spanning("Second Factors"),
            header(&["Product", "One", "Two", "Three"]),
            body(&["Beta", "4", "5", "6"]),
        ]);

        let sections = split_ruled_sections(merged);

        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].rows.len(), 3);
        assert_eq!(sections[1].rows.len(), 3);
        assert_eq!(sections[0].rows[0].cells[0].text, "First Factors");
        assert_eq!(sections[1].rows[0].cells[0].text, "Second Factors");
    }

    #[test]
    fn a_spanning_row_inside_a_body_is_not_a_section() {
        let single = table(vec![
            spanning("Factor Schedule"),
            header(&["Product", "One", "Two", "Three"]),
            body(&["Alpha", "1", "2", "3"]),
            spanning("Total"),
            body(&["Beta", "4", "5", "6"]),
            body(&["Gamma", "7", "8", "9"]),
        ]);

        assert_eq!(split_ruled_sections(single).len(), 1);
    }

    #[test]
    fn a_repeated_header_band_without_a_title_is_not_a_section() {
        let single = table(vec![
            spanning("Factor Schedule"),
            header(&["Product", "One", "Two", "Three"]),
            body(&["Alpha", "1", "2", "3"]),
            header(&["Product", "One", "Two", "Three"]),
            body(&["Beta", "4", "5", "6"]),
        ]);

        assert_eq!(split_ruled_sections(single).len(), 1);
    }

    #[test]
    fn a_long_spanning_note_is_not_a_title() {
        let merged = table(vec![
            spanning("First Factors"),
            header(&["Product", "One", "Two", "Three"]),
            body(&["Alpha", "1", "2", "3"]),
            spanning("Note: the factors above are subject to the Maximum Total Discount stated in the policy"),
            spanning("Second Factors"),
            header(&["Product", "One", "Two", "Three"]),
            body(&["Beta", "4", "5", "6"]),
        ]);

        let sections = split_ruled_sections(merged);

        assert_eq!(sections.len(), 2);
        // The note stays with the table it annotates, not with the next title.
        assert_eq!(sections[0].rows.len(), 4);
        assert_eq!(sections[1].rows[0].cells[0].text, "Second Factors");
    }

    const WORD_HEIGHT: u32 = 8;

    fn word(text: &str, left: u32, top: u32, width: u32) -> HocrWord {
        HocrWord {
            text: text.to_string(),
            left,
            top,
            width,
            height: WORD_HEIGHT,
            confidence: 95.0,
        }
    }

    /// One titled 4-column section: title row at `top`, header at `top + 14`,
    /// three numeric body rows from `top + 24`.
    fn section(title: &str, left: u32, top: u32, start: u32, seed: u32) -> Vec<HocrWord> {
        let mut region: Vec<HocrWord> = title
            .split_whitespace()
            .enumerate()
            .map(|(index, part)| word(part, left + 40 + index as u32 * 45, top, 40))
            .collect();
        region.extend([
            word("Value", left, top + 14, 60),
            word("One", left + 100, top + 14, 60),
            word("Two", left + 200, top + 14, 60),
            word("Three", left + 300, top + 14, 60),
        ]);
        for row in 0..3 {
            let value = start + row + seed;
            region.push(word(&value.to_string(), left, top + 24 + row * 9, 40));
            region.push(word("1", left + 100, top + 24 + row * 9, 40));
            region.push(word("2", left + 200, top + 24 + row * 9, 40));
            region.push(word("3", left + 300, top + 24 + row * 9, 40));
        }
        region
    }

    #[test]
    fn stacked_titled_sections_split_into_word_sections() {
        let mut region = section("First Factors", 0, 0, 10, 0);
        region.extend(section("Second Factors", 0, 80, 20, 0));

        let sections = split_titled_region(&region, 792.0).expect("two titled sections");

        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].titles, ["First Factors"]);
        assert_eq!(sections[1].titles, ["Second Factors"]);
        assert_eq!(sections[0].words.len(), 16);
        assert_eq!(sections[1].words.len(), 16);
        // The title words are carried separately, not reconstructed as columns.
        assert!(sections[0].words.iter().all(|word| word.text != "First"));
    }

    #[test]
    fn a_spanning_row_inside_a_body_does_not_split_the_region() {
        let mut region = section("Factor Schedule", 0, 0, 10, 0);
        region.push(word("Total", 0, 60, 120));
        for row in 0..3 {
            let value = 70 + row;
            region.push(word(&value.to_string(), 0, 70 + row * 9, 40));
            region.push(word("1", 100, 70 + row * 9, 40));
            region.push(word("2", 200, 70 + row * 9, 40));
            region.push(word("3", 300, 70 + row * 9, 40));
        }

        assert!(split_titled_region(&region, 792.0).is_none());
    }

    #[test]
    fn a_repeated_header_band_does_not_split_the_region() {
        // The same shape a table takes when a page break repeats its header:
        // a second header band with no title above it.
        let mut region = section("Factor Schedule", 0, 0, 10, 0);
        region.push(word("Value", 0, 60, 60));
        region.push(word("One", 100, 60, 60));
        region.push(word("Two", 200, 60, 60));
        region.push(word("Three", 300, 60, 60));
        for row in 0..3 {
            let value = 80 + row;
            region.push(word(&value.to_string(), 0, 70 + row * 9, 40));
            region.push(word("4", 100, 70 + row * 9, 40));
            region.push(word("5", 200, 70 + row * 9, 40));
            region.push(word("6", 300, 70 + row * 9, 40));
        }
        assert!(split_titled_region(&region, 792.0).is_none());
    }

    #[test]
    fn a_caption_alone_does_not_split_the_region() {
        let mut region = vec![word("Table 3: Results", 0, 0, 160)];
        for (label, left) in ["Code", "One", "Two", "Three"].iter().zip([0, 100, 200, 300]) {
            region.push(word(label, left, 14, 60));
        }
        for row in 0..3 {
            let top = 24 + row * 9;
            for left in [0, 100, 200, 300] {
                region.push(word("7", left, top, 40));
            }
        }
        assert!(split_titled_region(&region, 792.0).is_none());
    }

    #[test]
    fn side_by_side_titled_blocks_split_at_their_corridor() {
        let mut region = vec![word("Alpha Block", 150, 0, 80), word("Beta Block", 650, 0, 80)];
        let columns = [10, 110, 210, 310, 510, 610, 710, 810];
        let labels = ["Code", "One", "Two", "Three", "Code", "One", "Two", "Three"];
        for (label, left) in labels.iter().zip(columns) {
            region.push(word(label, left, 14, 60));
        }
        for row in 0..3 {
            let top = 24 + row * 9;
            for (index, left) in columns.iter().enumerate() {
                let text = if index % 4 == 0 {
                    (row + 10).to_string()
                } else {
                    (index % 3 + 1).to_string()
                };
                region.push(word(&text, *left, top, 40));
            }
        }

        let sections = split_titled_region(&region, 792.0).expect("two side-by-side blocks");

        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].titles, ["Alpha Block"]);
        assert_eq!(sections[1].titles, ["Beta Block"]);
        assert_eq!(sections[0].words.len(), 16);
        assert_eq!(sections[1].words.len(), 16);
    }

    #[test]
    fn prepending_titles_puts_a_full_width_row_above_the_grid() {
        let mut table = Table {
            cells: vec![
                vec!["Value".into(), "One".into(), "Two".into()],
                vec!["10".into(), "1".into(), "2".into()],
            ],
            ..Default::default()
        };

        assert!(prepend_titles(&mut table, &["Alpha Factors".into()]));

        assert_eq!(table.cells.len(), 3);
        assert_eq!(table.cells[0], ["Alpha Factors", "", ""]);
        assert!(table.markdown.contains("Alpha Factors"));
    }
}
