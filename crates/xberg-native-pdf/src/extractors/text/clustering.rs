//! Character clustering into spans, including ligature decisions.
//!
//! Split out of the parent's single 5,806-line `impl TextExtractor`, which made
//! `extractors/text.rs` 673 KiB — over the repository's 500 KiB file-safety limit.
//! A child module's `impl` is the same inherent impl and sees the parent's private
//! items unchanged. ~keep

use super::*;

/// Inputs for [`TextExtractor::correct_cluster_rtl_order`], grouped to keep the
/// method's parameter count within the file's quality gate. Purely a carrier —
/// no behavior of its own. ~keep
struct ClusterRtlContext<'a> {
    cluster: &'a [CharacterInfo],
    last: &'a CharacterInfo,
    text_matrix: Matrix,
    ctm: Matrix,
    font_name: Option<&'a str>,
    render_mode: u8,
}

impl<'doc> TextExtractor<'doc> {
    /// Partition character array into clusters at boundary positions.
    ///
    /// # Arguments
    /// * `characters` - Full character array from TJ processing
    /// * `boundaries` - Boundary indices (positions where word boundaries occur)
    ///
    /// # Returns
    /// Vector of character clusters, where boundaries separate clusters
    pub(super) fn partition_characters_by_boundaries(
        &self,
        characters: &[CharacterInfo],
        boundaries: Vec<usize>,
    ) -> Vec<Vec<CharacterInfo>> {
        if boundaries.is_empty() {
            return vec![characters.to_vec()];
        }

        let mut clusters = Vec::new();
        let mut prev = 0;

        for boundary_idx in boundaries {
            if boundary_idx > prev {
                clusters.push(characters[prev..boundary_idx].to_vec());
            }
            prev = boundary_idx;
        }

        if prev < characters.len() {
            clusters.push(characters[prev..].to_vec());
        }

        clusters
    }

    /// Convert a character cluster to a TextSpan.
    ///
    /// Calculates bounding box from character positions and creates
    /// a single TextSpan marked with primary_detected flag.
    ///
    /// # Arguments
    /// * `cluster` - Character cluster from partitioning
    pub(super) fn cluster_to_span(&mut self, cluster: &[CharacterInfo]) -> Result<()> {
        if cluster.is_empty() {
            return Ok(());
        }

        // Snapshot the current MCID scope before borrowing graphics
        // state so the borrow checker doesn't reject the
        // `current_mcid_scope()` call at span construction time. ~keep
        let mcid_scope = self.current_mcid_scope();
        let state = self.state_stack.current();
        let text_matrix = state.text_matrix;
        let ctm = state.ctm;

        // Safety: caller checks cluster.is_empty() above and returns early ~keep
        let last = cluster.last().expect("cluster verified non-empty above");
        let bbox = self.compute_cluster_bbox(cluster, text_matrix, ctm);

        let unicode_text = self.decode_cluster_unicode(cluster, state.font_name.as_deref());
        // Step 3b: RTL text correction. See `correct_cluster_rtl_order` for the
        // full rationale (unchanged, just split out for file size). ~keep
        let unicode_text = self.correct_cluster_rtl_order(
            unicode_text,
            ClusterRtlContext {
                cluster,
                last,
                text_matrix,
                ctm,
                font_name: state.font_name.as_deref(),
                render_mode: state.render_mode,
            },
        );

        let (font_weight, is_italic) = self.cluster_font_style(state.font_name.as_deref());

        let span = TextSpan {
            provenance: None,
            text: unicode_text,
            bbox,
            font_name: state.font_name.clone().unwrap_or_else(|| "Unknown".to_string()),
            font_size: cluster[0].font_size,
            font_weight,
            color: Color::new(state.fill_color_rgb.0, state.fill_color_rgb.1, state.fill_color_rgb.2),
            mcid: self.current_mcid,
            mcid_scope: Some(mcid_scope),
            sequence: self.span_sequence_counter,
            split_boundary_before: false,
            offset_semantic: false,
            char_spacing: state.char_space,
            word_spacing: state.word_space,
            horizontal_scaling: state.horizontal_scaling,
            is_italic,
            is_monospace: false,
            primary_detected: true,
            artifact_type: None,
            char_widths: vec![],
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
        if !self.is_content_suppressed() {
            self.spans.push(span);
        }

        Ok(())
    }

    /// RTL text correction — reverse visual-order characters to logical order.
    ///
    /// PDF stores characters in content-stream order. For RTL scripts
    /// (Arabic / Hebrew), the producer may emit text in either:
    ///   * **visual order** — glyphs drawn left-to-right in user space
    ///     even though the script reads right-to-left (legacy Acrobat
    ///     output, pre-shaped Arabic), OR
    ///   * **logical order** — glyphs drawn right-to-left in user space
    ///     because the producer ran its own bidi pass before drawing
    ///     (modern Word with bidi, the pdfium `hebrew_mirrored.pdf`
    ///     test fixture).
    ///
    /// We use the confidence-gated geometric detector
    /// [`text::bidi::detect_visual_order_run`] when the
    /// cluster has ≥4 RTL letters with clear X-monotonicity. For
    /// shorter clusters (or `Ambiguous` verdict) we fall back to a
    /// simpler `last_x > first_x` heuristic — keeps the
    /// existing 2-3-char RTL run behaviour byte-identical so the
    /// upstream invariants (Arabic CID-TrueType samples, the
    /// `right_to_left_02` fixture) still pass. ~keep
    fn correct_cluster_rtl_order(&self, unicode_text: String, rtl: ClusterRtlContext<'_>) -> String {
        let ClusterRtlContext {
            cluster,
            last,
            text_matrix,
            ctm,
            font_name,
            render_mode,
        } = rtl;
        if unicode_text.len() <= 1 || cluster.len() < 2 {
            return unicode_text;
        }
        let has_rtl = unicode_text
            .chars()
            .any(|c| crate::text::rtl_detector::is_rtl_text(c as u32));
        if !has_rtl {
            return unicode_text;
        }

        // Build (char, user_x) pairs for the geometric detector.
        // One pair per source character — when the decoded
        // string has more chars than the cluster (e.g. ligature
        // expansion `fi` → "fi"), use the first decoded char as
        // a representative since they share the same source x. ~keep
        let font_for_cluster = font_name.and_then(|n| self.fonts.get(n));
        let mut chars_with_x: Vec<(char, f32)> = Vec::with_capacity(cluster.len());
        for ci in cluster {
            let decoded_first = font_for_cluster
                .and_then(|f| f.char_to_unicode(ci.code))
                .and_then(|s| s.chars().next());
            if let Some(c) = decoded_first {
                let p = text_matrix.transform_point(ci.x_position, 0.0);
                let user_x = ctm.transform_point(p.x, p.y).x;
                chars_with_x.push((c, user_x));
            }
        }
        let verdict = crate::text::bidi::detect_visual_order_run(&chars_with_x);
        // The simpler heuristic — used only as the
        // `Ambiguous` fallback (short cluster or mixed signal) so
        // existing 2-3-char RTL runs keep working; the pdfium
        // `hebrew_mirrored.pdf` fixture and similar land on
        // `Logical` above and are left alone regardless. ~keep
        let first_x = {
            let p = text_matrix.transform_point(cluster[0].x_position, 0.0);
            ctm.transform_point(p.x, p.y).x
        };
        let last_x = {
            let p = text_matrix.transform_point(last.x_position, 0.0);
            ctm.transform_point(p.x, p.y).x
        };
        crate::text::bidi::apply_rtl_verdict(&unicode_text, verdict, last_x > first_x, matches!(render_mode, 3 | 7))
    }

    /// Compute a cluster's user-space bounding box from its first/last
    /// character positions, transformed by the current text and CTM
    /// matrices. Per PDF Spec ISO 32000-1:2008 Section 9.4.4 — split out of
    /// `cluster_to_span` unchanged, just for file size. ~keep
    fn compute_cluster_bbox(&self, cluster: &[CharacterInfo], text_matrix: Matrix, ctm: Matrix) -> Rect {
        let text_min_x = cluster[0].x_position;
        // Safety: caller (`cluster_to_span`) checks cluster.is_empty() and returns early ~keep
        let last = cluster.last().expect("cluster verified non-empty by caller");
        let text_max_x = last.x_position + last.width;
        let text_width = (text_max_x - text_min_x).max(0.0);

        let height = cluster[0].font_size.abs() * text_matrix.d.abs().max(1.0);

        let text_pos = text_matrix.transform_point(text_min_x, 0.0);
        let user_pos = ctm.transform_point(text_pos.x, text_pos.y);
        let user_width = text_width * text_matrix.a.abs() * ctm.a.abs();

        Rect {
            x: user_pos.x,
            y: user_pos.y,
            width: user_width.max(text_width), // Use larger of the two for safety ~keep
            height,
        }
    }

    /// Decode a character cluster's codes to Unicode text using the cluster's
    /// current font. Returns an empty string when no font is bound or the font
    /// is not registered — identical to the prior inline fallback. ~keep
    fn decode_cluster_unicode(&self, cluster: &[CharacterInfo], font_name: Option<&str>) -> String {
        let Some(font) = font_name.and_then(|name| self.fonts.get(name)) else {
            return String::new();
        };
        let unmapped_type3 =
            font.subtype == "Type3" && font.best_mapping_provenance() == crate::fonts::MappingProvenance::Fallback;
        let mut text = String::new();
        for char_info in cluster {
            if let Some(decoded) = font.char_to_unicode(char_info.code) {
                text.push_str(&decoded);
            } else if unmapped_type3 {
                text.push('?');
            }
        }
        text
    }

    /// Resolve the cluster's font weight and italic flag from the current font.
    /// Defaults to `(FontWeight::Normal, false)` when no font is bound or
    /// registered — identical to the prior inline fallbacks. ~keep
    fn cluster_font_style(&self, font_name: Option<&str>) -> (FontWeight, bool) {
        let Some(font) = font_name.and_then(|name| self.fonts.get(name)) else {
            return (FontWeight::Normal, false);
        };
        let weight = if font.is_bold() {
            FontWeight::Bold
        } else {
            FontWeight::Normal
        };
        (weight, font.is_italic())
    }

    /// Check if a character code is a ligature (U+FB00-U+FB04).
    ///
    /// Standard ligatures supported:
    /// - U+FB00: ff (LATIN SMALL LIGATURE FF)
    /// - U+FB01: fi (LATIN SMALL LIGATURE FI)
    /// - U+FB02: fl (LATIN SMALL LIGATURE FL)
    /// - U+FB03: ffi (LATIN SMALL LIGATURE FFI)
    /// - U+FB04: ffl (LATIN SMALL LIGATURE FFL)
    pub(super) fn is_ligature_code(code: u32) -> bool {
        matches!(code, 0xFB00..=0xFB04)
    }

    /// Apply ligature expansion decisions after word boundary detection.
    ///
    /// This method processes the character array after boundary detection,
    /// making intelligent decisions about whether to split ligatures.
    ///
    /// Algorithm:
    /// 1. Iterate through character array
    /// 2. For each ligature character:
    ///    - Get next character (if exists)
    ///    - Call LigatureDecisionMaker::decide()
    ///    - If Split: expand to component characters with proportional widths
    ///    - If Keep: leave as-is
    /// 3. Recalculate x_positions for all following characters after splits
    pub(super) fn apply_ligature_decisions(&mut self) -> Result<()> {
        use crate::text::ligature_processor::{LigatureDecision, LigatureDecisionMaker, expand_ligature_to_chars};

        let context = self.create_boundary_context();
        let mut result = Vec::new();
        let mut i = 0;

        // OPTIMIZATION: Single-pass reconstruction instead of Vec::insert() in loop
        // This fixes O(n²) complexity to O(n) by avoiding repeated insertions
        // Avoiding Vec::insert fixes a 50× slowdown for ligature-heavy PDFs ~keep
        while i < self.tj_character_array.len() {
            let char_info = &self.tj_character_array[i];

            if !char_info.is_ligature {
                result.push(char_info.clone());
                i += 1;
                continue;
            }

            // Get next character without cloning (eliminates unnecessary clones) ~keep
            let next_char = if i + 1 < self.tj_character_array.len() {
                Some(&self.tj_character_array[i + 1])
            } else {
                None
            };

            let decision = LigatureDecisionMaker::decide(char_info, &context, next_char);

            if decision == LigatureDecision::Split {
                let ligature_char = char::from_u32(char_info.code).unwrap_or('?');
                let original_width = char_info.width;
                let original_x = char_info.x_position;
                let font_size = char_info.font_size;

                let components = expand_ligature_to_chars(ligature_char, original_width);

                if !components.is_empty() {
                    let mut x_offset = 0.0;
                    result.push(CharacterInfo {
                        code: components[0].0 as u32,
                        glyph_id: char_info.glyph_id,
                        width: components[0].1,
                        x_position: original_x,
                        tj_offset: char_info.tj_offset,
                        font_size,
                        is_ligature: false,
                        original_ligature: Some(ligature_char),
                        protected_from_split: char_info.protected_from_split,
                    });
                    x_offset += components[0].1;

                    for (comp_char, comp_width) in components.iter().skip(1) {
                        result.push(CharacterInfo {
                            code: *comp_char as u32,
                            glyph_id: None,
                            width: *comp_width,
                            x_position: original_x + x_offset,
                            tj_offset: None,
                            font_size,
                            is_ligature: false,
                            original_ligature: Some(ligature_char),
                            protected_from_split: false,
                        });
                        x_offset += comp_width;
                    }
                } else {
                    result.push(char_info.clone());
                }
            } else {
                result.push(char_info.clone());
            }

            i += 1;
        }

        // OPTIMIZATION: Replace entire array once instead of multiple insertions ~keep
        self.tj_character_array = result;
        Ok(())
    }
}
