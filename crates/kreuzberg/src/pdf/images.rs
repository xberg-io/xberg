use super::error::{PdfError, Result};
use bytes::Bytes;
use lopdf::Document;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfImage {
    pub page_number: usize,
    pub image_index: usize,
    pub width: i64,
    pub height: i64,
    pub color_space: Option<String>,
    pub bits_per_component: Option<i64>,
    /// Original PDF stream filters (e.g. `["FlateDecode"]`, `["DCTDecode"]`).
    pub filters: Vec<String>,
    /// The decoded image bytes in a standard format (JPEG, PNG, etc.).
    pub data: Bytes,
    /// The format of `data` after decoding: `"jpeg"`, `"png"`, `"jpeg2000"`, `"ccitt"`, or `"raw"`.
    pub decoded_format: String,
}

#[derive(Debug)]
pub struct PdfImageExtractor {
    document: Document,
}

/// Decode raw PDF image stream bytes according to PDF filter(s).
///
/// Returns `(decoded_bytes, format_string)`.
///
/// | Filter         | Strategy                                                     | Format      |
/// |----------------|--------------------------------------------------------------|-------------|
/// | `DCTDecode`    | Raw bytes ARE JPEG — pass through                            | `"jpeg"`    |
/// | `FlateDecode`  | zlib-decompress raw pixels → re-encode as PNG                | `"png"`     |
/// | `JPXDecode`    | Raw bytes are JPEG 2000 — pass through                       | `"jpeg2000"`|
/// | `CCITTFaxDecode`| Pass through (bilevel fax — no full decode)                 | `"ccitt"`   |
/// | JBIG2Decode    | Pass through                                                 | `"jbig2"`   |
/// | unknown / none | Attempt format detection via magic bytes, else pass through  | detected or `"raw"` |
#[cfg(feature = "pdf")]
fn decode_image_data(
    raw: &[u8],
    filters: &[String],
    color_space: Option<&str>,
    width: i64,
    height: i64,
    bits_per_component: Option<i64>,
) -> (Bytes, String) {
    // Determine the primary filter (last applied is outermost / first to decode).
    // PDF applies filters in array order for encoding, so we decode in reverse.
    // For single-filter images (the common case) this is just filters[0].
    let primary = filters.first().map(String::as_str).unwrap_or("");

    match primary {
        "DCTDecode" => {
            // Content bytes are already a valid JPEG bitstream.
            (Bytes::from(raw.to_vec()), "jpeg".to_string())
        }
        "FlateDecode" => {
            // Content is zlib/deflate-compressed raw pixel data.
            // Decompress, then re-encode as PNG via the `image` crate.
            match decode_flate_to_png(raw, color_space, width, height, bits_per_component) {
                Ok(png_bytes) => (Bytes::from(png_bytes), "png".to_string()),
                Err(_) => {
                    // Fall back to raw bytes if decode fails.
                    (Bytes::from(raw.to_vec()), "raw".to_string())
                }
            }
        }
        "JPXDecode" => {
            // JPEG 2000 data — pass through as-is.
            (Bytes::from(raw.to_vec()), "jpeg2000".to_string())
        }
        "CCITTFaxDecode" => {
            // Bilevel fax encoding — pass through.
            (Bytes::from(raw.to_vec()), "ccitt".to_string())
        }
        "JBIG2Decode" => {
            // JBIG2 — pass through.
            (Bytes::from(raw.to_vec()), "jbig2".to_string())
        }
        _ => {
            // Unknown or absent filter — try to detect format from magic bytes.
            let format = detect_image_format(raw);
            (Bytes::from(raw.to_vec()), format)
        }
    }
}

/// Attempt to detect image format from magic bytes.
fn detect_image_format(data: &[u8]) -> String {
    if data.starts_with(b"\xff\xd8\xff") {
        "jpeg".to_string()
    } else if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        "png".to_string()
    } else if data.starts_with(b"GIF8") {
        "gif".to_string()
    } else if data.starts_with(b"II") || data.starts_with(b"MM") {
        "tiff".to_string()
    } else if data.starts_with(b"BM") {
        "bmp".to_string()
    } else {
        "raw".to_string()
    }
}

/// Decompress a FlateDecode (zlib) buffer, then re-encode the raw pixel data as PNG.
///
/// PDF FlateDecode image streams are zlib-deflated uncompressed pixel rows.  After
/// decompression the bytes are in `width × height × channels` layout (no row-padding
/// other than a possible PNG predictor byte per row — see PDF spec §7.4.4.4).
#[cfg(feature = "pdf")]
fn decode_flate_to_png(
    raw: &[u8],
    color_space: Option<&str>,
    width: i64,
    height: i64,
    bits_per_component: Option<i64>,
) -> std::result::Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    use flate2::read::ZlibDecoder;
    use image::ImageEncoder;
    use std::io::Read;

    // Decompress from zlib stream.
    let mut decoder = ZlibDecoder::new(raw);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;

    // Determine the number of color channels from the color space.
    let channels: u32 = match color_space {
        Some(cs) if cs.contains("RGB") => 3,
        Some(cs) if cs.contains("CMYK") => 4,
        Some(cs) if cs.contains("Gray") || cs.contains("grey") || cs.contains("grey") => 1,
        _ => 3, // Default to RGB
    };

    let bpc = bits_per_component.unwrap_or(8) as u32;
    let w = width as u32;
    let h = height as u32;

    // PDF FlateDecode pixel rows may carry a PNG predictor byte (value 2 = Sub,
    // etc. per PDF spec §7.4.4.4 / PNG predictor). Strip it if row-stride matches.
    let bytes_per_channel = (bpc + 7) / 8;
    let raw_row_stride = w * channels * bytes_per_channel; // bytes per row without predictor
    let pred_row_stride = raw_row_stride + 1; // +1 for predictor byte

    let pixel_data: Vec<u8> = if h > 0 && decompressed.len() as u32 == pred_row_stride * h {
        // Strip the predictor byte at the start of each row.
        let mut pixels = Vec::with_capacity((raw_row_stride * h) as usize);
        for row in 0..h {
            let row_start = (row * pred_row_stride) as usize;
            // Skip the predictor/filter byte (byte 0 of each row).
            pixels.extend_from_slice(&decompressed[row_start + 1..row_start + pred_row_stride as usize]);
        }
        pixels
    } else {
        decompressed
    };

    // Build an image::DynamicImage from the raw pixel data.
    let color_type = match (channels, bpc) {
        (1, 8) => image::ColorType::L8,
        (1, 16) => image::ColorType::L16,
        (3, 8) => image::ColorType::Rgb8,
        (3, 16) => image::ColorType::Rgb16,
        (4, 8) => image::ColorType::Rgba8,
        _ => image::ColorType::Rgb8,
    };

    // Encode to PNG in memory.
    let mut png_bytes: Vec<u8> = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut png_bytes);
    image::codecs::png::PngEncoder::new(&mut cursor)
        .write_image(&pixel_data, w, h, color_type.into())
        .map_err(|e| format!("PNG encoding failed: {e}"))?;

    Ok(png_bytes)
}

impl PdfImageExtractor {
    pub fn new(pdf_bytes: &[u8]) -> Result<Self> {
        Self::new_with_password(pdf_bytes, None)
    }

    pub fn new_with_password(pdf_bytes: &[u8], password: Option<&str>) -> Result<Self> {
        let mut doc =
            Document::load_mem(pdf_bytes).map_err(|e| PdfError::InvalidPdf(format!("Failed to load PDF: {}", e)))?;

        if doc.is_encrypted() {
            if let Some(pwd) = password {
                doc.decrypt(pwd).map_err(|_| PdfError::InvalidPassword)?;
            } else {
                return Err(PdfError::PasswordRequired);
            }
        }

        Ok(Self { document: doc })
    }

    pub fn extract_images(&self) -> Result<Vec<PdfImage>> {
        let mut all_images = Vec::new();
        let pages = self.document.get_pages();

        for (page_num, page_id) in pages.iter() {
            let images = self
                .document
                .get_page_images(*page_id)
                .map_err(|e| PdfError::MetadataExtractionFailed(format!("Failed to get page images: {}", e)))?;

            for (img_index, img) in images.iter().enumerate() {
                let filters = img.filters.clone().unwrap_or_default();

                #[cfg(feature = "pdf")]
                let (data, decoded_format) = decode_image_data(
                    img.content,
                    &filters,
                    img.color_space.as_deref(),
                    img.width,
                    img.height,
                    img.bits_per_component,
                );

                #[cfg(not(feature = "pdf"))]
                let (data, decoded_format) = (Bytes::from(img.content.to_vec()), "raw".to_string());

                all_images.push(PdfImage {
                    page_number: *page_num as usize,
                    image_index: img_index + 1,
                    width: img.width,
                    height: img.height,
                    color_space: img.color_space.clone(),
                    bits_per_component: img.bits_per_component,
                    filters,
                    data,
                    decoded_format,
                });
            }
        }

        Ok(all_images)
    }

    pub fn extract_images_from_page(&self, page_number: u32) -> Result<Vec<PdfImage>> {
        let pages = self.document.get_pages();
        let page_id = pages
            .get(&page_number)
            .ok_or(PdfError::PageNotFound(page_number as usize))?;

        let images = self
            .document
            .get_page_images(*page_id)
            .map_err(|e| PdfError::MetadataExtractionFailed(format!("Failed to get page images: {}", e)))?;

        let mut page_images = Vec::new();
        for (img_index, img) in images.iter().enumerate() {
            let filters = img.filters.clone().unwrap_or_default();

            #[cfg(feature = "pdf")]
            let (data, decoded_format) = decode_image_data(
                img.content,
                &filters,
                img.color_space.as_deref(),
                img.width,
                img.height,
                img.bits_per_component,
            );

            #[cfg(not(feature = "pdf"))]
            let (data, decoded_format) = (Bytes::from(img.content.to_vec()), "raw".to_string());

            page_images.push(PdfImage {
                page_number: page_number as usize,
                image_index: img_index + 1,
                width: img.width,
                height: img.height,
                color_space: img.color_space.clone(),
                bits_per_component: img.bits_per_component,
                filters,
                data,
                decoded_format,
            });
        }

        Ok(page_images)
    }

    pub fn get_image_count(&self) -> Result<usize> {
        let images = self.extract_images()?;
        Ok(images.len())
    }
}

pub fn extract_images_from_pdf(pdf_bytes: &[u8]) -> Result<Vec<PdfImage>> {
    let extractor = PdfImageExtractor::new(pdf_bytes)?;
    extractor.extract_images()
}

pub fn extract_images_from_pdf_with_password(pdf_bytes: &[u8], password: &str) -> Result<Vec<PdfImage>> {
    let extractor = PdfImageExtractor::new_with_password(pdf_bytes, Some(password))?;
    extractor.extract_images()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extractor_creation() {
        let result = PdfImageExtractor::new(b"not a pdf");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PdfError::InvalidPdf(_)));
    }

    #[test]
    fn test_extract_images_invalid_pdf() {
        let result = extract_images_from_pdf(b"not a pdf");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_images_empty_pdf() {
        let result = extract_images_from_pdf(b"");
        assert!(result.is_err());
    }

    #[test]
    fn test_detect_image_format_jpeg() {
        let jpeg_magic = b"\xff\xd8\xff\xe0some_jpeg_data";
        assert_eq!(detect_image_format(jpeg_magic), "jpeg");
    }

    #[test]
    fn test_detect_image_format_png() {
        let png_magic = b"\x89PNG\r\n\x1a\nsome_png_data";
        assert_eq!(detect_image_format(png_magic), "png");
    }

    #[test]
    fn test_detect_image_format_unknown() {
        assert_eq!(detect_image_format(b"\x00\x01\x02\x03"), "raw");
    }

    #[cfg(feature = "pdf")]
    #[test]
    fn test_decode_dct_passthrough() {
        let jpeg_bytes = b"\xff\xd8\xff\xe0fake_jpeg";
        let filters = vec!["DCTDecode".to_string()];
        let (data, format) = decode_image_data(jpeg_bytes, &filters, Some("DeviceRGB"), 100, 100, Some(8));
        assert_eq!(format, "jpeg");
        assert_eq!(data.as_ref(), jpeg_bytes);
    }

    #[cfg(feature = "pdf")]
    #[test]
    fn test_decode_jpx_passthrough() {
        let jpx_bytes = b"\x00\x00\x00\x0cjP  fake_jpx";
        let filters = vec!["JPXDecode".to_string()];
        let (data, format) = decode_image_data(jpx_bytes, &filters, Some("DeviceRGB"), 10, 10, Some(8));
        assert_eq!(format, "jpeg2000");
        assert_eq!(data.as_ref(), jpx_bytes);
    }

    #[cfg(feature = "pdf")]
    #[test]
    fn test_decode_unknown_filter_passthrough() {
        let raw_bytes = b"\x00\x01\x02\x03";
        let filters = vec!["RunLengthDecode".to_string()];
        let (data, format) = decode_image_data(raw_bytes, &filters, None, 2, 2, Some(8));
        assert_eq!(format, "raw");
        assert_eq!(data.as_ref(), raw_bytes);
    }

    #[cfg(feature = "pdf")]
    #[test]
    fn test_decode_no_filter_uses_detection() {
        let jpeg_bytes = b"\xff\xd8\xff\xe0fake";
        let filters: Vec<String> = vec![];
        let (data, format) = decode_image_data(jpeg_bytes, &filters, None, 10, 10, None);
        assert_eq!(format, "jpeg");
        assert_eq!(data.as_ref(), jpeg_bytes);
    }

    #[cfg(feature = "pdf")]
    #[test]
    fn test_decode_flate_valid_rgb_image() {
        use flate2::write::ZlibEncoder;
        use flate2::Compression;
        use std::io::Write;

        // Create a 2×2 RGB image: 4 pixels × 3 channels = 12 bytes raw.
        let raw_pixels: Vec<u8> = vec![
            255, 0, 0, // red
            0, 255, 0, // green
            0, 0, 255, // blue
            255, 255, 0, // yellow
        ];
        // Flatten into rows (width=2), wrapping each row with PNG predictor byte 0 (None).
        let row_stride = 2 * 3; // width * channels
        let mut rows_with_predictor: Vec<u8> = Vec::new();
        for row in 0..2usize {
            rows_with_predictor.push(0); // predictor byte
            rows_with_predictor.extend_from_slice(&raw_pixels[row * row_stride..(row + 1) * row_stride]);
        }

        // Compress with zlib.
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&rows_with_predictor).unwrap();
        let compressed = encoder.finish().unwrap();

        let filters = vec!["FlateDecode".to_string()];
        let (data, format) = decode_image_data(&compressed, &filters, Some("DeviceRGB"), 2, 2, Some(8));
        assert_eq!(format, "png", "FlateDecode images should be re-encoded as PNG");
        // PNG magic: \x89PNG\r\n\x1a\n
        assert!(
            data.starts_with(b"\x89PNG\r\n\x1a\n"),
            "Decoded data should be a valid PNG (got {} bytes, first bytes: {:?})",
            data.len(),
            &data[..data.len().min(8)]
        );
    }
}
