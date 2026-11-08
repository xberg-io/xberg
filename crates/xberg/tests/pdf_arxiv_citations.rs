//! A citation that names an arXiv identifier is body text. Only the arXiv margin stamp (identifier with
//! version, category and date) is page noise. The page below holds one stamp, two short reference
//! entries and one sentence that ends in a bare identifier, set in a page dense enough that the furniture
//! safety check does not hide a wrong removal.
#![cfg(all(feature = "pdf", feature = "tokio-runtime"))]

mod helpers;

use helpers::extract_bytes_document_blocking;
use xberg::core::config::{ContentFilterConfig, ExtractionConfig, OutputFormat};

const STAMP: &str = "arXiv:2107.09275v2 [hep-ph] 6 Dec 2023";
const VERSIONED_CITATION: &str = "[12] A. Author, A title, arXiv:2107.09275v2 [hep-ph].";
const UNVERSIONED_CITATION: &str = "[11] J. Doe, A short title, arXiv:2107.09275 [hep-ph].";
const BARE_IDENTIFIER_SENTENCE: &str =
    "The earlier study by Doe and Smith, Journal of Examples 12, 345 (2021), arXiv:2107.09275";
const BODY: &str = "The committee reviewed every proposal and decided that funding should continue for the coming year \
                    while the remaining questions about staffing are settled by the board in the spring.";

fn text_at(font: &str, size: f32, x: f32, y: f32, text: &str) -> String {
    format!("BT /{font} {size} Tf 1 0 0 1 {x} {y} Tm ({text}) Tj ET\n")
}

fn page_content() -> String {
    let mut page = text_at("F2", 18.0, 72.0, 750.0, "Results of the measurement");
    let mut y = 720.0;
    let (first, second) = BODY.split_at(95);
    for _ in 0..9 {
        page += &text_at("F1", 9.0, 72.0, y, first);
        page += &text_at("F1", 9.0, 72.0, y - 11.0, second.trim_start());
        y -= 31.0;
    }
    page += &text_at("F1", 9.0, 72.0, y, BARE_IDENTIFIER_SENTENCE);
    y -= 31.0;
    page += &text_at("F2", 12.0, 72.0, y, "References");
    y -= 22.0;
    page += &text_at("F1", 9.0, 72.0, y, VERSIONED_CITATION);
    y -= 22.0;
    page += &text_at("F1", 9.0, 72.0, y, UNVERSIONED_CITATION);
    // The stamp stands alone in the left margin, as arXiv sets it. ~keep
    page += &text_at("F1", 9.0, 20.0, 300.0, STAMP);
    page
}

/// One Letter page, Helvetica for the body and Helvetica-Bold for the headings, no embedded font.
fn build_pdf(content: &str) -> Vec<u8> {
    let objects: [Vec<u8>; 6] = [
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R \
          /Resources << /Font << /F1 5 0 R /F2 6 0 R >> >> >>"
            .to_vec(),
        [
            format!("<< /Length {} >>\nstream\n", content.len()).into_bytes(),
            content.as_bytes().to_vec(),
            b"endstream".to_vec(),
        ]
        .concat(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold >>".to_vec(),
    ];
    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        pdf.extend_from_slice(object);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    let xref = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
    for offset in &offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    pdf
}

fn extract(output_format: OutputFormat, include_watermarks: bool) -> String {
    let config = ExtractionConfig {
        output_format,
        disable_ocr: true,
        use_cache: false,
        content_filter: Some(ContentFilterConfig {
            include_watermarks,
            ..Default::default()
        }),
        ..Default::default()
    };
    let bytes = build_pdf(&page_content());
    let result = extract_bytes_document_blocking(&bytes, "application/pdf", &config).expect("fixture must extract");
    assert!(
        result.content.contains("Results of the measurement") && result.content.contains("References"),
        "the fixture page did not extract: {}",
        result.content
    );
    result.content
}

fn collapse(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn the_page_carries_every_string_when_watermarks_are_kept() {
    let raw = extract(OutputFormat::Markdown, true);
    let content = collapse(&raw);

    for expected in [
        STAMP,
        VERSIONED_CITATION,
        UNVERSIONED_CITATION,
        BARE_IDENTIFIER_SENTENCE,
    ] {
        let expected = expected.trim_start_matches(|c: char| c == '[' || c == ']' || c.is_ascii_digit() || c == ' ');
        assert!(content.contains(expected), "control: {expected:?} missing from {raw:?}");
    }
}

#[test]
fn markdown_output_keeps_a_short_versioned_citation() {
    let raw = extract(OutputFormat::Markdown, false);
    let content = collapse(&raw);

    assert!(
        content.contains("A. Author, A title, arXiv:2107.09275v2 [hep-ph]."),
        "{raw:?}"
    );
}

#[test]
fn markdown_output_keeps_a_short_unversioned_citation() {
    let raw = extract(OutputFormat::Markdown, false);
    let content = collapse(&raw);

    assert!(
        content.contains("J. Doe, A short title, arXiv:2107.09275 [hep-ph]."),
        "{raw:?}"
    );
}

#[test]
fn markdown_output_keeps_a_sentence_that_ends_at_a_bare_identifier() {
    let raw = extract(OutputFormat::Markdown, false);
    let content = collapse(&raw);

    assert!(content.contains(BARE_IDENTIFIER_SENTENCE), "{raw:?}");
}

#[test]
fn markdown_output_removes_the_margin_stamp() {
    let raw = extract(OutputFormat::Markdown, false);
    let content = collapse(&raw);

    assert!(!content.contains("6 Dec 2023"), "{raw:?}");
    assert!(!content.contains("arXiv:2107.09275v2 [hep-ph] 6"), "{raw:?}");
}

#[test]
fn plain_output_keeps_the_citations_and_removes_the_margin_stamp() {
    let raw = extract(OutputFormat::Plain, false);
    let content = collapse(&raw);

    for expected in [VERSIONED_CITATION, UNVERSIONED_CITATION, BARE_IDENTIFIER_SENTENCE] {
        let expected = expected.trim_start_matches(|c: char| c == '[' || c == ']' || c.is_ascii_digit() || c == ' ');
        assert!(content.contains(expected), "{expected:?} missing from {raw:?}");
    }
    assert!(!content.contains("6 Dec 2023"), "{raw:?}");
}
