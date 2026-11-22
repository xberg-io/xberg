use super::*;
use lopdf::{Document, Object, Stream, dictionary};

fn jpeg_page_document(paint: &[u8], mask: bool) -> Document {
    let jpeg = crate::extraction::image::jpeg_fixture::baseline_rgb_jpeg(&[0; 32 * 32 * 3], 32, 32);
    let mut doc = Document::with_version("1.7");
    let pages = doc.new_object_id();
    let mut image = dictionary! {
        "Type" => "XObject", "Subtype" => "Image", "Width" => 32, "Height" => 32,
        "ColorSpace" => "DeviceRGB", "BitsPerComponent" => 8, "Filter" => "DCTDecode"
    };
    if mask {
        image.set("Mask", vec![0.into(), 0.into(), 0.into(), 0.into(), 0.into(), 0.into()]);
    }
    let image = doc.add_object(Stream::new(image, jpeg));
    let font = doc.add_object(dictionary! {"Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica"});
    let content = doc.add_object(Stream::new(dictionary! {}, paint.to_vec()));
    let page = doc.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages, "Contents" => content,
        "MediaBox" => vec![0.into(), 0.into(), 32.into(), 32.into()],
        "Resources" => dictionary! {
            "XObject" => dictionary! {"Im0" => image},
            "Font" => dictionary! {"F1" => font},
        }
    });
    doc.objects.insert(
        pages,
        Object::Dictionary(dictionary! {"Type" => "Pages", "Count" => 1, "Kids" => vec![page.into()]}),
    );
    let catalog = doc.add_object(dictionary! {"Type" => "Catalog", "Pages" => pages});
    doc.trailer.set("Root", catalog);
    doc
}

fn native(mut doc: Document) -> PdfDocument {
    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).unwrap();
    PdfDocument::from_bytes(bytes).unwrap()
}

fn jpeg_page(paint: &[u8], mask: bool) -> PdfDocument {
    native(jpeg_page_document(paint, mask))
}

#[test]
fn only_the_exact_unmasked_full_page_jpeg_is_an_alternate_source() {
    const FULL: &[u8] = b"q 32 0 0 32 0 0 cm /Im0 Do Q\n";
    let render = (64, 64);
    let limits = SecurityLimits::default();
    let config = ExtractionConfig::default();
    let inspect = |paint, mask| certified_scan(&jpeg_page(paint, mask), 0, render, &limits, &config).unwrap();
    let complete = inspect(FULL, false).expect("the JPEG is the entire visible page");
    assert_eq!(complete.source, (32, 32));
    assert_eq!(complete.dpi, 72.0);
    assert!(complete.data.starts_with(&[0xff, 0xd8]));

    assert!(inspect(FULL, true).is_none());
    assert!(inspect(b"q 16 0 0 16 8 8 cm /Im0 Do Q\n", false).is_none());
    assert!(inspect(b"q 0 32 -32 0 32 0 cm /Im0 Do Q\n", false).is_none());
    assert!(
        inspect(
            b"q 32 0 0 32 0 0 cm /Im0 Do Q BT /F1 8 Tf 2 8 Td (Native title) Tj ET\n",
            false,
        )
        .is_none()
    );
}

#[test]
fn forced_and_explicit_ocr_keep_their_existing_raster_policy() {
    let config = ExtractionConfig {
        ocr: Some(crate::core::config::OcrConfig::default()),
        ..Default::default()
    };
    assert!(eligible(&config, 2));
    assert!(!eligible(
        &ExtractionConfig {
            force_ocr: true,
            ..config.clone()
        },
        2
    ));
    let targeted = ExtractionConfig {
        force_ocr_pages: Some(vec![2]),
        ..config.clone()
    };
    assert!(!eligible(&targeted, 2));
    assert!(eligible(&targeted, 1));
    let mut explicit = config;
    explicit.ocr.as_mut().unwrap().tesseract_config = Some(crate::types::TesseractConfig {
        psm: Some(7),
        ..Default::default()
    });
    assert!(!eligible(&explicit, 2));
}

#[test]
fn stored_scan_cannot_authorize_an_incomplete_content_prefix() {
    const COMPLETE: &[u8] = b"q 32 0 0 32 0 0 cm /Im0 Do Q\n";
    let inspect = |doc: &PdfDocument| {
        certified_scan(
            doc,
            0,
            (64, 64),
            &SecurityLimits::default(),
            &ExtractionConfig::default(),
        )
        .unwrap()
    };
    assert!(inspect(&jpeg_page(COMPLETE, false)).is_some());
    let mut malformed = COMPLETE.to_vec();
    malformed.extend_from_slice(b"(");
    let partial = xberg_native_pdf::content::parse_content_stream(&malformed).unwrap();
    assert_eq!(
        partial.len(),
        4,
        "the recovery parser silently returns the image prefix"
    );
    assert!(
        inspect(&jpeg_page(&malformed, false)).is_none(),
        "malformed text must fail closed"
    );

    let mut array = jpeg_page_document(COMPLETE, false);
    let page_id = array.get_pages()[&1];
    let page = array.get_object_mut(page_id).unwrap().as_dict_mut().unwrap();
    let original = page.get(b"Contents").unwrap().clone();
    page.set("Contents", vec![original, Object::Reference((999, 0))]);
    assert!(
        inspect(&native(array)).is_none(),
        "an unreadable array member is not an empty paint operation"
    );

    let mut capped = COMPLETE.to_vec();
    capped.extend_from_slice(b"q Q BT /F1 8 Tf 2 8 Td (Late visible title) Tj ET\n");
    assert!(
        inspect(&jpeg_page(&capped, false)).is_none(),
        "a complete image prefix cannot authorize trailing paint"
    );
}

/// A stored scan replaces the whole batch in the render phase, so the read and page phases index
/// the same one-entry set. Substituting it in the read phase alone left the page phase indexing
/// an empty render set, which panicked on the first page before any stored-scan guard could run.
#[cfg(feature = "tokio-runtime")]
#[test]
fn stored_scan_page_route_reads_its_single_batch_page() {
    const FULL: &[u8] = b"q 32 0 0 32 0 0 cm /Im0 Do Q\n";
    let document = jpeg_page(FULL, false);
    let scan = certified_scan(
        &document,
        0,
        (64, 64),
        &SecurityLimits::default(),
        &ExtractionConfig::default(),
    )
    .unwrap()
    .expect("the full-page JPEG is a stored scan");
    let (_, height_pt) = super::super::page_dimensions_pt(&document, 0);
    let config = ExtractionConfig {
        ocr: Some(crate::core::config::OcrConfig::default()),
        ..Default::default()
    };

    crate::extractors::pdf::ocr::run_on_ocr_test_stack(16 * 1024 * 1024, move || async move {
        let (_, _, _, _, _, _, texts, ..) = Box::pin(super::super::extract_with_ocr_for_page(
            None,
            None,
            #[cfg(feature = "layout-detection")]
            None,
            &config,
            None,
            0,
            true,
            Some(height_pt / scan.render.1 as f32),
            Some(PageOcrHints {
                source_dpi: Some(scan.dpi),
                known_full_page_scan: true,
                ..Default::default()
            }),
            0,
            Some(DetachedOcrPage {
                document: &document,
                stored_scan: Some(&scan),
                orientation_retry: None,
            }),
        ))
        .await
        .expect("the stored scan route reads its page");
        assert_eq!(texts.len(), 1, "the stored scan is read as exactly one page");
    });
}
