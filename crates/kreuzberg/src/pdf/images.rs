use super::error::{PdfError, Result};
use bytes::Bytes;
use lopdf::Document;
use pdfium_render::prelude::PdfPageObjectsCommon;
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
    pub(crate) max_images_per_page: Option<u32>,
}

/// Decode raw PDF image stream bytes according to PDF filter(s).
///
/// Returns `(decoded_bytes, format_string)`.
pub(crate) fn decode_image_data(
    data: &[u8],
    filters: &[String],
    color_space: Option<&str>,
    width: i64,
    height: i64,
    bits_per_component: Option<i64>,
    palette: Option<&[u8]>,
    palette_base_channels: u32,
) -> (Bytes, String) {
    let mut decoded = data.to_vec();
    let mut last_format = "raw".to_string();

    for filter in filters {
        match filter.as_str() {
            "FlateDecode" | "Fl" => {
                use flate2::read::ZlibDecoder;
                use std::io::Read;

                let mut decoder = ZlibDecoder::new(&decoded[..]);
                let mut buffer = Vec::new();
                if decoder.read_to_end(&mut buffer).is_ok() {
                    decoded = buffer;
                }
            }
            "DCTDecode" | "DCT" => {
                last_format = "jpeg".to_string();
            }
            "JPXDecode" => {
                last_format = "jpeg2000".to_string();
            }
            "CCITTFaxDecode" | "CCF" => {
                last_format = "ccitt".to_string();
            }
            "JBIG2Decode" => {
                last_format = "jbig2".to_string();
            }
            _ => {}
        }
    }

    // Post-processing for specific formats.
    if last_format == "raw" && color_space == Some("Indexed") {
        if let Some(p) = palette {
            if let Ok(png_data) =
                encode_indexed_as_png(&decoded, p, width, height, palette_base_channels, bits_per_component)
            {
                return (Bytes::from(png_data), "png".to_string());
            }
        } else {
            // Fallback: treat as grayscale if no palette found
            if let Ok(png_data) = encode_grayscale_as_png(&decoded, width, height, bits_per_component) {
                return (Bytes::from(png_data), "png".to_string());
            }
        }
    }

    (Bytes::from(decoded), last_format)
}

fn encode_grayscale_as_png(
    data: &[u8],
    width: i64,
    height: i64,
    bits: Option<i64>,
) -> std::result::Result<Vec<u8>, ()> {
    use image::{ColorType, ImageEncoder, codecs::png::PngEncoder};

    let color_type = match bits {
        Some(1) => ColorType::L8, // We'll expand 1-bit to 8-bit for simplicity
        _ => ColorType::L8,
    };

    let processed_data = if bits == Some(1) {
        let mut expanded = Vec::with_capacity(data.len() * 8);
        for &byte in data {
            for i in (0..8).rev() {
                expanded.push(if (byte >> i) & 1 == 1 { 255 } else { 0 });
            }
        }
        expanded
    } else {
        data.to_vec()
    };

    let mut png_bytes = Vec::new();
    let encoder = PngEncoder::new(&mut png_bytes);
    encoder
        .write_image(&processed_data, width as u32, height as u32, color_type)
        .map_err(|_| ())?;

    Ok(png_bytes)
}

fn encode_indexed_as_png(
    indices: &[u8],
    palette: &[u8],
    width: i64,
    height: i64,
    base_channels: u32,
    bits: Option<i64>,
) -> std::result::Result<Vec<u8>, ()> {
    use image::{ColorType, ImageEncoder, codecs::png::PngEncoder};

    // Expand indices if they are sub-byte.
    let expanded_indices = match bits {
        Some(b) if b < 8 => {
            let mut expanded = Vec::with_capacity(indices.len() * (8 / b as usize));
            let mask = (1 << b) - 1;
            for &byte in indices {
                for i in (0..(8 / b)).rev() {
                    expanded.push((byte >> (i * b as u32)) & mask);
                }
            }
            expanded
        }
        _ => indices.to_vec(),
    };

    // Convert indices + palette to RGB(A).
    let channels = base_channels as usize;
    let mut rgb_data = Vec::with_capacity(expanded_indices.len() * channels);
    for &idx in &expanded_indices {
        let start = idx as usize * channels;
        if start + channels <= palette.len() {
            rgb_data.extend_from_slice(&palette[start..start + channels]);
        } else {
            // Fallback for out-of-bounds index.
            rgb_data.resize(rgb_data.len() + channels, 0);
        }
    }

    let color_type = if channels == 4 {
        ColorType::Rgba8
    } else {
        ColorType::Rgb8
    };

    let mut png_bytes = Vec::new();
    let encoder = PngEncoder::new(&mut png_bytes);
    encoder
        .write_image(&rgb_data, width as u32, height as u32, color_type)
        .map_err(|_| ())?;

    Ok(png_bytes)
}

#[cfg(feature = "pdf")]
fn extract_indexed_palette(dict: &lopdf::Dictionary, doc: &Document) -> Option<(Vec<u8>, u32)> {
    use lopdf::Object;
    let cs = dict.get(b"ColorSpace").ok()?;

    let (base_cs_name, _hival, lookup) = match cs {
        Object::Array(arr) if arr.len() >= 4 && arr[0].as_name().ok() == Some(b"Indexed") => {
            let base = arr[1].as_name().ok()?;
            let hival = arr[2].as_i64().ok()?;
            let lookup = &arr[3];
            (base, hival, lookup)
        }
        _ => return None,
    };

    let channels = match base_cs_name {
        b"DeviceRGB" | b"RGB" => 3,
        b"DeviceCMYK" | b"CMYK" => 4,
        b"DeviceGray" | b"G" => 1,
        _ => 3, // Default to RGB.
    };

    let palette_data = match lookup {
        Object::String(s, _) => s.clone(),
        Object::Reference(id) => {
            if let Ok(stream) = doc.get_object(*id).and_then(|obj| obj.as_stream()) {
                stream.decode().ok()?
            } else {
                return None;
            }
        }
        _ => return None,
    };

    Some((palette_data, channels))
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

    pub(crate) fn extract_images(&self, max_images_per_page: Option<u32>) -> Result<Vec<PdfImage>> {
        let mut all_images = Vec::new();
        let pages = self.document.get_pages();

        for (page_num, page_id) in pages.iter() {
            // get_page_resources traverses the parent chain, handling inherited Resources
            let (resource_dict, resource_ids) = match self.document.get_page_resources(*page_id) {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!(page = page_num, error = %e, "Failed to get page resources; skipping page");
                    continue;
                }
            };

            // Collect all XObject IDs from page resources (inline dict + referenced dicts)
            let mut xobj_ids: Vec<lopdf::ObjectId> = Vec::new();
            if let Some(res) = resource_dict {
                xobj_ids.extend(self.xobj_ids_from_resources(res));
            }
            for res_id in resource_ids {
                if let Ok(res) = self.document.get_dictionary(res_id) {
                    xobj_ids.extend(self.xobj_ids_from_resources(res));
                }
            }

            let mut seen = std::collections::HashSet::new();
            let mut page_image_count = 0;
            self.collect_images_from_resources(
                xobj_ids,
                *page_num as usize,
                &mut seen,
                &mut all_images,
                max_images_per_page,
                &mut page_image_count,
            );
        }

        Ok(all_images)
    }

    /// Collect all XObject object IDs referenced by a resources dictionary.
    fn xobj_ids_from_resources(&self, res: &lopdf::Dictionary) -> Vec<lopdf::ObjectId> {
        use lopdf::Object;
        let xobj = match res.get(b"XObject") {
            Ok(Object::Dictionary(d)) => d,
            Ok(Object::Reference(id)) => match self.document.get_dictionary(*id) {
                Ok(d) => d,
                Err(_) => return vec![],
            },
            _ => return vec![],
        };
        xobj.iter().filter_map(|(_, v)| v.as_reference().ok()).collect()
    }

    /// Collect XObject IDs from a stream's inline Resources dict (used for Form XObjects).
    fn xobj_ids_from_stream_resources(&self, stream_dict: &lopdf::Dictionary) -> Vec<lopdf::ObjectId> {
        use lopdf::Object;
        match stream_dict.get(b"Resources") {
            Ok(Object::Dictionary(d)) => self.xobj_ids_from_resources(d),
            Ok(Object::Reference(id)) => match self.document.get_dictionary(*id) {
                Ok(d) => self.xobj_ids_from_resources(d),
                Err(_) => vec![],
            },
            _ => vec![],
        }
    }

    /// Walk a list of XObject IDs, extracting Image XObjects and recursing into Form XObjects.
    fn collect_images_from_resources(
        &self,
        xobj_ids: Vec<lopdf::ObjectId>,
        page_num: usize,
        seen: &mut std::collections::HashSet<lopdf::ObjectId>,
        all_images: &mut Vec<PdfImage>,
        max_images_per_page: Option<u32>,
        page_image_count: &mut u32,
    ) {
        use lopdf::Object;

        for id in xobj_ids {
            if !seen.insert(id) {
                continue; // prevent cycles in Form XObject references
            }

            let obj = match self.document.get_object(id) {
                Ok(o) => o,
                Err(_) => continue,
            };

            let stream = match obj.as_stream() {
                Ok(s) => s,
                Err(_) => continue,
            };

            let subtype = match stream.dict.get(b"Subtype") {
                Ok(Object::Name(n)) => n,
                _ => continue,
            };

            if subtype == b"Image" {
                // Check per-page cap before decoding.
                if let Some(cap) = max_images_per_page {
                    if *page_image_count >= cap {
                        tracing::warn!(
                            page_number = page_num,
                            cap,
                            "PDF page exceeds max_images_per_page; skipping remaining image extraction for this page"
                        );
                        return;
                    }
                }

                let width = stream.dict.get(b"Width").and_then(|w| w.as_i64()).unwrap_or(0);
                let height = stream.dict.get(b"Height").and_then(|h| h.as_i64()).unwrap_or(0);
                let color_space = stream.dict.get(b"ColorSpace").and_then(|cs| match cs {
                    Object::Name(n) => Some(String::from_utf8_lossy(n).into_owned()),
                    _ => None,
                });
                let bits = stream.dict.get(b"BitsPerComponent").and_then(|b| b.as_i64());
                let filters = match stream.dict.get(b"Filter") {
                    Ok(Object::Name(n)) => vec![String::from_utf8_lossy(n).into_owned()],
                    Ok(Object::Array(arr)) => arr
                        .iter()
                        .filter_map(|o| o.as_name().ok())
                        .map(|n| String::from_utf8_lossy(n).into_owned())
                        .collect(),
                    _ => vec![],
                };

                let (palette, palette_base_channels) = extract_indexed_palette(&stream.dict, &self.document)
                    .map(|(p, ch)| (Some(p), ch))
                    .unwrap_or((None, 0));

                let (data, decoded_format) = decode_image_data(
                    &stream.content,
                    &filters,
                    color_space.as_deref(),
                    width,
                    height,
                    bits,
                    palette.as_deref(),
                    palette_base_channels,
                );

                *page_image_count += 1;
                all_images.push(PdfImage {
                    page_number: page_num,
                    image_index: *page_image_count as usize,
                    width,
                    height,
                    color_space,
                    bits_per_component: bits,
                    filters,
                    data,
                    decoded_format,
                });
            } else if subtype == b"Form" {
                // Recursively collect from Form XObject resources.
                let nested_ids = self.xobj_ids_from_stream_resources(&stream.dict);
                self.collect_images_from_resources(
                    nested_ids,
                    page_num,
                    seen,
                    all_images,
                    max_images_per_page,
                    page_image_count,
                );
            }
        }
    }
}

pub(crate) fn extract_images_from_pdf(pdf_bytes: &[u8], max_images_per_page: Option<u32>) -> Result<Vec<PdfImage>> {
    let extractor = PdfImageExtractor::new(pdf_bytes)?;
    extractor.extract_images(max_images_per_page)
}

/// Re-extract images that have unusable formats (`"raw"`, `"ccitt"`, `"jbig2"`) by
/// rendering them through pdfium's bitmap pipeline, which handles all PDF filter
/// chains internally.
///
/// Returns the number of images successfully re-extracted.
#[cfg(feature = "pdf")]
pub(crate) fn reextract_raw_images_via_pdfium(pdf_bytes: &[u8], images: &mut [PdfImage]) -> Result<u32> {
    use image::ImageEncoder;

    if images.is_empty() {
        return Ok(0);
    }

    let mut reextracted_count = 0;
    let bindings = pdfium_render::prelude::Pdfium::bind_to_system_library()
        .or_else(|_| pdfium_render::prelude::Pdfium::bind_to_library("pdfium"))
        .map_err(|e| PdfError::MetadataExtractionFailed(format!("Failed to bind to pdfium: {}", e)))?;

    let pdfium = pdfium_render::prelude::Pdfium::new(bindings);
    let doc = pdfium
        .load_pdf_from_byte_slice(pdf_bytes, None)
        .map_err(|e| PdfError::InvalidPdf(format!("Failed to load PDF via pdfium: {:?}", e)))?;

    for image in images.iter_mut() {
        if !["raw", "ccitt", "jbig2"].contains(&image.decoded_format.as_str()) {
            continue;
        }

        let page = match doc.pages().get((image.page_number - 1) as i32) {
            Ok(p) => p,
            Err(_) => continue,
        };

        // Pdfium's image extraction is 0-indexed per page.
        // We use the image_index as a hint but we need to find the correct object.
        let mut found_bitmap = None;
        for (idx, obj) in page.objects().iter().enumerate() {
            if let Some(img_obj) = obj.as_image_object() {
                if idx + 1 == image.image_index {
                    if let Ok(bitmap) = img_obj.get_bitmap() {
                        found_bitmap = Some(bitmap);
                        break;
                    }
                }
            }
        }

        if let Some(bitmap) = found_bitmap {
            let mut png_bytes = Vec::new();
            let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);
            if encoder
                .write_image(
                    bitmap.as_byte_slice(),
                    bitmap.width() as u32,
                    bitmap.height() as u32,
                    image::ColorType::Rgba8,
                )
                .is_ok()
            {
                image.data = Bytes::from(png_bytes);
                image.decoded_format = "png".to_string();
                reextracted_count += 1;
            }
        }
    }

    Ok(reextracted_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_flate_indexed_png() {
        use flate2::Compression;
        use flate2::write::ZlibEncoder;
        use std::io::Write;

        // 2x2 indexed image: 1 (red), 0 (blue), 0, 1
        let palette = vec![
            0, 0, 255, // 0: Blue
            255, 0, 0, // 1: Red
        ];
        let indices = vec![0b10010000]; // 1, 0, 0, 1 packed as 2-bit? No, let's use 8-bit for simplicity
        let indices_8bit = vec![1, 0, 0, 1];

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&indices_8bit).unwrap();
        let compressed = encoder.finish().unwrap();

        let filters = vec!["FlateDecode".to_string()];
        let (data, format) =
            decode_image_data(&compressed, &filters, Some("Indexed"), 2, 2, Some(8), Some(&palette), 3);
        assert_eq!(format, "png", "Indexed FlateDecode should produce PNG");
        assert!(
            data.starts_with(b"\x89PNG\r\n\x1a\n"),
            "Decoded data should be a valid PNG"
        );
    }

    /// Regression test for issue #789: `max_images_per_page` must be enforced
    /// in the lopdf extraction path so pages with thousands of image objects
    /// do not cause an indefinite hang.
    #[cfg(feature = "pdf")]
    #[test]
    fn test_max_images_per_page_cap_skips_dense_page() {
        use flate2::Compression;
        use flate2::write::ZlibEncoder;
        use lopdf::{Document, Object, Stream, dictionary};
        use std::io::Write;

        // Build a PDF with two pages:
        //   page 1 → 3 images (below cap of 2 → wait, cap is 2 so 3 > 2 → skipped)
        //   page 2 → 1 image  (below cap → extracted)
        // After applying max_images_per_page = 2, only page 2's image is returned.

        let make_compressed_pixel = || {
            let raw = vec![255u8, 0u8, 0u8]; // 1×1 DeviceRGB
            let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
            enc.write_all(&raw).unwrap();
            enc.finish().unwrap()
        };

        let mut doc = Document::with_version("1.4");

        // Helper: add an Image XObject and return its id.
        let add_image = |doc: &mut Document| {
            let stream = Stream::new(
                dictionary! {
                    "Type" => Object::Name(b"XObject".to_vec()),
                    "Subtype" => Object::Name(b"Image".to_vec()),
                    "Width" => 1i64,
                    "Height" => 1i64,
                    "ColorSpace" => Object::Name(b"DeviceRGB".to_vec()),
                    "BitsPerComponent" => 8i64,
                    "Filter" => Object::Name(b"FlateDecode".to_vec())
                },
                make_compressed_pixel(),
            );
            doc.add_object(stream)
        };

        // Page 1: 3 images (will be skipped when cap = 2)
        let img1a = add_image(&mut doc);
        let img1b = add_image(&mut doc);
        let img1c = add_image(&mut doc);

        // Page 2: 1 image (below cap → extracted)
        let img2a = add_image(&mut doc);

        let pages_id = doc.new_object_id();

        let page1_id = doc.add_object(dictionary! {
            "Type" => Object::Name(b"Page".to_vec()),
            "Parent" => Object::Reference(pages_id),
            "MediaBox" => Object::Array(vec![0i64.into(), 0i64.into(), 100i64.into(), 100i64.into()]),
            "Resources" => dictionary! {
                "XObject" => dictionary! {
                    "Im0" => Object::Reference(img1a),
                    "Im1" => Object::Reference(img1b),
                    "Im2" => Object::Reference(img1c)
                }
            }
        });
        let page2_id = doc.add_object(dictionary! {
            "Type" => Object::Name(b"Page".to_vec()),
            "Parent" => Object::Reference(pages_id),
            "MediaBox" => Object::Array(vec![0i64.into(), 0i64.into(), 100i64.into(), 100i64.into()]),
            "Resources" => dictionary! {
                "XObject" => dictionary! {
                    "Im0" => Object::Reference(img2a)
                }
            }
        });

        doc.set_object(
            pages_id,
            dictionary! {
                "Type" => Object::Name(b"Pages".to_vec()),
                "Kids" => Object::Array(vec![Object::Reference(page1_id), Object::Reference(page2_id)]),
                "Count" => 2i64
            },
        );
        let catalog_id = doc.add_object(dictionary! {
            "Type" => Object::Name(b"Catalog".to_vec()),
            "Pages" => Object::Reference(pages_id)
        });
        doc.trailer.set("Root", Object::Reference(catalog_id));

        let mut pdf_bytes = Vec::new();
        doc.save_to(&mut pdf_bytes).unwrap();

        // No cap: all 4 images extracted.
        let all = extract_images_from_pdf(&pdf_bytes, None).expect("should parse");
        assert_eq!(all.len(), 4, "without cap: all images extracted");

        // Cap of 2: page 1 has 3 images → skipped; page 2 has 1 image → extracted.
        let capped = extract_images_from_pdf(&pdf_bytes, Some(2)).expect("should parse");
        assert_eq!(capped.len(), 1, "with cap=2: only page-2 image extracted");
        assert_eq!(capped[0].width, 1);

        // Edge case: cap=0 means every page with any images is skipped.
        let zero_capped = extract_images_from_pdf(&pdf_bytes, Some(0)).expect("should parse");
        assert_eq!(zero_capped.len(), 0, "cap=0: all pages skipped");

        // Edge case: cap exactly equal to page-1 image count (3) — page is NOT skipped.
        let exact_capped = extract_images_from_pdf(&pdf_bytes, Some(3)).expect("should parse");
        assert_eq!(exact_capped.len(), 4, "cap=exactly-page-count: all images extracted");
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
