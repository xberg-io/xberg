#![cfg(all(feature = "pdf", feature = "tokio-runtime"))]

mod helpers;

use sha2::{Digest, Sha256};
use xberg::types::document_structure::{AnnotationKind, NodeContent};
use xberg::{ExtractedDocument, ExtractionConfig, OutputFormat};

const NATIVE_TEST_STACK_BYTES: usize = 16 * 1024 * 1024;
const SOURCE_SHA: &str = "744596302924ecc3349541572c0f51c479907866bddead64620cb94bfa8ca08c";
const PROSE: &str = "Key = 2b 7e 15 16 28 ae d2 a6 ab f7 15 88 09 cf 4f 3c for Nk = 4, which results in";
const ROW: &str = "w0 = 2b7e1516 w1 = 28aed2a6 w2 = abf71588 w3 = 09cf4f3c";
const MARKDOWN_ROW: &str = "w~0~ = 2b7e1516 w~1~ = 28aed2a6 w~2~ = abf71588 w~3~ = 09cf4f3c";

fn extract_source(page: u8, sha: &str) -> ExtractedDocument {
    let path = helpers::get_test_file_path(&format!("pdf/regression_pages/nist-fips-197-upd1-p{page}.pdf"));
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("required source {}: {error}", path.display()));
    assert_eq!(hex::encode(Sha256::digest(&bytes)), sha);
    let pdf = xberg_native_pdf::PdfDocument::from_bytes(bytes.clone()).unwrap();
    assert_eq!(pdf.page_count().unwrap(), 1);
    let config = ExtractionConfig {
        disable_ocr: true,
        use_cache: false,
        enable_quality_processing: false,
        output_format: OutputFormat::Markdown,
        include_document_structure: true,
        ..Default::default()
    };
    helpers::extract_bytes_document_blocking(&bytes, "application/pdf", &config).unwrap()
}

fn assert_table_ownership(result: &ExtractedDocument) {
    let matching: Vec<_> = result
        .tables
        .iter()
        .map(|table| &table.cells)
        .filter(|cells| cells.len() == 17 && cells.iter().all(|row| row.len() == 8))
        .collect();
    let [cells] = matching.as_slice() else {
        panic!("expected one 17×8 source grid, found {}", matching.len());
    };
    assert_eq!(cells.len(), 17);
    assert!(cells.iter().all(|row| row.len() == 8));
    assert_eq!(
        cells[0],
        [
            "4", "09cf4f3c", "cf4f3c09", "8a84eb01", "01000000", "8b84eb01", "2b7e1516", "a0fafe17"
        ]
    );
    assert_eq!(cells[1][6], "28aed2a6");
    assert_eq!(cells[2][6], "abf71588");
    assert_eq!(cells[3][6], "09cf4f3c");
}

fn source_result(page: u8, sha: &'static str) -> ExtractedDocument {
    std::thread::Builder::new()
        .stack_size(NATIVE_TEST_STACK_BYTES)
        .spawn(move || extract_source(page, sha))
        .expect("spawn native PDF extraction")
        .join()
        .expect("native PDF extraction completed")
}

#[test]
fn source_display_assignments_are_a_separate_paragraph_with_four_scripts() {
    let result = source_result(37, SOURCE_SHA);
    assert_table_ownership(&result);
    let document = result.document.as_ref().expect("requested document structure");
    let nodes = &document.nodes;
    let matching: Vec<_> = nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node.content.text().is_some_and(|text| text.ends_with(ROW)))
        .collect();
    let [(index, node)] = matching.as_slice() else {
        panic!("expected one source display carrier, found {}", matching.len());
    };
    let NodeContent::Paragraph { text } = &node.content else {
        panic!("display carrier must keep its body paragraph role");
    };
    let row_start = text.len() - ROW.len();
    let scripts: Vec<_> = node
        .annotations
        .iter()
        .filter(|annotation| annotation.kind == AnnotationKind::Subscript)
        .map(|annotation| {
            (
                annotation.start as usize - row_start,
                annotation.end as usize - row_start,
            )
        })
        .collect();
    assert_eq!(scripts, [(1, 2), (15, 16), (29, 30), (43, 44)]);
    assert_eq!(
        scripts.iter().map(|&(start, end)| &ROW[start..end]).collect::<Vec<_>>(),
        ["0", "1", "2", "3"]
    );
    assert_eq!(
        text, ROW,
        "the physical four-assignment row must be separate from preceding prose"
    );
    assert!(matches!(&nodes[index - 1].content, NodeContent::Paragraph { text } if text == PROSE));
    assert!(matches!(&nodes[index - 2].content, NodeContent::Heading { text, .. }
        if text == "This section contains the key expansion of the following key:"));
    assert!(matches!(&nodes[index + 1].content, NodeContent::Paragraph { text }
        if text == "w[i] = i After After After XOR temp Rcon[i/Nk] w[i−Nk] temp⊕"));
    let bbox = node.bbox.expect("display row geometry");
    assert!(bbox.y0 < 519.062 && bbox.y1 > 519.062 && bbox.y1 < 545.435);
    assert_eq!(result.content.lines().filter(|line| *line == MARKDOWN_ROW).count(), 1);
}

#[test]
fn adjacent_indexed_display_rows_preserve_their_separate_source_baselines() {
    let result = source_result(38, "c8c9a756dd18278ae368f294ab43a9f246c6737a23fce15f28e1973555eb647f");
    let nodes = &result.document.as_ref().expect("requested document structure").nodes;
    let prose =
        "Key = 8e 73 b0 f7 da 0e 64 52 c8 10 f3 2b 80 90 79 e5 62 f8 ea d2 52 2c 6b 7b for Nk = 6, which results in";
    let start = nodes
        .iter()
        .position(|node| node.content.text() == Some(prose))
        .expect("separate Key prose");
    for (offset, (expected, digits, baseline)) in [
        ("w0 = 8e73b0f7 w1 = da0e6452 w2 = c810f32b", ["0", "1", "2"], 205.333),
        ("w3 = 809079e5 w4 = 62f8ead2 w5 = 522c6b7b", ["3", "4", "5"], 188.3805),
    ]
    .into_iter()
    .enumerate()
    {
        let node = &nodes[start + 1 + offset];
        assert!(matches!(&node.content, NodeContent::Paragraph { text } if text == expected));
        let bbox = node.bbox.expect("display row geometry");
        assert!(bbox.y0 < baseline && baseline < bbox.y1);
        let scripts: Vec<_> = node
            .annotations
            .iter()
            .map(|annotation| {
                assert_eq!(annotation.kind, AnnotationKind::Subscript);
                (annotation.start as usize, annotation.end as usize)
            })
            .collect();
        assert_eq!(scripts, [(1, 2), (15, 16), (29, 30)]);
        assert_eq!(
            scripts.iter().map(|&(a, b)| &expected[a..b]).collect::<Vec<_>>(),
            digits
        );
    }
    assert!(matches!(&nodes[start + 3].content, NodeContent::Paragraph { text }
        if text == "w[i] = i After After After XOR temp Rcon[i/Nk] w[i−Nk] temp⊕"));
    assert!(result.tables.iter().any(|table| table.cells.len() == 17
        && table.cells.iter().all(|row| row.len() == 8)
        && table.cells[0][1] == "11f915bc"
        && table.cells[0][6] == "d4d1c6f8"));
}
