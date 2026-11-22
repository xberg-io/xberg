//! Colour-resolution stage.
//!
//! This is the stage where capabilities that previously could not reach the
//! renderer are wired in:
//!
//! - **PostScript Type 4 calculator** tint transforms ([`crate::functions`]).
//!   Resolves `Separation` and `DeviceN` colour spaces whose `tintTransform`
//!   is a Type-4 function — the case the inline match arm at
//!   `page_renderer.rs:629-693` falls back to `1.0 - tint` for.
//! - **Type 2 exponential interpolation** tint transforms. Spec
//!   ISO 32000-1:2008 §7.10.3. The existing inline match arm handles this
//!   for `DeviceCMYK` alternate spaces only; the resolver handles `DeviceRGB`
//!   and `DeviceGray` alternates as well.
//! - **ICCBased** colour spaces. The resolver delegates to the
//!   [`crate::color::Transform`] CMM and falls
//!   back to the process-ink conversion otherwise. This is the same
//!   path image extraction uses, so we re-use [`crate::color`] rather than
//!   carrying a second copy of the conversion code.
//! - **Indexed** colour spaces. The resolver follows the index into the base
//!   space; for now we handle DeviceGray / DeviceRGB / DeviceCMYK base spaces
//!   and fall back to grayscale otherwise (matching the existing renderer).
//!
//! The output is a [`ResolvedColor::Rgba`] for composite consumers; a
//! follow-up branch will add the `Cmyk` and `PerChannel` variants behind the
//! same resolver entry point so separation backends share the same call.

use crate::error::Result;
use crate::object::Object;

use super::context::ResolutionContext;
use super::intent::{DeviceColor, LogicalColor};
use super::resolved::ResolvedColor;

/// Colour-resolution stage.
///
/// Stateless — the resolver is purely a function of `(LogicalColor,
/// ResolutionContext, gs.fill_alpha-or-stroke_alpha)`. The struct exists so
/// the pipeline can grow per-instance state later (e.g. a cache of compiled
/// Type-4 [`crate::functions::Program`] keyed by stream object id) without
/// changing the call surface.
pub(crate) struct ColorResolver;

impl ColorResolver {
    pub(crate) const fn new() -> Self {
        Self
    }

    /// Resolve `color` into an RGBA value the composite backend can paint.
    ///
    /// `alpha` is the pre-computed straight alpha from the graphics state
    /// (i.e. `gs.fill_alpha` for fill intents, `gs.stroke_alpha` for stroke
    /// intents). Folding it in here keeps backends simple.
    pub(crate) fn resolve(&self, color: &LogicalColor, ctx: &ResolutionContext, alpha: f32) -> Result<ResolvedColor> {
        match color {
            LogicalColor::Device(dev) => {
                // ISO 32000-1:2008 §8.6.5.6: when the page declares a
                // /DefaultGray, /DefaultRGB, or /DefaultCMYK entry in
                // its /Resources /ColorSpace dict, any bare device-family
                // paint operator (the canonical `g`/`rg`/`k`/`K` and
                // their stroking siblings) MUST be interpreted as if it
                // had named the override colour space instead of the
                // device family. The override therefore takes
                // precedence over the document /OutputIntents profile
                // for bare device paint — OutputIntent is only the
                // fallback default when no override has been declared. ~keep
                if let Some(resolved) = self.resolve_device_default_override(*dev, ctx, alpha)? {
                    return Ok(resolved);
                }
                Ok(device_to_rgba(*dev, alpha))
            }
            LogicalColor::Spaced { space, components } => self.resolve_spaced(space, components, ctx, alpha),
        }
    }

    /// §8.6.5.6 dispatch for bare device-family paint. Returns `Some`
    /// when the active page has declared a matching `/Default<Family>`
    /// override AND that override resolves successfully; otherwise
    /// returns `None` so the caller emits the device-family default.
    ///
    /// The override is resolved by recursively calling `resolve_spaced`
    /// on the override object with the original paint components. That
    /// reuses the existing colour-space machinery (ICCBased N=3/N=4,
    /// Separation, DeviceN, …) so a `/DefaultCMYK [/ICCBased ...]`
    /// override goes through the embedded-ICC path, picks up the
    /// per-page transform cache via `ctx.icc_transform_cache`, and
    /// emits `ResolvedColor::IccCmyk` exactly as for an explicit
    /// `[/ICCBased N=4]` colour space paint.
    ///
    /// Precedence note: this fires BEFORE the OutputIntent-aware CMYK
    /// projection at `cmyk_to_rgb_via_intent` because the override is
    /// the page's declared colour space and OutputIntent only fills
    /// in for the device family when no override is present.
    fn resolve_device_default_override(
        &self,
        dev: DeviceColor,
        ctx: &ResolutionContext,
        alpha: f32,
    ) -> Result<Option<ResolvedColor>> {
        let (override_obj, components): (Option<&Object>, smallvec::SmallVec<[f32; 4]>) = match dev {
            DeviceColor::Gray(g) => (ctx.default_gray, smallvec::smallvec![g]),
            DeviceColor::Rgb(r, g, b) => (ctx.default_rgb, smallvec::smallvec![r, g, b]),
            DeviceColor::Cmyk(c, m, y, k) => (ctx.default_cmyk, smallvec::smallvec![c, m, y, k]),
        };
        let Some(space) = override_obj else {
            return Ok(None);
        };

        // §8.6.5.6 requires the override entry to be a colour space:
        // either a Name (device-family alias such as `/DeviceCMYK`,
        // `/CalGray`) or an Array (`[/ICCBased ...]`, `[/Separation
        // ...]`, etc.). A malformed entry (string, integer, bool,
        // dictionary…) is structurally indistinguishable from the
        // entry being absent — honouring it would silently
        // mis-render through `resolve_spaced`'s `first_as_gray`
        // catch-all (a quarter-tint CMYK paint coming out as 25%
        // gray is worse than the spec-fallback / OutputIntent
        // render). Return None so the caller falls through to the
        // device-family path (`device_to_rgba`), which routes CMYK
        // through `cmyk_to_rgb_via_intent` and so consults
        // `/OutputIntents` when present, or the process-ink conversion
        // when not. ~keep
        if space.as_name().is_none() && space.as_array().is_none() {
            return Ok(None);
        }

        // The override resolves via the same colour-space pipeline
        // as an explicit `cs <space>` paint — that's the whole point
        // of §8.6.5.6: the override colour space stands in for the
        // device family. If the override object is just another Name
        // (e.g. `/DefaultCMYK /DeviceCMYK`, an identity declaration),
        // resolve_spaced's Name arm folds back to the device-family
        // default — returning Some is still correct because we've
        // honoured the override; it just produces the same value as
        // the no-override path. ~keep
        Ok(Some(self.resolve_spaced(space, &components, ctx, alpha)?))
    }

    fn resolve_spaced(
        &self,
        space: &Object,
        components: &[f32],
        ctx: &ResolutionContext,
        alpha: f32,
    ) -> Result<ResolvedColor> {
        // A `Name` here means a device family — the operator dispatcher
        // already folded those into LogicalColor::Device for the canonical
        // `g`/`rg`/`k`/`K` operators, but `SCN` against a Device* alias
        // still reaches us this way. ~keep
        if let Some(name) = space.as_name() {
            return Ok(resolve_device_alias(name, components, alpha));
        }

        let Some(arr) = space.as_array() else {
            // Unknown space shape — fall back to first-component-as-gray,
            // matching the existing inline behaviour at
            // `page_renderer.rs:709-712`. ~keep
            return Ok(first_as_gray(components, alpha));
        };

        let Some(type_name) = arr.first().and_then(|o| o.as_name()) else {
            return Ok(first_as_gray(components, alpha));
        };

        match type_name {
            "DeviceGray" | "G" | "CalGray" => Ok(first_as_gray(components, alpha)),
            "DeviceRGB" | "RGB" => Ok(three_as_rgb(components, alpha)),
            "CalRGB" => Ok(resolve_calrgb(arr, components, ctx, alpha)),
            "DeviceCMYK" | "CMYK" => Ok(four_as_cmyk_native(components, alpha)),
            "ICCBased" => self.resolve_iccbased(arr, components, ctx, alpha),
            "Separation" | "DeviceN" => self.resolve_separation_or_devicen(arr, components, ctx, alpha),
            "Indexed" => self.resolve_indexed(arr, components, ctx, alpha),
            _ => Ok(first_as_gray(components, alpha)),
        }
    }

    fn resolve_iccbased(
        &self,
        arr: &[Object],
        components: &[f32],
        ctx: &ResolutionContext,
        alpha: f32,
    ) -> Result<ResolvedColor> {
        // ICCBased array shape: [/ICCBased <stream-ref>]. The stream dict
        // carries /N indicating the input component count. ~keep
        let Some(stream_obj) = arr.get(1) else {
            return Ok(first_as_gray(components, alpha));
        };
        let resolved_stream = match ctx.doc.resolve_object(stream_obj) {
            Ok(o) => o,
            Err(_) => return Ok(first_as_gray(components, alpha)),
        };
        let Some(dict) = resolved_stream.as_dict() else {
            return Ok(first_as_gray(components, alpha));
        };
        let n = dict.get("N").and_then(|o| o.as_integer()).unwrap_or(3);

        // §8.6.5.5 precedence: an ICCBased colour space carries its own
        // conversion source. The embedded profile wins over the document
        // /OutputIntents profile when CMYK→RGB is requested. Decode the
        // stream, parse the bytes through IccProfile::parse (which
        // cross-checks the dict's /N against the ICC header signature),
        // and compile a qcms Transform against the active rendering
        // intent. On any failure (decode error, mismatched header,
        // qcms refusal) we fall through to the
        // device-family path — that path emits ResolvedColor::Cmyk for
        // N=4, which the composite projection then converts through
        // ctx.output_intent_cmyk: the document OutputIntent becomes the
        // default when the embedded profile can't actually drive a CMM.
        //
        // We emit the dual-payload `IccCmyk` variant so the per-plate
        // router still sees the four channel decomposition. The composite
        // backend reads the pre-computed RGB; the separation backend
        // reads the original CMYK quadruple. The ICC conversion is a
        // composite-surface concern — the plates ARE the press-target
        // ink coverage, so dropping the CMYK channel values for a
        // monolithic Rgba would zero out every plate. ~keep
        if n == 4
            && components.len() >= 4
            && let Ok(bytes) = resolved_stream.decode_stream_data()
            && let Some(profile) = crate::color::IccProfile::parse(bytes, 4)
        {
            let profile = std::sync::Arc::new(profile);
            // Per-page transform cache keyed on profile content
            // hash + intent (see IccTransformCache). The
            // embedded /ICCBased profile is parsed afresh on
            // every paint operator (the decode + parse happens
            // above), but the qcms CMM is the heavy bit and
            // gets reused across paints whose ICCBased stream
            // hashes identically. Unit tests skip the cache
            // (ctx.icc_transform_cache is None) and pay the
            // per-call build cost. ~keep
            let transform: std::sync::Arc<crate::color::Transform> = if let Some(cache) = ctx.icc_transform_cache {
                cache.get_or_build(&profile, ctx.rendering_intent)
            } else {
                std::sync::Arc::new(crate::color::Transform::new_srgb_target(
                    std::sync::Arc::clone(&profile),
                    ctx.rendering_intent,
                ))
            };
            if transform.has_cmm() {
                let c = components[0].clamp(0.0, 1.0);
                let m = components[1].clamp(0.0, 1.0);
                let y = components[2].clamp(0.0, 1.0);
                let k = components[3].clamp(0.0, 1.0);
                let c_u8 = (c * 255.0).round() as u8;
                let m_u8 = (m * 255.0).round() as u8;
                let y_u8 = (y * 255.0).round() as u8;
                let k_u8 = (k * 255.0).round() as u8;
                let rgb = transform.convert_cmyk_pixel(c_u8, m_u8, y_u8, k_u8);
                return Ok(ResolvedColor::IccCmyk {
                    r: rgb[0] as f32 / 255.0,
                    g: rgb[1] as f32 / 255.0,
                    b: rgb[2] as f32 / 255.0,
                    c,
                    m,
                    y,
                    k,
                    a: alpha,
                });
            }
        }

        // ICCBased N=3 — RGB source profile. The embedded profile
        // drives the conversion (§8.6.5.5); the §10.3.5 fallback only
        // fires when qcms refuses to compile the profile. This branch
        // is also the path the §8.6.5.6 /DefaultRGB override consumes:
        // declaring `/DefaultRGB [/ICCBased <N=3 stream>]` and painting
        // bare /DeviceRGB sends the three components through this arm.
        //
        // No per-plate routing complication here — RGB never lands on
        // CMYK plates — so we emit ResolvedColor::Rgba directly. The
        // per-page transform cache (originally introduced for CMYK,
        // but n_components-agnostic at the key level — see
        // `IccTransformCache` docstring) is consulted here too: an
        // /ICCBased N=3 profile used by a /DefaultRGB override gets
        // hit by every bare /DeviceRGB paint on the page, so caching
        // the compiled qcms transform pays back for the same reason
        // the CMYK arm above does. ~keep
        if n == 3
            && components.len() >= 3
            && let Ok(bytes) = resolved_stream.decode_stream_data()
            && let Some(profile) = crate::color::IccProfile::parse(bytes, 3)
        {
            let profile = std::sync::Arc::new(profile);
            let transform: std::sync::Arc<crate::color::Transform> = if let Some(cache) = ctx.icc_transform_cache {
                cache.get_or_build(&profile, ctx.rendering_intent)
            } else {
                std::sync::Arc::new(crate::color::Transform::new_srgb_target(
                    std::sync::Arc::clone(&profile),
                    ctx.rendering_intent,
                ))
            };
            if transform.has_cmm() {
                let r = components[0].clamp(0.0, 1.0);
                let g = components[1].clamp(0.0, 1.0);
                let b = components[2].clamp(0.0, 1.0);
                let r_u8 = (r * 255.0).round() as u8;
                let g_u8 = (g * 255.0).round() as u8;
                let b_u8 = (b * 255.0).round() as u8;
                let rgb = transform.convert_rgb_buffer(&[r_u8, g_u8, b_u8]);
                if rgb.len() >= 3 {
                    return Ok(ResolvedColor::Rgba {
                        r: rgb[0] as f32 / 255.0,
                        g: rgb[1] as f32 / 255.0,
                        b: rgb[2] as f32 / 255.0,
                        a: alpha,
                    });
                }
            }
        }

        // ICCBased N=1 — Gray source profile. The embedded profile
        // drives the conversion (§8.6.5.5) and is the path
        // /DefaultGray [/ICCBased <N=1 TRC stream>] consumes for bare
        // /DeviceGray paint. qcms 0.3.0 reads Gray ICC profiles via
        // the `kTRC` (gray Tone Reproduction Curve) tag —
        // `iccread.rs:1712-1714` — and runs a dedicated
        // gray-to-RGB transform path at `transform.rs:437-475`. The
        // input is one byte, the output is three RGB bytes; we read
        // the first three of `convert_gray_buffer`'s output.
        //
        // No per-plate routing complication — a Gray override emits
        // a single ink and lands on the K plate via the InkRouter's
        // gray-as-K handling; the composite RGB is what consumers
        // see, so ResolvedColor::Rgba is the right variant. The
        // per-page transform cache is consulted exactly as for N=3
        // and N=4 — the key is (profile.content_hash(), intent), no
        // n_components in the key, so the same cache amortises Gray
        // ICC alongside RGB and CMYK. ~keep
        if n == 1
            && !components.is_empty()
            && let Ok(bytes) = resolved_stream.decode_stream_data()
            && let Some(profile) = crate::color::IccProfile::parse(bytes, 1)
        {
            let profile = std::sync::Arc::new(profile);
            let transform: std::sync::Arc<crate::color::Transform> = if let Some(cache) = ctx.icc_transform_cache {
                cache.get_or_build(&profile, ctx.rendering_intent)
            } else {
                std::sync::Arc::new(crate::color::Transform::new_srgb_target(
                    std::sync::Arc::clone(&profile),
                    ctx.rendering_intent,
                ))
            };
            if transform.has_cmm() {
                let g = components[0].clamp(0.0, 1.0);
                let g_u8 = (g * 255.0).round() as u8;
                let rgb = transform.convert_gray_buffer(&[g_u8]);
                if rgb.len() >= 3 {
                    return Ok(ResolvedColor::Rgba {
                        r: rgb[0] as f32 / 255.0,
                        g: rgb[1] as f32 / 255.0,
                        b: rgb[2] as f32 / 255.0,
                        a: alpha,
                    });
                }
            }
        }

        // No usable embedded profile — fall through to the device-family
        // hint. For N=4 this emits ResolvedColor::Cmyk so per-plate
        // backends still see the channel decomposition, and the
        // composite projection routes through ctx.output_intent_cmyk
        // (which is the spec default when no embedded ICC is available). ~keep
        match n {
            1 => Ok(first_as_gray(components, alpha)),
            3 => Ok(three_as_rgb(components, alpha)),
            4 => Ok(four_as_cmyk_native(components, alpha)),
            _ => Ok(first_as_gray(components, alpha)),
        }
    }

    /// Resolve `Separation` and `DeviceN` colour spaces by evaluating the
    /// tint transform.
    ///
    /// Array shape: `[/Separation name altCS tintTransform]` or
    /// `[/DeviceN names altCS tintTransform attrs?]`. The tint transform is
    /// a PDF function dict whose `FunctionType` selects:
    ///
    /// - **Type 0** (sampled): N-dimensional multilinear interpolation over
    ///   the sample grid (see [`evaluate_type0_sampled`]) — handles both the
    ///   common 1-input Separation shape and genuinely multi-channel
    ///   DeviceN tint transforms.
    /// - **Type 2** (exponential): closed-form interpolation between `/C0`
    ///   and `/C1` with exponent `/N`. The existing inline path only handles
    ///   `N=1` against `DeviceCMYK` altCS; we generalise to any `N` and to
    ///   `DeviceRGB`/`DeviceGray` altCS as well.
    /// - **Type 3** (stitching): single-input by spec; picks the matching
    ///   sub-function and delegates.
    /// - **Type 4** (calculator): evaluated via [`crate::functions::Program`].
    fn resolve_separation_or_devicen(
        &self,
        arr: &[Object],
        components: &[f32],
        ctx: &ResolutionContext,
        alpha: f32,
    ) -> Result<ResolvedColor> {
        if components.is_empty() {
            return Ok(ResolvedColor::Rgba {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: alpha,
            });
        }

        // §8.6.6.3 reserved name: `/None` produces no visible output.
        // For composite output we emit a fully-transparent RGBA — the
        // splice carries it through as a no-op. The per-plate route
        // sees `InkSelector::None` via the OverprintPlan and skips
        // every plate regardless of this colour value. ~keep
        let type_name = arr.first().and_then(|o| o.as_name());
        if matches!(type_name, Some("Separation")) && arr.get(1).and_then(|o| o.as_name()) == Some("None") {
            return Ok(ResolvedColor::Rgba {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            });
        }

        // Determine alternate colour space and tint-transform function.
        // Separation: [/Separation name altCS tintTransform]
        // DeviceN: [/DeviceN names altCS tintTransform attrs?]
        //
        // When the array is malformed (no altCS or no tintTransform), or
        // the function dict is missing / unrecognised, we fall back to
        // `g = 1.0 - tint`. This mirrors the long-standing inline `scn`
        // and `SCN` behaviour: callers exist that rely on it as a
        // "darker = more ink" heuristic for spot inks that never wired
        // up a proper tint transform. Off-vs-on toggle parity holds
        // until the broader §8.6.6.4 fix lands. ~keep
        let invert_tint_fallback = |components: &[f32], alpha: f32| -> ResolvedColor {
            let t = components.first().copied().unwrap_or(0.0);
            let g = (1.0 - t).clamp(0.0, 1.0);
            ResolvedColor::Rgba {
                r: g,
                g,
                b: g,
                a: alpha,
            }
        };

        let alt_cs_obj = match arr.get(2) {
            Some(o) => o,
            None => return Ok(invert_tint_fallback(components, alpha)),
        };
        let func_obj = match arr.get(3) {
            Some(o) => o,
            None => return Ok(invert_tint_fallback(components, alpha)),
        };

        // The alternate colour space may itself be an indirect reference
        // (e.g. `[/Separation /Spot 6 0 R 5 0 R]`) - resolve it before
        // inspecting its shape, mirroring how `func_obj` is resolved just
        // below. Otherwise `.as_name()` returns `None` on the unresolved
        // `Reference` and the compound-array check also fails, so control
        // falls through to the `first_as_gray` fallback for a colour space
        // that is really `/DeviceRGB` or an indirect `[/ICCBased ...]`. ~keep
        let alt_cs_resolved = match ctx.doc.resolve_object(alt_cs_obj) {
            Ok(o) => o,
            Err(_) => return Ok(invert_tint_fallback(components, alpha)),
        };

        let func_resolved = match ctx.doc.resolve_object(func_obj) {
            Ok(o) => o,
            Err(_) => return Ok(invert_tint_fallback(components, alpha)),
        };
        // FunctionType may be in the dict directly (Type 2/3) or in the
        // stream dict (Type 0/4). `as_dict` handles both. ~keep
        let Some(func_dict) = func_resolved.as_dict() else {
            return Ok(invert_tint_fallback(components, alpha));
        };
        let func_type = func_dict.get("FunctionType").and_then(|o| o.as_integer()).unwrap_or(-1);

        let alt_cs_name = alt_cs_resolved.as_name();

        let altspace_values: Vec<f32> = match func_type {
            // Type 0 honours every input component (a multi-channel
            // DeviceN's sampled tint transform is genuinely N-dimensional);
            // Type 3 stitching is single-input by spec and only consults
            // the first component internally. ~keep
            0 | 3 => match evaluate_tint_function(ctx, &func_resolved, components, 0) {
                Some(v) => v,
                // Outside the supported envelope (exotic bit depths,
                // malformed Domain, over-deep nesting): keep the
                // long-standing fallback rather than guess. ~keep
                None => return Ok(invert_tint_fallback(components, alpha)),
            },
            2 => evaluate_type2(func_dict, components[0]),
            4 => evaluate_type4(&func_resolved, components)?,
            _ => return Ok(invert_tint_fallback(components, alpha)),
        };

        // Project the alternate-space values through their colour space.
        // The per-plate routing (which named plate gets the tint, what
        // happens to other plates) is determined by the source colour
        // space — Separation /Pantone-185 paints the Pantone-185 plate,
        // not the C/M/Y/K plates. That routing decision lives on the
        // OverprintPlan's `participating`, stamped by the pipeline
        // composer (see `apply_inks_selector_override`).
        //
        // The composite-side colour resolution is the alternate-space
        // value projected to RGBA — that's what the alternate is for
        // per §8.6.6.3 (composite-only fallback). Emit ResolvedColor::Rgba
        // here so the composite backend gets the right colour without
        // accidentally feeding the alternate's CMYK decomposition into
        // the per-plate path. ~keep
        match alt_cs_name {
            Some("DeviceCMYK") | Some("CMYK") if altspace_values.len() >= 4 => {
                Ok(four_as_cmyk(&altspace_values, alpha, ctx))
            }
            Some("DeviceRGB") | Some("RGB") if altspace_values.len() >= 3 => Ok(three_as_rgb(&altspace_values, alpha)),
            Some("DeviceGray") | Some("G") if !altspace_values.is_empty() => Ok(first_as_gray(&altspace_values, alpha)),
            _ => {
                // Compound alternate space (e.g. ICCBased). We synthesise a
                // logical Spaced colour and recurse — this lets a
                // Separation with an ICC alternate route through the ICC
                // branch correctly. ~keep
                if let Object::Array(_) = alt_cs_resolved {
                    self.resolve_spaced(&alt_cs_resolved, &altspace_values, ctx, alpha)
                } else {
                    Ok(first_as_gray(&altspace_values, alpha))
                }
            }
        }
    }

    fn resolve_indexed(
        &self,
        arr: &[Object],
        components: &[f32],
        ctx: &ResolutionContext,
        alpha: f32,
    ) -> Result<ResolvedColor> {
        if components.is_empty() {
            return Ok(ResolvedColor::Rgba {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: alpha,
            });
        }
        if arr.len() < 4 {
            return Ok(first_as_gray(components, alpha));
        }

        let base = ctx.doc.resolve_object(&arr[1]).unwrap_or_else(|_| arr[1].clone());
        let hival = ctx
            .doc
            .resolve_object(&arr[2])
            .unwrap_or_else(|_| arr[2].clone())
            .as_integer()
            .unwrap_or(0)
            .max(0) as usize;

        // ISO 32000-1 8.6.6.3: a real index is rounded to the nearest integer
        // and an out-of-range one is adjusted to the nearest value in 0..=hival. ~keep
        let idx = (components[0].round().max(0.0) as usize).min(hival);

        let lookup = ctx.doc.resolve_object(&arr[3]).unwrap_or_else(|_| arr[3].clone());
        let palette: Vec<u8> = match &lookup {
            Object::String(bytes) => bytes.clone(),
            Object::Stream { .. } => match lookup.decode_stream_data() {
                Ok(b) => b,
                Err(_) => return Ok(first_as_gray(components, alpha)),
            },
            _ => return Ok(first_as_gray(components, alpha)),
        };

        let n = base_component_count(&base, ctx);
        if n == 0 {
            return Ok(first_as_gray(components, alpha));
        }
        let off = idx * n;
        if off + n > palette.len() {
            return Ok(first_as_gray(components, alpha));
        }

        let base_components: Vec<f32> = palette[off..off + n].iter().map(|b| f32::from(*b) / 255.0).collect();
        self.resolve_spaced(&base, &base_components, ctx, alpha)
    }
}

/// ISO 32000-1 8.6.5.3 CalRGB: apply `/Gamma`, then `/Matrix`, then project
/// XYZ to sRGB. Sharing the DeviceRGB arm skipped the transfer entirely, which
/// renders the common `/Gamma [1 1 1]` (linear) components too dark.
fn resolve_calrgb(arr: &[Object], components: &[f32], ctx: &ResolutionContext, alpha: f32) -> ResolvedColor {
    let [a, b, c] = match components {
        [a, b, c, ..] => [*a, *b, *c],
        _ => return first_as_gray(components, alpha),
    };

    let dict = arr
        .get(1)
        .map(|o| ctx.doc.resolve_object(o).unwrap_or_else(|_| o.clone()));
    let dict = dict.as_ref().and_then(|o| o.as_dict());

    let nums = |key: &str, want: usize| -> Option<Vec<f32>> {
        let v: Vec<f32> = dict?
            .get(key)?
            .as_array()?
            .iter()
            .filter_map(|o| {
                o.as_real()
                    .map(|r| r as f32)
                    .or_else(|| o.as_integer().map(|i| i as f32))
            })
            .collect();
        (v.len() == want).then_some(v)
    };

    let g = nums("Gamma", 3).unwrap_or_else(|| vec![1.0, 1.0, 1.0]);
    let m = nums("Matrix", 9).unwrap_or_else(|| vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]);

    let ag = a.max(0.0).powf(g[0]);
    let bg = b.max(0.0).powf(g[1]);
    let cg = c.max(0.0).powf(g[2]);

    // Matrix is [XA YA ZA XB YB ZB XC YC ZC].
    let x = m[0] * ag + m[3] * bg + m[6] * cg;
    let y = m[1] * ag + m[4] * bg + m[7] * cg;
    let z = m[2] * ag + m[5] * bg + m[8] * cg;

    let (r, gg, bb) = crate::rendering::page_renderer::xyz_to_srgb(x, y, z);
    ResolvedColor::Rgba {
        r: r.clamp(0.0, 1.0),
        g: gg.clamp(0.0, 1.0),
        b: bb.clamp(0.0, 1.0),
        a: alpha,
    }
}

/// Component count of an `/Indexed` base space. Returns 0 for an unrecognised
/// family so the caller falls back instead of striding the palette wrongly.
fn base_component_count(base: &Object, ctx: &ResolutionContext) -> usize {
    if let Some(name) = base.as_name() {
        return match name {
            "DeviceGray" | "G" | "CalGray" => 1,
            "DeviceRGB" | "RGB" | "CalRGB" | "Lab" => 3,
            "DeviceCMYK" | "CMYK" => 4,
            _ => 0,
        };
    }
    let Some(arr) = base.as_array() else {
        return 0;
    };
    match arr.first().and_then(|o| o.as_name()) {
        Some("DeviceGray" | "G" | "CalGray") => 1,
        Some("DeviceRGB" | "RGB" | "CalRGB" | "Lab") => 3,
        Some("DeviceCMYK" | "CMYK") => 4,
        Some("ICCBased") => arr
            .get(1)
            .map(|o| ctx.doc.resolve_object(o).unwrap_or_else(|_| o.clone()))
            .and_then(|st| st.as_dict().and_then(|d| d.get("N").and_then(|n| n.as_integer())))
            .map(|n| n.clamp(0, 4) as usize)
            .unwrap_or(0),
        Some("Separation") => 1,
        Some("DeviceN") => arr
            .get(1)
            .and_then(|o| o.as_array())
            .map(|names| names.len())
            .unwrap_or(0),
        _ => 0,
    }
}

/// Convert a fully-evaluated device-family colour into a final
/// [`ResolvedColor`]. Cmyk passes through as `ResolvedColor::Cmyk` so
/// per-plate backends route by channel and the OPM=1 zero-component
/// rule (§11.7.4.3) can fire on DeviceCMYK direct sources. Composite
/// consumers project Cmyk → Rgba on demand (see page_renderer's
/// `run_pipeline_for_logical`).
fn device_to_rgba(dev: DeviceColor, alpha: f32) -> ResolvedColor {
    match dev {
        DeviceColor::Gray(g) => ResolvedColor::Rgba {
            r: g,
            g,
            b: g,
            a: alpha,
        },
        DeviceColor::Rgb(r, g, b) => ResolvedColor::Rgba { r, g, b, a: alpha },
        DeviceColor::Cmyk(c, m, y, k) => ResolvedColor::Cmyk {
            c: c.clamp(0.0, 1.0),
            m: m.clamp(0.0, 1.0),
            y: y.clamp(0.0, 1.0),
            k: k.clamp(0.0, 1.0),
            a: alpha,
        },
    }
}

fn resolve_device_alias(name: &str, components: &[f32], alpha: f32) -> ResolvedColor {
    match name {
        "DeviceGray" | "G" | "CalGray" => first_as_gray(components, alpha),
        "DeviceRGB" | "RGB" | "CalRGB" => three_as_rgb(components, alpha),
        "DeviceCMYK" | "CMYK" => four_as_cmyk_native(components, alpha),
        _ => first_as_gray(components, alpha),
    }
}

fn first_as_gray(components: &[f32], alpha: f32) -> ResolvedColor {
    let g = components.first().copied().unwrap_or(0.0).clamp(0.0, 1.0);
    ResolvedColor::Rgba {
        r: g,
        g,
        b: g,
        a: alpha,
    }
}

fn three_as_rgb(components: &[f32], alpha: f32) -> ResolvedColor {
    let [r, g, b] = match components {
        [r, g, b, ..] => [*r, *g, *b],
        _ => return first_as_gray(components, alpha),
    };
    ResolvedColor::Rgba {
        r: r.clamp(0.0, 1.0),
        g: g.clamp(0.0, 1.0),
        b: b.clamp(0.0, 1.0),
        a: alpha,
    }
}

/// Emit `ResolvedColor::Rgba` from a 4-component CMYK via the
/// context-aware CMYK→RGB path: the document's `/OutputIntents` CMYK
/// profile when present, otherwise the process-ink conversion. Used by
/// the Separation / DeviceN alternate-CMYK projection — the per-plate
/// routing for those sources is governed by the source colour space,
/// not the alternate's CMYK decomposition, so the alt is composite-
/// only.
fn four_as_cmyk(components: &[f32], alpha: f32, ctx: &ResolutionContext) -> ResolvedColor {
    let (r, g, b) = cmyk_to_rgb_via_intent(components[0], components[1], components[2], components[3], ctx);
    ResolvedColor::Rgba { r, g, b, a: alpha }
}

/// Emit `ResolvedColor::Cmyk` carrying the four-channel decomposition
/// for genuine DeviceCMYK / ICCBased N=4 sources. The per-plate
/// router consumes this directly (process-ink routing + OPM=1 zero-
/// component rule); the composite path projects to RGBA via the
/// process-ink `cmyk_to_rgb_via_intent` in `run_pipeline_for_logical`.
fn four_as_cmyk_native(components: &[f32], alpha: f32) -> ResolvedColor {
    let [c, m, y, k] = match components {
        [c, m, y, k, ..] => [*c, *m, *y, *k],
        _ => return first_as_gray(components, alpha),
    };
    ResolvedColor::Cmyk {
        c: c.clamp(0.0, 1.0),
        m: m.clamp(0.0, 1.0),
        y: y.clamp(0.0, 1.0),
        k: k.clamp(0.0, 1.0),
        a: alpha,
    }
}

/// DeviceCMYK → DeviceRGB via the PROCESS-INK conversion
/// (`crate::color::cmyk_to_rgb`, tetralinear over the 16 measured ink
/// corners), NOT the naive §10.3.5 additive clamp `R = 1 - min(1, C+K)`.
///
/// This is the no-OutputIntent fallback of the composite render path
/// (`run_pipeline_for_logical` → `cmyk_to_rgb_via_intent`), so it must
/// agree with the renderer's own `page_renderer::cmyk_to_rgb`, the image
/// pixel path (`extractors::images::cmyk_pixel_to_rgb`) and the
/// text/extraction path (`document.rs`/`text.rs`): the same CMYK value
/// resolves to the same RGB everywhere (100% K is `#231F20`, 100% cyan
/// `#00ADEF`). A real ICC/OutputIntent CMM still takes precedence when a
/// profile is available (see `cmyk_to_rgb_via_intent`).
fn cmyk_to_rgb(c: f32, m: f32, y: f32, k: f32) -> (f32, f32, f32) {
    crate::color::cmyk_to_rgb(c, m, y, k)
}

/// Context-aware CMYK → RGB convergence.
///
/// Precedence inside this function (callers handle the embedded-ICC
/// case before reaching here — those paths route through
/// `ColorResolver::resolve_iccbased` instead, and the §8.6.5.6
/// `/DefaultCMYK` override fires inside `ColorResolver::resolve` before
/// any device-CMYK reaches this helper):
///
/// 1. `ctx.output_intent_cmyk` — when the document declares an
///    `/OutputIntents` array with a `/N=4` `/DestOutputProfile`,
///    convert the CMYK quadruple through that profile via the
///    `crate::color::Transform` wrapper. The active rendering intent
///    (`ctx.rendering_intent`, §10.7.3) gates which qcms intent the
///    transform is built for. The 8-bit round-trip (quantise CMYK to
///    `[u8; 4]`, run qcms, decode the resulting RGB to `f32`) is the
///    same encoding the rest of `crate::color` uses — going wider
///    here would diverge from the image-decoder path that already
///    funnels through this CMM.
///
/// 2. `ctx.output_intent_cmyk` is `None` — the document didn't
///    declare a CMYK OutputIntent (or one is present but couldn't be
///    parsed). Falls through to the process-ink `cmyk_to_rgb`
///    (`crate::color::cmyk_to_rgb`), the same conversion the renderer,
///    image and extraction paths use, so a DeviceCMYK colour resolves
///    identically whether or not a broken OutputIntent is present.
///
/// **Black-Point Compensation (BPC) and rendering-intent caveats:**
/// qcms 0.3.0 does not implement BPC and, for CMYK sources, silently
/// drops the rendering-intent parameter (see qcms `lib.rs:29-36` and
/// `transform.rs:1283-1289`). The intent value is threaded through the
/// cache key here so a future CMM upgrade that honours intent doesn't
/// silently collapse cache entries; the byte-level output, however, is
/// CURRENTLY intent-invariant for any CMYK input. The HONEST_GAP probe
/// `qa_round4_bpc_paper_white_preservation_under_relative_colorimetric`
/// in `tests/test_render_output_intent.rs` pins this — a CMM upgrade
/// will turn the probe RED at the new per-intent expected references.
///
pub(crate) fn cmyk_to_rgb_via_intent(c: f32, m: f32, y: f32, k: f32, ctx: &ResolutionContext<'_>) -> (f32, f32, f32) {
    if let Some(profile) = ctx.output_intent_cmyk {
        let c_u8 = (c.clamp(0.0, 1.0) * 255.0).round() as u8;
        let m_u8 = (m.clamp(0.0, 1.0) * 255.0).round() as u8;
        let y_u8 = (y.clamp(0.0, 1.0) * 255.0).round() as u8;
        let k_u8 = (k.clamp(0.0, 1.0) * 255.0).round() as u8;
        // The per-page IccTransformCache holds the compiled qcms
        // transform across the many `ResolutionContext` instances the
        // operator dispatcher builds inside one render. Without the
        // cache, every CMYK paint operator rebuilds the 17⁴ CLUT
        // (qcms::Transform::new_to) — that's the perf trap the cache
        // exists to eliminate. The unit-test path skips the cache
        // (`with_icc_transform_cache` is the renderer-only opt-in)
        // and pays the per-call build cost; integration tests cover
        // the cached path through render_page. ~keep
        let rgb = if let Some(cache) = ctx.icc_transform_cache {
            let transform = cache.get_or_build(profile, ctx.rendering_intent);
            transform.convert_cmyk_pixel(c_u8, m_u8, y_u8, k_u8)
        } else {
            let transform =
                crate::color::Transform::new_srgb_target(std::sync::Arc::clone(profile), ctx.rendering_intent);
            transform.convert_cmyk_pixel(c_u8, m_u8, y_u8, k_u8)
        };
        return (rgb[0] as f32 / 255.0, rgb[1] as f32 / 255.0, rgb[2] as f32 / 255.0);
    }
    cmyk_to_rgb(c, m, y, k)
}

/// Evaluate a Type 2 (exponential interpolation) function at a single input.
/// `dict` is the function dictionary (`{/FunctionType 2 /C0 [...] /C1 [...]
/// /N <exponent> /Domain [...]}`). Returns the per-output samples.
///
/// Per ISO 32000-1:2008 §7.10.3: `y_j = C0_j + x^N * (C1_j - C0_j)`.
fn evaluate_type2(dict: &std::collections::HashMap<String, Object>, x: f32) -> Vec<f32> {
    let n = dict
        .get("N")
        .and_then(|o| o.as_real().or_else(|| o.as_integer().map(|i| i as f64)))
        .unwrap_or(1.0) as f32;
    let c0 = dict.get("C0").and_then(|o| o.as_array());
    let c1 = dict.get("C1").and_then(|o| o.as_array());

    let len = c0.map(|a| a.len()).max(c1.map(|a| a.len())).unwrap_or(1);

    let mut out = Vec::with_capacity(len);
    let x_pow = if n == 1.0 { x } else { x.powf(n) };
    for j in 0..len {
        let c0j = c0.and_then(|a| a.get(j)).map(object_to_f32).unwrap_or(0.0);
        let c1j = c1.and_then(|a| a.get(j)).map(object_to_f32).unwrap_or(1.0);
        out.push(c0j + x_pow * (c1j - c0j));
    }
    out
}

/// Evaluate a Type 4 (PostScript calculator) function via
/// [`crate::functions::Program`]. The function body is the stream content of
/// `func_obj`.
fn evaluate_type4(func_obj: &Object, components: &[f32]) -> Result<Vec<f32>> {
    let Object::Stream { dict, .. } = func_obj else {
        // Type-4 functions must be streams per §7.10.5. If we reached this
        // arm without a stream, the function is malformed; fall back to a
        // single-component identity to keep the renderer alive. ~keep
        return Ok(components.to_vec());
    };
    let bytes = func_obj.decode_stream_data()?;
    let domain = dict
        .get("Domain")
        .and_then(|o| o.as_array())
        .map(|a| array_to_pairs(a))
        .unwrap_or_default();
    let range = dict
        .get("Range")
        .and_then(|o| o.as_array())
        .map(|a| array_to_pairs(a))
        .unwrap_or_default();
    let inputs: Vec<f64> = components.iter().map(|&v| v as f64).collect();
    let out = crate::functions::evaluate_type4_clamped(&bytes, &inputs, &domain, &range)?;
    Ok(out.into_iter().map(|v| v as f32).collect())
}

/// Evaluate a tint-transform function of Type 0, 2, 3 or 4 (SS 7.10) against
/// one or more inputs. Used for the Separation / DeviceN path; `depth` caps
/// Type 3 nesting so a self-referential /Functions array cannot recurse
/// unboundedly. Types 2 and 3 are single-input by spec (SS 7.10.3, 7.10.4)
/// and only ever consult `inputs[0]`; Type 0 honours the full input vector
/// so multi-channel DeviceN tint transforms sample all their dimensions.
/// Returns `None` for anything outside the supported envelope so the caller
/// can apply its established fallback instead of guessing.
fn evaluate_tint_function(
    ctx: &ResolutionContext,
    func_resolved: &Object,
    inputs: &[f32],
    depth: usize,
) -> Option<Vec<f32>> {
    const MAX_TINT_DEPTH: usize = 4;
    if depth >= MAX_TINT_DEPTH {
        return None;
    }
    let dict = func_resolved.as_dict()?;
    let func_type = dict.get("FunctionType").and_then(|o| o.as_integer())?;
    let x0 = inputs.first().copied().unwrap_or(0.0);
    match func_type {
        0 => evaluate_type0_sampled(func_resolved, inputs),
        2 => Some(evaluate_type2(dict, x0)),
        3 => evaluate_type3_stitching(ctx, dict, x0, depth),
        4 => evaluate_type4(func_resolved, inputs).ok(),
        _ => None,
    }
}

/// Cap on the number of sampled-function input dimensions we'll evaluate.
/// Real-world DeviceN colorant counts top out around 8 (the same ceiling
/// `page_renderer.rs` applies to its own Type 0 evaluator); `2^8 = 256`
/// corner samples per evaluation keeps multilinear interpolation cheap while
/// still covering every DeviceN tint transform seen in practice. A `/Size`
/// array longer than this is rejected rather than allocating `2^N` corners
/// for an attacker-controlled `N`.
const MAX_SAMPLED_FUNCTION_DIMS: usize = 8;

const DEFAULT_ARRAY_EPSILON: f64 = 1e-9;

/// Whether `/Encode` is absent or holds its default `[0 (Size_0 - 1) 0 (Size_1 - 1) ...]`.
fn encode_is_default(dict: &std::collections::HashMap<String, Object>, sizes: &[usize]) -> bool {
    let Some(encode) = dict.get("Encode").and_then(|o| o.as_array()) else {
        return true;
    };
    if encode.len() != sizes.len() * 2 {
        return false;
    }
    sizes.iter().enumerate().all(|(i, &size)| {
        let lo = object_to_f64(&encode[i * 2]);
        let hi = object_to_f64(&encode[i * 2 + 1]);
        lo.abs() < DEFAULT_ARRAY_EPSILON && (hi - (size as f64 - 1.0)).abs() < DEFAULT_ARRAY_EPSILON
    })
}

/// Whether `/Decode` is absent or equals `/Range`, its default.
fn decode_is_default(dict: &std::collections::HashMap<String, Object>, range: &[[f64; 2]]) -> bool {
    let Some(decode) = dict.get("Decode").and_then(|o| o.as_array()) else {
        return true;
    };
    if decode.len() != range.len() * 2 {
        return false;
    }
    range.iter().enumerate().all(|(i, pair)| {
        let lo = object_to_f64(&decode[i * 2]);
        let hi = object_to_f64(&decode[i * 2 + 1]);
        (lo - pair[0]).abs() < DEFAULT_ARRAY_EPSILON && (hi - pair[1]).abs() < DEFAULT_ARRAY_EPSILON
    })
}

/// Evaluate a Type 0 (sampled) function (SS 7.10.2) for one or more inputs:
/// N-dimensional multilinear interpolation across the `2^N` sample-grid
/// corners nearest the input point, 8- or 16-bit samples, outputs mapped
/// through `/Range`. The common Separation / single-channel-DeviceN shape
/// is the N=1 case, which reduces to the same two-sample linear
/// interpolation this function always used. Returns `None` outside the
/// supported envelope (other bit depths, a non-default `/Encode`/`/Decode`,
/// malformed `/Domain`/`/Size`, a dimension count that doesn't match the
/// number of inputs, more dimensions than [`MAX_SAMPLED_FUNCTION_DIMS`], or
/// a truncated / oversized sample stream).
fn evaluate_type0_sampled(func_obj: &Object, inputs: &[f32]) -> Option<Vec<f32>> {
    let Object::Stream { dict, .. } = func_obj else {
        return None;
    };
    let size = dict.get("Size").and_then(|o| o.as_array())?;
    let n_dims = size.len();
    if n_dims == 0 || n_dims != inputs.len() || n_dims > MAX_SAMPLED_FUNCTION_DIMS {
        return None;
    }
    let sizes: Vec<usize> = size.iter().map(object_to_f64).map(|v| v as usize).collect();
    if sizes.contains(&0) {
        return None;
    }
    let bps = dict.get("BitsPerSample").and_then(|o| o.as_integer()).unwrap_or(8);
    if !(bps == 8 || bps == 16) {
        return None;
    }
    let range = dict.get("Range").and_then(|o| o.as_array())?;
    let range = array_to_pairs(range);
    let n_out = range.len();
    if n_out == 0 {
        return None;
    }
    // Only the default sample mapping is implemented, so a non-default
    // /Encode or /Decode declines; one that merely spells out the Table 39
    // defaults means the same as an absent entry. ~keep
    if !encode_is_default(dict, &sizes) || !decode_is_default(dict, &range) {
        return None;
    }
    let domain = dict
        .get("Domain")
        .and_then(|o| o.as_array())
        .map(|a| array_to_pairs(a))
        .unwrap_or_default();
    // /Domain is required by spec (one pair per input dimension); the N=1
    // case additionally tolerates an absent /Domain (defaulting to [0 1])
    // to preserve exactly the leniency this function always had for the
    // common single-input shape. ~keep
    let domain: Vec<[f64; 2]> = if domain.len() == n_dims {
        domain
    } else if n_dims == 1 && domain.is_empty() {
        vec![[0.0, 1.0]]
    } else {
        return None;
    };
    for [d0, d1] in &domain {
        if !(d0.is_finite() && d1.is_finite() && d0 <= d1) {
            return None;
        }
    }

    let raw = func_obj.decode_stream_data().ok()?;
    let bytes_per = if bps == 8 { 1usize } else { 2 };
    let total_samples = sizes.iter().try_fold(1usize, |acc, &s| acc.checked_mul(s))?;
    let needed = total_samples.checked_mul(n_out)?.checked_mul(bytes_per)?;
    if raw.len() < needed {
        return None;
    }
    let max = if bps == 8 { 255.0 } else { 65535.0 };

    // Per-dimension: clamp the input to its domain and compute the two
    // bracketing sample indices plus the interpolation fraction between
    // them, exactly like the single-input case did per-dimension. ~keep
    struct DimPos {
        i0: usize,
        i1: usize,
        frac: f64,
    }
    let mut dims = Vec::with_capacity(n_dims);
    for d in 0..n_dims {
        let [d0, d1] = domain[d];
        let n_samples = sizes[d];
        let t = (inputs[d] as f64).clamp(d0, d1);
        let span = d1 - d0;
        let pos = if span <= f64::EPSILON {
            0.0
        } else {
            (t - d0) / span * (n_samples - 1) as f64
        };
        let i0 = (pos.floor() as usize).min(n_samples - 1);
        let i1 = (i0 + 1).min(n_samples - 1);
        let frac = pos - i0 as f64;
        dims.push(DimPos { i0, i1, frac });
    }

    // Sample layout per SS 7.10.2: the first input dimension varies fastest
    // ("Sample(0,0,...), Sample(1,0,...), Sample(2,0,...), ..."), so stride
    // 0 is 1 and each subsequent dimension's stride is the product of all
    // earlier /Size entries. ~keep
    let mut strides = vec![1usize; n_dims];
    for d in 1..n_dims {
        strides[d] = strides[d - 1] * sizes[d - 1];
    }

    let sample_at = |idx: &[usize], k: usize| -> f64 {
        let flat: usize = (0..n_dims).map(|d| idx[d] * strides[d]).sum();
        let at = (flat * n_out + k) * bytes_per;
        let v = if bps == 8 {
            raw[at] as f64
        } else {
            u16::from_be_bytes([raw[at], raw[at + 1]]) as f64
        } / max;
        let [r0, r1] = range[k];
        r0 + v * (r1 - r0)
    };

    // Multilinear interpolation: blend the 2^n_dims corners of the grid
    // cell containing the input point, weighted by the product of each
    // dimension's (1-frac)/frac depending on which side of that corner sits. ~keep
    let corner_count = 1usize << n_dims;
    let mut out = vec![0f64; n_out];
    let mut idx = vec![0usize; n_dims];
    for corner in 0..corner_count {
        let mut weight = 1.0f64;
        for d in 0..n_dims {
            if (corner >> d) & 1 == 0 {
                idx[d] = dims[d].i0;
                weight *= 1.0 - dims[d].frac;
            } else {
                idx[d] = dims[d].i1;
                weight *= dims[d].frac;
            }
        }
        if weight == 0.0 {
            continue;
        }
        for (k, acc) in out.iter_mut().enumerate() {
            *acc += weight * sample_at(&idx, k);
        }
    }
    Some(out.into_iter().map(|v| v as f32).collect())
}

/// Evaluate a Type 3 (stitching) function for ONE input (SS 7.10.4): pick the
/// sub-function whose domain slice contains `x`, remap through `/Encode`, and
/// delegate (sub-functions may be Type 0/2/4 or nested Type 3, depth-capped).
fn evaluate_type3_stitching(
    ctx: &ResolutionContext,
    dict: &std::collections::HashMap<String, Object>,
    x: f32,
    depth: usize,
) -> Option<Vec<f32>> {
    let domain = dict.get("Domain").and_then(|o| o.as_array())?;
    let domain = array_to_pairs(domain);
    let (d0, d1) = domain.first().map(|p| (p[0], p[1]))?;
    if !(d0.is_finite() && d1.is_finite() && d0 <= d1) {
        return None;
    }
    let bounds: Vec<f64> = dict
        .get("Bounds")
        .and_then(|o| o.as_array())
        .map(|a| a.iter().map(object_to_f64).collect())
        .unwrap_or_default();
    let encode = dict
        .get("Encode")
        .and_then(|o| o.as_array())
        .map(|a| array_to_pairs(a))
        .unwrap_or_default();
    let funcs = dict.get("Functions").and_then(|o| o.as_array())?;
    if funcs.is_empty() {
        return None;
    }
    let t = (x as f64).clamp(d0, d1);
    let mut k = 0usize;
    while k < bounds.len() && t >= bounds[k] {
        k += 1;
    }
    let lo = if k == 0 { d0 } else { bounds[k - 1] };
    let hi = if k == bounds.len() { d1 } else { bounds[k] };
    let (e0, e1) = encode.get(k).map(|p| (p[0], p[1])).unwrap_or((0.0, 1.0));
    let u = if (hi - lo).abs() <= f64::EPSILON {
        e0
    } else {
        e0 + (t - lo) / (hi - lo) * (e1 - e0)
    };
    let sub = ctx.doc.resolve_object(funcs.get(k)?).ok()?;
    evaluate_tint_function(ctx, &sub, &[u as f32], depth + 1)
}

/// Flatten a `[min1 max1 min2 max2 ...]` PDF array into `[[min, max], ...]`.
fn array_to_pairs(arr: &[Object]) -> Vec<[f64; 2]> {
    arr.as_chunks::<2>()
        .0
        .iter()
        .map(|c| [object_to_f64(&c[0]), object_to_f64(&c[1])])
        .collect()
}

fn object_to_f32(o: &Object) -> f32 {
    object_to_f64(o) as f32
}

fn object_to_f64(o: &Object) -> f64 {
    o.as_real().or_else(|| o.as_integer().map(|i| i as f64)).unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rendering::resolution::test_support::fixture_doc;
    use std::collections::HashMap;

    fn ctx<'a>(doc: &'a crate::document::PdfDocument, spaces: &'a HashMap<String, Object>) -> ResolutionContext<'a> {
        ResolutionContext::new(doc, spaces)
    }

    /// Assert resolved colour matches expected RGBA. Accepts either
    /// `ResolvedColor::Rgba` directly or `ResolvedColor::Cmyk`
    /// projected via the same process-ink `cmyk_to_rgb` the composite
    /// render path uses (the resolver now emits Cmyk for Separation /
    /// DeviceN sources with a CMYK alternate so per-plate backends see
    /// the channel decomposition; composite consumers project on
    /// demand). Projecting through the engine's own converter keeps the
    /// expected RGB in this helper consistent with what the renderer
    /// actually paints for the same CMYK plates.
    fn assert_rgba(c: ResolvedColor, r: f32, g: f32, b: f32, a: f32) {
        let (rr, gg, bb, aa) = match c {
            ResolvedColor::Rgba { r, g, b, a } => (r, g, b, a),
            ResolvedColor::Cmyk { c, m, y, k, a } => {
                let (rr, gg, bb) = super::cmyk_to_rgb(c, m, y, k);
                (rr, gg, bb, a)
            }
            other => panic!("expected Rgba or Cmyk; got {other:?}"),
        };
        assert!((rr - r).abs() < 1e-3, "r: got {rr}, want {r}");
        assert!((gg - g).abs() < 1e-3, "g: got {gg}, want {g}");
        assert!((bb - b).abs() < 1e-3, "b: got {bb}, want {b}");
        assert!((aa - a).abs() < 1e-3, "a: got {aa}, want {a}");
    }

    #[test]
    fn resolves_device_gray_logical_color() {
        let doc = fixture_doc();
        let spaces = HashMap::new();
        let resolver = ColorResolver::new();
        let lc = LogicalColor::Device(DeviceColor::Gray(0.42));
        let c = resolver.resolve(&lc, &ctx(&doc, &spaces), 0.9).unwrap();
        assert_rgba(c, 0.42, 0.42, 0.42, 0.9);
    }

    #[test]
    fn resolves_device_rgb_logical_color() {
        let doc = fixture_doc();
        let spaces = HashMap::new();
        let resolver = ColorResolver::new();
        let lc = LogicalColor::Device(DeviceColor::Rgb(1.0, 0.5, 0.25));
        let c = resolver.resolve(&lc, &ctx(&doc, &spaces), 1.0).unwrap();
        assert_rgba(c, 1.0, 0.5, 0.25, 1.0);
    }

    #[test]
    fn resolves_device_cmyk_via_process_inks() {
        // DeviceCMYK composites through the process-ink converter
        // (`crate::color::cmyk_to_rgb`, tetralinear over the 16 measured
        // ink corners), NOT the §10.3.5 additive clamp. 100% cyan lands
        // on the measured corner `#00ADEF` = (0.0, 0.6784, 0.9373), not
        // (0, 1, 1). The resolver emits `Cmyk` (for per-plate routing);
        // the composite projection is `cmyk_to_rgb_via_intent`, whose
        // no-OutputIntent fallback is the process-ink path. ~keep
        let doc = fixture_doc();
        let spaces = HashMap::new();
        let resolver = ColorResolver::new();
        let lc = LogicalColor::Device(DeviceColor::Cmyk(1.0, 0.0, 0.0, 0.0));
        let c = resolver.resolve(&lc, &ctx(&doc, &spaces), 1.0).unwrap();
        let (cc, m, y, k, a) = match c {
            ResolvedColor::Cmyk { c, m, y, k, a } => (c, m, y, k, a),
            other => panic!("expected Cmyk; got {other:?}"),
        };
        let (r, g, b) = super::cmyk_to_rgb_via_intent(cc, m, y, k, &ctx(&doc, &spaces));
        assert!((r - 0.0).abs() < 1e-3, "r: got {r}, want 0.0");
        assert!((g - 0.6784).abs() < 1e-3, "g: got {g}, want 0.6784");
        assert!((b - 0.9373).abs() < 1e-3, "b: got {b}, want 0.9373");
        assert!((a - 1.0).abs() < 1e-3, "a: got {a}, want 1.0");
    }

    #[test]
    fn resolves_spaced_device_alias_as_rgb() {
        let doc = fixture_doc();
        let spaces = HashMap::new();
        let resolver = ColorResolver::new();
        let space = Object::Name("DeviceRGB".to_string());
        let lc = LogicalColor::Spaced {
            space: &space,
            components: smallvec::smallvec![0.2, 0.4, 0.6],
        };
        let c = resolver.resolve(&lc, &ctx(&doc, &spaces), 1.0).unwrap();
        assert_rgba(c, 0.2, 0.4, 0.6, 1.0);
    }

    #[test]
    fn separation_with_type2_cmyk_alternate_uses_function() {
        // /Separation /SpotInk /DeviceCMYK
        //   << /FunctionType 2 /N 1 /C0 [0 0 0 0] /C1 [0 1 0 0] /Domain [0 1] /Range [0 1 0 1 0 1 0 1] >>
        // tint=1 must produce CMYK(0,1,0,0), the process-ink magenta
        // corner #EC008C = (0.9255, 0, 0.5490). ~keep
        let mut func_dict: HashMap<String, Object> = HashMap::new();
        func_dict.insert("FunctionType".into(), Object::Integer(2));
        func_dict.insert("N".into(), Object::Integer(1));
        func_dict.insert(
            "C0".into(),
            Object::Array(vec![
                Object::Real(0.0),
                Object::Real(0.0),
                Object::Real(0.0),
                Object::Real(0.0),
            ]),
        );
        func_dict.insert(
            "C1".into(),
            Object::Array(vec![
                Object::Real(0.0),
                Object::Real(1.0),
                Object::Real(0.0),
                Object::Real(0.0),
            ]),
        );
        let func_obj = Object::Dictionary(func_dict);

        let arr = vec![
            Object::Name("Separation".into()),
            Object::Name("SpotInk".into()),
            Object::Name("DeviceCMYK".into()),
            func_obj,
        ];
        let space = Object::Array(arr);
        let doc = fixture_doc();
        let spaces = HashMap::new();
        let resolver = ColorResolver::new();
        let lc = LogicalColor::Spaced {
            space: &space,
            components: smallvec::smallvec![1.0],
        };
        let c = resolver.resolve(&lc, &ctx(&doc, &spaces), 1.0).unwrap();
        // CMYK(0,1,0,0) -> process-ink magenta corner (0.9255, 0, 0.5490) ~keep
        assert_rgba(c, 0.9255, 0.0, 0.5490, 1.0);
    }

    #[test]
    fn separation_with_type4_calculator_evaluates_program() {
        // /Separation /MagentaSpot /DeviceCMYK
        //   stream containing: { 0.0 exch dup 0.0 exch 0.0 }  ; tint → CMYK(0, tint, 0, 0)
        // tint=1.0 should yield CMYK(0,1,0,0) → RGB(1,0,1).
        //
        // This is the canonical test for the case where the existing inline
        // path at page_renderer.rs:690 returns `1.0 - tint` = 0.0 (solid black)
        // because it only recognises FunctionType==2. Through the resolver,
        // the Type-4 program runs to completion and the colour comes out
        // correct.
        //
        // PostScript stack convention: inputs are pushed in order, output is
        // read top-down from the final stack. With one input (tint) the
        // program needs to leave four values on the stack representing
        // C, M, Y, K. We use: `0.0 exch 0.0 0.0` — tint is on top after
        // exch, but we want the order C M Y K = 0 tint 0 0. The simplest
        // form: pop the tint into M position by emitting `0.0 3 1 roll
        // 0.0 0.0` doesn't actually work cleanly; instead use:
        //   `{ 0.0 exch 0.0 0.0 }` — wait this pushes 0, then swaps with
        //   tint giving stack [tint, 0], then pushes 0 0 giving
        //   [tint, 0, 0, 0]. That's C=tint not M=tint.
        //
        // To get [C, M, Y, K] = [0, tint, 0, 0] in PLRM stack order
        // (output order top-down so K is top), we need stack contents
        // bottom-to-top: [0, tint, 0, 0]. With tint on the stack from the
        // caller, we want: push 0 below tint (using exch), then push 0 0.
        // That's `0 exch 0 0` — yields stack bottom-to-top [0, tint, 0, 0],
        // i.e. C=0, M=tint, Y=0, K=0. (`evaluate_type4` returns the stack
        // from bottom to top as a Vec, so out[0]=C, out[1]=M, out[2]=Y,
        // out[3]=K.) ~keep
        let program = b"{ 0.0 exch 0.0 0.0 }";

        let mut func_dict: HashMap<String, Object> = HashMap::new();
        func_dict.insert("FunctionType".into(), Object::Integer(4));
        func_dict.insert(
            "Domain".into(),
            Object::Array(vec![Object::Integer(0), Object::Integer(1)]),
        );
        func_dict.insert(
            "Range".into(),
            Object::Array(vec![
                Object::Integer(0),
                Object::Integer(1),
                Object::Integer(0),
                Object::Integer(1),
                Object::Integer(0),
                Object::Integer(1),
                Object::Integer(0),
                Object::Integer(1),
            ]),
        );

        let func_obj = Object::Stream {
            dict: func_dict,
            data: program.to_vec().into(),
        };

        let arr = vec![
            Object::Name("Separation".into()),
            Object::Name("MagentaSpot".into()),
            Object::Name("DeviceCMYK".into()),
            func_obj,
        ];
        let space = Object::Array(arr);
        let doc = fixture_doc();
        let spaces = HashMap::new();
        let resolver = ColorResolver::new();
        let lc = LogicalColor::Spaced {
            space: &space,
            components: smallvec::smallvec![1.0],
        };
        let c = resolver.resolve(&lc, &ctx(&doc, &spaces), 1.0).unwrap();
        assert_rgba(c, 0.9255, 0.0, 0.5490, 1.0);
    }

    #[test]
    fn separation_full_tint_with_type4_no_longer_renders_solid_black() {
        // Regression guard for the structural class of bug where a
        // Separation with a Type-4 tint transform and a fully opaque tint
        // must not fall back to the `1.0 - tint = 0` grayscale
        // path. The previous test confirmed the resolved RGB is non-black;
        // this test asserts directly that none of the channels are zero
        // luminance, regardless of the specific colour produced.
        //
        // Program: `{ 0.0 exch 0.0 0.0 }` again — yields CMYK(0, tint, 0, 0),
        // RGB(1-0, 1-tint, 1-0) = (1, 1-tint, 1). At tint=1, that's (1, 0, 1). ~keep
        let program = b"{ 0.0 exch 0.0 0.0 }";
        let mut func_dict: HashMap<String, Object> = HashMap::new();
        func_dict.insert("FunctionType".into(), Object::Integer(4));
        let func_obj = Object::Stream {
            dict: func_dict,
            data: program.to_vec().into(),
        };
        let arr = vec![
            Object::Name("Separation".into()),
            Object::Name("MagentaSpot".into()),
            Object::Name("DeviceCMYK".into()),
            func_obj,
        ];
        let space = Object::Array(arr);
        let doc = fixture_doc();
        let spaces = HashMap::new();
        let resolver = ColorResolver::new();
        let lc = LogicalColor::Spaced {
            space: &space,
            components: smallvec::smallvec![1.0],
        };
        let c = resolver.resolve(&lc, &ctx(&doc, &spaces), 1.0).unwrap();
        // Separation with a DeviceCMYK alternate now emits Cmyk so the
        // per-plate router can route channels by name. Project the
        // result to RGBA for the regression-guard comparison. ~keep
        let (r, g, b) = match c {
            ResolvedColor::Rgba { r, g, b, .. } => (r, g, b),
            ResolvedColor::Cmyk { c, m, y, k, .. } => {
                let rr = (1.0 - (c + k).min(1.0)).clamp(0.0, 1.0);
                let gg = (1.0 - (m + k).min(1.0)).clamp(0.0, 1.0);
                let bb = (1.0 - (y + k).min(1.0)).clamp(0.0, 1.0);
                (rr, gg, bb)
            }
            other => panic!("expected Rgba or Cmyk; got {other:?}"),
        };
        // The old inline path would have produced gray = 1.0 - 1.0 = 0.0
        // for all channels. The pipeline must never produce that for a
        // Type-4 spot. ~keep
        assert!(
            !(r < 0.01 && g < 0.01 && b < 0.01),
            "full-tint Type-4 spot must not render solid black; got ({r}, {g}, {b})"
        );
    }

    #[test]
    fn type0_sampled_function_single_dimension_matches_prior_linear_interpolation() {
        // Pins the pre-existing single-input behaviour exactly: 3 samples
        // over Domain [0 1], input 0.25 sits 50% of the way between sample
        // 0 (0/255) and sample 1 (128/255). ~keep
        let mut func_dict: HashMap<String, Object> = HashMap::new();
        func_dict.insert("FunctionType".into(), Object::Integer(0));
        func_dict.insert(
            "Domain".into(),
            Object::Array(vec![Object::Integer(0), Object::Integer(1)]),
        );
        func_dict.insert(
            "Range".into(),
            Object::Array(vec![Object::Integer(0), Object::Integer(1)]),
        );
        func_dict.insert("Size".into(), Object::Array(vec![Object::Integer(3)]));
        func_dict.insert("BitsPerSample".into(), Object::Integer(8));
        let func_obj = Object::Stream {
            dict: func_dict,
            data: vec![0u8, 128, 255].into(),
        };
        let out = super::evaluate_type0_sampled(&func_obj, &[0.25]).expect("in supported envelope");
        assert_eq!(out.len(), 1);
        let expected = 0.5 * (128.0 / 255.0);
        assert!((out[0] - expected).abs() < 1e-4, "got {}, want {}", out[0], expected);
    }

    #[test]
    fn type0_sampled_function_two_dimensional_input_uses_all_components() {
        // A genuinely 2-D sampled function (Size [2 2]): the sample only
        // takes a non-zero value at the (1,1) grid corner. Feeding inputs
        // [0.25, 0.75] must land at 0.25*0.75 = 0.1875 — the bilinear
        // weight of that corner — which is impossible to produce from
        // `components[0]` alone (a single-input reading would ignore the
        // second component entirely and could never reach a component-1
        // dependent value). ~keep
        let mut func_dict: HashMap<String, Object> = HashMap::new();
        func_dict.insert("FunctionType".into(), Object::Integer(0));
        func_dict.insert(
            "Domain".into(),
            Object::Array(vec![
                Object::Integer(0),
                Object::Integer(1),
                Object::Integer(0),
                Object::Integer(1),
            ]),
        );
        func_dict.insert(
            "Range".into(),
            Object::Array(vec![Object::Integer(0), Object::Integer(1)]),
        );
        func_dict.insert(
            "Size".into(),
            Object::Array(vec![Object::Integer(2), Object::Integer(2)]),
        );
        func_dict.insert("BitsPerSample".into(), Object::Integer(8));
        // Sample order per SS 7.10.2 (dim 0 fastest): (0,0) (1,0) (0,1) (1,1). ~keep
        let func_obj = Object::Stream {
            dict: func_dict,
            data: vec![0u8, 0, 0, 255].into(),
        };
        let out = super::evaluate_type0_sampled(&func_obj, &[0.25, 0.75]).expect("in supported envelope");
        assert_eq!(out.len(), 1);
        assert!((out[0] - 0.1875).abs() < 1e-4, "got {}, want 0.1875", out[0]);

        // Sanity: the OLD single-input fallback formula (1 - components[0])
        // would have produced 0.75 here — a different value — confirming
        // this assertion actually exercises the second input dimension
        // rather than coincidentally matching the discarded-component path. ~keep
        assert!((out[0] - 0.75).abs() > 1e-3);
    }

    #[test]
    fn devicen_two_channel_type0_tint_transform_resolves_via_all_components() {
        // End-to-end: a DeviceN colour space with 2 named channels and a
        // Type 0 (sampled) tint transform, resolved through the full
        // Separation/DeviceN pipeline exactly as a `scn` operator would
        // drive it. Mirrors the structure of real-world multi-channel
        // DeviceN spot-colour PDFs (a sampled tint transform over an N>1
        // colorant set). ~keep
        let mut func_dict: HashMap<String, Object> = HashMap::new();
        func_dict.insert("FunctionType".into(), Object::Integer(0));
        func_dict.insert(
            "Domain".into(),
            Object::Array(vec![
                Object::Integer(0),
                Object::Integer(1),
                Object::Integer(0),
                Object::Integer(1),
            ]),
        );
        func_dict.insert(
            "Range".into(),
            Object::Array(vec![
                Object::Integer(0),
                Object::Integer(1),
                Object::Integer(0),
                Object::Integer(1),
                Object::Integer(0),
                Object::Integer(1),
            ]),
        );
        func_dict.insert(
            "Size".into(),
            Object::Array(vec![Object::Integer(2), Object::Integer(2)]),
        );
        func_dict.insert("BitsPerSample".into(), Object::Integer(8));
        // 3 output channels (R, G, B on the DeviceRGB alternate), 4 grid
        // corners. Only the (1,1) corner (both inputs at their max) is
        // pure red; every other corner is black. ~keep
        let samples: Vec<u8> = vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 0, 0];
        let func_obj = Object::Stream {
            dict: func_dict,
            data: samples.into(),
        };
        let arr = vec![
            Object::Name("DeviceN".into()),
            Object::Array(vec![Object::Name("Alpha".into()), Object::Name("Beta".into())]),
            Object::Name("DeviceRGB".into()),
            func_obj,
        ];
        let space = Object::Array(arr);
        let doc = fixture_doc();
        let spaces = HashMap::new();
        let resolver = ColorResolver::new();
        let lc = LogicalColor::Spaced {
            space: &space,
            components: smallvec::smallvec![1.0, 1.0],
        };
        let c = resolver.resolve(&lc, &ctx(&doc, &spaces), 1.0).unwrap();
        assert_rgba(c, 1.0, 0.0, 0.0, 1.0);
    }

    #[test]
    fn separation_none_resolves_to_fully_transparent_for_composite() {
        // §8.6.6.3 reserved name `/None`: composite output is fully
        // transparent so the splice carries no marks through, mirroring
        // the per-plate `Skip` decision the InkRouter makes off the
        // OverprintPlan's `selector: InkSelector::None`. ~keep
        let arr = vec![
            Object::Name("Separation".into()),
            Object::Name("None".into()),
            Object::Name("DeviceGray".into()),
            Object::Dictionary({
                let mut d = HashMap::new();
                d.insert("FunctionType".into(), Object::Integer(2));
                d
            }),
        ];
        let space = Object::Array(arr);
        let doc = fixture_doc();
        let spaces = HashMap::new();
        let resolver = ColorResolver::new();
        let lc = LogicalColor::Spaced {
            space: &space,
            components: smallvec::smallvec![0.5],
        };
        let c = resolver.resolve(&lc, &ctx(&doc, &spaces), 0.9).unwrap();
        match c {
            ResolvedColor::Rgba { a, .. } => {
                assert!((a - 0.0).abs() < 1e-6, "/None composite alpha must be 0");
            }
            other => panic!("expected Rgba; got {other:?}"),
        }
    }

    #[test]
    fn separation_with_unknown_function_type_falls_back_to_gray() {
        // FunctionType 99 is not a real PDF spec value; the resolver must
        // degrade safely rather than panic. Matches the existing inline
        // behaviour of "first component as gray". ~keep
        let mut func_dict: HashMap<String, Object> = HashMap::new();
        func_dict.insert("FunctionType".into(), Object::Integer(99));
        let func_obj = Object::Dictionary(func_dict);
        let arr = vec![
            Object::Name("Separation".into()),
            Object::Name("Whatever".into()),
            Object::Name("DeviceCMYK".into()),
            func_obj,
        ];
        let space = Object::Array(arr);
        let doc = fixture_doc();
        let spaces = HashMap::new();
        let resolver = ColorResolver::new();
        let lc = LogicalColor::Spaced {
            space: &space,
            components: smallvec::smallvec![0.5],
        };
        let c = resolver.resolve(&lc, &ctx(&doc, &spaces), 1.0).unwrap();
        assert_rgba(c, 0.5, 0.5, 0.5, 1.0);
    }

    #[test]
    fn iccbased_with_n4_routes_through_cmyk_fallback() {
        // ICCBased streams declare /N. With N=4 we treat components as
        // DeviceCMYK in the no-CMM fallback path (same as the existing
        // inline behaviour at `page_renderer.rs:584-617`). ~keep
        let mut stream_dict: HashMap<String, Object> = HashMap::new();
        stream_dict.insert("N".into(), Object::Integer(4));
        let icc_stream = Object::Stream {
            dict: stream_dict,
            data: Vec::new().into(),
        };
        let arr = vec![Object::Name("ICCBased".into()), icc_stream];
        let space = Object::Array(arr);
        let doc = fixture_doc();
        let spaces = HashMap::new();
        let resolver = ColorResolver::new();
        let lc = LogicalColor::Spaced {
            space: &space,
            components: smallvec::smallvec![1.0, 0.0, 0.0, 0.0],
        };
        let c = resolver.resolve(&lc, &ctx(&doc, &spaces), 1.0).unwrap();
        // ICCBased N=4 falls back to DeviceCMYK; CMYK(1,0,0,0) composites
        // through the process-ink converter to the cyan corner #00ADEF. ~keep
        assert_rgba(c, 0.0, 0.6784, 0.9373, 1.0);
    }

    #[test]
    fn alpha_passthrough_into_rgba() {
        // Every resolution path must fold the input alpha into the output
        // RGBA. Test the Device path here; the rest is covered by the
        // type-specific tests above. ~keep
        let doc = fixture_doc();
        let spaces = HashMap::new();
        let resolver = ColorResolver::new();
        let lc = LogicalColor::Device(DeviceColor::Gray(0.5));
        let c = resolver.resolve(&lc, &ctx(&doc, &spaces), 0.3).unwrap();
        match c {
            ResolvedColor::Rgba { a, .. } => assert!((a - 0.3).abs() < 1e-6),
            _ => panic!("expected Rgba"),
        }
    }

    #[test]
    fn cmyk_to_rgb_via_intent_with_no_output_intent_uses_process_inks() {
        // The fallback arm is the process-ink `cmyk_to_rgb`. Pin one
        // representative quadruple so a regression that re-routed the
        // no-OutputIntent path through some other conversion (e.g. back
        // to the §10.3.5 additive clamp) would surface here. CMYK(0.25,
        // 0, 0, 0) interpolates 0.75·paper + 0.25·cyan corner =
        // (0.75, 0.9196, 0.9843). ~keep
        let doc = fixture_doc();
        let spaces = HashMap::new();
        let ctx = ResolutionContext::new(&doc, &spaces);
        let (r, g, b) = super::cmyk_to_rgb_via_intent(0.25, 0.0, 0.0, 0.0, &ctx);
        assert!((r - 0.75).abs() < 1e-4, "r: got {r}, want 0.75");
        assert!((g - 0.9196).abs() < 1e-4, "g: got {g}, want 0.9196");
        assert!((b - 0.9843).abs() < 1e-4, "b: got {b}, want 0.9843");
    }

    #[test]
    fn cmyk_to_rgb_via_intent_falls_back_when_profile_has_no_cmm() {
        // The header-only stub profile parses (IccProfile::parse accepts
        // the 128-byte header) but qcms refuses to build a Transform
        // from it because there's no tag table. The wrapper devolves to
        // its no-CMM fallback internally — the helper must agree with
        // the no-OutputIntent path on the same input. This is the
        // shape a real but malformed /OutputIntents profile would take. ~keep
        let doc = fixture_doc();
        let spaces = HashMap::new();
        let mut header_only = vec![0u8; 128];
        header_only[8..12].copy_from_slice(&0x04000000u32.to_be_bytes());
        header_only[12..16].copy_from_slice(b"prtr");
        header_only[16..20].copy_from_slice(b"CMYK");
        header_only[20..24].copy_from_slice(b"Lab ");
        header_only[36..40].copy_from_slice(b"acsp");
        let profile = std::sync::Arc::new(crate::color::IccProfile::parse(header_only, 4).expect("stub parses"));
        let ctx = ResolutionContext::new(&doc, &spaces).with_output_intent(Some(&profile));
        let (r, g, b) = super::cmyk_to_rgb_via_intent(0.25, 0.0, 0.0, 0.0, &ctx);
        // The no-CMM fallback of `convert_cmyk_pixel` routes through
        // `crate::extractors::images::cmyk_pixel_to_rgb`, which is now
        // the process-ink `crate::color::cmyk_to_rgb` — the same
        // conversion the no-OutputIntent arm takes. So both arms agree
        // on the process-ink value for CMYK(0.25,0,0,0) ≈
        // (0.75, 0.9196, 0.9843); the 8-bit CMM round-trip widens the
        // tolerance slightly. ~keep
        assert!((r - 0.75).abs() < 0.01, "got r={r}");
        assert!((g - 0.9196).abs() < 0.01, "got g={g}");
        assert!((b - 0.9843).abs() < 0.01, "got b={b}");
    }

    #[test]
    fn three_as_rgb_degrades_when_operands_are_short() {
        assert_rgba(three_as_rgb(&[0.5], 1.0), 0.5, 0.5, 0.5, 1.0);
        assert_rgba(three_as_rgb(&[], 1.0), 0.0, 0.0, 0.0, 1.0);
        assert_rgba(three_as_rgb(&[0.25, 0.5], 1.0), 0.25, 0.25, 0.25, 1.0);
    }

    #[test]
    fn four_as_cmyk_native_degrades_when_operands_are_short() {
        assert_rgba(four_as_cmyk_native(&[0.5], 1.0), 0.5, 0.5, 0.5, 1.0);
        assert_rgba(four_as_cmyk_native(&[], 1.0), 0.0, 0.0, 0.0, 1.0);
        assert_rgba(four_as_cmyk_native(&[0.1, 0.2, 0.3], 1.0), 0.1, 0.1, 0.1, 1.0);
    }

    #[test]
    fn projection_helpers_still_use_the_operands_they_have() {
        assert_rgba(three_as_rgb(&[1.0, 0.0, 0.0], 1.0), 1.0, 0.0, 0.0, 1.0);
        assert_rgba(three_as_rgb(&[1.0, 0.0, 0.0, 0.9], 1.0), 1.0, 0.0, 0.0, 1.0);
        let (r, g, b) = super::cmyk_to_rgb(0.0, 0.0, 0.0, 1.0);
        assert_rgba(four_as_cmyk_native(&[0.0, 0.0, 0.0, 1.0], 1.0), r, g, b, 1.0);
    }
}
