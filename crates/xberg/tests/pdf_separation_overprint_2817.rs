#![cfg(all(feature = "pdf", feature = "ocr", feature = "bundle-tessdata-eng"))]

mod helpers;

use xberg::core::config::PageConfig;
use xberg::types::ExtractionMethod;
use xberg::{ExtractionConfig, OcrConfig, OutputFormat};

const EXPECTED: &str = include_str!("fixtures/pdf/separation-overprint-2817/sepop.ocr.txt");

#[test]
fn automatic_ocr_keeps_overprinted_separation_text_and_native_sibling() {
    let tessdata = tempfile::tempdir().expect("create an isolated tessdata directory");
    let english = xberg_tesseract::bundled_eng_traineddata().expect("bundled English data");
    std::fs::write(tessdata.path().join("eng.traineddata"), english).expect("seed bundled English data");
    let config = ExtractionConfig {
        use_cache: false,
        output_format: OutputFormat::Plain,
        ocr: Some(OcrConfig {
            tessdata_path: Some(tessdata.path().to_owned()),
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let expected: Vec<_> = EXPECTED.split_whitespace().collect();
    assert_eq!(expected.len(), 219);
    let cases = [
        (
            "no overprint",
            include_bytes!("fixtures/pdf/separation-overprint-2817/separation-no-overprint-mixed.pdf").as_slice(),
        ),
        (
            "overprint",
            include_bytes!("fixtures/pdf/separation-overprint-2817/sepop-mixed.pdf").as_slice(),
        ),
    ];
    for (label, bytes) in cases {
        let result = helpers::extract_bytes_document_blocking(bytes, "application/pdf", &config)
            .expect("extract the public mixed PDF");
        assert_eq!(result.extraction_method, Some(ExtractionMethod::Mixed), "{label}");
        let pages = result.pages.as_deref().expect("requested page output");
        let [ocr, native] = pages else {
            panic!("{label}: expected two pages, got {}", pages.len());
        };
        assert_eq!(ocr.content.split_whitespace().collect::<Vec<_>>(), expected, "{label}");
        let raster = ocr.image_preprocessing.as_ref().expect("OCR raster provenance");
        assert_eq!(raster.original_dimensions.width, 1020, "{label}");
        assert_eq!(raster.target_dpi, 120, "{label}");
        assert!(raster.new_dimensions.is_none(), "{label}: raster was resized");
        assert_eq!(
            native.content.trim(),
            "Native sibling control remains unchanged.",
            "{label}"
        );
        assert!(
            result
                .content
                .split_whitespace()
                .eq(expected.iter().copied().chain(native.content.split_whitespace())),
            "{label}: document transcript differs from its pages"
        );
    }
}

#[test]
fn reporter_folio_keeps_the_exact_transcript_with_automatic_ocr() {
    let tessdata = tempfile::tempdir().expect("create an isolated tessdata directory");
    let english = xberg_tesseract::bundled_eng_traineddata().expect("bundled English data");
    std::fs::write(tessdata.path().join("eng.traineddata"), english).expect("seed bundled English data");
    let mut config = ExtractionConfig {
        use_cache: false,
        output_format: OutputFormat::Plain,
        ocr: Some(OcrConfig {
            backend: "tesseract".into(),
            language: vec!["eng".into()],
            tessdata_path: Some(tessdata.path().to_owned()),
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let body = EXPECTED
        .strip_prefix("Library Planning Notes")
        .expect("mixed fixture title");
    let expected = format!("{body}\nPage 12");
    let native_incomplete = expected.replace("quiet queue", "uiet ueue");
    assert_eq!(expected.split_whitespace().count(), 218);
    assert_eq!(expected.matches("quiet queue").count(), 4);

    for (label, bytes, incomplete) in [
        (
            "Separation Black overprint",
            include_bytes!("fixtures/pdf/separation-overprint-2817/folio-sepop.pdf").as_slice(),
            true,
        ),
        (
            "plain black",
            include_bytes!("fixtures/pdf/separation-overprint-2817/folio-black.pdf").as_slice(),
            true,
        ),
        (
            "complete ToUnicode",
            include_bytes!("fixtures/pdf/separation-overprint-2817/folio-mapped.pdf").as_slice(),
            false,
        ),
    ] {
        config.disable_ocr = true;
        let native = if incomplete { &native_incomplete } else { &expected };
        assert_folio(bytes, &config, native, ExtractionMethod::Native, label);

        config.disable_ocr = false;
        let method = if incomplete {
            ExtractionMethod::Mixed
        } else {
            ExtractionMethod::Native
        };
        assert_folio(bytes, &config, &expected, method, label);
    }
}

#[test]
fn mixed_size_type0_text_keeps_the_exact_transcript_at_normal_density() {
    let tessdata = tempfile::tempdir().unwrap();
    std::fs::write(
        tessdata.path().join("eng.traineddata"),
        xberg_tesseract::bundled_eng_traineddata().unwrap(),
    )
    .unwrap();
    let config = ExtractionConfig {
        use_cache: false,
        output_format: OutputFormat::Plain,
        ocr: Some(OcrConfig {
            tessdata_path: Some(tessdata.path().to_owned()),
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let expected = format!("{}\nPage 12", EXPECTED.strip_prefix("Library Planning Notes").unwrap());
    let result = helpers::extract_bytes_document_blocking(
        include_bytes!("fixtures/pdf/separation-overprint-2817/folio-mixed-size.pdf"),
        "application/pdf",
        &config,
    )
    .unwrap();
    assert_eq!(result.extraction_method, Some(ExtractionMethod::Mixed));
    let [page] = result.pages.as_deref().expect("requested page output") else {
        panic!("expected one page");
    };
    for text in [&result.content, &page.content] {
        assert_eq!(
            text.split_whitespace().collect::<Vec<_>>(),
            expected.split_whitespace().collect::<Vec<_>>()
        );
    }
    let raster = page.image_preprocessing.as_ref();
    assert!(raster.is_none_or(|raster| raster.original_dimensions.width == 1275));
}

fn assert_folio(bytes: &[u8], config: &ExtractionConfig, expected: &str, method: ExtractionMethod, label: &str) {
    let mode = if config.disable_ocr { "native-only" } else { "Auto" };
    let result =
        helpers::extract_bytes_document_blocking(bytes, "application/pdf", config).expect("extract the public folio");
    assert_eq!(result.extraction_method, Some(method), "{label} {mode}");
    let pages = result.pages.as_deref().expect("requested page output");
    let [page] = pages else {
        panic!("{label} {mode}: expected one page, got {}", pages.len());
    };
    assert_eq!(page.page_number, 1, "{label} {mode}");
    if method == ExtractionMethod::Mixed {
        let raster = page.image_preprocessing.as_ref().expect("OCR raster provenance");
        assert_eq!(raster.original_dimensions.width, 1020, "{label}");
        assert_eq!(raster.target_dpi, 120, "{label}");
        assert!(raster.new_dimensions.is_none(), "{label}: raster was resized");
    }
    let tokens = expected.split_whitespace().collect::<Vec<_>>();
    for (surface, content) in [("document", &result.content), ("page", &page.content)] {
        assert_eq!(
            content.split_whitespace().collect::<Vec<_>>(),
            tokens,
            "{label} {mode} {surface}"
        );
        assert_eq!(content.matches("Page 12").count(), 1, "{label} {mode} {surface}");
        for phrase in [
            "committee met on Tuesday",
            "parking area",
            "architect said that building work",
        ] {
            assert!(
                content.contains(phrase),
                "{label} {mode} {surface} is missing {phrase:?}"
            );
        }
    }
}
