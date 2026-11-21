#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "bundle-tessdata-eng",
    feature = "tokio-runtime"
))]

//! #3337: replay the 100 DPI tinted table on the scan route, then record what each scan keeps.
//!
//! The resample fixtures cover the PNG route: the raster goes straight to OCR. The reporter's
//! scans are PDFs, so this replay wraps the same rasters into one-page 100 DPI PDFs and runs
//! PDF raster -> OCR at PSM 11. Each of the twelve source rows must come back as one output line
//! holding that row's six numbers in order; a count alone cannot tell a recovered row from six
//! numbers that survived in the wrong row.
//!
//! The reporter's four scans (11/12, 10/12, 0/12, 11/12 rows on the CI host) are not in the
//! repository. These fixtures are not those scans: they are the #3337 table shape rendered on
//! the reporter's tint. See the fixture README for the measured local baseline.

mod helpers;

use sha2::{Digest, Sha256};
use xberg::{ExtractedDocument, ExtractionConfig, OcrConfig, OutputFormat, PageConfig, TesseractConfig};

const TINTED_100_DPI_9PX: &[u8] = include_bytes!("fixtures/ocr_low_ppi_tinted_3337/tinted-100-9px.pdf");
const TINTED_100_DPI_7PX: &[u8] = include_bytes!("fixtures/ocr_low_ppi_tinted_3337/tinted-100-7px.pdf");

const ROWS: usize = 12;
const COLUMNS: usize = 6;

/// The generator writes 101..=172 into six columns by twelve rows, in reading order.
fn source_rows() -> Vec<Vec<u16>> {
    (0..ROWS)
        .map(|row| {
            (0..COLUMNS)
                .map(|column| 101 + (row * COLUMNS + column) as u16)
                .collect()
        })
        .collect()
}

/// Rows of 12 that must survive as intact rows on a scan, at PSM 11.
///
/// Measured baseline on the scan route (vendored Tesseract 5.5.4, macOS arm64):
///
/// - `tinted-100-9px.pdf`: 12 of 12 rows, 72 of 72 numbers.
/// - `tinted-100-7px.pdf`: 7 of 12 rows, 66 of 72 numbers. The scan route renders a 100 DPI
///   page at the 150 DPI default render floor before the OCR preprocessor resamples it, so a
///   7 px glyph reaches Tesseract through 1.5x then 2x interpolation; one or two digits of a
///   row come back wrong while the row itself stays a line. The 9 px scan keeps its digits, so
///   the reporter's 100 DPI scans are represented by that fixture and this one is the
///   measured floor for the smaller rendering.
///
/// The reporter's four 100 DPI tinted scans kept 11, 10, 0 and 11 rows on the CI host, so a
/// host that still degrades loses rows against these floors.
const KEPT_ROWS_100_DPI_9PX: usize = 12;
const KEPT_ROWS_100_DPI_7PX: usize = 7;

fn fixture(bytes: &[u8], sha256: &str) -> Vec<u8> {
    assert_eq!(hex::encode(Sha256::digest(bytes)), sha256);
    bytes.to_vec()
}

fn extract_scan(bytes: &[u8]) -> ExtractedDocument {
    let config = ExtractionConfig {
        use_cache: false,
        output_format: OutputFormat::Plain,
        ocr: Some(OcrConfig {
            tesseract_config: Some(TesseractConfig {
                psm: Some(11),
                ..Default::default()
            }),
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let extracted = helpers::extract_bytes_document_blocking(bytes, "application/pdf", &config).unwrap();
    assert!(!extracted.content.is_empty(), "the scan must be readable");
    extracted
}

fn numbers_on(line: &str) -> Vec<u16> {
    line.split(|character: char| !character.is_ascii_digit())
        .filter(|token| !token.is_empty())
        .filter_map(|token| token.parse().ok())
        .collect()
}

/// A row survives when one output line holds its six numbers in order, as whole tokens.
fn row_survives(lines: &[Vec<u16>], row: &[u16]) -> bool {
    lines.iter().any(|line| {
        let mut wanted = row.iter();
        let mut next = wanted.next();
        for number in line {
            if Some(number) == next {
                next = wanted.next();
                if next.is_none() {
                    return true;
                }
            }
        }
        false
    })
}

fn surviving_rows(text: &str) -> Vec<usize> {
    let lines: Vec<Vec<u16>> = text.lines().map(numbers_on).collect();
    source_rows()
        .iter()
        .enumerate()
        .filter(|(_, row)| row_survives(&lines, row))
        .map(|(index, _)| index)
        .collect()
}

/// Source numbers that came back as their own token anywhere on the scan.
fn kept_numbers(text: &str) -> usize {
    let tokens: std::collections::HashSet<u16> = text.lines().flat_map(numbers_on).collect();
    source_rows()
        .iter()
        .flatten()
        .filter(|number| tokens.contains(number))
        .count()
}

fn assert_rows_preserved(text: &str, minimum: usize, label: &str) -> usize {
    let rows = surviving_rows(text);
    println!(
        "#3337 scan replay: {label}: {} of {ROWS} rows preserved, {} of {} numbers (rows {rows:?})",
        rows.len(),
        kept_numbers(text),
        ROWS * COLUMNS
    );
    assert!(
        rows.len() >= minimum,
        "{label}: kept {} of {ROWS} rows, floor {minimum} (rows {rows:?})\n{text}",
        rows.len()
    );
    rows.len()
}

#[test]
fn nine_pixel_tinted_scan_keeps_every_source_row() {
    let bytes = fixture(
        TINTED_100_DPI_9PX,
        "1ed4461fede583c8cd030faeb5dc2e24979f91f224f755409c6e85cd1bfca81f",
    );
    let document = extract_scan(&bytes);

    let kept = assert_rows_preserved(&document.content, KEPT_ROWS_100_DPI_9PX, "tinted-100-9px.pdf");
    if let Some(pages) = document.pages.as_deref() {
        let [page] = pages else { panic!("one source page") };
        assert_eq!(
            assert_rows_preserved(&page.content, KEPT_ROWS_100_DPI_9PX, "tinted-100-9px.pdf page"),
            kept
        );
    }
}

#[test]
fn seven_pixel_tinted_scan_keeps_its_measured_baseline() {
    let bytes = fixture(
        TINTED_100_DPI_7PX,
        "79e795ac533bd27156e59870dfaca2d02aa7ba1917384b2fb763d4de2f6dadf3",
    );
    let document = extract_scan(&bytes);

    assert_rows_preserved(&document.content, KEPT_ROWS_100_DPI_7PX, "tinted-100-7px.pdf");
}
