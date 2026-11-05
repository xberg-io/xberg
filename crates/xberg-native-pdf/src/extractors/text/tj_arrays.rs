//! TJ-array buffering and displacement handling.
//!
//! Split out of the parent's single 5,806-line `impl TextExtractor`, which made
//! `extractors/text.rs` 673 KiB — over the repository's 500 KiB file-safety limit.
//! A child module's `impl` is the same inherent impl and sees the parent's private
//! items unchanged. ~keep

use super::*;

/// Text-state snapshot needed to size and advance one `TJ` show-string's
/// glyphs, captured once per array so per-character work doesn't re-borrow
/// `self.state_stack`.
struct TjCharacterMetrics {
    font_size: f32,
    horizontal_scaling: f32,
    char_space: f32,
    word_space: f32,
}

impl<'doc> TextExtractor<'doc> {
    /// Get the current artifact type from the marked content stack.
    pub(super) fn current_artifact_type(&self) -> Option<ArtifactType> {
        self.marked_content_stack
            .iter()
            .rev()
            .find_map(|ctx| ctx.artifact_type.clone())
    }

    /// Flush accumulated TJ buffer into a single TextSpan.
    ///
    /// This creates one span for the entire buffer content, properly calculating
    /// the total width including character spacing (Tc) and word spacing (Tw).
    pub(super) fn flush_tj_buffer(&mut self, mut buffer: TjBuffer) -> Result<()> {
        if buffer.is_empty() {
            return Ok(());
        }

        let total_width = buffer.accumulated_width * buffer.user_h_scale;

        // Use pre-computed values from buffer creation (avoids
        // matrix multiply + sqrt + HashMap lookup + transform_point per flush) ~keep
        let effective_font_size = buffer.effective_font_size;
        let font_weight = buffer.font_weight;
        let is_italic_span = buffer.is_italic;

        // Move owned strings out of buffer (avoids clone) ~keep
        let font_name_span = buffer.font_name.take().unwrap_or_else(|| "Unknown".to_string());

        // RTL text correction. See `correct_tj_buffer_rtl_order` for the full
        // rationale (unchanged, just split out for file size). ~keep
        let text = std::mem::take(&mut buffer.unicode);
        let text = Self::correct_tj_buffer_rtl_order(text, &buffer);

        let span = TextSpan {
            provenance: None,
            text,
            bbox: Rect {
                x: buffer.user_pos_x,
                y: buffer.user_pos_y,
                width: total_width,
                height: effective_font_size,
            },
            font_name: font_name_span,
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
            char_spacing: buffer.char_space, // Tc - captured from PDF content stream ~keep
            word_spacing: buffer.word_space, // Tw - captured from PDF content stream ~keep
            horizontal_scaling: buffer.horizontal_scaling,
            // ~keep
            is_italic: is_italic_span,
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

        self.push_span(span);
        Ok(())
    }

    /// RTL text correction for a flushed TJ buffer: use the confidence-gated
    /// geometric detector when `char_widths` gives us per-character
    /// user-space x-positions, falling back to the coarse "buffer's net
    /// horizontal advance is positive" heuristic only for genuinely
    /// ambiguous/short runs. Mirrors `flush_tj_span_buffer`'s handling — this
    /// used to be the one flush site still on the older
    /// `accumulated_width > 0.0` check, which (since `accumulated_width`
    /// only ever sums *positive* glyph widths — TJ kerning offsets never
    /// subtract from it) is true for nearly every non-empty RTL buffer and
    /// so was unconditionally reversing every RTL run regardless of its
    /// actual source order. ~keep
    fn correct_tj_buffer_rtl_order(text: String, buffer: &TjBuffer) -> String {
        if text.len() <= 1 {
            return text;
        }
        let has_rtl = text.chars().any(|c| crate::text::rtl_detector::is_rtl_text(c as u32));
        if !has_rtl {
            return text;
        }
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
        crate::text::bidi::apply_rtl_verdict(
            &text,
            verdict,
            buffer.accumulated_width > 0.0,
            matches!(buffer.render_mode, 3 | 7),
        )
    }

    /// Calculate total width of TJ buffer using PDF spec formula.
    ///
    /// Process TJ array according to configured word boundary detection mode.
    ///
    /// Per PDF Spec ISO 32000-1:2008 Section 9.4.4,
    /// this method dispatches to either:
    /// - process_tj_array_tiebreaker(): WordBoundaryMode::Tiebreaker (default)
    /// - process_tj_array_primary(): WordBoundaryMode::Primary
    pub(super) fn process_tj_array(&mut self, array: &[TextElement]) -> Result<()> {
        // A bare `Tj` immediately before this array leaves `self.tj_span_buffer` open: `Tj`
        // buffers into that field, while `TJ` runs through its own local buffer below and never
        // reads or clears it. Every other run-closing boundary -- Td, TD, T*, a non-continuing
        // Tm, BMC/BDC, ' and ", ET -- already flushes it first; TJ was the sole omission. Left
        // open, the buffer keeps the origin captured at its first `Tj`, so a later `Tj` appends
        // onto it and the eventual flush emits the combined run at that stale x. The
        // reading-order sort then splices it in beside whatever sits near that stale position,
        // scrambling the line (GH#1544). ~keep
        self.flush_tj_span_buffer()?;
        match self.word_boundary_mode {
            WordBoundaryMode::Tiebreaker => self.process_tj_array_tiebreaker(array),
            WordBoundaryMode::Primary => self.process_tj_array_primary(array),
        }
    }

    pub(super) fn has_following_tj_displacement(array: &[TextElement], index: usize) -> bool {
        matches!(array.get(index + 1), Some(TextElement::Offset(offset)) if *offset != 0.0)
    }

    /// Process TJ array using tiebreaker mode (backward compatible).
    ///
    /// This is the legacy code path used when
    /// WordBoundaryMode::Tiebreaker is configured.
    ///
    /// Maintains 100% backward compatibility with existing behavior.
    /// Word boundaries are detected only as a tiebreaker when TJ offset
    /// and geometric signals contradict each other.
    ///
    /// Per PDF Spec ISO 32000-1:2008, Section 9.4.4 NOTE 6:
    /// "The performance of text searching (and other text extraction operations) is
    /// significantly better if the text strings are as long as possible."
    ///
    /// This method buffers consecutive strings into a single span, only breaking on:
    /// - Large negative offsets (indicating word boundaries)
    /// - End of TJ array
    fn process_tj_array_tiebreaker(&mut self, array: &[TextElement]) -> Result<()> {
        // Character-level tracking for word boundary detection
        // Collect detailed character information during TJ array processing
        // Per ISO 32000-1:2008 Section 9.4.4, character-level data improves accuracy ~keep

        self.tj_character_array.clear();
        self.current_x_position = 0.0;

        // Copy state data to avoid holding reference while borrowing self mutably ~keep
        let font_size = self.state_stack.current().font_size;
        let horizontal_scaling = self.state_stack.current().horizontal_scaling / 100.0;
        let font_name = self.state_stack.current().font_name.clone();
        let char_space = self.state_stack.current().char_space;
        let word_space = self.state_stack.current().word_space;

        let mut buffer = TjBuffer::new_with_provenance_cache(
            self.state_stack.current(),
            self.current_mcid,
            self.cached_current_font.clone(),
            &self.font_provenance_cache,
        );
        let mut _element_count = 0;

        for (idx, element) in array.iter().enumerate() {
            _element_count += 1;
            match element {
                TextElement::String(s) => {
                    let metrics = TjCharacterMetrics {
                        font_size,
                        horizontal_scaling,
                        char_space,
                        word_space,
                    };
                    self.process_tj_string_element(array, idx, s, &metrics, font_name.as_deref(), &mut buffer)?;
                }
                TextElement::Offset(offset) => {
                    buffer = self.process_tj_offset_element(array, idx, *offset, buffer)?;
                }
            }
        }

        if !buffer.is_empty() {
            self.flush_tj_buffer(buffer)?;
        }

        Ok(())
    }

    /// Track per-character info for one `TJ` show-string element and append
    /// it to the accumulating buffer. Split out of
    /// [`Self::process_tj_array_tiebreaker`] to keep it short. ~keep
    fn process_tj_string_element(
        &mut self,
        array: &[TextElement],
        idx: usize,
        s: &[u8],
        metrics: &TjCharacterMetrics,
        font_name: Option<&str>,
        buffer: &mut TjBuffer,
    ) -> Result<()> {
        if let Some(name) = font_name
            && let Some(font) = self.fonts.get(name)
        {
            let width_table = Self::simple_widths(
                self.cached_extraction_widths.as_deref(),
                font,
                !Self::has_following_tj_displacement(array, idx),
            );
            for &byte in s.iter() {
                // Normalize character code through encoding.
                // This ensures word boundary detection works on actual characters,
                // not raw byte codes from custom encodings ~keep
                let char_code = font.get_encoded_char(byte).map(|ch| ch as u32).unwrap_or(byte as u32);

                let glyph_width = width_table[byte as usize];

                let is_ligature = Self::is_ligature_code(char_code);

                // Create CharacterInfo for this character
                // The tj_offset will be applied when we encounter the next Offset element
                // ~keep
                let char_info = CharacterInfo {
                    code: char_code,
                    glyph_id: None, // Could be enhanced to extract actual GID ~keep
                    width: glyph_width,
                    x_position: self.current_x_position,
                    tj_offset: None,
                    font_size: metrics.font_size,
                    is_ligature,
                    original_ligature: None,
                    protected_from_split: false,
                };

                self.tj_character_array.push(char_info);

                let char_advance = glyph_width * metrics.horizontal_scaling
                    + metrics.char_space
                    + (if byte == 0x20 { metrics.word_space } else { 0.0 });
                self.current_x_position += char_advance;
            }
        }

        let repair_zero_widths = !Self::has_following_tj_displacement(array, idx);
        self.append_advance_buffer(buffer, s, repair_zero_widths)
    }

    /// Apply one `TJ` numeric offset: update the running statistics, decide
    /// whether it is a word boundary, and either flush `buffer` into a span
    /// (returning a fresh one) or fold the offset into it. Split out of
    /// [`Self::process_tj_array_tiebreaker`] to keep it short. ~keep
    fn process_tj_offset_element(
        &mut self,
        array: &[TextElement],
        idx: usize,
        offset: f32,
        mut buffer: TjBuffer,
    ) -> Result<TjBuffer> {
        // Track TJ offset for statistical analysis
        // Per ISO 32000-1:2008 Section 9.4.4, collect all TJ values
        // to detect justified vs normal text through coefficient of variation ~keep
        if self.tj_offset_history.len() < 10000 {
            // Keep history reasonable size (first 10k offsets per document)
            // and update the running accumulators. ~keep
            let x = offset as f64;
            self.tj_sum += x;
            self.tj_sum_sq += x * x;
            self.tj_offset_history.push(offset);
            self.tj_stats_len = self.tj_offset_history.len();
        }

        // Associate TJ offset with the last character
        // The offset applies AFTER the previous string, affecting spacing to next string
        // ~keep
        if !self.tj_character_array.is_empty() {
            let last_idx = self.tj_character_array.len() - 1;
            self.tj_character_array[last_idx].tj_offset = Some(offset as i32);
        }

        // Check if this offset indicates a word boundary
        // Per PDF spec: negative offsets increase spacing
        // Use geometry-based adaptive threshold ~keep
        let threshold = self.calculate_adaptive_tj_threshold();
        if offset < threshold {
            self.flush_buffer_at_tj_word_boundary(array, idx, offset, buffer)
        } else {
            // Sub-threshold offset: matrix advances but the
            // current buffer keeps accumulating, so apply
            // the offset unconditionally here as well. ~keep
            self.advance_position_for_offset(offset)?;
            // Fold the same displacement into the buffer's
            // advance record. Historically only the text matrix
            // moved, so these kerning/word-space offsets were
            // dropped from `char_widths`/`accumulated_width` —
            // leaving the span's reconstructed per-glyph positions
            // drifting behind the true render (poppler/PDFium/
            // pymupdf all fold the offset into the advance). On
            // justified body text drawn as one continuous buffer,
            // the many small post-space offsets accumulate into a
            // multi-point undershoot. Folding keeps
            // `sum(char_widths) == accumulated_width == matrix
            // advance` by construction. ~keep
            self.fold_offset_into_buffer(&mut buffer, offset);
            Ok(buffer)
        }
    }

    /// A `TJ` offset past the word-boundary threshold: flush the current
    /// buffer into a span, insert a space unless one is already implied, and
    /// start a fresh buffer positioned after the offset. Split out of
    /// [`Self::process_tj_offset_element`] to keep it short. ~keep
    fn flush_buffer_at_tj_word_boundary(
        &mut self,
        array: &[TextElement],
        idx: usize,
        offset: f32,
        buffer: TjBuffer,
    ) -> Result<TjBuffer> {
        // Note: split-word symptoms ("diffe rent", "cha nge",
        // "equivalen t") are handled at the higher level by the
        // intra-word kerning guard in `should_insert_space`. An
        // earlier TJ-side guard here (commit b2c6484) used a
        // letter-letter + |offset| < space-glyph-width rule, but
        // that rule misclassified real inter-word gaps in
        // tightly-justified PDFs (LaTeX academic papers, Docling
        // output) where producers encode word boundaries as TJ
        // offsets smaller than a full space glyph. The
        // span-merge-time guard has more context (full bbox,
        // WordBoundaryDetector) and avoids that false positive. ~keep
        //
        // Check if buffer ends with space BEFORE flushing
        // This prevents double spaces when TJ processor inserts space
        // AND span merging would insert space at the same boundary. ~keep
        let buffer_ends_with_space = !buffer.unicode.is_empty()
            && buffer
                .unicode
                .chars()
                .next_back()
                .map(|c| c.is_whitespace())
                .unwrap_or(false);

        self.flush_tj_buffer(buffer)?;

        // Check if the next element in the TJ array is a string
        // that starts with whitespace. If so, DON'T insert a space to avoid doubling.
        // This prevents patterns like "word " + " next" = "word next" (double space)
        // ~keep
        let next_element_starts_with_space = if idx + 1 < array.len() {
            if let TextElement::String(next_s) = &array[idx + 1] {
                next_s
                    .first()
                    .is_some_and(|&byte| byte == 0x20 || byte == 0x09 || byte == 0x0A || byte == 0x0D)
            } else {
                false
            }
        } else {
            false
        };

        if !buffer_ends_with_space && !next_element_starts_with_space {
            self.insert_space_as_span()?;
        }

        // Apply the TJ offset to the text matrix BEFORE
        // creating the new buffer so its `user_pos_x`
        // captures the actual draw position of the next
        // string. Otherwise the buffer anchors at the
        // pre-offset position and every subsequent span
        // on the line inherits the missing tx. ~keep
        self.advance_position_for_offset(offset)?;

        Ok(TjBuffer::new_with_provenance_cache(
            self.state_stack.current(),
            self.current_mcid,
            self.cached_current_font.clone(),
            &self.font_provenance_cache,
        ))
    }

    /// Process TJ array using primary detection mode.
    ///
    /// This implementation:
    /// 1. Creates BoundaryContext from graphics state
    /// 2. Calls WordBoundaryDetector to detect boundaries in tj_character_array
    /// 3. Apply ligature expansion decisions
    /// 4. Partitions characters into clusters at boundary positions
    /// 5. Converts each cluster to a TextSpan with proper bounding boxes
    /// 6. Marks spans with primary_detected flag
    fn process_tj_array_primary(&mut self, array: &[TextElement]) -> Result<()> {
        if self.tj_character_array.is_empty() {
            return self.process_tj_array_tiebreaker(array);
        }

        // Mark pattern contexts BEFORE boundary detection
        // This protects email and URL patterns from being split at word boundaries ~keep
        let pattern_config = crate::extractors::PatternPreservationConfig::default();
        crate::extractors::PatternDetector::mark_pattern_contexts(&mut self.tj_character_array, &pattern_config)?;

        let context = self.create_boundary_context();

        // Step 3: Create WordBoundaryDetector and detect boundaries
        // OPTIMIZATION: Detect document script profile to skip unnecessary detectors
        // ~keep
        let script = DocumentScript::detect_from_characters(&self.tj_character_array);
        let detector = WordBoundaryDetector::new().with_document_script(script);
        let boundaries = detector.detect_word_boundaries(&self.tj_character_array, &context);

        if boundaries.is_empty() {
            return self.process_tj_array_tiebreaker(array);
        }

        self.apply_ligature_decisions()?;

        let clusters = self.partition_characters_by_boundaries(&self.tj_character_array, boundaries);

        for cluster in clusters {
            if !cluster.is_empty() {
                self.cluster_to_span(&cluster)?;
            }
        }

        Ok(())
    }

    /// Create BoundaryContext from current graphics state.
    ///
    /// Per ISO 32000-1:2008 Section 9.3, extracts text state parameters
    /// used by WordBoundaryDetector to make boundary decisions.
    pub(super) fn create_boundary_context(&self) -> BoundaryContext {
        let state = self.state_stack.current();
        BoundaryContext {
            font_size: state.font_size,
            horizontal_scaling: state.horizontal_scaling,
            word_spacing: state.word_space,
            char_spacing: state.char_space,
        }
    }
}
