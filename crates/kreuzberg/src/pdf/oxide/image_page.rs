//! Direct rendering of image-only pages.
//!
//! pdf_oxide has no JPEG 2000 decoder, so it drops `/JPXDecode` image XObjects and
//! rasterizes the page blank (#1158). For a page whose content is a single
//! full-page image, we decode that image directly via the in-tree decoders
//! ([`crate::extraction::image::load_image_for_ocr`]) instead. pdf_oxide still
//! handles every other page (vector, text, multi-image composition).

use image::DynamicImage;
use pdf_oxide::PdfDocument;
use pdf_oxide::object::Object;

/// Image filters pdf_oxide's rasterizer errors on (dropping the image, blanking
/// the page). Mirrors pdf_oxide's `image_has_unsupported_filter`; `J2` is the
/// ISO 32000 abbreviation for `JPXDecode`.
const RENDERER_UNSUPPORTED_FILTERS: &[&str] = &["JPXDecode", "J2"];

struct PageImage {
    /// Raw stream bytes. For `/JPXDecode` this is the JPEG 2000 codestream.
    raw: bytes::Bytes,
    /// Whether `/Filter` includes a filter pdf_oxide can't rasterize.
    unsupported_filter: bool,
    /// `/Width` * `/Height`, used to pick the dominant image.
    area: i128,
}

/// Whether `/Filter` (a `Name` or an `Array` of `Name`s) contains a filter
/// pdf_oxide can't rasterize.
fn has_unsupported_filter(dict: &std::collections::HashMap<String, Object>) -> bool {
    let Some(filter) = dict.get("Filter") else {
        return false;
    };
    if let Some(name) = filter.as_name() {
        return RENDERER_UNSUPPORTED_FILTERS.contains(&name);
    }
    filter.as_array().is_some_and(|arr| {
        arr.iter()
            .filter_map(Object::as_name)
            .any(|n| RENDERER_UNSUPPORTED_FILTERS.contains(&n))
    })
}

/// Read an integer dictionary entry, tolerating a `Real`.
fn read_dim(dict: &std::collections::HashMap<String, Object>, key: &str) -> i128 {
    match dict.get(key) {
        Some(Object::Integer(i)) => *i as i128,
        Some(Object::Real(r)) => *r as i128,
        _ => 0,
    }
}

/// Image XObjects referenced directly by the page's resource dictionary. Inline
/// images and images nested in Form XObjects are not covered; such pages fall
/// through to pdf_oxide's rasterizer.
fn collect_page_images(doc: &PdfDocument, page_index: usize) -> Vec<PageImage> {
    let resources = match doc.get_page_resources(page_index) {
        Ok(r) => r,
        Err(e) => {
            tracing::debug!(page = page_index + 1, "get_page_resources failed: {e}");
            return Vec::new();
        }
    };
    let Some(res_dict) = resources.as_dict() else {
        return Vec::new();
    };
    let Some(xobj_entry) = res_dict.get("XObject") else {
        return Vec::new();
    };

    // /XObject may be an indirect reference to the dictionary.
    let xobj_owned;
    let xobj_obj = if let Some(r) = xobj_entry.as_reference() {
        match doc.load_object(r) {
            Ok(o) => {
                xobj_owned = o;
                &xobj_owned
            }
            Err(e) => {
                tracing::debug!(page = page_index + 1, "load XObject dict failed: {e}");
                return Vec::new();
            }
        }
    } else {
        xobj_entry
    };
    let Some(xobj_dict) = xobj_obj.as_dict() else {
        return Vec::new();
    };

    // Sorted so the dominant-image choice is stable across runs.
    let mut names: Vec<&String> = xobj_dict.keys().collect();
    names.sort();

    let mut images = Vec::new();
    for name in names {
        let Some(val) = xobj_dict.get(name.as_str()) else {
            continue;
        };

        let loaded;
        let xobj = if let Some(r) = val.as_reference() {
            match doc.load_object(r) {
                Ok(o) => {
                    loaded = o;
                    &loaded
                }
                Err(_) => continue,
            }
        } else {
            val
        };

        let Some(dict) = xobj.as_dict() else { continue };
        if dict.get("Subtype").and_then(Object::as_name) != Some("Image") {
            continue;
        }
        let Object::Stream { data, .. } = xobj else { continue };

        images.push(PageImage {
            raw: data.clone(),
            unsupported_filter: has_unsupported_filter(dict),
            area: read_dim(dict, "Width") * read_dim(dict, "Height"),
        });
    }

    images
}

/// Decode an image-only page's dominant image XObject directly.
///
/// With `require_unsupported_filter` true (the proactive path), only fires when an
/// image uses a filter pdf_oxide can't rasterize (JPEG 2000) — call it before
/// rasterizing. With false (the safety net), fires for any image-only page — call
/// it after a blank render to recover a dropped image.
///
/// `None` when the page has no usable image, the trigger isn't met, or decoding
/// fails. A decode failure on a page that does carry an unsupported-filter image is
/// logged at `warn` so the blank never reaches OCR silently.
pub(crate) fn render_image_only_page(
    doc: &PdfDocument,
    page_index: usize,
    require_unsupported_filter: bool,
) -> Option<DynamicImage> {
    let images = collect_page_images(doc, page_index);
    if images.is_empty() {
        return None;
    }

    let has_unsupported = images.iter().any(|img| img.unsupported_filter);
    if require_unsupported_filter && !has_unsupported {
        return None;
    }

    let dominant = images.iter().max_by_key(|img| img.area)?;

    match crate::extraction::image::load_image_for_ocr(dominant.raw.as_ref()) {
        Ok(img) => {
            if images.len() > 1 {
                tracing::debug!(
                    page = page_index + 1,
                    image_count = images.len(),
                    "rendered the largest of multiple page images"
                );
            }
            Some(img)
        }
        // Warn only when pdf_oxide genuinely couldn't render the page; otherwise its
        // normal rasterization is correct and this is just a safety-net miss.
        Err(e) if has_unsupported => {
            tracing::warn!(
                page = page_index + 1,
                "page image uses a filter pdf_oxide cannot rasterize (e.g. JPEG 2000) and could not be \
                 decoded; the page may be blank: {e}"
            );
            None
        }
        Err(e) => {
            tracing::debug!(page = page_index + 1, "image-only decode failed: {e}");
            None
        }
    }
}
