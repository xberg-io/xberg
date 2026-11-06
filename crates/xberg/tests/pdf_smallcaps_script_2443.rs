#![cfg(feature = "pdf")]

mod helpers;

use helpers::{extract_bytes_document_blocking, get_test_documents_dir};
use sha2::{Digest, Sha256};
use xberg::types::document_structure::AnnotationKind;
use xberg::{ExtractionConfig, OutputFormat};

fn nist_key_expansion_page() -> xberg::ExtractedDocument {
    let root = std::env::var_os("XBERG_SCRIPT_RETENTION_FIXTURE_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(get_test_documents_dir);
    let bytes = std::fs::read(root.join("pdf/regression_pages/nist-fips-197-upd1-p38.pdf"))
        .expect("mandatory small-cap source fixture");
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        "c8c9a756dd18278ae368f294ab43a9f246c6737a23fce15f28e1973555eb647f",
        "exact source page identity"
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
    if let Some(directory) = std::env::var_os("XBERG_SMALLCAPS_RECEIPTS") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).expect("small-cap receipt directory");
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
            std::fs::write(directory.join(name), value).expect("write actual source receipt");
        }
    }
    result
}

#[test]
fn should_leave_actual_small_cap_rotword_without_subscript_annotation() {
    let result = nist_key_expansion_page();
    let document = result.document.as_ref().expect("requested actual structure");
    let nodes: Vec<_> = document
        .nodes
        .iter()
        .filter(|node| {
            node.content.text().is_some_and(|text| {
                text.chars()
                    .filter(|character| !character.is_whitespace())
                    .collect::<String>()
                    .contains("ROTWORD()")
            })
        })
        .collect();
    assert_eq!(nodes.len(), 1, "the complete source function name must survive");
    let node = nodes[0];
    let text = node.content.text().unwrap();
    let start = text.find("RO").unwrap();
    let end = start + text[start..].find("()").unwrap() + 2;
    assert_eq!(
        text[start..end]
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect::<String>(),
        "ROTWORD()"
    );
    assert!(
        !node.annotations.iter().any(|annotation| {
            matches!(annotation.kind, AnnotationKind::Subscript | AnnotationKind::Superscript)
                && (annotation.start as usize) < end
                && (annotation.end as usize) > start
        }),
        "small-cap function letters must not receive script annotations"
    );
}

#[test]
fn should_preserve_all_six_actual_key_expansion_subscripts() {
    let result = nist_key_expansion_page();
    let document = result.document.as_ref().expect("requested actual structure");
    let context: Vec<_> = document
        .nodes
        .iter()
        .filter(|node| node.content.text().is_some_and(|text| text.contains("8e 73 b0 f7")))
        .collect();
    assert_eq!(context.len(), 1, "the complete key-expansion context must survive");

    // The indexed assignments are display rows, kept apart from the prose line above them. ~keep
    for digit in 0..6 {
        let variable = format!("w{digit} = ");
        let holders: Vec<_> = document
            .nodes
            .iter()
            .filter(|node| node.content.text().is_some_and(|text| text.contains(&variable)))
            .collect();
        assert_eq!(holders.len(), 1, "every original indexed word survives once: {variable}");
        let node = holders[0];
        let offset = node.content.text().unwrap().find(&variable).unwrap() + 1;
        assert!(
            node.annotations.iter().any(|annotation| {
                matches!(annotation.kind, AnnotationKind::Subscript)
                    && annotation.start as usize == offset
                    && annotation.end as usize == offset + 1
            }),
            "retain the actual subscript annotation and byte offsets for {variable}"
        );
    }
}
