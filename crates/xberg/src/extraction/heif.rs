//! HEIF / HEIC / AVIF detection and decoding.
//!
//! The sniffer (`is_heif_container`) is always compiled — it's a 12-byte magic
//! check used by `extract_image_metadata` to dispatch. Actual pixel decoding
//! (`decode_heic_to_png`) lives behind the `heic` Cargo feature because it
//! pulls in the C `libheif` dependency via `xberg-libheif`.

#[cfg(feature = "heic")]
use crate::error::{Result, XbergError};

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
pub(crate) fn decode_heic_to_png(bytes: &[u8]) -> Result<Vec<u8>> {
    use image::ImageEncoder;
    use image::codecs::png::PngEncoder;
    use xberg_libheif::{ColorSpace, HeifContext, LibHeif, RgbChroma};

    let lib = LibHeif::new();
    let ctx = HeifContext::read_from_bytes(bytes)
        .map_err(|e| XbergError::parsing(format!("Failed to read HEIF container: {e}")))?;
    let handle = ctx
        .primary_image_handle()
        .map_err(|e| XbergError::parsing(format!("Failed to read HEIF primary image handle: {e}")))?;
    let image = lib
        .decode(&handle, ColorSpace::Rgb(RgbChroma::Rgba), None)
        .map_err(|e| XbergError::parsing(format!("Failed to decode HEIF image: {e}")))?;

    let width = image.width();
    let height = image.height();
    let planes = image.planes();
    let plane = planes
        .interleaved
        .ok_or_else(|| XbergError::parsing("HEIF decode returned no interleaved RGBA plane".to_string()))?;

    // libheif rows are padded to `stride` bytes. Repack to a tight (width * 4)-byte
    // row layout before handing off to the PNG encoder.
    let row_bytes = (width as usize) * 4;
    let mut packed = Vec::with_capacity(row_bytes * (height as usize));
    for row in 0..(height as usize) {
        let start = row * plane.stride;
        let end = start + row_bytes;
        if end > plane.data.len() {
            return Err(XbergError::parsing(
                "HEIF decoded plane shorter than declared dimensions".to_string(),
            ));
        }
        packed.extend_from_slice(&plane.data[start..end]);
    }

    let mut png_bytes = Vec::with_capacity(row_bytes * height as usize);
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
        // Synthesise an ftyp box with each major brand we register.
        let make = |brand: &[u8; 4]| {
            let mut buf = Vec::from(&b"\x00\x00\x00\x18"[..]); // box size (24)
            buf.extend_from_slice(b"ftyp");
            buf.extend_from_slice(brand);
            buf.extend_from_slice(&[0u8; 12]); // padding to fill the box
            buf
        };
        // ISO/IEC 14496-12 / 23008-12 / 23000-22 major brands actually emitted by
        // HEIF-family encoders. `heif` itself is not a real ftyp brand — HEIF still
        // images use `mif1` or codec-specific brands (`heic`, `heix`, `avif`, …).
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
        // PNG magic bytes
        assert!(!is_heif_container(b"\x89PNG\r\n\x1a\n0000"));
        // ftyp box with an unknown brand
        assert!(!is_heif_container(b"\x00\x00\x00\x18ftypxxxxRESERVED___"));
    }

    #[cfg(feature = "heic")]
    #[test]
    fn decode_heic_to_png_produces_valid_png() {
        use image::ImageReader;
        use std::io::Cursor;

        const HEIC: &[u8] = include_bytes!("../../../../test_documents/images/test.heic");
        let png = decode_heic_to_png(HEIC).expect("decode_heic_to_png");
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n", "output is not a PNG");

        let reader = ImageReader::new(Cursor::new(&png))
            .with_guessed_format()
            .expect("guess format on decoded PNG");
        let (w, h) = reader.into_dimensions().expect("PNG dimensions");
        assert!(w > 0 && h > 0);
    }
}
