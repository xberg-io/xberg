use super::super::recover_pipeline_page_from_image_xobjects;
use super::support::{BACKEND, Reply, complete_fixture, config, observer};
use lopdf::{Document, Object};

const JP2: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../xberg-native-pdf/tests/fixtures/jpx/gh1885_indices_grey.jp2"
));

fn source(raw: bool) -> xberg_native_pdf::PdfDocument {
    let mut pdf = Document::load_mem(&complete_fixture().bytes).unwrap();
    let mut changed = 0;
    for object in pdf.objects.values_mut() {
        if let Object::Stream(stream) = object
            && stream
                .dict
                .get(b"Subtype")
                .is_ok_and(|value| value.as_name().is_ok_and(|name| name == b"Image"))
        {
            stream.dict.set("Width", 120);
            stream.dict.set("Height", 40);
            if raw {
                stream.dict.remove(b"Filter");
                stream.set_content(vec![128; 120 * 40 * 3]);
            } else {
                stream.dict.set("ColorSpace", "DeviceGray");
                stream.dict.set("Filter", "JPXDecode");
                stream.set_content(JP2.to_vec());
            }
            changed += 1;
        }
    }
    assert_eq!(changed, 1);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    let doc = xberg_native_pdf::PdfDocument::from_bytes(bytes).unwrap();
    assert_eq!(doc.page_image_handles(0).unwrap().len(), 1);
    doc
}

#[tokio::test]
#[serial_test::serial]
async fn bounded_wrapper_never_decodes_raw_pixel_images_before_backend_admission() {
    let (_guard, calls) = observer(Reply::Text);
    let (config, ocr) = config(32 * 1024, 32 * 1024);
    let doc = source(true);
    let decoded = doc.page_image_handles(0).unwrap()[0].decode().unwrap();
    assert_eq!((decoded.width(), decoded.height()), (120, 40));
    assert!(
        recover_pipeline_page_from_image_xobjects(&doc, 0, &config, &ocr, BACKEND)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        calls.lock().unwrap().is_empty(),
        "raw pixels must never become a newly encoded PNG in this bounded route"
    );
}

#[tokio::test]
#[serial_test::serial]
async fn bounded_wrapper_hands_original_jpeg2000_bytes_to_the_backend() {
    let (_guard, calls) = observer(Reply::Text);
    let (config, ocr) = config(32 * 1024, 32 * 1024);
    let doc = source(false);
    let result = recover_pipeline_page_from_image_xobjects(&doc, 0, &config, &ocr, BACKEND)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result.images.len(), 1);
    assert_eq!(result.images[0].format, "jpeg2000");
    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(
        calls[0].bytes, JP2,
        "no native JPEG2000 pixel decode or PNG re-encoding"
    );
}
