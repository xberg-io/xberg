use super::*;
use lopdf::{Document, Object, Stream, dictionary};
use xberg_native_pdf::geometry::Rect;

const CELL: Rect = Rect {
    x: 50.0,
    y: 500.0,
    width: 40.0,
    height: 100.0,
};
const BASE: &str = "1000000000";
const TEXT: &str = "10000000009/l";

fn rotated_cell_pdf() -> Vec<u8> {
    let mut pdf = Document::with_version("1.7");
    let pages = pdf.new_object_id();
    let font = pdf.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica",
    });
    let content = format!(
        "0.5 w {} {} {} {} re S\nBT /F1 12 Tf 0 1 -1 0 80 510 Tm ({BASE}) Tj \
         /F1 8 Tf 3 Ts (9) Tj /F1 12 Tf 0 Ts (/l) Tj ET\n",
        CELL.x, CELL.y, CELL.width, CELL.height,
    );
    let stream = pdf.add_object(Stream::new(dictionary! {}, content.into_bytes()));
    let page = pdf.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages, "Contents" => stream,
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
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
    pdf.save_to(&mut bytes).expect("rotated ruled cell serializes");
    bytes
}

#[test]
fn should_map_real_rotated_script_spans_into_a_narrow_ruled_cell_gh2275() {
    let doc = NativeDocument::open_bytes(&rotated_cell_pdf()).expect("real rotated PDF opens");
    let mut warnings = Vec::new();
    let spans = page_spans(&doc, 0, &mut warnings);
    assert_eq!(warnings.len(), 0);
    let base = spans
        .spans
        .iter()
        .find(|span| span.text == BASE)
        .expect("long rotated base");
    assert_eq!(base.rotation_degrees, 90.0);
    assert!(base.bbox.width > CELL.width);
    let ordered = sort_spans_in_reading_order(spans.spans.iter().collect());
    let decoded: String = ordered.iter().map(|span| span.text.as_str()).collect();
    assert_eq!(decoded, TEXT, "complete source cell before annotation mapping");
    let cell = TableCell {
        text: TEXT.to_owned(),
        spans: Vec::new(),
        colspan: 1,
        rowspan: 1,
        mcids: Vec::new(),
        bbox: Some(CELL),
        is_header: false,
    };
    assert_eq!(
        cell_annotations(&cell, TEXT, &spans),
        vec![crate::types::builder::superscript(10, 11)]
    );
}
