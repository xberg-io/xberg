//! Device colors and initial color-space values for the page operator loop.

use super::{COLOR_OPERATOR_TRACE_TARGET, cmyk_to_rgb, sidecar_mod};
use crate::content::graphics_state::GraphicsState;
use crate::document::PdfDocument;
use crate::object::Object;
use crate::rendering::resolution::IccTransformCache;
use std::collections::HashMap;

pub(super) struct ColorSpaceContext<'a> {
    pub spaces: &'a HashMap<String, Object>,
    pub doc: &'a PdfDocument,
    pub transforms: &'a IccTransformCache,
}

pub(super) fn set_fill_rgb(gs: &mut GraphicsState, r: f32, g: f32, b: f32) {
    gs.fill_color_rgb = (r, g, b);
    gs.fill_color_space = "DeviceRGB".to_string();
    gs.fill_color_components.clear();
    gs.fill_color_components.extend_from_slice(&[r, g, b]);
    // Device-family fill paint: per §11.7.3 the source
    // covers only the process channels, so any spot ink
    // identity recorded by a prior /Separation or
    // /DeviceN paint is no longer the active source.
    // The sidecar's per-paint spot mirror reads this
    // empty list as "no spot lane writes for this paint". ~keep
    gs.fill_spot_inks.clear();
    // ISO 32000-1 §8.6.3: the fill colour and colour
    // space are coupled — switching to /DeviceRGB
    // invalidates any prior /DeviceCMYK identity. Failing
    // to clear `fill_color_cmyk` here means the §11.7.4.3
    // overprint path would still see the prior paint's
    // CMYK quadruple as the "current source colour",
    // producing wrong B(c_b, c_s) = c_s values for the
    // new RGB paint's region. ~keep
    gs.fill_color_cmyk = None;
    tracing::trace!(target: COLOR_OPERATOR_TRACE_TARGET, "SetFillRgb: [{}, {}, {}]", r, g, b);
}

pub(super) fn set_stroke_rgb(gs: &mut GraphicsState, r: f32, g: f32, b: f32) {
    gs.stroke_color_rgb = (r, g, b);
    gs.stroke_color_space = "DeviceRGB".to_string();
    gs.stroke_color_components.clear();
    gs.stroke_color_components.extend_from_slice(&[r, g, b]);
    gs.stroke_spot_inks.clear();
    gs.stroke_color_cmyk = None;
    tracing::trace!(target: COLOR_OPERATOR_TRACE_TARGET, "SetStrokeRgb: [{}, {}, {}]", r, g, b);
}

pub(super) fn set_fill_gray(gs: &mut GraphicsState, gray: f32) {
    let g = gray;
    gs.fill_color_rgb = (g, g, g);
    gs.fill_color_space = "DeviceGray".to_string();
    gs.fill_color_components.clear();
    gs.fill_color_components.push(g);
    gs.fill_spot_inks.clear();
    gs.fill_color_cmyk = None;
    tracing::trace!(target: COLOR_OPERATOR_TRACE_TARGET, "SetFillGray: {}", g);
}

pub(super) fn set_stroke_gray(gs: &mut GraphicsState, gray: f32) {
    let g = gray;
    gs.stroke_color_rgb = (g, g, g);
    gs.stroke_color_space = "DeviceGray".to_string();
    gs.stroke_color_components.clear();
    gs.stroke_color_components.push(g);
    gs.stroke_spot_inks.clear();
    gs.stroke_color_cmyk = None;
    tracing::trace!(target: COLOR_OPERATOR_TRACE_TARGET, "SetStrokeGray: {}", g);
}

pub(super) fn set_fill_cmyk(gs: &mut GraphicsState, c: f32, m: f32, y: f32, k: f32) {
    let (r, g, b) = cmyk_to_rgb(c, m, y, k);
    gs.fill_color_rgb = (r, g, b);
    gs.fill_color_cmyk = Some((c, m, y, k));
    gs.fill_color_space = "DeviceCMYK".to_string();
    gs.fill_color_components.clear();
    gs.fill_color_components.extend_from_slice(&[c, m, y, k]);
    gs.fill_spot_inks.clear();
    tracing::trace!(target: COLOR_OPERATOR_TRACE_TARGET, "SetFillCmyk: [{}, {}, {}, {}] -> {:?}", c, m, y, k, (r, g, b));
}

pub(super) fn set_stroke_cmyk(gs: &mut GraphicsState, c: f32, m: f32, y: f32, k: f32) {
    let (r, g, b) = cmyk_to_rgb(c, m, y, k);
    gs.stroke_color_rgb = (r, g, b);
    gs.stroke_color_cmyk = Some((c, m, y, k));
    gs.stroke_color_space = "DeviceCMYK".to_string();
    gs.stroke_color_components.clear();
    gs.stroke_color_components.extend_from_slice(&[c, m, y, k]);
    gs.stroke_spot_inks.clear();
    tracing::trace!(target: COLOR_OPERATOR_TRACE_TARGET, "SetStrokeCmyk: [{}, {}, {}, {}] -> {:?}", c, m, y, k, (r, g, b));
}

pub(super) fn set_fill_color_space(gs: &mut GraphicsState, name: &str, context: ColorSpaceContext<'_>) {
    // ISO 32000-1 §8.6.8: the `cs` operator shall also
    // set the current colour to its initial value, which
    // depends on the colour space. For Separation /
    // DeviceN the initial tint is 1.0 per colorant
    // (§8.6.6.4 / §8.6.6.5); for DeviceCMYK the initial
    // colour is (0, 0, 0, 1); device-family RGB / Gray
    // start at all-zeros. Failing to reset the colour
    // here means a paint after `cs /CS_B` without an
    // intervening `scn` would carry the prior space's
    // identity and tint, including its spot ink list —
    // round 2 QA pinned that the spot mirror would then
    // write the prior /CS_A's spot lane. ~keep
    let resolved = context.spaces.get(name).cloned();
    // §10.7.3: the §8.6.8 initial-colour evaluation runs an
    // ICC retarget for DeviceN /Process /ICCBased; thread
    // the live gs intent through so a prior `/Perceptual ri`
    // / ExtGState /RI propagates into the retarget tag pick. ~keep
    let intent_for_initial = crate::color::RenderingIntent::from_pdf_name(&gs.rendering_intent);
    let initial = sidecar_mod::initial_colour_for_space(
        name,
        resolved.as_ref(),
        context.doc,
        intent_for_initial,
        Some(context.transforms),
    );
    gs.fill_color_space = name.to_owned();
    gs.fill_color_rgb = initial.rgb;
    gs.fill_color_cmyk = initial.cmyk;
    gs.fill_color_components.clear();
    gs.fill_color_components.extend_from_slice(&initial.components);
    gs.fill_spot_inks = initial.spot_inks;
    // Selecting a colour space clears any previously selected
    // fill pattern; a fresh scn must re-name it (§8.7.3). ~keep
    gs.fill_pattern_name = None;
    tracing::trace!(target: COLOR_OPERATOR_TRACE_TARGET, "SetFillColorSpace: {}", name);
}

pub(super) fn set_stroke_color_space(gs: &mut GraphicsState, name: &str, context: ColorSpaceContext<'_>) {
    let resolved = context.spaces.get(name).cloned();
    let intent_for_initial = crate::color::RenderingIntent::from_pdf_name(&gs.rendering_intent);
    let initial = sidecar_mod::initial_colour_for_space(
        name,
        resolved.as_ref(),
        context.doc,
        intent_for_initial,
        Some(context.transforms),
    );
    gs.stroke_color_space = name.to_owned();
    gs.stroke_color_rgb = initial.rgb;
    gs.stroke_color_cmyk = initial.cmyk;
    gs.stroke_color_components.clear();
    gs.stroke_color_components.extend_from_slice(&initial.components);
    gs.stroke_spot_inks = initial.spot_inks;
}
