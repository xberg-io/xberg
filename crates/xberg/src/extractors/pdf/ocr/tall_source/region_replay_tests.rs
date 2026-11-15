use super::*;
use crate::{ExtractInput, ExtractionConfig};
use sha2::{Digest, Sha256};

#[test]
fn projected_ranges_follow_full_token_order_and_reject_an_omitted_owner() {
    let words = [StockWord {
        text: "same",
        transcript_bytes: Some(5..9),
        bounds: RegionBox {
            left: 1,
            top: 1,
            right: 2,
            bottom: 2,
        },
    }];
    let two = project("same same", "same\n\nsame", &words).unwrap();
    assert_eq!(two[0].transcript_bytes, Some(6..10));
    assert!(project("same same", "same", &words).is_none());
}

#[tokio::test]
#[ignore = "requires archived PDF bytes and an evidence directory"]
async fn replay_archived_selected_hocr_page_8() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("xberg::ocr::processor::execution=trace,xberg::extractors::pdf::ocr=trace")
        .with_ansi(false)
        .try_init();
    let archived = std::path::PathBuf::from(std::env::var_os("XBERG_2502_ARCHIVED_PAGES").unwrap());
    let evidence = std::path::PathBuf::from(std::env::var_os("XBERG_2502_EVIDENCE").unwrap());
    let bytes = std::fs::read(archived.join("page-8.pdf")).unwrap();
    let hash = hex::encode(Sha256::digest(&bytes));
    assert_eq!(hash, "cb7daf248063917ff62678543192bd0cd2e6fec46fab9b80378b5c7ad996018a");
    let model = xberg_tesseract::bundled_eng_traineddata().unwrap();
    assert_eq!(
        hex::encode(Sha256::digest(model)),
        "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2"
    );
    let tessdata = tempfile::tempdir().unwrap();
    std::fs::write(tessdata.path().join("eng.traineddata"), model).unwrap();
    let mut ocr = OcrConfig {
        tessdata_path: Some(tessdata.path().to_path_buf()),
        backend_options: Some(serde_json::json!({"use_cache": false, "source_region_reconciliation": false})),
        ..Default::default()
    };
    let mut config = ExtractionConfig {
        use_cache: false,
        ocr: Some(ocr.clone()),
        pages: Some(serde_json::from_value(serde_json::json!({"extract_pages": true})).unwrap()),
        ..Default::default()
    };
    let stock = crate::extract(
        ExtractInput::from_bytes(bytes.clone(), "application/pdf", None),
        &config,
    )
    .await
    .unwrap();
    ocr.backend_options.as_mut().unwrap()["source_region_reconciliation"] = true.into();
    let tess = ocr.tesseract_config.get_or_insert_default();
    tess.preprocessing = Some(crate::types::ImagePreprocessingConfig {
        deskew: false,
        ..Default::default()
    });
    config.ocr = Some(ocr.clone());
    assert!(CAPTURE.lock().replace(ReplayCapture::default()).is_none());
    let live = crate::extract(
        ExtractInput::from_bytes(bytes.clone(), "application/pdf", None),
        &config,
    )
    .await;
    let capture = CAPTURE.lock().take().unwrap();
    let mut live = live.unwrap();
    assert_eq!(capture.count, 1, "expected one unrotated selected hOCR read");
    let read = capture.first.unwrap();
    assert!(stock.errors.is_empty() && live.errors.is_empty());
    let [doc] = live.results.as_mut_slice() else {
        panic!("expected a single document")
    };
    let [page] = doc.pages.as_deref_mut().unwrap() else {
        panic!("expected one page")
    };
    assert!(doc.tables.is_empty() && page.tables.is_empty());
    let (read, raster) = read;
    let pdf = super::super::PdfDocument::from_bytes(bytes).unwrap();
    let attempt = Attempt::new(&pdf, 0, Some(raster), &config, &ocr);
    let backend = crate::plugins::registry::get_ocr_backend_registry()
        .read()
        .get("tesseract")
        .unwrap();
    // The archived witness only selects the line. The crop is the union of the mapped IDs. ~keep
    let witness = RegionBox {
        left: 195,
        top: 145,
        right: 365,
        bottom: 183,
    };
    let (document, page_text) = recover(&attempt, &backend, &ocr, &read, witness, (&doc.content, &page.content))
        .await
        .unwrap()
        .expect("expected admitted crop, agreeing OCR reads, and both projections");
    doc.content = document;
    page.content = page_text;

    for (arm, document) in [("stock", &stock.results[0]), ("live", &live.results[0])] {
        let dir = evidence.join(arm);
        std::fs::create_dir_all(&dir).unwrap();
        let report = serde_json::json!({"source_page": 8, "isolated_sha256": hash, "document": document});
        std::fs::write(dir.join("page-8.json"), serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
}

#[test]
#[ignore = "requires the pinned archived page 13 PDF"]
fn archived_page_13_tight_word_clips_ink_but_complete_line_owns_it() {
    let root = std::path::PathBuf::from(std::env::var_os("XBERG_2502_ARCHIVED_PAGES").unwrap());
    let bytes = std::fs::read(root.join("page-13.pdf")).unwrap();
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        "1cd54c866a321d90433ff575c282fe8ba6207156f1d6acb7963711f603833543"
    );
    let pdf = super::super::PdfDocument::from_bytes(bytes).unwrap();
    let config = ExtractionConfig::default();
    let ocr = OcrConfig::default();
    let attempt = Attempt::new(&pdf, 0, Some((1600, 4090, 0)), &config, &ocr);
    let source = IndexedSource::load(&attempt).expect("pinned native Indexed pixels");
    let column = RegionBox {
        left: 0,
        top: 5287,
        right: 742,
        bottom: 5773,
    };
    let tight = RegionBox {
        left: 320,
        top: 5455,
        right: 337,
        bottom: 5469,
    };
    let line = RegionBox {
        left: 319,
        top: 5452,
        right: 439,
        bottom: 5474,
    };
    assert!(source.certify_line_ink(tight, column).is_none());
    assert!(source.source_line_column(line).is_some());
    assert!(source.source_line_column(RegionBox { right: 1800, ..line }).is_none());
    assert!(source.certify_line_ink(line, column).is_some());
    assert!(
        source
            .certify_line_ink(line, RegionBox { right: 430, ..column })
            .is_none()
    );
}

#[tokio::test]
#[ignore = "requires the pinned archived page 13 PDF"]
async fn archived_page_13_bounded_read_and_final_receipts() {
    let root = std::path::PathBuf::from(std::env::var_os("XBERG_2502_ARCHIVED_PAGES").unwrap());
    let bytes = std::fs::read(root.join("page-13.pdf")).unwrap();
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        "1cd54c866a321d90433ff575c282fe8ba6207156f1d6acb7963711f603833543"
    );
    let tessdata = tempfile::tempdir().unwrap();
    let model = xberg_tesseract::bundled_eng_traineddata().unwrap();
    assert_eq!(
        hex::encode(Sha256::digest(model)),
        "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2"
    );
    std::fs::write(tessdata.path().join("eng.traineddata"), model).unwrap();
    let ocr = OcrConfig {
        tessdata_path: Some(tessdata.path().to_path_buf()),
        backend_options: Some(serde_json::json!({"use_cache": false})),
        ..Default::default()
    };
    let mut config = ExtractionConfig {
        use_cache: false,
        ocr: Some(ocr.clone()),
        pages: Some(serde_json::from_value(serde_json::json!({"extract_pages": true})).unwrap()),
        ..Default::default()
    };
    let stock = crate::extract(
        ExtractInput::from_bytes(bytes.clone(), "application/pdf", None),
        &config,
    )
    .await
    .unwrap();
    config.ocr.as_mut().unwrap().backend_options.as_mut().unwrap()["source_region_reconciliation"] = true.into();
    let capture = &crate::extraction::derive::FINAL_OCR_REPLAY;
    assert!(capture.lock().replace(Vec::new()).is_none());
    let live = crate::extract(
        ExtractInput::from_bytes(bytes.clone(), "application/pdf", None),
        &config,
    )
    .await;
    let receipts = capture.lock().take().unwrap();
    let mut live = live.unwrap();
    assert!(stock.errors.is_empty() && live.errors.is_empty());
    let document = &mut live.results[0];
    let page = &mut document.pages.as_mut().unwrap()[0];
    let selected: Vec<_> = receipts.into_iter().filter(|(_, id, _, _)| *id == 117153).collect();
    let [(1, _, dr, pr)] = selected.as_slice() else {
        panic!("expected one page-scoped final target receipt");
    };

    let pdf = super::super::PdfDocument::from_bytes(bytes).unwrap();
    let attempt = Attempt::new(&pdf, 0, Some((1600, 4090, 0)), &config, &ocr);
    let source = IndexedSource::load(&attempt).unwrap();
    let column = RegionBox {
        left: 0,
        top: 5287,
        right: 742,
        bottom: 5773,
    };
    let line = RegionBox {
        left: 319,
        top: 5452,
        right: 439,
        bottom: 5474,
    };
    assert!(source.certify_line_ink(line, column).is_some());
    let backend = crate::plugins::registry::get_ocr_backend_registry()
        .read()
        .get("tesseract")
        .unwrap();
    let [first, second] = read_pair(&attempt, &source, &backend, &ocr, line, column)
        .await
        .unwrap()
        .unwrap();
    assert!(first.iter().map(|word| word.text.as_str()).eq(["View", "the", "blog"]));
    assert!(second.iter().map(|word| word.text.as_str()).eq(["View", "the", "blog"]));
    let (before, before_page) = (
        &stock.results[0].content,
        &stock.results[0].pages.as_ref().unwrap()[0].content,
    );
    assert_eq!((&document.content, &page.content), (before, before_page));
    assert_eq!(document.content.get(dr.clone()), Some("wi"));
    assert_eq!(page.content.get(pr.clone()), Some("wi"));
    let cancel = crate::cancellation::CancellationToken::new();
    cancel.cancel();
    let cancelled = Attempt {
        cancel: Some(&cancel),
        ..attempt
    };
    assert!(matches!(
        read_pair(&cancelled, &source, &backend, &ocr, line, column).await,
        Err(crate::XbergError::Cancelled)
    ));
    assert_eq!((&document.content, &page.content), (before, before_page));
    super::publish_verified_pair(
        (&mut document.content, &mut page.content),
        (dr.clone(), pr.clone()),
        "wi",
        &first[0].text,
    )
    .unwrap();
    let expected =
        |text: &str, range: &std::ops::Range<usize>| format!("{}View{}", &text[..range.start], &text[range.end..]);
    assert_eq!(document.content, expected(before, dr));
    assert_eq!(page.content, expected(before_page, pr));
    assert_eq!(document.tables.len(), 3);
    for key in ["tables", "annotations"] {
        assert_eq!(
            serde_json::to_value(&stock.results[0]).unwrap().get(key),
            serde_json::to_value(&*document).unwrap().get(key)
        );
    }
}

#[test]
#[ignore = "requires exact archived positioned inventory and PDF"]
fn archived_page_13_discovers_five_anchored_regions_from_all_stock_words() {
    let root = std::path::PathBuf::from(std::env::var_os("XBERG_2502_ARCHIVED_PAGES").unwrap());
    let evidence = std::path::PathBuf::from(std::env::var_os("XBERG_2502_NATIVE_INVENTORY").unwrap());
    let bytes = std::fs::read(root.join("page-13.pdf")).unwrap();
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        "1cd54c866a321d90433ff575c282fe8ba6207156f1d6acb7963711f603833543"
    );
    let pdf = super::super::PdfDocument::from_bytes(bytes).unwrap();
    let bounds = |[left, top, right, bottom]: [u32; 4]| RegionBox {
        left,
        top,
        right,
        bottom,
    };
    let (words, tables): (Vec<[u32; 4]>, Vec<[u32; 4]>) =
        serde_json::from_slice(&std::fs::read(evidence).unwrap()).unwrap();
    let words: Vec<_> = words.into_iter().map(bounds).collect();
    let tables: Vec<_> = tables.into_iter().map(bounds).collect();
    assert_eq!((words.len(), tables.len()), (498, 3));
    let config = ExtractionConfig::default();
    let ocr = OcrConfig::default();
    let attempt = Attempt::new(&pdf, 0, Some((1600, 4090, 0)), &config, &ocr);
    let source = IndexedSource::load(&attempt).unwrap();
    let discovered = super::super::region_probe::discover_anchored_regions(&source, &words, &tables, 16).unwrap();
    assert_eq!(discovered.len(), 5);
    assert!(
        discovered
            .iter()
            .any(|(_, crop)| (crop.left, crop.top, crop.right, crop.bottom) == (319, 5453, 440, 5476))
    );
    assert!(super::super::region_probe::discover_anchored_regions(&source, &words, &tables, 8).is_none());
}

#[tokio::test]
#[ignore = "requires the exact archived page 13 PDF"]
async fn public_region_recovery_preserves_other_surfaces_and_abstains_on_multipage() {
    let root = std::path::PathBuf::from(std::env::var_os("XBERG_2502_ARCHIVED_PAGES").unwrap());
    let bytes = std::fs::read(root.join("page-13.pdf")).unwrap();
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        "1cd54c866a321d90433ff575c282fe8ba6207156f1d6acb7963711f603833543"
    );
    let mut stock_config = ExtractionConfig {
        use_cache: false,
        ocr: Some(OcrConfig {
            backend_options: Some(serde_json::json!({"use_cache": false})),
            tesseract_config: Some(crate::types::TesseractConfig {
                preprocessing: Some(crate::types::ImagePreprocessingConfig {
                    deskew: false,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }),
        pages: Some(serde_json::from_value(serde_json::json!({"extract_pages": true})).unwrap()),
        ..Default::default()
    };
    let model = xberg_tesseract::bundled_eng_traineddata().unwrap();
    assert_eq!(
        hex::encode(Sha256::digest(model)),
        "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2"
    );
    let models = tempfile::tempdir().unwrap();
    std::fs::write(models.path().join("eng.traineddata"), model).unwrap();
    stock_config.ocr.as_mut().unwrap().tessdata_path = Some(models.path().to_path_buf());
    let mut enabled = stock_config.clone();
    enabled.ocr.as_mut().unwrap().backend_options.as_mut().unwrap()["source_region_recovery"] = true.into();

    async fn extract(bytes: &[u8], config: &ExtractionConfig) -> crate::types::ExtractedDocument {
        let output = crate::extract(
            ExtractInput::from_bytes(bytes.to_vec(), "application/pdf", None),
            config,
        )
        .await
        .unwrap();
        assert!(output.errors.is_empty());
        let [document] = output.results.try_into().ok().unwrap();
        document
    }

    let stock = extract(&bytes, &stock_config).await;
    let live = extract(&bytes, &enabled).await;
    let expected = |text: &str| {
        let old = "

wi

the blog";
        assert_eq!(text.matches(old).count(), 1);
        text.replacen(
            old,
            "

View

the blog",
            1,
        )
    };
    assert_eq!(live.content, expected(&stock.content));
    assert_eq!(
        live.pages.as_ref().unwrap()[0].content,
        expected(&stock.pages.as_ref().unwrap()[0].content)
    );
    let stocks = serde_json::to_value(&stock).unwrap();
    let lives = serde_json::to_value(&live).unwrap();
    assert_eq!(stock.tables.len(), 3);
    for key in ["tables", "annotations"] {
        assert_eq!(stocks.get(key), lives.get(key));
    }

    stock_config.include_document_structure = true;
    enabled.include_document_structure = true;
    assert_eq!(
        serde_json::to_value(extract(&bytes, &stock_config).await).unwrap(),
        serde_json::to_value(extract(&bytes, &enabled).await).unwrap()
    );
    stock_config.include_document_structure = false;
    enabled.include_document_structure = false;
    let mut two_pages = lopdf::Document::load_mem(&bytes).unwrap();
    let page_id = two_pages.get_pages()[&1];
    let root_id = two_pages
        .catalog()
        .unwrap()
        .get(b"Pages")
        .unwrap()
        .as_reference()
        .unwrap();
    let mut page = two_pages.get_dictionary(page_id).unwrap().clone();
    page.set("Parent", root_id);
    two_pages
        .objects
        .insert(page_id, lopdf::Object::Dictionary(page.clone()));
    let second = two_pages.add_object(page);
    let tree = two_pages.get_dictionary_mut(root_id).unwrap();
    tree.set("Count", 2);
    tree.set(
        "Kids",
        vec![lopdf::Object::Reference(page_id), lopdf::Object::Reference(second)],
    );
    let mut multi = Vec::new();
    two_pages.save_to(&mut multi).unwrap();
    let multi_stock = extract(&multi, &stock_config).await;
    let multi_enabled = extract(&multi, &enabled).await;
    assert_eq!(multi_stock.pages.as_ref().unwrap().len(), 2);
    assert_eq!(
        serde_json::to_value(multi_stock).unwrap(),
        serde_json::to_value(multi_enabled).unwrap()
    );
}
