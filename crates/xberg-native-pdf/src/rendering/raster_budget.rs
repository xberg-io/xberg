//! Page raster sizing, working-memory checks, and PNG encoding.

use super::{PageRenderer, knockout_resources};
use crate::content::{operators::Operator, parser::parse_content_stream};
use crate::document::PdfDocument;
use crate::error::{Error, Result};
use crate::object::{Object, ObjectRef};
use crate::rendering::ext_gstate::parse_ext_g_state_inner;
use std::collections::{HashMap, HashSet};
use tiny_skia::Pixmap;

pub(in crate::rendering) struct PngRenderBudget {
    pub(in crate::rendering) max_working_bytes: usize,
    pub(in crate::rendering) minimum_dpi: u32,
}

impl PageRenderer {
    pub(super) fn check_source_image_budget(&mut self) -> Result<()> {
        self.knockout_resources.check()?;
        if let Some(budget) = &self.source_image_budget
            && let Some(error) = budget.error()
        {
            self.reported_source_budget_exceeded = budget.exceeded();
            return Err(error);
        }
        Ok(())
    }
}

/// Resize a premultiplied RGBA byte buffer using SIMD-accelerated bilinear filtering.
///
/// The resizer's default treats alpha as straight and multiplies every pixel by it before
/// filtering, then divides it out after; on a premultiplied buffer that leaves a soft edge with
/// more colour than its opacity allows, so the alpha pass is turned off. (GH#1905)
///
/// Returns `None` on failure (zero dimensions, SIMD dispatch error) so callers
/// can fall back to tiny_skia's own resampling path.
pub(super) fn resize_rgba(src: &[u8], src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) -> Option<Vec<u8>> {
    use fast_image_resize::images::Image;
    use fast_image_resize::pixels::PixelType;
    use fast_image_resize::{FilterType, ResizeAlg, ResizeOptions, Resizer};

    // from_slice_u8 needs a mutable slice; copy into a local buffer. ~keep
    let mut buf = src.to_vec();
    let src_img = Image::from_slice_u8(src_w, src_h, &mut buf, PixelType::U8x4).ok()?;
    let mut dst_img = Image::new(dst_w, dst_h, PixelType::U8x4);
    Resizer::new()
        .resize(
            &src_img,
            &mut dst_img,
            &ResizeOptions::new()
                .resize_alg(ResizeAlg::Convolution(FilterType::Bilinear))
                .use_alpha(false),
        )
        .ok()?;
    Some(dst_img.into_vec())
}

pub(super) fn raster_dimensions(page_width: f32, page_height: f32, scale: f32, has_scale_override: bool) -> (u32, u32) {
    let width = page_width * scale;
    let height = page_height * scale;
    if has_scale_override {
        // Exact fractional scale uses rounding and allows very narrow pages to stay one pixel.
        ((width.round() as u32).max(1), (height.round() as u32).max(1))
    } else {
        (width.ceil() as u32, height.ceil() as u32)
    }
}

#[derive(Default)]
struct SpotPlaneScan {
    // Depth preserves the remaining recursion allowance. Forms without local resources
    // can resolve the same name differently in each enclosing scope. ~keep
    memo: HashMap<(ObjectRef, u32, Option<usize>), bool>,
    next_scope: usize,
}

#[derive(Clone, Copy)]
struct ResourceScope<'a> {
    resources: &'a Object,
    id: usize,
}

fn inherited_form_calls(doc: &PdfDocument, stream: &Object, reference: Option<ObjectRef>) -> HashSet<String> {
    let data = match reference {
        Some(reference) => doc.decode_stream_with_encryption(stream, reference),
        None => stream.decode_stream_data(),
    };
    data.and_then(|data| parse_content_stream(&data))
        .unwrap_or_default()
        .into_iter()
        .filter_map(|op| match op {
            Operator::Do { name } => Some(name),
            _ => None,
        })
        .collect()
}

/// Detect a declared soft mask on the page or in its reachable forms. Pattern and
/// annotation resources are outside this scan. At the depth cap, allow for a mask.
pub(in crate::rendering) fn declares_spot_soft_mask(doc: &PdfDocument, resources: &Object) -> bool {
    let mut scan = SpotPlaneScan::default();
    let scope = scan.scope(resources);
    scan.resources(doc, scope, None, 0)
}

impl SpotPlaneScan {
    fn scope<'a>(&mut self, resources: &'a Object) -> ResourceScope<'a> {
        let id = self.next_scope;
        self.next_scope += 1;
        ResourceScope { resources, id }
    }

    fn form(&mut self, doc: &PdfDocument, raw: &Object, parent: ResourceScope<'_>, depth: u32) -> bool {
        let reference = raw.as_reference();
        if let Some(reference) = reference
            && let Some(known) = self
                .memo
                .get(&(reference, depth, None))
                .or_else(|| self.memo.get(&(reference, depth, Some(parent.id))))
        {
            return *known;
        }
        let Ok(stream) = doc.resolve_object(raw) else {
            return false;
        };
        let Object::Stream { dict, .. } = &stream else {
            return false;
        };
        if dict.get("Subtype").and_then(Object::as_name) != Some("Form") {
            return false;
        }

        let local = dict.get("Resources");
        let key = reference.map(|reference| (reference, depth, local.is_none().then_some(parent.id)));
        let found = if let Some(local) = local {
            doc.resolve_object(local).ok().is_some_and(|resources| {
                let scope = self.scope(&resources);
                self.resources(doc, scope, None, depth + 1)
            })
        } else {
            // A form inherits name bindings, but only its operators can invoke
            // siblings. Walking all bindings here would invent self-recursion. ~keep
            let names = inherited_form_calls(doc, &stream, reference);
            self.resources(doc, parent, Some(&names), depth + 1)
        };

        if let Some(key) = key {
            self.memo.insert(key, found);
        }
        found
    }

    fn resources(
        &mut self,
        doc: &PdfDocument,
        scope: ResourceScope<'_>,
        names: Option<&HashSet<String>>,
        depth: u32,
    ) -> bool {
        use crate::rendering::ext_gstate::SoftMaskValue;

        let Some(dict) = scope.resources.as_dict() else {
            return false;
        };
        if depth >= knockout_resources::MAX_NESTING_DEPTH as u32 {
            return true;
        }
        let resolve = |kind| dict.get(kind).and_then(|object| doc.resolve_object(object).ok());
        // ~keep An inherited scope already checked every state before following its forms.
        if names.is_none()
            && resolve("ExtGState")
                .as_ref()
                .and_then(Object::as_dict)
                .into_iter()
                .flatten()
                .any(|(_, state)| {
                    parse_ext_g_state_inner(state, doc)
                        .is_ok_and(|state| matches!(state.smask, Some(SoftMaskValue::Form(_))))
                })
        {
            return true;
        }
        let xobjects = resolve("XObject");
        xobjects
            .iter()
            .filter_map(Object::as_dict)
            .flatten()
            .any(|(name, raw)| names.is_none_or(|names| names.contains(name)) && self.form(doc, raw, scope, depth))
    }
}

/// Per-pixel bytes the PNG working-set fit charges for the RGBA canvas and its demultiply
/// copy; shared so the budget tests derive the same charge the fit does.
pub(in crate::rendering) const RGBA_AND_PNG_BYTES_PER_PIXEL: usize = 12;
/// Fixed per-page allowance the PNG working-set fit reserves for the encoded file's overhead.
pub(in crate::rendering) const PNG_FIXED_OVERHEAD: usize = 256 * 1024;

pub(super) fn fit_png_dpi(
    requested: u32,
    budget: &PngRenderBudget,
    spot_names: Option<&[String]>,
    soft_mask: bool,
    dimensions: impl Fn(u32) -> (u32, u32),
) -> Result<u32> {
    let fits = |dpi| {
        let (width, height) = dimensions(dpi);
        png_working_bytes_fit(width, height, spot_names, soft_mask, budget.max_working_bytes)
    };
    if fits(requested) {
        return Ok(requested);
    }

    let minimum = budget.minimum_dpi.max(1).min(requested);
    if minimum == requested || !fits(minimum) {
        let (width, height) = dimensions(minimum);
        if width == 0 || height == 0 {
            return Err(Error::InvalidPdf("Failed to create pixmap".to_string()));
        }
        return Err(Error::InvalidOperation(format!(
            "PDF raster {width}x{height} exceeds security_limits.max_content_size ({} bytes) or platform allocation bounds",
            budget.max_working_bytes
        )));
    }

    let mut low = minimum;
    let mut high = requested - 1;
    while low < high {
        let middle = low + (high - low) / 2 + 1;
        if fits(middle) {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    Ok(low)
}

pub(super) const MAX_WORKSPACE_DPI_RETRIES: usize = 16;

pub(super) fn dpi_after_workspace_refusal(dpi: u32, minimum: u32, required: usize, limit: usize) -> u32 {
    // Raster workspace normally scales with area. Rounding and later paint phases can
    // change the required bytes, so every attempt still uses the original runtime limit.
    let scaled = (f64::from(dpi) * (limit as f64 / required as f64).sqrt()).floor() as u32;
    scaled.clamp(minimum, dpi - 1)
}

pub(super) fn png_working_bytes_fit(
    width: u32,
    height: u32,
    spot_names: Option<&[String]>,
    soft_mask: bool,
    limit: usize,
) -> bool {
    // The canvas and PNG demultiply copy are both RGBA. Reserve the same per-pixel
    // and fixed PNG allowance as Core image encoding, plus the CMYK plane and two planes per spot ink.
    let sidecar_bytes_per_pixel = spot_names.map_or(Some(0), |names| names.len().checked_mul(2)?.checked_add(4));
    let charge = sidecar_bytes_per_pixel
        .and_then(|sidecar| {
            let copies = 1 + usize::from(soft_mask);
            RGBA_AND_PNG_BYTES_PER_PIXEL.checked_add(sidecar.checked_mul(copies)?)
        })
        .and_then(|bytes| (width as usize).checked_mul(height as usize)?.checked_mul(bytes))
        .and_then(|bytes| bytes.checked_add(PNG_FIXED_OVERHEAD));
    width > 0 && height > 0 && charge.is_some_and(|bytes| bytes <= limit && bytes <= isize::MAX as usize)
}

/// Encode a tiny_skia `Pixmap` to PNG.
///
/// Uses fdeflate (ultra-fast) compression via the `image` crate instead of
/// tiny_skia's built-in `encode_png`, which defaults to flate2 level 6 and is
/// 3–5× slower on typical page images.
pub(super) fn encode_png(pixmap: &Pixmap) -> Result<Vec<u8>> {
    let w = pixmap.width();
    let h = pixmap.height();

    // Demultiply: tiny_skia stores premultiplied RGBA; PNG expects straight alpha. ~keep
    let src = pixmap.data();
    let mut data = src.to_vec();
    for chunk in data.as_chunks_mut::<4>().0 {
        let a = chunk[3];
        if a != 0 && a != 255 {
            let a32 = a as u32;
            chunk[0] = ((chunk[0] as u32 * 255 + a32 / 2) / a32).min(255) as u8;
            chunk[1] = ((chunk[1] as u32 * 255 + a32 / 2) / a32).min(255) as u8;
            chunk[2] = ((chunk[2] as u32 * 255 + a32 / 2) / a32).min(255) as u8;
        }
    }

    use image::ImageEncoder;
    use image::codecs::png::{CompressionType, FilterType, PngEncoder};
    let mut output = Vec::new();
    PngEncoder::new_with_quality(&mut output, CompressionType::Fast, FilterType::Sub)
        .write_image(&data, w, h, image::ExtendedColorType::Rgba8)
        .map_err(|e| Error::InvalidPdf(format!("PNG encoding failed: {}", e)))?;
    Ok(output)
}
