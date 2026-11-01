#![cfg(feature = "pdf")]

mod helpers;

use helpers::{extract_bytes_document_blocking, get_test_file_path};
use sha2::{Digest, Sha256};
use xberg::core::config::{ExtractionConfig, OutputFormat, PdfBackend, PdfConfig};
use xberg::types::document_structure::{AnnotationKind, DocumentStructure, NodeContent};

fn source_document(name: &str, sha256: &str, extract_tables: bool) -> xberg::ExtractedDocument {
    let path = get_test_file_path(&format!("pdf/regression_pages/{name}.pdf"));
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("mandatory source {}: {error}", path.display()));
    assert_eq!(hex::encode(Sha256::digest(&bytes)), sha256, "exact source identity");
    assert!(bytes.starts_with(b"%PDF-"));
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        include_document_structure: true,
        disable_ocr: true,
        use_cache: false,
        enable_quality_processing: false,
        pdf_options: Some(PdfConfig {
            backend: PdfBackend::Native,
            extract_tables,
            ..Default::default()
        }),
        ..Default::default()
    };
    extract_bytes_document_blocking(&bytes, "application/pdf", &config).expect("public native source extraction")
}

fn paragraph_superscripts(document: &DocumentStructure, anchor: &str) -> Vec<String> {
    let nodes: Vec<_> = document
        .nodes
        .iter()
        .filter(|node| matches!(&node.content, NodeContent::Paragraph { text } if text.contains(anchor)))
        .collect();
    assert_eq!(nodes.len(), 1, "unique source paragraph containing {anchor:?}");
    let node = nodes[0];
    let text = node.content.text().expect("paragraph text");
    node.annotations
        .iter()
        .filter(|annotation| matches!(annotation.kind, AnnotationKind::Superscript))
        .map(|annotation| {
            text.get(annotation.start as usize..annotation.end as usize)
                .expect("script annotation uses valid UTF-8 byte offsets")
                .to_owned()
        })
        .collect()
}

fn assert_differentiation_source(extract_tables: bool) {
    let result = source_document(
        "gutenberg-33283-p59",
        "26154b7fb99ba469af046603135b49f87ce26e8bf6b7b191bc30de66f558ca2c",
        extract_tables,
    );
    assert!(result.content.contains("SUCCESSIVE DIFFERENTIATION"));
    assert!(
        result.content.contains("Sixth differentiation"),
        "retain the actual zero derivative row"
    );
    if extract_tables {
        assert_eq!(result.tables.len(), 1, "preserve the source table inventory");
        let table = &result.tables[0];
        assert_eq!(
            table.markdown.matches("x2").count(),
            2,
            "both third-derivative table powers survive"
        );
        assert!(table.markdown.contains("Sixth differentiation"));
        assert!(table.markdown.contains("= 0"));
    } else {
        assert!(result.tables.is_empty(), "table-disabled path remains disabled");
    }
    let document = result.document.expect("requested public structure");
    for (anchor, powers) in [
        ("Let y = x", vec!["5"]),
        ("First differentiation", vec!["4"]),
        ("Second differentiation", vec!["3", "3"]),
        ("a function of x", vec!["2", "n"]),
    ] {
        assert_eq!(
            paragraph_superscripts(&document, anchor),
            powers,
            "source-owned powers at {anchor:?}"
        );
    }
}

fn assert_aes_modulus_source(extract_tables: bool) {
    let result = source_document(
        "nist-fips-197-upd1-p16",
        "ddbb5a63987345c809b99487f583bc7eca2e9883131b38cc8e5fea59763ef33d",
        extract_tables,
    );
    assert!(
        result.tables.is_empty(),
        "the polynomial is prose, not an invented table"
    );
    for retained in [
        "b~7~",
        "b~0~",
        "a~7~",
        "01100011",
        "01010111",
        "10000011",
        "11010100",
        "hexadecimal",
        "Algebra [4].",
    ] {
        assert!(
            result.content.contains(retained),
            "retain existing source content {retained:?}"
        );
    }
    let document = result.document.expect("requested public structure");
    let modulus = document
        .nodes
        .iter()
        .find(|node| matches!(&node.content, NodeContent::Paragraph { text } if text.contains("m(x)")))
        .expect("source modulus paragraph");
    assert!(
        !modulus.content.text().unwrap().contains("Algebra"),
        "footnote body is independent of modulus"
    );
    assert_eq!(
        paragraph_superscripts(&document, "Algebra [4]."),
        Vec::<String>::new(),
        "footnote prose is not a power"
    );
    assert_eq!(
        paragraph_superscripts(&document, "m(x)"),
        ["8", "4", "3"],
        "exact source modulus powers exclude trailing native whitespace and footnote1"
    );
}

#[test]
fn should_preserve_six_source_powers_through_cached_table_hierarchy_2275() {
    assert_differentiation_source(true);
}

#[test]
fn should_preserve_six_source_powers_through_direct_native_hierarchy_2275() {
    assert_differentiation_source(false);
}

#[test]
fn should_preserve_aes_modulus_powers_without_absorbing_footnotes_cached_2275() {
    assert_aes_modulus_source(true);
}

#[test]
fn should_preserve_aes_modulus_powers_without_absorbing_footnotes_direct_2275() {
    assert_aes_modulus_source(false);
}
