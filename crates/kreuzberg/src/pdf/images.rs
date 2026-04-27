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
    pub(crate) document: Document,
    pub(crate) max_images_per_page: Option<u32>,
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
#[allow(clippy::too_many_arguments)]
fn decode_image_data(
    raw: &[u8],
    filters: &[String],
    color_space: Option<&str>,
    width: i64,
    height: i64,
    bits_per_component: Option<i64>,
    palette: Option<&[u8]>,
    palette_base_channels: u32,
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
            match decode_flate_to_png(
                raw,
                color_space,
                width,
                height,
                bits_per_component,
                palette,
                palette_base_channels,
            ) {
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
///
/// For Indexed (palette-based) color spaces, each pixel byte is a palette index.
/// If `palette` is provided, indices are expanded to RGB (or the base color space)
/// before PNG encoding. Otherwise, indices are treated as grayscale values.
#[cfg(feature = "pdf")]
fn decode_flate_to_png(
    raw: &[u8],
    color_space: Option<&str>,
    width: i64,
    height: i64,
    bits_per_component: Option<i64>,
    palette: Option<&[u8]>,
    palette_base_channels: u32,
) -> std::result::Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    use flate2::read::ZlibDecoder;
    use image::ImageEncoder;
    use std::io::Read;

    // Decompress from zlib stream.
    let mut decoder = ZlibDecoder::new(raw);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;

    let is_indexed = color_space.map(|cs| cs.contains("Indexed")).unwrap_or(false);

    // Determine the number of color channels from the color space.
    // For Indexed images, each pixel is 1 byte (a palette index).
    let index_channels: u32 = if is_indexed {
        1
    } else {
        match color_space {
            Some(cs) if cs.contains("RGB") => 3,
            Some(cs) if cs.contains("CMYK") => 4,
            Some(cs) if cs.contains("Gray") || cs.contains("grey") => 1,
            _ => 3, // Default to RGB
        }
    };

    let bpc = bits_per_component.unwrap_or(8) as u32;
    let w = width as u32;
    let h = height as u32;

    // PDF FlateDecode pixel rows may carry a PNG predictor byte (value 2 = Sub,
    // etc. per PDF spec §7.4.4.4 / PNG predictor). Strip it if row-stride matches.
    let bytes_per_channel = bpc.div_ceil(8);
    let raw_row_stride = w * index_channels * bytes_per_channel; // bytes per row without predictor
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

    // For indexed color spaces, expand palette indices to actual pixel values.
    if is_indexed {
        if let Some(palette_data) = palette {
            // Determine the base color space channels (typically 3 for RGB, 4 for CMYK, 1 for Gray).
            let base_ch = if palette_base_channels > 0 {
                palette_base_channels
            } else {
                3 // Default to RGB base
            };

            // Expand each index byte to its palette RGB/Gray/CMYK color.
            let mut expanded = Vec::with_capacity(pixel_data.len() * base_ch as usize);
            for &idx in &pixel_data {
                let offset = idx as usize * base_ch as usize;
                if offset + base_ch as usize <= palette_data.len() {
                    expanded.extend_from_slice(&palette_data[offset..offset + base_ch as usize]);
                } else {
                    // Out-of-range index: fill with zeros (black).
                    expanded.extend(std::iter::repeat_n(0u8, base_ch as usize));
                }
            }

            let color_type = match base_ch {
                1 => image::ColorType::L8,
                3 => image::ColorType::Rgb8,
                4 => image::ColorType::Rgba8,
                _ => image::ColorType::Rgb8,
            };

            let expected_len = (w * h * base_ch) as usize;
            if expanded.len() != expected_len {
                return Err(format!(
                    "PDF indexed image buffer length mismatch after palette expansion: \
                     expected {expected_len} bytes ({w}x{h} px, {base_ch} ch) but got {} bytes",
                    expanded.len()
                )
                .into());
            }

            let mut png_bytes: Vec<u8> = Vec::new();
            let mut cursor = std::io::Cursor::new(&mut png_bytes);
            image::codecs::png::PngEncoder::new(&mut cursor)
                .write_image(&expanded, w, h, color_type.into())
                .map_err(|e| format!("PNG encoding failed for indexed image: {e}"))?;

            return Ok(png_bytes);
        }

        // No palette available: treat indices as grayscale values.
        let expected_len = (w * h) as usize;
        if pixel_data.len() != expected_len {
            return Err(format!(
                "PDF indexed image buffer length mismatch (grayscale fallback): \
                 expected {expected_len} bytes ({w}x{h} px, 1 ch) but got {} bytes",
                pixel_data.len()
            )
            .into());
        }

        let mut png_bytes: Vec<u8> = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut png_bytes);
        image::codecs::png::PngEncoder::new(&mut cursor)
            .write_image(&pixel_data, w, h, image::ColorType::L8.into())
            .map_err(|e| format!("PNG encoding failed for indexed image (grayscale fallback): {e}"))?;

        return Ok(png_bytes);
    }

    // Non-indexed path: use channels directly.
    let channels = index_channels;

    // Build an image::DynamicImage from the raw pixel data.
    let color_type = match (channels, bpc) {
        (1, 8) => image::ColorType::L8,
        (1, 16) => image::ColorType::L16,
        (3, 8) => image::ColorType::Rgb8,
        (3, 16) => image::ColorType::Rgb16,
        (4, 8) => image::ColorType::Rgba8,
        _ => image::ColorType::Rgb8,
    };

    // Validate buffer length before encoding to prevent a panic inside the image
    // crate, which asserts `data.len() == width * height * channels * bytes_per_channel`.
    // Malformed PDF image streams can produce buffers that violate this (bug #552).
    let expected_len = (w * h * channels * bytes_per_channel) as usize;
    if pixel_data.len() != expected_len {
        return Err(format!(
            "PDF image buffer length mismatch: expected {expected_len} bytes \
             ({w}x{h} px, {channels} ch) but got {} bytes — skipping malformed image",
            pixel_data.len()
        )
        .into());
    }

    // Encode to PNG in memory.
    let mut png_bytes: Vec<u8> = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut png_bytes);
    image::codecs::png::PngEncoder::new(&mut cursor)
        .write_image(&pixel_data, w, h, color_type.into())
        .map_err(|e| format!("PNG encoding failed: {e}"))?;

    Ok(png_bytes)
}

/// Extract the palette (lookup table) and base color space channel count from an
/// Indexed color space array in a PDF image dictionary.
///
/// PDF Indexed color spaces are arrays: `[/Indexed base hival lookup]`
/// where `base` is the base color space (e.g. `/DeviceRGB`), `hival` is the max
/// index, and `lookup` is the palette data (inline string or stream reference).
///
/// Returns `(palette_bytes, base_channels)` if extraction succeeds.
#[cfg(feature = "pdf")]
fn extract_indexed_palette(dict: &lopdf::Dictionary, document: &Document) -> Option<(Vec<u8>, u32)> {
    use lopdf::Object;

    let cs_obj = dict.get(b"ColorSpace").ok()?;
    let array = match cs_obj {
        Object::Array(arr) => arr,
        _ => return None,
    };

    // Must be [/Indexed base hival lookup] — at least 4 elements.
    if array.len() < 4 {
        return None;
    }

    // Verify the first element is "Indexed".
    let name = array[0].as_name().ok()?;
    if name != b"Indexed" {
        return None;
    }

    // Determine base color space channel count from the second element.
    let base_channels = match &array[1] {
        Object::Name(name) => {
            let name_str = String::from_utf8_lossy(name);
            if name_str.contains("RGB") {
                3u32
            } else if name_str.contains("CMYK") {
                4
            } else if name_str.contains("Gray") || name_str.contains("grey") {
                1
            } else {
                3 // Default assumption
            }
        }
        Object::Array(base_arr) => {
            // Could be [/ICCBased <stream>] or similar — try the first name.
            if let Some(first) = base_arr.first() {
                let name_str = String::from_utf8_lossy(first.as_name().unwrap_or(b""));
                if name_str.contains("ICCBased") {
                    // ICCBased color spaces typically reference an ICC profile stream
                    // with an /N entry specifying the number of components.
                    if let Some(stream_ref) = base_arr.get(1)
                        && let Ok(obj_id) = stream_ref.as_reference()
                        && let Ok(obj) = document.get_object(obj_id)
                        && let Ok(stream) = obj.as_stream()
                        && let Ok(n) = stream.dict.get(b"N")
                        && let Ok(n_val) = n.as_i64()
                    {
                        return extract_palette_data(&array[3], document).map(|data| (data, n_val as u32));
                    }
                    3 // Default ICCBased to RGB
                } else {
                    3
                }
            } else {
                3
            }
        }
        _ => 3,
    };

    extract_palette_data(&array[3], document).map(|data| (data, base_channels))
}

/// Extract raw palette bytes from the lookup element of an Indexed color space.
/// The lookup can be an inline string/hex-string or a reference to a stream object.
#[cfg(feature = "pdf")]
fn extract_palette_data(lookup: &lopdf::Object, document: &Document) -> Option<Vec<u8>> {
    use lopdf::Object;

    match lookup {
        Object::String(bytes, _) => Some(bytes.clone()),
        Object::Reference(obj_id) => {
            let obj = document.get_object(*obj_id).ok()?;
            match obj {
                Object::Stream(stream) => {
                    // Try to get decompressed content; fall back to raw content.
                    Some(stream.content.clone())
                }
                Object::String(bytes, _) => Some(bytes.clone()),
                _ => None,
            }
        }
        _ => None,
    }
}

impl PdfImageExtractor {
    pub(crate) fn new(pdf_bytes: &[u8]) -> Result<Self> {
        Self::new_with_password(pdf_bytes, None)
    }

    pub(crate) fn new_with_password(pdf_bytes: &[u8], password: Option<&str>) -> Result<Self> {
        let mut doc =
            Document::load_mem(pdf_bytes).map_err(|e| PdfError::InvalidPdf(format!("Failed to load PDF: {}", e)))?;

        if doc.is_encrypted() {
            if let Some(pwd) = password {
                doc.decrypt(pwd).map_err(|_| PdfError::InvalidPassword)?;
            } else {
                return Err(PdfError::PasswordRequired);
            }
        }

        Ok(Self {
            document: doc,
            max_images_per_page: None,
        })
    }

    pub(crate) fn extract_images(&self) -> Result<Vec<PdfImage>> {
        let mut all_images = Vec::new();
        let pages = self.document.get_pages();

        for (page_num, page_id) in pages.iter() {
            let page_dict = match self.document.get_dictionary(*page_id) {
                Ok(d) => d,
                Err(_) => continue,
            };
            let resources = match self.document.get_dict_in_dict(page_dict, b"Resources") {
                Ok(r) => r,
                Err(_) => continue,
            };
            let mut img_index = 0usize;
            let mut visited = std::collections::HashSet::new();
            let mut page_image_count = 0u32;
            self.collect_images_from_resources(
                resources,
                *page_num as usize,
                &mut img_index,
                &mut all_images,
                0,
                &mut visited,
                &mut page_image_count,
            );
        }

        Ok(all_images)
    }

    /// Recursively collect images from a PDF resource dictionary.
    ///
    /// Walks the `XObject` sub-dictionary and:
    /// - Collects every entry with `Subtype = Image` as a `PdfImage`.
    /// - Recurses into every entry with `Subtype = Form` (up to `MAX_FORM_DEPTH`
    ///   levels) so that images nested inside Form XObjects — a common structure
    ///   for tiled technical drawings — are not silently dropped.
    ///
    /// Previously, kreuzberg delegated this scan to `lopdf::get_page_images`,
    /// which only looks at the page-level resource dict and never descends into
    /// Form XObjects.  That caused all images inside a Form XObject (e.g. 86
    /// raster tiles composing a single figure) to be invisible to the extractor.
    fn collect_images_from_resources(
        &self,
        resources: &lopdf::Dictionary,
        page_number: usize,
        img_index: &mut usize,
        out: &mut Vec<PdfImage>,
        depth: usize,
        visited: &mut std::collections::HashSet<lopdf::ObjectId>,
        page_image_count: &mut u32,
    ) {
        // Guard against pathological or cyclic Form XObject chains.
        const MAX_FORM_DEPTH: usize = 8;
        if depth > MAX_FORM_DEPTH {
            tracing::debug!(
                page_number,
                depth,
                "PDF image extraction: max Form XObject recursion depth reached, stopping"
            );
            return;
        }

        let xobject_dict = match self.document.get_dict_in_dict(resources, b"XObject") {
            Ok(d) => d,
            Err(_) => return, // XObject is optional
        };

        for (_, xvalue) in xobject_dict.iter() {
            if self.max_images_per_page.is_some_and(|cap| *page_image_count >= cap) {
                tracing::warn!(
                    page_number,
                    cap = self.max_images_per_page.unwrap(),
                    "PDF image extraction: max images per page reached, skipping remaining objects on this page"
                );
                return;
            }
            use lopdf::Object;

            let id = match xvalue.as_reference() {
                Ok(id) => id,
                Err(_) => continue,
            };
            // Skip XObjects already processed on this page — the same object can be
            // referenced from multiple Form parents (DAG), and processing it more than
            // once would corrupt the sequential image_index.
            if !visited.insert(id) {
                continue;
            }
            let stream = match self
                .document
                .get_object(id)
                .and_then(|o| o.as_stream().map(|s| s.clone()))
            {
                Ok(s) => s,
                Err(_) => continue,
            };
            let dict = &stream.dict;

            let subtype = match dict.get(b"Subtype").and_then(|o| o.as_name()) {
                Ok(n) => n,
                Err(_) => continue,
            };

            if subtype == b"Image" {
                // Collect this image.
                let width = match dict.get(b"Width").and_then(|o| o.as_i64()) {
                    Ok(w) => w,
                    Err(_) => continue,
                };
                let height = match dict.get(b"Height").and_then(|o| o.as_i64()) {
                    Ok(h) => h,
                    Err(_) => continue,
                };
                let color_space = match dict.get(b"ColorSpace") {
                    Ok(cs) => match cs {
                        Object::Array(array) => array[0].as_name().ok().map(|n| String::from_utf8_lossy(n).to_string()),
                        Object::Name(name) => Some(String::from_utf8_lossy(name).to_string()),
                        _ => None,
                    },
                    Err(_) => None,
                };
                let bits_per_component = dict.get(b"BitsPerComponent").and_then(|o| o.as_i64()).ok();
                let mut filters = vec![];
                if let Ok(filter) = dict.get(b"Filter") {
                    match filter {
                        Object::Array(array) => {
                            for obj in array.iter() {
                                if let Ok(name) = obj.as_name() {
                                    filters.push(String::from_utf8_lossy(name).to_string());
                                }
                            }
                        }
                        Object::Name(name) => {
                            filters.push(String::from_utf8_lossy(name).to_string());
                        }
                        _ => {}
                    }
                }

                #[cfg(feature = "pdf")]
                let (palette, palette_base_channels) = extract_indexed_palette(dict, &self.document)
                    .map(|(p, ch)| (Some(p), ch))
                    .unwrap_or((None, 0));

                #[cfg(feature = "pdf")]
                let (data, decoded_format) = decode_image_data(
                    &stream.content,
                    &filters,
                    color_space.as_deref(),
                    width,
                    height,
                    bits_per_component,
                    palette.as_deref(),
                    palette_base_channels,
                );

                #[cfg(not(feature = "pdf"))]
                let (data, decoded_format) = (Bytes::from(stream.content.clone()), "raw".to_string());

                *page_image_count += 1;
                *img_index += 1;
                out.push(PdfImage {
                    page_number,
                    image_index: *img_index,
                    width,
                    height,
                    color_space,
                    bits_per_component,
                    filters,
                    data,
                    decoded_format,
                });
            } else if subtype == b"Form" {
                // Recurse into the Form XObject's own resource dictionary.
                if let Ok(form_resources) = dict.get(b"Resources").and_then(|r| r.as_dict()) {
                    self.collect_images_from_resources(
                        form_resources,
                        page_number,
                        img_index,
                        out,
                        depth + 1,
                        visited,
                        page_image_count,
                    );
                }
            }
        }
    }

    pub(crate) fn extract_images_from_page(&self, page_number: u32) -> Result<Vec<PdfImage>> {
        let pages = self.document.get_pages();
        let page_id = pages
            .get(&page_number)
            .ok_or(PdfError::PageNotFound(page_number as usize))?;

        let page_dict = match self.document.get_dictionary(*page_id) {
            Ok(d) => d,
            Err(_) => return Ok(vec![]),
        };
        let resources = match self.document.get_dict_in_dict(page_dict, b"Resources") {
            Ok(r) => r,
            Err(_) => return Ok(vec![]),
        };

        let mut page_images = Vec::new();
        let mut img_index = 0usize;
        let mut visited = std::collections::HashSet::new();
        let mut page_image_count = 0u32;
        self.collect_images_from_resources(
            resources,
            page_number as usize,
            &mut img_index,
            &mut page_images,
            0,
            &mut visited,
            &mut page_image_count,
        );
        Ok(page_images)
    }

    pub(crate) fn get_image_count(&self) -> Result<usize> {
        let images = self.extract_images()?;
        Ok(images.len())
    }
}

pub(crate) fn extract_images_from_pdf(pdf_bytes: &[u8], max_images_per_page: Option<u32>) -> Result<Vec<PdfImage>> {
    let mut extractor = PdfImageExtractor::new(pdf_bytes)?;
    extractor.max_images_per_page = max_images_per_page;
    extractor.extract_images()
}

pub(crate) fn extract_images_from_pdf_with_password(
    pdf_bytes: &[u8],
    password: &str,
    max_images_per_page: Option<u32>,
) -> Result<Vec<PdfImage>> {
    let mut extractor = PdfImageExtractor::new_with_password(pdf_bytes, Some(password))?;
    extractor.max_images_per_page = max_images_per_page;
    extractor.extract_images()
}

/// Re-extract images that have unusable formats (`"raw"`, `"ccitt"`, `"jbig2"`) by
/// rendering them through pdfium's bitmap pipeline, which handles all PDF filter
/// chains internally.
///
/// Returns the number of images successfully re-extracted.
#[cfg(feature = "pdf")]
pub(crate) fn reextract_raw_images_via_pdfium(pdf_bytes: &[u8], images: &mut [PdfImage]) -> Result<u32> {
    use pdfium_render::prelude::*;

    let needs_fallback = images
        .iter()
        .any(|img| matches!(img.decoded_format.as_str(), "raw" | "ccitt" | "jbig2"));

    if !needs_fallback {
        return Ok(0);
    }

    let pdfium = super::bindings::bind_pdfium(PdfError::RenderingFailed, "image fallback rendering", None)?;
    let document = pdfium
        .load_pdf_from_byte_slice(pdf_bytes, None)
        .map_err(|e| PdfError::InvalidPdf(super::error::format_pdfium_error(e)))?;

    let mut reextracted = 0u32;

    for img in images.iter_mut() {
        if !matches!(img.decoded_format.as_str(), "raw" | "ccitt" | "jbig2") {
            continue;
        }

        // page_number is 1-indexed in PdfImage, pdfium pages are 0-indexed
        let page_idx: i32 = img.page_number.saturating_sub(1) as i32;
        let Ok(page) = document.pages().get(page_idx) else {
            continue;
        };

        // Find the nth image object on this page (image_index is 1-indexed).
        // Recurse into XObjectForm objects so that images nested inside Form
        // XObjects (e.g. tiled technical drawings) are reachable.
        let target_index = img.image_index;
        let mut current_image = 0usize;
        let mut found = false;

        'outer: for obj in page.objects().iter() {
            if collect_image_from_pdfium_obj(
                &obj,
                &document,
                target_index,
                &mut current_image,
                img,
                &mut reextracted,
                0,
            ) {
                found = true;
                break 'outer;
            }
        }

        if !found {
            tracing::debug!(
                page = img.page_number,
                target_index,
                "pdfium fallback: image index {} not found among {} objects",
                target_index,
                current_image,
            );
        }
    }

    Ok(reextracted)
}

/// Walk a single pdfium page object and its nested children, looking for the
/// image at `target_index` (1-based global counter across the page).
///
/// Returns `true` if the target image was found and processed.
#[cfg(feature = "pdf")]
fn collect_image_from_pdfium_obj(
    obj: &pdfium_render::prelude::PdfPageObject<'_>,
    document: &pdfium_render::prelude::PdfDocument<'_>,
    target_index: usize,
    current_image: &mut usize,
    img: &mut PdfImage,
    reextracted: &mut u32,
    depth: usize,
) -> bool {
    const MAX_XOBJECT_DEPTH: usize = 10;
    use image::ImageEncoder;
    use pdfium_render::prelude::PdfPageObject;

    match obj {
        PdfPageObject::Image(image_obj) => {
            *current_image += 1;
            if *current_image == target_index {
                if let Ok(dynamic_image) = image_obj.get_processed_image(document) {
                    let w = dynamic_image.width();
                    let h = dynamic_image.height();
                    let rgba = dynamic_image.to_rgba8();
                    let mut png_buf: Vec<u8> = Vec::new();
                    if image::codecs::png::PngEncoder::new(&mut png_buf)
                        .write_image(rgba.as_raw(), w, h, image::ExtendedColorType::Rgba8)
                        .is_ok()
                    {
                        img.data = Bytes::from(png_buf);
                        img.decoded_format = "png".to_string();
                        img.width = w as i64;
                        img.height = h as i64;
                        *reextracted += 1;
                    }
                }
                return true;
            }
        }
        PdfPageObject::XObjectForm(form_obj) => {
            if depth >= MAX_XOBJECT_DEPTH {
                return false;
            }
            for child in form_obj.iter() {
                if collect_image_from_pdfium_obj(
                    &child,
                    document,
                    target_index,
                    current_image,
                    img,
                    reextracted,
                    depth + 1,
                ) {
                    return true;
                }
            }
        }
        _ => {}
    }
    false
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
        let result = extract_images_from_pdf(b"not a pdf", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_images_empty_pdf() {
        let result = extract_images_from_pdf(b"", None);
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
        let (data, format) = decode_image_data(jpeg_bytes, &filters, Some("DeviceRGB"), 100, 100, Some(8), None, 0);
        assert_eq!(format, "jpeg");
        assert_eq!(data.as_ref(), jpeg_bytes);
    }

    #[cfg(feature = "pdf")]
    #[test]
    fn test_decode_jpx_passthrough() {
        let jpx_bytes = b"\x00\x00\x00\x0cjP  fake_jpx";
        let filters = vec!["JPXDecode".to_string()];
        let (data, format) = decode_image_data(jpx_bytes, &filters, Some("DeviceRGB"), 10, 10, Some(8), None, 0);
        assert_eq!(format, "jpeg2000");
        assert_eq!(data.as_ref(), jpx_bytes);
    }

    #[cfg(feature = "pdf")]
    #[test]
    fn test_decode_unknown_filter_passthrough() {
        let raw_bytes = b"\x00\x01\x02\x03";
        let filters = vec!["RunLengthDecode".to_string()];
        let (data, format) = decode_image_data(raw_bytes, &filters, None, 2, 2, Some(8), None, 0);
        assert_eq!(format, "raw");
        assert_eq!(data.as_ref(), raw_bytes);
    }

    #[cfg(feature = "pdf")]
    #[test]
    fn test_decode_no_filter_uses_detection() {
        let jpeg_bytes = b"\xff\xd8\xff\xe0fake";
        let filters: Vec<String> = vec![];
        let (data, format) = decode_image_data(jpeg_bytes, &filters, None, 10, 10, None, None, 0);
        assert_eq!(format, "jpeg");
        assert_eq!(data.as_ref(), jpeg_bytes);
    }

    #[cfg(feature = "pdf")]
    #[test]
    fn test_decode_flate_valid_rgb_image() {
        use flate2::Compression;
        use flate2::write::ZlibEncoder;
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
        let (data, format) = decode_image_data(&compressed, &filters, Some("DeviceRGB"), 2, 2, Some(8), None, 0);
        assert_eq!(format, "png", "FlateDecode images should be re-encoded as PNG");
        // PNG magic: \x89PNG\r\n\x1a\n
        assert!(
            data.starts_with(b"\x89PNG\r\n\x1a\n"),
            "Decoded data should be a valid PNG (got {} bytes, first bytes: {:?})",
            data.len(),
            &data[..data.len().min(8)]
        );
    }

    #[cfg(feature = "pdf")]
    #[test]
    fn test_decode_flate_indexed_with_palette() {
        use flate2::Compression;
        use flate2::write::ZlibEncoder;
        use std::io::Write;

        // 2x2 indexed image: 4 pixels, each 1 byte (palette index).
        let indices: Vec<u8> = vec![0, 1, 2, 0];

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&indices).unwrap();
        let compressed = encoder.finish().unwrap();

        // RGB palette: 3 entries x 3 channels = 9 bytes.
        let palette: Vec<u8> = vec![
            255, 0, 0, // index 0 = red
            0, 255, 0, // index 1 = green
            0, 0, 255, // index 2 = blue
        ];

        let filters = vec!["FlateDecode".to_string()];
        let (data, format) =
            decode_image_data(&compressed, &filters, Some("Indexed"), 2, 2, Some(8), Some(&palette), 3);
        assert_eq!(format, "png", "Indexed FlateDecode should produce PNG");
        assert!(
            data.starts_with(b"\x89PNG\r\n\x1a\n"),
            "Decoded data should be a valid PNG"
        );
    }

    /// Regression test for issue #788: images nested inside Form XObjects must
    /// be extracted. Previously, `lopdf::get_page_images` only scanned the
    /// page-level Resources dict and silently skipped Form-nested images.
    #[cfg(feature = "pdf")]
    #[test]
    fn test_extract_images_nested_in_form_xobject() {
        use flate2::Compression;
        use flate2::write::ZlibEncoder;
        use lopdf::{Document, Object, Stream, dictionary};
        use std::io::Write;

        // 1×1 DeviceRGB pixel compressed with FlateDecode
        let raw_pixel = vec![255u8, 0u8, 0u8];
        let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
        enc.write_all(&raw_pixel).unwrap();
        let compressed = enc.finish().unwrap();

        let mut doc = Document::with_version("1.4");

        // Image XObject (nested — NOT directly on the page)
        let image_stream = Stream::new(
            dictionary! {
                "Type" => Object::Name(b"XObject".to_vec()),
                "Subtype" => Object::Name(b"Image".to_vec()),
                "Width" => 1i64,
                "Height" => 1i64,
                "ColorSpace" => Object::Name(b"DeviceRGB".to_vec()),
                "BitsPerComponent" => 8i64,
                "Filter" => Object::Name(b"FlateDecode".to_vec())
            },
            compressed,
        );
        let image_id = doc.add_object(image_stream);

        // Form XObject whose Resources contain the image
        let form_stream = Stream::new(
            dictionary! {
                "Type" => Object::Name(b"XObject".to_vec()),
                "Subtype" => Object::Name(b"Form".to_vec()),
                "BBox" => Object::Array(vec![0i64.into(), 0i64.into(), 10i64.into(), 10i64.into()]),
                "Resources" => dictionary! {
                    "XObject" => dictionary! {
                        "Im0" => Object::Reference(image_id)
                    }
                }
            },
            b"/Im0 Do".to_vec(),
        );
        let form_id = doc.add_object(form_stream);

        // Page whose Resources reference only the Form (not the Image)
        let pages_id = doc.new_object_id();
        let page_id = doc.add_object(dictionary! {
            "Type" => Object::Name(b"Page".to_vec()),
            "Parent" => Object::Reference(pages_id),
            "MediaBox" => Object::Array(vec![0i64.into(), 0i64.into(), 100i64.into(), 100i64.into()]),
            "Resources" => dictionary! {
                "XObject" => dictionary! {
                    "Form1" => Object::Reference(form_id)
                }
            }
        });
        doc.set_object(
            pages_id,
            dictionary! {
                "Type" => Object::Name(b"Pages".to_vec()),
                "Kids" => Object::Array(vec![Object::Reference(page_id)]),
                "Count" => 1i64
            },
        );
        let catalog_id = doc.add_object(dictionary! {
            "Type" => Object::Name(b"Catalog".to_vec()),
            "Pages" => Object::Reference(pages_id)
        });
        doc.trailer.set("Root", Object::Reference(catalog_id));

        let mut pdf_bytes = Vec::new();
        doc.save_to(&mut pdf_bytes).unwrap();

        let images = extract_images_from_pdf(&pdf_bytes, None).expect("should parse PDF");
        assert_eq!(images.len(), 1, "image nested inside Form XObject must be found");
        assert_eq!(images[0].width, 1);
        assert_eq!(images[0].height, 1);
    }

    /// Regression: a Form XObject referenced twice from the same page must not
    /// double-count its images (would corrupt the sequential image_index).
    #[cfg(feature = "pdf")]
    #[test]
    fn test_extract_images_form_xobject_referenced_twice() {
        use lopdf::{Document, Object, Stream, dictionary};

        // Minimal JPEG magic — use DCTDecode passthrough so no FlateDecode is needed.
        let jpeg_bytes: Vec<u8> = vec![0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10, b'J', b'F', b'I', b'F'];

        let mut doc = Document::with_version("1.4");

        let image_stream = Stream::new(
            dictionary! {
                "Type" => Object::Name(b"XObject".to_vec()),
                "Subtype" => Object::Name(b"Image".to_vec()),
                "Width" => 10i64,
                "Height" => 10i64,
                "ColorSpace" => Object::Name(b"DeviceRGB".to_vec()),
                "BitsPerComponent" => 8i64,
                "Filter" => Object::Name(b"DCTDecode".to_vec())
            },
            jpeg_bytes,
        );
        let image_id = doc.add_object(image_stream);

        let form_stream = Stream::new(
            dictionary! {
                "Type" => Object::Name(b"XObject".to_vec()),
                "Subtype" => Object::Name(b"Form".to_vec()),
                "BBox" => Object::Array(vec![0i64.into(), 0i64.into(), 10i64.into(), 10i64.into()]),
                "Resources" => dictionary! {
                    "XObject" => dictionary! {
                        "Im0" => Object::Reference(image_id)
                    }
                }
            },
            b"/Im0 Do".to_vec(),
        );
        let form_id = doc.add_object(form_stream);

        let pages_id = doc.new_object_id();
        let page_id = doc.add_object(dictionary! {
            "Type" => Object::Name(b"Page".to_vec()),
            "Parent" => Object::Reference(pages_id),
            "MediaBox" => Object::Array(vec![0i64.into(), 0i64.into(), 100i64.into(), 100i64.into()]),
            "Resources" => dictionary! {
                "XObject" => dictionary! {
                    // Same form referenced under two different names — must not double-count
                    "Form1" => Object::Reference(form_id),
                    "Form2" => Object::Reference(form_id)
                }
            }
        });
        doc.set_object(
            pages_id,
            dictionary! {
                "Type" => Object::Name(b"Pages".to_vec()),
                "Kids" => Object::Array(vec![Object::Reference(page_id)]),
                "Count" => 1i64
            },
        );
        let catalog_id = doc.add_object(dictionary! {
            "Type" => Object::Name(b"Catalog".to_vec()),
            "Pages" => Object::Reference(pages_id)
        });
        doc.trailer.set("Root", Object::Reference(catalog_id));

        let mut pdf_bytes = Vec::new();
        doc.save_to(&mut pdf_bytes).unwrap();

        let images = extract_images_from_pdf(&pdf_bytes, None).expect("should parse PDF");
        assert_eq!(
            images.len(),
            1,
            "Form XObject referenced twice must yield exactly one image, not two"
        );
    }

    /// Regression: images nested at depth 2 (Form inside Form) must be extracted.
    #[cfg(feature = "pdf")]
    #[test]
    fn test_extract_images_depth2_nested_form_xobject() {
        use flate2::Compression;
        use flate2::write::ZlibEncoder;
        use lopdf::{Document, Object, Stream, dictionary};
        use std::io::Write;

        let raw_pixel = vec![0u8, 128u8, 255u8]; // 1×1 DeviceRGB
        let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
        enc.write_all(&raw_pixel).unwrap();
        let compressed = enc.finish().unwrap();

        let mut doc = Document::with_version("1.4");

        let image_stream = Stream::new(
            dictionary! {
                "Type" => Object::Name(b"XObject".to_vec()),
                "Subtype" => Object::Name(b"Image".to_vec()),
                "Width" => 1i64,
                "Height" => 1i64,
                "ColorSpace" => Object::Name(b"DeviceRGB".to_vec()),
                "BitsPerComponent" => 8i64,
                "Filter" => Object::Name(b"FlateDecode".to_vec())
            },
            compressed,
        );
        let image_id = doc.add_object(image_stream);

        // inner form → contains image
        let inner_form = Stream::new(
            dictionary! {
                "Type" => Object::Name(b"XObject".to_vec()),
                "Subtype" => Object::Name(b"Form".to_vec()),
                "BBox" => Object::Array(vec![0i64.into(), 0i64.into(), 5i64.into(), 5i64.into()]),
                "Resources" => dictionary! {
                    "XObject" => dictionary! {
                        "Im0" => Object::Reference(image_id)
                    }
                }
            },
            b"/Im0 Do".to_vec(),
        );
        let inner_form_id = doc.add_object(inner_form);

        // outer form → contains inner form
        let outer_form = Stream::new(
            dictionary! {
                "Type" => Object::Name(b"XObject".to_vec()),
                "Subtype" => Object::Name(b"Form".to_vec()),
                "BBox" => Object::Array(vec![0i64.into(), 0i64.into(), 10i64.into(), 10i64.into()]),
                "Resources" => dictionary! {
                    "XObject" => dictionary! {
                        "Inner" => Object::Reference(inner_form_id)
                    }
                }
            },
            b"/Inner Do".to_vec(),
        );
        let outer_form_id = doc.add_object(outer_form);

        let pages_id = doc.new_object_id();
        let page_id = doc.add_object(dictionary! {
            "Type" => Object::Name(b"Page".to_vec()),
            "Parent" => Object::Reference(pages_id),
            "MediaBox" => Object::Array(vec![0i64.into(), 0i64.into(), 100i64.into(), 100i64.into()]),
            "Resources" => dictionary! {
                "XObject" => dictionary! {
                    "Outer" => Object::Reference(outer_form_id)
                }
            }
        });
        doc.set_object(
            pages_id,
            dictionary! {
                "Type" => Object::Name(b"Pages".to_vec()),
                "Kids" => Object::Array(vec![Object::Reference(page_id)]),
                "Count" => 1i64
            },
        );
        let catalog_id = doc.add_object(dictionary! {
            "Type" => Object::Name(b"Catalog".to_vec()),
            "Pages" => Object::Reference(pages_id)
        });
        doc.trailer.set("Root", Object::Reference(catalog_id));

        let mut pdf_bytes = Vec::new();
        doc.save_to(&mut pdf_bytes).unwrap();

        let images = extract_images_from_pdf(&pdf_bytes, None).expect("should parse PDF");
        assert_eq!(images.len(), 1, "image at depth-2 Form nesting must be extracted");
        assert_eq!(images[0].width, 1);
        assert_eq!(images[0].height, 1);
    }

    #[cfg(feature = "pdf")]
    #[test]
    fn test_decode_flate_indexed_without_palette_grayscale_fallback() {
        use flate2::Compression;
        use flate2::write::ZlibEncoder;
        use std::io::Write;

        // 2x2 indexed image without palette: should fall back to grayscale.
        let indices: Vec<u8> = vec![10, 50, 100, 200];

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&indices).unwrap();
        let compressed = encoder.finish().unwrap();

        let filters = vec!["FlateDecode".to_string()];
        let (data, format) = decode_image_data(&compressed, &filters, Some("Indexed"), 2, 2, Some(8), None, 0);
        assert_eq!(
            format, "png",
            "Indexed without palette should still produce PNG (grayscale)"
        );
        assert!(
            data.starts_with(b"\x89PNG\r\n\x1a\n"),
            "Decoded data should be a valid PNG"
        );
    }
}
