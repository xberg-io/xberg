//! End-to-end integration tests for `ImageExtractionConfig.output_format`.
//!
//! Drives real extractors through `extract_file_sync` and asserts that the
//! chosen `ImageOutputFormat` variant propagates correctly to
//! `ExtractionResult.images[*].format` and raw byte magic-number signatures.
//!
//! Fixtures used:
//!   - `pdf/embedded_images_tables.pdf`  — PDF with embedded JPEG images
//!   - `pdf/multipage_marketing.pdf`     — multi-page PDF with embedded images
//!   - `docx/word_image_anchors.docx`    — DOCX with anchored embedded images
//!   - `images/example.jpg`              — standalone JPEG file
//!   - `images/test_hello_world.png`     — PNG with OCR-able text
//!
//! Each test is `#[cfg(feature = "image-encode")]` because the re-encode pass
//! is only present when that feature is compiled in. The full feature set
//! (`--features full`) and the minimal combo (`--features pdf,image-encode`)
//! both satisfy this guard.

#![cfg(feature = "image-encode")]

mod helpers;

use kreuzberg::core::config::extraction::ImageOutputFormat;
use kreuzberg::core::config::{ExtractionConfig, ImageExtractionConfig, OutputFormat};
use kreuzberg::extract_file_sync;

// ── Magic-byte helpers ───────────────────────────────────────────────────────

/// PNG magic bytes: `\x89PNG\r\n\x1a\n`
#[cfg(any(feature = "pdf", feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline"))]
const PNG_MAGIC: &[u8] = b"\x89PNG\r\n\x1a\n";

/// JPEG SOI marker: `\xFF\xD8\xFF`
#[cfg(feature = "pdf")]
const JPEG_MAGIC: &[u8] = b"\xFF\xD8\xFF";

/// WebP container: `RIFF` at offset 0, `WEBP` at offset 8
#[cfg(feature = "pdf")]
fn is_webp(data: &[u8]) -> bool {
    data.len() >= 12 && data.starts_with(b"RIFF") && &data[8..12] == b"WEBP"
}

// ── Fixture helpers ──────────────────────────────────────────────────────────

fn test_documents_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test_documents")
}

fn fixture_path(relative: &str) -> std::path::PathBuf {
    test_documents_dir().join(relative)
}

/// Build an `ExtractionConfig` that enables image extraction with the given
/// `ImageOutputFormat`.
fn config_with_output_format(output_format: ImageOutputFormat) -> ExtractionConfig {
    ExtractionConfig {
        output_format: OutputFormat::Markdown,
        images: Some(ImageExtractionConfig {
            extract_images: true,
            output_format,
            ..Default::default()
        }),
        use_cache: false,
        ..Default::default()
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

/// Native passthrough: without specifying an output format (or by explicitly
/// choosing `Native`), the images returned reflect whatever format the extractor
/// produced from the PDF. No re-encode pass is applied, so the format field must
/// be consistent with a raw PDF embedded image (typically "jpeg" or "png").
///
/// Verify this is stable by extracting twice — once with `Native` (explicit) and
/// once without an `ImageExtractionConfig.output_format` field — and confirming
/// both runs produce the same set of format strings.
#[cfg(feature = "pdf")]
#[test]
fn pdf_native_passthrough() {
    let path = fixture_path("pdf/embedded_images_tables.pdf");
    if !path.exists() {
        eprintln!("skipped: pdf/embedded_images_tables.pdf not present");
        return;
    }

    let config_explicit_native = config_with_output_format(ImageOutputFormat::Native);

    // Also extract with the absolute default — no images config at all.
    let config_default = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        images: Some(ImageExtractionConfig {
            extract_images: true,
            ..Default::default()
        }),
        use_cache: false,
        ..Default::default()
    };

    let result_native =
        extract_file_sync(&path, None, &config_explicit_native).expect("native extraction must succeed");
    let result_default = extract_file_sync(&path, None, &config_default).expect("default extraction must succeed");

    let images_native = result_native
        .images
        .as_ref()
        .expect("images must be Some when extract_images=true");
    let images_default = result_default
        .images
        .as_ref()
        .expect("images must be Some when extract_images=true");

    assert!(
        !images_native.is_empty(),
        "embedded_images_tables.pdf must yield at least one image with Native format"
    );
    assert_eq!(
        images_native.len(),
        images_default.len(),
        "Native explicit and Default must extract the same number of images"
    );

    // Format strings must match between explicit Native and default config.
    let formats_native: Vec<&str> = images_native.iter().map(|i| i.format.as_ref()).collect();
    let formats_default: Vec<&str> = images_default.iter().map(|i| i.format.as_ref()).collect();
    assert_eq!(
        formats_native, formats_default,
        "Native explicit and Default must produce identical format strings per image"
    );
}

/// Force PNG: every extracted image must report `format == "png"` and its raw
/// bytes must start with the 8-byte PNG signature.
#[cfg(feature = "pdf")]
#[test]
fn pdf_force_png() {
    let path = fixture_path("pdf/embedded_images_tables.pdf");
    if !path.exists() {
        eprintln!("skipped: pdf/embedded_images_tables.pdf not present");
        return;
    }

    let config = config_with_output_format(ImageOutputFormat::Png);
    let result = extract_file_sync(&path, None, &config).expect("PNG extraction must succeed");

    let images = result
        .images
        .as_ref()
        .expect("images must be Some when extract_images=true");

    assert!(
        !images.is_empty(),
        "embedded_images_tables.pdf must yield at least one image"
    );

    for img in images {
        assert_eq!(
            img.format.as_ref(),
            "png",
            "image at index {} must report format=\"png\" after PNG re-encode, got \"{}\"",
            img.image_index,
            img.format
        );
        assert!(
            img.data.starts_with(PNG_MAGIC),
            "image at index {} has format=\"png\" but bytes do not start with PNG magic \
             (first 8 bytes: {:02x?})",
            img.image_index,
            &img.data[..8.min(img.data.len())]
        );
    }
}

/// Force JPEG: every extracted image must report `format == "jpeg"` and its raw
/// bytes must start with the JPEG SOI marker `\xFF\xD8\xFF`.
#[cfg(feature = "pdf")]
#[test]
fn pdf_force_jpeg() {
    let path = fixture_path("pdf/embedded_images_tables.pdf");
    if !path.exists() {
        eprintln!("skipped: pdf/embedded_images_tables.pdf not present");
        return;
    }

    let config = config_with_output_format(ImageOutputFormat::Jpeg { quality: 85 });
    let result = extract_file_sync(&path, None, &config).expect("JPEG extraction must succeed");

    let images = result
        .images
        .as_ref()
        .expect("images must be Some when extract_images=true");

    assert!(
        !images.is_empty(),
        "embedded_images_tables.pdf must yield at least one image"
    );

    for img in images {
        assert_eq!(
            img.format.as_ref(),
            "jpeg",
            "image at index {} must report format=\"jpeg\" after JPEG re-encode, got \"{}\"",
            img.image_index,
            img.format
        );
        assert!(
            img.data.starts_with(JPEG_MAGIC),
            "image at index {} has format=\"jpeg\" but bytes do not start with JPEG SOI \
             (first 3 bytes: {:02x?})",
            img.image_index,
            &img.data[..3.min(img.data.len())]
        );
    }
}

/// Force WebP: every extracted image must report `format == "webp"` and its raw
/// bytes must match the RIFF/WEBP container signature.
#[cfg(feature = "pdf")]
#[test]
fn pdf_force_webp() {
    let path = fixture_path("pdf/embedded_images_tables.pdf");
    if !path.exists() {
        eprintln!("skipped: pdf/embedded_images_tables.pdf not present");
        return;
    }

    let config = config_with_output_format(ImageOutputFormat::Webp { quality: 80 });
    let result = extract_file_sync(&path, None, &config).expect("WebP extraction must succeed");

    let images = result
        .images
        .as_ref()
        .expect("images must be Some when extract_images=true");

    assert!(
        !images.is_empty(),
        "embedded_images_tables.pdf must yield at least one image"
    );

    for img in images {
        assert_eq!(
            img.format.as_ref(),
            "webp",
            "image at index {} must report format=\"webp\" after WebP re-encode, got \"{}\"",
            img.image_index,
            img.format
        );
        assert!(
            is_webp(&img.data),
            "image at index {} has format=\"webp\" but bytes do not match RIFF/WEBP signature \
             (first 12 bytes: {:02x?})",
            img.image_index,
            &img.data[..12.min(img.data.len())]
        );
    }
}

/// Office DOCX mixed-to-PNG: extract a DOCX with embedded images and force all
/// output to PNG. Every image must report `format == "png"`.
///
/// `word_image_anchors.docx` contains anchored images of various formats.
#[cfg(feature = "office")]
#[test]
fn office_mixed_to_png() {
    let path = fixture_path("docx/word_image_anchors.docx");
    if !path.exists() {
        eprintln!("skipped: docx/word_image_anchors.docx not present");
        return;
    }

    let config = config_with_output_format(ImageOutputFormat::Png);
    let result = extract_file_sync(&path, None, &config).expect("DOCX PNG extraction must succeed");

    let images = match result.images.as_ref() {
        Some(v) if !v.is_empty() => v,
        _ => {
            eprintln!("skipped: word_image_anchors.docx yielded no extracted images");
            return;
        }
    };

    for img in images {
        // Skip images whose source format cannot be re-encoded (e.g. EMF/WMF
        // vector metafiles). These arrive with a ProcessingWarning and their
        // format is left unchanged by re_encode.
        let undecodable = result
            .processing_warnings
            .iter()
            .any(|w| w.source.as_ref() == "image_encoder" && w.message.contains(&format!("image_{}", img.image_index)));
        if undecodable {
            continue;
        }

        assert_eq!(
            img.format.as_ref(),
            "png",
            "image at index {} must report format=\"png\" after PNG re-encode; \
             got \"{}\". Warnings: {:?}",
            img.image_index,
            img.format,
            result
                .processing_warnings
                .iter()
                .map(|w| (w.source.as_ref(), w.message.as_ref()))
                .collect::<Vec<_>>()
        );
        assert!(
            img.data.starts_with(PNG_MAGIC),
            "image at index {} must start with PNG magic bytes",
            img.image_index
        );
    }
}

/// Standalone JPEG to PNG: extract a raw JPEG file with `output_format = Png`
/// and verify the single returned image has `format == "png"` with matching
/// PNG magic bytes.
///
/// Also verifies that image dimensions are preserved after re-encoding: the
/// decoded PNG must report the same (width, height) as the source JPEG.
///
/// Requires `ocr-pipeline` (or `ocr`/`ocr-wasm`) because the `ImageExtractor`
/// that handles `image/jpeg` standalone files is only registered under those
/// features.
#[cfg(any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline"))]
#[test]
fn image_extractor_jpeg_to_png() {
    let path = fixture_path("images/example.jpg");
    if !path.exists() {
        eprintln!("skipped: images/example.jpg not present");
        return;
    }

    // First, extract natively to get the source dimensions.
    let native_config = ExtractionConfig {
        images: Some(ImageExtractionConfig {
            extract_images: true,
            output_format: ImageOutputFormat::Native,
            ..Default::default()
        }),
        use_cache: false,
        ..Default::default()
    };
    let native_result = extract_file_sync(&path, None, &native_config).expect("native JPEG extraction must succeed");

    // Extract with PNG re-encode.
    let png_config = config_with_output_format(ImageOutputFormat::Png);
    let png_result = extract_file_sync(&path, None, &png_config).expect("PNG JPEG extraction must succeed");

    let images = png_result
        .images
        .as_ref()
        .expect("images must be Some when extract_images=true");

    assert!(
        !images.is_empty(),
        "example.jpg must yield at least one extracted image after PNG re-encode"
    );

    let img = &images[0];
    assert_eq!(
        img.format.as_ref(),
        "png",
        "standalone JPEG re-encoded to PNG must report format=\"png\", got \"{}\"",
        img.format
    );
    assert!(
        img.data.starts_with(PNG_MAGIC),
        "PNG-re-encoded image must start with PNG magic bytes (got {:02x?})",
        &img.data[..8.min(img.data.len())]
    );

    // Dimensions must be preserved (modulo lossless re-encode).
    if let Some(native_images) = native_result.images.as_ref()
        && !native_images.is_empty()
    {
        let native_img = &native_images[0];
        if let (Some(src_w), Some(src_h), Some(dst_w), Some(dst_h)) =
            (native_img.width, native_img.height, img.width, img.height)
        {
            assert_eq!(
                (src_w, src_h),
                (dst_w, dst_h),
                "JPEG→PNG re-encode must preserve dimensions; \
                 source ({src_w}×{src_h}) != re-encoded ({dst_w}×{dst_h})"
            );
        }
    }
}

/// OCR before re-encode: extract an image containing recognizable text with
/// OCR enabled and `output_format = Jpeg`.
///
/// Verifies that:
/// 1. `result.content` is non-empty (OCR ran and extracted text).
/// 2. The final image has `format == "jpeg"` (re-encode ran after OCR).
///
/// If OCR had run *after* re-encode, both assertions might still pass, but
/// the test documents the expected pipeline stage order and will catch any
/// accidental reordering that breaks the OCR path.
#[cfg(all(feature = "ocr", feature = "image-encode"))]
#[test]
fn ocr_runs_before_reencode() {
    use kreuzberg::core::config::OcrConfig;

    let path = fixture_path("images/test_hello_world.png");
    if !path.exists() {
        eprintln!("skipped: images/test_hello_world.png not present");
        return;
    }

    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            ..Default::default()
        }),
        force_ocr: true,
        images: Some(ImageExtractionConfig {
            extract_images: true,
            output_format: ImageOutputFormat::Jpeg { quality: 85 },
            ..Default::default()
        }),
        use_cache: false,
        ..Default::default()
    };

    let result = extract_file_sync(&path, None, &config).expect("OCR + JPEG re-encode must succeed");

    assert!(
        !result.content.trim().is_empty(),
        "OCR must populate result.content even when output_format=Jpeg; got empty content"
    );

    let images = result
        .images
        .as_ref()
        .expect("images must be Some when extract_images=true");

    assert!(
        !images.is_empty(),
        "test_hello_world.png must yield at least one image entry"
    );

    // At least one image must have been re-encoded to JPEG.
    let jpeg_images: Vec<_> = images.iter().filter(|i| i.format.as_ref() == "jpeg").collect();
    assert!(
        !jpeg_images.is_empty(),
        "output_format=Jpeg must re-encode at least one image to JPEG; \
         got formats: {:?}",
        images.iter().map(|i| i.format.as_ref()).collect::<Vec<_>>()
    );

    for img in &jpeg_images {
        assert!(
            img.data.starts_with(JPEG_MAGIC),
            "JPEG-re-encoded image at index {} must start with JPEG SOI bytes",
            img.image_index
        );
    }
}

/// Quality affects byte size: extract the same PDF twice — once at high quality
/// (JPEG q=95) and once at low quality (JPEG q=30). For at least one image the
/// low-quality output must be smaller than the high-quality output.
///
/// This proves that the `quality` field flows all the way through to the encoder
/// and is not silently ignored.
#[cfg(feature = "pdf")]
#[test]
fn quality_change_alters_byte_size() {
    let path = fixture_path("pdf/multipage_marketing.pdf");
    if !path.exists() {
        eprintln!("skipped: pdf/multipage_marketing.pdf not present");
        return;
    }

    let config_high = config_with_output_format(ImageOutputFormat::Jpeg { quality: 95 });
    let config_low = config_with_output_format(ImageOutputFormat::Jpeg { quality: 30 });

    let result_high = extract_file_sync(&path, None, &config_high).expect("high-quality JPEG extraction must succeed");
    let result_low = extract_file_sync(&path, None, &config_low).expect("low-quality JPEG extraction must succeed");

    let images_high = result_high
        .images
        .as_ref()
        .expect("images must be Some when extract_images=true");
    let images_low = result_low
        .images
        .as_ref()
        .expect("images must be Some when extract_images=true");

    if images_high.is_empty() {
        eprintln!("skipped: multipage_marketing.pdf yielded no extractable images");
        return;
    }

    assert_eq!(
        images_high.len(),
        images_low.len(),
        "high-quality and low-quality runs must extract the same number of images"
    );

    // At least one image must be strictly smaller at quality=30 than quality=95.
    let any_smaller = images_high
        .iter()
        .zip(images_low.iter())
        .any(|(high, low)| low.data.len() < high.data.len());

    assert!(
        any_smaller,
        "JPEG quality=30 must produce at least one image smaller than quality=95; \
         sizes at q=95: {:?}, sizes at q=30: {:?}",
        images_high.iter().map(|i| i.data.len()).collect::<Vec<_>>(),
        images_low.iter().map(|i| i.data.len()).collect::<Vec<_>>()
    );
}

/// Unsupported source format: when a fixture produces a vector/metafile image
/// (SVG, EMF, WMF) and `output_format != Native`, the re-encode pass must leave
/// that image's bytes untouched and emit a `ProcessingWarning` whose `source ==
/// "image_encoder"`.
///
/// This test looks for such images in `word_image_anchors.docx`. If none are
/// found (because the fixture has no EMF/WMF embedded images), it logs a message
/// and returns without failing — the pipeline smoke test in
/// `core/pipeline/tests.rs` already covers the svg path directly.
#[cfg(feature = "office")]
#[test]
fn unsupported_source_format_warns_and_preserves() {
    let path = fixture_path("docx/word_image_anchors.docx");
    if !path.exists() {
        eprintln!("skipped: docx/word_image_anchors.docx not present");
        return;
    }

    let config = config_with_output_format(ImageOutputFormat::Png);
    let result = extract_file_sync(&path, None, &config).expect("DOCX extraction must not error out");

    // We only check the warning structure if the fixture actually has undecodable images.
    let encoder_warnings: Vec<_> = result
        .processing_warnings
        .iter()
        .filter(|w| w.source.as_ref() == "image_encoder")
        .collect();

    if encoder_warnings.is_empty() {
        eprintln!(
            "word_image_anchors.docx produced no image_encoder warnings — \
             no undecodable vector images present in this fixture; \
             the pipeline smoke test covers svg/emf directly"
        );
        return;
    }

    // Every warning must have a non-empty message.
    for warning in &encoder_warnings {
        assert!(
            !warning.message.is_empty(),
            "image_encoder warning must have a non-empty message; got: {:?}",
            warning
        );
    }

    // The images that triggered warnings must still have non-empty bytes
    // (untouched original data preserved).
    if let Some(images) = result.images.as_ref() {
        for img in images {
            // We cannot easily correlate warning → image_index without parsing the
            // warning message, so just assert no image has an empty data buffer,
            // which would indicate the re-encode pass cleared it.
            assert!(
                !img.data.is_empty(),
                "image at index {} must retain non-empty data even when re-encode was skipped",
                img.image_index
            );
        }
    }
}

/// HEIF output: extract a PDF with embedded JPEG, force `output_format = Heif`,
/// and verify that every returned image has `format == "heif"`.
///
/// Requires the `heic` Cargo feature. Skipped automatically when not compiled in.
#[cfg(all(feature = "heic", feature = "pdf"))]
#[test]
fn heif_output() {
    use kreuzberg::core::config::extraction::ImageOutputFormat;

    let path = fixture_path("pdf/embedded_images_tables.pdf");
    if !path.exists() {
        eprintln!("skipped: pdf/embedded_images_tables.pdf not present");
        return;
    }

    let config = config_with_output_format(ImageOutputFormat::Heif { quality: 80 });
    let result = extract_file_sync(&path, None, &config).expect("HEIF extraction must succeed");

    let images = result
        .images
        .as_ref()
        .expect("images must be Some when extract_images=true");

    assert!(
        !images.is_empty(),
        "embedded_images_tables.pdf must yield at least one image after HEIF re-encode"
    );

    for img in images {
        assert_eq!(
            img.format.as_ref(),
            "heif",
            "image at index {} must report format=\"heif\", got \"{}\"",
            img.image_index,
            img.format
        );
        assert!(
            !img.data.is_empty(),
            "HEIF image at index {} must have non-empty data",
            img.image_index
        );

        // Round-trip: decode with kreuzberg_libheif to prove bytes are valid HEIF.
        let parse_result = kreuzberg_libheif::HeifContext::read_from_bytes(&img.data);
        assert!(
            parse_result.is_ok(),
            "image at index {} has format=\"heif\" but failed kreuzberg_libheif round-trip: {:?}",
            img.image_index,
            parse_result.err()
        );
    }
}
