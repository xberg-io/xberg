use super::*;

fn pdf_painting_one_jpeg_repeatedly(copies: usize) -> Vec<u8> {
    let mut pdf = Document::load_mem(&embedded_image_pdf()).unwrap();
    let page_id = pdf.get_pages()[&1];
    let operations = format!("q 72 0 0 72 0 0 cm{} Q", " /Im0 Do".repeat(copies));
    let content_id = pdf.add_object(Stream::new(dictionary! {}, operations.into_bytes()));
    pdf.get_object_mut(page_id)
        .unwrap()
        .as_dict_mut()
        .unwrap()
        .set("Contents", content_id);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}

async fn assert_whole_document_jpeg_backing(pdf: Vec<u8>, reused_source: bool) {
    const COPIES: usize = 16;
    let document = xberg_native_pdf::PdfDocument::from_bytes(pdf.clone()).unwrap();
    let handles = document.page_image_handles(0).unwrap();
    assert_eq!(
        handles.iter().map(|image| image.paint_order).collect::<Vec<_>>(),
        (0..COPIES).collect::<Vec<_>>()
    );
    let jpeg = handles[0].raw_compressed_bytes().unwrap();
    for image in &handles {
        assert_eq!(image.filter_chain, [xberg_native_pdf::PdfFilter::DCTDecode]);
        assert_eq!(image.raw_compressed_bytes().unwrap(), jpeg);
    }

    let (_guard, calls) = register_backend();
    let limits = |max_content_size| SecurityLimits {
        max_content_size,
        ..Default::default()
    };
    let config = ExtractionConfig {
        security_limits: Some(limits(64 * 1024)),
        ..Default::default()
    };
    assert!(pdf.len() < config.security_limits.as_ref().unwrap().max_content_size);
    for admitted_sources in [COPIES, COPIES / 2] {
        let cap = admitted_sources * jpeg.len();
        let ocr = OcrConfig {
            security_limits: Some(limits(cap)),
            ..Default::default()
        };
        assert_eq!(
            crate::extraction::image_decode::decode_standard_rgb8_with_security_limits(
                &jpeg,
                ocr.security_limits.as_ref().unwrap()
            )
            .unwrap()
            .dimensions(),
            (32, 24)
        );
        let recovered = super::super::raw_collection_test_support::whole(&document, &config, &ocr, BACKEND)
            .await
            .unwrap();
        let distinct_backings = recovered
            .images
            .iter()
            .map(|image| image.data.as_ptr())
            .collect::<std::collections::HashSet<_>>()
            .len();
        let (count, distinct_expected) = if reused_source {
            (COPIES, 1)
        } else {
            (admitted_sources, admitted_sources)
        };
        assert_eq!(recovered.images.len(), count, "exact paint coverage under cap {cap}");
        assert_eq!(
            distinct_backings, distinct_expected,
            "simultaneously retained encoded buffers must track actual JPEG sources"
        );
        for (index, image) in recovered.images.iter().enumerate() {
            assert_eq!(image.data.as_ref(), jpeg.as_slice());
            assert_eq!(
                image.source_path.as_deref(),
                Some(format!("xobject:page1:{index}").as_str())
            );
            assert_eq!(
                image.description.as_deref(),
                Some(
                    "recovered from raw image XObject (embedded_jpeg) after the page rasterizer produced a blank page"
                )
            );
        }
        assert_eq!(recovered.text, vec![RECOVERED; count].join("\n\n"));
        let skipped = !reused_source && admitted_sources < COPIES;
        assert_eq!(recovered.warnings.len(), 1 + usize::from(skipped));
        if skipped {
            assert_eq!(recovered.warnings[1].source, "ocr");
            assert_eq!(
                recovered.warnings[1].message,
                "Some embedded JPEG images were skipped during OCR recovery because the retained encoded payload limit was reached."
            );
        }
        assert_eq!(
            recovered.warnings[0].message,
            format!(
                "Page 1 rendered blank but contains {count} image XObject(s) the PDF rasterizer could not draw; OCR was retried on the embedded image bytes."
            )
        );
    }
    let attempts = if reused_source { COPIES * 2 } else { COPIES + COPIES / 2 };
    assert_eq!(calls.load(Ordering::SeqCst), attempts);
}

#[tokio::test]
#[serial_test::serial]
async fn raw_jpeg_collection_reuses_one_source_without_dropping_paints() {
    assert_whole_document_jpeg_backing(pdf_painting_one_jpeg_repeatedly(16), true).await;
}

#[tokio::test]
#[serial_test::serial]
async fn raw_jpeg_collection_caps_distinct_sources_in_paint_order() {
    assert_whole_document_jpeg_backing(embedded_image_pdf_with_copies(16), false).await;
}

#[tokio::test]
#[serial_test::serial]
async fn raw_jpeg_collection_preserves_wrapped_jpeg_legacy_inputs() {
    let (_guard, calls) = register_backend();
    let pdf = embedded_jpeg_after_unsupported_image_on_second_page();
    let doc = xberg_native_pdf::PdfDocument::from_bytes(pdf.clone()).unwrap();
    let handles = doc.page_image_handles(1).unwrap();
    assert_eq!(handles[0].filter_chain.len(), 2);
    let plain = handles[1].raw_compressed_bytes().unwrap();
    assert_ne!(handles[0].raw_compressed_bytes().unwrap(), plain);
    let recovered = super::super::raw_collection_test_support::whole(
        &doc,
        &ExtractionConfig::default(),
        &OcrConfig::default(),
        BACKEND,
    )
    .await
    .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(recovered.images.len(), 2);
    for (index, image) in recovered.images.iter().enumerate() {
        assert_eq!(image.data.as_ref(), plain.as_slice());
        assert_eq!(
            image.source_path.as_deref(),
            Some(format!("xobject:page2:{index}").as_str())
        );
        assert_eq!(image.format, "jpeg");
    }
}

#[tokio::test]
#[serial_test::serial]
async fn whole_document_jpeg_backing_preserves_encrypted_jpeg_legacy_recovery() {
    let (_guard, calls) = register_backend();
    let source = embedded_image_pdf();
    let plain_doc = xberg_native_pdf::PdfDocument::from_bytes(source.clone()).unwrap();
    let plain = plain_doc.page_image_handles(0).unwrap()[0]
        .raw_compressed_bytes()
        .unwrap();
    let mut pdf = Document::load_mem(&source).unwrap();
    let file_id = Object::String(vec![1; 16], lopdf::StringFormat::Hexadecimal);
    pdf.trailer.set("ID", vec![file_id.clone(), file_id]);
    let state = lopdf::EncryptionState::try_from(lopdf::EncryptionVersion::V2 {
        document: &pdf,
        owner_password: "owner",
        user_password: "",
        key_length: 128,
        permissions: lopdf::Permissions::PRINTABLE,
    })
    .unwrap();
    pdf.encrypt(&state).unwrap();
    let mut encrypted = Vec::new();
    pdf.save_to(&mut encrypted).unwrap();
    let doc = xberg_native_pdf::PdfDocument::from_bytes(encrypted.clone()).unwrap();
    assert!(doc.is_encrypted());
    assert_ne!(
        doc.page_image_handles(0).unwrap()[0].raw_compressed_bytes().unwrap(),
        plain
    );
    let recovered = recover_pipeline_document_from_image_xobjects(
        Some(&encrypted),
        &ExtractionConfig::default(),
        &OcrConfig::default(),
        BACKEND,
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(recovered.images.len(), 1);
    assert_eq!(recovered.images[0].data.as_ref(), plain.as_slice());
    assert_eq!(recovered.images[0].source_path.as_deref(), Some("xobject:page1:0"));
    assert_eq!(recovered.text, RECOVERED);
}

#[tokio::test]
#[serial_test::serial]
async fn whole_document_jpeg_backing_keeps_failure_when_no_source_fits() {
    let (_guard, calls) = register_backend();
    let pdf = embedded_image_pdf();
    let document = xberg_native_pdf::PdfDocument::from_bytes(pdf.clone()).unwrap();
    let jpeg = document.page_image_handles(0).unwrap()[0]
        .raw_compressed_bytes()
        .unwrap();
    let ocr = OcrConfig {
        security_limits: Some(SecurityLimits {
            max_content_size: jpeg.len() - 1,
            ..Default::default()
        }),
        ..Default::default()
    };
    let recovered =
        recover_pipeline_document_from_image_xobjects(Some(&pdf), &ExtractionConfig::default(), &ocr, BACKEND)
            .await
            .unwrap();
    assert!(recovered.is_none());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}
