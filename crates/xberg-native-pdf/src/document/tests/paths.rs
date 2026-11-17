use super::super::*;
use super::pdf_fixtures::*;

#[test]
fn test_oc_name_ocg_ascii() {
    let doc = oc_test_doc();
    let dict = ocg_dict(Object::String(b"A-GRID".to_vec()));
    assert_eq!(doc.read_oc_name(dict.as_dict().unwrap(), 8).as_deref(), Some("A-GRID"));
}

#[test]
fn test_oc_name_ocg_utf16be_bom() {
    let doc = oc_test_doc();
    let dict = ocg_dict(utf16_string("EJES", true));
    assert_eq!(doc.read_oc_name(dict.as_dict().unwrap(), 8).as_deref(), Some("EJES"));
}

#[test]
fn test_oc_name_ocmd_single_ocg() {
    // OCMD has no /Name — resolution follows /OCGs (single OCG) to its name. ~keep
    let doc = oc_test_doc();
    let ocmd = ocmd_dict(ocg_dict(Object::String(b"M-DUCT".to_vec())));
    assert_eq!(doc.read_oc_name(ocmd.as_dict().unwrap(), 8).as_deref(), Some("M-DUCT"));
}

#[test]
fn test_oc_name_ocmd_ocgs_array_first_wins() {
    // /OCGs may be an array of OCGs; the first resolvable member wins. ~keep
    let doc = oc_test_doc();
    let arr = Object::Array(vec![
        ocg_dict(Object::String(b"S-COLS".to_vec())),
        ocg_dict(Object::String(b"S-BEAM".to_vec())),
    ]);
    let ocmd = ocmd_dict(arr);
    assert_eq!(doc.read_oc_name(ocmd.as_dict().unwrap(), 8).as_deref(), Some("S-COLS"));
}

#[test]
fn test_oc_name_ocmd_depth_guard() {
    // A pathological OCMD chain (each /OCGs points to another OCMD) must
    // terminate via the depth guard rather than recursing without bound. ~keep
    let doc = oc_test_doc();
    let mut nested = ocmd_dict(Object::Array(vec![]));
    for _ in 0..20 {
        nested = ocmd_dict(nested);
    }
    assert_eq!(doc.read_oc_name(nested.as_dict().unwrap(), 8), None);
}

#[test]
fn test_resolve_oc_name_via_resources_properties() {
    // Case 2 (name reference) resolves against the *passed-in* resources.
    // This is the crux of the Form-XObject fix: the resolver reads
    // /Properties /<name> from whatever resource scope the caller hands
    // it — page /Resources at page level, the XObject's own /Resources
    // when extracting inside a Form XObject. ~keep
    let doc = oc_test_doc();
    let mut props = std::collections::HashMap::new();
    props.insert("MC0".to_string(), ocg_dict(Object::String(b"A-WALL-DIM".to_vec())));
    let mut resources = std::collections::HashMap::new();
    resources.insert("Properties".to_string(), Object::Dictionary(props));
    let resources = Object::Dictionary(resources);

    let name_ref = Object::Name("MC0".to_string());
    assert_eq!(
        doc.resolve_oc_layer_name(Some(&resources), &name_ref).as_deref(),
        Some("A-WALL-DIM")
    );
}

#[test]
fn test_resolve_oc_name_inline_dict() {
    let doc = oc_test_doc();
    let inline = ocg_dict(Object::String(b"CORTES".to_vec()));
    assert_eq!(doc.resolve_oc_layer_name(None, &inline).as_deref(), Some("CORTES"));
}

#[test]
fn test_resolve_oc_name_unresolvable_is_none() {
    // A name reference with no resources in scope yields None (the path
    // is left unlabelled) rather than an error. ~keep
    let doc = oc_test_doc();
    let name_ref = Object::Name("MC9".to_string());
    assert_eq!(doc.resolve_oc_layer_name(None, &name_ref), None);
}

#[test]
fn form_path_dedup_preserves_inherited_fill_opacity_in_both_orders() {
    let page = "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R \
                /Resources << /XObject << /Bar 5 0 R >> \
                /ExtGState << /Clear << /ca 0 >> /Opaque << /ca 1 >> >> >> >>";
    let bar = "10 20 42 0.4 re f";
    let form = format!(
        "<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] /Length {} >>\nstream\n{bar}\nendstream",
        bar.len()
    );
    let mut included = Vec::new();
    for (first, second) in [("Clear", "Opaque"), ("Opaque", "Clear")] {
        let content = format!("q /{first} gs /Bar Do /Bar Do Q q /{second} gs /Bar Do /Bar Do Q");
        let stream = format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len());
        let pdf = build_catalog_test_pdf(
            b"<< /Type /Catalog /Pages 2 0 R >>",
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            &[(3, page.as_bytes()), (4, stream.as_bytes()), (5, form.as_bytes())],
        );
        let paths = PdfDocument::from_bytes(pdf).unwrap().extract_paths(0).unwrap();
        included.push(paths.iter().map(|path| path.has_fill()).collect::<Vec<_>>());
    }
    assert_eq!(included, [[false, true], [true, false]]);
}

#[test]
fn form_path_dedup_preserves_dark_white_dark_repaint_order() {
    let bar = "10 20 42 0.4 re f";
    let form = format!(
        "<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] /Length {} >>\nstream\n{bar}\nendstream",
        bar.len()
    );
    let doc = page_with_xobject("0 g /X1 Do 1 g /X1 Do 0 g /X1 Do", &form);
    let fills: Vec<_> = doc
        .extract_paths(0)
        .unwrap()
        .into_iter()
        .filter_map(|path| path.fill_color)
        .collect();
    let shade = |gray| crate::layout::Color::new(gray, gray, gray);
    assert_eq!(fills, [shade(0.0), shade(1.0), shade(0.0)]);
}

#[test]
fn shared_child_forms_charge_one_page_budget_in_paint_order() {
    use super::super::paths::FormPathBudget;

    let content = "0 g /First Do 1 g /Second Do";
    let parent = "/Child Do";
    let child = "10 20 42 0.4 re f";
    let page = "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R \
                /Resources << /XObject << /First 5 0 R /Second 6 0 R >> >> >>";
    let stream = format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len());
    let wrapper = format!(
        "<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Resources << /XObject << /Child 7 0 R >> >> /Length {} >>\nstream\n{parent}\nendstream",
        parent.len()
    );
    let bar = format!(
        "<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] /Length {} >>\nstream\n{child}\nendstream",
        child.len()
    );
    let pdf = build_catalog_test_pdf(
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        &[
            (3, page.as_bytes()),
            (4, stream.as_bytes()),
            (5, wrapper.as_bytes()),
            (6, wrapper.as_bytes()),
            (7, bar.as_bytes()),
        ],
    );
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let assert_paint = |paths: Vec<crate::elements::PathContent>| {
        let colors: Vec<_> = paths.into_iter().map(|path| path.fill_color.unwrap().r).collect();
        assert_eq!(colors, [0.0, 1.0]);
    };
    assert_paint(doc.extract_paths(0).unwrap());

    let bytes = 2 * (parent.len() + child.len());
    assert_paint(
        doc.extract_paths_with_form_budget(0, FormPathBudget::new(4, bytes, 6))
            .unwrap(),
    );
    for (budget, message) in [
        (FormPathBudget::new(3, bytes, 6), "exceeded the page work budget"),
        (FormPathBudget::new(4, bytes - 1, 6), "exceeded the page work budget"),
        (FormPathBudget::new(4, bytes, 5), "incomplete Form path parse"),
    ] {
        let error = doc.extract_paths_with_form_budget(0, budget).unwrap_err();
        assert!(error.to_string().contains(message), "{error}");
    }
    assert_paint(doc.extract_paths(0).unwrap());
}

#[test]
fn form_extraction_reports_incomplete_decode_and_parse() {
    use super::super::paths::FormPathBudget;
    use std::io::Write;

    let content = b"0 0 10 10 re f % Form rectangle: 0 0 10 10 re f, 0 0 10 10 re f";
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(content).unwrap();
    let encoded = encoder.finish().unwrap();
    let hex: String = encoded.iter().map(|byte| format!("{byte:02x}")).collect();
    let form = format!(
        "<< /Type /XObject /Subtype /Form /BBox [0 0 10 10] \
         /Filter [/ASCIIHexDecode /FlateDecode] /Length {} >>\nstream\n{hex}>\nendstream",
        hex.len() + 1
    );
    let fresh_page = || page_with_xobject("/X1 Do", &form);
    assert!(encoded.len() < content.len() - 1);
    for (bytes, filter) in [
        (encoded.len() - 1, "ASCIIHexDecode"),
        (content.len() - 1, "FlateDecode"),
    ] {
        let error = fresh_page()
            .extract_paths_with_form_budget(0, FormPathBudget::new(1, bytes, 2))
            .unwrap_err();
        let message = error.to_string();
        assert!(
            message.contains("incomplete Form path decode") && message.contains(filter),
            "{error}"
        );
    }
    let doc = fresh_page();
    assert_eq!(
        doc.extract_paths_with_form_budget(0, FormPathBudget::new(1, content.len(), 2))
            .unwrap()
            .len(),
        2
    );
    let parse_error = doc
        .extract_paths_with_form_budget(0, FormPathBudget::new(1, content.len(), 1))
        .unwrap_err();
    assert!(
        parse_error.to_string().contains("incomplete Form path parse"),
        "{parse_error}"
    );
}

#[test]
fn form_path_budget_allows_trailing_text_at_exact_fit() {
    use super::super::paths::FormPathBudget;
    let content = "0 0 10 10 re f BT /F1 12 Tf % ET is the text-block terminator\r\n(Caption) Tj ET";
    let form = format!(
        "<< /Type /XObject /Subtype /Form /BBox [0 0 10 10] /Length {} >>\nstream\n{content}\nendstream",
        content.len()
    );
    let doc = page_with_xobject("/X1 Do", &form);
    let paths = doc
        .extract_paths_with_form_budget(0, FormPathBudget::new(1, content.len(), 2))
        .unwrap();
    assert_eq!(paths.len(), 2);
}

#[test]
fn form_paint_reads_source_even_with_a_tolerant_cache_entry() {
    let doc = page_with_xobject("/X1 Do", FORM_X1);
    doc.xobject_stream_cache
        .lock()
        .unwrap()
        .insert(ObjectRef::new(5, 0), std::sync::Arc::new(b"".to_vec()));
    assert_eq!(doc.extract_paths(0).unwrap().len(), 2);
}

#[test]
fn form_lzw_early_change_round_trips_across_code_widths() {
    use super::super::paths::FormPathBudget;
    use weezl::{BitOrder, encode::Encoder};

    let mut content = String::from("0 0 10 10 re f BT /F1 12 Tf (");
    for line in 0..150 {
        use std::fmt::Write;
        writeln!(content, "Line {line}: subtotal {}", line * 37).unwrap();
    }
    content.push_str(") Tj ET");
    let late = Encoder::new(BitOrder::Msb, 8).encode(content.as_bytes()).unwrap();
    let early = Encoder::with_tiff_size_switch(BitOrder::Msb, 8)
        .encode(content.as_bytes())
        .unwrap();
    assert!(late.len().min(early.len()) > 256 * 9 / 8);
    assert_ne!(late, early);

    let params: &[(u32, &[u8])] = &[
        (6, b"<< /EarlyChange 0 >>"),
        (7, b"[null 6 0 R]"),
        (8, b"0"),
        (9, b"1"),
        (10, b"<< /EarlyChange 8 0 R >>"),
    ];
    for (encoded, decode_parms, chained) in [
        (&early, "", true),
        (&early, "/DecodeParms [null << /EarlyChange 1 >>]", true),
        (&late, "/DecodeParms [null << /EarlyChange 0 >>]", true),
        (&late, "/DecodeParms 6 0 R", false),
        (&late, "/DecodeParms [null 6 0 R]", true),
        (&late, "/DecodeParms 7 0 R", true),
        (&late, "/DecodeParms << /EarlyChange 8 0 R >>", false),
        (&early, "/DecodeParms [null << /EarlyChange 9 0 R >>]", true),
        (&late, "/DecodeParms [null 10 0 R]", true),
    ] {
        let (filters, data) = if chained {
            let mut hex: String = encoded.iter().map(|byte| format!("{byte:02x}")).collect();
            hex.push('>');
            ("[/ASCIIHexDecode /LZWDecode]", hex.into_bytes())
        } else {
            ("/LZWDecode", encoded.clone())
        };
        let mut form = format!(
            "<< /Type /XObject /Subtype /Form /BBox [0 0 10 10] /Filter {filters} \
             {decode_parms} /Length {} >>\nstream\n",
            data.len()
        )
        .into_bytes();
        form.extend_from_slice(&data);
        form.extend_from_slice(b"\nendstream");
        let doc = page_with_xobject_objects("/X1 Do", &form, params);
        let form_ref = ObjectRef::new(5, 0);
        let stream = doc.load_object(form_ref).unwrap();
        let decoded = doc
            .decode_form_stream_with_limit(&stream, form_ref, content.len())
            .unwrap_or_else(|error| panic!("{decode_parms}: {error}"));
        assert_eq!(decoded, content.as_bytes(), "{decode_parms}");
        assert_eq!(
            doc.extract_paths_with_form_budget(0, FormPathBudget::new(1, content.len(), 2))
                .unwrap()
                .len(),
            2
        );
    }
}

/// One page that draws a curve and paints the XObject `/X1` (object 5, `x1` is its dictionary and stream).
fn page_with_xobject(paint: &str, x1: &str) -> PdfDocument {
    page_with_xobject_objects(paint, x1.as_bytes(), &[])
}

fn page_with_xobject_objects(paint: &str, x1: &[u8], extra: &[(u32, &[u8])]) -> PdfDocument {
    let content = format!("q 20 0 0 20 300 300 cm {paint} Q 100 100 m 150 200 200 200 250 100 c S");
    let page = "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R \
                /Resources << /XObject << /X1 5 0 R >> >> >>";
    let stream = format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len());
    let mut objects = vec![(3, page.as_bytes()), (4, stream.as_bytes()), (5, x1)];
    objects.extend_from_slice(extra);
    let pdf = build_catalog_test_pdf(
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        &objects,
    );
    PdfDocument::from_bytes(pdf).unwrap()
}

fn page_local_path_count(doc: &PdfDocument) -> usize {
    doc.extract_bounded_page_local_paths(0, 1 << 20, 10_000).unwrap().len()
}

const IMAGE_X1: &str = "<< /Type /XObject /Subtype /Image /Width 2 /Height 2 /ColorSpace /DeviceGray \
                        /BitsPerComponent 8 /Length 4 >>\nstream\nabcd\nendstream";
const FORM_X1: &str = "<< /Type /XObject /Subtype /Form /BBox [0 0 1 1] /Length 12 >>\nstream\n0 0 1 1 re f\nendstream";

#[test]
fn bounded_page_local_paths_read_past_an_image_xobject() {
    assert_eq!(page_local_path_count(&page_with_xobject("", IMAGE_X1)), 1);
    assert_eq!(page_local_path_count(&page_with_xobject("/X1 Do", IMAGE_X1)), 1);
}

#[test]
fn bounded_page_local_paths_stop_at_a_form_xobject_or_an_unknown_name() {
    assert_eq!(page_local_path_count(&page_with_xobject("", FORM_X1)), 1);
    assert_eq!(page_local_path_count(&page_with_xobject("/X1 Do", FORM_X1)), 0);
    assert_eq!(page_local_path_count(&page_with_xobject("/X9 Do", IMAGE_X1)), 0);
}

fn pagination_separator_test_doc(contents: &str, first_stream: &[u8], filter: &str) -> PdfDocument {
    let page = format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents {contents} /Resources <<>> >>");
    let mut stream = format!("<< /Length {} {filter} >>\nstream\n", first_stream.len()).into_bytes();
    stream.extend_from_slice(first_stream);
    stream.extend_from_slice(b"\nendstream");
    PdfDocument::from_bytes(build_catalog_test_pdf(
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        &[
            (3, page.as_bytes()),
            (4, &stream),
            (5, b"<< /Filter /FlateDecode /Length 3 >>\nstream\nbad\nendstream"),
            (6, b"[4 0 R 5 0 R]"),
        ],
    ))
    .unwrap()
}

#[test]
fn bottom_pagination_query_requires_explicit_attachment_and_a_filled_rule() {
    use crate::geometry::Rect;

    let rect = "0 g 70 90 144 0.5 re f*";
    let bottom = "/Artifact << /Type /Pagination /Attached [/Bottom] >> BDC";
    let cases = [
        (
            format!("{bottom} q 0 0 612 792 re W* n Q {rect} EMC"),
            Some(Rect::new(70.0, 90.0, 144.0, 0.5)),
        ),
        (rect.to_owned(), None),
        (
            format!("/Artifact << /Type /Pagination /Attached [/Top] >> BDC {rect} EMC"),
            None,
        ),
        (
            format!("/Artifact << /Type /Pagination /Subtype /Footer >> BDC {rect} EMC"),
            None,
        ),
        (format!("{bottom} 0 g 70 90 144 0.5 re S EMC"), None),
        (format!("{bottom} q 0 0 20 20 re W* n {rect} Q EMC"), None),
        (format!("{bottom} {rect}"), None),
        (format!("/OC /Unresolved BDC {bottom} {rect} EMC EMC"), None),
        (format!("/OC BMC {bottom} {rect} EMC EMC"), None),
        (
            format!("/OC /Unresolved BDC EMC {bottom} {rect} EMC"),
            Some(Rect::new(70.0, 90.0, 144.0, 0.5)),
        ),
    ];
    for (stream, expected) in cases {
        let document = pagination_separator_test_doc("4 0 R", stream.as_bytes(), "");
        assert_eq!(
            document.bottom_pagination_separator_bounds(0).unwrap(),
            expected.into_iter().collect::<Vec<_>>(),
            "{stream}"
        );
        let normal_paths = document.extract_paths(0).unwrap();
        assert_eq!(normal_paths.len(), 1, "{stream}");
        assert!(normal_paths[0].artifact_type.is_none(), "{stream}");
    }
}

#[test]
fn bottom_pagination_query_requires_complete_single_content() {
    use crate::geometry::Rect;
    use std::io::Write;

    let content = b"/Artifact << /Type /Pagination /Attached [/Bottom] >> BDC 70 90 144 0.5 re f* EMC";
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(content).unwrap();
    let compressed = encoder.finish().unwrap();
    let test = |data: &[u8]| pagination_separator_test_doc("4 0 R", data, "/Filter /FlateDecode");
    assert_eq!(
        test(&compressed).bottom_pagination_separator_bounds(0).unwrap(),
        [Rect::new(70.0, 90.0, 144.0, 0.5)]
    );
    assert!(
        test(&compressed[..compressed.len() - 1])
            .bottom_pagination_separator_bounds(0)
            .unwrap()
            .is_empty()
    );

    for contents in ["[4 0 R 5 0 R]", "6 0 R"] {
        let document = pagination_separator_test_doc(contents, content, "");
        assert_eq!(document.extract_paths(0).unwrap().len(), 1, "{contents}");
        assert!(
            document.bottom_pagination_separator_bounds(0).unwrap().is_empty(),
            "{contents}"
        );
    }

    let mut malformed = content.to_vec();
    malformed.extend_from_slice(b" 20 Tf");
    let document = pagination_separator_test_doc("4 0 R", &malformed, "");
    assert_eq!(document.extract_paths(0).unwrap().len(), 1);
    assert!(document.bottom_pagination_separator_bounds(0).unwrap().is_empty());
}

#[test]
fn bottom_pagination_query_declines_text_clipping_modes() {
    use crate::geometry::Rect;

    for mode in 0..=7 {
        let content = format!(
            "BT /F1 12 Tf {mode} Tr (Clip) Tj ET \
             /Artifact << /Type /Pagination /Attached [/Bottom] >> BDC 70 90 144 0.5 re f* EMC"
        );
        let document = pagination_separator_test_doc("4 0 R", content.as_bytes(), "");
        assert_eq!(document.extract_paths(0).unwrap().len(), 1, "{mode}");
        let expected = if mode < 4 {
            vec![Rect::new(70.0, 90.0, 144.0, 0.5)]
        } else {
            Vec::new()
        };
        assert_eq!(
            document.bottom_pagination_separator_bounds(0).unwrap(),
            expected,
            "{mode}"
        );
    }
}
