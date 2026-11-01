//! The content-stream operator dispatch.
//!
//! Split out of the parent's single 5,806-line `impl TextExtractor`, which made
//! `extractors/text.rs` 673 KiB — over the repository's 500 KiB file-safety limit.
//! A child module's `impl` is the same inherent impl and sees the parent's private
//! items unchanged. ~keep

use super::*;

impl<'doc> TextExtractor<'doc> {
    /// Execute a single operator.
    ///
    /// Updates the graphics state and extracts text as appropriate.
    pub(super) fn execute_operator(&mut self, op: Operator) -> Result<()> {
        match op {
            Operator::Tf { font, size } => {
                let same_font = {
                    let state = self.state_stack.current();
                    state.font_size == size && state.font_name.as_deref() == Some(font.as_str())
                };
                if !same_font {
                    // Flush Tj buffer before changing font — the buffer decodes bytes
                    // using the font set at creation time, so a font change requires a
                    // new buffer to avoid decoding with the wrong ToUnicode CMap. ~keep
                    self.flush_tj_span_buffer()?;

                    let current_font = self.fonts.get(&font).cloned();
                    self.set_cached_current_font(current_font);
                    // Cache wmode on the graphics state so the advance hot
                    // path branches on a single primitive read instead of
                    // dereferencing the FontInfo every glyph. ~keep
                    let new_wmode = self.cached_current_font.as_deref().map(|f| f.wmode).unwrap_or(0);

                    let state = self.state_stack.current_mut();
                    state.font_name = Some(font);
                    state.font_size = size;
                    state.text_wmode = new_wmode;
                }
            }

            Operator::Tm { a, b, c, d, e, f } => {
                // Optimization: batch character-by-character Tm+Tj patterns.
                // Many PDFs position each character with individual Tm+Tj operators.
                // If the new Tm is on the same line with the same transform,
                // keep accumulating into the existing buffer instead of flushing
                // (avoids creating thousands of 1-char TextSpans per page).
                // When merge_tm_tj_runs is false, every Tm always starts a fresh span.
                //
                // Glyph-jitter tolerance. Microsoft Word emits each
                // glyph in its own `BT Tm Tj ET` block with ±2.5–5pt
                // sinusoidal baseline jitter for broken-image placeholder
                // text. ISO 32000-1 §9.4 leaves logical reading order to
                // the extractor, so a baseline delta far smaller than the
                // line's own height is the SAME visual line — only a
                // delta on the order of the font size is a real line
                // break (body leading ≳ 1.0× font size). The previous
                // `f.round() as i32 ==` check tolerated only ±0.5pt
                // split jittered glyphs into separate Y-banded spans that
                // the reading-order sort then scrambled. Tolerance is
                // scale-relative (0.5× the text-space glyph height, ≥0.5pt
                // floor) so it is correct at any font size and still
                // splits genuine line breaks.
                //
                // The `f`/`e` tests below only mean "same line" and "forward"
                // while the run advances along +x; under a rotated matrix the
                // two axes swap. The added conjunct re-checks both along the
                // run's own writing axis (ISO 32000-1 §9.4.4). ~keep
                let cur_font_size = self.state_stack.current().font_size;
                let is_continuation = self.merging_config.merge_tm_tj_runs
                    && match self.tj_span_buffer {
                        Some(ref mut buffer)
                            if !buffer.is_empty()
                                && (f - buffer.start_matrix.f).abs()
                                    <= ((cur_font_size * buffer.start_matrix.d).abs() * 0.5).max(0.5)
                                && a == buffer.start_matrix.a
                                && b == buffer.start_matrix.b
                                && c == buffer.start_matrix.c
                                && d == buffer.start_matrix.d
                                && e >= buffer.start_matrix.e
                                && Self::advances_along_writing_axis(
                                    buffer.start_matrix,
                                    buffer.wmode,
                                    e,
                                    f,
                                    cur_font_size,
                                ) =>
                        {
                            // Fixed-pitch word-gap recovery (GH#1770). This
                            // continuation path glues consecutive Tm+Tj glyph
                            // runs into one buffer purely on baseline/transform
                            // geometry — no space-insertion heuristic runs here at
                            // all, unlike the cross-span merge path in
                            // `span_merging.rs`. A fixed-pitch form/receipt
                            // producer that renders one glyph per Tm/Tj and marks
                            // a word gap as a bare empty cell (no space glyph) was
                            // therefore ALWAYS glued regardless of gap size.
                            //
                            // Scoped narrowly to monospace horizontal runs: a
                            // fixed-pitch font's own space-glyph advance IS its
                            // character-cell pitch, so a gap of at least half a
                            // cell beyond the buffer's own accumulated advance can
                            // only be an empty cell — no glyph, proportional or
                            // otherwise, occupies less than its own cell in a
                            // fixed-pitch layout. Left at wmode 0 and gated on
                            // `is_monospace` so proportional-font Tm+Tj runs (a
                            // separate, differently-shaped gap problem) are
                            // untouched. ~keep
                            let expected_next_e = buffer.start_matrix.e + buffer.accumulated_width;
                            let raw_gap = e - expected_next_e;
                            if buffer.wmode == 0
                                && buffer.is_monospace
                                && !buffer.unicode.ends_with(|c: char| c.is_whitespace())
                                && let Some(font) = buffer.cached_font.as_ref()
                            {
                                let cell_pitch = (font.get_space_glyph_width() / 1000.0) * cur_font_size;
                                if cell_pitch > 0.0 && raw_gap > cell_pitch * 0.5 {
                                    tracing::trace!(target: LOG_TARGET,
                                        "Fixed-pitch word gap: inserting space (gap={:.2}pt, cell_pitch={:.2}pt)",
                                        raw_gap,
                                        cell_pitch
                                    );
                                    buffer.unicode.push(' ');
                                    buffer.char_widths.push(raw_gap);
                                }
                            }

                            // Same line, same transform, LTR progression →
                            // update width to reflect actual visual extent ~keep
                            buffer.accumulated_width = e - buffer.start_matrix.e;
                            true
                        }
                        _ => false,
                    };

                if !is_continuation {
                    self.flush_tj_span_buffer()?;
                }

                let state = self.state_stack.current_mut();
                state.text_matrix = Matrix { a, b, c, d, e, f };
                state.text_line_matrix = state.text_matrix;
            }
            Operator::Td { tx, ty } => {
                self.flush_tj_span_buffer()?;
                let state = self.state_stack.current_mut();
                // Per ISO 32000-1:2008 §9.4.2, Table 108:
                // Tlm_new = T(tx,ty) × Tlm_old
                // The translation is in text-line space, so it must be
                // pre-multiplied to be scaled by the existing Tlm transform. ~keep
                let tm = Matrix::translation(tx, ty);
                state.text_line_matrix = tm.multiply(&state.text_line_matrix);
                state.text_matrix = state.text_line_matrix;
            }
            Operator::TD { tx, ty } => {
                self.flush_tj_span_buffer()?;

                let state = self.state_stack.current_mut();
                state.leading = -ty;
                // Per ISO 32000-1:2008 §9.4.2: Tlm_new = T(tx,ty) × Tlm_old ~keep
                let tm = Matrix::translation(tx, ty);
                state.text_line_matrix = tm.multiply(&state.text_line_matrix);
                state.text_matrix = state.text_line_matrix;
            }
            Operator::TStar => {
                self.flush_tj_span_buffer()?;

                let leading = self.state_stack.current().leading;
                let state = self.state_stack.current_mut();
                // Per ISO 32000-1:2008 §9.4.2: Tlm_new = T(0,-TL) × Tlm_old ~keep
                let tm = Matrix::translation(0.0, -leading);
                state.text_line_matrix = tm.multiply(&state.text_line_matrix);
                state.text_matrix = state.text_line_matrix;
            }

            Operator::Tj { text } => {
                // Note: We do NOT skip /Artifact content here.
                // Many PDFs incorrectly mark page content as artifacts.
                // For tagged PDFs, the structure tree already excludes artifacts
                // via MCID mapping, so no filtering is needed at extractor level. ~keep

                // ActualText override
                // Per PDF Spec ISO 32000-1:2008, Section 14.9.4:
                // ActualText provides replacement text for the marked-content
                // SEQUENCE — emitted ONCE, no matter how many Tj operators
                // sit inside. The peek/mark pair below handles both first-Tj
                // (emit replacement) and subsequent-Tj (suppress entirely,
                // advance only) cases. ~keep
                let (current_at, already_emitted) = self.peek_current_actual_text();
                if let Some(actual_text) = current_at {
                    if already_emitted {
                        // Subsequent show-text inside the same MC scope:
                        // glyphs are already covered by the one replacement
                        // that fired on the first Tj. Advance positioning so
                        // any later, OUTER-scope show-text lands correctly,
                        // but emit nothing. ~keep
                        let w = self.advance_position_for_string(&text, true)?;
                        if let Some(ref mut buffer) = self.tj_span_buffer {
                            buffer.accumulated_width += w;
                        }
                    } else {
                        tracing::trace!(target: LOG_TARGET, "Tj operator: emitting MC-scope ActualText '{}'", actual_text);
                        self.mark_actual_text_emitted();
                        if self.extract_spans {
                            // Use ActualText in span mode — push pre-decoded
                            // Unicode directly into the buffer, bypassing
                            // font character mapping. ~keep
                            if self.tj_span_buffer.is_none() {
                                self.tj_span_buffer = Some(TjBuffer::new_with_provenance_cache(
                                    self.state_stack.current(),
                                    self.current_mcid,
                                    self.cached_current_font.clone(),
                                    &self.font_provenance_cache,
                                ));
                            }
                            if let Some(ref mut buffer) = self.tj_span_buffer {
                                buffer.unicode.push_str(&actual_text);
                            }
                        } else {
                            // Character mode: show_text maps through font, but ActualText
                            // is already decoded. Fall back to show_text for positioning. ~keep
                            self.show_text(actual_text.as_bytes(), true)?;
                        }
                        // Advance position for the original text (to maintain layout) ~keep
                        let w = self.advance_position_for_string(&text, true)?;
                        if let Some(ref mut buffer) = self.tj_span_buffer {
                            buffer.accumulated_width += w;
                        }
                    }
                } else {
                    if self.extract_spans {
                        // NEW: Buffer consecutive Tj operators into single spans
                        // Per PDF Spec ISO 32000-1:2008, Section 9.4.4 NOTE 6:
                        // "text strings are as long as possible" ~keep

                        if self.tj_span_buffer.is_none() {
                            self.tj_span_buffer = Some(TjBuffer::new_with_provenance_cache(
                                self.state_stack.current(),
                                self.current_mcid,
                                self.cached_current_font.clone(),
                                &self.font_provenance_cache,
                            ));
                        }

                        self.append_and_advance(&text)?;
                    } else {
                        self.show_text(&text, true)?;
                    }
                }
            }
            Operator::TJ { array } => {
                // Note: We do NOT skip /Artifact content here.
                // Many PDFs incorrectly mark page content as artifacts.
                // For tagged PDFs, the structure tree already excludes artifacts
                // via MCID mapping, so no filtering is needed at extractor level. ~keep

                // ActualText override
                // Per PDF Spec ISO 32000-1:2008, Section 14.9.4:
                // The MC-scope `/ActualText` replaces the ENTIRE sequence
                // exactly once — see the Tj path above for the per-scope
                // peek/mark protocol that handles both first and
                // subsequent show-text operators inside the same scope. ~keep
                let (current_at, already_emitted) = self.peek_current_actual_text();
                if let Some(actual_text) = current_at {
                    if !already_emitted {
                        tracing::trace!(target: LOG_TARGET,
                            "TJ operator: emitting MC-scope ActualText '{}' (replacing {} elements)",
                            actual_text,
                            array.len()
                        );
                        self.mark_actual_text_emitted();
                        if self.extract_spans {
                            let mut buffer = TjBuffer::new_with_provenance_cache(
                                self.state_stack.current(),
                                self.current_mcid,
                                self.cached_current_font.clone(),
                                &self.font_provenance_cache,
                            );
                            buffer.unicode.push_str(&actual_text);
                            self.flush_tj_buffer(buffer)?;
                        } else {
                            self.show_text(actual_text.as_bytes(), true)?;
                        }
                    }
                    // First or subsequent: advance position for the
                    // entire TJ array so layout stays consistent. ~keep
                    for (index, element) in array.iter().enumerate() {
                        match element {
                            TextElement::String(s) => {
                                let repair_zero_widths = !Self::has_following_tj_displacement(&array, index);
                                let w = self.advance_position_for_string(s, repair_zero_widths)?;
                                if let Some(ref mut buffer) = self.tj_span_buffer {
                                    buffer.accumulated_width += w;
                                }
                            }
                            TextElement::Offset(offset) => {
                                self.advance_position_for_offset(*offset)?;
                            }
                        }
                    }
                } else {
                    if self.extract_spans {
                        // NEW: Use buffered TJ array processing for span extraction
                        // Per PDF Spec ISO 32000-1:2008, Section 9.4.4 NOTE 6:
                        // "text strings are as long as possible"
                        // This creates one span per logical text unit instead of fragmenting ~keep
                        self.process_tj_array(&array)?;
                    } else {
                        for (index, element) in array.iter().enumerate() {
                            match element {
                                TextElement::String(s) => {
                                    let repair_zero_widths = !Self::has_following_tj_displacement(&array, index);
                                    self.show_text(s, repair_zero_widths)?;
                                }
                                TextElement::Offset(offset) => {
                                    // Adjust text position by offset (in thousandths of em) ~keep
                                    let state = self.state_stack.current();
                                    let tx = -offset / 1000.0 * state.font_size * state.horizontal_scaling / 100.0;

                                    // HEURISTIC: Insert space character for significant negative offsets
                                    //
                                    // PDF Spec Reference: ISO 32000-1:2008, Section 9.4.4
                                    // The spec defines text positioning but does NOT specify when a positioning
                                    // offset represents a word boundary vs. tight kerning.
                                    //
                                    // In PDFs, spaces are often represented as negative positioning offsets in TJ arrays,
                                    // not as explicit space characters. For example:
                                    // [(Text1) -200 (Text2)] TJ <- the -200 creates visual spacing
                                    //
                                    // Geometry-based adaptive threshold (based on font metrics)
                                    // Formula: adaptive_threshold = -(average_glyph_width * word_margin_ratio)
                                    // This adapts to different font sizes and families.
                                    // Fallback: static threshold if font unavailable or adaptive disabled.
                                    // ~keep
                                    let threshold = self.calculate_adaptive_tj_threshold();
                                    if *offset < threshold {
                                        let text_matrix = state.text_matrix;
                                        let ctm = state.ctm;
                                        let font_name = state.font_name.clone();
                                        let font_size = state.font_size;
                                        let fill_color_rgb = state.fill_color_rgb;

                                        let combined = ctm.multiply(&text_matrix);
                                        let effective_font_size =
                                            font_size * (combined.d * combined.d + combined.b * combined.b).sqrt();

                                        let font = font_name.as_ref().and_then(|name| self.fonts.get(name));
                                        let font_weight = if let Some(font) = font {
                                            if font.is_bold() {
                                                FontWeight::Bold
                                            } else {
                                                FontWeight::Normal
                                            }
                                        } else {
                                            FontWeight::Normal
                                        };

                                        let text_pos = text_matrix.transform_point(0.0, 0.0);
                                        let pos = ctm.transform_point(text_pos.x, text_pos.y);
                                        let (r, g, b) = fill_color_rgb;
                                        let is_italic_space = font_name
                                            .as_ref()
                                            .and_then(|name| self.fonts.get(name))
                                            .map(|font| font.is_italic())
                                            .unwrap_or(false);
                                        let font_name_str = font_name.unwrap_or_default();
                                        let final_matrix = ctm.multiply(&text_matrix);
                                        let rotation_degrees = final_matrix.b.atan2(final_matrix.a).to_degrees();

                                        let space_char = TextChar {
                                            char: ' ',
                                            bbox: Rect::new(pos.x, pos.y, tx.abs(), effective_font_size),
                                            font_name: font_name_str,
                                            font_size: effective_font_size,
                                            font_weight,
                                            color: Color::new(r, g, b),
                                            mcid: self.current_mcid,
                                            is_italic: is_italic_space,
                                            is_monospace: false,
                                            origin_x: pos.x,
                                            origin_y: pos.y,
                                            rotation_degrees,
                                            advance_width: tx.abs(),
                                            rendered_advance: tx.abs(),
                                            ascent: font.map(|f| f.ascent).unwrap_or(0.95) * effective_font_size,
                                            descent: font.map(|f| f.descent).unwrap_or(-0.35) * effective_font_size,
                                            matrix: Some([
                                                final_matrix.a,
                                                final_matrix.b,
                                                final_matrix.c,
                                                final_matrix.d,
                                                final_matrix.e,
                                                final_matrix.f,
                                            ]),
                                        };
                                        self.push_char(space_char);
                                    }

                                    // Route through advance_text_matrix so the
                                    // axis swap (H vs V) lives in one place.
                                    // Per ISO 32000-1 §9.4.4 a TJ numeric
                                    // offset shifts along the active writing
                                    // axis: x for WMode 0, y for WMode 1. ~keep
                                    self.state_stack.current_mut().advance_text_matrix(tx);
                                }
                            }
                        }
                    }
                }
            }
            Operator::Quote { text } => {
                // ' operator: Move to next line (T*) and show text (Tj) ~keep
                // Flush any pending span buffer before line break
                self.flush_tj_span_buffer()?;

                let leading = self.state_stack.current().leading;
                {
                    let state = self.state_stack.current_mut();
                    let tm = Matrix::translation(0.0, -leading);
                    state.text_line_matrix = tm.multiply(&state.text_line_matrix);
                    state.text_matrix = state.text_line_matrix;
                }

                if self.extract_spans {
                    if self.tj_span_buffer.is_none() {
                        self.tj_span_buffer = Some(TjBuffer::new_with_provenance_cache(
                            self.state_stack.current(),
                            self.current_mcid,
                            self.cached_current_font.clone(),
                            &self.font_provenance_cache,
                        ));
                    }
                    self.append_and_advance(&text)?;
                } else {
                    self.show_text(&text, true)?;
                }
            }
            Operator::DoubleQuote {
                word_space,
                char_space,
                text,
            } => {
                // " operator: Set spacing, move to next line (T*), and show text (Tj) ~keep
                // Flush any pending span buffer before line break
                self.flush_tj_span_buffer()?;

                {
                    let state = self.state_stack.current_mut();
                    state.word_space = word_space;
                    state.char_space = char_space;
                    let leading = state.leading;
                    let tm = Matrix::translation(0.0, -leading);
                    state.text_line_matrix = tm.multiply(&state.text_line_matrix);
                    state.text_matrix = state.text_line_matrix;
                }

                if self.extract_spans {
                    if self.tj_span_buffer.is_none() {
                        self.tj_span_buffer = Some(TjBuffer::new_with_provenance_cache(
                            self.state_stack.current(),
                            self.current_mcid,
                            self.cached_current_font.clone(),
                            &self.font_provenance_cache,
                        ));
                    }
                    self.append_and_advance(&text)?;
                } else {
                    self.show_text(&text, true)?;
                }
            }

            Operator::Tc { char_space } => {
                self.state_stack.current_mut().char_space = char_space;
            }
            Operator::Tw { word_space } => {
                self.state_stack.current_mut().word_space = word_space;
            }
            Operator::Tz { scale } => {
                self.state_stack.current_mut().horizontal_scaling = scale;
            }
            Operator::TL { leading } => {
                self.state_stack.current_mut().leading = leading;
            }
            Operator::Ts { rise } => {
                self.state_stack.current_mut().text_rise = rise;
            }
            Operator::Tr { render } => {
                self.state_stack.current_mut().render_mode = render;
            }

            Operator::SaveState => {
                // Flush the Tj span buffer before pushing graphics state.
                // q/Q wraps a graphics-state block; restoring after Q can
                // re-set the CTM to an earlier value, leaving the
                // captured user_pos inside the buffer out of sync with
                // the active CTM. Flush so each q/Q block emits its
                // own clean cluster. ~keep
                self.flush_tj_span_buffer()?;
                self.state_stack.save();
            }
            Operator::RestoreState => {
                self.flush_tj_span_buffer()?;
                self.state_stack.restore();
                let current_font = self
                    .state_stack
                    .current()
                    .font_name
                    .as_ref()
                    .and_then(|name| self.fonts.get(name))
                    .cloned();
                self.set_cached_current_font(current_font);
                if !self.excluded_inks.is_empty() {
                    let cs = self.state_stack.current().fill_color_space.clone();
                    self.inside_excluded_ink = self.is_excluded_ink_color_space(&cs);
                }
            }
            Operator::Cm { a, b, c, d, e, f } => {
                // Flush the Tj span buffer before changing the CTM.
                // The buffer captured `user_pos_x`/`user_pos_y` and
                // `user_h_scale` from the CTM in effect when it was
                // created (TjBuffer::new at the first Tj after BT).
                // Non-conforming PDFs can issue cm operators inside
                // a text object — typically when figure / chart text
                // runs alternate `cm` for position with text
                // operators in the same BT/ET block. Without a
                // flush, subsequent Tj chars get a position derived
                // from the new CTM while the buffer still reports
                // the stale `user_pos`, dropping the cluster off
                // the page in the worst case. Flushing here emits
                // the current cluster at its captured position and
                // the next Tj creates a fresh buffer under the new
                // CTM. Spec basis: §9.4 lists cm as general
                // graphics state, not formally allowed inside
                // BT/ET, but conforming readers must process it. ~keep
                self.flush_tj_span_buffer()?;
                let state = self.state_stack.current_mut();
                let new_ctm = Matrix { a, b, c, d, e, f };
                // PDF spec ISO 32000-1:2008 §8.3.4: cm concatenates as M_cm × CTM ~keep
                state.ctm = new_ctm.multiply(&state.ctm);
            }

            Operator::SetFillRgb { r, g, b } => {
                // rg operator implicitly sets DeviceRGB — a process color. ~keep
                self.inside_excluded_ink = false;
                self.state_stack.current_mut().fill_color_rgb = (r, g, b);
            }
            Operator::SetStrokeRgb { r, g, b } => {
                self.state_stack.current_mut().stroke_color_rgb = (r, g, b);
            }
            Operator::SetFillGray { gray } => {
                // g operator implicitly sets DeviceGray — a process color,
                // so clear any active ink exclusion. ~keep
                self.inside_excluded_ink = false;
                self.state_stack.current_mut().fill_color_rgb = (gray, gray, gray);
            }
            Operator::SetStrokeGray { gray } => {
                self.state_stack.current_mut().stroke_color_rgb = (gray, gray, gray);
            }
            Operator::SetFillCmyk { c, m, y, k } => {
                // k operator implicitly sets DeviceCMYK — a process color. ~keep
                self.inside_excluded_ink = false;
                let state = self.state_stack.current_mut();
                state.fill_color_cmyk = Some((c, m, y, k));
                state.fill_color_rgb = cmyk_to_rgb(c, m, y, k);
            }
            Operator::SetStrokeCmyk { c, m, y, k } => {
                let state = self.state_stack.current_mut();
                state.stroke_color_cmyk = Some((c, m, y, k));
                state.stroke_color_rgb = cmyk_to_rgb(c, m, y, k);
            }

            Operator::SetFillColorSpace { name } => {
                // Check for excluded ink before mutating state (needs &self) ~keep
                let ink_excluded = self.is_excluded_ink_color_space(&name);
                self.inside_excluded_ink = ink_excluded;
                if ink_excluded {
                    tracing::trace!(target: LOG_TARGET, "Fill color space {:?} matches excluded ink, suppressing text", name);
                }

                let state = self.state_stack.current_mut();
                state.fill_color_space = name.clone();
                state.fill_color_rgb = (0.0, 0.0, 0.0);
                state.fill_color_cmyk = None;
            }
            Operator::SetStrokeColorSpace { name } => {
                let state = self.state_stack.current_mut();
                state.stroke_color_space = name.clone();
                state.stroke_color_rgb = (0.0, 0.0, 0.0);
                state.stroke_color_cmyk = None;
            }
            Operator::SetFillColor { components } => self.apply_set_fill_color(&components),
            Operator::SetStrokeColor { components } => self.apply_set_stroke_color(&components),
            Operator::SetFillColorN { components, name } => {
                self.apply_set_fill_color_n(&components, name.as_deref().map(String::as_str))
            }
            Operator::SetStrokeColorN { components, name } => {
                self.apply_set_stroke_color_n(&components, name.as_deref().map(String::as_str))
            }

            Operator::SetLineCap { cap_style } => {
                self.state_stack.current_mut().line_cap = cap_style;
            }
            Operator::SetLineJoin { join_style } => {
                self.state_stack.current_mut().line_join = join_style;
            }
            Operator::SetMiterLimit { limit } => {
                self.state_stack.current_mut().miter_limit = limit;
            }
            Operator::SetRenderingIntent { intent } => {
                self.state_stack.current_mut().rendering_intent = intent.clone();
            }
            Operator::SetFlatness { tolerance } => {
                self.state_stack.current_mut().flatness = tolerance;
            }
            Operator::SetExtGState { dict_name } => {
                // ExtGState operator - set graphics state from resource dictionary
                // PDF Spec: ISO 32000-1:2008, Section 8.4.5
                //
                // This operator references an ExtGState dictionary in the page resources
                // that contains transparency, blend modes, and other graphics state parameters.
                //
                // For now, we log the usage. Full implementation would require:
                // 1. Access to page resources (/ExtGState dictionary)
                // 2. Loading the named dictionary
                // 3. Extracting /CA (fill alpha), /ca (stroke alpha), /BM (blend mode), etc.
                // 4. Updating graphics state accordingly
                //
                // Future enhancement: Pass resources to text extractor for full support ~keep
                tracing::trace!(target: LOG_TARGET,
                    "ExtGState '{}' referenced (transparency/blend modes not yet fully supported)",
                    dict_name
                );
            }
            Operator::PaintShading { name } => {
                // Shading operator - paint gradient/shading pattern
                // PDF Spec: ISO 32000-1:2008, Section 8.7.4.3
                //
                // Shading patterns define smooth color gradients and can be:
                // Type 1: Function-based shading
                // Type 2: Axial shading (linear gradient)
                // Type 3: Radial shading (circular gradient)
                // Type 4-7: Mesh-based shadings (Gouraud, Coons patch, tensor-product)
                //
                // For text extraction, shading patterns don't affect text content.
                // Full implementation would require rendering the gradient for visual output. ~keep
                tracing::trace!(target: LOG_TARGET,
                    "Shading pattern '{}' referenced (gradients not rendered in text extraction)",
                    name
                );
            }
            Operator::InlineImage { dict, data } => {
                // Inline image operator - embedded image in content stream
                // PDF Spec: ISO 32000-1:2008, Section 8.9.7 - Inline Images
                //
                // Inline images are small images embedded directly in the content stream
                // using the BI...ID...EI sequence, rather than referenced as XObjects.
                //
                // For text extraction, inline images don't contribute to text content.
                // They would be rendered for visual output or extracted separately
                // for image extraction functionality.
                //
                // Common dictionary keys (abbreviated):
                // - W: Width, H: Height
                // - CS: ColorSpace (DeviceRGB, DeviceGray, etc.)
                // - BPC: BitsPerComponent
                // - F: Filter (FlateDecode, DCTDecode, etc.) ~keep
                let width = dict
                    .get("W")
                    .and_then(|obj| match obj {
                        Object::Integer(i) => Some(*i),
                        _ => None,
                    })
                    .unwrap_or(0);
                let height = dict
                    .get("H")
                    .and_then(|obj| match obj {
                        Object::Integer(i) => Some(*i),
                        _ => None,
                    })
                    .unwrap_or(0);
                tracing::trace!(target: LOG_TARGET,
                    "Inline image encountered: {}x{} pixels, {} bytes of data (not rendered in text extraction)",
                    width,
                    height,
                    data.len()
                );
            }

            // Text object operators (BT/ET)
            // PDF Spec ISO 32000-1:2008, Section 9.4.1:
            // "At the beginning of a text object, Tm and Tlm shall be
            // initialized to the identity matrix." ~keep
            Operator::BeginText => {
                let state = self.state_stack.current_mut();
                state.text_matrix = Matrix::identity();
                state.text_line_matrix = Matrix::identity();
            }
            Operator::EndText => {
                self.flush_tj_span_buffer()?;
            }

            // Marked content operators - for tagged PDF structure
            // PDF Spec: ISO 32000-1:2008, Section 14.6 - Marked Content
            // These operators define logical structure and accessibility metadata.
            // Per PDF Spec Section 14.6, we track artifact status to filter out
            // non-text content (headers, footers, watermarks, resource paths). ~keep
            Operator::BeginMarkedContent { tag } => {
                // Flush the Tj span buffer at the marked-content boundary
                // (ISO 32000-1:2008 §14.6). Without this, consecutive Tj
                // operators that straddle a BMC/BDC/EMC boundary get
                // glued into a single span whose `mcid` reflects only
                // the FIRST Tj — fusing two structurally-distinct
                // elements and breaking every downstream consumer that
                // relies on MCID identity (structure-tree reading
                // order, tree-scope ActualText suppression,
                // table-cell membership). ~keep
                self.flush_tj_span_buffer()?;
                if tag == "ReversedChars" {
                    self.saw_reversed_chars = true;
                }
                // BMC doesn't have properties, but the tag can indicate artifacts ~keep
                let is_artifact = tag == "Artifact";
                // InDesign placed-PDF figure region (see MarkedContentContext::is_placed_pdf).
                // ~keep
                let is_placed_pdf = tag == "PlacedPDF";
                self.marked_content_stack.push(MarkedContentContext {
                    tag: tag.clone(),
                    is_artifact,
                    artifact_type: None,
                    actual_text: None,
                    actual_text_emitted: false,
                    expansion: None,
                    is_excluded_layer: false,
                    is_placed_pdf,
                    own_mcid: None,
                });
                self.update_artifact_state();
                self.update_layer_state();

                if is_artifact {
                    tracing::trace!(target: LOG_TARGET, "Entered /Artifact marked content (BMC, no subtype)");
                }
            }

            Operator::BeginMarkedContentDict { tag, properties } => {
                // See `BeginMarkedContent` for the rationale; same
                // reasoning applies to BDC. ~keep
                self.flush_tj_span_buffer()?;
                // BDC can have properties including MCID, artifact indicators, ActualText, and expansion
                // Properties can be an inline dictionary or a name referencing /Properties resource
                // ~keep
                let mut actual_text = None;
                let mut artifact_type = None;
                let mut expansion = None;
                let mut own_mcid: Option<u32> = None;

                let mut is_excluded_layer = false;

                if let Some(props_dict) = self.resolve_bdc_properties(&properties) {
                    if let Some(mcid_obj) = props_dict.get("MCID") {
                        // Same id-space contract as the structure-tree side:
                        // wrapping would alias a malformed id onto a real MCID. ~keep
                        if let Some(mcid) = mcid_obj.as_integer().and_then(crate::structure::checked_mcid) {
                            own_mcid = Some(mcid);
                            self.current_mcid = Some(mcid);
                            tracing::trace!(target: LOG_TARGET, "Entered marked content with MCID: {}", mcid);
                        }
                    }

                    if let Some(actual_text_obj) = props_dict.get("ActualText")
                        && let Some(text_bytes) = actual_text_obj.as_string()
                    {
                        actual_text = Some(Self::decode_pdf_text_string(text_bytes));
                        tracing::trace!(target: LOG_TARGET, "Marked content has ActualText: {:?}", actual_text);
                        // Record that this MCID's in-stream
                        // /ActualText is the authoritative
                        // replacement (MC-scope wins over any
                        // ancestor's struct-tree-scope
                        // /ActualText). ~keep
                        if let Some(mcid) = self.current_mcid {
                            self.mc_actualtext_mcids.insert(mcid);
                        }
                    }

                    if let Some(expansion_obj) = props_dict.get("E")
                        && let Some(text_bytes) = expansion_obj.as_string()
                    {
                        expansion = Some(Self::decode_pdf_text_string(text_bytes));
                        tracing::trace!(target: LOG_TARGET, "Marked content has expansion /E: {:?}", expansion);
                    }

                    if tag == "Artifact" {
                        artifact_type = Self::parse_artifact_type(&props_dict);
                    }

                    // OCG / OCMD (Optional Content) filtering.
                    // Per ISO 32000-1:2008 Section 8.11.2:
                    //  - Direct OCG: << /Type /OCG /Name /LayerName >>
                    //  - OCMD:       << /Type /OCMD /OCGs [refs...] /P /policy >> ~keep
                    if tag == "OC" && !self.excluded_layers.is_empty() {
                        is_excluded_layer = self.check_ocg_excluded(&props_dict);
                    }
                }

                // Check if this is an artifact (per PDF Spec Section 14.6) ~keep
                let is_artifact = tag == "Artifact";
                // InDesign placed-PDF figure region (see MarkedContentContext::is_placed_pdf).
                // ~keep
                let is_placed_pdf = tag == "PlacedPDF";
                self.marked_content_stack.push(MarkedContentContext {
                    tag: tag.clone(),
                    is_artifact,
                    artifact_type: artifact_type.clone(),
                    actual_text,
                    actual_text_emitted: false,
                    expansion,
                    is_excluded_layer,
                    is_placed_pdf,
                    own_mcid,
                });
                self.update_artifact_state();
                self.update_layer_state();

                if is_artifact {
                    if let Some(ref atype) = artifact_type {
                        tracing::trace!(target: LOG_TARGET, "Entered /Artifact marked content: {:?}", atype);
                    } else {
                        tracing::trace!(target: LOG_TARGET, "Entered /Artifact marked content (no type specified)");
                    }
                }
            }

            Operator::EndMarkedContent => {
                // Flush the Tj span buffer at the marked-content
                // boundary; see `BeginMarkedContent` for the
                // rationale. ~keep
                self.flush_tj_span_buffer()?;
                // EMC ends the current marked content sequence.
                // Pop the stack THEN restore `current_mcid` from the
                // nearest enclosing BDC that carried `/MCID` — per
                // ISO 32000-1:2008 §14.6, marked-content sequences
                // nest, and a `Tj` issued after an inner EMC must
                // attribute to its enclosing scope. Blanking
                // `current_mcid` here would orphan that `Tj`'s span
                // (MAJOR-1 regression #...). ~keep
                if !self.marked_content_stack.is_empty() {
                    self.marked_content_stack.pop();
                    self.update_artifact_state();
                    self.update_layer_state();
                }
                let restored = self.marked_content_stack.iter().rev().find_map(|ctx| ctx.own_mcid);
                if let Some(prev) = self.current_mcid {
                    tracing::trace!(target: LOG_TARGET,
                        "Exited marked content with MCID: {} -> restoring to {:?}",
                        prev,
                        restored
                    );
                }
                self.current_mcid = restored;
            }

            Operator::Do { name } => {
                // Flush the Tj span buffer before invoking a Form XObject.
                // `process_xobject` applies the form's /Matrix to the CTM
                // (§8.10.1) and may execute cm/Tm operators inside the
                // form's content stream. The buffer's captured user_pos
                // would no longer correspond to the CTM in effect when
                // the form's text is emitted, so subsequent Tj chars
                // would be stitched into the wrong cluster. ~keep
                self.flush_tj_span_buffer()?;

                // Process Form XObjects to extract text from reusable content.
                // Form XObjects can contain text that is not duplicated in the main stream.
                // We track processed XObjects to avoid infinite loops and duplicates. ~keep
                if let Err(error) = self.process_xobject(&name) {
                    // Log error but continue processing - don't fail the entire extraction ~keep
                    tracing::warn!(target: LOG_TARGET,
                        error_code = error.telemetry_code(),
                        error_offset = ?error.telemetry_offset(),
                        "failed to process XObject"
                    );
                }
            }

            _ => {}
        }

        Ok(())
    }

    /// `sc`/`SC` fill color. Split out of `execute_operator` unchanged, for
    /// file size — see the `Operator::SetFillColor` doc comment for the
    /// component-count-per-space rules. ~keep
    fn apply_set_fill_color(&mut self, components: &[f32]) {
        let state = self.state_stack.current_mut();
        match state.fill_color_space.as_str() {
            "DeviceGray" | "CalGray" if components.len() == 1 => {
                let gray = components[0];
                state.fill_color_rgb = (gray, gray, gray);
            }
            "DeviceRGB" | "CalRGB" if components.len() == 3 => {
                state.fill_color_rgb = (components[0], components[1], components[2]);
            }
            "Lab" if components.len() == 3 => {
                // CIE L*a*b* color space
                // For now, treat as RGB (proper conversion requires whitepoint)
                // L* is lightness (0-100), a* and b* are color opponents
                // Simplified conversion: normalize and treat as RGB ~keep
                let l = components[0] / 100.0;
                state.fill_color_rgb = (l, l, l);
                tracing::trace!(target: LOG_TARGET,
                    "Lab color space simplified to grayscale (full conversion not yet implemented)"
                );
            }
            "DeviceCMYK" if components.len() == 4 => {
                state.fill_color_cmyk = Some((components[0], components[1], components[2], components[3]));
                state.fill_color_rgb = cmyk_to_rgb(components[0], components[1], components[2], components[3]);
            }
            "ICCBased" => {
                // ICC profile-based color space
                // For now, assume RGB and use components directly ~keep
                if components.len() == 3 {
                    state.fill_color_rgb = (components[0], components[1], components[2]);
                } else if components.len() == 1 {
                    let gray = components[0];
                    state.fill_color_rgb = (gray, gray, gray);
                } else if components.len() == 4 {
                    state.fill_color_cmyk = Some((components[0], components[1], components[2], components[3]));
                    state.fill_color_rgb = cmyk_to_rgb(components[0], components[1], components[2], components[3]);
                }
                tracing::trace!(target: LOG_TARGET, "ICCBased color space using simplified conversion (ICC profile not processed)");
            }
            "Separation" if components.len() == 1 => {
                // Separation color space (spot color)
                // Component is tint value (0.0 = no ink, 1.0 = full ink)
                // For now, treat as grayscale ~keep
                let tint = components[0];
                let gray = 1.0 - tint; // Inverted (0 tint = white, 1 tint = black) ~keep
                state.fill_color_rgb = (gray, gray, gray);
                tracing::trace!(target: LOG_TARGET, "Separation color space simplified to grayscale");
            }
            "DeviceN" if !components.is_empty() => {
                // DeviceN color space (multiple colorants)
                // For now, use simplified conversion ~keep
                if components.len() == 4 {
                    state.fill_color_cmyk = Some((components[0], components[1], components[2], components[3]));
                    state.fill_color_rgb = cmyk_to_rgb(components[0], components[1], components[2], components[3]);
                } else {
                    let gray = 1.0 - components[0];
                    state.fill_color_rgb = (gray, gray, gray);
                }
                tracing::trace!(target: LOG_TARGET, "DeviceN color space using simplified conversion");
            }
            _ => {
                // Named color space reference (e.g. "Cs1") or unknown —
                // fall back by component count to avoid warn spam. ~keep
                match components.len() {
                    1 => {
                        let gray = components[0];
                        state.fill_color_rgb = (gray, gray, gray);
                    }
                    3 => {
                        state.fill_color_rgb = (components[0], components[1], components[2]);
                    }
                    4 => {
                        state.fill_color_cmyk = Some((components[0], components[1], components[2], components[3]));
                        state.fill_color_rgb = cmyk_to_rgb(components[0], components[1], components[2], components[3]);
                    }
                    _ => {}
                }
                tracing::trace!(target: LOG_TARGET,
                    "Unknown fill color space {:?} with {} components; \
                     applied component-count fallback",
                    state.fill_color_space,
                    components.len()
                );
            }
        }
    }

    /// `SC` stroke color — mirrors [`Self::apply_set_fill_color`] on the
    /// stroke fields. Split out of `execute_operator` unchanged, for file
    /// size. ~keep
    fn apply_set_stroke_color(&mut self, components: &[f32]) {
        let state = self.state_stack.current_mut();
        match state.stroke_color_space.as_str() {
            "DeviceGray" | "CalGray" if components.len() == 1 => {
                let gray = components[0];
                state.stroke_color_rgb = (gray, gray, gray);
            }
            "DeviceRGB" | "CalRGB" if components.len() == 3 => {
                state.stroke_color_rgb = (components[0], components[1], components[2]);
            }
            "Lab" if components.len() == 3 => {
                let l = components[0] / 100.0;
                state.stroke_color_rgb = (l, l, l);
                tracing::trace!(target: LOG_TARGET, "Lab stroke color space simplified to grayscale");
            }
            "DeviceCMYK" if components.len() == 4 => {
                state.stroke_color_cmyk = Some((components[0], components[1], components[2], components[3]));
                state.stroke_color_rgb = cmyk_to_rgb(components[0], components[1], components[2], components[3]);
            }
            "ICCBased" => {
                if components.len() == 3 {
                    state.stroke_color_rgb = (components[0], components[1], components[2]);
                } else if components.len() == 1 {
                    let gray = components[0];
                    state.stroke_color_rgb = (gray, gray, gray);
                } else if components.len() == 4 {
                    state.stroke_color_cmyk = Some((components[0], components[1], components[2], components[3]));
                    state.stroke_color_rgb = cmyk_to_rgb(components[0], components[1], components[2], components[3]);
                }
                tracing::trace!(target: LOG_TARGET, "ICCBased stroke color using simplified conversion");
            }
            "Separation" if components.len() == 1 => {
                let tint = components[0];
                let gray = 1.0 - tint;
                state.stroke_color_rgb = (gray, gray, gray);
                tracing::trace!(target: LOG_TARGET, "Separation stroke color simplified to grayscale");
            }
            "DeviceN" if !components.is_empty() => {
                if components.len() == 4 {
                    state.stroke_color_cmyk = Some((components[0], components[1], components[2], components[3]));
                    state.stroke_color_rgb = cmyk_to_rgb(components[0], components[1], components[2], components[3]);
                } else {
                    let gray = 1.0 - components[0];
                    state.stroke_color_rgb = (gray, gray, gray);
                }
                tracing::trace!(target: LOG_TARGET, "DeviceN stroke color using simplified conversion");
            }
            _ => {
                match components.len() {
                    1 => {
                        let gray = components[0];
                        state.stroke_color_rgb = (gray, gray, gray);
                    }
                    3 => {
                        state.stroke_color_rgb = (components[0], components[1], components[2]);
                    }
                    4 => {
                        state.stroke_color_cmyk = Some((components[0], components[1], components[2], components[3]));
                        state.stroke_color_rgb =
                            cmyk_to_rgb(components[0], components[1], components[2], components[3]);
                    }
                    _ => {}
                }
                tracing::trace!(target: LOG_TARGET,
                    "Unknown stroke color space {:?} with {} components; \
                     applied component-count fallback",
                    state.stroke_color_space,
                    components.len()
                );
            }
        }
    }

    /// `scn` fill color, with optional pattern name. Split out of
    /// `execute_operator` unchanged, for file size. ~keep
    fn apply_set_fill_color_n(&mut self, components: &[f32], name: Option<&str>) {
        if name.is_some() {
            // Pattern color space - for now, just log and ignore ~keep
            tracing::trace!(target: LOG_TARGET, "Pattern fill color not yet supported: {:?}", name);
            return;
        }
        let state = self.state_stack.current_mut();
        match state.fill_color_space.as_str() {
            "DeviceGray" | "CalGray" if components.len() == 1 => {
                let gray = components[0];
                state.fill_color_rgb = (gray, gray, gray);
            }
            "DeviceRGB" | "CalRGB" if components.len() == 3 => {
                state.fill_color_rgb = (components[0], components[1], components[2]);
            }
            "Lab" if components.len() == 3 => {
                let l = components[0] / 100.0;
                state.fill_color_rgb = (l, l, l);
            }
            "DeviceCMYK" if components.len() == 4 => {
                state.fill_color_cmyk = Some((components[0], components[1], components[2], components[3]));
                state.fill_color_rgb = cmyk_to_rgb(components[0], components[1], components[2], components[3]);
            }
            "ICCBased" => {
                if components.len() == 3 {
                    state.fill_color_rgb = (components[0], components[1], components[2]);
                } else if components.len() == 1 {
                    let gray = components[0];
                    state.fill_color_rgb = (gray, gray, gray);
                } else if components.len() == 4 {
                    state.fill_color_cmyk = Some((components[0], components[1], components[2], components[3]));
                    state.fill_color_rgb = cmyk_to_rgb(components[0], components[1], components[2], components[3]);
                }
            }
            "Separation" if components.len() == 1 => {
                let tint = components[0];
                let gray = 1.0 - tint;
                state.fill_color_rgb = (gray, gray, gray);
            }
            "DeviceN" if !components.is_empty() => {
                if components.len() == 4 {
                    state.fill_color_cmyk = Some((components[0], components[1], components[2], components[3]));
                    state.fill_color_rgb = cmyk_to_rgb(components[0], components[1], components[2], components[3]);
                } else {
                    let gray = 1.0 - components[0];
                    state.fill_color_rgb = (gray, gray, gray);
                }
            }
            _ => {
                match components.len() {
                    1 => {
                        let gray = components[0];
                        state.fill_color_rgb = (gray, gray, gray);
                    }
                    3 => {
                        state.fill_color_rgb = (components[0], components[1], components[2]);
                    }
                    4 => {
                        state.fill_color_cmyk = Some((components[0], components[1], components[2], components[3]));
                        state.fill_color_rgb = cmyk_to_rgb(components[0], components[1], components[2], components[3]);
                    }
                    _ => {}
                }
                tracing::trace!(target: LOG_TARGET,
                    "Unknown fill color space {:?} with {} components; \
                     applied component-count fallback",
                    state.fill_color_space,
                    components.len()
                );
            }
        }
    }

    /// `SCN` stroke color, with optional pattern name — mirrors
    /// [`Self::apply_set_fill_color_n`] on the stroke fields. Split out of
    /// `execute_operator` unchanged, for file size. ~keep
    fn apply_set_stroke_color_n(&mut self, components: &[f32], name: Option<&str>) {
        if name.is_some() {
            // Pattern color space - for now, just log and ignore ~keep
            tracing::trace!(target: LOG_TARGET, "Pattern stroke color not yet supported: {:?}", name);
            return;
        }
        let state = self.state_stack.current_mut();
        match state.stroke_color_space.as_str() {
            "DeviceGray" | "CalGray" if components.len() == 1 => {
                let gray = components[0];
                state.stroke_color_rgb = (gray, gray, gray);
            }
            "DeviceRGB" | "CalRGB" if components.len() == 3 => {
                state.stroke_color_rgb = (components[0], components[1], components[2]);
            }
            "Lab" if components.len() == 3 => {
                let l = components[0] / 100.0;
                state.stroke_color_rgb = (l, l, l);
            }
            "DeviceCMYK" if components.len() == 4 => {
                state.stroke_color_cmyk = Some((components[0], components[1], components[2], components[3]));
                state.stroke_color_rgb = cmyk_to_rgb(components[0], components[1], components[2], components[3]);
            }
            "ICCBased" => {
                if components.len() == 3 {
                    state.stroke_color_rgb = (components[0], components[1], components[2]);
                } else if components.len() == 1 {
                    let gray = components[0];
                    state.stroke_color_rgb = (gray, gray, gray);
                } else if components.len() == 4 {
                    state.stroke_color_cmyk = Some((components[0], components[1], components[2], components[3]));
                    state.stroke_color_rgb = cmyk_to_rgb(components[0], components[1], components[2], components[3]);
                }
            }
            "Separation" if components.len() == 1 => {
                let tint = components[0];
                let gray = 1.0 - tint;
                state.stroke_color_rgb = (gray, gray, gray);
            }
            "DeviceN" if !components.is_empty() => {
                if components.len() == 4 {
                    state.stroke_color_cmyk = Some((components[0], components[1], components[2], components[3]));
                    state.stroke_color_rgb = cmyk_to_rgb(components[0], components[1], components[2], components[3]);
                } else {
                    let gray = 1.0 - components[0];
                    state.stroke_color_rgb = (gray, gray, gray);
                }
            }
            _ => {
                match components.len() {
                    1 => {
                        let gray = components[0];
                        state.stroke_color_rgb = (gray, gray, gray);
                    }
                    3 => {
                        state.stroke_color_rgb = (components[0], components[1], components[2]);
                    }
                    4 => {
                        state.stroke_color_cmyk = Some((components[0], components[1], components[2], components[3]));
                        state.stroke_color_rgb =
                            cmyk_to_rgb(components[0], components[1], components[2], components[3]);
                    }
                    _ => {}
                }
                tracing::trace!(target: LOG_TARGET,
                    "Unknown stroke color space {:?} with {} components; \
                     applied component-count fallback",
                    state.stroke_color_space,
                    components.len()
                );
            }
        }
    }
}
