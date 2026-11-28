//! GH#3447: a multi-page TIFF is OCR'd frame by frame, one page per frame.
//!
//! The fixture is built in memory from three rendered PNGs, each holding a different word.

#![allow(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)] // ~keep: org logging policy exempts tests
#![cfg(feature = "ocr")]

mod helpers;
use helpers::extract_bytes_document_blocking;
use xberg::core::config::{ExtractionConfig, OcrConfig, PageConfig};
use xberg::extractors::security::SecurityLimits;
use xberg::types::TesseractConfig;

/// Rendered fixtures with one distinct, unambiguous word each, in page order.
const FRAMES: [(&[u8], &str); 3] = [
    (include_bytes!("../../../fixtures/images/test_hello_world.png"), "HELLO"),
    (include_bytes!("fixtures/ocr/gh1789_number_test.png"), "1172"),
    (include_bytes!("fixtures/ocr/lone_quantity_invoice.png"), "ESPRESSO"),
];

/// A TIFF whose frames are the first `count` fixtures, each as an 8-bit grayscale page.
fn multi_page_tiff(count: usize) -> Vec<u8> {
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut encoder = tiff::encoder::TiffEncoder::new(&mut cursor).expect("tiff encoder");
        for (png, _) in FRAMES.iter().take(count) {
            let gray = image::load_from_memory(png).expect("decode fixture").into_luma8();
            encoder
                .write_image::<tiff::encoder::colortype::Gray8>(gray.width(), gray.height(), gray.as_raw())
                .expect("write frame");
        }
    }
    cursor.into_inner()
}

fn ocr_config(max_pages: Option<usize>) -> ExtractionConfig {
    ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: vec!["eng".to_string()],
            tesseract_config: Some(TesseractConfig {
                use_cache: false,
                ..Default::default()
            }),
            ..Default::default()
        }),
        force_ocr: true,
        use_cache: false,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        security_limits: max_pages.map(|max_pages| SecurityLimits {
            max_pages: Some(max_pages),
            ..Default::default()
        }),
        ..Default::default()
    }
}

#[test]
fn three_frame_tiff_yields_one_page_per_frame_with_that_frames_text() {
    let tiff = multi_page_tiff(3);
    let result = extract_bytes_document_blocking(&tiff, "image/tiff", &ocr_config(None)).expect("OCR must not error");

    let pages = result.pages.as_ref().expect("pages must be returned");
    assert_eq!(pages.len(), 3, "one page per frame, got {pages:#?}");
    for (index, (page, expected)) in pages.iter().zip(FRAMES.map(|(_, word)| word)).enumerate() {
        assert_eq!(page.page_number, index as u32 + 1);
        assert!(
            page.content.to_ascii_uppercase().contains(expected),
            "page {} must hold {expected}, got {:?}",
            page.page_number,
            page.content
        );
        for other in FRAMES
            .map(|(_, word)| word)
            .into_iter()
            .filter(|word| *word != expected)
        {
            assert!(
                !page.content.to_ascii_uppercase().contains(other),
                "page {} must not hold {other}, got {:?}",
                page.page_number,
                page.content
            );
        }
    }
    let upper = result.content.to_ascii_uppercase();
    let positions: Vec<usize> = FRAMES
        .map(|(_, word)| word)
        .iter()
        .map(|word| {
            upper
                .find(word)
                .unwrap_or_else(|| panic!("{word} missing from {upper:?}"))
        })
        .collect();
    assert!(
        positions.windows(2).all(|pair| pair[0] < pair[1]),
        "frames out of order"
    );
    assert_eq!(result.metadata.pages.as_ref().map(|pages| pages.total_count), Some(3));
}

#[test]
fn tiff_with_more_frames_than_max_pages_is_rejected_not_truncated() {
    let tiff = multi_page_tiff(3);
    let error = extract_bytes_document_blocking(&tiff, "image/tiff", &ocr_config(Some(2)))
        .expect_err("3 frames must not be silently cut to the 2-page cap");
    assert!(error.to_string().contains("max_pages"), "unexpected error: {error}");
}

#[test]
fn tiff_at_the_max_pages_cap_keeps_every_frame() {
    let tiff = multi_page_tiff(3);
    let result =
        extract_bytes_document_blocking(&tiff, "image/tiff", &ocr_config(Some(3))).expect("OCR must not error");
    assert_eq!(result.pages.as_ref().map(Vec::len), Some(3));
}

#[test]
fn multi_frame_tiff_without_ocr_reports_the_real_frame_count() {
    let tiff = multi_page_tiff(3);
    let config = ExtractionConfig {
        disable_ocr: true,
        use_cache: false,
        ..Default::default()
    };
    let result = extract_bytes_document_blocking(&tiff, "image/tiff", &config).expect("metadata extraction");
    assert_eq!(result.metadata.pages.as_ref().map(|pages| pages.total_count), Some(3));
}
