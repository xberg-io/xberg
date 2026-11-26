//! Text operator arms (`BT`, `ET`, `Td`, `TD`, `T*`, `Tf`, `Tj`, `'`, `TJ`, `"`) of the operator loop.

use super::*;
use crate::content::operators::TextElement;

impl PageRenderer {
    pub(super) fn op_end_text(&mut self, st: &mut OpLoopState<'_>) {
        st.in_text_object = false;
        // WS1.5b — apply the accumulated text clip path (Tr 4–7).
        // If no clip-mode text was shown the accumulator is None and
        // ET behaves exactly as before. An all-transparent
        // accumulator (e.g. every glyph was whitespace or lacked an
        // outline) is treated as degenerate and leaves the clip
        // unchanged rather than collapsing it to empty. ~keep
        if let Some(scratch) = st.text_clip_accum.take() {
            let has_coverage = scratch.data().as_chunks::<4>().0.iter().any(|px| px[3] != 0);
            if has_coverage {
                let text_mask = tiny_skia::Mask::from_pixmap(scratch.as_ref(), tiny_skia::MaskType::Alpha);
                // Intersect (logical AND) the glyph silhouette with
                // the current scope's clip so subsequent content is
                // confined to the text shape *within* the existing
                // clip — never widened past it. ~keep
                if let Some(slot) = st.clip_stack.last_mut() {
                    let existing = slot.take();
                    *slot = Some(std::sync::Arc::new(intersect_with_inherited(
                        text_mask,
                        existing.as_deref(),
                    )));
                }
            }
        }
    }

    pub(super) fn op_tj(
        &mut self,
        pixmap: &mut Pixmap,
        st: &mut OpLoopState<'_>,
        env: OpEnv<'_>,
        text: &[u8],
    ) -> Result<()> {
        // `Tj` is the show the text-contrast probe samples.
        self.show_text_run(pixmap, st, env, TextRun::String(text), None, Some(text))
    }

    pub(super) fn op_quote(
        &mut self,
        pixmap: &mut Pixmap,
        st: &mut OpLoopState<'_>,
        env: OpEnv<'_>,
        text: &[u8],
    ) -> Result<()> {
        if st.in_text_object {
            // Quote (') is T* followed by Tj — always advance line ~keep
            Self::advance_to_next_line(st.gs_stack.current_mut());
            self.show_text_run(pixmap, st, env, TextRun::String(text), Some("' (Quote)"), None)?;
        }
        Ok(())
    }

    pub(super) fn op_tj_array(
        &mut self,
        pixmap: &mut Pixmap,
        st: &mut OpLoopState<'_>,
        env: OpEnv<'_>,
        array: &[TextElement],
    ) -> Result<()> {
        self.show_text_run(pixmap, st, env, TextRun::Array(array), Some("TJ"), None)
    }

    pub(super) fn op_double_quote(
        &mut self,
        pixmap: &mut Pixmap,
        st: &mut OpLoopState<'_>,
        env: OpEnv<'_>,
        word_space: f32,
        char_space: f32,
        text: &[u8],
    ) -> Result<()> {
        if st.in_text_object {
            // Double Quote (") always updates state ~keep
            let gs_mut = st.gs_stack.current_mut();
            gs_mut.word_space = word_space;
            gs_mut.char_space = char_space;
            Self::advance_to_next_line(gs_mut);
            self.show_text_run(pixmap, st, env, TextRun::String(text), Some("\" (DoubleQuote)"), None)?;
        }
        Ok(())
    }

    /// Shared body of the text-showing operators (`Tj`, `'`, `TJ`, `"`).
    ///
    /// Runs the Type 3 fast path, the `Tr` 4–7 clip accumulation, the modulated
    /// render and the text-matrix advance. Pre-steps owned by one operator
    /// (`'`/`"` line advance, `"` `Tw`/`Tc`) stay in its arm. The whole run is
    /// rendered in one call, so a `TJ` array's numeric offsets only adjust
    /// positioning; they cannot change the active colour mid-string.
    ///
    /// `trace_label` is the operator name for the trace line; `None` keeps
    /// `Tj`'s untraced path. `contrast_literal` is the `Tj` string whose paint
    /// the text-contrast probe samples.
    fn show_text_run(
        &mut self,
        pixmap: &mut Pixmap,
        st: &mut OpLoopState<'_>,
        env: OpEnv<'_>,
        run: TextRun<'_>,
        trace_label: Option<&str>,
        contrast_literal: Option<&[u8]>,
    ) -> Result<()> {
        if !st.in_text_object {
            return Ok(());
        }
        // Type 3 fonts have no outline program; each glyph is a
        // CharProcs content stream painted under FontMatrix ×
        // text-space × CTM (ISO 32000-1 §9.6.5). It is handled
        // here because it re-enters the content-stream renderer. ~keep
        if self.try_show_type3_run(pixmap, st, env, run)? {
            return Ok(());
        }
        let advance = if st.excluded_layer_depth == 0 {
            self.paint_text_run(pixmap, st, env, run, trace_label, contrast_literal)?
        } else {
            self.measure_text_run(run, st.gs_stack.current())
        };

        // The rasterizer returns a scalar magnitude along the
        // active writing axis. advance_text_matrix routes it
        // to x (WMode 0) or y (WMode 1), keeping the axis
        // swap in exactly one place. ~keep
        st.gs_stack.current_mut().advance_text_matrix(advance);
        Ok(())
    }

    fn try_show_type3_run(
        &mut self,
        pixmap: &mut Pixmap,
        st: &mut OpLoopState<'_>,
        env: OpEnv<'_>,
        run: TextRun<'_>,
    ) -> Result<bool> {
        match run {
            TextRun::String(text) => self.try_show_type3_text(pixmap, st, env, text),
            TextRun::Array(array) => self.try_show_type3_tj_array(pixmap, st, env, array),
        }
    }

    fn paint_text_run(
        &mut self,
        pixmap: &mut Pixmap,
        st: &mut OpLoopState<'_>,
        env: OpEnv<'_>,
        run: TextRun<'_>,
        trace_label: Option<&str>,
        contrast_literal: Option<&[u8]>,
    ) -> Result<f32> {
        let OpEnv {
            base_transform,
            doc,
            page_num,
            resources,
        } = env;
        let gs = st.gs_stack.current();
        let clip = st.clip_stack.last().and_then(|c| c.as_deref());
        let transform = combine_transforms(base_transform, &gs.ctm);
        if let Some(label) = trace_label {
            tracing::trace!(
                "{}: rendering {} at Tm=[{}, {}, {}, {}, {}, {}]",
                label,
                text_run_noun(run),
                gs.text_matrix.a,
                gs.text_matrix.b,
                gs.text_matrix.c,
                gs.text_matrix.d,
                gs.text_matrix.e,
                gs.text_matrix.f
            );
        }
        // WS1.5b — modes 4–7 add this show's glyph
        // outlines to the text clip path (applied at ET).
        // Gated here so modes 0–3 pay nothing. ~keep
        if gs.render_mode >= 4 {
            self.accumulate_text_clip_run(&mut st.text_clip_accum, pixmap, run, transform, gs, env);
        }
        let contrast_samples = contrast_literal.and_then(|literal| {
            self.text_contrast_probe
                .as_mut()
                .and_then(|probe| probe.capture(pixmap, literal, base_transform))
        });
        let text_paint = GroupTextPaint {
            gs,
            doc,
            resources,
            clip,
            transform,
        };
        // A shading pattern paints the glyphs through their own coverage; the advance still comes
        // from the rasteriser so the text matrix keeps moving.
        let through_pattern = if self.text_takes_a_fill_pattern(gs) {
            let soft_mask = self.paint_mask_snapshots(pixmap, gs)?;
            let coverage_gs = Self::coverage_only_gs(gs);
            let mut scratch = Pixmap::new(pixmap.width(), pixmap.height());
            let advance = match scratch.as_mut() {
                Some(scratch) => Some(self.draw_text(
                    scratch,
                    run,
                    GroupTextPaint {
                        gs: &coverage_gs,
                        ..text_paint
                    },
                    None,
                )?),
                None => None,
            };
            match (scratch, advance) {
                (Some(scratch), Some(advance)) => {
                    let mask = tiny_skia::Mask::from_pixmap(scratch.as_ref(), tiny_skia::MaskType::Alpha);
                    let painted = self.paint_pattern_through_mask_from_resources(
                        pixmap,
                        &mask,
                        base_transform,
                        gs,
                        doc,
                        resources,
                    )?;
                    // Only a painted pattern is masked here; the fallback below masks its own paint. ~keep
                    if let (true, Some(snapshot)) = (painted, soft_mask) {
                        self.apply_paint_mask(
                            pixmap,
                            snapshot,
                            PaintModulatorContext {
                                gs,
                                doc,
                                page_num,
                                resources,
                                base_transform,
                                fill_side: true,
                                knockout_coverage: None,
                            },
                        )?;
                    }
                    painted.then_some(advance)
                }
                _ => None,
            }
        } else {
            None
        };
        let advance = match through_pattern {
            Some(advance) => advance,
            None => self.render_text_with_modulators(pixmap, run, text_paint, page_num, base_transform)?,
        };
        if let Some((probe, samples)) = self.text_contrast_probe.as_mut().zip(contrast_samples) {
            probe.observe(pixmap, samples);
        }
        Ok(advance)
    }

    fn accumulate_text_clip_run(
        &self,
        accum: &mut Option<Pixmap>,
        pixmap: &Pixmap,
        run: TextRun<'_>,
        transform: Transform,
        gs: &GraphicsState,
        env: OpEnv<'_>,
    ) {
        let OpEnv { doc, resources, .. } = env;
        match run {
            TextRun::String(text) => self.accumulate_text_clip_tj(
                accum,
                pixmap.width(),
                pixmap.height(),
                text,
                transform,
                gs,
                resources,
                doc,
            ),
            TextRun::Array(array) => self.accumulate_text_clip_tj_array(
                accum,
                pixmap.width(),
                pixmap.height(),
                array,
                transform,
                gs,
                resources,
                doc,
            ),
        }
    }

    fn measure_text_run(&self, run: TextRun<'_>, gs: &GraphicsState) -> f32 {
        match run {
            TextRun::String(text) => self.text_rasterizer.measure_text(text, gs, &self.fonts),
            TextRun::Array(array) => self.text_rasterizer.measure_tj_array(array, gs, &self.fonts),
        }
    }

    pub(super) fn op_begin_text(&mut self, st: &mut OpLoopState<'_>) {
        st.in_text_object = true;
        // Start each text object with a clean text-clip path
        // (§9.4.1: the text clip path is reset at BT and applied
        // at ET). Any leftover from a malformed/unterminated prior
        // block is discarded here. ~keep
        st.text_clip_accum = None;
        let gs = st.gs_stack.current_mut();
        gs.text_matrix = Matrix::identity();
        gs.text_line_matrix = Matrix::identity();
        tracing::trace!("BT (BeginText)");
    }

    pub(super) fn op_td(&mut self, st: &mut OpLoopState<'_>, tx: f32, ty: f32) {
        if st.in_text_object {
            let gs = st.gs_stack.current_mut();
            let translation = Matrix::translation(tx, ty);
            gs.text_line_matrix = translation.multiply(&gs.text_line_matrix);
            gs.text_matrix = gs.text_line_matrix;
            tracing::trace!("Td: [{}, {}], text_matrix now: {:?}", tx, ty, gs.text_matrix);
        }
    }

    pub(super) fn op_td_set_leading(&mut self, st: &mut OpLoopState<'_>, tx: f32, ty: f32) {
        if st.in_text_object {
            let gs = st.gs_stack.current_mut();
            gs.leading = -ty;
            let translation = Matrix::translation(tx, ty);
            gs.text_line_matrix = translation.multiply(&gs.text_line_matrix);
            gs.text_matrix = gs.text_line_matrix;
            tracing::trace!("TD: [{}, {}], text_matrix now: {:?}", tx, ty, gs.text_matrix);
        }
    }

    pub(super) fn op_t_star(&mut self, st: &mut OpLoopState<'_>) {
        if st.in_text_object {
            let gs = st.gs_stack.current_mut();
            let leading = gs.leading;
            let translation = Matrix::translation(0.0, -leading);
            gs.text_line_matrix = translation.multiply(&gs.text_line_matrix);
            gs.text_matrix = gs.text_line_matrix;
            tracing::trace!("T*: text_matrix now: {:?}", gs.text_matrix);
        }
    }

    pub(super) fn op_tf(&mut self, st: &mut OpLoopState<'_>, font: &str, size: f32) {
        // Cache the font's writing mode on the graphics state so
        // the rasterizer hot path can branch on a single
        // primitive read instead of dereferencing the FontInfo
        // through the cache for every glyph. ~keep
        let wmode = self.fonts.get(font).map(|f| f.wmode).unwrap_or(0);
        let gs = st.gs_stack.current_mut();
        gs.font_name = Some(font.to_string());
        gs.font_size = size;
        gs.text_wmode = wmode;
    }

    fn try_show_type3_text(
        &mut self,
        pixmap: &mut Pixmap,
        st: &mut OpLoopState<'_>,
        env: OpEnv<'_>,
        text: &[u8],
    ) -> Result<bool> {
        let OpEnv {
            base_transform,
            doc,
            page_num,
            resources,
        } = env;
        if self.current_font_is_type3(st.gs_stack.current()) {
            let advance = if st.excluded_layer_depth == 0 {
                let gs_snap = st.gs_stack.current().clone();
                self.render_type3_text(pixmap, text, base_transform, &gs_snap, doc, page_num, resources)?
            } else {
                self.text_rasterizer
                    .measure_text(text, st.gs_stack.current(), &self.fonts)
            };
            st.gs_stack.current_mut().advance_text_matrix(advance);
            return Ok(true);
        }
        Ok(false)
    }

    /// `TJ` counterpart of [`Self::try_show_type3_text`]: same Type 3 fast
    /// path, one array show at a time.
    fn try_show_type3_tj_array(
        &mut self,
        pixmap: &mut Pixmap,
        st: &mut OpLoopState<'_>,
        env: OpEnv<'_>,
        array: &[TextElement],
    ) -> Result<bool> {
        let OpEnv {
            base_transform,
            doc,
            page_num,
            resources,
        } = env;
        if self.current_font_is_type3(st.gs_stack.current()) {
            let advance = if st.excluded_layer_depth == 0 {
                let gs_snap = st.gs_stack.current().clone();
                self.render_type3_tj_array(pixmap, array, base_transform, &gs_snap, doc, page_num, resources)?
            } else {
                self.text_rasterizer
                    .measure_tj_array(array, st.gs_stack.current(), &self.fonts)
            };
            st.gs_stack.current_mut().advance_text_matrix(advance);
            return Ok(true);
        }
        Ok(false)
    }

    fn advance_to_next_line(gs_mut: &mut GraphicsState) {
        let leading = gs_mut.leading;
        let translation = Matrix::translation(0.0, -leading);
        gs_mut.text_line_matrix = translation.multiply(&gs_mut.text_line_matrix);
        gs_mut.text_matrix = gs_mut.text_line_matrix;
    }
}

fn text_run_noun(run: TextRun<'_>) -> &'static str {
    match run {
        TextRun::String(_) => "text",
        TextRun::Array(_) => "array",
    }
}
