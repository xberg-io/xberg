#![cfg(feature = "pdf")]

mod helpers;

use helpers::{extract_bytes_document_blocking, get_test_documents_dir};
use sha2::{Digest, Sha256};
use xberg::{ExtractionConfig, OutputFormat};

fn ambiguity_page(name: &str, hash: &str) -> String {
    let root = std::env::var_os("XBERG_SCRIPT_RETENTION_FIXTURE_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(get_test_documents_dir);
    let bytes = std::fs::read(root.join("pdf/regression_pages").join(name)).expect("mandatory ambiguity fixture");
    assert_eq!(hex::encode(Sha256::digest(&bytes)), hash, "exact source PDF identity");
    let config = ExtractionConfig {
        disable_ocr: true,
        use_cache: false,
        enable_quality_processing: false,
        output_format: OutputFormat::Markdown,
        ..Default::default()
    };
    let result =
        extract_bytes_document_blocking(&bytes, "application/pdf", &config).expect("actual ambiguity extraction");
    if let Some(directory) = std::env::var_os("XBERG_SCRIPT_AMBIGUITY_RECEIPTS") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).expect("actual receipt directory");
        std::fs::write(
            directory.join(format!("{name}.result.json")),
            serde_json::to_vec_pretty(&result).expect("serialize actual document"),
        )
        .expect("write actual document");
    }
    result.content
}

#[test]
fn should_not_annotate_sine_fraction_numerators_as_exponents() {
    let text = ambiguity_page(
        "arxiv-2405.00569v2-p9.pdf",
        "92339e619eb9397ffe4550f1b1342cafe7b2b70f90ba13e8b07b048d8216ae64",
    );
    assert!(text.contains("sin("), "the source formula must be present");
    assert!(
        !text.contains("sin(<sup>ω</sup>)"),
        "a fraction numerator is not a proved exponent"
    );
}

#[test]
fn should_not_annotate_mixed_fraction_numerators_as_exponents() {
    let text = ambiguity_page(
        "gutenberg-36670-p169.pdf",
        "70ae20eae453381732868eb65ef868bea7bbd3c36e2774401ccb52e2b1539c0e",
    );
    assert!(
        text.contains("4") && text.contains("%"),
        "the mixed rate must be present"
    );
    assert!(
        !text.contains("4<sup>1</sup>"),
        "a mixed fraction numerator is not an exponent"
    );
}

#[test]
fn should_not_annotate_parenthesized_fraction_numerators_as_exponents() {
    let text = ambiguity_page(
        "gutenberg-38769-p500.pdf",
        "950d23bb5d8f1c47c6a8cca4e23d2230f883a12b4dbf5cf12793f1f9c1512a4b",
    );
    assert!(text.contains("Determine which"), "the source problem must be present");
    assert!(
        !text.contains("(<sup>1</sup>"),
        "a parenthesized fraction numerator is not an exponent"
    );
}

#[test]
fn should_not_omit_a_decimal_point_from_a_new_exponent_annotation() {
    let text = ambiguity_page(
        "ntrs-20040081236-p13.pdf",
        "71cd7420f93701108312f7b8bd1ba0b1c55665386e1e4051ae2a05782e639308",
    );
    assert!(text.contains("Log Re"), "the denominator must be present");
    assert!(
        !text.contains("<sup>258</sup>"),
        "a decimal exponent must be complete or remain unassociated"
    );
}
