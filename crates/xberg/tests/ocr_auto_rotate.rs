//! Regression tests for explicit OCR orientation and implicit PDF recovery.
//!
//! Tesseract's `DetectOrientationScript` corrupts engine memory in the vendored
//! build (SIGABRT/SIGSEGV, uncatchable), so orientation detection runs on the
//! ONNX PP-LCNet classifier instead. This test drives the tesseract backend with
//! `auto_rotate: true` end-to-end; before the fix the whole test process died
//! with a signal.
//!
//! Set `XBERG_ORIENTATION_TEST_MODEL` to the pinned PP-LCNet ONNX artifact
//! and enable `bundle-tessdata-eng` to run the isolated PDF cases locally.
//! CI requires the model with `XBERG_REQUIRE_MODELS=1`.
#![cfg(all(feature = "ocr", any(feature = "auto-rotate", feature = "pdf")))]

#[cfg(feature = "auto-rotate")]
#[tokio::test]
async fn should_extract_with_auto_rotate_without_crashing() {
    let bytes = std::fs::read("../../test_documents/images/balance_sheet_1.png").expect("fixture must exist");
    let config = xberg::ExtractionConfig {
        ocr: Some(xberg::core::config::OcrConfig {
            backend: "tesseract".to_string(),
            language: vec!["eng".to_string()],
            auto_rotate: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let input = xberg::core::config::ExtractInput {
        kind: xberg::core::config::ExtractInputKind::Bytes,
        bytes: Some(bytes),
        mime_type: Some("image/png".to_string()),
        ..Default::default()
    };

    let result = xberg::extract(input, &config).await.expect("extraction must succeed");
    let text = result.results.first().map(|d| d.content.clone()).unwrap_or_default();
    assert!(
        !text.trim().is_empty(),
        "OCR with auto_rotate must still produce text output"
    );
}

#[cfg(all(feature = "pdf", any(feature = "auto-rotate", feature = "auto-rotate-tract")))]
mod helpers;

#[cfg(all(feature = "pdf", any(feature = "auto-rotate", feature = "auto-rotate-tract")))]
fn in_orientation_test_process(case: &str, warm: bool) -> bool {
    use std::{env, fs, process::Command};

    const MODEL: &str = concat!(
        "models--xberg-io--paddleocr-onnx-models/snapshots/bc5ec866cf0e798e667808dfa51b0ba8ad0dafc8/",
        "v2/classifiers/PP-LCNet_x1_0_doc_ori.onnx"
    );
    const CHILD_CASE: &str = "XBERG_ORIENTATION_TEST_CHILD";
    if env::var(CHILD_CASE).as_deref() == Ok(case) {
        return true;
    }
    let Some(model) = env::var_os("XBERG_ORIENTATION_TEST_MODEL") else {
        assert_ne!(
            env::var("XBERG_REQUIRE_MODELS").as_deref(),
            Ok("1"),
            "XBERG_ORIENTATION_TEST_MODEL must point to the pinned document-orientation ONNX file"
        );
        return false;
    };

    let root = tempfile::tempdir().expect("isolated orientation test cache");
    let hub = root.path().join("hub");
    fs::create_dir(&hub).expect("empty Hugging Face Hub cache");
    if warm {
        use sha2::{Digest, Sha256};

        let destination = hub.join(MODEL);
        fs::create_dir_all(destination.parent().expect("model cache parent")).expect("model cache directories");
        let bytes = fs::read(model).expect("read the pinned orientation model");
        assert_eq!(
            hex::encode(Sha256::digest(&bytes)),
            "6b742aebce6f0f7f71f747931ac7becfc7c96c51641e14943b291eeb334e7947"
        );
        fs::write(destination, bytes).expect("copy the pinned orientation model into the test cache");
    }

    let tessdata = root.path().join("tessdata");
    fs::create_dir(&tessdata).expect("private Tesseract data directory");
    let english = xberg_tesseract::bundled_eng_traineddata().expect("enable bundle-tessdata-eng for these tests");
    fs::write(tessdata.join("eng.traineddata"), english).expect("seed pinned English data");

    // A fresh process isolates both environment variables and the process-global detector. ~keep
    let output = Command::new(env::current_exe().expect("test executable"))
        .args(["--exact", case, "--nocapture"])
        .env(CHILD_CASE, case)
        .env("HF_HOME", root.path())
        .env("HF_HUB_CACHE", &hub)
        .env("HF_HUB_OFFLINE", "1")
        .env("TESSDATA_PREFIX", tessdata)
        .env("XBERG_CACHE_DIR", root.path().join("xberg"))
        .output()
        .expect("run isolated orientation test");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("test result: ok. 1 passed; 0 failed; 0 ignored;"),
        "{case} child exited {}\n{stdout}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    false
}

#[cfg(all(feature = "pdf", any(feature = "auto-rotate", feature = "auto-rotate-tract")))]
fn rotated_pdf() -> (Vec<u8>, xberg::ExtractionConfig) {
    use xberg::core::config::PageConfig;

    let path = helpers::get_test_file_path("pdf/ocr_test_rotated_90.pdf");
    let bytes = std::fs::read(path).expect("rotated PDF fixture");
    assert!(bytes.starts_with(b"%PDF-"));
    let config = xberg::ExtractionConfig {
        force_ocr: true,
        use_cache: false,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    assert!(config.ocr.is_none());
    (bytes, config)
}

#[cfg(all(feature = "pdf", any(feature = "auto-rotate", feature = "auto-rotate-tract")))]
#[test]
fn implicit_pdf_ocr_recovers_a_low_confidence_rotated_page() {
    if !in_orientation_test_process("implicit_pdf_ocr_recovers_a_low_confidence_rotated_page", true) {
        return;
    }
    use xberg::core::config::{ExtractionConfig, OcrConfig};

    let (bytes, config) = rotated_pdf();

    let extract = |config: &ExtractionConfig| {
        helpers::extract_bytes_document_blocking(&bytes, "application/pdf", config).expect("PDF OCR succeeds")
    };
    let normalize = |text: &str| text.split_whitespace().collect::<Vec<_>>().join(" ");
    let expected = "Docling bundles PDF document conversion to JSON and Markdown in an easy self contained package";
    let implicit = extract(&config);
    let explicit = extract(&ExtractionConfig {
        ocr: Some(OcrConfig {
            auto_rotate: true,
            ..Default::default()
        }),
        ..config.clone()
    });
    assert_eq!(normalize(&explicit.content), expected, "cached orientation control");
    let disabled = extract(&ExtractionConfig {
        ocr: Some(OcrConfig {
            auto_rotate: false,
            ..Default::default()
        }),
        ..config
    });
    assert_eq!(normalize(&disabled.content), "zy O O");
    assert!(
        disabled
            .processing_warnings
            .iter()
            .any(|warning| warning.message.contains("recognition noise"))
    );

    let pages = implicit.pages.as_ref().expect("page extraction is enabled");
    assert_eq!(pages.len(), 1);
    assert!(pages[0].ocr_confidence.is_some(), "Tesseract must have run");
    assert_eq!(normalize(&implicit.content), expected, "implicit rotated PDF OCR");
    assert_eq!(normalize(&pages[0].content), expected);
    assert!(
        implicit
            .processing_warnings
            .iter()
            .all(|warning| warning.source != "ocr"),
        "{:?}",
        implicit.processing_warnings
    );
}

#[cfg(all(feature = "pdf", any(feature = "auto-rotate", feature = "auto-rotate-tract")))]
#[test]
fn implicit_pdf_ocr_keeps_its_original_warning_when_no_orientation_model_is_available() {
    if !in_orientation_test_process(
        "implicit_pdf_ocr_keeps_its_original_warning_when_no_orientation_model_is_available",
        false,
    ) {
        return;
    }
    use xberg::core::config::{ExtractionConfig, OcrConfig};

    let (bytes, config) = rotated_pdf();
    let extract = |config: &ExtractionConfig| {
        helpers::extract_bytes_document_blocking(&bytes, "application/pdf", config).expect("PDF OCR succeeds")
    };
    let implicit = extract(&config);
    let disabled = extract(&ExtractionConfig {
        ocr: Some(OcrConfig {
            auto_rotate: false,
            ..Default::default()
        }),
        ..config
    });
    assert_eq!(implicit.content, disabled.content);
    assert_eq!(
        implicit.content.split_whitespace().collect::<Vec<_>>(),
        ["zy", "O", "O"]
    );
    let warnings = |doc: &xberg::ExtractedDocument| {
        doc.processing_warnings
            .iter()
            .map(|w| (w.source.clone(), w.message.clone()))
            .collect::<Vec<_>>()
    };
    assert_eq!(warnings(&implicit), warnings(&disabled));
    assert!(
        implicit
            .processing_warnings
            .iter()
            .any(|w| w.message.contains("recognition noise"))
    );
}

#[cfg(all(feature = "pdf", any(feature = "auto-rotate", feature = "auto-rotate-tract")))]
#[test]
fn implicit_upright_pdf_ocr_preserves_text_confidence_and_geometry() {
    if !in_orientation_test_process("implicit_upright_pdf_ocr_preserves_text_confidence_and_geometry", true) {
        return;
    }
    use xberg::core::config::{ExtractionConfig, OcrConfig, PageConfig};

    let bytes = std::fs::read(helpers::get_test_file_path("pdf/ocr_test.pdf")).expect("upright PDF fixture");
    let config = ExtractionConfig {
        force_ocr: true,
        use_cache: false,
        include_document_structure: true,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let extract = |config: &ExtractionConfig| {
        helpers::extract_bytes_document_blocking(&bytes, "application/pdf", config).expect("PDF OCR succeeds")
    };
    let implicit = extract(&config);
    let disabled = extract(&ExtractionConfig {
        ocr: Some(OcrConfig {
            auto_rotate: false,
            ..Default::default()
        }),
        ..config
    });
    let expected = "Docling bundles PDF document conversion to JSON and Markdown in an easy self contained package";
    assert_eq!(
        implicit.content.split_whitespace().collect::<Vec<_>>().join(" "),
        expected
    );
    assert_eq!(implicit.content, disabled.content);
    assert!(implicit.processing_warnings.is_empty());
    let implicit_pages = implicit.pages.as_ref().expect("implicit page payload");
    let disabled_pages = disabled.pages.as_ref().expect("explicit page payload");
    assert_eq!(implicit_pages.len(), 1);
    assert_eq!(implicit_pages[0].content, disabled_pages[0].content);
    let confidence = |page: &xberg::types::PageContent| {
        page.ocr_confidence
            .as_ref()
            .and_then(|confidence| confidence.score)
            .expect("actual OCR page confidence")
    };
    assert_eq!(confidence(&implicit_pages[0]), confidence(&disabled_pages[0]));

    let implicit_structure = implicit.document.as_ref().expect("implicit OCR structure");
    let disabled_structure = disabled.document.as_ref().expect("explicit OCR structure");
    assert_eq!(
        implicit_structure
            .nodes
            .iter()
            .filter(|node| node.bbox.is_some())
            .count(),
        3
    );
    assert_eq!(
        serde_json::to_value(implicit_structure).expect("implicit geometry"),
        serde_json::to_value(disabled_structure).expect("explicit geometry")
    );
}

#[cfg(all(feature = "pdf", feature = "auto-rotate-tract"))]
fn scan_270_text(ocr: Option<xberg::OcrConfig>) -> String {
    let path = helpers::get_test_file_path("pdf/ocr_test_rotated_270.pdf");
    let bytes = std::fs::read(path).expect("rotated PDF fixture");
    let config = xberg::ExtractionConfig {
        force_ocr: true,
        use_cache: false,
        ocr,
        ..Default::default()
    };
    helpers::extract_bytes_document_blocking(&bytes, "application/pdf", &config)
        .expect("PDF OCR succeeds")
        .content
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(all(feature = "pdf", feature = "auto-rotate-tract"))]
#[test]
fn implicit_pdf_ocr_recovers_the_empty_270_degree_scan() {
    if !in_orientation_test_process("implicit_pdf_ocr_recovers_the_empty_270_degree_scan", true) {
        return;
    }
    let expected = "Docling bundles PDF document conversion to JSON and Markdown in an easy self contained package";
    assert_eq!(
        scan_270_text(Some(xberg::OcrConfig::default())),
        "",
        "explicit orientation opt-out"
    );
    let implicit = scan_270_text(None);
    assert_eq!(
        scan_270_text(Some(xberg::OcrConfig {
            auto_rotate: true,
            ..Default::default()
        })),
        expected,
        "explicit cached orientation control"
    );
    assert_eq!(implicit, expected, "implicit cached orientation recovery");
}

#[cfg(all(feature = "pdf", feature = "auto-rotate-tract"))]
#[test]
fn implicit_pdf_ocr_keeps_the_270_degree_scan_empty_without_a_cached_model() {
    if !in_orientation_test_process(
        "implicit_pdf_ocr_keeps_the_270_degree_scan_empty_without_a_cached_model",
        false,
    ) {
        return;
    }
    assert_eq!(scan_270_text(None), "");
}

#[cfg(all(feature = "pdf", any(feature = "auto-rotate", feature = "auto-rotate-tract")))]
#[test]
fn implicit_pdf_ocr_recovers_the_complete_rotated_table() {
    if !in_orientation_test_process("implicit_pdf_ocr_recovers_the_complete_rotated_table", true) {
        return;
    }
    use xberg::core::config::{ExtractionConfig, OcrConfig};

    let bytes = include_bytes!("fixtures/ocr/rotated_financial_table.pdf");
    let config = ExtractionConfig {
        use_cache: false,
        ..Default::default()
    };
    let extract = |config: &ExtractionConfig| {
        helpers::extract_bytes_document_blocking(bytes, "application/pdf", config).expect("table PDF OCR succeeds")
    };
    assert_recovered_financial_table(&extract(&config));

    let disabled = extract(&ExtractionConfig {
        ocr: Some(OcrConfig {
            auto_rotate: false,
            ..Default::default()
        }),
        ..config
    });
    assert!(disabled.tables.is_empty());
    assert!(disabled.content.split_whitespace().count() < FINANCIAL_TABLE_WORDS);
    assert!(
        disabled
            .processing_warnings
            .iter()
            .any(|warning| warning.message.contains("recognition noise"))
    );
}

#[cfg(all(
    paddle_ocr,
    feature = "pdf",
    any(feature = "auto-rotate", feature = "auto-rotate-tract")
))]
#[test]
fn implicit_detached_pdf_ocr_scales_recovered_table_to_page() {
    if !in_orientation_test_process("implicit_detached_pdf_ocr_scales_recovered_table_to_page", true) {
        return;
    }
    use xberg::ExtractionConfig;
    use xberg::core::config::{ImageExtractionConfig, PageConfig};

    let rotated = include_bytes!("fixtures/ocr/rotated_financial_table.pdf");
    let mixed = prepend_native_page(rotated);
    let config = ExtractionConfig {
        use_cache: false,
        images: Some(ImageExtractionConfig {
            target_dpi: 150,
            auto_adjust_dpi: false,
            extract_images: false,
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let extract = |bytes: &[u8], config: &ExtractionConfig| {
        helpers::extract_bytes_document_blocking(bytes, "application/pdf", config).expect("extract PDF")
    };
    let native = extract(
        &mixed,
        &ExtractionConfig {
            disable_ocr: true,
            ..config.clone()
        },
    );
    let whole = extract(
        rotated,
        &ExtractionConfig {
            force_ocr: true,
            ..config.clone()
        },
    );
    let detached = extract(
        &mixed,
        &ExtractionConfig {
            force_ocr_pages: Some(vec![2]),
            ..config
        },
    );

    assert_detached_table(&native, &whole, &detached);
}

#[cfg(all(
    paddle_ocr,
    feature = "pdf",
    any(feature = "auto-rotate", feature = "auto-rotate-tract")
))]
fn assert_detached_table(
    native: &xberg::ExtractedDocument,
    whole: &xberg::ExtractedDocument,
    detached: &xberg::ExtractedDocument,
) {
    use xberg::types::ExtractionMethod;

    let [native_page, raster_page] = native.pages.as_deref().unwrap() else {
        panic!("two native views")
    };
    let [kept, recovered] = detached.pages.as_deref().unwrap() else {
        panic!("two mixed pages")
    };
    assert!(native_page.content.contains("The archive register"));
    assert!(raster_page.content.trim().is_empty());
    assert_eq!(kept.content, native_page.content);
    assert_eq!((kept.page_number, recovered.page_number), (1, 2));
    assert_eq!(detached.extraction_method, Some(ExtractionMethod::Mixed));
    let preprocessing = recovered.image_preprocessing.as_ref().expect("detached OCR dimensions");
    let raster = &preprocessing.original_dimensions;
    let processed = preprocessing
        .new_dimensions
        .as_ref()
        .expect("OCR must resize the raster");
    assert_eq!((raster.width, raster.height), (1650, 1276));
    assert_eq!((processed.width, processed.height), (3300, 2552));
    let [direct] = whole.tables.as_slice() else {
        panic!("one whole-page OCR table")
    };
    let [table] = detached.tables.as_slice() else {
        panic!("one detached OCR table")
    };
    assert_eq!(table.page_number, 2);
    assert_eq!(table.cells.len(), 48);
    assert!(table.cells.iter().all(|row| row.len() == 8));
    assert_eq!(table.cells, direct.cells);
    assert_eq!(table.markdown, direct.markdown);
    assert_eq!(recovered.content.trim(), table.markdown.trim());
    assert_eq!(
        table
            .cells
            .iter()
            .flatten()
            .map(|cell| cell.split_whitespace().count())
            .sum::<usize>(),
        FINANCIAL_TABLE_WORDS
    );
    assert_rotated_table_box(table.bounding_box.as_ref().expect("detached table geometry"));
}

#[cfg(all(
    paddle_ocr,
    feature = "pdf",
    any(feature = "auto-rotate", feature = "auto-rotate-tract")
))]
fn prepend_native_page(rotated: &[u8]) -> Vec<u8> {
    use lopdf::{Document, Object, Stream, dictionary};

    let mut document = Document::load_mem(rotated).unwrap();
    assert_eq!(document.get_pages().len(), 1);
    let pages = document
        .catalog()
        .unwrap()
        .get(b"Pages")
        .unwrap()
        .as_reference()
        .unwrap();
    let font = document.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica",
    });
    let mut text = String::from("BT /F1 12 Tf 50 720 Td 36 TL ");
    for _ in 0..8 {
        text.push_str("(The archive register contains original native material.) Tj T* ");
    }
    text.push_str("ET");
    let contents = document.add_object(Stream::new(dictionary! {}, text.into_bytes()));
    let page = document.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages, "Contents" => contents,
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        "Resources" => dictionary! { "Font" => dictionary! { "F1" => font } },
    });
    let page_tree = document.get_object_mut(pages).unwrap().as_dict_mut().unwrap();
    page_tree
        .get_mut(b"Kids")
        .unwrap()
        .as_array_mut()
        .unwrap()
        .insert(0, Object::Reference(page));
    page_tree.set("Count", 2);
    let mut bytes = Vec::new();
    document.save_to(&mut bytes).unwrap();
    bytes
}

#[cfg(all(feature = "pdf", any(feature = "auto-rotate", feature = "auto-rotate-tract")))]
const FINANCIAL_TABLE_WORDS: usize = 480;

#[cfg(all(feature = "pdf", any(feature = "auto-rotate", feature = "auto-rotate-tract")))]
fn assert_recovered_financial_table(recovered: &xberg::ExtractedDocument) {
    let [table] = recovered.tables.as_slice() else {
        panic!("expected one table after cached orientation recovery");
    };
    // The snapshot captures the known OCR reading of the synthetic table, including OCR substitutions. ~keep
    let expected: Vec<Vec<_>> = include_str!("fixtures/ocr/rotated_financial_table.tsv")
        .lines()
        .map(|row| row.split('\t').collect())
        .collect();
    assert_eq!(expected.len(), 48);
    assert!(expected.iter().all(|row| row.len() == 8));
    assert_eq!(table.cells, expected, "ordered table cells");
    assert_eq!(table.page_number, 1);
    let words: Vec<_> = expected
        .iter()
        .flatten()
        .flat_map(|cell| cell.split_whitespace())
        .collect();
    assert_eq!(words.len(), FINANCIAL_TABLE_WORDS);
    assert_eq!(recovered.content.split_whitespace().collect::<Vec<_>>(), words);
    assert!(recovered.processing_warnings.is_empty());
    assert!(
        recovered.ocr_elements.is_none(),
        "internal OCR evidence must stay private"
    );
    assert_rotated_table_box(table.bounding_box.as_ref().expect("table geometry"));
}

#[cfg(all(feature = "pdf", any(feature = "auto-rotate", feature = "auto-rotate-tract")))]
fn assert_rotated_table_box(bounds: &xberg::types::BoundingBox) {
    assert!(0.0 <= bounds.x0 && bounds.x0 < bounds.x1 && bounds.x1 <= 792.0);
    assert!(0.0 <= bounds.y0 && bounds.y0 < bounds.y1 && bounds.y1 <= 612.48);
    for (actual, ink) in [
        (bounds.x0, 31.20),
        (bounds.y0, 29.28),
        (bounds.x1, 737.76),
        (bounds.y1, 568.80),
    ] {
        assert!(
            (actual - ink).abs() <= 10.0,
            "{actual} must bound the drawn table near {ink}"
        );
    }
}
