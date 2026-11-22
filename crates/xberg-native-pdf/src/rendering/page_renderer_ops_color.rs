//! Colour-setting operator arms (`sc`, `SC`, `scn`, `SCN`) of the operator loop.

use super::*;

impl PageRenderer {
    pub(super) fn op_set_fill_color(&mut self, st: &mut OpLoopState<'_>, env: OpEnv<'_>, components: &[f32]) {
        let OpEnv { doc, .. } = env;
        let gs = st.gs_stack.current_mut();
        let space_name = gs.fill_color_space.clone();
        let resolved_space = self.color_spaces.get(&space_name);
        gs.fill_color_components.clear();
        gs.fill_color_components.extend_from_slice(components);
        // ISO 32000-1 §8.6.3 + §11.7.4.3: `sc` mutates the
        // current fill colour for the active colour space.
        // Clear any stale CMYK identity left over from a
        // prior DeviceCMYK paint; the DeviceCMYK arm below
        // refills it. Without this clear, a SetFillColor on
        // a non-CMYK space leaves the prior CMYK quadruple
        // visible to the §11.7.4.3 overprint path and
        // corrupts the per-channel B(c_b, c_s) result. ~keep
        gs.fill_color_cmyk = None;

        match space_name.as_str() {
            "DeviceGray" | "G" if !components.is_empty() => {
                let g = components[0];
                gs.fill_color_rgb = (g, g, g);
            }
            "DeviceRGB" | "RGB" if components.len() >= 3 => {
                gs.fill_color_rgb = (components[0], components[1], components[2]);
            }
            "DeviceCMYK" | "CMYK" if components.len() >= 4 => {
                gs.fill_color_rgb = cmyk_to_rgb(components[0], components[1], components[2], components[3]);
                gs.fill_color_cmyk = Some((components[0], components[1], components[2], components[3]));
            }
            // Inline Separation/DeviceN evaluation used to live
            // here as a partial reimplementation of the colour-
            // resolver's tint-transform path. Wave 5 promoted
            // the pipeline to the single source of truth — the
            // pipeline runs the full Type 2 / 3 / 4 evaluator at
            // paint time and splices the resulting RGBA via
            // pipeline_resolve_paint_gs. The dispatcher just
            // records the components on gs.fill_color_components
            // above; the pipeline reads those when the paint op
            // fires. See `resolve_generic_fill_color`. ~keep
            _ => match resolve_generic_fill_color(resolved_space, doc, components) {
                FillColorSpaceResolution::Rgb(r, g, b) => gs.fill_color_rgb = (r, g, b),
                FillColorSpaceResolution::HandledNoChange => {}
                FillColorSpaceResolution::Unhandled => {
                    if !components.is_empty() {
                        let g = components[0];
                        gs.fill_color_rgb = (g, g, g);
                    }
                }
            },
        }
        // Per ISO 32000-1 §8.6.6.4 / §8.6.6.5: when the fill
        // colour space is /Separation or /DeviceN, record the
        // colorant names + tints for the sidecar's per-paint
        // spot lane mirror. Other spaces clear the slot so a
        // subsequent paint does not inherit stale spot data
        // from a prior /Separation set. ~keep
        gs.fill_spot_inks = resolved_space
            .map(|rs| crate::rendering::sidecar::extract_paint_spot_inks(rs, components, doc))
            .unwrap_or_default();
        tracing::trace!(
            "SetFillColor: {} {:?} -> {:?}",
            space_name,
            components,
            gs.fill_color_rgb
        );
    }

    pub(super) fn op_set_stroke_color(&mut self, st: &mut OpLoopState<'_>, env: OpEnv<'_>, components: &[f32]) {
        let OpEnv { doc, .. } = env;
        let gs = st.gs_stack.current_mut();
        let space_name = gs.stroke_color_space.clone();
        let resolved_space = self.color_spaces.get(&space_name);
        gs.stroke_color_components.clear();
        gs.stroke_color_components.extend_from_slice(components);
        gs.stroke_color_cmyk = None;

        match space_name.as_str() {
            "DeviceGray" | "G" if !components.is_empty() => {
                let g = components[0];
                gs.stroke_color_rgb = (g, g, g);
            }
            "DeviceRGB" | "RGB" if components.len() >= 3 => {
                gs.stroke_color_rgb = (components[0], components[1], components[2]);
            }
            "DeviceCMYK" | "CMYK" if components.len() >= 4 => {
                gs.stroke_color_rgb = cmyk_to_rgb(components[0], components[1], components[2], components[3]);
                gs.stroke_color_cmyk = Some((components[0], components[1], components[2], components[3]));
            }
            _ => match resolve_generic_stroke_color(resolved_space, doc, components) {
                Some((r, g, b)) => gs.stroke_color_rgb = (r, g, b),
                None => {
                    if !components.is_empty() {
                        let g = components[0];
                        gs.stroke_color_rgb = (g, g, g);
                    }
                }
            },
        }
        gs.stroke_spot_inks = resolved_space
            .map(|rs| crate::rendering::sidecar::extract_paint_spot_inks(rs, components, doc))
            .unwrap_or_default();
        tracing::trace!(
            "SetStrokeColor: {} {:?} -> {:?}",
            space_name,
            components,
            gs.stroke_color_rgb
        );
    }

    pub(super) fn op_set_fill_color_n(
        &mut self,
        st: &mut OpLoopState<'_>,
        env: OpEnv<'_>,
        components: &[f32],
        name: Option<&str>,
    ) {
        let OpEnv { doc, .. } = env;
        let gs = st.gs_stack.current_mut();
        let space_name = gs.fill_color_space.clone();
        let resolved_space = self.color_spaces.get(&space_name);
        gs.fill_color_components.clear();
        gs.fill_color_components.extend_from_slice(components);
        gs.fill_color_cmyk = None;
        // §8.7.3: retain the pattern name for the Fill path when the
        // active fill space is /Pattern; clear it otherwise so a
        // later device-colour scn cannot paint a stale pattern. ~keep
        gs.fill_pattern_name = if self.is_pattern_space(&space_name) {
            name.map(str::to_string)
        } else {
            None
        };

        match space_name.as_str() {
            "DeviceGray" | "G" if !components.is_empty() => {
                let g = components[0];
                gs.fill_color_rgb = (g, g, g);
            }
            "DeviceRGB" | "RGB" if components.len() >= 3 => {
                gs.fill_color_rgb = (components[0], components[1], components[2]);
            }
            "DeviceCMYK" | "CMYK" if components.len() >= 4 => {
                gs.fill_color_rgb = cmyk_to_rgb(components[0], components[1], components[2], components[3]);
                gs.fill_color_cmyk = Some((components[0], components[1], components[2], components[3]));
            }
            // Pipeline owns the colour for /Separation /
            // /DeviceN and /Indexed at paint time — the
            // dispatcher just records the components above. The
            // §11.7.4.3 CompatibleOverprint CMYK-identity
            // recovery for /Process-attributed /DeviceN is
            // handled inside `resolve_scn_color`. ~keep
            _ => match self.resolve_scn_color(resolved_space, doc, components, &gs.rendering_intent) {
                ScnColorSpaceResolution::Rgb(r, g, b) => gs.fill_color_rgb = (r, g, b),
                ScnColorSpaceResolution::RgbCmyk(rgb, cmyk) => {
                    gs.fill_color_cmyk = Some(cmyk);
                    gs.fill_color_rgb = rgb;
                }
                ScnColorSpaceResolution::HandledNoChange => {}
                ScnColorSpaceResolution::Unhandled => {
                    if !components.is_empty() {
                        let g = components[0];
                        gs.fill_color_rgb = (g, g, g);
                    }
                }
            },
        }
        gs.fill_spot_inks = resolved_space
            .map(|rs| crate::rendering::sidecar::extract_paint_spot_inks(rs, components, doc))
            .unwrap_or_default();
        tracing::trace!(
            "SetFillColorN: {} {:?} -> {:?}",
            space_name,
            components,
            gs.fill_color_rgb
        );
    }

    pub(super) fn op_set_stroke_color_n(&mut self, st: &mut OpLoopState<'_>, env: OpEnv<'_>, components: &[f32]) {
        let OpEnv { doc, .. } = env;
        let gs = st.gs_stack.current_mut();
        let space_name = gs.stroke_color_space.clone();
        let resolved_space = self.color_spaces.get(&space_name);
        gs.stroke_color_components.clear();
        gs.stroke_color_components.extend_from_slice(components);
        gs.stroke_color_cmyk = None;
        match space_name.as_str() {
            "DeviceGray" | "G" if !components.is_empty() => {
                let g = components[0];
                gs.stroke_color_rgb = (g, g, g);
            }
            "DeviceRGB" | "RGB" if components.len() >= 3 => {
                gs.stroke_color_rgb = (components[0], components[1], components[2]);
            }
            "DeviceCMYK" | "CMYK" if components.len() >= 4 => {
                gs.stroke_color_rgb = cmyk_to_rgb(components[0], components[1], components[2], components[3]);
                gs.stroke_color_cmyk = Some((components[0], components[1], components[2], components[3]));
            }
            // Pipeline owns the colour at paint time — see the
            // matching comment in the SetFillColorN arm above.
            // The §11.7.4.3 CompatibleOverprint source-CMYK
            // reconstruction for /Process-attributed /DeviceN
            // runs the same way as the fill side via
            // `resolve_scn_color`. ~keep
            _ => match self.resolve_scn_color(resolved_space, doc, components, &gs.rendering_intent) {
                ScnColorSpaceResolution::Rgb(r, g, b) => gs.stroke_color_rgb = (r, g, b),
                ScnColorSpaceResolution::RgbCmyk(rgb, cmyk) => {
                    gs.stroke_color_cmyk = Some(cmyk);
                    gs.stroke_color_rgb = rgb;
                }
                ScnColorSpaceResolution::HandledNoChange => {}
                ScnColorSpaceResolution::Unhandled => {
                    if !components.is_empty() {
                        let g = components[0];
                        gs.stroke_color_rgb = (g, g, g);
                    }
                }
            },
        }
        gs.stroke_spot_inks = resolved_space
            .map(|rs| crate::rendering::sidecar::extract_paint_spot_inks(rs, components, doc))
            .unwrap_or_default();
        tracing::trace!(
            "SetStrokeColorN: {} {:?} -> {:?}",
            space_name,
            components,
            gs.stroke_color_rgb
        );
    }
}
