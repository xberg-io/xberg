//! Pure-Rust rendering of image-only PDF pages.
//!
//! pdf_oxide's rasterizer has no JPEG 2000 decoder: `extract_image_from_xobject`
//! returns `UnsupportedFilter` for `/JPXDecode`, so the rasterizer drops the image
//! and the page comes out blank. Scanned PDFs that store each page as a full-page
//! JPEG 2000 image therefore rasterize to white, and every OCR backend extracts
//! nothing (issue #1158).
//!
//! kreuzberg already bundles pure-Rust decoders for these formats
//! (`hayro-jpeg2000`, `hayro-jbig2`) for standalone image extraction. This module
//! reuses them, via [`crate::extraction::image::load_image_for_ocr`], to render an
//! image-only page directly from its image XObject — bypassing the rasterizer for
//! exactly the pages pdf_oxide cannot handle.
//!
//! Scope: the page must be image-composable — its drawing dominated by a single
//! full-page image XObject (the scanned-page case). pdf_oxide keeps doing all page
//! composition (CTM, masks, vector, text); this module only substitutes the
//! image-decode step pdf_oxide lacks. Pages mixing such an image with significant
//! vector/text content are recovered best-effort (dominant image only); perfectly
//! compositing them would mean re-implementing pdf_oxide's rasterizer.

use image::DynamicImage;
use pdf_oxide::PdfDocument;
use pdf_oxide::object::Object;

/// Image filters pdf_oxide's rasterizer cannot decode: it errors and drops the
/// image, blanking the page. Mirrors pdf_oxide's own `image_has_unsupported_filter`
/// gate in `rendering/separation_renderer.rs` (JPEG 2000 — no pure-Rust decoder is
/// bundled in pdf_oxide). `J2` is the ISO 32000 abbreviation for `JPXDecode`.
const RENDERER_UNSUPPORTED_FILTERS: &[&str] = &["JPXDecode", "J2"];

/// An image XObject discovered on a page, with the metadata needed to decide
/// whether and how to decode it ourselves.
struct PageImage {
    /// Raw (encoded) stream bytes. For `/JPXDecode` these are the JPEG 2000
    /// codestream verbatim, which is exactly what the JP2 decoder consumes.
    raw: bytes::Bytes,
    /// `/Filter` chain names in array order.
    filters: Vec<String>,
    /// Pixel area (`/Width` * `/Height`), used to pick the dominant image.
    area: i128,
}

impl PageImage {
    fn has_unsupported_filter(&self) -> bool {
        self.filters
            .iter()
            .any(|f| RENDERER_UNSUPPORTED_FILTERS.contains(&f.as_str()))
    }
}

/// Read the `/Filter` entry (a `Name` or an `Array` of `Name`s) as a list of
/// filter names, in order.
fn read_filters(dict: &std::collections::HashMap<String, Object>) -> Vec<String> {
    match dict.get("Filter") {
        Some(f) => {
            if let Some(name) = f.as_name() {
                vec![name.to_string()]
            } else if let Some(arr) = f.as_array() {
                arr.iter().filter_map(|o| o.as_name().map(str::to_string)).collect()
            } else {
                Vec::new()
            }
        }
        None => Vec::new(),
    }
}

/// Read a dictionary integer entry (e.g. `/Width`), tolerating a `Real` value.
fn read_dim(dict: &std::collections::HashMap<String, Object>, key: &str) -> i128 {
    match dict.get(key) {
        Some(Object::Integer(i)) => *i as i128,
        Some(Object::Real(r)) => *r as i128,
        _ => 0,
    }
}

/// Collect every image XObject directly referenced by the page's resource
/// dictionary, with the metadata needed to decide whether to decode it ourselves.
///
/// Inline images (`BI`/`EI`) and images nested inside Form XObjects are not
/// covered — those are rare for the scanned-page case this module targets, and a
/// page that needs them simply falls through to pdf_oxide's rasterizer.
fn collect_page_images(doc: &PdfDocument, page_index: usize) -> Vec<PageImage> {
    let resources = match doc.get_page_resources(page_index) {
        Ok(r) => r,
        Err(e) => {
            tracing::debug!(page = page_index + 1, "image-only render: get_page_resources failed: {e}");
            return Vec::new();
        }
    };
    let Some(res_dict) = resources.as_dict() else {
        return Vec::new();
    };
    let Some(xobj_entry) = res_dict.get("XObject") else {
        return Vec::new();
    };

    // The /XObject value may be an indirect reference to the dictionary.
    let xobj_owned;
    let xobj_obj = if let Some(r) = xobj_entry.as_reference() {
        match doc.load_object(r) {
            Ok(o) => {
                xobj_owned = o;
                &xobj_owned
            }
            Err(e) => {
                tracing::debug!(page = page_index + 1, "image-only render: load XObject dict failed: {e}");
                return Vec::new();
            }
        }
    } else {
        xobj_entry
    };
    let Some(xobj_dict) = xobj_obj.as_dict() else {
        return Vec::new();
    };

    // Deterministic order so the "dominant image" choice is stable across runs.
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
        // The raw stream bytes are the encoded image data. For /JPXDecode this is
        // the JPEG 2000 codestream; pdf_oxide could not turn it into pixels.
        let Object::Stream { data, .. } = xobj else { continue };

        images.push(PageImage {
            raw: data.clone(),
            filters: read_filters(dict),
            area: read_dim(dict, "Width") * read_dim(dict, "Height"),
        });
    }

    images
}

/// Render an image-only page directly from its dominant image XObject, using
/// kreuzberg's pure-Rust decoders.
///
/// `require_unsupported_filter` selects the trigger:
/// - `true` — proactive path: only fires when the dominant image uses a filter
///   pdf_oxide cannot rasterize (JPEG 2000). Call this *before* rasterizing to skip
///   a render that pdf_oxide would blank.
/// - `false` — safety-net path: fires for any image-only page. Call this *after* a
///   blank pdf_oxide render to recover an image pdf_oxide silently dropped.
///
/// Returns `None` when the page has no usable image, the trigger condition is not
/// met, or no image could be decoded. A decode failure on a page that does carry an
/// unsupported-filter image is logged at `warn`, so the blank page never passes
/// silently to OCR.
pub(crate) fn render_image_only_page(
    doc: &PdfDocument,
    page_index: usize,
    require_unsupported_filter: bool,
) -> Option<DynamicImage> {
    let images = collect_page_images(doc, page_index);
    if images.is_empty() {
        return None;
    }

    let has_unsupported = images.iter().any(PageImage::has_unsupported_filter);
    if require_unsupported_filter && !has_unsupported {
        return None;
    }

    // The dominant image is the largest by pixel area — for a scanned page that is
    // the full-page scan. Ties resolve to the first in deterministic name order.
    let dominant = images.iter().max_by_key(|img| img.area)?;

    match crate::extraction::image::load_image_for_ocr(dominant.raw.as_ref()) {
        Ok(img) => {
            if images.len() > 1 {
                tracing::debug!(
                    page = page_index + 1,
                    image_count = images.len(),
                    "image-only render: page has multiple images; rendered the largest one"
                );
            }
            Some(img)
        }
        Err(e) => {
            // Only escalate to a warning when pdf_oxide genuinely could not render
            // this page (it carries an unsupported-filter image). Otherwise the
            // caller's normal rasterization is the right result and this is just a
            // safety-net miss.
            if has_unsupported {
                tracing::warn!(
                    page = page_index + 1,
                    "page image uses a filter pdf_oxide cannot rasterize (e.g. JPEG 2000) and could \
                     not be decoded for rendering/OCR; the page may be blank: {e}"
                );
            } else {
                tracing::debug!(page = page_index + 1, "image-only render: decode failed: {e}");
            }
            None
        }
    }
}
