//! XObject, inline-image and shading operator arms (`Do`, `BI`, `sh`) of the operator loop.

use super::*;

impl PageRenderer {
    pub(super) fn op_do(
        &mut self,
        pixmap: &mut Pixmap,
        st: &mut OpLoopState<'_>,
        env: OpEnv<'_>,
        name: &str,
    ) -> Result<()> {
        let OpEnv {
            base_transform,
            doc,
            page_num,
            resources,
        } = env;
        if st.excluded_layer_depth == 0 {
            let gs_clone = st.gs_stack.current().clone();
            let transform = combine_transforms(base_transform, &gs_clone.ctm);
            let clip = st.clip_stack.last().and_then(|c| c.as_ref());
            tracing::trace!("Do: rendering XObject '{}'", name);
            let (is_form, group_boundary) = self.xobject_kind(name, resources, doc).unwrap_or_default();
            let _image_workspace = if is_form {
                None
            } else {
                self.reserve_raster_workspace(pixmap, &gs_clone, 6, knockout::SIDECAR_SNAPSHOT_COPIES)?
            };
            let _ordinary_form_mask = if is_form && !group_boundary && gs_clone.smask.is_some() {
                Some(self.knockout_resources.reserve(Some(0))?)
            } else {
                None
            };
            let snapshots = self.xobject_paint_snapshots(pixmap, &gs_clone, doc, is_form, group_boundary)?;
            // §8.9.5 + §8.9.6.2 + §11.7.3: rasterise the
            // Image / ImageMask footprint + stencil-bit
            // coverage so the spot mirror has a geometry-
            // true per-pixel mask. Skipped for Form
            // XObjects (their per-paint mirror runs
            // inside the recursive content stream — the
            // post-Do mirror for Forms is already
            // suppressed by round 3's P0 fix). ~keep
            let image_coverage = (!is_form && (snapshots.spot.is_some() || self.knockout_shape.is_some()))
                .then(|| self.rasterise_image_xobject_coverage(name, transform, &gs_clone, resources, doc, clip, false))
                .flatten();
            let image_geometry = (self.knockout_shape.is_some() && !is_form)
                .then(|| self.rasterise_image_xobject_coverage(name, transform, &gs_clone, resources, doc, clip, true))
                .flatten();
            self.render_xobject(
                pixmap,
                name,
                transform,
                &gs_clone,
                resources,
                doc,
                page_num,
                clip,
                base_transform,
            )?;
            self.apply_paint_modulators(
                pixmap,
                snapshots,
                image_coverage.as_deref(),
                PaintModulatorContext {
                    gs: &gs_clone,
                    doc,
                    page_num,
                    resources,
                    base_transform,
                    fill_side: true,
                    knockout_coverage: image_geometry.as_deref(),
                },
            )?;
        }
        Ok(())
    }

    pub(super) fn op_inline_image(
        &mut self,
        pixmap: &mut Pixmap,
        st: &mut OpLoopState<'_>,
        env: OpEnv<'_>,
        dict: &std::collections::HashMap<String, Object>,
        data: &[u8],
    ) -> Result<()> {
        let OpEnv {
            base_transform,
            doc,
            resources,
            ..
        } = env;
        if st.excluded_layer_depth == 0 {
            let gs_clone = st.gs_stack.current().clone();
            let transform = combine_transforms(base_transform, &gs_clone.ctm);
            let clip = st.clip_stack.last().and_then(|c| c.as_ref());
            let expanded = crate::extractors::images::expand_inline_image_dict(dict.clone());
            let is_image_mask = expanded
                .get("ImageMask")
                .map(|o| matches!(o, Object::Boolean(true)))
                .unwrap_or(false);
            let synthetic = Object::Stream {
                dict: expanded,
                data: bytes::Bytes::copy_from_slice(data),
            };
            if self.knockout_shape.is_some() {
                self.collect_inline_image_shape(&synthetic, transform, &gs_clone, doc, clip, is_image_mask)?;
            }
            if is_image_mask {
                if let Err(error) = self.render_image_mask(
                    pixmap,
                    &synthetic,
                    None,
                    transform,
                    doc,
                    clip,
                    &gs_clone,
                    (base_transform, resources),
                ) {
                    tracing::warn!(
                        error_code = error.telemetry_code(),
                        error_offset = ?error.telemetry_offset(),
                        "skipping unrenderable inline ImageMask"
                    );
                }
            } else if let Err(error) =
                self.render_image(pixmap, &synthetic, None, transform, doc, clip, None, None, &gs_clone)
            {
                tracing::warn!(
                    error_code = error.telemetry_code(),
                    error_offset = ?error.telemetry_offset(),
                    "skipping unrenderable inline image"
                );
            }
        }
        Ok(())
    }

    pub(super) fn op_paint_shading(
        &mut self,
        pixmap: &mut Pixmap,
        st: &mut OpLoopState<'_>,
        env: OpEnv<'_>,
        name: &str,
    ) -> Result<()> {
        let OpEnv {
            base_transform,
            doc,
            page_num,
            resources,
        } = env;
        if st.excluded_layer_depth == 0 {
            let mut gs_clone = st.gs_stack.current().clone();
            self.seed_shading_spot_inks(&mut gs_clone, name, resources, doc);
            let transform = combine_transforms(base_transform, &gs_clone.ctm);
            let clip = st.clip_stack.last().and_then(|c| c.as_ref());
            // §11.4.7 + §11.7.4 + §11.4 cycle: shading is
            // a fill-side paint, so the snapshot/apply
            // cadence mirrors the path-Fill arm. The
            // overprint and compose-first paths short-
            // circuit when the active fill colour is not
            // CMYK (the shading paint's per-pixel colour
            // comes from the gradient interpolator, not
            // `gs.fill_color_cmyk`), so they only fire when
            // the page set a CMYK fill before invoking
            // `sh`. ~keep
            let smask_snap = self.paint_mask_snapshots(pixmap, &gs_clone)?;
            let overprint_snap = self.overprint_snapshot(pixmap, &gs_clone, true);
            let cmyk_compose_snap = self.cmyk_compose_snapshot(pixmap, &gs_clone, doc, true);
            let spot_snap = self.spot_paint_snapshot(pixmap, &gs_clone, true);
            // §8.7.4 + §11.7.3: rasterise the shading
            // geometry (intersected with the active clip)
            // so the spot mirror sees the geometry-true
            // per-pixel coverage of the gradient. ~keep
            let shading_coverage = (spot_snap.is_some() || self.knockout_shape.is_some())
                .then(|| self.rasterise_shading_coverage(name, transform, &gs_clone, resources, doc, clip))
                .flatten();
            self.render_shading(pixmap, name, transform, &gs_clone, resources, doc, clip)?;
            self.apply_paint_modulators(
                pixmap,
                PaintModulatorSnapshots {
                    process_path: None,
                    cmyk_compose: cmyk_compose_snap,
                    overprint: overprint_snap,
                    spot: spot_snap,
                    smask: smask_snap,
                },
                shading_coverage.as_deref(),
                PaintModulatorContext {
                    gs: &gs_clone,
                    doc,
                    page_num,
                    resources,
                    base_transform,
                    fill_side: true,
                    knockout_coverage: None,
                },
            )?;
        }
        Ok(())
    }

    fn xobject_paint_snapshots(
        &mut self,
        pixmap: &mut Pixmap,
        gs_clone: &GraphicsState,
        doc: &PdfDocument,
        is_form: bool,
        group_boundary: bool,
    ) -> Result<PaintModulatorSnapshots> {
        // §11.4.7 + §11.7.4 + §11.4 cycle: the entire
        // XObject paint (Form or Image) sits inside the
        // snapshot bracket so a /SMask attached via
        // ExtGState modulates the cumulative
        // contribution. Image XObjects always behave as
        // fill-side paints; Form XObjects honour their
        // own internal ExtGState changes (the snapshot
        // captures the page-level state, the Form runs
        // recursively, and the apply blends the Form's
        // contribution against the captured backdrop).
        //
        // Per-subtype dispatch for the post-Do colour-
        // lane modulators: Image / ImageMask XObjects do
        // NOT execute their own paint operators — their
        // pixel data is painted using the outer
        // graphics state, so the post-Do CMYK compose,
        // overprint and spot-lane mirrors are how those
        // lanes learn about the contribution. Form
        // XObjects DO execute their own paint operators
        // (Fill / Stroke / FillStroke / Do / ShowText /
        // shading), each of which runs its own per-
        // paint sidecar mirror with the FORM's gs at
        // the time of the paint. Re-applying the outer
        // gs's CMYK / overprint / spot mirror after a
        // Form Do would composite the form's region
        // again with whatever colour the OUTER gs had,
        // double-counting (and, when the outer colour
        // differs from the form's, overwriting the
        // form's mirror writes — the QA-6 / QA-6-DIAG-2
        // failure mode where outer /K's iteration 2
        // /Inner Do lost the inner Form's spot
        // contribution). SMask attenuation always
        // applies — an outer /SMask gs in effect at the
        // Do attaches to the Do's entire region
        // regardless of how the inner produced its
        // pixels. ~keep
        let smask_snap = if group_boundary {
            None
        } else {
            self.paint_mask_snapshots(pixmap, gs_clone)?
        };
        let overprint_snap = if is_form {
            None
        } else {
            self.overprint_snapshot(pixmap, gs_clone, true)
        };
        let cmyk_compose_snap = if is_form {
            None
        } else {
            self.cmyk_compose_snapshot(pixmap, gs_clone, doc, true)
        };
        let spot_snap = if is_form {
            None
        } else {
            self.spot_paint_snapshot(pixmap, gs_clone, true)
        };
        Ok(PaintModulatorSnapshots {
            process_path: None,
            cmyk_compose: cmyk_compose_snap,
            overprint: overprint_snap,
            spot: spot_snap,
            smask: smask_snap,
        })
    }

    fn seed_shading_spot_inks(
        &mut self,
        gs_clone: &mut GraphicsState,
        name: &str,
        resources: &Object,
        doc: &PdfDocument,
    ) {
        // §8.7.4 + §11.7.3: when the shading's
        // /ColorSpace is /Separation or non-process
        // /DeviceN, surface the ink-name list (paired
        // with the /Function /C0 endpoint tints) onto
        // `gs_clone.fill_spot_inks` so the spot mirror
        // sees a non-empty source ink set and fires.
        // Without this the shading paint silently
        // bypasses the spot mirror because the gating
        // (`spot_paint_active`) checks
        // `gs.fill_spot_inks`, which is otherwise
        // populated only by `cs`/`scn` colour-set
        // operators — none of which fire before `sh`. ~keep
        if !self.spot_paint_active(&*gs_clone, true) && self.cmyk_sidecar.is_some() {
            if let Some(inks) = self.resolve_shading_spot_inks(name, resources, doc) {
                if !inks.is_empty() {
                    gs_clone.fill_spot_inks = inks;
                }
            }
        }
    }
}
