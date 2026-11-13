use super::MatchedLayerOccurrences;
use crate::extractors::pdf::ocr::document::{
    MisreadOcrRow, PlainOcrRows, finish_mixed_layer_prepend, flat_ocr_page_document, layer_lines_to_put_first,
    merge_structured_ocr_pages_into_internal_document, text_with_layer_first,
};
use crate::ocr_metadata_keys::{OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY, OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY};
use crate::types::{BoundingBox, ExtractedDocument, OcrBoundingGeometry, OcrElement, OcrElementLevel, Table};
use lopdf::{Document, Object, Stream, dictionary};

const PAGE_SIZE: u32 = 300;
const LAYER: &str = "769 NativeOnly";

fn source() -> xberg_native_pdf::PdfDocument {
    let mut pdf = Document::with_version("1.4");
    let pages = pdf.new_object_id();
    let font = pdf.add_object(dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica" });
    let content = pdf.add_object(Stream::new(
        dictionary! {},
        b"BT /F1 16 Tf 40 220 Td (769 NativeOnly) Tj ET".to_vec(),
    ));
    let page = pdf.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages, "Contents" => content,
        "MediaBox" => vec![0.into(), 0.into(), PAGE_SIZE.into(), PAGE_SIZE.into()],
        "Resources" => dictionary! { "Font" => dictionary! { "F1" => font } },
    });
    pdf.objects.insert(
        pages,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1,
        }),
    );
    let catalog = pdf.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
    pdf.trailer.set("Root", catalog);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    xberg_native_pdf::PdfDocument::from_bytes(bytes).unwrap()
}

fn iterator_word(text: &str, left: u32, top: u32, width: u32, height: u32) -> OcrElement {
    OcrElement {
        text: text.into(),
        geometry: OcrBoundingGeometry::Rectangle {
            left,
            top,
            width,
            height,
        },
        level: OcrElementLevel::Word,
        backend_metadata: [("backend".into(), serde_json::json!("tesseract-iterator"))].into(),
        ..Default::default()
    }
}

fn selected_read(pdf: &xberg_native_pdf::PdfDocument) -> ExtractedDocument {
    let lines = pdf.extract_text_lines(0).unwrap();
    let word = lines
        .iter()
        .flat_map(|line| &line.words)
        .find(|word| word.text == "769")
        .unwrap();
    let bounds = word.bbox;
    let table = Table {
        cells: vec![vec!["769".into()], vec!["OcrOnly".into()]],
        bounding_box: Some(BoundingBox {
            x0: 0.0,
            y0: 0.0,
            x1: f64::from(PAGE_SIZE),
            y1: f64::from(PAGE_SIZE),
        }),
        ..Default::default()
    };
    let mut read = ExtractedDocument {
        content: crate::rendering::common::render_table_markdown(&table.cells),
        tables: vec![table],
        ocr_elements: Some(vec![
            iterator_word(
                "769",
                bounds.x as u32,
                (PAGE_SIZE as f32 - bounds.y - bounds.height) as u32,
                bounds.width.ceil() as u32,
                bounds.height.ceil() as u32,
            ),
            iterator_word("OcrOnly", 40, 120, 55, 15),
        ]),
        ..Default::default()
    };
    for key in [
        OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY,
        OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY,
    ] {
        read.metadata.additional.insert(key.into(), PAGE_SIZE.into());
    }
    read
}

#[test]
fn mixed_prepend_keeps_769_once_and_preserves_a_standalone_ocr_cell() {
    let pdf = source();
    let read = selected_read(&pdf);
    let mut first = layer_lines_to_put_first(
        LAYER,
        &read.content,
        true,
        &Default::default(),
        None,
        PlainOcrRows::of_page(None, None),
    )
    .expect("the native row is absent from the selected OCR read");
    first.lines = format!("  {}\n\n  \n\n  Tail", first.lines);
    let expected_prefix = "  NativeOnly\n\n  \n\n  Tail";
    let count = |text: &str, token: &str| text.split_whitespace().filter(|word| *word == token).count();
    assert_eq!(count(&text_with_layer_first(&read.content, &first), "769"), 2);

    let proof =
        MatchedLayerOccurrences::capture(&pdf, 0, LAYER, &read).expect("selected OCR geometry proves the match");
    let mut flat = format!("{}\nNativeAlternate", read.content);
    let mut structured = flat_ocr_page_document("NativeAlternate");
    first.misread_ocr_rows.push(MisreadOcrRow {
        page: None,
        line: None,
        bbox: None,
        key: "nativealternate".into(),
        native_key: "769 nativeonly".into(),
    });
    structured.tables = read.tables;
    proof.trim_prepend(&mut first, &flat, &structured, 1, Some((1.0, 1.0)));
    finish_mixed_layer_prepend(&mut flat, Some(&mut structured), &first);
    assert_eq!(structured.elements[0].text, "NativeOnly");
    assert_eq!(structured.elements[1].text, "Tail");
    assert!(flat.starts_with(expected_prefix));
    let mut merged = flat_ocr_page_document("native placeholder");
    merged.elements[0].page = Some(1);
    merge_structured_ocr_pages_into_internal_document(
        &mut merged,
        &ahash::AHashMap::from([(1, flat.clone())]),
        &ahash::AHashMap::from([(1, structured)]),
    );
    for output in [
        flat,
        crate::rendering::render_plain(&merged),
        crate::rendering::render_markdown(&merged),
    ] {
        for token in ["769", "NativeOnly", "OcrOnly", "NativeAlternate"] {
            assert_eq!(count(&output, token), 1, "{token} in {output:?}");
        }
    }
    assert_eq!(merged.tables[0].cells, [vec!["769"], vec!["OcrOnly"]]);
}

#[test]
fn unmatched_prepend_preserves_legacy_flat_and_structured_bytes() {
    let first = crate::extractors::pdf::ocr::document::LayerFirst {
        lines: "  Native  row\n  indented\n\n  Second paragraph  ".into(),
        lost_glyphs: 0,
        misread_ocr_rows: Vec::new(),
    };
    let mut flat = "  OCR text\nwith spacing  \n".to_owned();
    let mut structured = flat_ocr_page_document(&flat);
    finish_mixed_layer_prepend(&mut flat, Some(&mut structured), &first);
    assert_eq!(
        flat.as_bytes(),
        b"  Native  row\n  indented\n\n  Second paragraph  \n\n  OCR text\nwith spacing  \n"
    );
    assert_eq!(
        crate::rendering::render_plain(&structured).as_bytes(),
        b"Native  row\n  indented\n\nSecond paragraph\n\nOCR text\nwith spacing",
    );
}
