#![cfg(all(feature = "ocr", feature = "bundle-tessdata-eng", feature = "tokio-runtime"))]
#![allow(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)] // ~keep: org logging policy exempts tests

//! #2503: the low-ppi table cases still open after the exact-row claim keep their measured floors.
//!
//! The claim landed for the lossless 150 dpi six-column picture and for the 150 dpi dot-leader
//! scans. What stayed open was the JPEG q45 six-column picture and the 7 px tinted picture: both
//! reach OCR at 9 px or below and lose digits. Measured on main at 281f71511c and reproduced at
//! 26549dc005 after the table-render commits, every row missing from the output is a *recognition*
//! loss -- its cells are still on one line, one or two of them read wrong (`111` -> `211`,
//! `155` -> `185`, `147`+`148` -> `147148`) -- never a row split across lines. The six-column q45
//! picture at PSM 11 adds two leader artifacts (`11` -> `Li`, a leader -> `ee`). The floors below
//! pin that shape so a structure regression in any case fails.
//!
//! Recognition at these resolutions belongs to the resample decision of #2327, not to row
//! structure: all three rasters take the default preprocessing and the 300 DPI resample (scale 2.4
//! for the 150 dpi JPEG, 3.0 for the 100 dpi ones) and still lose the smallest glyphs. Tuning a
//! recognizer threshold here would hide that, so the test records the counts instead.

mod helpers;

use sha2::{Digest, Sha256};
use xberg::types::{ImagePreprocessingMetadata, TesseractConfig};
use xberg::{ExtractedDocument, ExtractionConfig, OcrConfig, OutputFormat, PageConfig};

const SIX_COLUMN_LOSSLESS: &[u8] = include_bytes!("fixtures/ocr_six_column_rows_2503/six-column-150dpi.png");
const SIX_COLUMN_Q45: &[u8] = include_bytes!("fixtures/ocr_ppi_rows_2503/six-column-150dpi-q45.jpg");
const SMALL_SIX_COLUMN_Q45: &[u8] = include_bytes!("fixtures/ocr_ppi_rows_2503/six-column-100dpi-9px-q45.jpg");
const TINTED_100_DPI_7PX: &[u8] = include_bytes!("fixtures/ocr_low_ppi_tinted_3337/tinted-100-7px.png");

/// The #2503 six-column generator rows: one label and five values per row.
const SIX_COLUMN_ROWS: &str = include_str!("fixtures/ocr_ppi_rows_2503/rows.tsv");
/// The #3337 low-ppi grid rows: six three-digit numbers per row, 101..172.
const NUMBER_ROWS: &str = include_str!("fixtures/ocr_ppi_rows_2503/number-rows.tsv");

/// Rows kept whole on one line, the acceptance metric of #2503.
///
/// Values are the counts measured on main at 281f71511c (vendored Tesseract 5.5.4, macOS arm64).
/// A structure regression (a row split across lines, or a row printed twice) takes a row away from
/// these counts; a recognizer that reads one more glyph differently does too, which forces a
/// re-measure instead of silent drift.
struct Case {
    label: &'static str,
    bytes: &'static [u8],
    mime: &'static str,
    sha256: &'static str,
    rows: &'static str,
    number_grid: bool,
    floors: &'static [(Format, i32, usize)],
}

#[derive(Clone, Copy)]
enum Format {
    Plain,
    Markdown,
}

impl Format {
    fn output(self) -> OutputFormat {
        match self {
            Self::Plain => OutputFormat::Plain,
            Self::Markdown => OutputFormat::Markdown,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Plain => "plain",
            Self::Markdown => "markdown",
        }
    }
}

const CASES: &[Case] = &[
    Case {
        label: "six-column 150dpi lossless png",
        bytes: SIX_COLUMN_LOSSLESS,
        mime: "image/png",
        sha256: "b369ad154e8ea715b10db8e23cc6ced72820cef0a0d623f5aa46190ffe9eb803",
        rows: SIX_COLUMN_ROWS,
        number_grid: false,
        floors: &[
            (Format::Plain, 6, 12),
            (Format::Plain, 11, 12),
            (Format::Markdown, 6, 12),
            (Format::Markdown, 11, 12),
        ],
    },
    Case {
        label: "six-column 150dpi q45 jpeg",
        bytes: SIX_COLUMN_Q45,
        mime: "image/jpeg",
        sha256: "d7b2f02032199e7d9506fe9036aca8613a60b4c4efd068d9727a54d7af1fa11e",
        rows: SIX_COLUMN_ROWS,
        number_grid: false,
        floors: &[
            (Format::Plain, 6, 12),
            (Format::Plain, 11, 10),
            (Format::Markdown, 6, 12),
            (Format::Markdown, 11, 10),
        ],
    },
    Case {
        label: "six-column 100dpi 9px q45 jpeg",
        bytes: SMALL_SIX_COLUMN_Q45,
        mime: "image/jpeg",
        sha256: "fe21997298f943469ca774b3125136f767ca5bdf7b69a9214eab8795aedf36ff",
        rows: NUMBER_ROWS,
        number_grid: true,
        floors: &[
            (Format::Plain, 6, 11),
            (Format::Plain, 11, 12),
            (Format::Markdown, 6, 11),
            (Format::Markdown, 11, 12),
        ],
    },
    Case {
        label: "tinted 100dpi 7px png",
        bytes: TINTED_100_DPI_7PX,
        mime: "image/png",
        sha256: "5af0e10e0a92130a61534e15ccd42f85aa052b3726bdfd09004a3c9b37e334e3",
        rows: NUMBER_ROWS,
        number_grid: true,
        floors: &[
            (Format::Plain, 6, 5),
            (Format::Plain, 11, 11),
            (Format::Markdown, 6, 5),
            (Format::Markdown, 11, 11),
        ],
    },
];

fn config(format: Format, psm: i32) -> ExtractionConfig {
    ExtractionConfig {
        use_cache: false,
        output_format: format.output(),
        ocr: Some(OcrConfig {
            tesseract_config: Some(TesseractConfig {
                psm: Some(psm),
                ..Default::default()
            }),
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn extract(case: &Case, format: Format, psm: i32) -> ExtractedDocument {
    let document = helpers::extract_bytes_document_blocking(case.bytes, case.mime, &config(format, psm)).unwrap();
    assert!(!document.content.trim().is_empty(), "{} must be readable", case.label);
    document
}

fn preprocessing(document: &ExtractedDocument) -> ImagePreprocessingMetadata {
    document
        .metadata
        .image_preprocessing
        .clone()
        .or_else(|| {
            document
                .pages
                .as_deref()
                .and_then(|pages| pages.first())
                .and_then(|page| page.image_preprocessing.clone())
        })
        .expect("a page-like raster must take the default preprocessing")
}

/// Whether `line` carries the source row: the label (trailing leader dots trimmed) and the five
/// values in order, as `ocr_six_column_html_2503.rs` compares them.
fn leader_row_matches(line: &str, source: &[&str]) -> bool {
    let [label, values @ ..] = source else {
        return false;
    };
    let flattened = line.replace('|', " ");
    let words: Vec<_> = flattened.split_whitespace().collect();
    let Some(split) = words.len().checked_sub(values.len()) else {
        return false;
    };
    let (actual_label, actual_values) = words.split_at(split);
    let end = actual_label
        .iter()
        .rposition(|word| {
            !word
                .chars()
                .all(|character| matches!(character, '.' | '\u{b7}' | '_' | '\u{2026}' | '='))
        })
        .map_or(0, |at| at + 1);
    let actual_label = actual_label[..end].join(" ");
    actual_values == values && actual_label.trim_end_matches('.').trim_end() == *label
}

/// Whether `line` is exactly the source row of a number grid: six digit tokens, in order.
fn number_row_matches(line: &str, values: &[&str]) -> bool {
    let tokens: Vec<String> = line
        .split_whitespace()
        .map(|word| word.chars().filter(char::is_ascii_digit).collect::<String>())
        .filter(|token| !token.is_empty())
        .collect();
    tokens == values
}

fn rows_kept(document: &ExtractedDocument, case: &Case) -> usize {
    case.rows
        .lines()
        .filter(|row| {
            let cells: Vec<_> = row.split('\t').collect();
            document
                .content
                .lines()
                .filter(|line| {
                    if case.number_grid {
                        number_row_matches(line, &cells)
                    } else {
                        leader_row_matches(line, &cells)
                    }
                })
                .count()
                == 1
        })
        .count()
}

fn assert_rows_kept(document: &ExtractedDocument, case: &Case, format: Format, psm: i32, floor: usize) -> usize {
    let kept = rows_kept(document, case);
    println!(
        "[2503] {:<32} {:<8} psm {:>2}: {kept:>2}/12 rows kept (floor {floor})",
        case.label,
        format.name(),
        psm
    );
    assert!(
        kept >= floor,
        "{} {} psm {psm}: kept {kept} of 12 rows, floor {floor}.\n{}",
        case.label,
        format.name(),
        document.content
    );
    kept
}

#[test]
fn low_ppi_table_rows_keep_their_measured_floors() {
    for case in CASES {
        assert_eq!(
            hex::encode(Sha256::digest(case.bytes)),
            case.sha256,
            "{}: fixture bytes",
            case.label
        );
        assert_eq!(case.rows.lines().count(), 12, "{}: fixture rows", case.label);
        for (format, psm, floor) in case.floors {
            let document = extract(case, *format, *psm);
            let metadata = preprocessing(&document);
            assert!(
                !metadata.skipped_resize,
                "{}: a low-ppi raster must reach the recognizer resampled, not at its own resolution",
                case.label
            );
            assert!(
                metadata.scale_factor > 1.0,
                "{}: the resample must upscale, got {}",
                case.label,
                metadata.scale_factor
            );
            assert_rows_kept(&document, case, *format, *psm, *floor);
            let pages = document.pages.as_deref().expect("requested page output");
            let [page] = pages else {
                panic!("{}: one source page", case.label)
            };
            assert!(
                !page.content.trim().is_empty(),
                "{}: the source page must publish its transcript",
                case.label
            );
        }
    }
}
