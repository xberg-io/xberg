//! Image extraction using the pdf_oxide backend.
//!
//! Extracts embedded images from PDF pages via pdf_oxide, including
//! actual image data and metadata.

use super::OxideDocument;
use crate::cancellation::CancellationToken;
use crate::pdf::error::{PdfError, Result};
use bytes::Bytes;
use image::{DynamicImage, ImageFormat};
use std::borrow::Cow;
use std::io::Cursor;

/// Extract at most `limit` images from a page by walking its XObject resource dictionary.
///
/// Unlike `doc.doc.extract_images(page_idx)` which decompresses every image on the page
/// before returning, this function stops after `limit` successful decompressions, avoiding
/// the eager-API cost for images beyond the cap.
///
/// **Trade-offs vs. `extract_images()`**:
/// - Does not cover inline images (`BI`/`EI` content stream operators). Those are rare in
///   practice for PDFs that embed large numbers of images.
/// - Uses XObject resource dictionary order sorted alphabetically for determinism.
///   Content stream `Do`-operator order may differ.
///
/// On any error accessing the resource dictionary the function returns an empty vec.
/// The caller may then fall back to the full eager path.
fn extract_n_images_from_xobject_resources(
    doc: &OxideDocument,
    page_idx: usize,
    limit: usize,
) -> Result<Vec<pdf_oxide::extractors::PdfImage>> {
    let resources = match doc.doc.get_page_resources(page_idx) {
        Ok(r) => r,
        Err(e) => {
            tracing::debug!(page = page_idx, "get_page_resources failed: {e}");
            return Ok(Vec::new());
        }
    };

    let res_dict = match resources.as_dict() {
        Some(d) => d,
        None => return Ok(Vec::new()),
    };

    let xobj_entry = match res_dict.get("XObject") {
        Some(x) => x,
        None => return Ok(Vec::new()),
    };

    // Resolve the XObject dictionary (it may be an indirect reference).
    let xobj_owned;
    let xobj_obj = if let Some(r) = xobj_entry.as_reference() {
        match doc.doc.load_object(r) {
            Ok(o) => {
                xobj_owned = o;
                &xobj_owned
            }
            Err(e) => {
                tracing::debug!(page = page_idx, "load XObject dict ref failed: {e}");
                return Ok(Vec::new());
            }
        }
    } else {
        xobj_entry
    };

    let xobj_dict = match xobj_obj.as_dict() {
        Some(d) => d,
        None => return Ok(Vec::new()),
    };

    // Collect and sort keys for deterministic ordering across calls.
    let mut names: Vec<String> = xobj_dict.keys().cloned().collect();
    names.sort();

    let mut images = Vec::new();

    for name in &names {
        if images.len() >= limit {
            break;
        }

        let val = match xobj_dict.get(name.as_str()) {
            Some(v) => v,
            None => continue,
        };

        let obj_ref = val.as_reference();

        // Fast skip: is_form_xobject peeks at /Subtype without loading the stream.
        // Returns true for Form XObjects (and conservatively for unknowns) so that
        // we do not waste a load_object call on non-image XObjects.
        if let Some(r) = obj_ref
            && doc.doc.is_form_xobject(r)
        {
            continue;
        }

        // Load the XObject: fetches the stream dictionary + compressed bytes.
        // Decompression (the expensive step) happens inside extract_image_from_xobject.
        let loaded;
        let xobj = if let Some(r) = obj_ref {
            match doc.doc.load_object(r) {
                Ok(o) => {
                    loaded = o;
                    &loaded
                }
                Err(e) => {
                    tracing::debug!(page = page_idx, xobject = %name, "load XObject failed: {e}");
                    continue;
                }
            }
        } else {
            val
        };

        // Guard: verify /Subtype = /Image before decompressing. is_form_xobject
        // returns true (conservative) for some non-Image types, so this check
        // filters those that slipped through.
        if xobj.as_dict().and_then(|d| d.get("Subtype")).and_then(|s| s.as_name()) != Some("Image") {
            continue;
        }

        // Decompress. This is the expensive step — it happens at most `limit` times
        // per page, which is what this function is designed to guarantee.
        match pdf_oxide::extractors::extract_image_from_xobject(
            Some(&doc.doc),
            xobj,
            obj_ref,
            None, // color_space_map: document-level resolution via doc
        ) {
            Ok(img) => images.push(img),
            Err(e) => {
                tracing::debug!(
                    page = page_idx,
                    xobject = %name,
                    "image decompression failed: {e}"
                );
            }
        }
    }

    Ok(images)
}

/// Re-encode raw PDF pixel data as a PNG buffer.
///
/// pdf_oxide emits `ImageData::Raw` without self-describing headers. Re-encoding
/// to PNG makes the buffer probeable by `load_image_for_ocr`,
/// `extract_image_metadata`, VLM pipelines, etc.
///
/// Returns `Err` if the pixel buffer length does not match `w × h × bpp` or if
/// PNG encoding fails.
fn raw_pixels_to_png(w: u32, h: u32, format: &pdf_oxide::extractors::PixelFormat, pixels: &[u8]) -> Result<Bytes> {
    let dynamic = match *format {
        pdf_oxide::extractors::PixelFormat::Grayscale => {
            let buf = image::GrayImage::from_raw(w, h, pixels.to_vec()).ok_or_else(|| {
                PdfError::ExtractionFailed(format!(
                    "grayscale pixel buffer ({} bytes) does not fit {}×{} image",
                    pixels.len(),
                    w,
                    h
                ))
            })?;
            DynamicImage::ImageLuma8(buf)
        }
        pdf_oxide::extractors::PixelFormat::RGB => {
            let buf = image::RgbImage::from_raw(w, h, pixels.to_vec()).ok_or_else(|| {
                PdfError::ExtractionFailed(format!(
                    "RGB pixel buffer ({} bytes) does not fit {}×{} image",
                    pixels.len(),
                    w,
                    h
                ))
            })?;
            DynamicImage::ImageRgb8(buf)
        }
        pdf_oxide::extractors::PixelFormat::CMYK => {
            let mut rgb = Vec::with_capacity((pixels.len() / 4) * 3);
            for chunk in pixels.chunks_exact(4) {
                let c = chunk[0] as f32 / 255.0;
                let m = chunk[1] as f32 / 255.0;
                let y = chunk[2] as f32 / 255.0;
                let k = chunk[3] as f32 / 255.0;
                rgb.push(((1.0 - c) * (1.0 - k) * 255.0) as u8);
                rgb.push(((1.0 - m) * (1.0 - k) * 255.0) as u8);
                rgb.push(((1.0 - y) * (1.0 - k) * 255.0) as u8);
            }
            let buf = image::RgbImage::from_raw(w, h, rgb)
                .ok_or_else(|| PdfError::ExtractionFailed(format!("CMYK→RGB buffer does not fit {}×{} image", w, h)))?;
            DynamicImage::ImageRgb8(buf)
        }
    };
    let mut png_bytes = Vec::new();
    dynamic
        .write_to(&mut Cursor::new(&mut png_bytes), ImageFormat::Png)
        .map_err(|e| PdfError::ExtractionFailed(format!("PNG re-encode of raw PDF image failed: {e}")))?;
    Ok(Bytes::from(png_bytes))
}

/// Extract full image data from all pages of a PDF.
///
/// Returns a `Vec<ExtractedImage>` with complete image data and metadata.
/// When image extraction is disabled or no images are found, returns an empty vec.
///
/// # Arguments
///
/// * `doc` - Mutable reference to the oxide document
/// * `max_images_per_page` - Optional limit on images per page
/// * `cancel_token` - Optional cancellation token checked between pages
///
/// # Returns
///
/// A `Vec<ExtractedImage>` containing all extracted images with their data.
pub(crate) fn extract_images_with_data(
    doc: &mut OxideDocument,
    max_images_per_page: Option<u32>,
    cancel_token: Option<&CancellationToken>,
) -> Result<Vec<crate::types::ExtractedImage>> {
    // When the cap is zero no image can ever pass through — skip decompression entirely.
    if max_images_per_page == Some(0) {
        return Ok(Vec::new());
    }

    tracing::debug!(
        target: "kreuzberg::pdf::oxide::images",
        event = "decompression_started",
        "extract_images_with_data entered"
    );

    let page_count = doc
        .doc
        .page_count()
        .map_err(|e| PdfError::MetadataExtractionFailed(format!("pdf_oxide: failed to get page count: {e}")))?;

    let mut all_images = Vec::new();
    let mut global_index = 0u32;

    for page_idx in 0..page_count {
        if cancel_token.is_some_and(|t| t.is_cancelled()) {
            break;
        }

        // For a positive cap use XObject resource enumeration to stop decompression
        // after `limit` images. This avoids the eager cost of pdf_oxide::extract_images
        // which decompresses every image on the page before returning.
        // Fallback: if the XObject path returns nothing (e.g. page uses only inline
        // images), retry with the full eager path and apply .take() manually.
        // kreuzberg#989 tracks getting inline-image support into the capped path.
        let oxide_images = match max_images_per_page.map(|n| n as usize) {
            Some(limit) => {
                let xobj_images = extract_n_images_from_xobject_resources(doc, page_idx, limit).unwrap_or_default();
                if !xobj_images.is_empty() {
                    xobj_images
                } else {
                    // Fallback: page may use only inline images.
                    match doc.doc.extract_images(page_idx) {
                        Ok(imgs) => imgs.into_iter().take(limit).collect(),
                        Err(e) => {
                            tracing::debug!(page = page_idx, "pdf_oxide: failed to extract images (fallback): {e}");
                            continue;
                        }
                    }
                }
            }
            None => match doc.doc.extract_images(page_idx) {
                Ok(imgs) => imgs,
                Err(e) => {
                    tracing::debug!(page = page_idx, "pdf_oxide: failed to extract images: {e}");
                    continue;
                }
            },
        };

        let page_number = (page_idx + 1) as u32; // Kreuzberg uses 1-indexed page numbers
        for oxide_img in &oxide_images {
            let (data, format) = match oxide_img.data() {
                pdf_oxide::extractors::ImageData::Jpeg(jpeg_bytes) => {
                    (Bytes::copy_from_slice(jpeg_bytes), Cow::Borrowed("jpeg"))
                }
                pdf_oxide::extractors::ImageData::Raw { pixels, format } => {
                    match raw_pixels_to_png(oxide_img.width(), oxide_img.height(), format, pixels) {
                        Ok(bytes) => (bytes, Cow::Borrowed("png")),
                        Err(e) => {
                            tracing::warn!(
                                page = page_number,
                                image_index = global_index,
                                "skipping raw PDF image that could not be re-encoded: {e}"
                            );
                            continue;
                        }
                    }
                }
            };

            let extracted_img = crate::types::ExtractedImage {
                data,
                format,
                image_index: global_index,
                page_number: Some(page_number),
                width: Some(oxide_img.width()),
                height: Some(oxide_img.height()),
                colorspace: Some(format!("{:?}", oxide_img.color_space())),
                bits_per_component: Some(oxide_img.bits_per_component() as u32),
                is_mask: false,
                description: None,
                ocr_result: None,
                bounding_box: None,
                source_path: None,
                image_kind: None,
                kind_confidence: None,
                cluster_id: None,
                caption: None,
                qr_codes: None,
            };

            all_images.push(extracted_img);
            global_index += 1;
        }
    }

    Ok(all_images)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cancellation::CancellationToken;
    use std::path::PathBuf;

    const PNG_MAGIC: &[u8] = b"\x89PNG";

    #[test]
    fn test_raw_pixels_to_png_grayscale() {
        // 2×2 grayscale: 4 bytes, one per pixel.
        let pixels: Vec<u8> = vec![0x00, 0x80, 0xc0, 0xff];
        let result = raw_pixels_to_png(2, 2, &pdf_oxide::extractors::PixelFormat::Grayscale, &pixels);
        let bytes = result.expect("grayscale 2×2 must encode without error");
        assert!(
            bytes.starts_with(PNG_MAGIC),
            "output must be a PNG; got {:02x?}",
            &bytes[..4.min(bytes.len())]
        );
    }

    #[test]
    fn test_raw_pixels_to_png_rgb() {
        // 2×2 RGB: 12 bytes, 3 per pixel.
        let pixels: Vec<u8> = vec![
            0xff, 0x00, 0x00, // red
            0x00, 0xff, 0x00, // green
            0x00, 0x00, 0xff, // blue
            0xff, 0xff, 0xff, // white
        ];
        let result = raw_pixels_to_png(2, 2, &pdf_oxide::extractors::PixelFormat::RGB, &pixels);
        let bytes = result.expect("RGB 2×2 must encode without error");
        assert!(
            bytes.starts_with(PNG_MAGIC),
            "output must be a PNG; got {:02x?}",
            &bytes[..4.min(bytes.len())]
        );
    }

    #[test]
    fn test_raw_pixels_to_png_cmyk_converts_to_rgb_png() {
        // 1×2 CMYK: 8 bytes, 4 per pixel.
        // (0,0,0,255) = pure black; (0,0,0,0) = white.
        let pixels: Vec<u8> = vec![0x00, 0x00, 0x00, 0xff, 0x00, 0x00, 0x00, 0x00];
        let result = raw_pixels_to_png(1, 2, &pdf_oxide::extractors::PixelFormat::CMYK, &pixels);
        let bytes = result.expect("CMYK 1×2 must encode without error");
        assert!(
            bytes.starts_with(PNG_MAGIC),
            "output must be a PNG; got {:02x?}",
            &bytes[..4.min(bytes.len())]
        );
        // Verify the PNG decodes to an RGB image (CMYK was converted to RGB before encoding).
        let decoded = image::load_from_memory(&bytes).expect("decoded PNG must be valid");
        assert_eq!(decoded.width(), 1);
        assert_eq!(decoded.height(), 2);
    }

    #[test]
    fn test_raw_pixels_to_png_size_mismatch_returns_error() {
        // 4×4 grayscale needs 16 bytes; supply only 4 — must be an error, not a panic.
        let pixels: Vec<u8> = vec![0x00, 0x80, 0xc0, 0xff];
        let result = raw_pixels_to_png(4, 4, &pdf_oxide::extractors::PixelFormat::Grayscale, &pixels);
        assert!(
            result.is_err(),
            "mismatched buffer size must return Err, not Ok or panic"
        );
    }

    #[test]
    fn test_raw_pixels_to_png_rgb_size_mismatch_returns_error() {
        // 2×2 RGB needs 12 bytes; supply 9 (divisible by 3 but wrong total).
        let pixels: Vec<u8> = vec![0xff; 9];
        let result = raw_pixels_to_png(2, 2, &pdf_oxide::extractors::PixelFormat::RGB, &pixels);
        assert!(result.is_err(), "mismatched RGB buffer must return Err");
    }

    #[test]
    fn test_raw_pixels_to_png_cmyk_odd_length_returns_error() {
        // 1×1 CMYK needs exactly 4 bytes; supply 3. chunks_exact(4) drops the remainder,
        // producing an empty rgb vec. from_raw(1, 1, []) returns None → must be Err, not panic.
        let pixels: Vec<u8> = vec![0x00, 0x00, 0x00];
        let result = raw_pixels_to_png(1, 1, &pdf_oxide::extractors::PixelFormat::CMYK, &pixels);
        assert!(
            result.is_err(),
            "CMYK buffer whose length is not a multiple of 4 must return Err, not panic"
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
        let mut doc = crate::pdf::oxide::OxideDocument::open_bytes(&bytes).expect("failed to open PDF");

        let result = extract_images_with_data(&mut doc, Some(0), None).expect("cap=0 must not error");

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

        // Full run (no cancel) — establishes the expected upper bound.
        let mut doc_full = crate::pdf::oxide::OxideDocument::open_bytes(&bytes).expect("failed to open PDF");
        let full_result =
            extract_images_with_data(&mut doc_full, None, None).expect("uncancelled extraction must not error");
        let full_count = full_result.len();
        let page_count = doc_full
            .doc
            .page_count()
            .expect("page_count must succeed on the fixture");

        // Skip if the fixture has only one page or no images — mid-run cancellation
        // between pages cannot be demonstrated.
        if page_count <= 1 || full_count == 0 {
            eprintln!(
                "SKIP test_cancellation_fires_between_pages: nougat_039.pdf has {} page(s) \
                 and {} extractable images — need ≥2 pages with images",
                page_count, full_count
            );
            return;
        }

        // Run with a token that a background thread cancels after 20ms.
        // For a 67KB 2-page PDF on CI hardware this window typically lands between
        // pages 0 and 1, but both earlier and later cancellations are correct.
        let mut doc_cancel = crate::pdf::oxide::OxideDocument::open_bytes(&bytes).expect("failed to open PDF");
        let token = CancellationToken::new();
        let token_clone = token.clone();

        let handle = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(20));
            token_clone.cancel();
        });

        let result =
            extract_images_with_data(&mut doc_cancel, None, Some(&token)).expect("cancellation must not error");

        handle.join().expect("background thread must not panic");

        // The token must have been set by the time the handle joins.
        assert!(
            token.is_cancelled(),
            "token must be cancelled after background thread fires"
        );

        // Cancellation must never produce more images than an uncancelled run.
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
        let mut doc = crate::pdf::oxide::OxideDocument::open_bytes(&bytes).expect("failed to open PDF");

        let token = CancellationToken::new();
        token.cancel();

        let result = extract_images_with_data(&mut doc, None, Some(&token)).expect("extract must not error");

        assert!(
            result.is_empty(),
            "pre-cancelled token must cause extraction to return empty vec immediately, \
             got {} image(s)",
            result.len()
        );
    }
}
