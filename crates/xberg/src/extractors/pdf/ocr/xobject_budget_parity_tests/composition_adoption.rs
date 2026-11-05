use super::super::{recover_pipeline_document_from_image_xobjects, recover_pipeline_page_from_image_xobjects};
use super::support::{BACKEND, Reply, TEXT, complete_fixture, config, observer};
use lopdf::{Document, Stream, dictionary};

fn modified_source(rotation: bool) -> (Vec<u8>, xberg_native_pdf::PdfDocument) {
    let source = complete_fixture();
    let mut pdf = Document::load_mem(&source.bytes).unwrap();
    let page = pdf.get_pages()[&1];
    if rotation {
        pdf.get_object_mut(page)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("Rotate", 90);
    } else {
        let content = pdf.add_object(Stream::new(dictionary! {}, b"q 16 0 0 12 4 4 cm /Im0 Do Q\n".to_vec()));
        pdf.get_object_mut(page)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("Contents", content);
    }
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    let doc = xberg_native_pdf::PdfDocument::from_bytes(bytes.clone()).unwrap();
    assert_eq!(doc.page_image_handles(0).unwrap().len(), 1);
    (bytes, doc)
}

#[tokio::test]
#[serial_test::serial]
async fn complete_page_adoption_abstains_from_unmapped_insets_and_rotations() {
    let (_guard, calls) = observer(Reply::Text);
    let (config, ocr) = config(16_384, 16_384);
    for rotation in [false, true] {
        let (bytes, doc) = modified_source(rotation);
        assert!(
            recover_pipeline_document_from_image_xobjects(Some(&bytes), &config, &ocr, BACKEND)
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            recover_pipeline_page_from_image_xobjects(&doc, 0, &config, &ocr, BACKEND)
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            calls.lock().unwrap().is_empty(),
            "no backend can claim this unqualified projection"
        );
    }
}

fn mixed_document() -> Vec<u8> {
    let source = complete_fixture();
    let mut pdf = Document::load_mem(&source.bytes).unwrap();
    let first = pdf.get_pages()[&1];
    let mut second = pdf.get_object(first).unwrap().as_dict().unwrap().clone();
    second.set(
        "Annots",
        vec![dictionary! {"Type"=>"Annot","Subtype"=>"Stamp","Rect"=>vec![0.into(),0.into(),8.into(),8.into()]}.into()],
    );
    let second = pdf.add_object(second);
    let pages = pdf
        .get_object(first)
        .unwrap()
        .as_dict()
        .unwrap()
        .get(b"Parent")
        .unwrap()
        .as_reference()
        .unwrap();
    let pages = pdf.get_object_mut(pages).unwrap().as_dict_mut().unwrap();
    pages.set("Count", 2);
    pages.set("Kids", vec![first.into(), second.into()]);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}

#[tokio::test]
#[serial_test::serial]
async fn a_partial_stored_jpeg_recovery_records_the_unprojected_physical_page() {
    let (_guard, calls) = observer(Reply::Text);
    let (config, ocr) = config(16_384, 16_384);
    let bytes = mixed_document();
    let result = recover_pipeline_document_from_image_xobjects(Some(&bytes), &config, &ocr, BACKEND)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result.page_texts, vec![TEXT.to_owned(), String::new()]);
    assert_eq!(calls.lock().unwrap().len(), 1);
    assert_eq!(result.page_failures.len(), 1);
    assert_eq!(
        (result.page_failures[0].page, result.page_failures[0].recovered),
        (2, false)
    );
    assert!(
        result.page_failures[0]
            .error
            .contains("complete visible page composition")
    );
}

#[tokio::test]
#[serial_test::serial]
async fn a_public_partial_recovery_exposes_the_unrecovered_page_record() {
    let (_guard, calls) = observer(Reply::Text);
    let cap = usize::try_from(super::super::estimate_png_encode_page_peak_bytes(1, 1).unwrap()).unwrap() - 1;
    let (mut config, mut ocr) = config(cap, cap);
    ocr.pipeline = Some(
        serde_json::from_value(serde_json::json!({
            "stages":[{"backend":BACKEND,"priority":1}],"quality_thresholds":{"pipeline_min_quality":0.0}
        }))
        .unwrap(),
    );
    config.ocr = Some(ocr);
    config.force_ocr = true;
    config.enable_quality_processing = false;
    let result = crate::extract(
        crate::ExtractInput::from_bytes(mixed_document(), "application/pdf", None),
        &config,
    )
    .await
    .unwrap();
    assert!(result.errors.is_empty(), "{:?}", result.errors);
    assert_eq!(result.results.len(), 1);
    assert!(result.results[0].content.contains(TEXT));
    assert_eq!(
        result.results[0].content.matches(TEXT).count(),
        1,
        "unprojected page words are not adopted"
    );
    assert!(!calls.lock().unwrap().is_empty(), "actual backend transport must fire");
    let failures = result.results[0]
        .ocr_page_failures
        .as_ref()
        .expect("public typed failure surface");
    assert_eq!(failures.len(), 1);
    assert_eq!((failures[0].page, failures[0].recovered), (2, false));
}

#[tokio::test]
#[serial_test::serial]
async fn complete_page_adoption_never_discards_a_jpeg_mask() {
    let (_guard, calls) = observer(Reply::Text);
    let (config, ocr) = config(32 * 1024, 32 * 1024);
    let mut pdf = Document::load_mem(&complete_fixture().bytes).unwrap();
    let mask = pdf.add_object(Stream::new(
        dictionary! {
            "Type"=>"XObject","Subtype"=>"Image","Width"=>32,"Height"=>24,
            "ColorSpace"=>"DeviceGray","BitsPerComponent"=>8
        },
        vec![0; 32 * 24],
    ));
    let mut changed = 0;
    for (_, object) in pdf.objects.iter_mut() {
        if let lopdf::Object::Stream(stream) = object
            && stream
                .dict
                .get(b"Filter")
                .is_ok_and(|value| value.as_name().is_ok_and(|name| name == b"DCTDecode"))
        {
            stream.dict.set("SMask", mask);
            changed += 1;
        }
    }
    assert_eq!(changed, 1);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    let doc = xberg_native_pdf::PdfDocument::from_bytes(bytes.clone()).unwrap();
    assert!(
        recover_pipeline_page_from_image_xobjects(&doc, 0, &config, &ocr, BACKEND)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        recover_pipeline_document_from_image_xobjects(Some(&bytes), &config, &ocr, BACKEND)
            .await
            .unwrap()
            .is_none()
    );
    assert!(calls.lock().unwrap().is_empty());
}
