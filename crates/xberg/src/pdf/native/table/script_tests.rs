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

fn span(text: &str, x: f32, baseline: f32, width: f32, font_size: f32) -> TextSpan {
    TextSpan {
        text: text.to_owned(),
        bbox: Rect::new(x, baseline, width, font_size),
        font_size,
        ..TextSpan::default()
    }
}

fn short_cell() -> TableCell {
    TableCell {
        text: "R2".to_owned(),
        spans: Vec::new(),
        colspan: 1,
        rowspan: 1,
        mcids: Vec::new(),
        bbox: Some(Rect::new(36.0, 378.6, 60.0, 12.5)),
        is_header: false,
    }
}

fn page_spans_of(spans: Vec<TextSpan>) -> PageSpans {
    PageSpans {
        spans,
        margin: 8.0 * crate::script_run::SCRIPT_RUN_MAX_BASELINE_FONT_FACTOR,
    }
}

/// The row above a short cell reaches into the script margin: its text must not hide the cell's own
/// raised `2` (the `R2` label of a regression table).
#[test]
fn should_keep_a_cell_script_when_the_row_above_falls_in_the_script_margin() {
    let spans = page_spans_of(vec![
        span("R", 40.0, 381.4, 5.0, 8.0),
        span("2", 45.1, 384.9, 3.0, 5.3),
        span("N", 40.0, 392.5, 5.0, 8.0),
    ]);
    assert_eq!(
        cell_annotations(&short_cell(), "R2", &spans),
        vec![crate::types::builder::superscript(1, 2)]
    );
}

/// A script drawn just above the cell's top edge is outside the cell's own spans and is found only
/// by the margin pass.
#[test]
fn should_find_a_cell_script_raised_above_the_cell_top_through_the_margin() {
    let spans = page_spans_of(vec![span("R", 40.0, 387.5, 5.0, 8.0), span("2", 45.1, 391.2, 3.0, 5.3)]);
    assert_eq!(
        cell_annotations(&short_cell(), "R2", &spans),
        vec![crate::types::builder::superscript(1, 2)]
    );
}

/// Neighbouring text that no pass can fit to the cell text yields no annotation rather than a guess.
#[test]
fn should_annotate_nothing_when_a_neighbour_hides_the_cell_text() {
    let spans = page_spans_of(vec![span("R", 40.0, 381.4, 5.0, 8.0), span("N", 40.0, 392.5, 5.0, 8.0)]);
    assert!(cell_annotations(&short_cell(), "R2", &spans).is_empty());
}
