//! Adjacent-span merging.
//!
//! Split out of the parent's single 5,806-line `impl TextExtractor`, which made
//! `extractors/text.rs` 673 KiB — over the repository's 500 KiB file-safety limit.
//! A child module's `impl` is the same inherent impl and sees the parent's private
//! items unchanged. ~keep

use super::*;

impl<'doc> TextExtractor<'doc> {
    /// This matches the behavior of industry-standard PDF tools.
    pub(super) fn merge_adjacent_spans(&mut self) {
        if self.spans.is_empty() {
            return;
        }

        let old_len = self.spans.len();
        let spans = std::mem::take(&mut self.spans);
        // Geometry of every drawn (non-whitespace) glyph run, captured
        // before the fold consumes the list. The decimal-merge branch below
        // needs it: a separator glyph between two digit runs — the comma of
        // a subscript index pair like `P_{1,0}` — is often drawn elsewhere
        // in the content stream, so the fold sees the digits as adjacent
        // and only a geometric test over ALL runs can spot the ink sitting
        // in the gap. ~keep
        let ink_boxes: Vec<Rect> = spans
            .iter()
            .filter(|s| !s.text.trim().is_empty())
            .map(|s| s.bbox)
            .collect();
        // Per-line bimodal word-gap thresholds, indexed to `spans`,
        // used below to rescue a narrow word gap the fixed kerning guard
        // suppressed. Computed before the fold consumes the list. ~keep
        let line_thresholds = Self::bimodal_line_gap_thresholds(&spans);
        let mut merged = Vec::with_capacity(old_len);
        let mut current_span: Option<TextSpan> = None;

        for (span_idx, span) in spans.into_iter().enumerate() {
            if current_span.is_none() {
                current_span = Some(span);
                continue;
            }

            // Take ownership of current to avoid borrow checker issues.
            // Safety: checked is_none() above which continues, so this is always Some. ~keep
            let mut current = match current_span.take() {
                Some(s) => s,
                None => {
                    current_span = Some(span);
                    continue;
                }
            };

            // Spans drawn under different writing modes must never merge,
            // even when their baselines coincide. A horizontal (`wmode=0`)
            // span advances along x; a vertical (`wmode=1`) span advances
            // along y. The text-level merge semantics (same word, same line,
            // gap small enough to glue) all assume a single advance axis,
            // and the bbox-extension at the end of the merge branch grows
            // a horizontal bbox even if the right-hand side was a vertical
            // column. Fold per-span wmode into the line-equality test
            // up-front so all downstream merge variants (same-font,
            // cross-font glue, small-caps, decimal-merge) inherit the
            // gate. Without this, `BT 100 700 Td /F1 12 Tf (A) Tj
            // /F2 12 Tf (B) Tj ET` with F1 horizontal + F2 vertical glues
            // the two glyphs into a single horizontal span and clobbers
            // the wmode metadata for the vertical glyph. ~keep
            let wmode_compatible = current.wmode == span.wmode;
            // ±90°-rotated runs (text matrix rotation, not wmode) advance
            // along Y with their line axis on X, so the portrait same-line
            // test below reads PERPENDICULAR geometry for them: two runs
            // from adjacent rotated lines share a baseline-Y and sit a
            // word-gap apart in X, which glued words from different lines
            // of a rotated table into one span ("row" + "row" → "row row",
            //). Runs in a rotated frame never merge here; each
            // stays per-literal and the rotated-frame reading order and
            // word assembly handle them downstream.
            // "Never merge here" per the comment above means exactly that —
            // any non-zero rotation on either side stays per-literal, not
            // just the ±90° (vertical-quadrant) case. A 180°-rotated run
            // previously slipped through this gate (only ±90° was checked),
            // letting two upside-down lines' runs glue together under the
            // portrait same-line test below even though 180° text advances
            // in the opposite X direction. ~keep
            let rotation_compatible = current.rotation_degrees == 0.0 && span.rotation_degrees == 0.0;
            let y_diff = (span.bbox.y - current.bbox.y).abs();
            let same_line = y_diff < 1.0 && wmode_compatible && rotation_compatible;

            let current_end_x = current.bbox.x + current.bbox.width;
            let gap = span.bbox.x - current_end_x;
            // Fallback-width correction: When the previous
            // span's font has no explicit `/Widths` array, every glyph in
            // that span reports the 500/550/600-thousandths-of-em fallback
            // from `FontInfo::new`. For proportional Latin fonts whose
            // real glyphs are narrower than that fallback (`SR` in the
            // NASA Apollo report is a concrete example), the span's
            // `bbox.width` is systematically inflated and `current_end_x`
            // overshoots the actual end of the rendered text — often by
            // enough to swallow the real inter-word gap entirely, turning
            // the visible word boundary into a negative `gap` value
            // tripping merge logic that then glues the words without a
            // space.
            //
            // `space_gap` is a corrected gap value used ONLY for the
            // space-insertion decision below. The original `gap` is left
            // unchanged so the merge-vs-column decision, the decimal-merge
            // heuristic, and any downstream branch that reasons about the
            // actual bbox layout still see the real layout and don't
            // suddenly reclassify legitimate adjacent words as column
            // boundaries. In other words: the merge still happens exactly
            // as before on fallback-width fonts, but once we're inside the
            // merge branch we consult a more honest gap to decide whether
            // a space is warranted. ~keep
            let reliable_widths = self
                .fonts
                .get(&current.font_name)
                .map(|f| f.has_explicit_widths())
                .unwrap_or(true);
            let space_gap = corrected_space_gap(
                gap,
                reliable_widths,
                current.bbox.width,
                current.text.is_empty(),
                current_end_x.abs().max(span.bbox.x.abs()),
            );

            // Column-boundary gap, font-size-aware. The same 6pt gap is
            // a column gutter at 11pt body text but normal word kerning
            // at a 36pt title; use 0.5em as a floor above the configured
            // absolute threshold. ~keep
            let font_size_ref = current.font_size.max(span.font_size);
            let column_threshold = self
                .merging_config
                .column_boundary_threshold_pt
                .max(font_size_ref * 0.5);
            let large_gap_indicates_column = gap > column_threshold;

            // SPLIT BOUNDARY CHECK: Respect boundaries from CamelCase splitting
            // If a span has split_boundary_before=true, it represents a word boundary
            // from a split operation (e.g., "the" + "General" from "theGeneral")
            // These should always be merged WITH a space, never without. ~keep
            let has_split_boundary = span.split_boundary_before;

            let is_same_font = current.font_name == span.font_name
                && (current.font_size - span.font_size).abs() < 0.01
                && current.font_weight == span.font_weight
                && current.is_italic == span.is_italic;

            // MCID identity: per ISO 32000-1:2008 §14.6, two adjacent
            // Tj operators that sit in different marked-content
            // sequences belong to different *structure elements*.
            // Merging them would silently fuse their identities (the
            // merged span keeps `current.mcid`) and lose the
            // boundary that downstream consumers — structure-tree
            // reading order, tree-scope ActualText suppression,
            // table-cell membership — rely on. Adjacent spans whose
            // MCIDs differ (including one `None` ↔ one `Some(_)`)
            // are kept separate. ~keep
            let same_mcid = current.mcid == span.mcid;

            // Cross-font word glue: same-baseline spans in different
            // fonts/weights, tight gap (<0.25em), both sides alphabetic,
            // and one side is a single character. Targets the drop-cap /
            // single-letter-small-caps typography pattern where per-
            // letter emphasis runs would corrupt proper nouns.
            //
            // CJK ideographs satisfy
            // `is_alphabetic()` per Unicode, so a CJK→Latin (or Latin→CJK)
            // transition between adjacent characters in different fonts —
            // the standard mixed-script PDF layout pattern — was triggering
            // cross-font glue and concatenating "神鹰集团" + "Z" into
            // "神鹰集团Z" with no separator. Word-F1 against pdftotext
            // ground truth (which inserts a space at every CJK↔non-CJK
            // boundary) then loses both the trailing CJK token and the
            // leading Latin/digit token. Skip cross-font glue when the
            // boundary crosses CJK / non-CJK scripts.
            //
            // EXCLUDES fullwidth ASCII (U+FF01..FF5E) and CJK Symbols
            // Punctuation (U+3000..303F) — those operator-style glyphs sit
            // inline with adjacent Latin/digit in CJK technical writing
            // (e.g. "60000≤Q＜80000"). Treating them as a CJK
            // boundary would split the compound token. ~keep
            let is_cjk_char = |c: char| {
                matches!(
                    c as u32,
                    0x3040..=0x309F      // Hiragana ~keep
                    | 0x30A0..=0x30FF    // Katakana ~keep
                    | 0x3400..=0x4DBF    // CJK Unified Ideographs Extension A ~keep
                    | 0x4E00..=0x9FFF    // CJK Unified Ideographs ~keep
                    | 0xAC00..=0xD7AF    // Hangul Syllables ~keep
                    | 0x20000..=0x2A6DF  // CJK Unified Ideographs Extension B ~keep
                    | 0xFF66..=0xFF9F    // Halfwidth Katakana ~keep
                )
            };
            let prev_tail_char = current.text.chars().last();
            let curr_head_char = span.text.chars().next();
            let crosses_cjk_boundary = match (prev_tail_char, curr_head_char) {
                (Some(p), Some(c)) => is_cjk_char(p) != is_cjk_char(c),
                _ => false,
            };
            // Drop-caps / single-letter emphasis sit TIGHT against their word
            // (gap ~0, often overlapping). A gap in word-space territory
            // (≥~0.15em) across a font change is a genuine token boundary —
            // typically a word followed by a single-letter math variable in a
            // math-italic run (`solution` → `U`). Gluing those drops the space
            // poppler/PDFium keep. The 0.12em ceiling is the valley between
            // drop-cap kerning (~0) and a word space (≥~0.2em). (Previously
            // 0.25em, which is itself a full word space: an
            // advance-fold made per-glyph advance accurate enough that these
            // ~0.24em gaps — formerly inflated above 0.25em by the advance
            // undershoot — dropped under the ceiling and began gluing.) ~keep
            let cross_font_word_glue = !is_same_font
                && same_line
                && gap > -1.0
                && gap < font_size_ref * 0.12
                && !current.text.is_empty()
                && !span.text.is_empty()
                && !crosses_cjk_boundary
                && prev_tail_char.is_some_and(|c| c.is_alphabetic())
                && curr_head_char.is_some_and(|c| c.is_alphabetic())
                && (current.text.chars().count() == 1 || span.text.chars().count() == 1);

            // Small-caps / drop-cap glue: same base font and same
            // weight/italic flags but different font_size, adjacent
            // on the same baseline, both alphabetic. PDFs simulate
            // small-caps by rendering the capital initial at body
            // font size and the remaining letters at a reduced
            // size in the same font, emitted as separate Tj runs
            // with zero gap between them. The strict `is_same_font`
            // gate rejects the merge because of the size mismatch,
            // and the single-character drop-cap glue above doesn't
            // help when both runs are multi-character (an initial
            // run of several full-size capitals followed by a
            // reduced-size remainder). Spec basis: PDF §9.3.1
            // treats font_size as a graphics-state parameter that
            // may change between Tj operators; nothing in §9.4
            // makes such a change a word boundary. ~keep
            let small_caps_glue = !is_same_font
                && current.font_name == span.font_name
                && current.font_weight == span.font_weight
                && current.is_italic == span.is_italic
                && same_line
                && gap.abs() < 1.0
                && !current.text.is_empty()
                && !span.text.is_empty()
                && !crosses_cjk_boundary
                && prev_tail_char.is_some_and(|c| c.is_alphabetic())
                && curr_head_char.is_some_and(|c| c.is_alphabetic());

            // Merge threshold: Use configured values
            // Negative gaps: use severe_overlap_threshold_pt (default -0.5pt)
            // Positive gaps: use a threshold that allows for justified text but
            // avoids merging across clear column boundaries.
            // Same-font spans are merged more aggressively to reconstruct words. ~keep
            let merge_threshold_pt = if is_same_font {
                column_threshold.max(3.0)
            } else {
                // Different fonts: only merge if they are effectively overlapping
                // to handle minor kerning/rounding issues, but generally keep separate. ~keep
                0.5
            };

            let should_merge = (same_line
                && is_same_font
                && same_mcid
                && (self.merging_config.severe_overlap_threshold_pt..merge_threshold_pt).contains(&gap)
                && !large_gap_indicates_column)
                || (same_line && has_split_boundary && same_mcid)
                || (cross_font_word_glue && same_mcid)
                || (small_caps_glue && same_mcid);

            // DECIMAL VALUE MERGE: Some forms place integer and decimal parts
            // of dollar amounts in separate fixed-width boxes.
            // e.g., "123456" (integer box) + "72" (cents box) with ~10pt gap.
            // Detect this pattern: both spans are pure digits, the second is
            // exactly 1-2 digits (cents), same line, and there's a meaningful
            // column-boundary-sized gap between them.
            //
            // Without a minimum-gap floor this
            // also matches tightly-packed adjacent digit characters from CJK
            // documents that emit each glyph as its own Tj — e.g. the year
            // "2013" rendered as four separate TjL operators with sub-pixel
            // gaps was being mangled into "201.3", losing the year token from
            // word-F1 scoring. Real "$123 _ 45" split-box layouts always have
            // a gap > ~half the font size; tight letter spacing is < 0.1 em.
            // A separator glyph drawn INSIDE the gap is proof the two digit
            // runs are distinct tokens (the comma of a subscript index pair
            // like `P_{1,0}`, drawn out of content-stream order): a genuine
            // split-box amount has nothing between its boxes. The gap band
            // alone cannot make this call — an index pair and a real
            // split-box amount can sit at the same gap-to-font-size ratio.
            // A genuine split-box amount prints its integer and cents at
            // the SAME size; a digit run markedly smaller than its
            // neighbour is super/subscript context (the exponent of a
            // scientific-notation value next to the following value's
            // mantissa), and fusing those fabricates a decimal. ~keep
            let decimal_sizes_match = {
                let (a, b) = (current.font_size, span.font_size);
                a > 0.0 && b > 0.0 && (a.min(b) / a.max(b)) >= 0.85
            };
            //
            // The gap also needs an upper ceiling. In scientific and math
            // PDFs, subscript index pairs like `P_{1,0}` draw the two subscript
            // digits in a smaller font (~7pt) spaced ~1.5-1.7x the font size
            // apart; too loose a ceiling lets the rule fire and invent a
            // decimal ("1" + "0" -> "1.0"). Genuine split-box amounts cluster
            // near ~0.8-1.0x the font size, so a 1.3x ceiling separates real
            // integer/cents boxes from widely-spaced subscripts. ~keep
            let min_decimal_gap = current.font_size * 0.4;
            let max_decimal_gap = current.font_size * 1.3;
            let decimal_merge = same_line
                && same_mcid
                && decimal_sizes_match
                && gap > min_decimal_gap
                && gap < max_decimal_gap
                && !current.text.is_empty()
                && !span.text.is_empty()
                && current.text.chars().all(|c| c.is_ascii_digit())
                && span.text.chars().all(|c| c.is_ascii_digit())
                && (1..=2).contains(&span.text.len())
                && !decimal_gap_has_ink(&ink_boxes, &current.bbox, &span.bbox);

            // Snapshot the pre-merge shape for the positional `char_widths`
            // maintenance below: the merged text is `current + [separator] +
            // span`, so each contribution's widths must land at the same
            // position its chars occupy. (Captured before any branch mutates
            // `current.text` / `current.bbox`.) ~keep
            let current_chars_before = current.text.chars().count();
            let span_char_count = span.text.chars().count();

            if decimal_merge {
                tracing::trace!(target: LOG_TARGET,
                    "Decimal value merge: '{}' + '{}' -> '{}.{}' (gap={:.1}pt)",
                    current.text,
                    span.text,
                    current.text,
                    span.text,
                    gap
                );
                current.text.push('.');
                current.text.push_str(&span.text);
            } else if cross_font_word_glue {
                // Mid-word font/weight change: concatenate without any space
                // or space-heuristic — these are same-word character runs. ~keep
                current.text.push_str(&span.text);
            } else if should_merge {
                // Check if next span is entirely whitespace-only OR marked as offset_semantic space
                // If either is true, never insert an additional space - just concatenate directly
                // This prevents double-space issue when TJ processor creates space spans ~keep
                let next_is_whitespace_only = span.text.chars().all(|c| c.is_whitespace());
                let next_is_offset_semantic_space = span.offset_semantic && next_is_whitespace_only;

                if next_is_whitespace_only {
                    tracing::trace!(target: LOG_TARGET,
                        "Merging with whitespace-only span: '{}' + '{}' (whitespace, offset_semantic={})",
                        current.text,
                        span.text.escape_default(),
                        span.offset_semantic
                    );
                    current.text.push_str(&span.text);
                } else {
                    let tj_offset_triggered_override = has_split_boundary;
                    let mut space_decision = should_insert_space(
                        &current.text,
                        &span.text,
                        space_gap,
                        current.font_size,
                        &current.font_name,
                        &self.fonts,
                        tj_offset_triggered_override,
                        &self.merging_config,
                        Some(&current.bbox),
                        Some(&span.bbox),
                        current.font_size,
                        span.font_size,
                    );

                    // Narrow-word-gap rescue. The fixed intra-word
                    // kerning guard suppresses genuine word gaps on condensed/
                    // tracked lines with no space glyph (bold headings, running
                    // footers). When this line's own gap distribution is clearly
                    // bimodal and this gap sits in the inter-word cluster, honor
                    // the boundary. Only ever ADDS a space (never removes one),
                    // and ONLY when the suppression came from the purely-
                    // geometric intra-word kerning guard — never the semantic
                    // no-space rules (complex-script/Brahmic, CJK, ligature),
                    // else Bengali/Devanagari syllables shatter into fragments.
                    // RTL is excluded too — the ReversedChars guard below owns
                    // that decision.
                    // Two guards keep the narrow-gap rescue off dense math, whose
                    // sub/superscript gaps are the same ~0.10 em magnitude as a
                    // condensed footer's word gap:
                    //   * same-baseline: never rescue directly across a
                    //     super/subscript baseline shift, and
                    //   * empty-gap: never rescue when another glyph's ink sits
                    //     inside the gap — a subscript drawn between a variable
                    //     and the next symbol (`λᵢ r…`) inflates the `λ`→`r` gap
                    //     though both share the baseline; the ink in the gap marks
                    //     it as not-a-word-boundary. A genuine footer word gap is
                    //     empty. (`λᵢ` must not become `λ i`.) ~keep
                    let same_baseline =
                        (current.bbox.y - span.bbox.y).abs() < current.font_size.max(span.font_size).max(1.0) * 0.04;
                    if space_decision.source == SpaceSource::IntraWordKerning
                        && !self.saw_reversed_chars
                        && same_baseline
                        && !gap_has_intervening_glyph(&ink_boxes, &current.bbox, &span.bbox)
                    {
                        // Split only when the PER-LINE bimodal threshold fires.
                        // A uniform per-pair advance floor (the way pdfminer/
                        // poppler decide word boundaries) would catch a few more
                        // footer instances this adaptive test misses, but a fixed
                        // magnitude cannot tell a 0.10 em condensed word gap from
                        // 0.10 em loose intra-word tracking, so it also splits
                        // real words on loosely-set/scanned lines
                        // (`walking` → `wa lking`) — exactly the over-splitting
                        // pdfminer exhibits. The per-line bimodal only fires when
                        // the intra-word cluster is genuinely tight, so it never
                        // over-splits, at the cost of the handful of footer
                        // instances whose gap distribution is not cleanly bimodal. ~keep
                        if let Some(thr) = line_thresholds.get(span_idx).copied().flatten()
                            && gap > thr
                        {
                            space_decision = SpaceDecision::insert(SpaceSource::GeometricGap, 0.9);
                        }
                    }

                    // ReversedChars Arabic word-shatter guard (ISO 32000-1
                    // §14.8.2.3.3). On a page that draws RTL glyphs individually
                    // under /ReversedChars, real word boundaries are marked with
                    // explicit space glyphs (preserved above as whitespace-only
                    // spans). A GEOMETRIC space between two cursively-adjacent
                    // Arabic letters is therefore a positioning artifact, not a
                    // word break — suppress it so words stay whole (إسبريسو, not
                    // إس بر يسو). Only fires on ReversedChars pages, so ordinary
                    // geometric-spaced Arabic producers are unaffected. ~keep
                    if self.saw_reversed_chars && space_decision.insert_space {
                        use crate::text::rtl_detector::is_arabic_letter;
                        let prev_ar = current
                            .text
                            .chars()
                            .next_back()
                            .is_some_and(|c| is_arabic_letter(c as u32));
                        let next_ar = span.text.chars().next().is_some_and(|c| is_arabic_letter(c as u32));
                        if prev_ar && next_ar {
                            space_decision.insert_space = false;
                        }
                    }

                    tracing::trace!(target: LOG_TARGET,
                        "Span merge decision: gap={:.2}pt, decision={:?}, source={:?}, confidence={:.2}, offset_semantic={}",
                        gap,
                        space_decision.insert_space,
                        space_decision.source,
                        space_decision.confidence,
                        span.offset_semantic
                    );

                    if space_decision.insert_space {
                        // Space insertion triggered by unified decision
                        // But SKIP if this span is already a TJ-offset space (would create double space)
                        // ~keep
                        if next_is_offset_semantic_space {
                            tracing::trace!(target: LOG_TARGET, "Suppressing space insertion: next span is already TJ-offset space");
                            current.text.push_str(&span.text);
                        } else {
                            let would_create_double_space = current.text.ends_with(' ') && span.text.starts_with(' ');

                            if would_create_double_space {
                                tracing::trace!(target: LOG_TARGET,
                                    "Preventing double-space: current ends with space, next starts with space"
                                );
                                current.text.push_str(&span.text);
                            } else {
                                tracing::trace!(target: LOG_TARGET, "Space via {:?}", space_decision.source);
                                current.text.push(' ');
                                current.text.push_str(&span.text);
                            }
                        }
                    } else {
                        tracing::trace!(target: LOG_TARGET, "No space insertion: decision source={:?}", space_decision.source);
                        current.text.push_str(&span.text);
                    }
                }
            }

            if decimal_merge || should_merge || cross_font_word_glue {
                // A merged span is logical-draw RTL if any of its glyph runs was
                // drawn right-to-left (see `detect_rtl_draw_direction`). ~keep
                current.rtl_draw_logical |= span.rtl_draw_logical;
                let new_width = (span.bbox.x + span.bbox.width) - current.bbox.x;
                let new_height = current.bbox.height.max(span.bbox.height);

                current.bbox.width = new_width;
                current.bbox.height = new_height;

                // Keep `char_widths` in POSITIONAL lockstep with the merged
                // text. The downstream width-based splitters
                // `is_column_spanning_decimal` and `char_widths_boundary_split`
                // (document.rs) fire when `char_widths.len() < char_count`, and
                // `TextSpan::to_chars` pairs each glyph's accurate
                // `char_x_offsets` origin with `char_widths[i]` — so every
                // width entry must sit at the same index as its char, not
                // merely make the lengths match. A trailing `resize` after a
                // width-less contribution (e.g. a TJ-offset space span merging
                // FIRST) shifted every later width one slot left, pairing each
                // glyph with its neighbor's advance and opening phantom
                // intra-word gaps that the word-gap clusterer split on
                // (`module` → `m|odu|le`). Maintain the merged
                // vector as `current + [separator] + span`, normalizing each
                // contribution at its own position instead. ~keep
                let pad = if current.font_size > 0.0 {
                    current.font_size * 0.25
                } else {
                    1.0
                };
                // 1. Normalize the accumulated widths to the pre-merge char
                //    count. A width-less contribution is split uniformly
                //    across its bbox (matching `to_chars`' uniform fallback);
                //    a partially-populated one keeps the legacy tail-pad. ~keep
                if current.char_widths.is_empty() && current_chars_before > 0 {
                    let old_width = (current_end_x - current.bbox.x).max(0.0);
                    current
                        .char_widths
                        .resize(current_chars_before, old_width / current_chars_before as f32);
                } else if current.char_widths.len() != current_chars_before {
                    current.char_widths.resize(current_chars_before, pad);
                }
                // 2. Inserted separator ('.' or ' ') widths land at the
                //    separator's own position: the real geometric gap the
                //    separator stands in for, with the legacy pad as the
                //    fallback for overlapping/degenerate layouts. ~keep
                let merged_char_count = current.text.chars().count();
                let separator_count = merged_char_count.saturating_sub(current_chars_before + span_char_count);
                if separator_count > 0 {
                    let sep_gap = span.bbox.x - current_end_x;
                    let sep_width = if sep_gap.is_finite() && sep_gap > 0.0 {
                        sep_gap / separator_count as f32
                    } else {
                        pad
                    };
                    current
                        .char_widths
                        .resize(current_chars_before + separator_count, sep_width);
                }
                if span.char_widths.is_empty() && span_char_count > 0 {
                    let per_char = (span.bbox.width / span_char_count as f32).max(0.0);
                    current
                        .char_widths
                        .extend(std::iter::repeat_n(per_char, span_char_count));
                } else {
                    current.char_widths.extend_from_slice(&span.char_widths);
                    current.char_widths.resize(merged_char_count, pad);
                }
                debug_assert_eq!(
                    current.char_widths.len(),
                    merged_char_count,
                    "char_widths must stay in lockstep with merged text"
                );

                // Preserve the merged-in glyph's TRUE origin for scrambled-RTL
                // producers (e.g. /ReversedChars + per-glyph /ActualText Arabic,
                // ISO 32000-1 §14.8.2.3.3 / §14.9.4). Such producers reposition
                // glyphs out of advance-order, so the appended raw advances collapse
                // the merged span to advance-flow and `to_chars()` loses each glyph's
                // true x — the RTL visual-order sort (`merge_rtl_line_to_visual_span`)
                // then mis-places zero-width marks (القهوة → قالهوة). After the
                // char_widths are in lockstep with the (possibly space-inserted) text,
                // stretch the advance leading into the merged-in span's LAST glyph so
                // `to_chars()` reconstructs it at `span.bbox.x`. Gated to Arabic so
                // Latin/CJK output stays byte-identical. ~keep
                let touches_arabic = |t: &str| {
                    t.chars().any(|c| {
                        ('\u{0600}'..='\u{06FF}').contains(&c)
                            || ('\u{0750}'..='\u{077F}').contains(&c)
                            || ('\u{08A0}'..='\u{08FF}').contains(&c)
                    })
                };
                let n = current.char_widths.len();
                let span_chars = span.text.chars().count();
                if n >= 2
                    && span_chars >= 1
                    && span_chars < n
                    && (touches_arabic(&current.text) || touches_arabic(&span.text))
                {
                    // Index of the merged-in span's FIRST glyph in the (possibly
                    // space-inserted) merged text, and its target relative x. ~keep
                    let first_idx = n - span_chars;
                    let prefix: f32 = current.char_widths[..first_idx].iter().sum();
                    let want = span.bbox.x - current.bbox.x;
                    let adjust = want - prefix;
                    if adjust.abs() > 0.01 {
                        current.char_widths[first_idx - 1] += adjust;
                    }
                }

                // After a cross-font glue, adopt the longer run's font
                // metadata. The single-letter side was typographic
                // decoration, not semantic emphasis, so the dominant-run
                // style should win. ~keep
                if cross_font_word_glue {
                    let span_chars = span.text.chars().count();
                    let current_chars_before = current.text.chars().count() - span_chars;
                    if span_chars > current_chars_before {
                        current.font_name = span.font_name.clone();
                        current.font_weight = span.font_weight;
                        current.is_italic = span.is_italic;
                    }
                }

                tracing::trace!(target: LOG_TARGET,
                    "Merged span: appended '{}' (gap={:.1}pt, now {} chars)",
                    span.text,
                    gap,
                    current.text.len()
                );

                current_span = Some(current);
            } else {
                if same_line {
                    if span.split_boundary_before {
                        tracing::trace!(target: LOG_TARGET,
                            "Not merging spans (split boundary): '{}' | '{}'",
                            current.text,
                            span.text
                        );
                    } else {
                        tracing::trace!(target: LOG_TARGET,
                            "Not merging spans (gap={:.1}pt > 3pt): '{}' | '{}'",
                            gap,
                            current.text,
                            span.text
                        );
                    }
                }
                merged.push(current);
                current_span = Some(span);
            }
        }

        if let Some(last) = current_span {
            merged.push(last);
        }

        tracing::debug!(target: LOG_TARGET, "Merged adjacent spans: {} -> {} spans", old_len, merged.len());

        self.spans = merged;
    }

    /// Sort extracted characters by reading order (top-to-bottom, left-to-right).
    ///
    /// This is critical for proper text extraction as PDF content streams are
    /// organized for rendering efficiency, not reading order.
    pub(super) fn sort_by_reading_order(&mut self) {
        self.chars.sort_by(|a, b| {
            if !a.bbox.y.is_finite() {
                return if b.bbox.y.is_finite() {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Equal
                };
            }
            if !b.bbox.y.is_finite() {
                return std::cmp::Ordering::Less;
            }

            // Sort by Y descending (top first), then by X ascending (left to right)
            // Round Y coordinates to ensure transitivity of the comparison function ~keep
            let a_y_rounded = a.bbox.y.round() as i32;
            let b_y_rounded = b.bbox.y.round() as i32;

            match b_y_rounded.cmp(&a_y_rounded) {
                std::cmp::Ordering::Equal => {
                    if !a.bbox.x.is_finite() {
                        return if b.bbox.x.is_finite() {
                            std::cmp::Ordering::Greater
                        } else {
                            std::cmp::Ordering::Equal
                        };
                    }
                    if !b.bbox.x.is_finite() {
                        return std::cmp::Ordering::Less;
                    }

                    if a.bbox.x < b.bbox.x {
                        std::cmp::Ordering::Less
                    } else if a.bbox.x > b.bbox.x {
                        std::cmp::Ordering::Greater
                    } else {
                        std::cmp::Ordering::Equal
                    }
                }
                other => other,
            }
        });
    }
}
