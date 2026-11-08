//! Geometry evidence for re-reading a standalone picture as one uniform block (#2503, #2432).
//!
//! Everything here reads word boxes only: whether the first pass shows a column structure, and
//! whether a second read keeps every word of the first while putting more rows on those
//! columns. No condition depends on what a word says beyond comparing the two word multisets.

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

/// Rows with words on at least [`MIN_COLUMNS_PER_ROW`] distinct shared column edges, and how many
/// shared edges the page has. Prose shares only its left margin; a table shares one edge per
/// column.
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
            hit.len() >= MIN_COLUMNS_PER_ROW
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

/// The second read puts more whole rows on one line across the first read's shared columns, and
/// every word of the first read that sits on a shared column edge is still there: the second
/// read places a word over the same box, or tokens that begin the same tokens on the same text
/// row. A leader run a block read prints over what the first read split into fragments therefore
/// keeps them.
pub(super) fn second_read_keeps_more_rows(first: &InternalDocument, second: &InternalDocument) -> bool {
    let Some(words) = word_boxes(first) else {
        return false;
    };
    let tolerance = median_height(&words) * TOLERANCE_NUMERATOR / TOLERANCE_DENOMINATOR;
    let edges = column_edges(&rows(&words, tolerance), tolerance);
    if lines_spanning_columns(second, &edges, tolerance) <= lines_spanning_columns(first, &edges, tolerance) {
        return false;
    }
    let on_edge = |word: &WordBox| {
        edges.iter().any(|(edge, is_right)| {
            let at = if *is_right { word.right } else { word.left };
            at.abs_diff(*edge) <= tolerance
        })
    };
    let second_words = placed_words(second);
    placed_words(first)
        .iter()
        .filter(|word| on_edge(&word.boxed))
        .all(|word| {
            let mut over: Vec<&Placed> = second_words
                .iter()
                .filter(|candidate| {
                    let middle = candidate.boxed.left / 2 + candidate.boxed.right / 2;
                    candidate.boxed.center.abs_diff(word.boxed.center) <= tolerance
                        && word.boxed.left.saturating_sub(tolerance) <= middle
                        && middle <= word.boxed.right.saturating_add(tolerance)
                })
                .collect();
            over.sort_unstable_by_key(|candidate| candidate.boxed.left);
            let second_text: String = over.iter().map(|candidate| alphanumerics(candidate.text)).collect();
            second_text.contains(&alphanumerics(word.text))
        })
}

/// Comparison normalization only: dropping punctuation lets `Label` match `Label.........`
/// where a block read prints a leader. It is not a text-shape condition on a recovery rule
/// (#2616); the digits and letters themselves must match.
fn alphanumerics(text: &str) -> String {
    text.chars().filter(|c| c.is_alphanumeric()).collect()
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

    pub(super) fn table(rows: usize) -> Vec<OcrElement> {
        (0..rows)
            .flat_map(|row| {
                let top = 10 + row as u32 * 40;
                [
                    element("Label", 10, top, 120),
                    element("12", 300, top, 30),
                    element("345,678", 420, top, 90),
                    element("(9.10)", 600, top, 70),
                ]
            })
            .collect()
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

    #[test]
    fn a_second_read_that_loses_an_aligned_word_is_refused() {
        let first = doc(table(6));
        let mut lost = table(6);
        lost.retain(|e| e.text != "345,678");
        assert!(!second_read_keeps_more_rows(&first, &with_lines(lost, 6)));
    }

    #[test]
    fn a_second_read_with_whole_rows_and_every_aligned_word_is_kept() {
        let mut first = table(6);
        first.push(element("ee", 200, 10, 40));
        let mut second = table(6);
        second[0] = element("Label.........", 10, 10, 200);
        assert!(second_read_keeps_more_rows(&doc(first), &with_lines(second, 6)));
    }

    #[test]
    fn a_second_read_that_misreads_a_digit_in_the_same_box_is_refused() {
        let first = doc(table(6));
        let mut second = table(6);
        second[2] = element("345,679", 420, 10, 90);
        assert!(!second_read_keeps_more_rows(&first, &with_lines(second, 6)));
    }

    #[test]
    fn a_second_read_that_shortens_a_word_to_a_prefix_elsewhere_on_the_row_is_refused() {
        let first = doc(table(6));
        let mut second = table(6);
        second[1] = element("1", 300, 10, 30);
        assert!(!second_read_keeps_more_rows(&first, &with_lines(second, 6)));
    }

    #[test]
    fn a_second_read_without_more_whole_rows_is_refused() {
        assert!(!second_read_keeps_more_rows(&doc(table(6)), &doc(table(6))));
    }
}
