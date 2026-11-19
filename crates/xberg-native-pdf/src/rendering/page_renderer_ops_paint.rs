//! Path-painting, clipping and path-ending operator arms (`S`, `s`, `f`, `f*`, `B`, `b`, `B*`, `b*`,
//! `W`, `W*`, `n`) of the operator loop.

use super::*;

/// Per-path inputs shared by the fill and stroke sub-helpers.
#[derive(Clone, Copy)]
struct PathPaint<'a> {
    path: &'a tiny_skia::Path,
    transform: Transform,
    clip: Option<&'a tiny_skia::Mask>,
    gs: &'a GraphicsState,
    render_gs: &'a GraphicsState,
    fill_rule: tiny_skia::FillRule,
    env: OpEnv<'a>,
}

impl PageRenderer {
    pub(super) fn op_stroke(
        &mut self,
        pixmap: &mut Pixmap,
        st: &mut OpLoopState<'_>,
        env: OpEnv<'_>,
        op: &Operator,
    ) -> Result<()> {
        let OpEnv {
            base_transform,
            doc,
            page_num,
            resources,
        } = env;
        if st.excluded_layer_depth == 0 {
            apply_pending_clip(
                &mut st.pending_clip,
                &mut st.clip_stack,
                pixmap,
                base_transform,
                &st.gs_stack,
            );
            let clip = st.clip_stack.last().and_then(|c| c.as_ref());
            // ISO 32000-1 §8.5.3.2 Table 60: `s` is exactly `h S`. ~keep
            if matches!(op, Operator::CloseStroke) {
                st.current_path.close();
            }
            if let Some(path) = std::mem::replace(&mut st.current_path, PathBuilder::new()).finish() {
                let gs_clone = st.gs_stack.current().clone();
                // Stroke side mirrors the path-fill routing —
                // route through the pipeline so Type 4 Separation
                // strokes resolve correctly. Line width / cap /
                // join / dash come from the cloned `gs`
                // unchanged, so the stroke geometry is unaffected
                // by the colour splice. ~keep
                let spliced = self.pipeline_resolve_paint_gs(doc, &gs_clone, PipelinePaintKind::PathStroke);
                let render_gs: &GraphicsState = spliced.as_ref().unwrap_or(&gs_clone);
                let transform = combine_transforms(base_transform, &gs_clone.ctm);
                self.charge_path_work(pixmap, &path, transform, &gs_clone, op)?;
                let smask_snap = self.paint_mask_snapshots(pixmap, &gs_clone)?;
                let overprint_snap = self.overprint_snapshot(pixmap, &gs_clone, false);
                let cmyk_compose_snap = self.cmyk_compose_snapshot(pixmap, &gs_clone, doc, false);
                let cmyk_sidecar_snap = self.cmyk_sidecar_snapshot(pixmap, &gs_clone, false);
                let rgb_sidecar_snap = self.cmyk_sidecar_snapshot_for_rgb_paint(pixmap, &gs_clone, false);
                let cmyk_coverage = self.rasterise_stroke_coverage(&path, transform, &gs_clone, clip);
                self.path_rasterizer
                    .stroke_path_clipped(pixmap, &path, transform, render_gs, clip);
                self.apply_sidecar_paint_modulators(
                    pixmap,
                    SidecarPaintModulatorSnapshots {
                        cmyk_compose: cmyk_compose_snap,
                        overprint: overprint_snap,
                        cmyk_sidecar: cmyk_sidecar_snap,
                        rgb_sidecar: rgb_sidecar_snap,
                        smask: smask_snap,
                    },
                    cmyk_coverage.as_deref(),
                    PaintModulatorContext {
                        gs: &gs_clone,
                        doc,
                        page_num,
                        resources,
                        base_transform,
                        fill_side: false,
                        knockout_coverage: None,
                    },
                )?;
            }
        } else {
            let _ = std::mem::replace(&mut st.current_path, PathBuilder::new()).finish();
        }
        Ok(())
    }

    pub(super) fn op_fill(
        &mut self,
        pixmap: &mut Pixmap,
        st: &mut OpLoopState<'_>,
        env: OpEnv<'_>,
        op: &Operator,
    ) -> Result<()> {
        let OpEnv {
            base_transform,
            doc,
            page_num,
            resources,
        } = env;
        if st.excluded_layer_depth == 0 {
            apply_pending_clip(
                &mut st.pending_clip,
                &mut st.clip_stack,
                pixmap,
                base_transform,
                &st.gs_stack,
            );
            let clip = st.clip_stack.last().and_then(|c| c.as_ref());
            if let Some(path) = std::mem::replace(&mut st.current_path, PathBuilder::new()).finish() {
                let gs_clone = st.gs_stack.current().clone();
                // Resolve the active fill colour through the
                // pipeline (PostScript Type 4 tint transforms,
                // ICCBased N=4, etc.) and splice the resulting
                // RGBA into a transient GraphicsState copy the
                // rasteriser consumes. ~keep
                let spliced = self.pipeline_resolve_paint_gs(doc, &gs_clone, PipelinePaintKind::PathFill);
                let render_gs: &GraphicsState = spliced.as_ref().unwrap_or(&gs_clone);
                let transform = combine_transforms(base_transform, &gs_clone.ctm);
                self.charge_path_work(pixmap, &path, transform, &gs_clone, op)?;
                let paint = PathPaint {
                    path: &path,
                    transform,
                    clip,
                    gs: &gs_clone,
                    render_gs,
                    fill_rule: tiny_skia::FillRule::Winding,
                    env,
                };
                // §8.7.3: a Pattern-space fill routes to the
                // tiling-pattern rasteriser first. When it paints
                // the region the solid-colour paint below is
                // skipped; unsupported/shading patterns return
                // false and fall through to the solid fallback. ~keep
                if gs_clone.fill_color_space == "Pattern"
                    && gs_clone.fill_pattern_name.is_some()
                    && self.fill_with_tiling_pattern(
                        pixmap,
                        &path,
                        base_transform,
                        transform,
                        tiny_skia::FillRule::Winding,
                        clip,
                        &gs_clone,
                        doc,
                        page_num,
                        resources,
                    )?
                {
                    // Painted by the tiling pattern. ~keep
                } else {
                    self.paint_plain_fill(pixmap, paint)?;
                }
            }
        } else {
            let _ = std::mem::replace(&mut st.current_path, PathBuilder::new()).finish();
        }
        Ok(())
    }

    pub(super) fn op_fill_stroke(
        &mut self,
        pixmap: &mut Pixmap,
        st: &mut OpLoopState<'_>,
        env: OpEnv<'_>,
        op: &Operator,
    ) -> Result<()> {
        let OpEnv {
            base_transform, doc, ..
        } = env;
        if st.excluded_layer_depth == 0 {
            apply_pending_clip(
                &mut st.pending_clip,
                &mut st.clip_stack,
                pixmap,
                base_transform,
                &st.gs_stack,
            );
            let clip = st.clip_stack.last().and_then(|c| c.as_ref());
            // ISO 32000-1 §8.5.3.1 Table 60: `b` and `b*` close
            // the path before fill+stroke. Neither parser
            // decomposes them, so the dispatcher must perform the
            // close itself or the final segment of an open subpath
            // will not be painted by the stroke. The same holds for
            // `s` via `Operator::CloseStroke`: only the byte-level
            // parser decomposes it, the object parser does not. ~keep
            if matches!(op, Operator::CloseFillStroke | Operator::CloseFillStrokeEvenOdd) {
                st.current_path.close();
            }
            if let Some(path) = std::mem::replace(&mut st.current_path, PathBuilder::new()).finish() {
                let gs_clone = st.gs_stack.current().clone();
                let transform = combine_transforms(base_transform, &gs_clone.ctm);
                self.charge_path_work(pixmap, &path, transform, &gs_clone, op)?;
                let fill_rule = if matches!(op, Operator::CloseFillStrokeEvenOdd) {
                    tiny_skia::FillRule::EvenOdd
                } else {
                    tiny_skia::FillRule::Winding
                };
                // Combos resolve fill and stroke independently
                // through the pipeline (two `PaintIntent`s per
                // operator). Each side falls back to the
                // GraphicsState's existing RGBA if its colour
                // can't be resolved, so a Type 4 Separation on
                // the fill side and a plain DeviceRGB on the
                // stroke side route correctly without
                // entangling the two.
                //
                // Single splice for both sides — the rasteriser
                // reads fill fields for the fill pass and stroke
                // fields for the stroke pass, so one clone with
                // both sides written is equivalent to two
                // single-side clones. ~keep
                let spliced = self.pipeline_resolve_paint_gs(doc, &gs_clone, PipelinePaintKind::PathFillStroke);
                let render_gs: &GraphicsState = spliced.as_ref().unwrap_or(&gs_clone);
                let paint = PathPaint {
                    path: &path,
                    transform,
                    clip,
                    gs: &gs_clone,
                    render_gs,
                    fill_rule: fill_rule,
                    env,
                };

                self.paint_fill_then_stroke(pixmap, paint)?;
            }
        } else {
            let _ = std::mem::replace(&mut st.current_path, PathBuilder::new()).finish();
        }
        Ok(())
    }

    pub(super) fn op_fill_even_odd(
        &mut self,
        pixmap: &mut Pixmap,
        st: &mut OpLoopState<'_>,
        env: OpEnv<'_>,
        op: &Operator,
    ) -> Result<()> {
        let OpEnv {
            base_transform, doc, ..
        } = env;
        if st.excluded_layer_depth == 0 {
            apply_pending_clip(
                &mut st.pending_clip,
                &mut st.clip_stack,
                pixmap,
                base_transform,
                &st.gs_stack,
            );
            let clip = st.clip_stack.last().and_then(|c| c.as_ref());
            if let Some(path) = std::mem::replace(&mut st.current_path, PathBuilder::new()).finish() {
                let gs_clone = st.gs_stack.current().clone();
                let transform = combine_transforms(base_transform, &gs_clone.ctm);
                self.charge_path_work(pixmap, &path, transform, &gs_clone, op)?;
                // One unified resolve covers both fill and the
                // optional stroke pass — for plain `f*` the
                // helper produces a fill-only splice; for
                // `B*`/`b*` both sides are spliced into the
                // same clone. Either way, the rasteriser reads
                // the side it needs from `render_gs`. ~keep
                let kind = if matches!(op, Operator::FillStrokeEvenOdd) {
                    PipelinePaintKind::PathFillStroke
                } else {
                    PipelinePaintKind::PathFill
                };
                let spliced = self.pipeline_resolve_paint_gs(doc, &gs_clone, kind);
                let render_gs: &GraphicsState = spliced.as_ref().unwrap_or(&gs_clone);
                let paint = PathPaint {
                    path: &path,
                    transform,
                    clip,
                    gs: &gs_clone,
                    render_gs,
                    fill_rule: tiny_skia::FillRule::EvenOdd,
                    env,
                };

                let fill_by_pattern = self.fill_by_tiling_pattern(pixmap, paint)?;

                if !fill_by_pattern {
                    self.paint_fill_side(pixmap, paint)?;
                }

                if matches!(op, Operator::FillStrokeEvenOdd) {
                    self.paint_stroke_side(pixmap, paint)?;
                }
            }
        } else {
            let _ = std::mem::replace(&mut st.current_path, PathBuilder::new()).finish();
        }
        Ok(())
    }

    pub(super) fn op_clip_non_zero(&mut self, st: &mut OpLoopState<'_>) {
        if st.excluded_layer_depth == 0 {
            if let Some(path) = st.current_path.clone().finish() {
                st.pending_clip = Some((path, tiny_skia::FillRule::Winding));
            }
        }
    }

    pub(super) fn op_clip_even_odd(&mut self, st: &mut OpLoopState<'_>) {
        if st.excluded_layer_depth == 0 {
            if let Some(path) = st.current_path.clone().finish() {
                st.pending_clip = Some((path, tiny_skia::FillRule::EvenOdd));
            }
        }
    }

    pub(super) fn op_end_path(&mut self, pixmap: &mut Pixmap, st: &mut OpLoopState<'_>, env: OpEnv<'_>) {
        let OpEnv { base_transform, .. } = env;
        if st.excluded_layer_depth == 0 {
            apply_pending_clip(
                &mut st.pending_clip,
                &mut st.clip_stack,
                pixmap,
                base_transform,
                &st.gs_stack,
            );
        } else {
            let _ = st.pending_clip.take();
        }
        st.current_path = PathBuilder::new();
    }

    fn fill_by_tiling_pattern(&mut self, pixmap: &mut Pixmap, paint: PathPaint<'_>) -> Result<bool> {
        let PathPaint {
            path,
            transform,
            clip,
            gs: gs_clone,
            fill_rule,
            env,
            ..
        } = paint;
        let OpEnv {
            base_transform,
            doc,
            page_num,
            resources,
        } = env;
        Ok(gs_clone.fill_color_space == "Pattern"
            && gs_clone.fill_pattern_name.is_some()
            && self.fill_with_tiling_pattern(
                pixmap,
                &path,
                base_transform,
                transform,
                fill_rule,
                clip,
                &gs_clone,
                doc,
                page_num,
                resources,
            )?)
    }

    fn paint_plain_fill(&mut self, pixmap: &mut Pixmap, paint: PathPaint<'_>) -> Result<()> {
        let PathPaint {
            path,
            transform,
            clip,
            gs: gs_clone,
            render_gs,
            env,
            ..
        } = paint;
        let OpEnv {
            base_transform,
            doc,
            page_num,
            resources,
        } = env;
        // §11.4.7 + §11.7.4: snapshot before the
        // paint so the post-paint modulators can
        // blend the backdrop (snapshot) with the
        // painted result. ~keep
        let smask_snap = self.paint_mask_snapshots(pixmap, &gs_clone)?;
        let overprint_snap = self.overprint_snapshot(pixmap, &gs_clone, true);
        let cmyk_compose_snap = self.cmyk_compose_snapshot(pixmap, &gs_clone, doc, true);
        let cmyk_sidecar_snap = self.cmyk_sidecar_snapshot(pixmap, &gs_clone, true);
        let rgb_sidecar_snap = self.cmyk_sidecar_snapshot_for_rgb_paint(pixmap, &gs_clone, true);
        let cmyk_coverage = self.rasterise_fill_coverage(&path, transform, tiny_skia::FillRule::Winding, clip);
        self.path_rasterizer
            .fill_path_clipped(pixmap, &path, transform, render_gs, tiny_skia::FillRule::Winding, clip);
        self.apply_sidecar_paint_modulators(
            pixmap,
            SidecarPaintModulatorSnapshots {
                cmyk_compose: cmyk_compose_snap,
                overprint: overprint_snap,
                cmyk_sidecar: cmyk_sidecar_snap,
                rgb_sidecar: rgb_sidecar_snap,
                smask: smask_snap,
            },
            cmyk_coverage.as_deref(),
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
        Ok(())
    }

    fn paint_fill_side(&mut self, pixmap: &mut Pixmap, paint: PathPaint<'_>) -> Result<()> {
        let PathPaint {
            path,
            transform,
            clip,
            gs: gs_clone,
            render_gs,
            fill_rule,
            env,
        } = paint;
        let OpEnv {
            base_transform,
            doc,
            page_num,
            resources,
        } = env;
        let fill_smask_snap = self.paint_mask_snapshots(pixmap, &gs_clone)?;
        let fill_overprint_snap = self.overprint_snapshot(pixmap, &gs_clone, true);
        let fill_cmyk_compose_snap = self.cmyk_compose_snapshot(pixmap, &gs_clone, doc, true);
        let process_path = self.group_path_snapshot(pixmap, &gs_clone, doc, true)?;
        let fill_spot_snap = self.spot_paint_snapshot(pixmap, &gs_clone, true);
        // §11.7.3 + §11.3.3 require per-pixel
        // coverage on every lane. The path-Fill
        // helper uses `rasterise_fill_coverage`;
        // the combo arm uses the same call so AA
        // edges receive fractional coverage and an
        // alternate-CS RGB collision with backdrop
        // does not mask the paint from the spot
        // mirror's diff branch. ~keep
        let fill_cmyk_coverage = self.rasterise_fill_coverage(&path, transform, fill_rule, clip);
        self.path_rasterizer
            .fill_path_clipped(pixmap, &path, transform, render_gs, fill_rule, clip);
        self.apply_paint_modulators(
            pixmap,
            PaintModulatorSnapshots {
                process_path,
                cmyk_compose: fill_cmyk_compose_snap,
                overprint: fill_overprint_snap,
                spot: fill_spot_snap,
                smask: fill_smask_snap,
            },
            fill_cmyk_coverage.as_deref(),
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
        Ok(())
    }

    fn paint_stroke_side(&mut self, pixmap: &mut Pixmap, paint: PathPaint<'_>) -> Result<()> {
        let PathPaint {
            path,
            transform,
            clip,
            gs: gs_clone,
            render_gs,
            env,
            ..
        } = paint;
        let OpEnv {
            base_transform,
            doc,
            page_num,
            resources,
        } = env;
        let stroke_smask_snap = self.paint_mask_snapshots(pixmap, &gs_clone)?;
        let stroke_overprint_snap = self.overprint_snapshot(pixmap, &gs_clone, false);
        let stroke_cmyk_compose_snap = self.cmyk_compose_snapshot(pixmap, &gs_clone, doc, false);
        let process_path = self.group_path_snapshot(pixmap, &gs_clone, doc, false)?;
        let stroke_spot_snap = self.spot_paint_snapshot(pixmap, &gs_clone, false);
        let stroke_cmyk_coverage = self.rasterise_stroke_coverage(&path, transform, &gs_clone, clip);
        self.path_rasterizer
            .stroke_path_clipped(pixmap, &path, transform, render_gs, clip);
        self.apply_paint_modulators(
            pixmap,
            PaintModulatorSnapshots {
                process_path,
                cmyk_compose: stroke_cmyk_compose_snap,
                overprint: stroke_overprint_snap,
                spot: stroke_spot_snap,
                smask: stroke_smask_snap,
            },
            stroke_cmyk_coverage.as_deref(),
            PaintModulatorContext {
                gs: &gs_clone,
                doc,
                page_num,
                resources,
                base_transform,
                fill_side: false,
                knockout_coverage: None,
            },
        )?;
        Ok(())
    }

    fn paint_fill_then_stroke(&mut self, pixmap: &mut Pixmap, paint: PathPaint<'_>) -> Result<()> {
        // §8.7.3: Pattern-space fills route to the tiling
        // rasteriser first; on success the solid fill side
        // is skipped (the stroke side still runs below). ~keep
        let fill_by_pattern = self.fill_by_tiling_pattern(pixmap, paint)?;

        // Fill side: snapshot before paint, paint,
        // then run compose-first / overprint / SMask
        // correctors against the fill-side gs fields.
        // The §11.7.4 + §11.4.7 + §11.4 rules apply
        // to combos exactly as they do to plain `f`
        // — the only difference here is the stroke
        // pass also lays paint on top, so each side
        // gets its own snapshot/apply cycle. ~keep
        if !fill_by_pattern {
            self.paint_fill_side(pixmap, paint)?;
        }

        self.paint_stroke_side(pixmap, paint)?;
        Ok(())
    }
}
