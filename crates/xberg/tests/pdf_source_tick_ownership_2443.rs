#![cfg(all(feature = "pdf", feature = "tokio-runtime"))]

mod helpers;

use sha2::{Digest, Sha256};
use xberg::{ExtractedDocument, ExtractionConfig, OutputFormat};
use xberg_native_pdf::document::ReadingOrder;

const SOURCE_SHA: &str = "1508aa9a0e12e8aad8bbc594a99ded16ce7517518e2689fc6002e59263c3c005";
const LEFT_TICK: (f32, f32) = (350.686_37, 645.023_44);
const RIGHT_TICK: (f32, f32) = (444.646_36, 663.616_94);

fn source_bytes() -> Vec<u8> {
    let path = std::env::var_os("XBERG_2443_SOURCE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| helpers::get_test_documents_dir().join("pdf/regression_pages/arxiv-2302.00681v1-p3.pdf"));
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("required source {}: {error}", path.display()));
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        SOURCE_SHA,
        "independent source PDF identity"
    );
    bytes
}

fn assert_native_source_ticks(bytes: &[u8]) {
    let pdf = xberg_native_pdf::PdfDocument::from_bytes(bytes.to_vec()).unwrap();
    assert_eq!(pdf.page_count().unwrap(), 1);
    for order in [ReadingOrder::TopToBottom, ReadingOrder::ColumnAware] {
        let page = pdf.extract_page_text_with_script_geometry(0, order).unwrap();
        for (expected, (left, baseline)) in [
            ("0.6", LEFT_TICK),
            ("0.6", RIGHT_TICK),
            ("0.4", (RIGHT_TICK.0, 633.722_05)),
        ] {
            let mut glyphs: Vec<_> = page
                .spans
                .iter()
                .filter(|span| {
                    span.bbox.y.to_bits() == baseline.to_bits() && span.bbox.x >= left && span.bbox.x < left + 12.0
                })
                .collect();
            glyphs.sort_by(|left, right| left.bbox.x.total_cmp(&right.bbox.x));
            assert_eq!(glyphs.len(), 3, "three separately located source glyphs per tick");
            assert_eq!(
                glyphs.iter().map(|span| span.text.as_str()).collect::<String>(),
                expected
            );
        }
    }
}

fn source_result() -> ExtractedDocument {
    let bytes = source_bytes();
    assert_native_source_ticks(&bytes);
    let config = ExtractionConfig {
        disable_ocr: true,
        use_cache: false,
        enable_quality_processing: false,
        output_format: OutputFormat::Markdown,
        include_document_structure: true,
        ..Default::default()
    };
    let result = helpers::extract_bytes_document_blocking(&bytes, "application/pdf", &config).unwrap();
    assert!(!result.content.trim().is_empty());
    result
}

#[test]
fn both_source_ticks_survive_table_and_paragraph_ownership() {
    let result = source_result();
    assert_eq!(
        result.content.matches("0.6").count(),
        2,
        "retain both independently located source ticks"
    );
}

fn contains_point(bbox: xberg::types::BoundingBox, point: (f64, f64)) -> bool {
    bbox.x0 <= point.0 && bbox.x1 >= point.0 && bbox.y0 <= point.1 && bbox.y1 >= point.1
}

#[test]
fn public_glyph_inventory_retains_the_left_tick_without_an_owned_right_zero() {
    let result = source_result();
    let document = result.document.as_ref().expect("requested source geometry");
    let left_point = (LEFT_TICK.0 as f64 + 2.0, LEFT_TICK.1 as f64 + 4.0);
    let left: Vec<_> = document
        .nodes
        .iter()
        .filter(|node| {
            node.bbox.is_some_and(|bbox| contains_point(bbox, left_point))
                && node
                    .content
                    .text()
                    .is_some_and(|text| text.split_whitespace().any(|word| word == "0.6"))
        })
        .collect();
    assert_eq!(
        left.len(),
        1,
        "one non-table source carrier at the original left-tick location"
    );
    assert!(
        left[0].bbox.unwrap().x1 < RIGHT_TICK.0 as f64,
        "the left carrier cannot borrow the right-panel occurrence"
    );
    let right_point = (RIGHT_TICK.0 as f64 + 2.0, 633.722_045_898_437_5 + 4.0);
    assert!(
        document.nodes.iter().all(|node| {
            node.content.text().is_none() || !node.bbox.is_some_and(|bbox| contains_point(bbox, right_point))
        }),
        "the owned right zero cannot also survive in a prose carrier"
    );
    let cells: Vec<_> = result
        .tables
        .iter()
        .flat_map(|table| table.cells.iter().flatten())
        .filter(|cell| cell.as_str() == "0.6 0.4")
        .collect();
    assert_eq!(
        cells.len(),
        1,
        "the Native cell still owns both complete right-panel runs"
    );
    let left_text = left[0].content.text().expect("visible left tick");
    assert_eq!(left_text, "0.6");
    let affected = format!("{left_text}{}", cells[0]);
    for (glyph, expected) in [('0', 3), ('.', 3), ('6', 2), ('4', 1)] {
        assert_eq!(
            affected.matches(glyph).count(),
            expected,
            "visible source inventory for {glyph}"
        );
    }
}

#[test]
fn rotated_chart_label_keeps_its_paragraph_role() {
    use xberg::types::document_structure::NodeContent;

    let result = source_result();
    let document = result.document.as_ref().expect("requested source geometry");
    for (label, point) in [("i", (349.0, 700.0)), ("(0)", (350.0, 688.0))] {
        assert_eq!(
            document
                .nodes
                .iter()
                .filter(|node| {
                    node.bbox.is_some_and(|bbox| contains_point(bbox, point))
                        && matches!(&node.content, NodeContent::Paragraph { text } if text == label)
                })
                .count(),
            1,
            "chart label {label} remains a paragraph at its source location"
        );
    }
}
