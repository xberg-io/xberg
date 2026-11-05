#![cfg(feature = "pdf")]

mod helpers;

use helpers::{extract_bytes_document_blocking, get_test_documents_dir};
use sha2::{Digest, Sha256};
use xberg::types::document_structure::AnnotationKind;
use xberg::{ExtractionConfig, OutputFormat};

fn ambiguity_page(name: &str, hash: &str) -> xberg::ExtractedDocument {
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
        include_document_structure: true,
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
    result
}

fn superscripts<'a>(result: &'a xberg::ExtractedDocument, context: &str) -> Vec<&'a str> {
    let document = result.document.as_ref().expect("requested actual document structure");
    let nodes: Vec<_> = document
        .nodes
        .iter()
        .filter(|node| node.content.text().is_some_and(|text| text.contains(context)))
        .collect();
    assert!(!nodes.is_empty(), "the actual structured formula context must survive");
    nodes
        .into_iter()
        .flat_map(|node| {
            let text = node.content.text().expect("selected textual node");
            node.annotations
                .iter()
                .filter(|annotation| matches!(annotation.kind, AnnotationKind::Superscript))
                .map(move |annotation| {
                    text.get(annotation.start as usize..annotation.end as usize)
                        .expect("actual annotation must have valid UTF-8 byte offsets")
                })
        })
        .collect()
}

#[test]
fn should_not_annotate_sine_fraction_numerators_as_exponents() {
    let result = ambiguity_page(
        "arxiv-2405.00569v2-p9.pdf",
        "92339e619eb9397ffe4550f1b1342cafe7b2b70f90ba13e8b07b048d8216ae64",
    );
    assert_eq!(result.content.matches('ω').count(), 27, "retain every source omega");
    let scripts = superscripts(&result, "2a[8 sin(");
    assert!(
        !scripts.iter().any(|text| text.contains('ω')),
        "a fraction numerator must have no exponent annotation"
    );
    assert_eq!(result.content.matches("2a[8 sin( ) − 4 sin(ω)]").count(), 2);
    assert!(result.content.contains("rπ ω 2b 3ω 2c 5ω 2"));
    assert!(result.content.contains("\n\nω\n\n2a[8 sin( )"));
}

#[test]
fn should_not_annotate_mixed_fraction_numerators_as_exponents() {
    let result = ambiguity_page(
        "gutenberg-36670-p169.pdf",
        "70ae20eae453381732868eb65ef868bea7bbd3c36e2774401ccb52e2b1539c0e",
    );
    let scripts = superscripts(&result, "at 4");
    assert!(
        !scripts.iter().any(|text| text.contains('1')),
        "a mixed fraction numerator must have no exponent annotation"
    );
    assert_eq!(
        result.content.matches("1\n\nat 4~2~%.").count(),
        2,
        "retain both complete legacy mixed rates, including their numerator and denominator"
    );
}

#[test]
fn should_not_annotate_parenthesized_fraction_numerators_as_exponents() {
    let result = ambiguity_page(
        "gutenberg-38769-p500.pdf",
        "950d23bb5d8f1c47c6a8cca4e23d2230f883a12b4dbf5cf12793f1f9c1512a4b",
    );
    let scripts = superscripts(&result, "Determine which");
    assert!(
        !scripts.iter().any(|text| text.contains('1')),
        "a parenthesized fraction numerator must have no exponent annotation"
    );
    assert_eq!(scripts.iter().filter(|text| **text == "2").count(), 1);
    assert!(
        result
            .content
            .contains("1 1 3 π\n\n2. Determine which of (~2~e) and ( 2)<sup>2</sup> is the greater."),
        "retain the complete legacy fraction context and the genuine existing exponent"
    );
}

#[test]
fn should_not_omit_a_decimal_point_from_a_new_exponent_annotation() {
    let result = ambiguity_page(
        "ntrs-20040081236-p13.pdf",
        "71cd7420f93701108312f7b8bd1ba0b1c55665386e1e4051ae2a05782e639308",
    );
    let scripts = superscripts(&result, "Log Re");
    assert!(
        scripts.is_empty() || scripts == ["2.58"],
        "the actual decimal annotation must be complete or remain unassociated"
    );
    if scripts.is_empty() {
        assert!(
            result
                .content
                .contains("and for untripped data 091. *C*~*d*~ =+0 .0017 (3) 258.\n\n0\n\n( Log Re) In arriving"),
            "retain the complete unchanged source cohort when decimal ownership is unsupported"
        );
    } else {
        assert!(result.content.contains("( Log Re)<sup>2.58</sup>"));
    }
}

fn assert_complete_power(result: &xberg::ExtractedDocument, power: &str, base: &str) {
    let document = result.document.as_ref().expect("requested formula structure");
    let matches = document
        .nodes
        .iter()
        .filter_map(|node| node.content.text().map(|text| (node, text)))
        .flat_map(|(node, text)| {
            node.annotations.iter().filter_map(move |annotation| {
                let start = annotation.start as usize;
                let end = annotation.end as usize;
                (matches!(annotation.kind, AnnotationKind::Superscript) && text.get(start..end) == Some(power))
                    .then_some(&text[..start])
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(
        matches.len(),
        1,
        "the complete source exponent {power} must be annotated once"
    );
    assert!(
        matches[0].ends_with(base),
        "power {power} must immediately follow its actual base {base}: {}",
        matches[0]
    );
}

#[test]
fn should_preserve_both_complete_signed_decimal_powers_on_their_actual_bases() {
    let result = ambiguity_page(
        "ntrs-20040081236-p13.pdf",
        "71cd7420f93701108312f7b8bd1ba0b1c55665386e1e4051ae2a05782e639308",
    );
    let mut glyphs = result
        .document
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .filter_map(|node| node.content.text())
        .flat_map(str::chars)
        .filter(|character| !character.is_whitespace())
        .collect::<Vec<_>>();
    glyphs.sort_unstable();
    let inventory: String = glyphs.into_iter().collect();
    assert_eq!(
        inventory.chars().count(),
        2048,
        "retain the complete baseline body glyph inventory"
    );
    assert_eq!(
        hex::encode(Sha256::digest(inventory.as_bytes())),
        "5c2053eef4dc5dafcfb4f18055a81a6f878c02f79dc6badfc1fabfa017f4c15f"
    );
    assert_complete_power(&result, "6", "10");
    assert_complete_power(&result, "−0.15", "Re");
    assert_complete_power(&result, "2.58", "Re)");
    assert!(result.content.contains("Re<sup>−0.15</sup>"));
    assert!(result.content.contains("Re)<sup>2.58</sup>"));
}

#[test]
fn should_preserve_complete_decimal_powers_in_the_actual_drag_denominators() {
    let result = ambiguity_page(
        "ntrs-20040081236-p36.pdf",
        "4002b2651dc5f7a49dcf4c5107a4192974a15817af0df141c91e4a881c096b99",
    );
    let mut glyphs = result
        .document
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .filter_map(|node| node.content.text())
        .flat_map(str::chars)
        .filter(|character| !character.is_whitespace())
        .collect::<Vec<_>>();
    glyphs.sort_unstable();
    let inventory: String = glyphs.into_iter().collect();
    assert_eq!(
        inventory.chars().count(),
        2297,
        "retain the complete baseline body glyph inventory"
    );
    assert_eq!(
        hex::encode(Sha256::digest(inventory.as_bytes())),
        "7179d926ff39247ee7b3c0a5905af670ba2b9e72c4369d85b9c4b68df58e8cf6"
    );
    assert_complete_power(&result, "0.08", "Re");
    assert_complete_power(&result, "0.2", "Re");
    assert!(result.content.contains("Re<sup>0.08</sup>"));
    assert!(result.content.contains("Re<sup>0.2</sup>"));
}
