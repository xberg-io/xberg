//! The work of the misread match is bounded by construction: a native row reads at most 16 OCR
//! rows, the nearest to it in length and place, whatever the lengths of the rows on its page. And a misread
//! row leaves the page text at its own line on a page with line boxes.

use super::super::document::{
    LayerFirst, LineAt, LineBox, MISREAD_CANDIDATES_READ, MISREAD_SIMILARITY_CALLS, PageLineBoxes, PlainOcrRows,
    layer_lines_to_put_first, text_with_layer_first,
};
use crate::extractors::pdf::unmapped_glyphs::LOST_SYMBOL_GLYPH_MARKER;

/// The most OCR rows that one native row reads, and the most similarity tests it runs. ~keep
const CAP: usize = 16;

/// The similarity tests run and the OCR rows read while the native rows go first on one page
/// without line boxes, and the rows put first. ~keep
fn work_of(native: &[String], read: &[String]) -> (usize, usize, Option<LayerFirst>) {
    let thresholds = crate::core::config::OcrQualityThresholds::default();
    let calls = MISREAD_SIMILARITY_CALLS.with(std::cell::Cell::get);
    let reads = MISREAD_CANDIDATES_READ.with(std::cell::Cell::get);
    let (native, read) = (native.join("\n"), read.join("\n"));
    let first = layer_lines_to_put_first(&native, &read, true, &thresholds, None, PlainOcrRows::every());
    (
        MISREAD_SIMILARITY_CALLS.with(std::cell::Cell::get) - calls,
        MISREAD_CANDIDATES_READ.with(std::cell::Cell::get) - reads,
        first,
    )
}

/// The page text after the native rows go first, without line boxes. ~keep
fn merged(native: &[String], read: &[String]) -> String {
    let (_, _, first) = work_of(native, read);
    let read = read.join("\n");
    first.map_or_else(|| read.clone(), |first| text_with_layer_first(&read, &first))
}

/// Asserts that `rows` native rows ran at most `CAP` similarity tests and read at most `CAP` OCR
/// rows each. ~keep
fn assert_bounded(shape: &str, rows: usize, (calls, reads): (usize, usize)) {
    assert!(
        calls <= CAP * rows && reads <= CAP * rows,
        "{shape}: {rows} rows ran {calls} similarity tests and read {reads} OCR rows, over {} each",
        CAP * rows
    );
}

/// Eight words in an order drawn from `n` and `salt`: rows of one length that share every word. ~keep
fn permuted_row(n: usize, salt: usize) -> String {
    let mut words = ["alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf", "hotel"];
    let mut x: u64 = (n as u64)
        .wrapping_mul(2_654_435_761)
        .wrapping_add(salt as u64 * 40_503 + 1);
    for i in (1..words.len()).rev() {
        x = x
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let j = (x >> 33) as usize % (i + 1);
        words.swap(i, j);
    }
    words.join(" ")
}

/// Equal-length rows that share all their words pass every cheap check, and no row misreads
/// another: the similarity tests stay linear in the rows. ~keep
#[test]
fn equal_length_rows_that_share_their_words_run_a_bounded_number_of_similarity_tests() {
    for rows in [250, 1000] {
        let native: Vec<String> = (0..rows).map(|n| permuted_row(n, 1)).collect();
        let read: Vec<String> = (0..rows).map(|n| permuted_row(n, 2)).collect();
        let (calls, reads, _) = work_of(&native, &read);
        assert_bounded("permuted", rows, (calls, reads));
    }
}

/// Each OCR row misreads its own native row (a dropped apostrophe), all of one length: the work
/// stays linear and every misread row still leaves. ~keep
#[test]
fn equal_length_rows_that_each_misread_one_native_row_stay_bounded_and_leave() {
    for rows in [250, 1000] {
        let native: Vec<String> = (0..rows).map(|n| permuted_row(n, 1)).collect();
        let read: Vec<String> = native.iter().map(|row| format!("{row}\u{2019}")).collect();
        let (calls, reads, first) = work_of(&native, &read);
        assert_bounded("misread", rows, (calls, reads));
        // A native row drawn twice has two OCR rows that misread it, and each takes one. ~keep
        let left = first.map_or(0, |first| first.misread_ocr_rows.len());
        assert_eq!(left, rows, "every misread row leaves");
    }
}

/// Every OCR row misreads every native row (one swap of two words in a long row), so a search
/// for the best fit over all rows tests every pair: the cap bounds this page too. ~keep
#[test]
fn rows_that_all_misread_each_other_stay_bounded_and_leave() {
    let words: Vec<String> = (0..101)
        .map(|n: u8| format!("{}{}", (b'a' + n / 26) as char, (b'a' + n % 26) as char))
        .collect();
    let swapped = |at: usize| {
        let mut row = words.clone();
        row.swap(at, at + 1);
        row.join(" ")
    };
    let rows = 100;
    let native: Vec<String> = (0..rows).map(swapped).collect();
    let read: Vec<String> = native.iter().map(|row| format!("{row}\u{2019}")).collect();
    let (calls, reads, first) = work_of(&native, &read);
    assert_bounded("all-pairs", rows, (calls, reads));
    let left = first.map_or(0, |first| first.misread_ocr_rows.len());
    assert_eq!(left, rows, "every misread row leaves");
}

/// Native rows that lost a symbol glyph at a word edge read OCR rows of any length (the glued
/// pattern ignores the length): the rows they read stay linear too. ~keep
#[test]
fn marked_rows_read_a_bounded_number_of_ocr_rows() {
    let rows = 500;
    let native: Vec<String> = (0..rows)
        .map(|n| format!("{LOST_SYMBOL_GLYPH_MARKER}{} row {n}", permuted_row(n, 1)))
        .collect();
    let read: Vec<String> = (0..rows)
        .map(|n| format!("scanned {} of {} words", "x ".repeat(n % 40), n))
        .collect();
    let (calls, reads, _) = work_of(&native, &read);
    assert_bounded("marked", rows, (calls, reads));
}

/// A native row of one-letter words and the OCR row that drops the given ones: a misread of one
/// fewer character for each dropped word. ~keep
fn letters_row(dropped: &[char]) -> String {
    let words: Vec<String> = ('a'..='t')
        .filter(|letter| !dropped.contains(letter))
        .map(|letter| letter.to_string())
        .collect();
    format!("plain words of the row {}", words.join(" "))
}

/// Sixteen OCR rows that drop two letters come before the one that drops a single letter. The
/// native row reads the rows nearest to its length first, so it takes the better misread although
/// the cap stops it before the others. ~keep
#[test]
fn a_native_row_reads_the_ocr_rows_nearest_its_length_first() {
    let native = vec![letters_row(&[])];
    let pairs = ('a'..='p').map(|letter| [letter, 't']);
    let mut read: Vec<String> = pairs.map(|pair| letters_row(&pair)).collect();
    read.push(letters_row(&['t']));
    assert_eq!(read.len(), CAP + 1);
    let mut expected = vec![native[0].clone(), String::new()];
    expected.extend(read[..CAP].iter().cloned());
    assert_eq!(merged(&native, &read).lines().collect::<Vec<_>>(), expected);
}

/// Sixteen OCR rows one character shorter than the native row come before one row one character
/// longer, the better misread. At an equal distance in length the longer row is read first, so
/// the native row takes it. ~keep
#[test]
fn a_native_row_reads_the_longer_ocr_row_first_at_an_equal_distance_in_length() {
    let native = vec![letters_row(&[])];
    let mut read: Vec<String> = ('a'..='p').map(|letter| letters_row(&[letter])).collect();
    read.push(format!("{} a", letters_row(&[])));
    assert_eq!(read.len(), CAP + 1);
    let mut expected = vec![native[0].clone(), String::new()];
    expected.extend(read[..CAP].iter().cloned());
    assert_eq!(merged(&native, &read).lines().collect::<Vec<_>>(), expected);
}

/// Forty OCR rows of one length each misread only the native row at their own place, after twelve
/// OCR rows of that length that misread no native row. A native row reads the rows nearest to its
/// place too, so each takes its own misread although its reads by length stop at the first rows of
/// that length, which misread nothing. ~keep
#[test]
fn a_native_row_reads_the_ocr_rows_nearest_its_place_too() {
    let row = |n: usize, unit: &str| format!("entry {n:03} of the ledger total {:03} {unit}", n * 7);
    let noise = |n: usize| {
        row(n, "cm3")
            .chars()
            .map(|c| if c.is_alphabetic() { 'q' } else { c })
            .collect::<String>()
    };
    let native: Vec<String> = (0..12)
        .map(|n| format!("native line {n:03} only"))
        .chain((0..40).map(|n| row(n, "cm3")))
        .collect();
    let read: Vec<String> = (0..12)
        .map(noise)
        .chain((0..40).map(|n| row(n, "cm\u{2019}")))
        .collect();
    assert!(
        read.iter().all(|text| text.chars().count() == read[0].chars().count()),
        "one length"
    );
    let text = merged(&native, &read);
    assert!(
        !text.contains('\u{2019}'),
        "a misread OCR row stays in the page text:\n{text}"
    );
    for line in &native[12..] {
        assert_eq!(
            text.lines().filter(|printed| printed == line).count(),
            1,
            "{line} prints once"
        );
    }
}

/// A line box at row `row` of a page, one row high. ~keep
fn line_at(text: &str, row: usize) -> LineAt {
    let top = row as f32 * 12.0;
    LineAt::new(
        text,
        LineBox {
            left: 0.0,
            top,
            right: 500.0,
            bottom: top + 10.0,
        },
    )
}

/// The page text after the native rows go first with the given line boxes. ~keep
fn merged_with_boxes(layer: &str, read: &str, boxes: &PageLineBoxes) -> String {
    let thresholds = crate::core::config::OcrQualityThresholds::default();
    let first = layer_lines_to_put_first(layer, read, false, &thresholds, Some(boxes), PlainOcrRows::every())
        .expect("the native row is put first");
    text_with_layer_first(read, &first)
}

/// A boxed OCR row misreads the native row whose box owns it, and the same text is on an earlier
/// OCR line too. The owned line leaves, not the first line with that text. ~keep
#[test]
fn a_boxed_misread_row_leaves_at_its_own_line() {
    let layer = "alpha words here\nbeta words there\ntotal 12 cm3";
    let read = "total 12 cm\u{2019}\nalpha words here\nbeta words there\ntotal 12 cm\u{2019}";
    let boxes = PageLineBoxes {
        layer: vec![
            line_at("alpha words here", 1),
            line_at("beta words there", 2),
            line_at("total 12 cm3", 3),
        ],
        ocr: read.lines().enumerate().map(|(row, text)| line_at(text, row)).collect(),
    };
    assert_eq!(
        merged_with_boxes(layer, read, &boxes),
        "total 12 cm3\n\ntotal 12 cm\u{2019}\nalpha words here\nbeta words there",
        "the OCR line the native box owns leaves; the one above it stays"
    );
}

/// The control of the line test above: one boxed misread line and no copy of it. ~keep
#[test]
fn a_boxed_misread_row_with_one_copy_leaves() {
    let layer = "alpha words here\nbeta words there\ntotal 12 cm3";
    let read = "alpha words here\nbeta words there\ntotal 12 cm\u{2019}";
    let boxes = PageLineBoxes {
        layer: vec![
            line_at("alpha words here", 0),
            line_at("beta words there", 1),
            line_at("total 12 cm3", 2),
        ],
        ocr: read.lines().enumerate().map(|(row, text)| line_at(text, row)).collect(),
    };
    assert_eq!(
        merged_with_boxes(layer, read, &boxes),
        "total 12 cm3\n\nalpha words here\nbeta words there",
        "the owned misread line leaves"
    );
}

/// OCR lines without a box (a recognised table) hold the same misread text twice, after the boxed
/// lines. The native row takes the nearer copy in reading order, and that line of the page text
/// leaves, found through its place among all the lines of the page. ~keep
#[test]
fn an_unboxed_misread_row_leaves_at_its_own_line() {
    let layer = "alpha words here\nbravo words here too\ncharlie and more words\ndelta\necho\nfox\ntotal 12 cm3";
    let read = "alpha words here\nbravo words here too\ncharlie and more words\ntotal 12 cm\u{2019}\nkilo\nlima\n\
                total 12 cm\u{2019}";
    let boxes = PageLineBoxes {
        layer: vec![
            line_at("alpha words here", 0),
            line_at("bravo words here too", 1),
            line_at("charlie and more words", 2),
        ],
        ocr: read
            .lines()
            .take(3)
            .enumerate()
            .map(|(row, text)| line_at(text, row))
            .collect(),
    };
    assert_eq!(
        merged_with_boxes(layer, read, &boxes),
        "delta\necho\nfox\ntotal 12 cm3\n\nalpha words here\nbravo words here too\ncharlie and more words\n\
         total 12 cm\u{2019}\nkilo\nlima",
        "the later unboxed copy leaves; the earlier one stays"
    );
}
