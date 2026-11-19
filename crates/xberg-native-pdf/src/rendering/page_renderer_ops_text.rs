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
                    *slot = Some(intersect_with_inherited(text_mask, existing.as_ref()));
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
        let OpEnv {
            base_transform,
            doc,
            page_num,
            resources,
        } = env;
        if st.in_text_object {
            // Type 3 fonts have no outline program; each glyph is a
            // CharProcs content stream painted under FontMatrix ×
            // text-space × CTM (ISO 32000-1 §9.6.5). It is handled
            // here because it re-enters the content-stream renderer. ~keep
            if self.try_show_type3_text(pixmap, st, env, text)? {
                return Ok(());
            }
            let gs = st.gs_stack.current();
            let advance = if st.excluded_layer_depth == 0 {
                let clip = st.clip_stack.last().and_then(|c| c.as_ref());
                let transform = combine_transforms(base_transform, &gs.ctm);
                // WS1.5b — modes 4–7 add this show's glyph
                // outlines to the text clip path (applied at ET).
                // Gated here so modes 0–3 pay nothing. ~keep
                if gs.render_mode >= 4 {
                    self.accumulate_text_clip_tj(
                        &mut st.text_clip_accum,
                        pixmap.width(),
                        pixmap.height(),
                        text,
                        transform,
                        gs,
                        resources,
                        doc,
                    );
                }
                let contrast_samples = self
                    .text_contrast_probe
                    .as_mut()
                    .and_then(|probe| probe.capture(pixmap, text, base_transform));
                let adv = self.render_text_with_modulators(
                    pixmap,
                    TextRun::String(text),
                    GroupTextPaint {
                        gs,
                        doc,
                        resources,
                        clip,
                        transform,
                    },
                    page_num,
                    base_transform,
                )?;
                if let Some((probe, samples)) = self.text_contrast_probe.as_mut().zip(contrast_samples) {
                    probe.observe(pixmap, samples);
                }
                adv
            } else {
                self.text_rasterizer.measure_text(text, gs, &self.fonts)
            };

            // The rasterizer returns a scalar magnitude along the
            // active writing axis. advance_text_matrix routes it
            // to x (WMode 0) or y (WMode 1), keeping the axis
            // swap in exactly one place. ~keep
            st.gs_stack.current_mut().advance_text_matrix(advance);
        }
        Ok(())
    }

    pub(super) fn op_quote(
        &mut self,
        pixmap: &mut Pixmap,
        st: &mut OpLoopState<'_>,
        env: OpEnv<'_>,
        text: &[u8],
    ) -> Result<()> {
        let OpEnv {
            base_transform,
            doc,
            page_num,
            resources,
        } = env;
        if st.in_text_object {
            // Quote (') is T* followed by Tj — always advance line ~keep
            let gs_mut = st.gs_stack.current_mut();
            Self::advance_to_next_line(gs_mut);

            if self.try_show_type3_text(pixmap, st, env, text)? {
                return Ok(());
            }

            let gs = st.gs_stack.current();
            let advance = if st.excluded_layer_depth == 0 {
                let clip = st.clip_stack.last().and_then(|c| c.as_ref());
                let transform = combine_transforms(base_transform, &gs.ctm);
                tracing::trace!(
                    "' (Quote): rendering text at Tm=[{}, {}, {}, {}, {}, {}]",
                    gs.text_matrix.a,
                    gs.text_matrix.b,
                    gs.text_matrix.c,
                    gs.text_matrix.d,
                    gs.text_matrix.e,
                    gs.text_matrix.f
                );
                if gs.render_mode >= 4 {
                    self.accumulate_text_clip_tj(
                        &mut st.text_clip_accum,
                        pixmap.width(),
                        pixmap.height(),
                        text,
                        transform,
                        gs,
                        resources,
                        doc,
                    );
                }
                // Same shape as `Tj`. `'` is `T* Tj` per
                // ISO 32000-1; the resolved colour depends only
                // on the prior colour-setting ops, so the resolve
                // happens here, not inside `T*`. ~keep
                self.render_text_with_modulators(
                    pixmap,
                    TextRun::String(text),
                    GroupTextPaint {
                        gs,
                        doc,
                        resources,
                        clip,
                        transform,
                    },
                    page_num,
                    base_transform,
                )?
            } else {
                self.text_rasterizer.measure_text(text, gs, &self.fonts)
            };

            st.gs_stack.current_mut().advance_text_matrix(advance);
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
        let OpEnv {
            base_transform,
            doc,
            page_num,
            resources,
        } = env;
        if st.in_text_object {
            if self.current_font_is_type3(st.gs_stack.current()) {
                let advance = if st.excluded_layer_depth == 0 {
                    let gs_snap = st.gs_stack.current().clone();
                    self.render_type3_tj_array(pixmap, array, base_transform, &gs_snap, doc, page_num, resources)?
                } else {
                    self.text_rasterizer
                        .measure_tj_array(array, st.gs_stack.current(), &self.fonts)
                };
                st.gs_stack.current_mut().advance_text_matrix(advance);
                return Ok(());
            }
            let gs = st.gs_stack.current();
            let advance = if st.excluded_layer_depth == 0 {
                let clip = st.clip_stack.last().and_then(|c| c.as_ref());
                let transform = combine_transforms(base_transform, &gs.ctm);
                tracing::trace!(
                    "TJ: rendering array at Tm=[{}, {}, {}, {}, {}, {}]",
                    gs.text_matrix.a,
                    gs.text_matrix.b,
                    gs.text_matrix.c,
                    gs.text_matrix.d,
                    gs.text_matrix.e,
                    gs.text_matrix.f
                );
                if gs.render_mode >= 4 {
                    self.accumulate_text_clip_tj_array(
                        &mut st.text_clip_accum,
                        pixmap.width(),
                        pixmap.height(),
                        array,
                        transform,
                        gs,
                        resources,
                        doc,
                    );
                }
                // Resolve once for the whole `TJ` array — the
                // numeric offsets inside `array` only adjust
                // positioning; they cannot alter the active
                // colour mid-string. The rasteriser threads the
                // override into the per-element `render_text`
                // calls so the colour propagates without an
                // operator-arm-side clone of `gs`. ~keep
                self.render_text_with_modulators(
                    pixmap,
                    TextRun::Array(array),
                    GroupTextPaint {
                        gs,
                        doc,
                        resources,
                        clip,
                        transform,
                    },
                    page_num,
                    base_transform,
                )?
            } else {
                self.text_rasterizer.measure_tj_array(array, gs, &self.fonts)
            };

            st.gs_stack.current_mut().advance_text_matrix(advance);
        }
        Ok(())
    }

    pub(super) fn op_double_quote(
        &mut self,
        pixmap: &mut Pixmap,
        st: &mut OpLoopState<'_>,
        env: OpEnv<'_>,
        word_space: &f32,
        char_space: &f32,
        text: &[u8],
    ) -> Result<()> {
        let OpEnv {
            base_transform,
            doc,
            page_num,
            resources,
        } = env;
        if st.in_text_object {
            // Double Quote (") always updates state ~keep
            let gs_mut = st.gs_stack.current_mut();
            gs_mut.word_space = *word_space;
            gs_mut.char_space = *char_space;

            Self::advance_to_next_line(gs_mut);

            if self.try_show_type3_text(pixmap, st, env, text)? {
                return Ok(());
            }

            let gs = st.gs_stack.current();
            let advance = if st.excluded_layer_depth == 0 {
                let clip = st.clip_stack.last().and_then(|c| c.as_ref());
                let transform = combine_transforms(base_transform, &gs.ctm);
                tracing::trace!(
                    "\" (DoubleQuote): rendering text at Tm=[{}, {}, {}, {}, {}, {}]",
                    gs.text_matrix.a,
                    gs.text_matrix.b,
                    gs.text_matrix.c,
                    gs.text_matrix.d,
                    gs.text_matrix.e,
                    gs.text_matrix.f
                );
                if gs.render_mode >= 4 {
                    self.accumulate_text_clip_tj(
                        &mut st.text_clip_accum,
                        pixmap.width(),
                        pixmap.height(),
                        text,
                        transform,
                        gs,
                        resources,
                        doc,
                    );
                }
                // `"` is equivalent to setting Tw, Tc, then
                // `T* Tj`. Tw/Tc are state-only and don't
                // influence the resolved colour, so the resolve
                // happens immediately before painting just like
                // in `Tj` / `'`. ~keep
                self.render_text_with_modulators(
                    pixmap,
                    TextRun::String(text),
                    GroupTextPaint {
                        gs,
                        doc,
                        resources,
                        clip,
                        transform,
                    },
                    page_num,
                    base_transform,
                )?
            } else {
                self.text_rasterizer.measure_text(text, gs, &self.fonts)
            };

            st.gs_stack.current_mut().advance_text_matrix(advance);
        }
        Ok(())
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

    pub(super) fn op_td(&mut self, st: &mut OpLoopState<'_>, tx: &f32, ty: &f32) {
        if st.in_text_object {
            let gs = st.gs_stack.current_mut();
            let translation = Matrix::translation(*tx, *ty);
            gs.text_line_matrix = translation.multiply(&gs.text_line_matrix);
            gs.text_matrix = gs.text_line_matrix;
            tracing::trace!("Td: [{}, {}], text_matrix now: {:?}", tx, ty, gs.text_matrix);
        }
    }

    pub(super) fn op_td_set_leading(&mut self, st: &mut OpLoopState<'_>, tx: &f32, ty: &f32) {
        if st.in_text_object {
            let gs = st.gs_stack.current_mut();
            gs.leading = -(*ty);
            let translation = Matrix::translation(*tx, *ty);
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

    pub(super) fn op_tf(&mut self, st: &mut OpLoopState<'_>, font: &String, size: &f32) {
        // Cache the font's writing mode on the graphics state so
        // the rasterizer hot path can branch on a single
        // primitive read instead of dereferencing the FontInfo
        // through the cache for every glyph. ~keep
        let wmode = self.fonts.get(font).map(|f| f.wmode).unwrap_or(0);
        let gs = st.gs_stack.current_mut();
        gs.font_name = Some(font.clone());
        gs.font_size = *size;
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

    fn advance_to_next_line(gs_mut: &mut GraphicsState) {
        let leading = gs_mut.leading;
        let translation = Matrix::translation(0.0, -leading);
        gs_mut.text_line_matrix = translation.multiply(&gs_mut.text_line_matrix);
        gs_mut.text_matrix = gs_mut.text_line_matrix;
    }
}
