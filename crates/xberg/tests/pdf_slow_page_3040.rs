//! Regression test for issue #3040: one page of `bayesian_data_analysis` took over 12 s in a
//! debug build, so the extraction time limit's stop never reached a page boundary inside the
//! grace. The page's cost was a content-stream tokenizer that compared the whole remaining stream
//! once per token, which makes the page cost quadratic in the stream length.
//!
//! The check is a ratio, not a time: the same page is read with a content stream of `n` and of
//! `4n` operators, and quadrupling the stream must roughly quadruple the time. A quadratic page
//! multiplies it by 16 before fixed costs dilute that, and a slow or loaded runner scales both readings alike.

// The cost is only visible unoptimized: an optimized build reads either size in milliseconds, where
// timer noise outweighs the ratio.
#![cfg(all(feature = "pdf", debug_assertions))]

use std::fmt::Write as _;
use std::time::Instant;
use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::document::ReadingOrder;

/// Filler pairs at the smaller size. Four times this stays under the 256 KiB at which the text-only
/// parser switches to its pre-scan, which would skip the per-token path under test.
const OPERATORS: usize = 4_000;
/// The larger stream is this many times the smaller one.
const STEP: usize = 4;
/// Linear growth measures 4.0 and the old tokenizer measured 8.3 to 8.6 (the rest of the page is
/// linear and dilutes the quadratic part), so the limit sits between them.
const MAX_RATIO: f64 = 6.0;
const READINGS: usize = 5;
const PRESCAN_THRESHOLD_BYTES: usize = 256 * 1024;

/// One page whose content stream holds a line of text followed by `filler_pairs` empty marked-content
/// pairs inside the text object. Their dictionary operand is outside the text parser's fast path, so
/// each one goes through the general tokenizer, which is the code #3040 made quadratic. The stream
/// length is the only thing that varies between sizes.
fn single_page_pdf(filler_pairs: usize) -> Vec<u8> {
    let mut stream = String::from("BT /F1 12 Tf 72 700 Td (Slow page) Tj\n");
    for _ in 0..filler_pairs {
        stream.push_str("/P<<>>BDC EMC ");
    }
    stream.push_str("ET");
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R \
         /Resources << /Font << /F1 5 0 R >> >> >>"
            .to_string(),
        format!("<< /Length {} >>\nstream\n{stream}\nendstream", stream.len()),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
    ];
    let mut pdf = String::from("%PDF-1.4\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        write!(pdf, "{} 0 obj\n{body}\nendobj\n", index + 1).unwrap();
    }
    let xref_at = pdf.len();
    write!(pdf, "xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).unwrap();
    for offset in offsets {
        writeln!(pdf, "{offset:010} 00000 n ").unwrap();
    }
    write!(
        pdf,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
        objects.len() + 1
    )
    .unwrap();
    pdf.into_bytes()
}

/// Fastest of `READINGS` full opens-and-reads of the page, in seconds.
fn read_page_seconds(filler_pairs: usize) -> f64 {
    (0..READINGS)
        .map(|_| {
            let document = PdfDocument::from_bytes(single_page_pdf(filler_pairs)).expect("open the generated PDF");
            let started = Instant::now();
            let spans = document
                .extract_spans_with_reading_order(0, ReadingOrder::ColumnAware)
                .expect("read the page");
            let took = started.elapsed().as_secs_f64();
            assert!(!spans.is_empty(), "the generated page read no text");
            took
        })
        .fold(f64::INFINITY, f64::min)
}

#[test]
fn test_page_with_a_large_content_stream_reads_in_linear_time() {
    assert!(
        single_page_pdf(STEP * OPERATORS).len() < PRESCAN_THRESHOLD_BYTES,
        "the larger stream must stay under the pre-scan threshold"
    );
    let single = read_page_seconds(OPERATORS);
    let double = read_page_seconds(STEP * OPERATORS);
    let ratio = double / single;
    assert!(
        ratio < MAX_RATIO,
        "{STEP}x the content stream multiplied the page time by {ratio:.2} ({single:.3}s -> {double:.3}s): the cost is not linear"
    );
}
