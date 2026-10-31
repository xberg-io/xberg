#![cfg(feature = "pdf")]

mod helpers;

use helpers::{extract_bytes_document_blocking, get_test_file_path};
use sha2::{Digest, Sha256};
use xberg::core::config::{ExtractionConfig, OutputFormat, PdfBackend, PdfConfig};
use xberg::types::document_structure::{AnnotationKind, DocumentStructure};

fn source_document(name: &str, sha256: &str) -> xberg::ExtractedDocument {
    let path = get_test_file_path(&format!("pdf/regression_pages/{name}.pdf"));
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("mandatory source {}: {error}", path.display()));
    assert!(bytes.starts_with(b"%PDF-"));
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        sha256,
        "visually inspected source identity"
    );
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        include_document_structure: true,
        disable_ocr: true,
        use_cache: false,
        enable_quality_processing: false,
        pdf_options: Some(PdfConfig {
            backend: PdfBackend::Native,
            extract_tables: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let result = extract_bytes_document_blocking(&bytes, "application/pdf", &config).expect("public native extraction");
    assert!(!result.content.is_empty());
    assert!(
        result
            .document
            .as_ref()
            .is_some_and(|document| !document.nodes.is_empty())
    );
    result
}

fn assert_whole_source_power(document: &DocumentStructure, anchor: &str, base: &str, exponent: &str) {
    let nodes: Vec<_> = document
        .nodes
        .iter()
        .filter(|node| node.content.text().is_some_and(|text| text.contains(anchor)))
        .collect();
    assert_eq!(nodes.len(), 1, "unique source context {anchor:?}");
    let node = nodes[0];
    let text = node.content.text().expect("source context text");
    let anchor_position = text.find(anchor).unwrap();
    let powers: Vec<_> = node
        .annotations
        .iter()
        .filter(|annotation| annotation.kind == AnnotationKind::Superscript)
        .filter(|annotation| annotation.start as usize >= anchor_position)
        .filter_map(|annotation| {
            let start = annotation.start as usize;
            let end = annotation.end as usize;
            let power = text.get(start..end).expect("valid script UTF-8 offsets");
            (power == exponent && text[..start].trim_end().ends_with(base)).then_some(power)
        })
        .collect();
    assert_eq!(
        powers,
        [exponent],
        "whole source power {base}^({exponent}), not a detached remainder: {text}"
    );
}

fn public_text(document: &DocumentStructure) -> String {
    document
        .nodes
        .iter()
        .filter_map(|node| node.content.text())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn should_keep_fractional_source_exponent_as_one_attached_power_2275() {
    let result = source_document(
        "gutenberg-38769-p338",
        "222601df883b0bacf319e7eb305cc5abe3ce37d07b31f3042a15a717eac3f9d0",
    );
    assert!(result.content.contains("Mean Value Theorem"));
    assert_whole_source_power(&result.document.unwrap(), "If x = −a", "x", "1/3");
}

#[test]
fn should_keep_subtracted_source_exponent_as_one_attached_power_2275() {
    let result = source_document(
        "gutenberg-38769-p412",
        "0434635c502acb1a0ea50aa92c90111f54d49a679718061bb35b0091191f26b5",
    );
    assert!(result.content.contains("Integral Test"));
    assert_whole_source_power(&result.document.unwrap(), "Compare with", "n", "s−t");
}

#[test]
fn should_retain_both_source_derivative_coefficients_on_exercise_answer_page_2275() {
    let result = source_document(
        "gutenberg-33283-p265",
        "805a2818d433265d910a3a5ae18732aed4ffecf980290079b5b04dde71a0b211",
    );
    assert!(result.content.contains("Exercises IV."));
    for coefficient in ["3240", "1944"] {
        assert_eq!(
            result.content.matches(coefficient).count(),
            1,
            "retain source derivative coefficient {coefficient}"
        );
    }
}

#[test]
fn should_preserve_source_prose_spacing_without_duplicating_display_exercises_2275() {
    let result = source_document(
        "gutenberg-38769-p458",
        "f073871cd757fe2c2e29b4da24f1d92c42bd3ded897653d39e6f10791a8f90f0",
    );
    let text = public_text(result.document.as_ref().unwrap());
    assert_eq!(
        text.matches("where a and b are positive").count(),
        2,
        "both source exercise premises retain word boundaries"
    );
    assert_eq!(
        result.content.matches("Prove the formula").count(),
        1,
        "source exercise33 occurs once across prose and tables"
    );
}

#[test]
fn should_preserve_polynomial_prose_word_boundaries_on_series_page_2275() {
    let result = source_document(
        "gutenberg-38769-p490",
        "167db3babb23da236cc4593ab155fdf8fe4e78732d7f4ad18d31929978cfaaca",
    );
    let text = public_text(result.document.as_ref().unwrap());
    assert!(
        text.contains("is a polynomial of degree r in n."),
        "source polynomial definition keeps ordinary spaces: {text}"
    );
    assert!(
        text.contains("the last series is equal to zero when x = −2."),
        "source conclusion keeps prose and inline variable boundaries: {text}"
    );
    assert_eq!(result.content.matches("Multiply numerator and denominator").count(), 1);
}
