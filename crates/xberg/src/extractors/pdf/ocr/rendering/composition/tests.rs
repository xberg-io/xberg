use super::*;
use lopdf::{Dictionary, Document, Object, Stream, dictionary};

fn page(pdf: &mut Document) -> &mut Dictionary {
    let id = pdf.get_pages()[&1];
    pdf.get_object_mut(id).unwrap().as_dict_mut().unwrap()
}

fn source() -> Document {
    let jpeg = crate::extraction::image::jpeg_fixture::baseline_rgb_jpeg(&vec![0; 32 * 32 * 3], 32, 32);
    let mut pdf = Document::with_version("1.7");
    let pages = pdf.new_object_id();
    let image = pdf.add_object(Stream::new(
        dictionary! {
            "Type"=>"XObject","Subtype"=>"Image","Width"=>32,"Height"=>32,
            "ColorSpace"=>"DeviceRGB","BitsPerComponent"=>8,"Filter"=>"DCTDecode"
        },
        jpeg,
    ));
    let content = pdf.add_object(Stream::new(dictionary! {}, b"q 32 0 0 32 0 0 cm /Im0 Do Q\n".to_vec()));
    let page = pdf.add_object(dictionary! {
        "Type"=>"Page","Parent"=>pages,"MediaBox"=>vec![0.into(),0.into(),32.into(),32.into()],
        "Contents"=>content,"Resources"=>dictionary! {"XObject"=>dictionary! {"Im0"=>image}}
    });
    pdf.objects.insert(
        pages,
        Object::Dictionary(dictionary! {"Type"=>"Pages","Count"=>1,"Kids"=>vec![page.into()]}),
    );
    let catalog = pdf.add_object(dictionary! {"Type"=>"Catalog","Pages"=>pages});
    pdf.trailer.set("Root", catalog);
    pdf
}

fn native(mut pdf: Document) -> PdfDocument {
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    PdfDocument::from_bytes(bytes).unwrap()
}

fn content(pdf: &mut Document, content: &[u8]) {
    let stream = pdf.add_object(Stream::new(dictionary! {}, content.to_vec()));
    page(pdf).set("Contents", stream);
}

fn image(pdf: &mut Document) -> &mut Stream {
    let id = page(pdf)
        .get(b"Resources")
        .unwrap()
        .as_dict()
        .unwrap()
        .get(b"XObject")
        .unwrap()
        .as_dict()
        .unwrap()
        .get(b"Im0")
        .unwrap()
        .as_reference()
        .unwrap();
    pdf.get_object_mut(id).unwrap().as_stream_mut().unwrap()
}

fn form(pdf: &mut Document) {
    let old = page(pdf).get(b"Resources").unwrap().as_dict().unwrap().clone();
    let form = pdf.add_object(Stream::new(
        dictionary! {
            "Type"=>"XObject","Subtype"=>"Form","BBox"=>vec![0.into(),0.into(),1.into(),1.into()],"Resources"=>old
        },
        b"/Im0 Do\n".to_vec(),
    ));
    page(pdf).set("Resources", dictionary! {"XObject"=>dictionary! {"Outer"=>form}});
    content(pdf, b"q 32 0 0 32 0 0 cm /Outer Do Q\n");
}

fn allowed(doc: &PdfDocument) -> bool {
    let limits = SecurityLimits::default();
    let mut budget = crate::extractors::security::SecurityBudget::from_limits(&limits);
    let mut context = XObjectRecoveryContext::with_budget(&mut budget, None);
    can_project_stored_jpeg_page(doc, 0, &limits, &mut context).unwrap()
}

#[test]
fn a_single_complete_upright_stored_image_is_a_certified_projection() {
    let doc = native(source());
    assert_eq!(doc.page_image_handles(0).unwrap().len(), 1);
    assert!(allowed(&doc));
}

#[test]
fn annotations_overlays_forms_clipping_and_unmapped_geometry_are_not_complete_jpeg_pages() {
    let changes: [(&str, fn(&mut Document)); 9] = [
        ("text", |pdf| content(pdf, b"q 32 0 0 32 0 0 cm /Im0 Do Q BT ET\n")),
        ("vector", |pdf| {
            content(pdf, b"q 32 0 0 32 0 0 cm /Im0 Do Q 0 0 3 3 re f\n")
        }),
        ("clip", |pdf| {
            content(pdf, b"0 0 16 16 re W n q 32 0 0 32 0 0 cm /Im0 Do Q\n")
        }),
        ("inset", |pdf| content(pdf, b"q 16 0 0 16 4 4 cm /Im0 Do Q\n")),
        ("unbalanced", |pdf| content(pdf, b"q 32 0 0 32 0 0 cm /Im0 Do\n")),
        ("rotate", |pdf| page(pdf).set("Rotate", 90)),
        ("crop", |pdf| {
            page(pdf).set("CropBox", vec![0.into(), 0.into(), 16.into(), 16.into()])
        }),
        ("annotation", |pdf| {
            page(pdf).set("Annots", vec![dictionary! {"Subtype"=>"Stamp"}.into()])
        }),
        ("form", form),
    ];
    for (name, change) in changes {
        let mut pdf = source();
        change(&mut pdf);
        let doc = native(pdf);
        assert!(
            doc.page_image_handles(0)
                .unwrap()
                .iter()
                .any(|handle| handle.plain_stored_jpeg_for_ocr_recovery().is_some()),
            "valid stored source: {name}"
        );
        assert!(!allowed(&doc), "{name}");
    }
}

#[test]
fn optional_content_membership_is_not_ignored_by_stored_image_recovery() {
    let mut pdf = source();
    image(&mut pdf).dict.set(
        "OC",
        dictionary! {"Type"=>"OCG","Name"=>Object::string_literal("layer")},
    );
    assert!(!allowed(&native(pdf)));
}

#[test]
fn legacy_non_stored_pixel_recovery_retains_its_existing_mode() {
    let mut pdf = source();
    let stream = image(&mut pdf);
    stream.dict.remove(b"Filter");
    stream.content = vec![0; 32 * 32 * 3];
    stream.dict.set("Length", stream.content.len() as i64);
    assert!(allowed(&native(pdf)));
}

#[test]
fn projection_proof_obeys_the_existing_attempt_budget_and_cancellation() {
    let doc = native(source());
    let limits = SecurityLimits {
        max_iterations: 0,
        ..Default::default()
    };
    let mut budget = crate::extractors::security::SecurityBudget::from_limits(&limits);
    let mut context = XObjectRecoveryContext::with_budget(&mut budget, None);
    assert!(can_project_stored_jpeg_page(&doc, 0, &limits, &mut context).is_err());
    let token = crate::cancellation::CancellationToken::new();
    token.cancel();
    let mut context = XObjectRecoveryContext::with_budget(&mut budget, Some(&token));
    assert!(matches!(
        can_project_stored_jpeg_page(&doc, 0, &SecurityLimits::default(), &mut context),
        Err(crate::XbergError::Cancelled)
    ));
}
