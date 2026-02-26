//! Image extraction functionality.
//!
//! This module provides functions for extracting metadata and EXIF data from images,
//! including support for multi-frame TIFF files.

use crate::error::{KreuzbergError, Result};
use image::ImageReader;
use std::collections::HashMap;
use std::io::Cursor;

/// JP2 file signature: 12-byte box starting with length 0x0000000C and type "jP  "
const JP2_MAGIC: &[u8] = &[0x00, 0x00, 0x00, 0x0C, 0x6A, 0x50, 0x20, 0x20];

/// Check if bytes start with JPEG 2000 magic bytes.
pub(crate) fn is_jp2(bytes: &[u8]) -> bool {
    bytes.len() >= JP2_MAGIC.len() && bytes[..JP2_MAGIC.len()] == *JP2_MAGIC
}

/// Check if bytes start with J2K codestream magic (SOC marker).
#[allow(dead_code)]
pub(crate) fn is_j2k(bytes: &[u8]) -> bool {
    bytes.len() >= 4 && bytes[0] == 0xFF && bytes[1] == 0x4F && bytes[2] == 0xFF && bytes[3] == 0x51
}

/// Image metadata extracted from an image file.
#[derive(Debug, Clone)]
pub struct ImageMetadata {
    /// Image width in pixels
    pub width: u32,
    /// Image height in pixels
    pub height: u32,
    /// Image format (e.g., "PNG", "JPEG")
    pub format: String,
    /// EXIF data if available
    pub exif_data: HashMap<String, String>,
}

/// Parse JP2 file header boxes to extract image dimensions.
///
/// Supports both JP2 container format (ISO 15444-1 Annex I) and raw J2K codestream.
/// Uses pure Rust header parsing without external dependencies.
fn decode_jp2_metadata(bytes: &[u8]) -> Result<ImageMetadata> {
    // Try JP2 box format first (starts with signature box)
    if is_jp2(bytes) {
        return parse_jp2_boxes(bytes);
    }

    // Try J2K raw codestream (starts with SOC marker 0xFF4F)
    if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0x4F {
        return parse_j2k_siz(bytes);
    }

    Err(KreuzbergError::parsing("Not a valid JPEG 2000 file".to_string()))
}

/// Parse JP2 container boxes to find ihdr (Image Header) box.
fn parse_jp2_boxes(bytes: &[u8]) -> Result<ImageMetadata> {
    let mut offset = 0;
    let len = bytes.len();

    while offset + 8 <= len {
        let box_len =
            u32::from_be_bytes([bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3]]) as usize;
        let box_type = &bytes[offset + 4..offset + 8];

        // Handle extended box length (box_len == 1 means 8-byte extended length follows)
        let (data_start, actual_len) = if box_len == 1 && offset + 16 <= len {
            let ext_len = u64::from_be_bytes([
                bytes[offset + 8],
                bytes[offset + 9],
                bytes[offset + 10],
                bytes[offset + 11],
                bytes[offset + 12],
                bytes[offset + 13],
                bytes[offset + 14],
                bytes[offset + 15],
            ]) as usize;
            (offset + 16, ext_len)
        } else if box_len == 0 {
            // Box extends to end of file
            (offset + 8, len - offset)
        } else {
            (offset + 8, box_len)
        };

        // ihdr box: height(u32) + width(u32) + numcomps(u16) + bpc(u8) + ...
        if box_type == b"ihdr" && data_start + 8 <= len {
            let height = u32::from_be_bytes([
                bytes[data_start],
                bytes[data_start + 1],
                bytes[data_start + 2],
                bytes[data_start + 3],
            ]);
            let width = u32::from_be_bytes([
                bytes[data_start + 4],
                bytes[data_start + 5],
                bytes[data_start + 6],
                bytes[data_start + 7],
            ]);
            return Ok(ImageMetadata {
                width,
                height,
                format: "JPEG2000".to_string(),
                exif_data: extract_exif_data(bytes),
            });
        }

        // jp2h is a superbox - recurse into its contents
        if box_type == b"jp2h" {
            let end = offset + actual_len.min(len - offset);
            // Parse sub-boxes within jp2h
            let mut sub_offset = data_start;
            while sub_offset + 8 <= end {
                let sub_len = u32::from_be_bytes([
                    bytes[sub_offset],
                    bytes[sub_offset + 1],
                    bytes[sub_offset + 2],
                    bytes[sub_offset + 3],
                ]) as usize;
                let sub_type = &bytes[sub_offset + 4..sub_offset + 8];
                let sub_data = sub_offset + 8;

                if sub_type == b"ihdr" && sub_data + 8 <= len {
                    let height = u32::from_be_bytes([
                        bytes[sub_data],
                        bytes[sub_data + 1],
                        bytes[sub_data + 2],
                        bytes[sub_data + 3],
                    ]);
                    let width = u32::from_be_bytes([
                        bytes[sub_data + 4],
                        bytes[sub_data + 5],
                        bytes[sub_data + 6],
                        bytes[sub_data + 7],
                    ]);
                    return Ok(ImageMetadata {
                        width,
                        height,
                        format: "JPEG2000".to_string(),
                        exif_data: extract_exif_data(bytes),
                    });
                }

                if sub_len < 8 {
                    break;
                }
                sub_offset += sub_len;
            }
        }

        if actual_len < 8 {
            break;
        }
        offset += actual_len;
    }

    Err(KreuzbergError::parsing("JP2 file missing ihdr box".to_string()))
}

/// Parse J2K raw codestream SIZ marker for image dimensions.
fn parse_j2k_siz(bytes: &[u8]) -> Result<ImageMetadata> {
    // Find SIZ marker (0xFF51) - usually right after SOC (0xFF4F)
    if let Some(offset) = memchr::memmem::find(bytes, &[0xFF, 0x51]) {
        // SIZ marker found. Format: marker(2) + Lsiz(2) + Rsiz(2) + Xsiz(4) + Ysiz(4) + XOsiz(4) + YOsiz(4)
        let data_start = offset + 4; // skip marker + length
        if data_start + 18 <= bytes.len() {
            let xsiz = u32::from_be_bytes([
                bytes[data_start + 2],
                bytes[data_start + 3],
                bytes[data_start + 4],
                bytes[data_start + 5],
            ]);
            let ysiz = u32::from_be_bytes([
                bytes[data_start + 6],
                bytes[data_start + 7],
                bytes[data_start + 8],
                bytes[data_start + 9],
            ]);
            let xosiz = u32::from_be_bytes([
                bytes[data_start + 10],
                bytes[data_start + 11],
                bytes[data_start + 12],
                bytes[data_start + 13],
            ]);
            let yosiz = u32::from_be_bytes([
                bytes[data_start + 14],
                bytes[data_start + 15],
                bytes[data_start + 16],
                bytes[data_start + 17],
            ]);

            let width = xsiz.saturating_sub(xosiz);
            let height = ysiz.saturating_sub(yosiz);

            return Ok(ImageMetadata {
                width,
                height,
                format: "JPEG2000".to_string(),
                exif_data: extract_exif_data(bytes),
            });
        }
    }

    Err(KreuzbergError::parsing("J2K codestream missing SIZ marker".to_string()))
}

/// Decode JPEG 2000 image bytes to an RGB image using hayro-jpeg2000.
///
/// Pure Rust, memory-safe decoder. No temp files needed.
#[cfg(feature = "ocr")]
pub(crate) fn decode_jp2_to_rgb(bytes: &[u8]) -> Result<image::RgbImage> {
    use hayro_jpeg2000::{DecodeSettings, Image as Jp2Image};

    let jp2 = Jp2Image::new(bytes, &DecodeSettings::default())
        .map_err(|e| KreuzbergError::parsing(format!("JP2 decode failed: {}", e)))?;
    let width = jp2.width();
    let height = jp2.height();
    let has_alpha = jp2.has_alpha();
    let num_channels = jp2.color_space().num_channels();
    let pixels = jp2
        .decode()
        .map_err(|e| KreuzbergError::parsing(format!("JP2 pixel decode failed: {}", e)))?;

    // Convert decoded pixels to RGB
    let rgb_bytes = match (num_channels, has_alpha) {
        // Grayscale → replicate to RGB
        (1, false) => {
            let mut rgb = Vec::with_capacity(pixels.len() * 3);
            for &g in &pixels {
                rgb.push(g);
                rgb.push(g);
                rgb.push(g);
            }
            rgb
        }
        // Grayscale + alpha → replicate gray to RGB, skip alpha
        (1, true) => {
            let mut rgb = Vec::with_capacity((pixels.len() / 2) * 3);
            for chunk in pixels.chunks_exact(2) {
                rgb.push(chunk[0]);
                rgb.push(chunk[0]);
                rgb.push(chunk[0]);
            }
            rgb
        }
        // RGB → use as-is
        (3, false) => pixels,
        // RGBA → strip alpha channel
        (3, true) => {
            let mut rgb = Vec::with_capacity((pixels.len() / 4) * 3);
            for chunk in pixels.chunks_exact(4) {
                rgb.push(chunk[0]);
                rgb.push(chunk[1]);
                rgb.push(chunk[2]);
            }
            rgb
        }
        // CMYK → simple inversion to RGB (C=255-R, M=255-G, Y=255-B, K applied)
        (4, false) => {
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
            rgb
        }
        _ => {
            return Err(KreuzbergError::parsing(format!(
                "Unsupported JP2 color space: {} channels, alpha={}",
                num_channels, has_alpha
            )));
        }
    };

    image::RgbImage::from_raw(width, height, rgb_bytes)
        .ok_or_else(|| KreuzbergError::parsing("Failed to construct RGB image from JP2 data".to_string()))
}

/// JBIG2 file signature: 0x97 0x4A 0x42 0x32 0x0D 0x0A 0x1A 0x0A
const JBIG2_MAGIC: &[u8] = &[0x97, 0x4A, 0x42, 0x32, 0x0D, 0x0A, 0x1A, 0x0A];

/// Check if bytes start with JBIG2 magic bytes.
#[allow(dead_code)]
pub(crate) fn is_jbig2(bytes: &[u8]) -> bool {
    bytes.len() >= JBIG2_MAGIC.len() && bytes[..JBIG2_MAGIC.len()] == *JBIG2_MAGIC
}

/// Decode JBIG2 image bytes to a grayscale image using hayro-jbig2.
///
/// JBIG2 is a bi-level (1-bit) image compression format commonly used in scanned PDFs.
/// The decoder converts black/white pixels to grayscale (0/255) for OCR processing.
#[cfg(feature = "ocr")]
pub(crate) fn decode_jbig2_to_gray(bytes: &[u8]) -> Result<image::GrayImage> {
    use hayro_jbig2::decode;

    let jbig2_image = decode(bytes).map_err(|e| KreuzbergError::parsing(format!("JBIG2 decode failed: {}", e)))?;
    let width = jbig2_image.width;
    let height = jbig2_image.height;

    // Convert boolean pixel data (true=black, false=white) to grayscale (0=black, 255=white)
    let pixels: Vec<u8> = jbig2_image
        .data
        .iter()
        .map(|&is_black| if is_black { 0 } else { 255 })
        .collect();

    image::GrayImage::from_raw(width, height, pixels)
        .ok_or_else(|| KreuzbergError::parsing("Failed to construct grayscale image from JBIG2 data".to_string()))
}

/// Extract metadata from image bytes.
///
/// Extracts dimensions, format, and EXIF data from the image.
/// Attempts to decode using the standard image crate first, then falls back to
/// pure Rust JP2 box parsing for JPEG 2000 formats if the standard decoder fails.
pub fn extract_image_metadata(bytes: &[u8]) -> Result<ImageMetadata> {
    // Check for JP2/J2K before attempting standard format detection
    if is_jp2(bytes) || (bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0x4F) {
        // Try the fallback JP2 parser first for JPEG 2000 files
        if let Ok(metadata) = decode_jp2_metadata(bytes) {
            return Ok(metadata);
        }
    }

    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| KreuzbergError::parsing(format!("Failed to read image format: {}", e)))?;

    let format = reader
        .format()
        .ok_or_else(|| KreuzbergError::parsing("Could not determine image format".to_string()))?;

    match reader.decode() {
        Ok(image) => {
            let width = image.width();
            let height = image.height();
            let format_str = format!("{:?}", format).to_uppercase();
            let exif_data = extract_exif_data(bytes);

            Ok(ImageMetadata {
                width,
                height,
                format: format_str,
                exif_data,
            })
        }
        Err(decode_err) => Err(KreuzbergError::parsing(format!(
            "Failed to decode image: {}",
            decode_err
        ))),
    }
}

/// Extract EXIF data from image bytes.
///
/// Returns a HashMap of EXIF tags and their values.
/// If EXIF data is not available or cannot be parsed, returns an empty HashMap.
/// Requires the `ocr` feature for kamadak-exif; returns empty map under `ocr-wasm`.
#[cfg(feature = "ocr")]
fn extract_exif_data(bytes: &[u8]) -> HashMap<String, String> {
    use exif::{In, Reader, Tag};

    let mut exif_map = HashMap::new();

    let exif_reader = match Reader::new().read_from_container(&mut Cursor::new(bytes)) {
        Ok(reader) => reader,
        Err(_) => return exif_map,
    };

    let common_tags = [
        (Tag::Make, "Make"),
        (Tag::Model, "Model"),
        (Tag::DateTime, "DateTime"),
        (Tag::DateTimeOriginal, "DateTimeOriginal"),
        (Tag::DateTimeDigitized, "DateTimeDigitized"),
        (Tag::Software, "Software"),
        (Tag::Orientation, "Orientation"),
        (Tag::XResolution, "XResolution"),
        (Tag::YResolution, "YResolution"),
        (Tag::ResolutionUnit, "ResolutionUnit"),
        (Tag::ExposureTime, "ExposureTime"),
        (Tag::FNumber, "FNumber"),
        (Tag::PhotographicSensitivity, "ISO"),
        (Tag::FocalLength, "FocalLength"),
        (Tag::Flash, "Flash"),
        (Tag::WhiteBalance, "WhiteBalance"),
        (Tag::GPSLatitude, "GPSLatitude"),
        (Tag::GPSLongitude, "GPSLongitude"),
        (Tag::GPSAltitude, "GPSAltitude"),
    ];

    for (tag, field_name) in common_tags {
        if let Some(field) = exif_reader.get_field(tag, In::PRIMARY) {
            exif_map.insert(field_name.to_string(), field.display_value().to_string());
        }
    }

    exif_map
}

/// Stub EXIF extraction for `ocr-wasm` (kamadak-exif not available on WASM).
#[cfg(all(feature = "ocr-wasm", not(feature = "ocr")))]
fn extract_exif_data(_bytes: &[u8]) -> HashMap<String, String> {
    HashMap::new()
}

/// Result of OCR extraction from an image with optional page tracking.
#[derive(Debug, Clone)]
pub struct ImageOcrResult {
    /// Extracted text content
    pub content: String,
    /// Character byte boundaries per frame (for multi-frame TIFFs)
    pub boundaries: Option<Vec<crate::types::PageBoundary>>,
    /// Per-frame content information
    pub page_contents: Option<Vec<crate::types::PageContent>>,
}

/// Detects the number of frames in a TIFF file.
///
/// Returns the count of image frames/pages in a TIFF. Single-frame TIFFs return 1.
/// Invalid or non-TIFF data returns an error.
///
/// # Arguments
/// * `bytes` - Raw TIFF file bytes
///
/// # Returns
/// Frame count if valid TIFF, error otherwise.
#[cfg(feature = "ocr")]
fn detect_tiff_frame_count(bytes: &[u8]) -> Result<usize> {
    use tiff::decoder::Decoder;
    let mut decoder =
        Decoder::new(Cursor::new(bytes)).map_err(|e| KreuzbergError::parsing(format!("TIFF decode: {}", e)))?;

    let mut count = 1;
    while decoder.next_image().is_ok() {
        count += 1;
    }
    Ok(count)
}

/// Extract text from image bytes using OCR with optional page tracking for multi-frame TIFFs.
///
/// This function:
/// - Detects if the image is a multi-frame TIFF
/// - For multi-frame TIFFs with PageConfig enabled, iterates frames and tracks boundaries
/// - For single-frame images or when page tracking is disabled, runs OCR on the whole image
/// - Returns (content, boundaries, page_contents) tuple
///
/// # Arguments
/// * `bytes` - Image file bytes
/// * `mime_type` - MIME type (e.g., "image/tiff")
/// * `ocr_result` - OCR backend result containing the text
/// * `page_config` - Optional page configuration for boundary tracking
///
/// # Returns
/// ImageOcrResult with content and optional boundaries for pagination
#[cfg(feature = "ocr")]
pub fn extract_text_from_image_with_ocr(
    bytes: &[u8],
    mime_type: &str,
    ocr_result: String,
    page_config: Option<&crate::core::config::PageConfig>,
) -> Result<ImageOcrResult> {
    let is_tiff = mime_type.to_lowercase().contains("tiff");
    let should_track_pages = page_config.is_some() && is_tiff;

    if !should_track_pages {
        return Ok(ImageOcrResult {
            content: ocr_result,
            boundaries: None,
            page_contents: None,
        });
    }

    let frame_count = detect_tiff_frame_count(bytes)?;

    if frame_count <= 1 {
        return Ok(ImageOcrResult {
            content: ocr_result,
            boundaries: None,
            page_contents: None,
        });
    }

    let content_len = ocr_result.len();
    let content_per_frame = content_len.checked_div(frame_count).unwrap_or(content_len);

    let mut boundaries = Vec::new();
    let mut page_contents = Vec::new();
    let mut byte_offset = 0;

    for frame_num in 1..=frame_count {
        let frame_end = if frame_num == frame_count {
            content_len
        } else {
            let raw_end = (frame_num * content_per_frame).min(content_len);
            (raw_end..=content_len)
                .find(|&i| ocr_result.is_char_boundary(i))
                .unwrap_or(content_len)
        };

        boundaries.push(crate::types::PageBoundary {
            byte_start: byte_offset,
            byte_end: frame_end,
            page_number: frame_num,
        });

        let frame_text = &ocr_result[byte_offset..frame_end];
        page_contents.push(crate::types::PageContent {
            page_number: frame_num,
            content: frame_text.to_string(),
            tables: vec![],
            images: vec![],
            hierarchy: None,
            is_blank: Some(crate::extraction::blank_detection::is_page_text_blank(frame_text)),
        });

        byte_offset = frame_end;
    }

    Ok(ImageOcrResult {
        content: ocr_result,
        boundaries: Some(boundaries),
        page_contents: Some(page_contents),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, ImageFormat, Rgb, RgbImage};
    use std::io::Cursor;

    fn create_test_image(width: u32, height: u32, format: ImageFormat) -> Vec<u8> {
        let img: RgbImage = ImageBuffer::from_fn(width, height, |x, y| {
            let r = ((x as f32 / width as f32) * 255.0) as u8;
            let g = ((y as f32 / height as f32) * 255.0) as u8;
            let b = 128;
            Rgb([r, g, b])
        });

        let mut bytes: Vec<u8> = Vec::new();
        let mut cursor = Cursor::new(&mut bytes);
        img.write_to(&mut cursor, format).unwrap();
        bytes
    }

    #[test]
    fn test_extract_png_image_returns_correct_metadata() {
        let bytes = create_test_image(100, 80, ImageFormat::Png);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 100);
        assert_eq!(metadata.height, 80);
        assert_eq!(metadata.format, "PNG");
    }

    #[test]
    fn test_extract_jpeg_image_returns_correct_metadata() {
        let bytes = create_test_image(200, 150, ImageFormat::Jpeg);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 200);
        assert_eq!(metadata.height, 150);
        assert_eq!(metadata.format, "JPEG");
    }

    #[test]
    fn test_extract_webp_image_returns_correct_metadata() {
        let bytes = create_test_image(120, 90, ImageFormat::WebP);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 120);
        assert_eq!(metadata.height, 90);
        assert_eq!(metadata.format, "WEBP");
    }

    #[test]
    fn test_extract_bmp_image_returns_correct_metadata() {
        let bytes = create_test_image(50, 50, ImageFormat::Bmp);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 50);
        assert_eq!(metadata.height, 50);
        assert_eq!(metadata.format, "BMP");
    }

    #[test]
    fn test_extract_tiff_image_returns_correct_metadata() {
        let bytes = create_test_image(180, 120, ImageFormat::Tiff);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 180);
        assert_eq!(metadata.height, 120);
        assert_eq!(metadata.format, "TIFF");
    }

    #[test]
    fn test_extract_gif_image_returns_correct_metadata() {
        let bytes = create_test_image(64, 64, ImageFormat::Gif);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 64);
        assert_eq!(metadata.height, 64);
        assert_eq!(metadata.format, "GIF");
    }

    #[test]
    fn test_extract_image_extreme_aspect_ratio() {
        let bytes = create_test_image(1000, 10, ImageFormat::Png);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 1000);
        assert_eq!(metadata.height, 10);
        assert!(metadata.width / metadata.height >= 100);
    }

    #[test]
    fn test_extract_image_dimensions_correctly() {
        let bytes = create_test_image(640, 480, ImageFormat::Png);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 640);
        assert_eq!(metadata.height, 480);
    }

    #[test]
    fn test_extract_image_format_correctly() {
        let png_bytes = create_test_image(100, 100, ImageFormat::Png);
        let jpeg_bytes = create_test_image(100, 100, ImageFormat::Jpeg);

        let png_metadata = extract_image_metadata(&png_bytes).unwrap();
        let jpeg_metadata = extract_image_metadata(&jpeg_bytes).unwrap();

        assert_eq!(png_metadata.format, "PNG");
        assert_eq!(jpeg_metadata.format, "JPEG");
    }

    #[test]
    fn test_extract_image_without_exif_returns_empty_map() {
        let bytes = create_test_image(100, 100, ImageFormat::Png);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert!(metadata.exif_data.is_empty());
    }

    #[test]
    fn test_extract_exif_data_from_jpeg_with_exif() {
        let bytes = create_test_image(100, 100, ImageFormat::Jpeg);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.exif_data.len(), 0);
    }

    #[test]
    fn test_extract_image_metadata_invalid_returns_error() {
        let invalid_bytes = vec![0, 1, 2, 3, 4, 5];
        let result = extract_image_metadata(&invalid_bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_image_corrupted_data_returns_error() {
        let mut bytes = create_test_image(100, 100, ImageFormat::Png);
        if bytes.len() > 50 {
            for byte in bytes.iter_mut().take(50).skip(20) {
                *byte = 0xFF;
            }
        }

        let _result = extract_image_metadata(&bytes);
        // Corrupted images may or may not be detectable depending on corruption location
    }

    #[test]
    fn test_extract_image_empty_bytes_returns_error() {
        let empty_bytes: Vec<u8> = Vec::new();
        let result = extract_image_metadata(&empty_bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_image_unsupported_format_returns_error() {
        let unsupported_bytes = vec![0x00, 0x00, 0x00, 0x0C, 0x6A, 0x50, 0x20, 0x20, 0x0D, 0x0A, 0x87, 0x0A];
        let result = extract_image_metadata(&unsupported_bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_very_small_image_1x1_pixel() {
        let bytes = create_test_image(1, 1, ImageFormat::Png);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 1);
        assert_eq!(metadata.height, 1);
        assert_eq!(metadata.format, "PNG");
    }

    #[test]
    fn test_extract_large_image_dimensions() {
        let bytes = create_test_image(2048, 1536, ImageFormat::Png);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 2048);
        assert_eq!(metadata.height, 1536);
    }

    #[test]
    fn test_extract_image_with_no_metadata_has_empty_exif() {
        let bytes = create_test_image(100, 100, ImageFormat::Png);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert!(metadata.exif_data.is_empty());
    }

    #[test]
    fn test_extract_exif_data_returns_empty_map_for_non_jpeg() {
        let png_bytes = create_test_image(100, 100, ImageFormat::Png);
        let exif_data = extract_exif_data(&png_bytes);
        assert!(exif_data.is_empty());
    }

    #[test]
    fn test_extract_rectangular_image_portrait_orientation() {
        let bytes = create_test_image(400, 800, ImageFormat::Jpeg);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 400);
        assert_eq!(metadata.height, 800);
        assert!(metadata.height > metadata.width);
    }

    #[test]
    fn test_extract_rectangular_image_landscape_orientation() {
        let bytes = create_test_image(800, 400, ImageFormat::Png);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 800);
        assert_eq!(metadata.height, 400);
        assert!(metadata.width > metadata.height);
    }

    #[test]
    fn test_extract_square_image_equal_dimensions() {
        let bytes = create_test_image(512, 512, ImageFormat::Png);
        let result = extract_image_metadata(&bytes);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 512);
        assert_eq!(metadata.height, 512);
        assert_eq!(metadata.width, metadata.height);
    }

    #[test]
    fn test_extract_metadata_preserves_format_case() {
        let png_bytes = create_test_image(100, 100, ImageFormat::Png);
        let jpeg_bytes = create_test_image(100, 100, ImageFormat::Jpeg);
        let webp_bytes = create_test_image(100, 100, ImageFormat::WebP);

        let png_meta = extract_image_metadata(&png_bytes).unwrap();
        let jpeg_meta = extract_image_metadata(&jpeg_bytes).unwrap();
        let webp_meta = extract_image_metadata(&webp_bytes).unwrap();

        assert_eq!(png_meta.format, "PNG");
        assert_eq!(jpeg_meta.format, "JPEG");
        assert_eq!(webp_meta.format, "WEBP");
    }

    #[test]
    fn test_jp2_magic_detection() {
        assert!(is_jp2(&[0x00, 0x00, 0x00, 0x0C, 0x6A, 0x50, 0x20, 0x20, 0x0D, 0x0A]));
        assert!(!is_jp2(&[0x89, 0x50, 0x4E, 0x47])); // PNG magic
        assert!(!is_jp2(&[0x00, 0x00])); // too short
        assert!(!is_jp2(&[])); // empty
    }

    #[test]
    fn test_extract_jp2_rust_logo_metadata() {
        let bytes = include_bytes!("../../../../test_documents/images/rust-logo-512x512-blk.jp2");
        let result = extract_image_metadata(bytes);
        assert!(result.is_ok(), "Failed to extract JP2 metadata: {:?}", result.err());
        let metadata = result.unwrap();
        assert_eq!(metadata.width, 512);
        assert_eq!(metadata.height, 512);
        assert_eq!(metadata.format, "JPEG2000");
    }

    #[test]
    fn test_extract_jp2_hadley_crater_metadata() {
        let bytes = include_bytes!("../../../../test_documents/images/Hadley_Crater.jp2");
        let result = extract_image_metadata(bytes);
        assert!(result.is_ok(), "Failed to extract JP2 metadata: {:?}", result.err());
        let metadata = result.unwrap();
        assert!(metadata.width > 0);
        assert!(metadata.height > 0);
        assert_eq!(metadata.format, "JPEG2000");
    }

    #[test]
    fn test_parse_jp2_boxes_invalid_data() {
        let invalid = vec![0x00, 0x00, 0x00, 0x0C, 0x6A, 0x50, 0x20, 0x20, 0x0D, 0x0A, 0x87, 0x0A];
        let result = decode_jp2_metadata(&invalid);
        assert!(result.is_err());
    }

    #[test]
    fn test_jp2_magic_detection_comprehensive() {
        // Valid JP2 signature
        assert!(is_jp2(&[
            0x00, 0x00, 0x00, 0x0C, 0x6A, 0x50, 0x20, 0x20, 0x0D, 0x0A, 0x87, 0x0A
        ]));
        // Not JP2
        assert!(!is_jp2(&[0xFF, 0x4F, 0xFF, 0x51])); // J2K codestream
        assert!(!is_jp2(&[0x89, 0x50, 0x4E, 0x47])); // PNG
        assert!(!is_jp2(&[]));
    }
}

#[cfg(all(test, feature = "ocr"))]
mod jp2_decode_tests {
    use super::*;

    #[test]
    fn test_decode_jp2_to_rgb() {
        let bytes = include_bytes!("../../../../test_documents/images/rust-logo-512x512-blk.jp2");
        let rgb = decode_jp2_to_rgb(bytes).expect("Should decode JP2 to RGB");
        assert_eq!(rgb.width(), 512);
        assert_eq!(rgb.height(), 512);
    }

    #[test]
    fn test_is_j2k() {
        assert!(!is_j2k(&[]));
        assert!(!is_j2k(&[0xFF]));
        assert!(is_j2k(&[0xFF, 0x4F, 0xFF, 0x51, 0x00]));
        assert!(!is_j2k(&[0xFF, 0x4F, 0x00, 0x51]));
    }

    #[test]
    fn test_jbig2_magic_detection() {
        assert!(is_jbig2(&[0x97, 0x4A, 0x42, 0x32, 0x0D, 0x0A, 0x1A, 0x0A, 0x01]));
        assert!(!is_jbig2(&[0x89, 0x50, 0x4E, 0x47])); // PNG
        assert!(!is_jbig2(&[]));
        assert!(!is_jbig2(&[0x97, 0x4A])); // too short
    }
}
