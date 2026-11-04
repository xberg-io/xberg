#![cfg(feature = "pdf")]

mod helpers;

use helpers::{extract_bytes_document_blocking, get_test_documents_dir};
use sha2::{Digest, Sha256};
use xberg::types::document_structure::AnnotationKind;
use xberg::{ExtractionConfig, OutputFormat};

fn derivatives_and_integrals_page() -> xberg::ExtractedDocument {
    let root = std::env::var_os("XBERG_PRIME_PLACEMENT_FIXTURE_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(get_test_documents_dir);
    let bytes = std::fs::read(root.join("pdf/regression_pages/gutenberg-38769-p319.pdf"))
        .expect("mandatory prime placement fixture");
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        "0ce1933e43fd03bb095a03f1007df9bf40decdb7924bf7b81f2743eaa071d16a",
        "exact source page identity",
    );
    let config = ExtractionConfig {
        disable_ocr: true,
        use_cache: false,
        enable_quality_processing: false,
        output_format: OutputFormat::Markdown,
        include_document_structure: true,
        ..Default::default()
    };
    let result = extract_bytes_document_blocking(&bytes, "application/pdf", &config).expect("actual native extraction");
    if let Some(directory) = std::env::var_os("XBERG_PRIME_PLACEMENT_RECEIPTS") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).expect("prime placement receipt directory");
        let native = xberg_native_pdf::PdfDocument::from_bytes(bytes).expect("same native source");
        let page = native
            .extract_page_text_with_script_geometry(0, xberg_native_pdf::document::ReadingOrder::TopToBottom)
            .expect("actual preserved native spans");
        for (name, value) in [
            ("actual-result.json", serde_json::to_vec_pretty(&result).unwrap()),
            (
                "native-preserved-spans.json",
                serde_json::to_vec_pretty(&page.spans).unwrap(),
            ),
        ] {
            std::fs::write(directory.join(name), value).expect("write prime placement receipt");
        }
    }
    result
}

#[test]
fn should_keep_each_geometric_prime_with_its_operand_in_the_source_equation() {
    let result = derivatives_and_integrals_page();
    let document = result.document.as_ref().expect("requested actual structure");
    let equations: Vec<_> = document
        .nodes
        .iter()
        .filter_map(|node| {
            let compact = node.content.text()?.split_whitespace().collect::<String>();
            (compact.contains("(PRP") && compact.contains("(ONPP0)")).then_some((node, compact))
        })
        .collect();
    assert_eq!(equations.len(), 1, "the actual displayed equation must survive");
    let (node, compact) = &equations[0];
    assert!(
        compact.contains("(PRP′)+(NN′RP)=(NN′P′P)"),
        "all four geometric primes must stay with their actual operands: {compact}",
    );
    for (offset, glyph) in node.content.text().unwrap().match_indices('′') {
        assert!(!node.annotations.iter().any(|annotation| {
            matches!(annotation.kind, AnnotationKind::Subscript | AnnotationKind::Superscript)
                && (annotation.start as usize) < offset + glyph.len()
                && (annotation.end as usize) > offset
        }));
    }
}

#[test]
fn should_keep_prose_quotes_and_the_actual_area_subscript() {
    let result = derivatives_and_integrals_page();
    let document = result.document.as_ref().expect("requested actual structure");
    let node = document
        .nodes
        .iter()
        .find(|node| {
            node.content
                .text()
                .is_some_and(|text| text.contains("ascribing an 'area'"))
        })
        .expect("original prose and ordinary quotes");
    let text = node.content.text().unwrap();
    let position = text.find("P0").expect("actual P0 notation") + 1;
    assert!(
        node.annotations.iter().any(|annotation| {
            annotation.kind == AnnotationKind::Subscript
                && annotation.start as usize == position
                && annotation.end as usize == position + 1
        }),
        "retain the true subscript and its byte range",
    );
}
