//! Text-position advance, space insertion, and glyph emission.
//!
//! Split out of the parent's single 5,806-line `impl TextExtractor`, which made
//! `extractors/text.rs` 673 KiB — over the repository's 500 KiB file-safety limit.
//! A child module's `impl` is the same inherent impl and sees the parent's private
//! items unchanged. ~keep

use super::*;

impl<'doc> TextExtractor<'doc> {
    /// Advance text position for a string (used in TJ array processing).
    /// Advance the text matrix position by the width of a text string.
    /// Returns the computed width so callers can accumulate it.
    pub(super) fn advance_position_for_string(&mut self, text: &[u8], repair_zero_widths: bool) -> Result<f32> {
        let state = self.state_stack.current();
        let font_size = state.font_size;
        let horizontal_scaling = state.horizontal_scaling;
        let char_space = state.char_space;
        let word_space = state.word_space;
        let wmode = state.text_wmode;

        let font = self.cached_current_font.as_deref();

        // Hoist loop-invariant computations (font cannot change mid-operator).
        // font_matrix_a converts glyph-space widths to text-space units.
        // Standard fonts (Type1/TrueType): font_matrix_a = 0.001.
        // Type3 with identity FontMatrix: font_matrix_a = 1.0 (no /1000 division).
        // Assumes FontMatrix[1] = 0 (no glyph-axis rotation), which holds for all
        // standard fonts and virtually all Type3 fonts encountered in practice. ~keep
        let font_matrix_a = font.map(|f| f.font_matrix_a).unwrap_or(0.001);
        let fs_factor = font_size * font_matrix_a;
        let hs_factor = horizontal_scaling / 100.0;
        let cs_hs = char_space * hs_factor;
        let ws_hs = word_space * hs_factor;

        let total_width = if let Some(font) = font {
            if font.subtype != "Type0" {
                // Fast path: use precomputed 256-entry width table (simple fonts) ~keep
                let width_table =
                    Self::simple_widths(self.cached_extraction_widths.as_deref(), font, repair_zero_widths);
                let mut w_sum = 0.0f32;
                for &byte in text {
                    let mut w = width_table[byte as usize] * fs_factor * hs_factor;
                    w += cs_hs;
                    if byte == 0x20 {
                        w += ws_hs;
                    }
                    w_sum += w;
                }
                w_sum
            } else if wmode == 0 {
                // Type0/CID font, horizontal: use TextCharIter so that the byte-width
                // (1 or 2) is determined by the font's encoding / ToUnicode CMap
                // codespace, not hardcoded to 2. Per ISO 32000-1:2008 §9.7.6.2. ~keep
                let mut w_sum = 0.0f32;
                for (code, nbytes) in TextCharIter::new(text, Some(font)) {
                    // GH #1631: `code` is a raw content-stream character
                    // code, not a CID — it must go through `code_to_cid`
                    // before it keys /W/DW/W2, or a non-Identity
                    // predefined/embedded CMap gets the wrong width. The
                    // Tw-eligibility check below stays on the untranslated
                    // `code`: ISO 32000-1 §9.3.3 gates on the single-byte
                    // *character code* 32, not on whatever CID it resolves
                    // to. ~keep
                    let cid = font.code_to_cid(code as u32);
                    let mut w = font.get_glyph_width(cid) * fs_factor * hs_factor;
                    w += cs_hs;
                    // Per ISO 32000-1:2008 §9.3.3: Tw applies ONLY to the
                    // single-byte character code 32, never to the byte value 32
                    // inside a multi-byte code. `TextCharIter` yields the raw
                    // code plus its byte width, so gate on a single-byte 32 — a
                    // 2-byte CID #32 (0x0020) in an Identity-H/CJK font must not
                    // take Tw (it would over-advance and mis-position the run). ~keep
                    if nbytes == 1 && code == 32 {
                        w += ws_hs;
                    }
                    w_sum += w;
                }
                w_sum
            } else {
                // Type0/CID font, vertical (WMode 1): per-glyph displacement
                // is `w1y` (from /W2 or /DW2 default), in 1000ths-of-em. ~keep
                //
                // Per ISO 32000-1:2008 §9.4.4 the vertical formula is
                //     ty = (w1y * Tfs) + Tc + Tw
                // with NO Th factor. §9.3.4 defines Tz as the horizontal
                // glyph-stretching axis — it does not scale w1y, Tc, or
                // Tw in vertical mode. ~keep
                let mut w_sum = 0.0f32;
                for (code, nbytes) in TextCharIter::new(text, Some(font)) {
                    // GH #1631: translate the raw code to a CID before the
                    // /W2 lookup; the Tw check stays on the raw code. ~keep
                    let cid = font.code_to_cid(code as u32);
                    let w1y = font.get_vertical_metrics(cid).w1y;
                    let mut w = w1y * fs_factor;
                    w += char_space;
                    if nbytes == 1 && code == 32 {
                        w += word_space;
                    }
                    w_sum += w;
                }
                w_sum
            }
        } else {
            let default_w = 500.0 * fs_factor * hs_factor + cs_hs;
            let space_w = default_w + ws_hs;
            let mut w_sum = 0.0f32;
            for &byte in text {
                w_sum += if byte == 0x20 { space_w } else { default_w };
            }
            w_sum
        };

        // Update text matrix position per ISO 32000-1:2008 §9.4.4. The
        // axis-swap (horizontal vs vertical) is encapsulated in
        // GraphicsState::advance_text_matrix so this site does not branch. ~keep
        self.state_stack.current_mut().advance_text_matrix(total_width);

        Ok(total_width)
    }

    /// Combined Unicode decode + width calculation in a single pass.
    /// Merges TjBuffer::append and advance_position_for_string for simple fonts,
    /// eliminating one full per-byte iteration per Tj operator.
    pub(super) fn append_and_advance(&mut self, text: &[u8]) -> Result<()> {
        let text = if text.len() > 32_767 { &text[..32_767] } else { text };

        let state = self.state_stack.current();
        let font_size = state.font_size;
        let horizontal_scaling = state.horizontal_scaling;
        let char_space = state.char_space;
        let word_space = state.word_space;
        let wmode = state.text_wmode;

        // Disjoint field borrows: cached_current_font (immutable) + tj_span_buffer (mutable) ~keep
        let font = self.cached_current_font.as_deref();
        // font_matrix_a converts glyph-space widths to text-space units.
        // Standard fonts (Type1/TrueType): font_matrix_a = 0.001.
        // Type3 with identity FontMatrix: font_matrix_a = 1.0 (no /1000 division).
        // Assumes FontMatrix[1] = 0 (no glyph-axis rotation), which holds for all
        // standard fonts and virtually all Type3 fonts encountered in practice. ~keep
        let font_matrix_a = font.map(|f| f.font_matrix_a).unwrap_or(0.001);
        let fs_factor = font_size * font_matrix_a;
        let hs_factor = horizontal_scaling / 100.0;
        let cs_hs = char_space * hs_factor;
        let ws_hs = word_space * hs_factor;
        // Safety: tj_span_buffer is always initialized via begin_text_object() ~keep
        let buffer = self
            .tj_span_buffer
            .as_mut()
            .expect("tj_span_buffer initialized in begin_text_object");

        let total_width = if let Some(font) = font {
            if font.subtype != "Type0" {
                // UTF-8-in-simple-font detection (same heuristic as
                // `append_advance_buffer`). Some producers emit raw UTF-8
                // bytes inside PDF string literals when the font declares
                // only a Latin encoding and no ToUnicode CMap. Byte-by-byte
                // Latin decoding produces mojibake. When the slice is valid
                // UTF-8 with at least one non-Latin-1 codepoint, decode as
                // UTF-8 so non-Latin scripts (Cyrillic, Greek, CJK, …) come
                // through as their intended codepoints. ~keep
                if font.to_unicode.is_none() && text.len() >= 2 {
                    let has_high = text.iter().any(|&b| b >= 0x80);
                    if has_high
                        && let Ok(decoded) = std::str::from_utf8(text)
                        && decoded.chars().any(|c| c as u32 > 0xFF)
                    {
                        let width_table = Self::simple_widths(self.cached_extraction_widths.as_deref(), font, true);
                        let mut w_sum = 0.0f32;
                        for &byte in text {
                            let mut w = width_table[byte as usize] * fs_factor * hs_factor;
                            w += cs_hs;
                            if byte == 0x20 {
                                w += ws_hs;
                            }
                            w_sum += w;
                        }
                        let char_count = decoded.chars().count();
                        if char_count > 0 {
                            let per_char = w_sum / char_count as f32;
                            for ch in decoded.chars() {
                                buffer.unicode.push(ch);
                                buffer.char_widths.push(per_char);
                            }
                        }
                        // Fall through to the matrix update at the
                        // bottom of the function via `w_sum`. Vertical
                        // mode flips the axis inside the helper. ~keep
                        self.state_stack.current_mut().advance_text_matrix(w_sum);
                        return Ok(());
                    }
                }

                let char_table = font.get_byte_to_char_table();
                let width_table = Self::simple_widths(self.cached_extraction_widths.as_deref(), font, true);
                let mut w_sum = 0.0f32;
                for &byte in text {
                    let len_before = buffer.unicode.len();
                    let c = char_table[byte as usize];
                    if c != '\0' {
                        buffer.unicode.push(c);
                    } else {
                        // Rare: multi-char mapping or unmapped byte ~keep
                        if let Some(s) = font.char_to_unicode(byte as u32) {
                            if s != "\u{FFFD}" || preserve_unmapped_glyphs() {
                                for ch in s.chars() {
                                    if ch >= '\x20' || ch == '\t' || ch == '\n' || ch == '\r' {
                                        buffer.unicode.push(ch);
                                    }
                                }
                            }
                        } else {
                            let fb = fallback_char_to_unicode(byte as u32);
                            if fb != "\u{FFFD}" || preserve_unmapped_glyphs() {
                                for ch in fb.chars() {
                                    if ch >= '\x20' || ch == '\t' || ch == '\n' || ch == '\r' {
                                        buffer.unicode.push(ch);
                                    }
                                }
                            }
                        }
                    }
                    let mut w = width_table[byte as usize] * fs_factor * hs_factor;
                    w += cs_hs;
                    if byte == 0x20 {
                        w += ws_hs;
                    }
                    w_sum += w;
                    let chars_added = buffer.unicode.len() - len_before;
                    if chars_added == 1 {
                        buffer.char_widths.push(w);
                    } else if chars_added > 1 {
                        let per_char = w / chars_added as f32;
                        for _ in 0..chars_added {
                            buffer.char_widths.push(per_char);
                        }
                    }
                }
                w_sum
            } else if wmode == 0 {
                // Type0/CID font, horizontal: unified iterator handles 1- or
                // 2-byte codes per ToUnicode codespace. ~keep
                buffer.append(text)?;
                let mut w_sum = 0.0f32;
                for (code, nbytes) in TextCharIter::new(text, Some(font)) {
                    // GH #1631: translate to a CID before the width lookup;
                    // Tw stays keyed on the raw code (§9.3.3). ~keep
                    let cid = font.code_to_cid(code as u32);
                    let mut w = font.get_glyph_width(cid) * fs_factor * hs_factor;
                    w += cs_hs;
                    // Per ISO 32000-1:2008 §9.3.3: Tw applies only to the
                    // single-byte character code 32 — a 2-byte CID 32 inside
                    // an Identity-H/CJK font must not take Tw. ~keep
                    if nbytes == 1 && code == 32 {
                        w += ws_hs;
                    }
                    w_sum += w;
                    buffer.char_widths.push(w);
                }
                w_sum
            } else {
                // Type0/CID font, vertical (WMode 1): per-glyph displacement
                // is `w1y` (from /W2 or /DW2), in 1000ths-of-em.
                //
                // Per ISO 32000-1:2008 §9.4.4: `ty = (w1y * Tfs) + Tc + Tw`,
                // with no Th (Tz only stretches glyphs along the horizontal
                // axis per §9.3.4). ~keep
                buffer.append(text)?;
                let mut w_sum = 0.0f32;
                for (code, nbytes) in TextCharIter::new(text, Some(font)) {
                    let cid = font.code_to_cid(code as u32);
                    let w1y = font.get_vertical_metrics(cid).w1y;
                    let mut w = w1y * fs_factor;
                    w += char_space;
                    // Per ISO 32000-1:2008 §9.3.3: Tw applies only to the
                    // single-byte character code 32. ~keep
                    if nbytes == 1 && code == 32 {
                        w += word_space;
                    }
                    w_sum += w;
                    buffer.char_widths.push(w);
                }
                w_sum
            }
        } else {
            buffer.append(text)?;
            let default_w = 500.0 * fs_factor * hs_factor + cs_hs;
            let space_w = default_w + ws_hs;
            let mut w_sum = 0.0f32;
            for &byte in text {
                let w = if byte == 0x20 { space_w } else { default_w };
                w_sum += w;
                buffer.char_widths.push(w);
            }
            w_sum
        };

        buffer.accumulated_width += total_width;

        // Update text matrix position per ISO 32000-1:2008 §9.4.4. The
        // axis-swap (H vs V) is encapsulated in advance_text_matrix. ~keep
        self.state_stack.current_mut().advance_text_matrix(total_width);

        Ok(())
    }

    /// Combined Unicode decode + width + position advance for a local buffer.
    /// Same as append_and_advance but works on an explicit buffer parameter
    /// instead of self.tj_span_buffer. Used by TJ array processing.
    pub(super) fn append_advance_buffer(
        &mut self,
        buffer: &mut TjBuffer,
        text: &[u8],
        repair_zero_widths: bool,
    ) -> Result<()> {
        let text = if text.len() > 32_767 { &text[..32_767] } else { text };

        let state = self.state_stack.current();
        let font_size = state.font_size;
        let horizontal_scaling = state.horizontal_scaling;
        let char_space = state.char_space;
        let word_space = state.word_space;
        let wmode = state.text_wmode;

        let font = self.cached_current_font.as_deref();
        // font_matrix_a converts glyph-space widths to text-space units.
        // Standard fonts (Type1/TrueType): font_matrix_a = 0.001.
        // Type3 with identity FontMatrix: font_matrix_a = 1.0 (no /1000 division).
        // Assumes FontMatrix[1] = 0 (no glyph-axis rotation), which holds for all
        // standard fonts and virtually all Type3 fonts encountered in practice. ~keep
        let font_matrix_a = font.map(|f| f.font_matrix_a).unwrap_or(0.001);
        let fs_factor = font_size * font_matrix_a;
        let hs_factor = horizontal_scaling / 100.0;
        let cs_hs = char_space * hs_factor;
        let ws_hs = word_space * hs_factor;

        let total_width = if let Some(font) = font {
            if font.subtype != "Type0" {
                // UTF-8-in-simple-font detection.
                //
                // Some producers (Russian CAD exporters, MS Office via
                // non-English locales) emit UTF-8 byte sequences inside PDF
                // string literals for a font that only declares a Latin
                // encoding (WinAnsi, StandardEncoding, MacRoman) and no
                // ToUnicode CMap. Byte-by-byte decoding through the Latin
                // encoding produces mojibake like `ÐÐ¸ÑÑ` for "Лист".
                //
                // Heuristic: when the font has no ToUnicode and the entire
                // text slice is a valid UTF-8 sequence whose decoded
                // codepoints contain at least one non-Latin-1 character
                // (U+0100 and above), treat the slice as UTF-8 directly.
                // The non-Latin-1 gate prevents mis-interpreting genuine
                // Latin-1 Supplement content (`Résumé`, etc.) — those
                // decode entirely into U+0000..U+00FF and are left alone. ~keep
                let utf8_width: Option<f32> = if font.to_unicode.is_none() && text.len() >= 2 {
                    let has_high = text.iter().any(|&b| b >= 0x80);
                    if has_high {
                        if let Ok(decoded) = std::str::from_utf8(text) {
                            let has_non_latin1 = decoded.chars().any(|c| c as u32 > 0xFF);
                            if has_non_latin1 {
                                let width_table = Self::simple_widths(
                                    self.cached_extraction_widths.as_deref(),
                                    font,
                                    repair_zero_widths,
                                );
                                let mut w_sum = 0.0f32;
                                for &byte in text {
                                    let mut w = width_table[byte as usize] * fs_factor * hs_factor;
                                    w += cs_hs;
                                    if byte == 0x20 {
                                        w += ws_hs;
                                    }
                                    w_sum += w;
                                }
                                let char_count = decoded.chars().count();
                                if char_count > 0 {
                                    let per_char = w_sum / char_count as f32;
                                    for ch in decoded.chars() {
                                        buffer.unicode.push(ch);
                                        buffer.char_widths.push(per_char);
                                    }
                                }
                                tracing::trace!(target: LOG_TARGET,
                                    "UTF-8 mojibake repair: decoded {} Latin-1 bytes as {} chars via UTF-8 in font '{}'",
                                    text.len(),
                                    char_count,
                                    font.base_font
                                );
                                Some(w_sum)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };
                if let Some(w) = utf8_width {
                    buffer.accumulated_width += w;
                    self.state_stack.current_mut().advance_text_matrix(w);
                    return Ok(());
                }

                let char_table = font.get_byte_to_char_table();
                let width_table =
                    Self::simple_widths(self.cached_extraction_widths.as_deref(), font, repair_zero_widths);
                let mut w_sum = 0.0f32;
                for &byte in text {
                    let len_before = buffer.unicode.len();
                    let c = char_table[byte as usize];
                    if c != '\0' {
                        buffer.unicode.push(c);
                    } else if let Some(s) = font.char_to_unicode(byte as u32) {
                        if s != "\u{FFFD}" || preserve_unmapped_glyphs() {
                            for ch in s.chars() {
                                if ch >= '\x20' || ch == '\t' || ch == '\n' || ch == '\r' {
                                    buffer.unicode.push(ch);
                                }
                            }
                        }
                    } else {
                        let fb = fallback_char_to_unicode(byte as u32);
                        if fb != "\u{FFFD}" || preserve_unmapped_glyphs() {
                            for ch in fb.chars() {
                                if ch >= '\x20' || ch == '\t' || ch == '\n' || ch == '\r' {
                                    buffer.unicode.push(ch);
                                }
                            }
                        }
                    }
                    let mut w = width_table[byte as usize] * fs_factor * hs_factor;
                    w += cs_hs;
                    if byte == 0x20 {
                        w += ws_hs;
                    }
                    w_sum += w;
                    let chars_added = buffer.unicode.len() - len_before;
                    if chars_added == 1 {
                        buffer.char_widths.push(w);
                    } else if chars_added > 1 {
                        let per_char = w / chars_added as f32;
                        for _ in 0..chars_added {
                            buffer.char_widths.push(per_char);
                        }
                    }
                }
                w_sum
            } else if wmode == 0 {
                buffer.append(text)?;
                // Width calculation: use TextCharIter so byte-width respects the
                // CMap codespace (1 or 2 bytes per character). Fixes CJK fonts
                // whose encoding name doesn't match the well-known Identity-H/EUC/…
                // keyword patterns but whose ToUnicode CMap declares a 2-byte
                // codespace range (§9.7.5). ~keep
                let mut w_sum = 0.0f32;
                for (code, nbytes) in TextCharIter::new(text, Some(font)) {
                    // GH #1631: translate to a CID before the width lookup;
                    // Tw stays keyed on the raw code (§9.3.3). ~keep
                    let cid = font.code_to_cid(code as u32);
                    let mut w = font.get_glyph_width(cid) * fs_factor * hs_factor;
                    w += cs_hs;
                    if nbytes == 1 && code == 32 {
                        w += ws_hs;
                    }
                    w_sum += w;
                    buffer.char_widths.push(w);
                }
                w_sum
            } else {
                // Type0/CID font, vertical mode: per-glyph displacement is
                // /W2 `w1y` (or /DW2 default), in 1000ths-of-em. The
                // vertical formula `ty = (w1y * Tfs) + Tc + Tw` (§9.4.4)
                // does NOT apply Th — Tz only scales glyphs horizontally
                // (§9.3.4). ~keep
                buffer.append(text)?;
                let mut w_sum = 0.0f32;
                for (code, nbytes) in TextCharIter::new(text, Some(font)) {
                    let cid = font.code_to_cid(code as u32);
                    let w1y = font.get_vertical_metrics(cid).w1y;
                    let mut w = w1y * fs_factor;
                    w += char_space;
                    if nbytes == 1 && code == 32 {
                        w += word_space;
                    }
                    w_sum += w;
                    buffer.char_widths.push(w);
                }
                w_sum
            }
        } else {
            buffer.append(text)?;
            let default_w = 500.0 * fs_factor * hs_factor + cs_hs;
            let space_w = default_w + ws_hs;
            let mut w_sum = 0.0f32;
            for &byte in text {
                let w = if byte == 0x20 { space_w } else { default_w };
                w_sum += w;
                buffer.char_widths.push(w);
            }
            w_sum
        };

        buffer.accumulated_width += total_width;

        self.state_stack.current_mut().advance_text_matrix(total_width);

        Ok(())
    }

    /// Insert a space character as a separate span.
    pub(super) fn insert_space_as_span(&mut self) -> Result<()> {
        let mcid_scope = self.current_mcid_scope();
        let state = self.state_stack.current();
        let font_size = state.font_size;
        let text_matrix = state.text_matrix;
        let ctm = state.ctm;
        let combined = ctm.multiply(&text_matrix);
        let effective_font_size = font_size * (combined.d * combined.d + combined.b * combined.b).sqrt();
        let word_space = state.word_space;
        let horizontal_scaling = state.horizontal_scaling;
        let wmode = state.text_wmode;

        // Calculate space displacement along the active writing axis. In
        // horizontal mode this is the glyph width (250/1000 em ≈ quarter
        // em) plus Tw, scaled by Th. In vertical mode Tz does not apply
        // (§9.3.4) and we use the same magnitude as a writing-axis step
        // — the synthetic gap a TJ offset stands in for.
        //
        // NOTE: the displacement is expressed against the raw `Tf` size,
        // not the `Tm`-scaled effective size, so for print-era producers
        // that set `/F 1 Tf` with the size in `Tm` this span is narrower
        // in device space than a quarter em. That geometry is load-bearing
        // for the downstream column/line heuristics, which were tuned
        // against it — widening it reorders text on real documents — so
        // the lockstep fix below keeps a `char_widths` entry
        // consistent with this bbox rather than rescaling both. ~keep
        let space_advance = if wmode == 0 {
            (250.0 * font_size / 1000.0 + word_space) * horizontal_scaling / 100.0
        } else {
            250.0 * font_size / 1000.0 + word_space
        };

        // Apply CTM to get position in user space
        // Per PDF Spec ISO 32000-1:2008 Section 9.4.4 ~keep
        let text_pos = text_matrix.transform_point(0.0, 0.0);
        let user_pos = ctm.transform_point(text_pos.x, text_pos.y);

        tracing::trace!(target: LOG_TARGET,
            "Inserting space span from TJ offset (offset_semantic=true) at position ({:.2}, {:.2})",
            user_pos.x,
            user_pos.y
        );

        let font_name_space = state.font_name.clone().unwrap_or_else(|| "Unknown".to_string());
        let is_italic_space = state
            .font_name
            .as_ref()
            .and_then(|name| self.fonts.get(name))
            .map(|font| font.is_italic())
            .unwrap_or(false);
        // Bbox geometry follows the writing axis: a horizontal gap is
        // wide and font-tall; a vertical gap is glyph-em-wide and tall
        // along the writing direction. Downstream layout heuristics
        // (column detection, line breaking) read width vs height to
        // decide orientation, so labeling the synthetic-space geometry
        // correctly keeps them honest. ~keep
        let (space_width, space_height) = if wmode == 0 {
            (space_advance, effective_font_size)
        } else {
            (effective_font_size, space_advance.abs())
        };
        let span = TextSpan {
            provenance: None,
            text: " ".to_string(),
            bbox: Rect {
                x: user_pos.x,
                y: user_pos.y,
                width: space_width,
                height: space_height,
            },
            font_name: font_name_space,
            font_size: effective_font_size,
            font_weight: FontWeight::Normal,
            color: Color::new(state.fill_color_rgb.0, state.fill_color_rgb.1, state.fill_color_rgb.2),
            mcid: self.current_mcid,
            mcid_scope: Some(mcid_scope),
            sequence: self.span_sequence_counter,
            split_boundary_before: false,
            offset_semantic: true,
            char_spacing: state.char_space, // Tc - captured from PDF content stream ~keep
            word_spacing: state.word_space, // Tw - captured from PDF content stream ~keep
            horizontal_scaling: state.horizontal_scaling,
            // ~keep
            is_italic: is_italic_space,
            is_monospace: false,
            primary_detected: false,
            artifact_type: self.current_artifact_type(),
            // One synthetic space char ⇒ one width entry, so the span-merge
            // lockstep (`char_widths.len() == text.chars().count()`) holds
            // from birth regardless of merge order. The width is
            // the bbox extent along x, consistent with `to_chars` geometry. ~keep
            char_widths: vec![space_width],
            char_x_offsets: Vec::new(),
            heading_level: None,
            rotation_degrees: snap_run_rotation(&state.ctm.multiply(&state.text_matrix)),
            wmode: state.text_wmode,
            text_rise: if state.font_size > 0.0 {
                state.text_rise / state.font_size
            } else {
                0.0
            },
            rtl_draw_logical: false,
            mirrored: run_is_mirrored(&state.ctm.multiply(&state.text_matrix)),
            page_rotation_applied: 0,
        };
        self.span_sequence_counter += 1;

        tracing::trace!(target: LOG_TARGET, "PUSH space span with offset_semantic={}", span.offset_semantic);

        if !self.is_content_suppressed() {
            self.spans.push(span);
        }

        // Do NOT advance the text matrix here. The caller drives the
        // matrix forward by the *actual* TJ offset via
        // `advance_position_for_offset` immediately after; advancing
        // by `space_width` on top of that would double-count the gap
        // and capture the wrong `user_pos_x` when the next buffer is
        // created, producing spans whose bbox.x sits ~one synthetic
        // space-width to the right of the character actually drawn. ~keep

        Ok(())
    }

    /// Advance text position for a TJ offset value.
    ///
    /// Per ISO 32000-1:2008 §9.4.4 a number element in a TJ array shifts
    /// the position along the **active** writing axis:
    ///   horizontal: tx = -offset / 1000 * font_size * Th
    ///   vertical:   ty = -offset / 1000 * font_size     (NO Th)
    /// Th (Tz) is the horizontal glyph-stretching axis (§9.3.4) and does
    /// not apply in vertical mode. The matrix-side axis-swap lives in
    /// `advance_text_matrix`.
    pub(super) fn advance_position_for_offset(&mut self, offset: f32) -> Result<()> {
        let state = self.state_stack.current();
        let font_size = state.font_size;
        let horizontal_scaling = state.horizontal_scaling;
        let wmode = state.text_wmode;

        let tx = if wmode == 0 {
            -offset / 1000.0 * font_size * horizontal_scaling / 100.0
        } else {
            -offset / 1000.0 * font_size
        };

        self.state_stack.current_mut().advance_text_matrix(tx);

        Ok(())
    }

    /// Fold a sub-threshold TJ offset into the active buffer's advance record
    /// so its `char_widths`/`accumulated_width` track the text-matrix position.
    ///
    /// The displacement is computed identically to `advance_position_for_offset`
    /// (text space, before the `user_h_scale` applied at flush) so it lands in
    /// the same units as the per-glyph advances pushed during string append.
    /// The offset conventionally belongs to the *preceding* glyph (it adjusts
    /// spacing after it), so it is added to the last recorded advance; if no
    /// glyph has been recorded yet the matrix move alone already positions the
    /// next buffer, so there is nothing to fold.
    pub(super) fn fold_offset_into_buffer(&self, buffer: &mut TjBuffer, offset: f32) {
        let Some(last) = buffer.char_widths.last_mut() else {
            return;
        };
        let state = self.state_stack.current();
        let adv = if state.text_wmode == 0 {
            -offset / 1000.0 * state.font_size * state.horizontal_scaling / 100.0
        } else {
            -offset / 1000.0 * state.font_size
        };
        *last += adv;
        buffer.accumulated_width += adv;
    }

    /// Whether `(e, f)` continues `start`'s run along that run's writing axis.
    ///
    /// ISO 32000-1:2008 §9.4.4 places the writing direction along the matrix's
    /// `(a, b)` row, so the displacement resolves into a component along it
    /// (the advance) and one perpendicular (the line offset). For any `b = 0,
    /// a > 0` matrix the along test equals the caller's raw `e` test and the
    /// perpendicular test is implied by the raw `f` band (`hypot(c, d) >=
    /// |d|`, equal only when unskewed), so ANDing it in cannot change upright
    /// output.
    ///
    /// WMode 1 is exempt: vertical text advances along `(c, d)` instead, the
    /// branch [`GraphicsState::advance_text_matrix`] already makes, and reading
    /// its advance as a perpendicular offset splits a column glyph by glyph.
    pub(super) fn advances_along_writing_axis(start: Matrix, wmode: u8, e: f32, f: f32, font_size: f32) -> bool {
        if wmode != 0 {
            return true;
        }
        // Unit vector along the writing direction. A degenerate (zero-scale)
        // matrix has no direction to speak of; fall back to +x so such runs
        // behave exactly as they did before this test existed. ~keep
        let axis = (start.a * start.a + start.b * start.b).sqrt();
        let (ux, uy) = if axis > 0.0 {
            (start.a / axis, start.b / axis)
        } else {
            (1.0, 0.0)
        };
        let (dx, dy) = (e - start.e, f - start.f);
        let along = ux * dx + uy * dy;
        let perp = -uy * dx + ux * dy;
        // Perpendicular scale; `hypot(c, d) >= |d|`, so an upright (`b == 0`)
        // run keeps at least the raw `f` band. ~keep
        let line_scale = (start.c * start.c + start.d * start.d).sqrt();
        let tolerance = ((font_size * line_scale).abs() * 0.5).max(0.5);
        perp.abs() <= tolerance && along >= 0.0
    }

    /// Flush accumulated Tj span buffer into a single TextSpan.
    ///
    /// This is similar to flush_tj_buffer but works with the tj_span_buffer field
    /// which accumulates consecutive Tj operators.
    pub(super) fn flush_tj_span_buffer(&mut self) -> Result<()> {
        if let Some(mut buffer) = self.tj_span_buffer.take()
            && !buffer.is_empty()
        {
            let total_width = buffer.accumulated_width * buffer.user_h_scale;

            // Use pre-computed values from buffer creation (avoids
            // matrix multiply + sqrt + HashMap lookup per flush) ~keep
            let effective_font_size = buffer.effective_font_size;
            let font_weight = buffer.font_weight;
            let is_italic_buf = buffer.is_italic;

            // Move owned strings out of buffer (avoids clone) ~keep
            let font_name_buf = buffer.font_name.take().unwrap_or_else(|| "Unknown".to_string());

            // RTL visual-order detection for the Tj-span
            // path, via the shared `apply_rtl_verdict` decision point
            // (also used by `flush_tj_buffer` and `cluster_to_span`) —
            // geometric detector when `char_widths` give us per-char x,
            // falling back to the coarse `accumulated_width > 0`
            // heuristic only when ambiguous. ~keep
            let mut text = std::mem::take(&mut buffer.unicode);
            if text.len() > 1 {
                let has_rtl = text.chars().any(|c| crate::text::rtl_detector::is_rtl_text(c as u32));
                if has_rtl {
                    // char_widths contains text-space relative widths;
                    // reconstruct absolute user-space x by accumulating,
                    // scaling by user_h_scale and offsetting by user_pos_x. ~keep
                    let chars: Vec<char> = text.chars().collect();
                    let verdict = if chars.len() == buffer.char_widths.len() && !buffer.char_widths.is_empty() {
                        let mut chars_with_x: Vec<(char, f32)> = Vec::with_capacity(chars.len());
                        let mut cursor_text_space = 0.0_f32;
                        for (i, c) in chars.iter().enumerate() {
                            let user_x = buffer.user_pos_x + cursor_text_space * buffer.user_h_scale;
                            chars_with_x.push((*c, user_x));
                            cursor_text_space += buffer.char_widths[i];
                        }
                        crate::text::bidi::detect_visual_order_run(&chars_with_x)
                    } else {
                        crate::text::bidi::RunOrder::Ambiguous
                    };
                    text = crate::text::bidi::apply_rtl_verdict(
                        &text,
                        verdict,
                        buffer.accumulated_width > 0.0,
                        matches!(buffer.render_mode, 3 | 7),
                    );
                }
            }

            let span = TextSpan {
                provenance: None,
                text,
                bbox: Rect {
                    x: buffer.user_pos_x,
                    y: buffer.user_pos_y,
                    width: total_width,
                    height: effective_font_size,
                },
                font_name: font_name_buf,
                font_size: effective_font_size,
                font_weight,
                color: Color::new(
                    buffer.fill_color_rgb.0,
                    buffer.fill_color_rgb.1,
                    buffer.fill_color_rgb.2,
                ),
                mcid: buffer.mcid,
                mcid_scope: Some(self.current_mcid_scope()),
                sequence: self.span_sequence_counter,
                split_boundary_before: false,
                offset_semantic: false,
                char_spacing: 0.0,         // Tc - per ISO 32000-1:2008 Section 9.3.1 ~keep
                word_spacing: 0.0,         // Tw - per ISO 32000-1:2008 Section 9.3.1 ~keep
                horizontal_scaling: 100.0, // Tz - per ISO 32000-1:2008 Section 9.3.1 ~keep
                is_italic: is_italic_buf,
                is_monospace: buffer.is_monospace,
                primary_detected: false,
                artifact_type: self.current_artifact_type(),
                char_widths: {
                    let mut cw = std::mem::take(&mut buffer.char_widths);
                    let h = buffer.user_h_scale;
                    for w in &mut cw {
                        *w *= h;
                    }
                    cw
                },
                char_x_offsets: Vec::new(),
                heading_level: None,
                rotation_degrees: buffer.rotation_degrees,
                wmode: buffer.wmode,
                text_rise: buffer.text_rise,
                rtl_draw_logical: false,
                mirrored: buffer.mirrored,
                page_rotation_applied: 0,
            };
            self.span_sequence_counter += 1;

            tracing::trace!(target: LOG_TARGET,
                "FLUSH_TJ_SPAN_BUFFER creating span: text='{}', offset_semantic={} (space-only spans marked as offset_semantic)",
                if span.text.chars().all(|c| c.is_whitespace()) {
                    "<space-only>"
                } else {
                    crate::utils::safe_prefix(&span.text, 20)
                },
                span.offset_semantic
            );

            if !self.is_content_suppressed() {
                self.spans.push(span);
            }
        }
        Ok(())
    }

    pub(super) fn show_text(&mut self, text: &[u8], repair_zero_widths: bool) -> Result<()> {
        // PDF spec Section 7.3.4.2: implementation limit of 32,767 bytes per string. ~keep
        let text = if text.len() > 32_767 {
            tracing::warn!(target: LOG_TARGET,
                "String exceeds PDF spec limit: {} bytes (max 32,767), truncating",
                text.len()
            );
            &text[..32_767]
        } else {
            text
        };

        let state = self.state_stack.current();
        let font_size = state.font_size;
        let horizontal_scaling = state.horizontal_scaling;
        let char_space = state.char_space;
        let word_space = state.word_space;
        let fill_color_rgb = state.fill_color_rgb;
        let ctm = state.ctm;
        let wmode = state.text_wmode;

        let font = self.cached_current_font.as_deref();

        let simple_widths = font
            .filter(|font| font.subtype != "Type0")
            .map(|font| Self::simple_widths(self.cached_extraction_widths.as_deref(), font, repair_zero_widths));

        for (char_code, nbytes) in TextCharIter::new(text, font) {
            // Get current text matrix (may be updated by previous characters in this string) ~keep
            let state = self.state_stack.current();
            let text_matrix = state.text_matrix;

            let unicode_string = if let Some(font) = font {
                font.char_to_unicode(char_code as u32)
                    .unwrap_or_else(|| fallback_char_to_unicode(char_code as u32))
            } else if char_code < 256 && (char_code as u8).is_ascii() {
                (char_code as u8 as char).to_string()
            } else {
                "?".to_string()
            };

            let text_pos = text_matrix.transform_point(0.0, 0.0);
            let pos = ctm.transform_point(text_pos.x, text_pos.y);

            let combined_char = ctm.multiply(&text_matrix);
            let effective_font_size =
                font_size * (combined_char.d * combined_char.d + combined_char.b * combined_char.b).sqrt();

            let glyph_width_font_units = if let Some(widths) = simple_widths.as_ref() {
                widths[char_code as usize]
            } else if let Some(font) = font {
                // GH #1631: `simple_widths` is `None` here only because this
                // font is Type0 (the `None`-font case is handled by the
                // `else` arm below), so `char_code` is a raw content-stream
                // code that must be translated to a CID before the /W
                // lookup. ~keep
                font.get_glyph_width(font.code_to_cid_if_type0(char_code as u32))
            } else {
                500.0
            };

            // font_matrix_a converts glyph-space widths to text-space units.
            // Standard fonts (Type1/TrueType): font_matrix_a = 0.001.
            // Type3 with identity FontMatrix: font_matrix_a = 1.0 (no /1000 division).
            // Assumes FontMatrix[1] = 0 (no glyph-axis rotation), which holds for all
            // standard fonts and virtually all Type3 fonts encountered in practice. ~keep
            let font_matrix_a = font.map(|f| f.font_matrix_a).unwrap_or(0.001);
            let fs_factor = font_size * font_matrix_a;
            let hs_factor = horizontal_scaling / 100.0;
            let glyph_width_user_space = glyph_width_font_units * fs_factor * hs_factor;

            // Advance along the active writing axis per ISO 32000-1 §9.4.4:
            //   horizontal: tx = (w0 * Tfs + Tc + Tw) * Th
            //   vertical:   ty = w1y * Tfs + Tc + Tw    (NO Th — Tz is a
            //               glyph-stretching factor on the X axis only;
            //               see §9.3.4).
            // Word spacing applies only to the SINGLE-BYTE code 32
            // (ISO 32000-1 §9.3.3), never to a multi-byte code whose value
            // happens to be 32. ~keep
            let ws_applies = char_code == 32 && nbytes == 1;
            let mut tx = if wmode == 0 {
                glyph_width_user_space + char_space * hs_factor + if ws_applies { word_space * hs_factor } else { 0.0 }
            } else {
                // GH #1631: translate to a CID before the /W2 lookup. ~keep
                let w1y = font
                    .map(|f| f.get_vertical_metrics(f.code_to_cid_if_type0(char_code as u32)).w1y)
                    .unwrap_or(crate::fonts::VerticalMetrics::SPEC_DEFAULT.w1y);
                w1y * fs_factor + char_space + if ws_applies { word_space } else { 0.0 }
            };

            let glyph_width_device_space = glyph_width_user_space * combined_char.a.abs();
            let tx_device_space = tx * combined_char.a.abs();
            let height_device_space = effective_font_size;
            // Quiet unused-mut warning when wmode != 0 and tx is read-only after this point. ~keep
            let _ = &mut tx;

            let (font_weight, is_italic_char) = if let Some(font) = font {
                (
                    if font.is_bold() {
                        FontWeight::Bold
                    } else {
                        FontWeight::Normal
                    },
                    font.is_italic(),
                )
            } else {
                (FontWeight::Normal, false)
            };

            let (r, g, b) = fill_color_rgb;
            let color = Color::new(r, g, b);

            let final_matrix = ctm.multiply(&text_matrix);
            let rotation_degrees = final_matrix.b.atan2(final_matrix.a).to_degrees();

            // Guard against malformed fonts ~keep
            let unicode_string = if unicode_string.chars().count() > 8 {
                unicode_string.chars().next().unwrap_or('?').to_string()
            } else {
                unicode_string
            };

            let char_count = unicode_string.chars().count();
            let char_width_device = if char_count > 0 {
                glyph_width_device_space / char_count as f32
            } else {
                glyph_width_device_space
            };
            let char_width_user = if char_count > 0 {
                glyph_width_user_space / char_count as f32
            } else {
                glyph_width_user_space
            };
            // Spread the total advance evenly across the ligature's output chars.
            // Tc applies once per character *code*, not per output glyph, so this
            // approximation slightly over-distributes Tc for multi-char ligatures —
            // the same trade-off advance_width already makes for glyph_width_device. ~keep
            let rendered_advance_per_char = if char_count > 0 {
                tx_device_space / char_count as f32
            } else {
                tx_device_space
            };

            for (char_index, unicode_char) in unicode_string.chars().enumerate() {
                let should_skip = unicode_char == '\0'
                    || (unicode_char.is_control()
                        && unicode_char != '\t'
                        && unicode_char != '\n'
                        && unicode_char != '\r');

                if !should_skip {
                    let x_offset_device = char_index as f32 * char_width_device;
                    let x_offset_user = char_index as f32 * char_width_user;

                    let char_origin_x = pos.x + x_offset_device;
                    let char_origin_y = pos.y;

                    let text_char = TextChar {
                        char: unicode_char,
                        bbox: Rect::new(char_origin_x, char_origin_y, char_width_device, height_device_space),
                        font_name: font.map(|f| f.base_font.clone()).unwrap_or_default(),
                        font_size: effective_font_size,
                        font_weight,
                        color,
                        mcid: self.current_mcid,
                        is_italic: is_italic_char,
                        is_monospace: false,
                        origin_x: char_origin_x,
                        origin_y: char_origin_y,
                        rotation_degrees,
                        advance_width: char_width_device,
                        rendered_advance: rendered_advance_per_char,
                        ascent: font.map(|f| f.ascent).unwrap_or(0.95) * effective_font_size,
                        descent: font.map(|f| f.descent).unwrap_or(-0.35) * effective_font_size,
                        matrix: Some([
                            final_matrix.a,
                            final_matrix.b,
                            final_matrix.c,
                            final_matrix.d,
                            final_matrix.e + x_offset_user,
                            final_matrix.f,
                        ]),
                    };

                    if !self.is_content_suppressed() {
                        self.chars.push(text_char);
                    }
                }
            }

            // Update text matrix per ISO 32000-1:2008 §9.4.4. The axis swap
            // (x for WMode 0, y for WMode 1) is encapsulated in
            // advance_text_matrix so this site does not branch. ~keep
            self.state_stack.current_mut().advance_text_matrix(tx);
        }

        Ok(())
    }
}
