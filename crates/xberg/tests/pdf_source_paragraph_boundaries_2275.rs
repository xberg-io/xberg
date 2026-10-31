#![cfg(feature = "pdf")]

mod helpers;

use helpers::{extract_uri_document_blocking, get_test_file_path};
use sha2::{Digest, Sha256};
use xberg::core::config::{ExtractionConfig, OutputFormat};

fn source_markdown(name: &str, sha256: &str) -> String {
    let path = get_test_file_path(&format!("pdf/regression_pages/{name}.pdf"));
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("mandatory source {}: {error}", path.display()));
    assert!(bytes.starts_with(b"%PDF-"));
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        sha256,
        "visually inspected source identity"
    );
    let config = ExtractionConfig {
        disable_ocr: true,
        use_cache: false,
        enable_quality_processing: false,
        output_format: OutputFormat::Markdown,
        ..Default::default()
    };
    assert!(
        config.pdf_options.is_none(),
        "retain canonical native snapshot defaults"
    );
    let result = extract_uri_document_blocking(&path, Some("application/pdf"), &config)
        .expect("canonical native URI source extraction");
    assert!(!result.content.is_empty());
    result.content
}

fn assert_unique_source_paragraph(markdown: &str, anchor: &str, continuation: &str) {
    assert_eq!(markdown.matches(anchor).count(), 1, "unique source context {anchor:?}");
    let paragraphs: Vec<_> = markdown.split("\n\n").filter(|text| text.contains(anchor)).collect();
    assert_eq!(paragraphs.len(), 1, "one source paragraph containing {anchor:?}");
    let paragraph = paragraphs[0].split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        paragraph.contains(continuation),
        "physical line wrapping must retain the complete source phrase in this paragraph: {paragraph:?}; expected {continuation:?}"
    );
}

#[test]
fn should_keep_following_whole_in_its_source_explanatory_paragraph_2275() {
    let markdown = source_markdown(
        "gutenberg-33283-p27",
        "4d7fe54cedd4daf177144e6b8a7d863b7e071e637d259b4bb4006835c07cf2d3",
    );
    assert_unique_source_paragraph(
        &markdown,
        "In case the reader has no one to guide him",
        "it may here be simply said that one reads differential coefficients in the following way.",
    );
}

#[test]
fn should_keep_distribution_whole_in_amys_source_example_2275() {
    let markdown = source_markdown(
        "irs-p590b--2024-p13",
        "6fd4e6f0f33b777cff7df21204d5286af05c12208d1da38bbf62bebec6af0a58",
    );
    assert_unique_source_paragraph(
        &markdown,
        "and the distribu",
        "and the distribution is made directly by the trustee to a qualified organization",
    );
}

#[test]
fn should_keep_receiving_whole_in_the_source_age_seventy_rule_2275() {
    let markdown = source_markdown(
        "irs-p590b--2024-p6",
        "56678818069de8b9171fbd01b65519679e7df867c22b31eaa397c777cb73228b",
    );
    assert_unique_source_paragraph(
        &markdown,
        "If you were born before July 1, 1949",
        "you were required to begin receiving required minimum distributions by April 1",
    );
}
