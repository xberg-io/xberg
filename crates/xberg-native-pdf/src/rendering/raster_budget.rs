//! Page raster sizing, working-memory checks, and PNG encoding.

use super::{PageRenderer, knockout, knockout_resources};
use crate::document::PdfDocument;
use crate::error::{Error, Result};
use crate::object::{Object, ObjectRef};
use crate::rendering::ext_gstate::parse_ext_g_state_inner;
use std::collections::HashMap;
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

/// Finished scans by form reference, invocation kind (mask or XObject), and depth.
/// Including depth makes cyclic paths reach the conservative cap and lets shared forms reuse
/// results without bypassing the remaining depth limit.
type MemoOfSpotPlaneUse = HashMap<(ObjectRef, bool, u32), SpotPlaneUse>;

/// What painting a page can hold beyond the live sidecar planes, read from its resources.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::rendering) struct SpotPlaneUse {
    /// An ExtGState soft mask (`smask_spot_snapshot`).
    pub(in crate::rendering) soft_mask: bool,
    /// Deepest nesting of transparency groups, knockout groups included.
    pub(in crate::rendering) groups: usize,
    /// Deepest nesting of knockout groups.
    pub(in crate::rendering) knockouts: usize,
}

impl SpotPlaneUse {
    fn nested_in_group(self, is_group: bool, is_knockout: bool) -> Self {
        Self {
            soft_mask: self.soft_mask,
            groups: self.groups + usize::from(is_group),
            knockouts: self.knockouts + usize::from(is_knockout),
        }
    }

    fn widest(self, other: Self) -> Self {
        Self {
            soft_mask: self.soft_mask || other.soft_mask,
            groups: self.groups.max(other.groups),
            knockouts: self.knockouts.max(other.knockouts),
        }
    }
}

/// Scans the page and the resources a form can reach (ExtGState and XObject only; pattern and
/// annotation appearance resources are not walked, and the runtime reservations in
/// `knockout_resources` still bound them). A scan that hits the depth cap answers conservatively.
pub(in crate::rendering) fn spot_plane_use(doc: &PdfDocument, page_num: usize, resources: &Object) -> SpotPlaneUse {
    let page_knockout = knockout::page_is_knockout(doc, page_num);
    scan_resources_for_spot_plane_use(doc, resources, &mut HashMap::new(), 0)
        .nested_in_group(page_knockout, page_knockout)
}

/// The `/G` forms of the ExtGState soft masks the resources declare.
fn soft_mask_forms(doc: &PdfDocument, dict: &HashMap<String, Object>) -> Vec<ObjectRef> {
    use crate::rendering::ext_gstate::SoftMaskValue;
    let states = dict
        .get("ExtGState")
        .and_then(|object| doc.resolve_object(object).ok())
        .and_then(|object| object.as_dict().cloned());
    states
        .iter()
        .flat_map(|states| states.values())
        .filter_map(|state| match parse_ext_g_state_inner(state, doc).ok()?.smask {
            Some(SoftMaskValue::Form(form)) => Some(form.form_ref),
            _ => None,
        })
        .collect()
}

fn scan_soft_mask_forms(
    doc: &PdfDocument,
    forms: &[ObjectRef],
    memo: &mut MemoOfSpotPlaneUse,
    depth: u32,
) -> SpotPlaneUse {
    let mut found = SpotPlaneUse::default();
    for &reference in forms {
        let key = (reference, true, depth);
        let scanned = if let Some(&known) = memo.get(&key) {
            known
        } else {
            let scanned = match doc.load_object(reference) {
                Ok(Object::Stream { dict, .. }) => scan_form_for_spot_plane_use(doc, &dict, memo, depth, true),
                _ => SpotPlaneUse::default(),
            };
            memo.insert(key, scanned);
            scanned
        };
        found = found.widest(scanned);
    }
    found
}

fn scan_form_for_spot_plane_use(
    doc: &PdfDocument,
    form: &HashMap<String, Object>,
    memo: &mut MemoOfSpotPlaneUse,
    depth: u32,
    as_mask: bool,
) -> SpotPlaneUse {
    let group = form.get("Group").and_then(|group| doc.resolve_object(group).ok());
    let is_knockout = group.as_ref().is_some_and(knockout::group_is_knockout);
    let is_group = is_knockout
        || group
            .as_ref()
            .and_then(Object::as_dict)
            .is_some_and(|group| group.get("S").and_then(Object::as_name) == Some("Transparency"));
    let inner = match form.get("Resources").map(|object| doc.resolve_object(object)) {
        Some(Ok(resources)) => scan_resources_for_spot_plane_use(doc, &resources, memo, depth + 1),
        _ => SpotPlaneUse::default(),
    };
    // The soft-mask allowance covers a plain mask group by itself. Count its level when
    // child groups can hold reservations concurrently, or when it has a knockout workspace. ~keep
    inner.nested_in_group(is_group && (!as_mask || inner.groups != 0 || is_knockout), is_knockout)
}

fn scan_resources_for_spot_plane_use(
    doc: &PdfDocument,
    resources: &Object,
    memo: &mut MemoOfSpotPlaneUse,
    depth: u32,
) -> SpotPlaneUse {
    let Some(dict) = resources.as_dict() else {
        return SpotPlaneUse::default();
    };
    if depth >= knockout_resources::MAX_NESTING_DEPTH as u32 {
        // Past the nesting cap the renderer refuses further groups; answer with the cap. ~keep
        return SpotPlaneUse {
            soft_mask: true,
            groups: knockout_resources::MAX_NESTING_DEPTH,
            knockouts: knockout_resources::MAX_NESTING_DEPTH,
        };
    }
    let mask_forms = soft_mask_forms(doc, dict);
    let mut found = SpotPlaneUse {
        soft_mask: !mask_forms.is_empty(),
        ..SpotPlaneUse::default()
    }
    .widest(scan_soft_mask_forms(doc, &mask_forms, memo, depth));
    let xobjects = dict
        .get("XObject")
        .and_then(|object| doc.resolve_object(object).ok())
        .and_then(|object| object.as_dict().cloned());
    for raw in xobjects.iter().flat_map(|forms| forms.values()) {
        let key = raw.as_reference().map(|reference| (reference, false, depth));
        if let Some(known) = key.and_then(|key| memo.get(&key)) {
            found = found.widest(*known);
            continue;
        }
        let Ok(Object::Stream { dict: form, .. }) = doc.resolve_object(raw) else {
            continue;
        };
        if form.get("Subtype").and_then(Object::as_name) != Some("Form") {
            continue;
        }
        let scanned = scan_form_for_spot_plane_use(doc, &form, memo, depth, false);
        if let Some(key) = key {
            memo.insert(key, scanned);
        }
        found = found.widest(scanned);
    }
    found
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
    transient: SpotPlaneUse,
    dimensions: impl Fn(u32) -> (u32, u32),
) -> Result<u32> {
    let fits = |dpi| {
        let (width, height) = dimensions(dpi);
        png_working_bytes_fit(width, height, spot_names, transient, budget.max_working_bytes)
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

/// Per-pixel bytes the render holds at its peak for the spot planes `transient` declares, from
/// the reservations in `knockout_resources`; zero when the page holds nothing beyond the live
/// planes. `sidecar` is the per-pixel size of the CMYK and spot planes.
/// The run-time reservations in `knockout_resources` peak at the canvas plus one soft mask
/// snapshot when only a mask paints; inside groups, at the canvas, one rgba + sidecar copy per
/// nested group and one for non-isolated source recovery, per nested knockout group its `knockout()` reservation (twice rgba +
/// sidecar, twice the shape) and shape workspace, and the widest per-paint scratch. The
/// shape workspace is charged even where no knockout shape exists (an over-charge). A soft
/// mask inside a group adds its mask workspace, mask backdrop, paint snapshots and the mask
/// form's own group. The PNG encode copies come after rendering, so the charge is the larger.
fn render_peak_bytes_per_pixel(sidecar: usize, ink_count: usize, transient: SpotPlaneUse) -> Option<usize> {
    let canvas = sidecar.checked_add(4)?;
    let shape = ink_count.checked_add(1)?;
    let groups = transient.groups.min(knockout_resources::MAX_NESTING_DEPTH);
    let knockouts = transient.knockouts.min(groups);
    if groups == 0 {
        return Some(if transient.soft_mask {
            RGBA_AND_PNG_BYTES_PER_PIXEL.checked_add(sidecar.checked_mul(2)?)?
        } else {
            0
        });
    }
    // One more canvas for the non-isolated source recovery (`recover_nonisolated_source`). ~keep
    let nested_groups = canvas.checked_mul(groups.checked_add(2)?)?;
    let knockout_workspace = canvas
        .checked_mul(2)?
        .checked_add(shape.checked_mul(3)?)?
        .checked_mul(knockouts)?;
    let soft_mask = if transient.soft_mask {
        // Mask workspace (2 rgba), backdrop (rgba), paint snapshots and the mask form's group. ~keep
        4usize
            .checked_mul(3)?
            .checked_add(canvas)?
            .checked_add(shape)?
            .checked_add(canvas)?
    } else {
        0
    };
    nested_groups
        .checked_add(knockout_workspace)?
        .checked_add(knockout::MAX_PAINT_SCRATCH_BYTES_PER_PIXEL)?
        .checked_add(soft_mask)
}

pub(super) fn png_working_bytes_fit(
    width: u32,
    height: u32,
    spot_names: Option<&[String]>,
    transient: SpotPlaneUse,
    limit: usize,
) -> bool {
    // The canvas and PNG demultiply copy are both RGBA. Reserve the same per-pixel
    // and fixed PNG allowance as Core image encoding, plus the CMYK plane and two planes per spot ink.
    let sidecar_bytes_per_pixel = spot_names.map_or(Some(0), |names| names.len().checked_mul(2)?.checked_add(4));
    let live_bytes_per_pixel =
        sidecar_bytes_per_pixel.and_then(|sidecar| RGBA_AND_PNG_BYTES_PER_PIXEL.checked_add(sidecar));
    let render_peak_bytes_per_pixel = sidecar_bytes_per_pixel
        .and_then(|sidecar| render_peak_bytes_per_pixel(sidecar, spot_names.map_or(0, <[String]>::len), transient));
    let charge = live_bytes_per_pixel
        .and_then(|live| Some(live.max(render_peak_bytes_per_pixel?)))
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
