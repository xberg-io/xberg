//! The decision to read a standalone picture a second time as one uniform block, and to keep
//! that read (#2503, #2432).
//!
//! Admission is geometry only: the first read's word boxes must form a lattice table. Four or
//! more rows share a few column edges with wide gaps between at least three columns, the words
//! come out row by row, and the page is not a lattice of dozens of edges. Nothing depends on what
//! a word says. The second read is adopted only if it finds a table, so a page that is not a
//! readable table keeps its first read. A token it misreads is kept; a read that failed outright
//! is discarded (see [`second_read_is_usable`]).

use crate::types::internal::InternalDocument;
use crate::types::{OcrBoundingGeometry, OcrElementLevel};

/// Rows a column edge must be shared by before it counts as a column.
const MIN_ROWS_PER_COLUMN: usize = 4;
/// Distinct columns a row must touch to count as a table row.
const MIN_COLUMNS_PER_ROW: usize = 3;
/// Shared edges beyond which a page is a lattice of words rather than a table.
const MAX_SHARED_EDGES: usize = 12;
/// A second read must keep at least this fraction of the first read's letters and digits. On the
/// #2432 set the useful re-reads keep 0.87 or more and the failed ones 0.42 or less (the 7 px
/// pictures at 0.50 and 0.59 gain no rows either way).
const MIN_RETAINED: (usize, usize) = (3, 4);
/// Edge tolerance and row-membership tolerance, as a fraction of the median word height.
const TOLERANCE_NUMERATOR: u32 = 1;
const TOLERANCE_DENOMINATOR: u32 = 2;

#[derive(Clone, Copy)]
struct WordBox {
    left: u32,
    right: u32,
    center: u32,
    height: u32,
}

fn word_boxes(doc: &InternalDocument) -> Option<Vec<WordBox>> {
    let elements = doc.prebuilt_ocr_elements.as_ref()?;
    let words: Vec<_> = elements
        .iter()
        .filter(|element| element.level == OcrElementLevel::Word && !element.text.trim().is_empty())
        .filter_map(|element| match element.geometry {
            OcrBoundingGeometry::Rectangle {
                left,
                top,
                width,
                height,
            } if width > 0 && height > 0 => Some(WordBox {
                left,
                right: left.saturating_add(width),
                center: top.saturating_add(height / 2),
                height,
            }),
            _ => None,
        })
        .collect();
    (!words.is_empty()).then_some(words)
}

fn median_height(words: &[WordBox]) -> u32 {
    let mut heights: Vec<u32> = words.iter().map(|word| word.height).collect();
    heights.sort_unstable();
    heights[heights.len() / 2]
}

/// Words grouped into text rows by the vertical position of their centres.
fn rows(words: &[WordBox], tolerance: u32) -> Vec<Vec<WordBox>> {
    let mut sorted = words.to_vec();
    sorted.sort_unstable_by_key(|word| word.center);
    let mut rows: Vec<Vec<WordBox>> = Vec::new();
    for word in sorted {
        match rows.last_mut() {
            Some(row) if word.center.abs_diff(row[row.len() - 1].center) <= tolerance => row.push(word),
            _ => rows.push(vec![word]),
        }
    }
    rows
}

/// Column edges (left or right) shared by at least [`MIN_ROWS_PER_COLUMN`] distinct rows and
/// by at least two thirds of all rows, so the dense ragged edges of prose never form one,
/// as sorted `(edge, is_right)` cluster anchors.
fn column_edges(rows: &[Vec<WordBox>], tolerance: u32) -> Vec<(u32, bool)> {
    let mut edges = Vec::new();
    for is_right in [false, true] {
        let mut points: Vec<(u32, usize)> = rows
            .iter()
            .enumerate()
            .flat_map(|(index, row)| {
                row.iter()
                    .map(move |word| (if is_right { word.right } else { word.left }, index))
            })
            .collect();
        points.sort_unstable();
        let mut start = 0;
        while start < points.len() {
            let mut end = start + 1;
            while end < points.len() && points[end].0 - points[start].0 <= tolerance {
                end += 1;
            }
            let mut members: Vec<usize> = points[start..end].iter().map(|point| point.1).collect();
            members.sort_unstable();
            members.dedup();
            if members.len() >= MIN_ROWS_PER_COLUMN.max(rows.len() * 2 / 3) {
                edges.push((points[(start + end) / 2].0, is_right));
            }
            start = end;
        }
    }
    edges
}

/// Gaps between neighbouring words of a row that are at least three tolerances wide (one and a
/// half word heights): the space between cells, not between the words of one cell.
fn wide_gaps(row: &[WordBox], tolerance: u32) -> usize {
    let mut sorted = row.to_vec();
    sorted.sort_unstable_by_key(|word| word.left);
    sorted
        .windows(2)
        .filter(|pair| pair[1].left.saturating_sub(pair[0].right) >= tolerance * 3)
        .count()
}

/// Rows with words on at least [`MIN_COLUMNS_PER_ROW`] distinct shared column edges and a wide
/// gap between each pair of columns, and how many shared edges the page has. Prose shares only
/// its left margin, and a justified two-column page has one gutter, not a gap per column.
fn column_structure(doc: &InternalDocument) -> (usize, usize) {
    let Some(words) = word_boxes(doc) else {
        return (0, 0);
    };
    let tolerance = median_height(&words) * TOLERANCE_NUMERATOR / TOLERANCE_DENOMINATOR;
    let rows = rows(&words, tolerance);
    let edges = column_edges(&rows, tolerance);
    let on_columns = rows
        .iter()
        .filter(|row| {
            let mut hit: Vec<usize> = row
                .iter()
                .flat_map(|word| {
                    edges.iter().enumerate().filter_map(move |(index, (edge, is_right))| {
                        let at = if *is_right { word.right } else { word.left };
                        (at.abs_diff(*edge) <= tolerance).then_some(index)
                    })
                })
                .collect();
            hit.sort_unstable();
            hit.dedup();
            hit.len() >= MIN_COLUMNS_PER_ROW && wide_gaps(row, tolerance) >= MIN_COLUMNS_PER_ROW - 1
        })
        .count();
    (on_columns, edges.len())
}

/// The row of the first read a word belongs to, by the nearest row centre within tolerance.
fn row_of(centers: &[u32], word: &WordBox, tolerance: u32) -> Option<usize> {
    centers
        .iter()
        .enumerate()
        .map(|(index, center)| (index, center.abs_diff(word.center)))
        .filter(|(_, distance)| *distance <= tolerance)
        .min_by_key(|(_, distance)| *distance)
        .map(|(index, _)| index)
}

/// Whether the words come out row by row. A page read column by column would be reordered into
/// interleaved rows by a block read, so it is not re-read.
fn emits_row_by_row(words: &[WordBox], tolerance: u32) -> bool {
    let table_rows = rows(words, tolerance);
    let centers: Vec<u32> = table_rows
        .iter()
        .map(|row| row.iter().map(|word| word.center).sum::<u32>() / row.len() as u32)
        .collect();
    let mut furthest = 0;
    for word in words {
        let Some(row) = row_of(&centers, word, tolerance) else {
            return false;
        };
        if row < furthest {
            return false;
        }
        furthest = row;
    }
    true
}

/// The first pass shows a lattice table: enough rows sit on a handful of shared column edges
/// with wide gaps between columns, and the words come out row by row. A lattice of words on
/// dozens of edges is a grid of cells, not a table. Every refusal is logged with the counts.
pub(super) fn shows_column_structure(doc: &InternalDocument) -> bool {
    let (rows, edges) = column_structure(doc);
    if edges > MAX_SHARED_EDGES {
        tracing::debug!(
            rows,
            edges,
            "standalone table re-read skipped: a lattice of words, not a table"
        );
        return false;
    }
    if rows < MIN_ROWS_PER_COLUMN {
        tracing::debug!(
            rows,
            edges,
            "standalone table re-read skipped: too few rows share the column edges"
        );
        return false;
    }
    let Some(words) = word_boxes(doc) else {
        tracing::debug!("standalone table re-read skipped: the first read has no word boxes");
        return false;
    };
    let tolerance = median_height(&words) * TOLERANCE_NUMERATOR / TOLERANCE_DENOMINATOR;
    if !emits_row_by_row(&words, tolerance) {
        tracing::debug!("standalone table re-read skipped: the first read emits its words column by column");
        return false;
    }
    true
}

/// Letters and digits (Unicode alphanumerics, so any script's digits) in the word elements of a
/// read; lines, blocks and pages are not counted again.
fn alphanumeric_count(doc: &InternalDocument) -> usize {
    doc.prebuilt_ocr_elements
        .iter()
        .flatten()
        .filter(|element| element.level == OcrElementLevel::Word)
        .map(|element| element.text.chars().filter(|c| c.is_alphanumeric()).count())
        .sum()
}

/// Dots in a run a word must hold to count as a leader.
const MIN_LEADER_DOTS: usize = 5;

/// Words that are nothing but a run of dots: the leaders a block read keeps between a label and its
/// values.
fn leader_words(doc: &InternalDocument) -> usize {
    doc.prebuilt_ocr_elements
        .iter()
        .flatten()
        .filter(|element| element.level == OcrElementLevel::Word)
        .filter(|element| {
            let text = element.text.trim();
            text.chars().count() >= MIN_LEADER_DOTS && text.chars().all(|c| c == '.')
        })
        .count()
}

/// A block read that found no table but whose words are leader rows on shared column edges: the
/// table check drops a dot-leader table as prose, and the row recovery keeps each row on one line.
fn reads_leader_rows(doc: &InternalDocument) -> bool {
    leader_words(doc) >= MIN_ROWS_PER_COLUMN && shows_column_structure(doc)
}

/// A second read with no letters or digits in its word elements, or with less than three quarters
/// of the first read's, failed to read the picture. A second read with no table did not confirm the
/// table the geometry showed, so the first read stands, unless its words are dot-leader rows (see
/// [`reads_leader_rows`]). Any other second read is adopted as is. A
/// first read with no letters or digits is replaced by any second read that has some and a table. A
/// second read with more letters is not capped: a block read legitimately recovers text the sparse
/// read missed, and the guard only separates a failed read from a read.
pub(super) fn second_read_is_usable(first: &InternalDocument, second: &InternalDocument) -> Result<(), &'static str> {
    let (before, after) = (alphanumeric_count(first), alphanumeric_count(second));
    tracing::debug!(before, after, "standalone table re-read: letters and digits per read");
    if after == 0 {
        return Err("the second read has no letters or digits in its word elements");
    }
    if after * MIN_RETAINED.1 < before * MIN_RETAINED.0 {
        return Err("the second read kept too few letters and digits");
    }
    if second.tables.is_empty() && !reads_leader_rows(second) {
        return Err("the second read found no table");
    }
    Ok(())
}

/// A first-pass document whose words form an eight-row, four-column table.
#[cfg(test)]
pub(super) fn sample_table_document() -> InternalDocument {
    tests::doc(tests::table(8))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{OcrConfidence, OcrElement};

    fn element(text: &str, left: u32, top: u32, width: u32) -> OcrElement {
        OcrElement {
            text: text.to_string(),
            geometry: OcrBoundingGeometry::Rectangle {
                left,
                top,
                width,
                height: 20,
            },
            confidence: OcrConfidence::default(),
            level: OcrElementLevel::Word,
            rotation: None,
            page_number: 1,
            parent_id: None,
            backend_metadata: Default::default(),
        }
    }

    pub(super) fn doc(elements: Vec<OcrElement>) -> InternalDocument {
        InternalDocument {
            prebuilt_ocr_elements: Some(elements),
            ..Default::default()
        }
    }

    /// A read that found a table among its words.
    fn doc_with_table(elements: Vec<OcrElement>) -> InternalDocument {
        let mut read = doc(elements);
        read.tables.push(Default::default());
        read
    }

    fn table_row(row: u32) -> [OcrElement; 4] {
        let top = 10 + row * 40;
        [
            element("Label", 10, top, 120),
            element("12", 300, top, 30),
            element("345,678", 420, top, 90),
            element("(9.10)", 600, top, 70),
        ]
    }

    pub(super) fn table(rows: usize) -> Vec<OcrElement> {
        (0..rows as u32).flat_map(table_row).collect()
    }

    fn prose(lines: usize) -> Vec<OcrElement> {
        let mut state = 7_u32;
        let mut elements = Vec::new();
        for line in 0..lines {
            let top = 10 + line as u32 * 40;
            let mut x = 10;
            for _ in 0..9 {
                state = state.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                let width = 25 + (state >> 16) % 90;
                elements.push(element("word", x, top, width));
                x += width + 12;
            }
        }
        elements
    }

    #[test]
    fn aligned_columns_show_a_column_structure() {
        let table = doc(table(8));
        assert_eq!(column_structure(&table).0, 8);
        assert!(shows_column_structure(&table));
    }

    #[test]
    fn a_lattice_of_words_is_not_a_table() {
        let lattice: Vec<_> = (0..8)
            .flat_map(|row| (0..16).map(move |column| element("ad", 10 + column * 96, 10 + row * 40, 24)))
            .collect();
        assert!(!shows_column_structure(&doc(lattice)));
    }

    #[test]
    fn ragged_prose_shows_no_column_structure() {
        let prose = doc(prose(10));
        assert!(!shows_column_structure(&prose), "{:?}", column_structure(&prose));
    }

    fn two_columns(row_major: bool) -> Vec<OcrElement> {
        let cell = |row: u32, column: u32, word: u32| {
            let left = 10 + column * 400 + word * 120;
            element("text", left, 10 + row * 40, 100 + (word + row) % 2 * 20)
        };
        let order: Vec<(u32, u32)> = if row_major {
            (0..8).flat_map(|row| (0..2).map(move |column| (row, column))).collect()
        } else {
            (0..2).flat_map(|column| (0..8).map(move |row| (row, column))).collect()
        };
        order
            .into_iter()
            .flat_map(|(row, column)| (0..3).map(move |word| (row, column, word)))
            .map(|(row, column, word)| cell(row, column, word))
            .collect()
    }

    #[test]
    fn a_two_column_page_with_one_gutter_is_not_a_table() {
        assert!(!shows_column_structure(&doc(two_columns(true))));
        assert!(!shows_column_structure(&doc(two_columns(false))));
    }

    /// A four-column table whose words are emitted column by column instead of row by row.
    fn table_by_column(rows: u32) -> Vec<OcrElement> {
        (0..4)
            .flat_map(|column| (0..rows).map(move |row| table_row(row)[column].clone()))
            .collect()
    }

    #[test]
    fn a_table_emitted_column_by_column_is_not_re_read_but_the_same_table_by_rows_is() {
        assert!(shows_column_structure(&doc(table(6))));
        assert!(!shows_column_structure(&doc(table_by_column(6))));
    }

    #[test]
    fn a_table_with_too_few_rows_is_not_re_read() {
        assert!(!shows_column_structure(&doc(table(3))));
    }

    #[test]
    fn a_two_column_table_with_one_wide_gap_is_not_re_read() {
        let two: Vec<_> = table(8)
            .into_iter()
            .filter(|word| word.text == "Label" || word.text == "12")
            .collect();
        assert!(!shows_column_structure(&doc(two)));
    }

    #[test]
    fn a_re_read_that_changes_a_token_is_adopted() {
        let first = doc(table(6));
        let mut changed = table(6);
        changed[1].text = "17".to_string();
        changed[3].text = "9.10".to_string();
        assert_eq!(second_read_is_usable(&first, &doc_with_table(changed)), Ok(()));
    }

    fn reads_of(first: &str, second: &str) -> Result<(), &'static str> {
        second_read_is_usable(
            &doc(vec![element(first, 0, 0, 10)]),
            &doc_with_table(vec![element(second, 0, 0, 10)]),
        )
    }

    #[test]
    fn an_empty_re_read_is_refused() {
        let first = doc(table(6));
        let reason = "the second read has no letters or digits in its word elements";
        assert_eq!(second_read_is_usable(&first, &doc(Vec::new())), Err(reason));
        assert_eq!(reads_of("abcd", "...."), Err(reason));
        let mut lines_only = element("abcd", 0, 0, 10);
        lines_only.level = OcrElementLevel::Line;
        assert_eq!(second_read_is_usable(&first, &doc(vec![lines_only])), Err(reason));
    }

    #[test]
    fn a_re_read_without_a_table_is_refused_but_the_same_read_with_one_is_adopted() {
        let first = doc(table(6));
        let reason = "the second read found no table";
        assert_eq!(second_read_is_usable(&first, &doc(table(6))), Err(reason));
        assert_eq!(second_read_is_usable(&first, &doc_with_table(table(6))), Ok(()));
    }

    fn leader_table(rows: usize, leader: &str) -> Vec<OcrElement> {
        (0..rows as u32)
            .flat_map(|row| {
                let top = 10 + row * 40;
                [
                    element("Label", 10, top, 120),
                    element(leader, 140, top, 140),
                    element("12", 300, top, 30),
                    element("345,678", 420, top, 90),
                    element("(9.10)", 600, top, 70),
                ]
            })
            .collect()
    }

    #[test]
    fn a_re_read_without_a_table_is_adopted_when_its_rows_carry_leaders() {
        let first = doc(table(6));
        let reason = "the second read found no table";
        assert_eq!(
            second_read_is_usable(&first, &doc(leader_table(6, "........."))),
            Ok(())
        );
        assert_eq!(
            second_read_is_usable(&first, &doc(leader_table(6, "...."))),
            Err(reason)
        );
        assert_eq!(
            second_read_is_usable(&first, &doc(leader_table(6, "ee eae"))),
            Err(reason)
        );
        assert_eq!(
            second_read_is_usable(&doc(table(3)), &doc(leader_table(3, "........."))),
            Err(reason)
        );
    }

    #[test]
    fn leaders_without_column_structure_are_not_a_table() {
        let first = doc(vec![element("abcd", 10, 400, 30)]);
        let mut leaders: Vec<_> = (0..6).map(|row| element(".........", 10, 10 + row * 40, 140)).collect();
        leaders.push(element("abcd", 10, 400, 30));
        assert_eq!(
            second_read_is_usable(&first, &doc(leaders)),
            Err("the second read found no table")
        );
    }

    #[test]
    fn the_retention_boundary_is_exactly_three_quarters() {
        assert_eq!(reads_of("abcdefgh", "abcdef"), Ok(()), "6 of 8 is exactly 3/4");
        assert!(reads_of("abcdefgh", "abcde").is_err(), "5 of 8 is under 3/4");
        assert_eq!(reads_of("abcd", "abc"), Ok(()), "3 of 4");
        assert!(reads_of("abcdefghi", "abcdef").is_err(), "6 of 9 is under 3/4");
    }

    #[test]
    fn a_first_read_without_letters_or_digits_is_replaced_by_any_second_read_that_has_some() {
        assert_eq!(reads_of("---", "ab"), Ok(()));
        assert!(reads_of("---", "...").is_err());
    }

    #[test]
    fn a_longer_second_read_is_not_capped() {
        assert_eq!(reads_of("ab", "abcdefghijklmnopqrstuvwxyz0123456789"), Ok(()));
    }

    #[test]
    fn digits_and_letters_of_any_script_count() {
        assert_eq!(
            reads_of("\u{661}\u{662}\u{663}\u{664}", "\u{661}\u{662}\u{663}"),
            Ok(())
        );
        assert!(reads_of("\u{661}\u{662}\u{663}\u{664}", "\u{661}").is_err());
        assert_eq!(reads_of("\u{ff11}\u{ff12}", "\u{ff11}\u{ff12}"), Ok(()));
        assert!(reads_of("\u{5de5}\u{4e8b}\u{8a18}\u{8f09}", "\u{5de5}").is_err());
    }

    #[test]
    fn a_far_smaller_re_read_is_refused() {
        let first = doc(table(6));
        let mut sparse = table(6);
        sparse.retain(|word| word.text == "12");
        assert!(second_read_is_usable(&first, &doc(sparse)).is_err());
        let mut lost_label = table(6);
        lost_label.retain(|word| word.text != "Label");
        assert!(second_read_is_usable(&first, &doc(lost_label)).is_err());
        let mut lost_count = table(6);
        lost_count.retain(|word| word.text != "12");
        assert_eq!(second_read_is_usable(&first, &doc_with_table(lost_count)), Ok(()));
    }
}
