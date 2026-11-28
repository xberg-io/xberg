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

/// Directory offsets of a classic little-endian TIFF written by the `tiff` encoder.
fn ifd_offsets(bytes: &[u8]) -> Vec<usize> {
    let read = |at: usize, len: usize| {
        (0..len).fold(0usize, |value, index| {
            value | (usize::from(bytes[at + index]) << (8 * index))
        })
    };
    let mut offsets = Vec::new();
    let mut next = read(4, 4);
    while next != 0 {
        offsets.push(next);
        let entries = read(next, 2);
        next = read(next + 2 + entries * 12, 4);
    }
    offsets
}

#[test]
fn tiff_beyond_the_default_frame_ceiling_is_rejected_without_a_max_pages_setting() {
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut encoder = tiff::encoder::TiffEncoder::new(&mut cursor).expect("tiff encoder");
        for _ in 0..1001 {
            encoder
                .write_image::<tiff::encoder::colortype::Gray8>(1, 1, &[255])
                .expect("write frame");
        }
    }
    let error = extract_bytes_document_blocking(&cursor.into_inner(), "image/tiff", &ocr_config(None))
        .expect_err("1001 frames exceed the default ceiling of 1000");
    assert!(
        error.to_string().contains("too many pages"),
        "unexpected error: {error}"
    );
}

#[test]
fn a_directory_link_past_the_end_of_the_file_keeps_the_frames_read_and_warns() {
    let mut tiff = multi_page_tiff(3);
    let offsets = ifd_offsets(&tiff);
    let entries = usize::from(tiff[offsets[1]]) | (usize::from(tiff[offsets[1] + 1]) << 8);
    let link = offsets[1] + 2 + entries * 12;
    let past_end = (tiff.len() as u32 + 4096).to_le_bytes();
    tiff[link..link + 4].copy_from_slice(&past_end);

    let result = extract_bytes_document_blocking(&tiff, "image/tiff", &ocr_config(None)).expect("OCR must not error");
    let pages = result.pages.as_ref().expect("pages");
    assert_eq!(pages.len(), 2, "the two readable frames are kept");
    assert!(pages[0].content.to_ascii_uppercase().contains("HELLO"));
    assert!(pages[1].content.contains("1172"));
    assert!(
        result
            .processing_warnings
            .iter()
            .any(|warning| warning.source == "tiff_frames" && warning.message.contains("frame 3")),
        "the lost frame is reported: {:?}",
        result.processing_warnings
    );
}

#[test]
fn a_frame_that_cannot_be_decoded_is_reported_and_the_other_frames_are_still_read() {
    let mut tiff = multi_page_tiff(3);
    let second = ifd_offsets(&tiff)[1];
    let entries = usize::from(tiff[second]) | (usize::from(tiff[second + 1]) << 8);
    let compression = (0..entries)
        .map(|entry| second + 2 + entry * 12)
        .find(|at| tiff[*at] == 0x03 && tiff[*at + 1] == 0x01)
        .expect("compression tag");
    tiff[compression + 8..compression + 10].copy_from_slice(&9999u16.to_le_bytes());

    let result = extract_bytes_document_blocking(&tiff, "image/tiff", &ocr_config(None)).expect("OCR must not error");
    let pages = result.pages.as_ref().expect("pages");
    assert_eq!(pages.len(), 3);
    assert!(pages[0].content.to_ascii_uppercase().contains("HELLO"));
    assert!(pages[1].content.is_empty(), "the undecodable frame is a blank page");
    assert!(pages[2].content.to_ascii_uppercase().contains("ESPRESSO"));
    let failures = result.ocr_page_failures.as_ref().expect("failure recorded");
    assert_eq!(failures.iter().map(|failure| failure.page).collect::<Vec<_>>(), vec![2]);
}

#[test]
fn markdown_output_keeps_each_frames_text_on_its_own_page() {
    let tiff = multi_page_tiff(3);
    let mut config = ocr_config(None);
    config.output_format = xberg::core::config::OutputFormat::Markdown;
    let result = extract_bytes_document_blocking(&tiff, "image/tiff", &config).expect("OCR must not error");
    let pages = result.pages.as_ref().expect("pages");
    assert_eq!(pages.len(), 3);
    for (page, word) in pages.iter().zip(FRAMES.map(|(_, word)| word)) {
        assert!(page.content.to_ascii_uppercase().contains(word), "{:?}", page.content);
    }
    let upper = result.content.to_ascii_uppercase();
    assert!(upper.find("HELLO") < upper.find("1172") && upper.find("1172") < upper.find("ESPRESSO"));
}

#[cfg(feature = "chunking")]
#[test]
fn chunks_report_the_frame_pages_they_span() {
    let tiff = multi_page_tiff(3);
    let mut config = ocr_config(None);
    config.chunking = Some(xberg::core::config::ChunkingConfig {
        max_characters: 40,
        overlap: 0,
        ..Default::default()
    });
    let result = extract_bytes_document_blocking(&tiff, "image/tiff", &config).expect("OCR must not error");
    let chunks = result.chunks.as_ref().expect("chunks");
    let hello = chunks
        .iter()
        .find(|chunk| chunk.content.to_ascii_uppercase().contains("HELLO"))
        .expect("hello chunk");
    let espresso = chunks
        .iter()
        .find(|chunk| chunk.content.to_ascii_uppercase().contains("ESPRESSO"))
        .expect("espresso chunk");
    assert_eq!(hello.metadata.first_page, Some(1));
    assert_eq!(espresso.metadata.last_page, Some(3));
}

#[test]
fn an_undecodable_first_frame_does_not_fail_the_frames_after_it() {
    let mut tiff = multi_page_tiff(3);
    let first = ifd_offsets(&tiff)[0];
    let entries = usize::from(tiff[first]) | (usize::from(tiff[first + 1]) << 8);
    let compression = (0..entries)
        .map(|entry| first + 2 + entry * 12)
        .find(|at| tiff[*at] == 0x03 && tiff[*at + 1] == 0x01)
        .expect("compression tag");
    tiff[compression + 8..compression + 10].copy_from_slice(&9999u16.to_le_bytes());

    let result = extract_bytes_document_blocking(&tiff, "image/tiff", &ocr_config(None)).expect("OCR must not error");
    let pages = result.pages.as_ref().expect("pages");
    assert_eq!(pages.len(), 3);
    assert!(pages[0].content.is_empty());
    assert!(pages[1].content.contains("1172"));
    assert!(pages[2].content.to_ascii_uppercase().contains("ESPRESSO"));
    let failures = result.ocr_page_failures.as_ref().expect("failure recorded");
    assert_eq!(failures.iter().map(|failure| failure.page).collect::<Vec<_>>(), vec![1]);
}
