//! A word must not be cut where a producer sets the text matrix back over a run it already drew and
//! reaches the next glyph with a large leading `TJ` offset (GH#2322).
//!
//! The producer draws each run with `Td`, draws a ligature in a second font, and then sets `Tm` to the
//! start of an earlier run. The array `[-K (rip)] TJ` moves the pen from there to the next free place.
//! The offset is not a gap between two drawn glyphs, so it must not put a space into the word under the
//! `Tm` anchor. ~keep

#![cfg(feature = "pdf")]

mod helpers;

use helpers::extract_bytes_document_blocking;
use xberg::core::config::{ExtractionConfig, OutputFormat};

/// Advance of each glyph in thousandths of an em (the `/Widths` below), from a common sans serif face.
const SPACE: f32 = 318.0;
const FFI: f32 = 967.0;
const FI: f32 = 630.0;

fn letter_width(c: char) -> f32 {
    match c {
        ' ' => SPACE,
        '\u{80}' => FFI,
        '\u{81}' => FI,
        'f' => 352.0,
        'i' => 278.0,
        't' => 392.0,
        'r' => 411.0,
        's' => 521.0,
        'c' => 550.0,
        'a' | 'o' => 612.0,
        'e' => 615.0,
        'n' => 634.0,
        _ => 635.0,
    }
}

/// Where the matrix is set before the array, as (run index, extra thousandths of an em).
enum Lead {
    /// `Tm` to the start of a run plus an extra amount, then a lead that reaches the next free place.
    Anchor(usize, f32),
    /// No `Tm`: the array follows the runs directly, with this lead.
    Plain(f32),
}

/// The geometry of the line: the size in `Tm` with `1 Tf`, or the size in `Tf` with an unscaled `Tm`.
#[derive(Clone, Copy)]
enum Size {
    InMatrix,
    InFont,
}

fn width_of(run: &str) -> f32 {
    run.chars().map(letter_width).sum()
}

/// One line of runs `(font, text)` drawn with `Td` from each run start, then `tail` drawn the same way,
/// then the array `[lead (rip)]` and `( more)`. A code `\x80` is the ffi ligature and `\x81` the fi
/// ligature, both in the second font.
fn line_stream(runs: &[(&str, &str)], tail: &str, lead: Lead, size: Size) -> String {
    const POINTS: f32 = 11.0;
    let (font_size, scale) = match size {
        Size::InMatrix => (1.0, POINTS),
        Size::InFont => (POINTS, 1.0),
    };
    let mut ops = vec![
        "BT /F1 11 Tf 72 740 Td (A plain line of text comes first on the page.) Tj ET".to_string(),
        format!("BT {scale} 0 0 {scale} 72 700 Tm"),
    ];
    let mut starts = Vec::new();
    let mut pen = 0.0_f32;
    for (font, text) in runs.iter().copied().chain([("F1", tail)]) {
        let advance = width_of(text) / 1000.0 * font_size;
        let escaped: String = text
            .chars()
            .map(|c| match c {
                '\u{80}' => "\\200".to_string(),
                '\u{81}' => "\\201".to_string(),
                _ => c.to_string(),
            })
            .collect();
        ops.push(format!("/{font} {font_size} Tf ({escaped}) Tj {advance:.5} 0 Td"));
        starts.push(pen);
        pen += advance;
    }
    match lead {
        Lead::Anchor(run, extra) => {
            let anchor = starts[run] + extra / 1000.0 * font_size;
            let kern = -(pen - anchor) / font_size * 1000.0;
            let x = 72.0 + anchor * scale;
            ops.push(format!("{scale} 0 0 {scale} {x:.4} 700 Tm [{kern:.1} (rip)] TJ"));
        }
        Lead::Plain(kern) => ops.push(format!("[{kern} (rip)] TJ")),
    }
    ops.push("( more) Tj ET".to_string());
    ops.push("BT /F1 11 Tf 72 660 Td (A last plain line ends the page.) Tj ET".to_string());
    ops.join("\n")
}

fn pdf_with(stream: &str) -> Vec<u8> {
    let letters = (32u8..=126)
        .map(|code| letter_width(char::from(code)).to_string())
        .collect::<Vec<_>>()
        .join(" ");
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R \
         /Resources << /Font << /F1 5 0 R /F2 6 0 R >> >> >>"
            .to_owned(),
        format!("<< /Length {} >>\nstream\n{stream}\nendstream", stream.len() + 1),
        format!(
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /FirstChar 32 /LastChar 126 \
             /Widths [{letters}] >>"
        ),
        format!(
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /FirstChar 128 /LastChar 129 /Widths [{FFI} {FI}] \
             /Encoding << /Type /Encoding /Differences [128 /ffi /fi] >> >>"
        ),
    ];
    let mut bytes = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n{object}\nendobj\n", index + 1).as_bytes());
    }
    let xref = bytes.len();
    bytes.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
    for offset in offsets {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    bytes
}

fn words_of(stream: &str) -> Vec<String> {
    let config = ExtractionConfig {
        output_format: OutputFormat::Plain,
        disable_ocr: true,
        use_cache: false,
        ..Default::default()
    };
    let result = extract_bytes_document_blocking(&pdf_with(stream), "application/pdf", &config)
        .expect("a self-contained native PDF");
    result.content.split_whitespace().map(str::to_string).collect()
}

/// Every word in `expected` is a whole word of the page text, in each geometry of the line.
fn assert_whole_words(runs: &[(&str, &str)], tail: &str, lead: impl Fn() -> Lead, expected: &[&str]) {
    let mut failures = Vec::new();
    for (size, name) in [(Size::InMatrix, "size in Tm"), (Size::InFont, "size in Tf")] {
        let words = words_of(&line_stream(runs, tail, lead(), size));
        assert!(
            words.iter().any(|w| w == "plain"),
            "{name}: the page text holds no plain line: {words:?}"
        );
        let missing: Vec<_> = expected
            .iter()
            .filter(|word| !words.iter().any(|w| w == *word))
            .collect();
        if !missing.is_empty() {
            failures.push(format!("{name}: {missing:?} not whole words of {words:?}"));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

const OFFICE: &[(&str, &str)] = &[
    ("F1", "the "),
    ("F1", "o"),
    ("F2", "\u{80}"),
    ("F1", "ce"),
    ("F1", " and "),
];

#[test]
fn leading_offset_after_tm_at_the_run_after_a_ligature_keeps_the_word_whole() {
    let expected = ["the", "office", "and", "strip", "more"];
    assert_whole_words(OFFICE, "st", || Lead::Anchor(3, 0.0), &expected);
}

#[test]
fn leading_offset_after_tm_at_a_ligature_keeps_the_word_whole() {
    let runs = [
        ("F1", "the "),
        ("F1", "bene"),
        ("F2", "\u{81}"),
        ("F1", "ts"),
        ("F1", " and "),
    ];
    let expected = ["the", "benefits", "and", "strip", "more"];
    assert_whole_words(&runs, "st", || Lead::Anchor(2, 0.0), &expected);
}

/// The anchor sits at a real word gap: the words on each side stay apart.
#[test]
fn leading_offset_after_tm_at_a_word_gap_keeps_the_gap() {
    let expected = ["the", "office", "and", "strip", "more"];
    assert_whole_words(OFFICE, "st", || Lead::Anchor(4, SPACE), &expected);
}

/// No `Tm`: a leading offset after `Td` is a real gap between the last drawn word and the next one.
#[test]
fn leading_offset_without_tm_keeps_the_gap() {
    let runs = [("F1", "the "), ("F1", "word")];
    assert_whole_words(&runs, "", || Lead::Plain(-300.0), &["the", "word", "rip", "more"]);
}
