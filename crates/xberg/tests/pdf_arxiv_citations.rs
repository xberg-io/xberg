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

/// The stamp as arXiv sets it: one gray string with two spaces between its parts, 20 points, turned
/// 90 degrees in the left margin and read bottom to top.
fn rotated_stamp() -> String {
    "q 0.5 G 0.5 g BT /F1 20 Tf 0 1 -1 0 32 237 Tm (arXiv:2107.09275v2  [hep-ph]  6 Dec 2023) Tj ET Q\n".to_string()
}

/// The same stamp drawn as three strings far apart along the margin, so the extractor reads its
/// identifier, its category and its date as three paragraphs.
fn split_stamp() -> String {
    split_stamp_at(32.0)
}

/// The three strings of the split stamp, turned 90 degrees with their baselines at `x`.
fn split_stamp_at(x: f32) -> String {
    [
        (150.0, "arXiv:2311.04567v3"),
        (400.0, "[hep-ph]"),
        (520.0, "6 Dec 2023"),
    ]
    .map(|(y, part)| format!("q 0.5 G 0.5 g BT /F1 20 Tf 0 1 -1 0 {x} {y} Tm ({part}) Tj ET Q\n"))
    .concat()
}

fn extract_rotated(output_format: OutputFormat, include_watermarks: bool) -> String {
    extract_with_stamp(&rotated_stamp(), output_format, include_watermarks)
}

fn extract_with_stamp(stamp: &str, output_format: OutputFormat, include_watermarks: bool) -> String {
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
    let content = page_content().replace(&text_at("F1", 9.0, 20.0, 300.0, STAMP), stamp);
    let bytes = build_pdf(&content);
    let result = extract_bytes_document_blocking(&bytes, "application/pdf", &config).expect("fixture must extract");
    collapse(&result.content)
}

#[test]
fn a_rotated_stamp_is_extracted_when_watermarks_are_kept() {
    let content = extract_rotated(OutputFormat::Markdown, true);
    assert!(content.contains(STAMP), "control: {STAMP:?} missing from {content:?}");
}

#[test]
fn markdown_output_removes_a_rotated_stamp() {
    let content = extract_rotated(OutputFormat::Markdown, false);
    assert!(!content.contains("6 Dec 2023"), "{content:?}");
    assert!(!content.contains("arXiv:2107.09275v2 [hep-ph] 6"), "{content:?}");
    assert!(content.contains(BARE_IDENTIFIER_SENTENCE), "{content:?}");
}

#[test]
fn plain_output_removes_a_rotated_stamp() {
    let content = extract_rotated(OutputFormat::Plain, false);
    assert!(!content.contains("6 Dec 2023"), "{content:?}");
    assert!(content.contains(BARE_IDENTIFIER_SENTENCE), "{content:?}");
}

#[test]
fn a_split_stamp_is_extracted_when_watermarks_are_kept() {
    let content = extract_with_stamp(&split_stamp(), OutputFormat::Markdown, true);
    for part in ["arXiv:2311.04567v3", "[hep-ph]", "6 Dec 2023"] {
        assert!(content.contains(part), "control: {part:?} missing from {content:?}");
    }
}

#[test]
fn markdown_output_removes_a_stamp_split_across_paragraphs() {
    let content = extract_with_stamp(&split_stamp(), OutputFormat::Markdown, false);
    assert!(!content.contains("6 Dec 2023"), "{content:?}");
    assert!(!content.contains("arXiv:2311.04567v3"), "{content:?}");
    assert!(content.contains(BARE_IDENTIFIER_SENTENCE), "{content:?}");
    assert!(content.contains(VERSIONED_CITATION), "{content:?}");
}

#[test]
fn plain_output_removes_a_stamp_split_across_paragraphs() {
    let content = extract_with_stamp(&split_stamp(), OutputFormat::Plain, false);
    assert!(!content.contains("6 Dec 2023"), "{content:?}");
    assert!(!content.contains("arXiv:2311.04567v3"), "{content:?}");
    assert!(content.contains(VERSIONED_CITATION), "{content:?}");
}

#[test]
fn a_split_stamp_in_the_middle_of_the_page_is_kept() {
    let stamp = split_stamp_at(300.0);
    let content = extract_with_stamp(&stamp, OutputFormat::Markdown, false);
    for part in ["arXiv:2311.04567v3", "[hep-ph]", "6 Dec 2023"] {
        assert!(content.contains(part), "{part:?} missing from {content:?}");
    }
}

/// First pages of real arXiv papers from the regression page corpus, each with its rotated margin stamp.
const ARXIV_PAGES: [(&str, &str); 3] = [
    ("arxiv-2310.09680v4-p1.pdf", "arXiv:2310.09680v4 [cs.CL] 4 Mar 2024"),
    ("arxiv-2403.01153v1-p1.pdf", "arXiv:2403.01153v1 [eess.SP] 2 Mar 2024"),
    ("arxiv-2405.00399v1-p1.pdf", "arXiv:2405.00399v1 [math.NA] 1 May 2024"),
];

/// The extracted text of a real page, or `None` when the corpus is absent and the local opt-out is set.
/// The pages are bucket-fetched (`python3 test_documents/scripts/fetch_corpus.py`); without them the
/// test fails unless `XBERG_REGRESSION_PAGES_SKIP=1`, which panics when `CI` is set.
fn extract_real_page(name: &str, output_format: OutputFormat, include_watermarks: bool) -> Option<String> {
    let path = helpers::get_test_file_path(&format!("pdf/regression_pages/{name}"));
    if !path.exists() {
        let skip = std::env::var("XBERG_REGRESSION_PAGES_SKIP").is_ok_and(|value| value == "1");
        assert!(
            skip && std::env::var_os("CI").is_none(),
            "{} is missing; fetch the corpus with python3 test_documents/scripts/fetch_corpus.py",
            path.display()
        );
        return None;
    }
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
    let result =
        helpers::extract_uri_document_blocking(&path, Some("application/pdf"), &config).expect("page extracts");
    Some(collapse(&result.content))
}

/// The words of `text`, counted, so two outputs can be compared word for word.
fn word_counts(text: &str) -> std::collections::BTreeMap<&str, usize> {
    let mut counts = std::collections::BTreeMap::new();
    for word in text.split_whitespace() {
        *counts.entry(word).or_insert(0) += 1;
    }
    counts
}

/// The stamp of each real page is removed, and it is the only text removed: every other word of the
/// output with watermarks kept is still there.
///
/// On these three pages the extractor reads the stamp as one paragraph, so they cover the whole-stamp
/// path. None of them reaches the split-stamp path (measured with a print in that path: no page has a
/// partial stamp paragraph), so the real pages cover the whole-stamp path only. The split path is covered by
/// the synthetic page above and the unit tests.
fn assert_real_stamps_removed_alone(output_format: OutputFormat) {
    for (name, stamp) in ARXIV_PAGES {
        let (Some(kept), Some(removed)) = (
            extract_real_page(name, output_format.clone(), true),
            extract_real_page(name, output_format.clone(), false),
        ) else {
            continue;
        };
        assert!(
            kept.contains(stamp),
            "control: {name} has no stamp {stamp:?} when watermarks are kept"
        );
        let identifier = stamp.split(' ').next().expect("identifier");
        assert!(
            !removed.contains(identifier),
            "{name}: the stamp survives in {removed:?}"
        );
        let mut expected = word_counts(&kept);
        for word in stamp.split(' ') {
            if let Some(count) = expected.get_mut(word) {
                *count -= 1;
            }
        }
        expected.retain(|_, count| *count > 0);
        assert_eq!(
            word_counts(&removed),
            expected,
            "{name}: removing the stamp changed other words"
        );
    }
}

#[test]
fn markdown_output_removes_the_rotated_stamp_of_real_arxiv_pages_and_nothing_else() {
    assert_real_stamps_removed_alone(OutputFormat::Markdown);
}

#[test]
fn plain_output_removes_the_rotated_stamp_of_real_arxiv_pages_and_nothing_else() {
    assert_real_stamps_removed_alone(OutputFormat::Plain);
}
