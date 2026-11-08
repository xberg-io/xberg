//! Geometry evidence for re-reading a standalone picture as one uniform block (#2503, #2432).
//!
//! Whether a picture is re-read depends on word boxes only: the first pass must show a column
//! structure. Whether the re-read is kept compares both reads row by row: the text of every row
//! must be identical except for leader dots, and the re-read must put more whole rows on the
//! shared columns.

use crate::types::internal::InternalDocument;
use crate::types::{OcrBoundingGeometry, OcrElementLevel};

/// Rows a column edge must be shared by before it counts as a column.
const MIN_ROWS_PER_COLUMN: usize = 4;
/// Distinct columns a row must touch to count as a table row.
const MIN_COLUMNS_PER_ROW: usize = 3;
/// Shared edges beyond which a page is a lattice of words rather than a table.
const MAX_SHARED_EDGES: usize = 12;
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

/// The first pass shows a table's column structure: enough rows sit on a handful of shared
/// column edges. A lattice of words on dozens of edges is a grid of cells, not a table.
pub(super) fn shows_column_structure(doc: &InternalDocument) -> bool {
    let (rows, edges) = column_structure(doc);
    if edges > MAX_SHARED_EDGES {
        tracing::debug!(edges, "standalone re-read skipped: a lattice of words, not a table");
    } else if rows < MIN_ROWS_PER_COLUMN && rows > 0 {
        tracing::debug!(rows, "standalone re-read skipped: too few rows share the column edges");
    }
    rows >= MIN_ROWS_PER_COLUMN && edges <= MAX_SHARED_EDGES
}

struct Placed<'a> {
    text: &'a str,
    boxed: WordBox,
}

fn placed_words(doc: &InternalDocument) -> Vec<Placed<'_>> {
    doc.prebuilt_ocr_elements
        .iter()
        .flatten()
        .filter(|element| element.level == OcrElementLevel::Word && !element.text.trim().is_empty())
        .filter_map(|element| match element.geometry {
            OcrBoundingGeometry::Rectangle {
                left,
                top,
                width,
                height,
            } if width > 0 && height > 0 => Some(Placed {
                text: element.text.trim(),
                boxed: WordBox {
                    left,
                    right: left.saturating_add(width),
                    center: top.saturating_add(height / 2),
                    height,
                },
            }),
            _ => None,
        })
        .collect()
}

/// Line elements whose box reaches across at least [`MIN_COLUMNS_PER_ROW`] of the shared column
/// edges: a whole table row read as one line.
fn lines_spanning_columns(doc: &InternalDocument, edges: &[(u32, bool)], tolerance: u32) -> usize {
    doc.prebuilt_ocr_elements
        .iter()
        .flatten()
        .filter(|element| element.level == OcrElementLevel::Line)
        .filter(|element| match element.geometry {
            OcrBoundingGeometry::Rectangle { left, width, .. } => {
                let right = left.saturating_add(width);
                edges
                    .iter()
                    .filter(|(edge, _)| {
                        left.saturating_sub(tolerance) <= *edge && *edge <= right.saturating_add(tolerance)
                    })
                    .count()
                    >= MIN_COLUMNS_PER_ROW
            }
            _ => false,
        })
        .count()
}

/// Dots a block read prints for a leader are not a word: a trailing run is dropped from a word
/// and a word made only of dots is dropped entirely. A word of at least [`MIN_LEADER_DOTS`] trailing
/// dots also absorbs the first-pass fragments inside the span those dots occupy.
const MIN_LEADER_DOTS: usize = 3;

fn trailing_dots(text: &str) -> usize {
    text.chars().rev().take_while(|c| *c == '.').count()
}

/// The text of a word without its trailing leader dots and without whitespace, so that
/// `Tree planting` and `Tree` + `planting..........` compare equal while `(9.10)` and `9.10` do not.
fn comparable(text: &str) -> String {
    text.trim_end_matches('.')
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect()
}

/// The horizontal span the trailing dots of `word` occupy, from its width shared by character.
fn leader_span(word: &Placed) -> Option<(u32, u32)> {
    let dots = trailing_dots(word.text);
    if dots < MIN_LEADER_DOTS {
        return None;
    }
    let chars = u32::try_from(word.text.chars().count()).ok()?;
    let dots = u32::try_from(dots).ok()?;
    let width = word.boxed.right - word.boxed.left;
    Some((word.boxed.right - width * dots / chars, word.boxed.right))
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

/// Whether the second read may replace the first, or the reason it may not.
///
/// It must read more whole rows across the first read's shared columns, and its words must say
/// the same thing as the first read's, row by row: equal text in x order apart from leader dots.
/// First-read fragments lying inside a leader the second read printed are absorbed by it. The
/// first read's emission order must be row by row, otherwise the second read would reorder
/// columns of text into rows.
pub(super) fn judge_second_read(first: &InternalDocument, second: &InternalDocument) -> Result<(), &'static str> {
    let first_words = placed_words(first);
    let second_words = placed_words(second);
    let boxes: Vec<WordBox> = first_words.iter().map(|word| word.boxed).collect();
    if boxes.is_empty() {
        return Err("the first read has no word boxes");
    }
    let tolerance = median_height(&boxes) * TOLERANCE_NUMERATOR / TOLERANCE_DENOMINATOR;
    let table_rows = rows(&boxes, tolerance);
    let edges = column_edges(&table_rows, tolerance);
    if lines_spanning_columns(second, &edges, tolerance) <= lines_spanning_columns(first, &edges, tolerance) {
        return Err("the second read has no more rows across the shared columns");
    }
    let centers: Vec<u32> = table_rows
        .iter()
        .map(|row| row.iter().map(|word| word.center).sum::<u32>() / row.len() as u32)
        .collect();

    let mut furthest = 0;
    let mut first_by_row: Vec<Vec<&Placed>> = vec![Vec::new(); centers.len()];
    for word in &first_words {
        let Some(row) = row_of(&centers, &word.boxed, tolerance) else {
            return Err("a first-read word belongs to no row");
        };
        if row < furthest {
            return Err("the first read emits its words column by column");
        }
        furthest = row;
        first_by_row[row].push(word);
    }
    let mut second_by_row: Vec<Vec<&Placed>> = vec![Vec::new(); centers.len()];
    for word in &second_words {
        let Some(row) = row_of(&centers, &word.boxed, tolerance) else {
            return Err("a second-read word belongs to no row of the first read");
        };
        second_by_row[row].push(word);
    }
    for (mut before, mut after) in first_by_row.into_iter().zip(second_by_row) {
        before.sort_unstable_by_key(|word| word.boxed.left);
        after.sort_unstable_by_key(|word| word.boxed.left);
        let leaders: Vec<(u32, u32)> = after.iter().filter_map(|word| leader_span(word)).collect();
        let absorbed = |word: &Placed| {
            leaders.iter().any(|(start, end)| {
                word.boxed.left.saturating_add(tolerance) >= *start && word.boxed.right <= end.saturating_add(tolerance)
            })
        };
        let kept: String = before
            .iter()
            .filter(|word| !absorbed(word))
            .map(|word| comparable(word.text))
            .collect();
        let reread: String = after.iter().map(|word| comparable(word.text)).collect();
        if kept != reread {
            return Err("a row reads differently in the second read");
        }
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

    fn with_lines(mut elements: Vec<OcrElement>, rows: usize) -> InternalDocument {
        for row in 0..rows {
            let mut line = element("row", 10, 10 + row as u32 * 40, 670);
            line.level = OcrElementLevel::Line;
            elements.push(line);
        }
        doc(elements)
    }

    fn reread(first: Vec<OcrElement>, second: Vec<OcrElement>) -> Result<(), &'static str> {
        judge_second_read(&doc(first), &with_lines(second, 6))
    }

    /// Replaces the text of the first word at `left` in row `row` of a `table(6)`-shaped list.
    fn reword(mut words: Vec<OcrElement>, row: u32, left: u32, text: &str) -> Vec<OcrElement> {
        let top = 10 + row * 40;
        let target = words
            .iter_mut()
            .find(|word| {
                matches!(word.geometry, OcrBoundingGeometry::Rectangle { left: l, top: t, .. } if l == left && t == top)
            })
            .expect("word at that position");
        target.text = text.to_string();
        words
    }

    #[test]
    fn an_identical_read_with_more_whole_rows_is_kept() {
        assert_eq!(reread(table(6), table(6)), Ok(()));
    }

    #[test]
    fn a_second_read_without_more_whole_rows_is_refused() {
        assert!(judge_second_read(&doc(table(6)), &doc(table(6))).is_err());
    }

    #[test]
    fn a_dropped_sign_or_parenthesis_is_refused() {
        for (first_text, second_text) in [("(9.10)", "9.10"), ("-12", "12"), ("$5", "5"), ("1,204", "1.204")] {
            let first = reword(table(6), 2, 600, first_text);
            let second = reword(table(6), 2, 600, second_text);
            assert!(reread(first, second).is_err(), "{first_text} -> {second_text}");
        }
    }

    #[test]
    fn a_word_without_letters_or_digits_cannot_be_dropped() {
        let first = reword(table(6), 1, 300, "\u{2014}");
        let mut second = reword(table(6), 1, 300, "\u{2014}");
        second.retain(|word| word.text != "\u{2014}");
        assert!(reread(first, second).is_err());
    }

    #[test]
    fn an_appended_or_prefixed_digit_is_refused() {
        for second_text in ["123", "912", "1"] {
            let second = reword(table(6), 3, 300, second_text);
            assert!(reread(table(6), second).is_err(), "12 -> {second_text}");
        }
    }

    #[test]
    fn a_misread_digit_is_refused_even_when_a_neighbour_cell_holds_the_new_digit() {
        let first = reword(reword(table(6), 4, 300, "8"), 4, 420, "18");
        let second = reword(reword(table(6), 4, 300, "3"), 4, 420, "18");
        assert!(reread(first, second).is_err());
    }

    #[test]
    fn a_misread_digit_in_the_same_box_is_refused() {
        let second = reword(table(6), 0, 420, "345,679");
        assert!(reread(table(6), second).is_err());
    }

    #[test]
    fn a_misread_in_a_column_without_a_shared_edge_is_refused() {
        let off_edge = |text: &str| -> Vec<OcrElement> {
            [0_u32, 7, 3, 11, 5, 9]
                .into_iter()
                .enumerate()
                .flat_map(|(row, offset)| {
                    let row = row as u32;
                    let mut cells = table_row(row).to_vec();
                    cells.insert(2, element(text, 200 + offset, 10 + row * 40, 30));
                    cells
                })
                .collect()
        };
        assert_eq!(reread(off_edge("77"), off_edge("77")), Ok(()));
        assert!(reread(off_edge("77"), off_edge("71")).is_err());
    }

    fn with_fragments(leader: bool) -> (Vec<OcrElement>, Vec<OcrElement>) {
        let first = (0..6_u32)
            .flat_map(|row| {
                let mut cells = table_row(row).to_vec();
                cells.insert(1, element("ee", 100, 10 + row * 40, 40));
                cells
            })
            .collect();
        let second = (0..6_u32)
            .flat_map(|row| {
                let mut cells = table_row(row).to_vec();
                if leader {
                    cells[0] = element("Label.........", 10, 10 + row * 40, 200);
                }
                cells
            })
            .collect();
        (first, second)
    }

    #[test]
    fn fragments_inside_a_printed_leader_are_absorbed_and_otherwise_count_as_lost_words() {
        let (first, second) = with_fragments(true);
        assert_eq!(reread(first, second), Ok(()));
        let (first, second) = with_fragments(false);
        assert!(
            reread(first, second).is_err(),
            "a fragment with no leader over it is a lost word"
        );
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

    #[test]
    fn a_two_column_page_read_column_by_column_is_not_reordered_into_rows() {
        let page = doc(two_columns(false));
        let mut second = two_columns(false);
        for row in 0..8 {
            let mut line = element("row", 10, 10 + row * 40, 800);
            line.level = OcrElementLevel::Line;
            second.push(line);
        }
        assert_eq!(
            judge_second_read(&page, &doc(second.clone())),
            Err("the first read emits its words column by column")
        );
        let mut row_major = two_columns(true);
        row_major.extend(second.into_iter().filter(|word| word.level == OcrElementLevel::Line));
        assert_eq!(judge_second_read(&doc(two_columns(true)), &doc(row_major)), Ok(()));
    }
}
