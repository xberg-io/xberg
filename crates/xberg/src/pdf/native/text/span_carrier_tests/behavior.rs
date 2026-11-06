use super::*;
use lopdf::{Document, Object, Stream, dictionary};

fn geometry_and_layer_source() -> Vec<u8> {
    let mut pdf = Document::with_version("1.7");
    let pages = pdf.new_object_id();
    let layer = pdf.add_object(dictionary! { "Type" => "OCG", "Name" => Object::string_literal("Hidden") });
    let font = pdf.add_object(dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica" });
    let content = b"BT /F1 12 Tf 40 600 Td (Visible source words) Tj ET\n\
        BT /F1 12 Tf 0 1 -1 0 100 300 Tm (Rotated source words) Tj ET\n\
        BT /F1 12 Tf 40 500 Td (Power) Tj /F1 8 Tf 4 Ts (2) Tj ET\n\
        /OC /OC1 BDC BT /F1 12 Tf 40 450 Td (Hidden duplicate words) Tj ET EMC";
    let stream = pdf.add_object(Stream::new(dictionary! {}, content.to_vec()));
    let page = pdf.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages, "Contents" => stream,
        "MediaBox" => vec![10.into(), (-100).into(), 622.into(), 692.into()],
        "Resources" => dictionary! { "Font" => dictionary! { "F1" => font },
            "Properties" => dictionary! { "OC1" => layer } },
    });
    pdf.objects.insert(
        pages,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1,
        }),
    );
    let root = pdf.add_object(dictionary! {
        "Type" => "Catalog", "Pages" => pages,
        "OCProperties" => dictionary! { "OCGs" => vec![layer.into()],
            "D" => dictionary! { "OFF" => vec![layer.into()] } },
    });
    pdf.trailer.set("Root", root);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}

#[test]
fn should_preserve_exact_span_geometry_filtering_rotation_scripts_and_origin_dimensions() {
    let document = xberg_native_pdf::PdfDocument::from_bytes(geometry_and_layer_source()).unwrap();
    assert_eq!(document.page_count().unwrap(), 1);
    let hidden = xberg_native_pdf::optional_content::compute_default_off_ocgs(&document);
    assert_eq!(
        hidden.len(),
        1,
        "a real default-off layer must exercise filtered extraction"
    );
    let mut jobs = 0;
    for excluded in [HashSet::new(), hidden, HashSet::from(["absent-layer".to_owned()])] {
        let reference = if excluded.is_empty() {
            document
                .extract_page_text_with_options(0, ReadingOrder::ColumnAware)
                .unwrap()
                .spans
        } else {
            document
                .extract_spans_filtered_with_reading_order(
                    0,
                    ReadingOrder::ColumnAware,
                    excluded.clone(),
                    HashSet::new(),
                )
                .unwrap()
        };
        let page = page_text_with_options_excluding_layers(&document, 0, &excluded).unwrap();
        assert_eq!(
            serde_json::to_value(&page.spans).unwrap(),
            serde_json::to_value(reference).unwrap()
        );
        assert_eq!(page.page_width, if excluded.is_empty() { 612.0 } else { 622.0 });
        assert_eq!(page_vertical_bounds(&document, 0).unwrap(), (-100.0, 692.0));
        assert!(page.spans.iter().any(|span| span.rotation_degrees.abs() > 1.0));
        let text: String = page.spans.iter().map(|span| span.text.as_str()).collect();
        assert!(text.contains("Visible source words") && text.contains("Power") && text.contains('2'));
        assert_eq!(text.contains("Hidden duplicate words"), !excluded.contains("Hidden"));
        jobs += 1;
    }
    assert_eq!(jobs, 3);
}
