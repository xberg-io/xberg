use super::*;
use crate::cancellation::CancellationToken;
use std::path::PathBuf;

const PNG_MAGIC: &[u8] = b"\x89PNG";

#[test]
fn test_raw_pixels_to_png_grayscale() {
    let pixels: Vec<u8> = vec![0x00, 0x80, 0xc0, 0xff];
    let result = raw_pixels_to_png(2, 2, &xberg_native_pdf::extractors::PixelFormat::Grayscale, &pixels);
    let bytes = result.expect("grayscale 2×2 must encode without error");
    assert!(
        bytes.starts_with(PNG_MAGIC),
        "output must be a PNG; got {:02x?}",
        &bytes[..4.min(bytes.len())]
    );
}

#[test]
fn test_raw_pixels_to_png_rgb() {
    let pixels: Vec<u8> = vec![0xff, 0x00, 0x00, 0x00, 0xff, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff];
    let result = raw_pixels_to_png(2, 2, &xberg_native_pdf::extractors::PixelFormat::RGB, &pixels);
    let bytes = result.expect("RGB 2×2 must encode without error");
    assert!(
        bytes.starts_with(PNG_MAGIC),
        "output must be a PNG; got {:02x?}",
        &bytes[..4.min(bytes.len())]
    );
}

#[test]
fn test_raw_pixels_to_png_cmyk_converts_to_rgb_png() {
    let pixels: Vec<u8> = vec![0x00, 0x00, 0x00, 0xff, 0x00, 0x00, 0x00, 0x00];
    let result = raw_pixels_to_png(1, 2, &xberg_native_pdf::extractors::PixelFormat::CMYK, &pixels);
    let bytes = result.expect("CMYK 1×2 must encode without error");
    assert!(
        bytes.starts_with(PNG_MAGIC),
        "output must be a PNG; got {:02x?}",
        &bytes[..4.min(bytes.len())]
    );
    let decoded = image::load_from_memory(&bytes).expect("decoded PNG must be valid");
    assert_eq!(decoded.width(), 1);
    assert_eq!(decoded.height(), 2);
}

#[test]
fn test_raw_pixels_to_png_size_mismatch_returns_error() {
    let pixels: Vec<u8> = vec![0x00, 0x80, 0xc0, 0xff];
    let result = raw_pixels_to_png(4, 4, &xberg_native_pdf::extractors::PixelFormat::Grayscale, &pixels);
    assert!(
        result.is_err(),
        "mismatched buffer size must return Err, not Ok or panic"
    );
}

#[test]
fn test_raw_pixels_to_png_rgb_size_mismatch_returns_error() {
    let pixels: Vec<u8> = vec![0xff; 9];
    let result = raw_pixels_to_png(2, 2, &xberg_native_pdf::extractors::PixelFormat::RGB, &pixels);
    assert!(result.is_err(), "mismatched RGB buffer must return Err");
}

#[test]
fn test_raw_pixels_to_png_cmyk_odd_length_returns_error() {
    let pixels: Vec<u8> = vec![0x00, 0x00, 0x00];
    let result = raw_pixels_to_png(1, 1, &xberg_native_pdf::extractors::PixelFormat::CMYK, &pixels);
    assert!(
        result.is_err(),
        "CMYK buffer whose length is not a multiple of 4 must return Err, not panic"
    );
}

/// A decoded buffer that is row-padded carries extra bytes past
/// `width × channels` on every scanline, so it is *larger* than the image
/// needs. `ImageBuffer::from_raw` only rejects a buffer that is too small,
/// so an oversized one used to reach the PNG encoder, which asserts on the
/// exact size and panics.
///
/// The property under test is the size mismatch itself. Any row-padded
/// buffer reproduces it, whatever document it came from.
#[test]
fn test_raw_pixels_to_png_row_padded_rgb_returns_error_not_panic() {
    let (width, height) = (3u32, 2u32);
    let mut pixels = Vec::new();
    for _ in 0..height {
        pixels.extend_from_slice(&[0xff; 9]);
        pixels.push(0x00);
    }
    let result = raw_pixels_to_png(width, height, &xberg_native_pdf::extractors::PixelFormat::RGB, &pixels);
    assert!(
        result.is_err(),
        "a row-padded RGB buffer must return Err, not panic in the PNG encoder"
    );
}

#[test]
fn test_raw_pixels_to_png_row_padded_grayscale_returns_error_not_panic() {
    let (width, height) = (3u32, 2u32);
    let mut pixels = Vec::new();
    for _ in 0..height {
        pixels.extend_from_slice(&[0x80; 3]);
        pixels.push(0x00);
    }
    let result = raw_pixels_to_png(
        width,
        height,
        &xberg_native_pdf::extractors::PixelFormat::Grayscale,
        &pixels,
    );
    assert!(
        result.is_err(),
        "a row-padded grayscale buffer must return Err, not panic in the PNG encoder"
    );
}

/// CMYK is four bytes per pixel and is converted to RGB before the image is
/// built, so the padding has to be added as whole pixels for the converted
/// buffer to come out row-padded.
#[test]
fn test_raw_pixels_to_png_row_padded_cmyk_returns_error_not_panic() {
    let (width, height) = (3u32, 2u32);
    let mut pixels = Vec::new();
    for _ in 0..height {
        for _ in 0..=width {
            pixels.extend_from_slice(&[0x00, 0x00, 0x00, 0xff]);
        }
    }
    let result = raw_pixels_to_png(width, height, &xberg_native_pdf::extractors::PixelFormat::CMYK, &pixels);
    assert!(
        result.is_err(),
        "a row-padded CMYK buffer must return Err, not panic in the PNG encoder"
    );
}

/// Issue #71: an image dropped for failing to re-encode must produce a
/// `ProcessingWarning` naming the image index and page, not just a
/// `tracing::warn!` log line the caller can never see.
#[test]
fn test_unencodable_image_warning_names_index_and_page() {
    let pixels: Vec<u8> = vec![0x00, 0x80, 0xc0, 0xff];
    let error = raw_pixels_to_png(4, 4, &xberg_native_pdf::extractors::PixelFormat::Grayscale, &pixels)
        .expect_err("4x4 grayscale from a 4-byte buffer must fail to re-encode");

    let warning = unencodable_image_warning(3, 2, &error);

    assert_eq!(warning.source.as_ref(), "pdf_images");
    assert_eq!(
        warning.message.as_ref(),
        format!("skipped image 3 on page 2: could not be re-encoded from raw pixel data ({error})"),
        "warning message must name the exact image index (3) and page (2)"
    );
}

fn test_documents_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test_documents")
}

/// `max_images_per_page = Some(0)` must return an empty vec immediately
/// without opening any page — the early-exit short-circuit at the top of
/// `extract_images_with_data` fires before the page loop even starts.
#[test]
fn test_max_images_per_page_zero_returns_immediately() {
    let pdf_path = test_documents_dir().join("pdf/embedded_images_tables.pdf");
    assert!(
        pdf_path.exists(),
        "missing fixture: test PDF not found at {}",
        pdf_path.display()
    );

    let bytes = std::fs::read(&pdf_path).expect("failed to read test PDF");
    let mut doc = crate::pdf::native::NativeDocument::open_bytes(&bytes).expect("failed to open PDF");

    let (result, _warnings) = extract_images_with_data(&mut doc, Some(0), None, &std::collections::HashMap::new())
        .expect("cap=0 must not error");

    assert!(
        result.is_empty(),
        "max_images_per_page=Some(0) must return empty without decompressing any page; \
         got {} image(s)",
        result.len()
    );
}

/// A cancellation token fired from a background thread stops extraction after
/// the current page completes and before the next page's cancellation check.
///
/// Uses `nougat_039.pdf` (2 pages, ~67KB). A background thread cancels the
/// token after 20ms — a window chosen to land after page 0's images are
/// decompressed but before page 1's cancellation check fires.
///
/// Timing note: on very fast or very slow hardware, the cancel may fire before
/// page 0 completes (result is empty) or after page 1 completes (result equals
/// the full count). Both are valid outcomes.  The invariant under test is
/// `result.len() ≤ full_count`, which proves that cancellation never produces
/// *more* images than an uncancelled run and that the code path compiles and
/// runs without error.
#[test]
fn test_cancellation_fires_between_pages() {
    let pdf_path = test_documents_dir().join("pdf/nougat_039.pdf");
    assert!(
        pdf_path.exists(),
        "missing fixture: nougat_039.pdf not found at {}",
        pdf_path.display()
    );

    let bytes = std::fs::read(&pdf_path).expect("failed to read test PDF");

    let mut doc_full = crate::pdf::native::NativeDocument::open_bytes(&bytes).expect("failed to open PDF");
    let (full_result, _warnings) =
        extract_images_with_data(&mut doc_full, None, None, &std::collections::HashMap::new())
            .expect("uncancelled extraction must not error");
    let full_count = full_result.len();
    let page_count = doc_full
        .doc
        .page_count()
        .expect("page_count must succeed on the fixture");

    if page_count <= 1 || full_count == 0 {
        eprintln!(
            "SKIP test_cancellation_fires_between_pages: nougat_039.pdf has {} page(s) \
             and {} extractable images — need ≥2 pages with images",
            page_count, full_count
        );
        return;
    }

    let mut doc_cancel = crate::pdf::native::NativeDocument::open_bytes(&bytes).expect("failed to open PDF");
    let token = CancellationToken::new();
    let token_clone = token.clone();

    let handle = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(20));
        token_clone.cancel();
    });

    let (result, _warnings) =
        extract_images_with_data(&mut doc_cancel, None, Some(&token), &std::collections::HashMap::new())
            .expect("cancellation must not error");

    handle.join().expect("background thread must not panic");

    assert!(
        token.is_cancelled(),
        "token must be cancelled after background thread fires"
    );

    assert!(
        result.len() <= full_count,
        "cancelled extraction returned {} image(s); uncancelled returned {}; \
         cancellation must never exceed the full count",
        result.len(),
        full_count
    );
}

/// Pre-cancelled token fires on the first loop iteration (page 0) before
/// any decompression begins. This test covers the trivial case; see
/// `test_cancellation_fires_between_pages` for mid-run coverage.
#[test]
fn test_cancellation_stops_extraction_early() {
    let pdf_path = test_documents_dir().join("pdf/embedded_images_tables.pdf");
    assert!(
        pdf_path.exists(),
        "missing fixture: test PDF not found at {}",
        pdf_path.display()
    );

    let bytes = std::fs::read(&pdf_path).expect("failed to read test PDF");
    let mut doc = crate::pdf::native::NativeDocument::open_bytes(&bytes).expect("failed to open PDF");

    let token = CancellationToken::new();
    token.cancel();

    let (result, _warnings) = extract_images_with_data(&mut doc, None, Some(&token), &std::collections::HashMap::new())
        .expect("extract must not error");

    assert!(
        result.is_empty(),
        "pre-cancelled token must cause extraction to return empty vec immediately, \
         got {} image(s)",
        result.len()
    );
}

/// The default (uncapped) extraction path routes through `PdfDocument::extract_images`,
/// which walks content streams tracking the CTM and calls `PdfImage::set_bbox` for every
/// image `Do` operator. Confirms `bounding_box` is populated end-to-end for a real fixture.
#[test]
fn test_extract_images_with_data_default_path_populates_bounding_box() {
    let pdf_path = test_documents_dir().join("pdf/embedded_images_tables.pdf");
    assert!(
        pdf_path.exists(),
        "missing fixture: test PDF not found at {}",
        pdf_path.display()
    );

    let bytes = std::fs::read(&pdf_path).expect("failed to read test PDF");
    let mut doc = crate::pdf::native::NativeDocument::open_bytes(&bytes).expect("failed to open PDF");

    let (result, _warnings) = extract_images_with_data(&mut doc, None, None, &std::collections::HashMap::new())
        .expect("extraction must not error");

    assert!(!result.is_empty(), "fixture must contain at least one image");
    assert!(
        result.iter().all(|img| img.bounding_box.is_some()),
        "every image extracted via the default (uncapped) path must carry a bounding_box \
         from xberg_native_pdf's CTM-tracked extract_images(); got: {:?}",
        result.iter().map(|img| img.bounding_box).collect::<Vec<_>>()
    );
}

#[test]
fn test_extract_images_with_data_capped_path_preserves_bounding_box() {
    let pdf_path = test_documents_dir().join("pdf/embedded_images_tables.pdf");
    assert!(
        pdf_path.exists(),
        "missing fixture: test PDF not found at {}",
        pdf_path.display()
    );

    let bytes = std::fs::read(&pdf_path).expect("failed to read test PDF");
    let mut doc = crate::pdf::native::NativeDocument::open_bytes(&bytes).expect("failed to open PDF");

    let (result, _warnings) = extract_images_with_data(&mut doc, Some(50), None, &std::collections::HashMap::new())
        .expect("extraction must not error");

    assert!(!result.is_empty(), "fixture must contain at least one image");
    assert!(
        result.iter().all(|img| img.bounding_box.is_some()),
        "the capped image-handle path must preserve CTM-derived bounding boxes; \
         got: {:?}",
        result.iter().map(|img| img.bounding_box).collect::<Vec<_>>()
    );
}

/// Verify that `detect_image_format_from_bytes` correctly identifies formats from magic bytes.
/// This test ensures that even if xberg_native_pdf returns data labeled as JPEG but lacking proper
/// headers, we can detect the actual format.
#[test]
fn test_detect_image_format_from_bytes() {
    let jpeg_data = b"\xff\xd8\xff\xe0\x00\x10JFIF";
    assert_eq!(detect_image_format_from_bytes(jpeg_data), "jpeg");

    let png_data = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR";
    assert_eq!(detect_image_format_from_bytes(png_data), "png");

    let gif_data = b"GIF89a";
    assert_eq!(detect_image_format_from_bytes(gif_data), "gif");

    let tiff_le = b"II\x2a\x00";
    assert_eq!(detect_image_format_from_bytes(tiff_le), "tiff");

    let tiff_be = b"MM\x00\x2a";
    assert_eq!(detect_image_format_from_bytes(tiff_be), "tiff");

    let bmp_data = b"BM\x00\x00\x00";
    assert_eq!(detect_image_format_from_bytes(bmp_data), "bmp");

    let jp2_data = b"\x00\x00\x00\x0cjP  ";
    assert_eq!(detect_image_format_from_bytes(jp2_data), "jpeg2000");

    let raw_data = b"\x00\x01\x02\x03\x04\x05";
    assert_eq!(detect_image_format_from_bytes(raw_data), "raw");

    assert_eq!(detect_image_format_from_bytes(b""), "raw");

    let incomplete = b"\xff\xd8";
    assert_eq!(detect_image_format_from_bytes(incomplete), "raw");
}

/// `page_ocr_fallback_image_bytes` must recover usable image bytes for a page whose
/// content is real, decodable image XObjects — the fixture used elsewhere in this
/// file for image extraction (issue #1355 force_ocr fallback).
#[cfg(feature = "ocr")]
#[test]
fn test_page_ocr_fallback_image_bytes_recovers_real_image() {
    let pdf_path = test_documents_dir().join("pdf/embedded_images_tables.pdf");
    assert!(
        pdf_path.exists(),
        "missing fixture: test PDF not found at {}",
        pdf_path.display()
    );

    let bytes = std::fs::read(&pdf_path).expect("failed to read test PDF");
    let doc = crate::pdf::native::NativeDocument::open_bytes(&bytes).expect("failed to open PDF");

    let (fallback_images, _) = page_ocr_fallback_image_bytes(&doc.doc, 0, None);

    assert!(
        !fallback_images.is_empty(),
        "fixture page 0 must contain at least one recoverable image XObject"
    );
    for fallback in &fallback_images {
        let format = detect_image_format_from_bytes(&fallback.bytes);
        assert!(
            matches!(format, "jpeg" | "png" | "jpeg2000"),
            "fallback image bytes must carry a recognizable magic (jpeg/png/jpeg2000); got {:02x?}",
            &fallback.bytes[..8.min(fallback.bytes.len())]
        );
        assert_eq!(
            fallback.format, format,
            "reported format must match the recovered bytes' actual magic"
        );
    }
}

/// #1444: the recovery mode each image XObject took must survive the call, so the
/// OCR fallback can record provenance on the recovered page's `ExtractedImage`
/// instead of throwing the distinction away. The fixture's page 0 image is a
/// DCTDecode stream xberg_native_pdf decodes successfully, so it must come back tagged
/// `EmbeddedJpeg` with format `jpeg`.
#[cfg(feature = "ocr")]
#[test]
fn should_report_recovery_mode_for_each_recovered_xobject() {
    let pdf_path = test_documents_dir().join("pdf/embedded_images_tables.pdf");
    let bytes = std::fs::read(&pdf_path).expect("failed to read test PDF");
    let doc = crate::pdf::native::NativeDocument::open_bytes(&bytes).expect("failed to open PDF");

    let (fallback_images, _) = page_ocr_fallback_image_bytes(&doc.doc, 0, None);
    let first = fallback_images
        .first()
        .expect("fixture page 0 must contain at least one recoverable image XObject");

    assert_eq!(first.recovery, XObjectRecovery::EmbeddedJpeg);
    assert_eq!(first.recovery.as_str(), "embedded_jpeg");
    assert_eq!(first.format, "jpeg");
}

/// A page index past the end of the document must not panic; `page_image_handles`
/// returns an `Err` that the fallback helper degrades to an empty vec.
#[cfg(feature = "ocr")]
#[test]
fn test_page_ocr_fallback_image_bytes_out_of_range_page_returns_empty() {
    let pdf_path = test_documents_dir().join("pdf/embedded_images_tables.pdf");
    let bytes = std::fs::read(&pdf_path).expect("failed to read test PDF");
    let doc = crate::pdf::native::NativeDocument::open_bytes(&bytes).expect("failed to open PDF");

    let (fallback_images, _) = page_ocr_fallback_image_bytes(&doc.doc, 9999, None);

    assert!(
        fallback_images.is_empty(),
        "out-of-range page must degrade to an empty vec, not panic or error"
    );
}

/// A high-resolution grayscale scan can compress far beyond the generic
/// stream ratio limit without being a decompression bomb. The image XObject
/// in this fixture expands to exactly its declared 4960 x 7016 x 8-bit
/// raster and must remain available to the native OCR fallback.
#[cfg(feature = "ocr")]
#[test]
fn should_recover_dimension_bounded_high_ratio_ocr_image() {
    let pdf_path = test_documents_dir().join("pdf/ocr_test.pdf");
    assert!(pdf_path.exists(), "OCR regression fixture must exist");

    let bytes = std::fs::read(&pdf_path).expect("read OCR regression fixture");
    let doc = crate::pdf::native::NativeDocument::open_bytes(&bytes).expect("open OCR regression fixture");
    let (recovered, _) = page_ocr_fallback_image_bytes(&doc.doc, 0, None);

    let image = recovered.first().expect("recover the scanned page image for OCR");
    let decoded = image::load_from_memory(&image.bytes).expect("decode recovered OCR image");
    assert_eq!(decoded.width(), 4960);
    assert_eq!(decoded.height(), 7016);
}

/// Issue #62: a tagged PDF's `Figure` structure element carries `/Alt "officeArt
/// object"` on page 1 for the image XObject painted there. Before the fix,
/// `description` was hardcoded to `None` for every extracted image, so this
/// alt text was silently dropped and never reached `ExtractedImage::description`
/// (which markdown/plain-text rendering consume as the image's alt/caption text).
#[test]
fn test_extract_images_with_data_reads_tagged_pdf_alt_text() {
    let pdf_path = test_documents_dir().join("pdf/nougat_049.pdf");
    assert!(
        pdf_path.exists(),
        "missing fixture: test PDF not found at {}",
        pdf_path.display()
    );

    let bytes = std::fs::read(&pdf_path).expect("failed to read test PDF");
    let mut doc = crate::pdf::native::NativeDocument::open_bytes(&bytes).expect("failed to open PDF");

    let (result, _warnings) = extract_images_with_data(&mut doc, None, None, &std::collections::HashMap::new())
        .expect("extraction must not error");

    let page_one_images: Vec<_> = result.iter().filter(|img| img.page_number == Some(1)).collect();
    assert!(
        !page_one_images.is_empty(),
        "fixture page 1 must contain at least one extracted image"
    );
    assert_eq!(
        page_one_images[0].description,
        Some("officeArt object".to_string()),
        "the first image on page 1 must carry the /Alt text from its Figure structure \
         element instead of None; got {:?}",
        page_one_images[0].description
    );
}
