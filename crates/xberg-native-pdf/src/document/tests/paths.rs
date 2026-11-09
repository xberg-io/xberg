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

/// One page that draws a curve and paints the XObject `/X1` (object 5, `x1` is its dictionary and stream).
fn page_with_xobject(paint: &str, x1: &str) -> PdfDocument {
    let content = format!("q 20 0 0 20 300 300 cm {paint} Q 100 100 m 150 200 200 200 250 100 c S");
    let page = "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R \
                /Resources << /XObject << /X1 5 0 R >> >> >>";
    let stream = format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len());
    let pdf = build_catalog_test_pdf(
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        &[(3, page.as_bytes()), (4, stream.as_bytes()), (5, x1.as_bytes())],
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
