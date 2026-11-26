//! HEIF / HEIC / AVIF detection and decoding.
//!
//! The sniffer (`is_heif_container`) is always compiled — it's a 12-byte magic
//! check used by `extract_image_metadata` to dispatch. Actual pixel decoding
//! (`decode_heic_to_png`) lives behind the `heic` Cargo feature because it
//! pulls in the C `libheif` dependency via `xberg-libheif`.

#[cfg(feature = "heic")]
use crate::error::{Result, XbergError};
#[cfg(feature = "heic")]
use crate::extraction::image_decode::{
    ImageDecodeBudget, copy_decoded_rows, decoded_byte_count, image_dimension_error,
};
#[cfg(feature = "heic")]
use crate::extractors::security::SecurityLimits;

#[cfg(feature = "heic")]
const HEIF_TO_PNG_BUFFER_COUNT: u64 = 3;

/// A missing libheif (`heic-dynamic` without the library) as a missing dependency, so it is not
/// reported as a malformed file.
#[cfg(feature = "heic")]
pub(crate) fn require_libheif() -> Result<()> {
    xberg_libheif::ensure_available()
        .map_err(|error| XbergError::MissingDependency(format!("HEIF decoding needs libheif: {}", error.message)))
}

#[cfg(feature = "heic")]
fn validate_heif_encoded_input_budget(bytes: &[u8], limits: &SecurityLimits) -> Result<()> {
    ImageDecodeBudget::from_security_limits(limits).validate(1, 1, u64::try_from(bytes.len()).unwrap_or(u64::MAX))
}

#[cfg(feature = "heic")]
fn validate_heif_decode_budget(width: u32, height: u32, encoded_bytes: usize, limits: &SecurityLimits) -> Result<()> {
    let rgba_bytes = decoded_byte_count(width, height, u64::from(image::ColorType::Rgba8.bytes_per_pixel()))?;
    let peak_decoded_bytes = rgba_bytes
        .checked_mul(HEIF_TO_PNG_BUFFER_COUNT)
        .and_then(|bytes| bytes.checked_add(u64::try_from(encoded_bytes).unwrap_or(u64::MAX)))
        .ok_or_else(|| image_dimension_error(width, height, u64::MAX, u64::MAX))?;
    ImageDecodeBudget::from_security_limits(limits).validate(width, height, peak_decoded_bytes)
}

/// Detect a HEIF-family container (HEIC / HEIF / AVIF / HEICS / AVCS) by
/// sniffing the `ftyp` box brand at offset 4..8 with one of the known major
/// brands at 8..12.
///
/// The function is always compiled (12-byte magic check, zero deps), but every
/// caller lives behind one of the OCR features. Reranker-only builds without
/// any OCR feature would surface this as `dead_code`; the `#[allow]` keeps the
/// unconditional definition stance documented in the module doc.
#[allow(dead_code)]
pub(crate) fn is_heif_container(bytes: &[u8]) -> bool {
    if bytes.len() < 12 || &bytes[4..8] != b"ftyp" {
        return false;
    }
    matches!(
        &bytes[8..12],
        b"heic"
            | b"heix"
            | b"heim"
            | b"heis"
            | b"hevc"
            | b"hevm"
            | b"hevs"
            | b"mif1"
            | b"msf1"
            | b"avif"
            | b"avis"
            | b"avcs"
    )
}

/// Decode any HEIF-family container to PNG bytes via the vendored libheif
/// bindings.
///
/// Decoded as interleaved RGBA, then re-encoded as PNG so the result can flow
/// through the existing OCR / image pipeline without further special-casing.
#[cfg(feature = "heic")]
pub(crate) fn decode_heic_to_png(bytes: &[u8], limits: &SecurityLimits) -> Result<Vec<u8>> {
    use image::ImageEncoder;
    use image::codecs::png::PngEncoder;
    use xberg_libheif::{ColorSpace, HeifContext, LibHeif, RgbChroma};

    require_libheif()?;
    let lib = LibHeif::new();
    validate_heif_encoded_input_budget(bytes, limits)?;
    let ctx = HeifContext::read_from_bytes(bytes)
        .map_err(|e| XbergError::parsing(format!("Failed to read HEIF container: {e}")))?;
    let handle = ctx
        .primary_image_handle()
        .map_err(|e| XbergError::parsing(format!("Failed to read HEIF primary image handle: {e}")))?;
    let width = handle.width();
    let height = handle.height();
    validate_heif_decode_budget(width, height, bytes.len(), limits)?;
    let image = lib
        .decode(&handle, ColorSpace::Rgb(RgbChroma::Rgba), None)
        .map_err(|e| XbergError::parsing(format!("Failed to decode HEIF image: {e}")))?;

    let decoded_width = image.width();
    let decoded_height = image.height();
    if decoded_width != width || decoded_height != height {
        return Err(XbergError::parsing(format!(
            "HEIF decoded dimensions {decoded_width}x{decoded_height} do not match declared dimensions {width}x{height}"
        )));
    }
    let planes = image.planes();
    let plane = planes
        .interleaved
        .ok_or_else(|| XbergError::parsing("HEIF decode returned no interleaved RGBA plane".to_string()))?;

    let packed = copy_decoded_rows(
        plane.data,
        plane.stride,
        width,
        height,
        u64::from(image::ColorType::Rgba8.bytes_per_pixel()),
    )?;

    let mut png_bytes = Vec::new();
    let packed_bytes = packed.len();
    png_bytes
        .try_reserve_exact(packed_bytes)
        .map_err(|error| XbergError::parsing(format!("Failed to reserve HEIF PNG output buffer: {error}")))?;
    PngEncoder::new(&mut png_bytes)
        .write_image(&packed, width, height, image::ExtendedColorType::Rgba8)
        .map_err(|e| XbergError::parsing(format!("Failed to re-encode HEIF as PNG: {e}")))?;
    Ok(png_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_known_heif_brands() {
        let make = |brand: &[u8; 4]| {
            let mut buf = Vec::from(&b"\x00\x00\x00\x18"[..]);
            buf.extend_from_slice(b"ftyp");
            buf.extend_from_slice(brand);
            buf.extend_from_slice(&[0u8; 12]);
            buf
        };
        for brand in [b"heic", b"heix", b"avif", b"avcs", b"mif1", b"avis"] {
            assert!(
                is_heif_container(&make(brand)),
                "brand {:?} should sniff as HEIF",
                std::str::from_utf8(brand).unwrap()
            );
        }
    }

    #[test]
    fn rejects_non_heif() {
        assert!(!is_heif_container(b""));
        assert!(!is_heif_container(b"hello world"));
        assert!(!is_heif_container(&[0u8; 4]));
        assert!(!is_heif_container(b"\x89PNG\r\n\x1a\n0000"));
        assert!(!is_heif_container(b"\x00\x00\x00\x18ftypxxxxRESERVED___"));
    }

    #[cfg(feature = "heic")]
    #[test]
    fn decode_heic_to_png_produces_valid_png() {
        use image::ImageReader;
        use std::io::Cursor;

        let Some(heic) = crate::utils::read_test_fixture("images/test.heic") else {
            return;
        };
        let png = decode_heic_to_png(&heic, &SecurityLimits::default()).expect("decode_heic_to_png");
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n", "output is not a PNG");

        let reader = ImageReader::new(Cursor::new(&png))
            .with_guessed_format()
            .expect("guess format on decoded PNG");
        let (w, h) = reader.into_dimensions().expect("PNG dimensions");
        assert!(w > 0 && h > 0);
    }

    #[cfg(feature = "heic")]
    #[test]
    fn should_reject_oversized_heic_dimensions_before_pixel_decode() {
        let error = validate_heif_decode_budget(6_000, 6_000, 0, &SecurityLimits::default())
            .expect_err("oversized HEIC must fail at the decoded-image budget");

        assert!(
            error.to_string().contains("security_limits.max_content_size"),
            "unexpected error: {error}"
        );
    }

    #[cfg(feature = "heic")]
    #[test]
    fn should_count_encoded_heif_bytes_between_old_and_new_thresholds() {
        let limits = SecurityLimits {
            max_content_size: 1_250,
            ..Default::default()
        };

        let error = validate_heif_decode_budget(10, 10, 100, &limits)
            .expect_err("encoded HEIF bytes must remain live with decode and PNG buffers");

        assert!(matches!(error, XbergError::Validation { .. }));
    }

    #[cfg(feature = "heic-dynamic")]
    #[test]
    fn a_missing_libheif_is_a_missing_dependency_not_a_parsing_error() {
        let test = "a_missing_libheif_is_a_missing_dependency_not_a_parsing_error";
        let child = "XBERG_HEIF_MISSING_LIBRARY_CHILD";
        if std::env::var(child).as_deref() == Ok(test) {
            let heif = b"\x00\x00\x00\x18ftypheic\x00\x00\x00\x00mif1heic";
            let error = decode_heic_to_png(heif, &SecurityLimits::default()).expect_err("no libheif to decode with");
            assert!(
                matches!(&error, XbergError::MissingDependency(message) if message.contains("XBERG_LIBHEIF_PATH")),
                "decode: {error:?}"
            );
            #[cfg(any(feature = "ocr", feature = "ocr-wasm", feature = "ocr-pipeline"))]
            {
                let error = crate::extraction::image::extract_image_metadata_with_security_limits(
                    heif,
                    &SecurityLimits::default(),
                )
                .expect_err("no libheif to read the container with");
                assert!(matches!(error, XbergError::MissingDependency(_)), "metadata: {error:?}");
            }
            return;
        }

        let directory = tempfile::tempdir().expect("create a directory for the missing library");
        let output = std::process::Command::new(std::env::current_exe().expect("locate the test executable"))
            .args(["--exact", &format!("extraction::heif::tests::{test}"), "--nocapture"])
            .env("XBERG_LIBHEIF_PATH", directory.path().join("libheif-missing"))
            .env(child, test)
            .output()
            .expect("run the isolated missing-libheif test");
        assert!(
            output.status.success() && String::from_utf8_lossy(&output.stdout).contains("1 passed; 0 failed"),
            "isolated missing-libheif test failed: stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }
}
