//! Span spacing, merging, and bidi/CJK text normalization.
//!
//! Split out of the parent's single 18,544-line `impl PdfDocument`, which made
//! `document.rs` 1.2 MiB and tripped the 500 KiB file-safety limit. A child module's
//! `impl` is the same inherent impl and sees the parent's private items unchanged. ~keep

use super::*;

impl PdfDocument {
    /// Normalize Kangxi Radical characters to CJK Unified Ideographs.
    ///
    /// Some PDF fonts/CMaps emit Kangxi Radicals (U+2F00–U+2FD5) or CJK Radicals
    /// Supplement (U+2E80–U+2EFF) instead of the standard CJK Unified Ideographs.
    /// While visually similar, these are different Unicode codepoints and will break
    /// text search, string matching, and NLP pipelines.
    pub(super) fn normalize_kangxi_radicals(text: &str) -> String {
        if !text.chars().any(|c| {
            let cp = c as u32;
            (0x2E80..=0x2EFF).contains(&cp) || (0x2F00..=0x2FD5).contains(&cp)
        }) {
            return text.to_string();
        }

        text.chars()
            .map(|c| crate::text::kangxi::kangxi_to_unified(c).unwrap_or(c))
            .collect()
    }

    /// Reverse visual-order RTL character runs to logical reading order.
    ///
    /// Some PDFs position Arabic/Hebrew characters individually left-to-right
    /// (visual order). For correct text extraction, runs of single-character
    /// RTL spans on the same line are collected, reversed, and merged into
    /// a single span to produce correct logical reading order.
    pub(super) fn reverse_rtl_visual_order_runs(spans: &mut Vec<TextSpan>) {
        use crate::text::rtl_detector::is_rtl_text;

        // Pass 0: reverse visual-order characters inside a single span
        // when the producer clearly emitted pre-shaped Arabic.
        //
        // Some PDFs (e.g. `ArabicCIDTrueType.pdf` in the pdfjs regression
        // corpus) emit Arabic with an entire line as a single Tj-produced
        // span whose `text` is stored in *visual* order — rightmost
        // rendered glyph first. That matches what the content stream
        // literally drew on the page, but downstream consumers expect
        // reading-order (logical) text.
        //
        // The gate for reversal is the presence of **Arabic Presentation
        // Forms A or B** (U+FB50-U+FDFF, U+FE70-U+FEFF). Those code points
        // only appear when the PDF producer has explicitly pre-shaped the
        // glyphs, and producers that pre-shape almost universally also
        // store them in visual order because that's the order the content
        // stream draws them. Plain base-Arabic text (U+0600-U+06FF) is
        // left alone because those files are usually already in logical
        // order — the PDF viewer applies shaping and bidi reordering at
        // render time, so reversing would produce a wrong result.
        //
        // We still require at least 4 characters and >50 % non-whitespace
        // RTL ratio so that punctuation or stray markers adjacent to
        // Arabic do not trigger a reversal.
        //
        // Pass 1 below handles the other common shape where each Arabic
        // character is emitted as its own short span and the reversal is
        // a span-granularity concern. The two passes are independent:
        // a span either fires Pass 0 (pre-shaped, reverse in place) or
        // Pass 1 (per-glyph spans, reverse span order), never both.
        //
        // This is separate from `normalize_arabic_presentation_forms`,
        // which runs later on the assembled output string and unshapes
        // contextual glyphs back to their base Unicode letters. ~keep
        for span in spans.iter_mut() {
            let mut total = 0usize;
            let mut rtl_count = 0usize;
            let mut has_presentation_form = false;
            for c in span.text.chars() {
                if c.is_whitespace() {
                    continue;
                }
                total += 1;
                let cp = c as u32;
                if is_rtl_text(cp) {
                    rtl_count += 1;
                }
                if (0xFB50..=0xFDFF).contains(&cp) || (0xFE70..=0xFEFF).contains(&cp) {
                    has_presentation_form = true;
                }
            }
            // Pass 0 only applies to a *whole-line* visual-order span —
            // one span holding several words separated by internal whitespace,
            // in the order the content stream drew them (rightmost first). When
            // the extractor instead emits one span PER WORD (the common
            // CID-TrueType case, e.g. ArabicCIDTrueType.pdf), each word's
            // characters are already in logical order, so char-reversing them
            // here corrupts them. Their right-to-left *word* order is fixed
            // separately by the span-run reversal pass below. Gate on internal
            // whitespace so per-word logical spans are left untouched. ~keep
            let has_internal_whitespace = span.text.trim().chars().any(|c| c.is_whitespace());
            if has_presentation_form && has_internal_whitespace && total >= 4 && rtl_count * 2 > total {
                let reversed: String = span.text.chars().rev().collect();
                span.text = reversed;
            }
        }

        // Pass 0.5: per-word RTL span ORDER. The row-aware sort placed
        // spans left-to-right (x ascending), but a right-to-left script reads
        // the words in the opposite direction. For each maximal run of
        // consecutive same-line spans that is purely RTL (every non-space span
        // holds RTL letters and no Latin letters), reverse the run's order so
        // the words come out in logical reading order. Each word's characters
        // are left as-is (they are already logical — see Pass 0's gate). ~keep
        let is_space = |s: &TextSpan| s.text.trim().is_empty();
        let is_rtl_word = |s: &TextSpan| {
            let mut has_rtl = false;
            for c in s.text.chars() {
                if c.is_ascii_alphabetic() {
                    return false;
                }
                if is_rtl_text(c as u32) {
                    has_rtl = true;
                }
            }
            has_rtl
        };
        let mut i = 0;
        while i < spans.len() {
            if !is_rtl_word(&spans[i]) {
                i += 1;
                continue;
            }
            let y = spans[i].bbox.y;
            let start = i;
            let mut end = i + 1;
            while end < spans.len()
                && (spans[end].bbox.y - y).abs() < 2.0
                && (is_rtl_word(&spans[end]) || is_space(&spans[end]))
            {
                end += 1;
            }
            // Trim trailing space spans so separators stay between words. ~keep
            let mut last = end;
            while last > start + 1 && is_space(&spans[last - 1]) {
                last -= 1;
            }
            if last - start >= 2 {
                spans[start..last].reverse();
            }
            i = end;
        }

        if spans.len() < 4 {
            return;
        }

        // Iterate forward; drain consumed runs so subsequent indices stay valid ~keep
        let mut i = 0;
        while i < spans.len() {
            let is_short_rtl =
                spans[i].text.chars().count() <= 2 && spans[i].text.chars().any(|c| is_rtl_text(c as u32));

            if !is_short_rtl {
                i += 1;
                continue;
            }

            let run_start = i;
            let y = spans[i].bbox.y;
            let mut j = i + 1;
            while j < spans.len() {
                let y_same = (spans[j].bbox.y - y).abs() < 2.0;
                let is_short = spans[j].text.chars().count() <= 2;
                let has_rtl_or_space = spans[j].text.chars().all(|c| is_rtl_text(c as u32) || c == ' ');
                if y_same && is_short && has_rtl_or_space {
                    j += 1;
                } else {
                    break;
                }
            }
            let run_end = j;
            let run_len = run_end - run_start;

            // Only process runs of 4+ spans (avoid false positives) ~keep
            if run_len >= 4 {
                let mut reversed_text = String::new();
                for span in spans[run_start..run_end].iter().rev() {
                    reversed_text.push_str(&span.text);
                }

                let last_span = &spans[run_end - 1];
                let new_width = (last_span.bbox.x + last_span.bbox.width) - spans[run_start].bbox.x;
                spans[run_start].text = reversed_text;
                spans[run_start].bbox.width = new_width;

                spans.drain(run_start + 1..run_end);

                i = run_start + 1;
            } else {
                i = run_end;
            }
        }
    }

    /// Normalize Arabic Presentation Forms to base Unicode characters.
    ///
    /// Arabic PDFs often use presentation forms (U+FE70-U+FEFF for Forms-B,
    /// U+FB50-U+FDFF for Forms-A) which represent contextual glyph shapes.
    /// For text extraction, these should be normalized to base characters.
    pub(super) fn normalize_arabic_presentation_forms(text: &str) -> String {
        if !text.chars().any(|c| {
            let cp = c as u32;
            (0xFB50..=0xFDFF).contains(&cp) || (0xFE70..=0xFEFF).contains(&cp)
        }) {
            return text.to_string();
        }

        text.chars()
            .map(|c| {
                let cp = c as u32;
                // Arabic Presentation Forms-B (U+FE70-U+FEFF): contextual forms
                // Each base letter has isolated/final/initial/medial forms ~keep
                let base = match cp {
                    0xFE80 => 0x0621,
                    0xFE81 | 0xFE82 => 0x0622,
                    0xFE83 | 0xFE84 => 0x0623,
                    0xFE85 | 0xFE86 => 0x0624,
                    0xFE87 | 0xFE88 => 0x0625,
                    0xFE89..=0xFE8C => 0x0626,
                    0xFE8D | 0xFE8E => 0x0627,
                    0xFE8F..=0xFE92 => 0x0628,
                    0xFE93 | 0xFE94 => 0x0629,
                    0xFE95..=0xFE98 => 0x062A,
                    0xFE99..=0xFE9C => 0x062B,
                    0xFE9D..=0xFEA0 => 0x062C,
                    0xFEA1..=0xFEA4 => 0x062D,
                    0xFEA5..=0xFEA8 => 0x062E,
                    0xFEA9 | 0xFEAA => 0x062F,
                    0xFEAB | 0xFEAC => 0x0630,
                    0xFEAD | 0xFEAE => 0x0631,
                    0xFEAF | 0xFEB0 => 0x0632,
                    0xFEB1..=0xFEB4 => 0x0633,
                    0xFEB5..=0xFEB8 => 0x0634,
                    0xFEB9..=0xFEBC => 0x0635,
                    0xFEBD..=0xFEC0 => 0x0636,
                    0xFEC1..=0xFEC4 => 0x0637,
                    0xFEC5..=0xFEC8 => 0x0638,
                    0xFEC9..=0xFECC => 0x0639,
                    0xFECD..=0xFED0 => 0x063A,
                    0xFED1..=0xFED4 => 0x0641,
                    0xFED5..=0xFED8 => 0x0642,
                    0xFED9..=0xFEDC => 0x0643,
                    0xFEDD..=0xFEE0 => 0x0644,
                    0xFEE1..=0xFEE4 => 0x0645,
                    0xFEE5..=0xFEE8 => 0x0646,
                    0xFEE9..=0xFEEC => 0x0647,
                    0xFEED | 0xFEEE => 0x0648,
                    0xFEEF | 0xFEF0 => 0x0649,
                    0xFEF1..=0xFEF4 => 0x064A,
                    // Lam-Alef ligatures → expand to two characters ~keep
                    0xFEF5 | 0xFEF6 => {
                        return '\u{0644}'; // Just return Lam; Alef is separate ~keep
                    }
                    0xFEF7 | 0xFEF8 => {
                        return '\u{0644}';
                    }
                    0xFEF9 | 0xFEFA => {
                        return '\u{0644}';
                    }
                    0xFEFB | 0xFEFC => {
                        return '\u{0644}';
                    }
                    0xFE70 => 0x064B,
                    0xFE71 => 0x064B,
                    0xFE72 => 0x064C,
                    0xFE74 => 0x064D,
                    0xFE76 => 0x064E,
                    0xFE77 => 0x064E,
                    0xFE78 => 0x064F,
                    0xFE79 => 0x064F,
                    0xFE7A => 0x0650,
                    0xFE7B => 0x0650,
                    0xFE7C => 0x0651,
                    0xFE7D => 0x0651,
                    0xFE7E => 0x0652,
                    0xFE7F => 0x0652,
                    _ => cp,
                };
                char::from_u32(base).unwrap_or(c)
            })
            .collect()
    }

    /// Returns the Y tolerance (in points) for treating two spans as
    /// belonging to the same visual line during text assembly.
    ///
    /// The threshold scales with the larger font size so mixed-size runs
    /// (for example superscripts and subscripts) are not split by a fixed
    /// absolute tolerance.
    pub(super) fn same_line_threshold(prev: &TextSpan, current: &TextSpan) -> f32 {
        let max_fs = prev.font_size.max(current.font_size).max(1.0);
        let min_fs = prev.font_size.min(current.font_size).max(1.0);
        // Continuous formula — avoids the step discontinuity at the 4×
        // ratio boundary. Examples:
        //   same-size 12 pt body: max(12×1.2, 12×0.3) = 14.4 pt ← 1.2× leading
        //   heading+body 24+10 pt: max(10×1.2, 24×0.3) = 12.0 pt ← keeps para break
        //   superscript 12+6 pt: max(6×1.2, 12×0.3) = 7.2 pt ← same line
        // Prior formula was max_fs×0.5 for normal ratios; new formula uses 1.2× of the
        // smaller font, which is wider and reduces false newlines for normal leading.
        // Formula: max(min_fs * 1.2, max_fs * 0.3) ~keep
        (min_fs * 1.2).max(max_fs * 0.3)
    }

    /// True when a line break falls *inside* a Hangul word (eojeol) that wrapped
    /// mid-syllable — Korean breaks anywhere, not only at word boundaries, so a
    /// mid-eojeol wrap carries no separator in the source and the two halves
    /// must rejoin with nothing ("집고양" ⏎ "이의" → "집고양이의"). An
    /// eojeol-BOUNDARY wrap keeps its explicit inter-eojeol space, so `text`
    /// ends with ' ' and this returns false (the break still separates).
    /// Scoped to Hangul (not Chinese/Japanese) to avoid the CJK
    /// line-break-collapse regressions seen previously.
    pub(super) fn hangul_midword_line_wrap(text: &str, prev: &TextSpan, span: &TextSpan) -> bool {
        let is_hangul = |c: char| (0xAC00..=0xD7AF).contains(&(c as u32));
        !text.ends_with(' ')
            && prev.text.chars().next_back().is_some_and(is_hangul)
            && span.text.chars().next().is_some_and(is_hangul)
    }

    /// Returns `true` if `inner` is contained within `outer`,
    /// allowing `eps` points of floating-point slack on all four
    /// edges. Used at the table-retain sites to absorb ~0.02pt drift
    /// in span right-edges relative to table bboxes computed from
    /// min/max reductions over many cell edges.
    pub(super) fn contains_rect_with_tolerance(
        outer: &crate::geometry::Rect,
        inner: &crate::geometry::Rect,
        eps: f32,
    ) -> bool {
        inner.left() >= outer.left() - eps
            && inner.right() <= outer.right() + eps
            && inner.top() >= outer.top() - eps
            && inner.bottom() <= outer.bottom() + eps
    }

    /// Returns `true` if a tentative left-to-right X-ordering of `run`
    /// contains a horizontal gap exceeding
    /// `SAME_LINE_REORDER_MAX_GAP_FACTOR * max(font_size)` between any
    /// two consecutive spans. Used by [`reorder_same_line_runs`] to
    /// reject candidate runs that are vertically close but horizontally
    /// disjoint (e.g. tightly-set footer/header rows split across the
    /// page).
    ///
    /// The slice is not mutated; the X-order is computed on a local
    /// copy of `(left_x, right_x, font_size)` triples.
    fn run_has_large_x_gap(run: &[TextSpan]) -> bool {
        if run.len() < 2 {
            return false;
        }

        let mut edges: Vec<(f32, f32, f32)> = run
            .iter()
            .map(|s| (s.bbox.x, s.bbox.x + s.bbox.width, s.font_size))
            .collect();

        edges.sort_by(|a, b| crate::utils::safe_float_cmp(a.0, b.0));

        for pair in edges.windows(2) {
            let prev = pair[0];
            let cur = pair[1];

            let gap = cur.0 - prev.1;
            if gap <= 0.0 {
                continue;
            }

            let max_fs = prev.2.max(cur.2).max(1.0);
            if gap > SAME_LINE_REORDER_MAX_GAP_FACTOR * max_fs {
                return true;
            }
        }

        false
    }

    /// True when a candidate run contains spans whose X-extents OVERLAP — the
    /// signature of two (or more) distinct text lines that the same-line Y
    /// tolerance merged into one band, NOT a single line. A real line lays its
    /// spans out left-to-right with non-overlapping advances; only stacked lines
    /// (leading just under `same_line_threshold`, e.g. a two-line title or a
    /// running head sitting above the line below it) put two spans at the same
    /// horizontal position. X-sorting such a band interleaves the two lines word
    /// by word, so the caller must leave it in row order instead. Mirrors
    /// [`run_has_large_x_gap`] for the opposite defect.
    fn run_has_x_overlap(run: &[TextSpan]) -> bool {
        if run.len() < 2 {
            return false;
        }

        let mut edges: Vec<(f32, f32, f32)> = run
            .iter()
            .map(|s| (s.bbox.x, s.bbox.x + s.bbox.width, s.font_size))
            .collect();

        edges.sort_by(|a, b| crate::utils::safe_float_cmp(a.0, b.0));

        for pair in edges.windows(2) {
            let prev = pair[0];
            let cur = pair[1];

            // prev.right - cur.left > 0 ⇒ the next span starts before the previous
            // one ends (horizontal overlap). Half an em of overlap is well beyond
            // kerning/italic side-bearing and only happens across stacked lines. ~keep
            let overlap = prev.1 - cur.0;
            let max_fs = prev.2.max(cur.2).max(1.0);
            if overlap > 0.5 * max_fs {
                return true;
            }
        }

        false
    }

    /// True when a run is structurally two-or-more stacked text LINES: it has at
    /// least two distinct Y levels that EACH carry at least two spans. This
    /// separates a real two-line title / running-head block (many words on each
    /// of two baselines) — where de-interleaving is correct — from a single span
    /// that merely overlaps a line in X (a drop cap, a `©`/`c` mark, a lone
    /// super-script), where the existing X-sort already does the right thing and
    /// reordering by Y would misplace the stray glyph.
    fn run_is_stacked_lines(run: &[TextSpan]) -> bool {
        if run.len() < 4 {
            return false; // need ≥2 lines × ≥2 spans ~keep
        }
        let mut rows: Vec<(f32, f32)> = run.iter().map(|s| (s.bbox.y, s.font_size)).collect();
        rows.sort_by(|a, b| crate::utils::safe_float_cmp(b.0, a.0));

        let mut multi_rows = 0usize;
        let mut anchor_y = f32::NAN;
        let mut count = 0usize;
        for (y, fs) in rows {
            if anchor_y.is_nan() || (anchor_y - y).abs() <= 0.5 * fs.max(1.0) {
                if anchor_y.is_nan() {
                    anchor_y = y;
                }
                count += 1;
            } else {
                if count >= 2 {
                    multi_rows += 1;
                }
                anchor_y = y;
                count = 1;
            }
        }
        if count >= 2 {
            multi_rows += 1;
        }
        multi_rows >= 2
    }

    /// Re-sort same-line spans by X after row-aware band sorting.
    ///
    /// Row-aware sorting can place off-baseline glyphs such as superscripts or
    /// subscripts in adjacent Y bands before their base glyphs. This helper finds
    /// candidate runs with the existing same-line threshold, then tentatively views
    /// each candidate in X order. If that tentative X order contains a large gap,
    /// the candidate is treated as disjoint footer/header/field content and is
    /// left in the existing row-aware order.
    ///
    /// At the slice level no spans are merged or dropped; successful candidates are
    /// only permuted. Downstream text assembly may then emit the reordered spans
    /// into one visual line, which is the user-observable effect.
    pub(super) fn reorder_same_line_runs(spans: &mut [TextSpan]) {
        let mut i = 0;

        while i < spans.len() {
            let mut j = i + 1;

            while j < spans.len() {
                let anchor = &spans[i];
                let prev = &spans[j - 1];
                let cur = &spans[j];

                let to_prev = (cur.bbox.y - prev.bbox.y).abs();
                let to_anchor = (cur.bbox.y - anchor.bbox.y).abs();

                let tol_prev = Self::same_line_threshold(prev, cur);
                let tol_anchor = Self::same_line_threshold(anchor, cur);

                if to_prev > tol_prev || to_anchor > tol_anchor {
                    break;
                }

                j += 1;
            }

            if j - i > 1 {
                if Self::run_has_large_x_gap(&spans[i..j]) {
                    // Candidate spans are vertically close but not horizontally
                    // contiguous (disjoint header/footer columns). Do not X-sort
                    // them into a fake line; preserve the row-aware order. ~keep
                    i = j;
                    continue;
                }

                if Self::run_has_x_overlap(&spans[i..j]) && Self::run_is_stacked_lines(&spans[i..j]) {
                    // Spans OVERLAP horizontally AND form ≥2 lines of ≥2 spans each:
                    // two stacked lines the Y tolerance merged into one band (a
                    // two-line title, a running head above the line below it). A
                    // flat X-sort interleaves them word by word. De-interleave by
                    // ordering on (Y-descending, then X) so each real line stays
                    // contiguous and in reading order. The stacked-lines gate keeps
                    // a lone overlapping glyph (drop cap, `©`, super-script) on the
                    // normal X-sort path below. ~keep
                    spans[i..j].sort_by(|a, b| {
                        crate::utils::safe_float_cmp(b.bbox.y, a.bbox.y)
                            .then(crate::utils::safe_float_cmp(a.bbox.x, b.bbox.x))
                            .then(a.sequence.cmp(&b.sequence))
                    });
                    i = j;
                    continue;
                }

                spans[i..j].sort_by(|a, b| {
                    let cmp = crate::utils::safe_float_cmp(a.bbox.x, b.bbox.x);
                    if cmp != std::cmp::Ordering::Equal {
                        return cmp;
                    }
                    a.sequence.cmp(&b.sequence)
                });
            }

            i = j;
        }
    }

    /// Distinguish a genuine tight-kerning overlap of a single word drawn as
    /// two same-font runs ("PLANAL"+"TINA") from an inflated-width artifact.
    ///
    /// A font with no `/Widths` array falls back to a uniform 550/1000-em
    /// advance for every glyph, which over-reports each glyph's width and drags
    /// the previous span's right edge past where the next span really starts —
    /// a fake overlap that the assembler must break with a space (the NASA
    /// "STATION"+"FREEDOM" header case). A genuine kerning overlap, by
    /// contrast, has real per-glyph metrics that VARY across the run, a modest
    /// overlap (well under one em), the same font on both sides, and word
    /// characters at the join. When those hold the two runs are one word and no
    /// space must be synthesized. This works purely on the assembled text — the
    /// spans are left unmerged, so page layout and table detection are
    /// unaffected (a span merge here would shift XY-cut/table statistics).
    pub(super) fn is_reliable_kerning_overlap(prev: &TextSpan, span: &TextSpan, gap: f32) -> bool {
        let fs = prev.font_size.max(span.font_size).max(1.0);
        let prev_last = prev.text.chars().next_back();
        let next_first = span.text.chars().next();
        gap < 0.0
            && gap > -fs
            && prev.font_name == span.font_name
            && prev.font_weight == span.font_weight
            && prev.is_italic == span.is_italic
            && prev_last.is_some_and(|c| c.is_alphanumeric())
            && next_first.is_some_and(|c| c.is_alphanumeric())
            // A lowercase→uppercase transition at the join is a word/sentence
            // boundary ("...with"+"Gp53", "Alg"+"The"), never the middle of a
            // single word split by kerning — real intra-word splits continue in
            // the same case tier ("PLANAL"+"TINA", "eigenv"+"alue"). Excluding
            // it keeps the two overlapping runs as separate words with a space. ~keep
            && !(prev_last.is_some_and(|c| c.is_lowercase())
                && next_first.is_some_and(|c| c.is_uppercase()))
            && {
                // Real proportional font metrics take many distinct per-glyph
                // advances; a missing-/Widths fallback emits ONE uniform
                // advance, and coarse/artifact width tables only a couple.
                // Require at least THREE distinct advances so a genuine
                // proportional run ("PLANAL": 6.67/5.56/7.22) is accepted while
                // a 1- or 2-value fallback table (which manufactures fake
                // overlaps between separate words) is not. ~keep
                let mut distinct: [i32; 3] = [i32::MIN, i32::MIN, i32::MIN];
                let mut n = 0usize;
                for w in prev.char_widths.iter().map(|w| (w * 100.0).round() as i32) {
                    if !distinct[..n].contains(&w) {
                        if n < 3 {
                            distinct[n] = w;
                        }
                        n += 1;
                        if n >= 3 {
                            break;
                        }
                    }
                }
                n >= 3
            }
    }

    /// Keep a small, separate numeric script from changing the value of a
    /// preceding number in plain text (`comma 3` + footnote `5` is not `35`).
    /// A text-rise operator can mark the script even without a smaller font.
    fn needs_numeric_script_boundary(base: &TextSpan, script: &TextSpan) -> bool {
        if !base.text.chars().next_back().is_some_and(|c| c.is_ascii_digit())
            || script.text.is_empty()
            || script.text.chars().count() > 3
            || !script.text.chars().all(|c| c.is_ascii_digit())
            || (script.font_size >= base.font_size * 0.8 && script.text_rise.abs() < 0.10)
        {
            return false;
        }

        let gap = script.bbox.x - (base.bbox.x + base.bbox.width);
        let y_diff = (base.bbox.y - script.bbox.y).abs();
        gap >= -0.1 * base.font_size && gap <= 0.25 * base.font_size && y_diff <= 0.75 * base.font_size
    }

    /// # Returns
    /// `true` if a space should be inserted between the spans
    pub(super) fn should_insert_space(prev: &TextSpan, current: &TextSpan) -> bool {
        let font_size = prev.font_size.max(current.font_size).max(1.0);

        // Same-line gate. Uses the shared threshold so the assembly
        // loop's same-line decision and the space-insertion decision
        // cannot disagree about where a line ends. ~keep
        let y_diff = (prev.bbox.y - current.bbox.y).abs();
        if y_diff > Self::same_line_threshold(prev, current) {
            return false;
        }

        // CJK scripts (Chinese, Japanese, Korean) do not use spaces between
        // words. If both the tail of prev and the head of current are CJK characters,
        // inserting a space would produce incorrect tokenisation. ~keep
        let prev_tail = prev.text.chars().next_back();
        let curr_head = current.text.chars().next();
        let is_cjk = |c: char| {
            matches!(
                c as u32,
                0x3040..=0x309F
                | 0x30A0..=0x30FF
                | 0x3400..=0x4DBF
                | 0x4E00..=0x9FFF
                | 0xAC00..=0xD7AF
                | 0x20000..=0x2A6DF
                | 0xFF00..=0xFFEF
                | 0x3000..=0x303F
            )
        };
        if prev_tail.is_some_and(is_cjk) && curr_head.is_some_and(is_cjk) {
            return false;
        }

        // Complex Brahmic / South-East-Asian scripts (Devanagari, Bengali,
        // Tamil, Telugu, …, Thai, Khmer): an inter-glyph gap *inside* a word is
        // not a word break. These scripts render dependent vowel signs
        // (matras), conjuncts, and reordered glyphs with their own positional
        // advances, so the Latin-tuned proportional-gap test below fires inside
        // a syllable cluster (e.g. a Bengali consonant following a wide matra
        // sits ~0.7em from it). Word boundaries in conforming text are carried
        // by an explicit SPACE glyph — ISO 32000-1 §14.8.2.5 requires the
        // spacing characters that separate words to be present — so a heuristic
        // space here only double-counts a boundary the explicit space already
        // marks. Suppress it when both sides are the *same* complex script;
        // this mirrors the CJK guard above (CJK uses no inter-word space at
        // all, these scripts carry it explicitly). ~keep
        {
            use crate::text::complex_script_detector::detect_complex_script;
            let prev_script = prev_tail.and_then(|c| detect_complex_script(c as u32));
            let curr_script = curr_head.and_then(|c| detect_complex_script(c as u32));
            if let (Some(p), Some(c)) = (prev_script, curr_script)
                && p == c
            {
                return false;
            }
        }

        // Emoji / pictographic → letter boundary: a wide pictographic glyph
        // (e.g. 📄) abuts the next token, so the proportional-gap test below
        // would drop the inter-token space (`📄README` instead of `📄 README`).
        // Word boundaries are reader latitude (ISO 32000-1:2008 §9.10); keep the
        // space. The alphabetic-follower requirement excludes combined ZWJ/VS
        // emoji sequences (whose next char is a selector or another pictograph). ~keep
        if prev_tail.is_some_and(crate::extractors::text::is_pictographic) && curr_head.is_some_and(char::is_alphabetic)
        {
            return true;
        }

        let prev_end_x = prev.bbox.x + prev.bbox.width;
        let gap = current.bbox.x - prev_end_x;

        if Self::needs_numeric_script_boundary(prev, current) {
            return true;
        }

        // CJK script ↔ non-CJK boundary: pdftotext (and the GT it produces)
        // inserts a space wherever a CJK *script* glyph (ideograph, kana, or
        // hangul) meets a Latin/digit character on the same line, regardless
        // of how tightly the two were typeset. Without this, mixed-script
        // content like "神鹰集团" + "2015" collapses into one token
        // "神鹰集团2015", which never matches GT's separate "神鹰集团"
        // "2015" tokens (issue 484, pr-136).
        //
        // IMPORTANT: this MUST exclude fullwidth ASCII variants (U+FF01..FF5E
        // — ＜＞＝＠ etc.) and CJK Symbols and Punctuation (U+3000..303F) even
        // though they are technically "CJK characters". Those are *operator*
        // glyphs that sit inline with adjacent digits and Latin in CJK
        // technical documents — pdftotext keeps "60000≤Q＜80000"
        // "20＜μ≤30" as compound tokens (issue 484, issue-336). Forcing a
        // boundary space there destroys the compound and regresses Jaccard. ~keep
        let is_cjk_script = |c: char| {
            matches!(
                c as u32,
                0x3040..=0x309F
                | 0x30A0..=0x30FF
                | 0x3400..=0x4DBF
                | 0x4E00..=0x9FFF
                | 0xAC00..=0xD7AF
                | 0x20000..=0x2A6DF
                | 0xFF66..=0xFF9F
            )
        };
        let crosses_cjk_boundary = match (prev_tail, curr_head) {
            (Some(p), Some(c)) => is_cjk_script(p) != is_cjk_script(c),
            _ => false,
        };
        // ASCII punctuation hugs the preceding token in every script —
        // pdftotext's GT renders "する." with no space and "神鹰，2015"
        // with no space before the comma either. Suppress the boundary
        // forced-space when the transitioning glyph IS the punctuation;
        // the space-threshold path below still handles real gaps. ~keep
        let is_clause_punct = |c: char| {
            matches!(
                c,
                '.' | ',' | ';' | ':' | '!' | '?' | ')' | ']' | '}'
                // Indic danda / double danda (sentence terminators that hug the
                // preceding token like a Latin full stop). The danda lives in the
                // Devanagari block but is shared by Bengali, Gurmukhi, etc., so a
                // Bengali sentence + danda would otherwise read as a script
                // transition and take a geometric space ("প্রাণী ।"). ~keep
                | '\u{0964}' | '\u{0965}'
                // Arabic comma / semicolon / question mark (RTL clause punctuation). ~keep
                | '\u{060C}' | '\u{061B}' | '\u{061F}'
            )
        };
        let punct_at_boundary =
            curr_head.is_some_and(is_clause_punct) || prev_tail.is_some_and(|c| matches!(c, '(' | '[' | '{'));
        // Hangul↔digit is NOT a word boundary: a Korean numeral hugs its
        // Sino-Korean counter ("1만년" = 10,000 years, "약 1만년"). Unlike a
        // Chinese ideograph meeting a Latin year ("神鹰集团" + "2015", which
        // pdftotext splits, issue 484), Korean keeps the digit and counter as
        // one token, so forcing a space here over-segments the eojeol. ~keep
        let is_hangul = |c: char| (0xAC00..=0xD7AF).contains(&(c as u32));
        let hangul_digit_boundary = match (prev_tail, curr_head) {
            (Some(p), Some(c)) => (is_hangul(p) && c.is_ascii_digit()) || (p.is_ascii_digit() && is_hangul(c)),
            _ => false,
        };
        if crosses_cjk_boundary && !punct_at_boundary && !hangul_digit_boundary && gap > -0.5 && gap < font_size * 5.0 {
            return true;
        }

        // Space threshold: 0.15 × font size
        // Typical space width is ~0.25em, so 0.15em catches gaps > 60% of a space.
        // This aligns with the text extractor's font-aware threshold (~50% of space width). ~keep
        // Only a ligature drawn as its own run is relaxed; a word-final ligature inside a
        // longer run sits next to a real word gap that is nearly as narrow. A run is a ligature
        // by its code point or by the mark the decoder set when it expanded the glyph to letters,
        // never by its letters: real words contain `ff` and `fi`. ~keep
        let is_bare_ligature =
            |span: &TextSpan| span.ligature_glyph || crate::extractors::text::starts_with_agl_ligature(&span.text);
        let bare_ligature_run = is_bare_ligature(prev) || is_bare_ligature(current);
        let space_threshold = if bare_ligature_run {
            font_size * 0.15 * crate::extractors::text::LIGATURE_BOUNDARY_GAP_FACTOR
        } else {
            font_size * 0.15
        };

        // Insert space if gap is significant. Previously the upper bound was
        // `gap < font_size * 5.0` on the rationale that very large gaps mean
        // "column boundary, no space needed" — but downstream the caller
        // concatenates the two spans together when this returns false, so
        // "column boundary" actually rendered as `3.80%4.41%` on wide rate
        // tables (issue 487 pr-138-example.pdf). Drop the upper bound so any
        // gap above the inter-glyph threshold gets at least a single space.
        //
        // Clause punctuation hugs the preceding word in Brahmic scripts. The
        // producer leaves a wide advance after a Bengali/Devanagari syllable
        // (matra/akhand positioning), so the geometric test would float a danda
        // ("প্রাণী ।") or a comma ("रोशनी ,") off as its own token. Scope this to
        // a *complex-script* previous glyph (or an Indic danda) so the universal
        // Latin/math/form paths — where the same suppression interacts badly with
        // the forward-gap line-break heuristic — stay byte-for-byte unchanged. ~keep
        {
            use crate::text::complex_script_detector::detect_complex_script;
            let prev_is_complex = prev_tail.and_then(|c| detect_complex_script(c as u32)).is_some();
            let curr_is_indic_punct = curr_head.is_some_and(|c| matches!(c, '\u{0964}' | '\u{0965}'));
            if curr_head.is_some_and(is_clause_punct) && (prev_is_complex || curr_is_indic_punct) {
                return false;
            }
        }
        gap > space_threshold
    }

    /// Stacked two-line column/table-header cell detector, applied ONLY on the
    /// structure-tree (tagged-content) assembly path — never the main flow.
    ///
    /// A tagged table can draw a header cell as two stacked rows ("Comparison"
    /// over "rate"). When the structure-tree assembler linearizes the cell's
    /// spans it sees them as consecutive, horizontally OVERLAPPING (negative
    /// gap) spans whose baseline drop stayed just under `same_line_threshold`,
    /// so it treats them as one line and — because the gap is negative —
    /// `should_insert_space` returns false and they fuse ("Comparisonrate").
    /// A negative gap combined with a genuine baseline shift is two stacked
    /// tokens, never intra-word kerning (which shares a baseline), so a space
    /// is warranted.
    ///
    /// This deliberately lives OUTSIDE `should_insert_space`: the main flow
    /// (untagged PDFs, e.g. LaTeX math) already routes backtracking
    /// baseline-shifted runs — a fraction's numerator over its denominator —
    /// through dedicated newline branches before the space decision, and adding
    /// this rule there fragments equations. Scoping it to the tagged path keeps
    /// those inputs byte-identical while fixing stacked header cells.
    pub(super) fn stacked_cell_needs_space(prev: &TextSpan, current: &TextSpan) -> bool {
        let font_size = prev.font_size.max(current.font_size).max(1.0);
        let y_diff = (prev.bbox.y - current.bbox.y).abs();
        let gap = current.bbox.x - (prev.bbox.x + prev.bbox.width);
        // Under the caller's same-line band (else the caller line-breaks), a
        // real baseline shift (> 0.5 em) with horizontal overlap (negative gap)
        // is a stacked cell. Both sides must be alphanumeric word content, not
        // punctuation/symbol runs. ~keep
        gap < -0.5
            && y_diff > font_size * 0.5
            && prev.text.chars().next_back().is_some_and(|c| c.is_alphanumeric())
            && current.text.chars().next().is_some_and(|c| c.is_alphanumeric())
    }

    /// Detect a span whose text is `N.M` (all-digit groups around one dot) and whose
    /// bbox.width is >40% larger than char_widths imply. This pattern occurs in
    /// sailing-score / competition-table PDFs where two adjacent columns (e.g. Q8=1,
    /// F9=10) are stored as a single Tj text run "1.10" spanning both column cells.
    /// Reference ground truth tokenises them as separate words; we must split at the dot.
    pub(crate) fn is_column_spanning_decimal(span: &TextSpan) -> bool {
        let text = &span.text;
        let dot_pos = match text.find('.') {
            Some(p) if p > 0 && p < text.len() - 1 => p,
            _ => return false,
        };
        if text[dot_pos + 1..].contains('.') {
            return false;
        }
        if !text[..dot_pos].chars().all(|c| c.is_ascii_digit()) {
            return false;
        }
        if !text[dot_pos + 1..].chars().all(|c| c.is_ascii_digit()) {
            return false;
        }
        let char_count = text.chars().count();
        // Signal 1: sparse char_widths array. When the font's glyph
        // iteration produces fewer advance-width entries than there are
        // characters in the decoded string, the span was assembled from two
        // (or more) concatenated Tj runs whose widths come from different
        // points in the glyph table. This is the exact pattern issue 487
        // nougat_018 sailing-score grids hit: each score cell is emitted as
        // a single Tj like `1.10` with `char_widths=[w]` while the PDF
        // semantically means "1" followed by "10" in adjacent score
        // columns. bbox.width can still be tight here (the producer set
        // it to cover just the rendered glyph run), so the existing
        // bbox-inflation check below misses these. Catch them via the
        // sparse-cw signal directly. ~keep
        if !span.char_widths.is_empty() && span.char_widths.len() < char_count {
            return true;
        }
        let expected_width = if !span.char_widths.is_empty() {
            let cw_sum: f32 = span.char_widths.iter().sum();
            cw_sum * (char_count as f32 / span.char_widths.len() as f32)
        } else if span.font_size > 0.0 {
            // Digits are narrower than average; 0.50em per char is a safe
            // upper bound for all-digit strings (avoids the 0.60 fallback
            // producing false negatives on column-spanning sailing scores
            // when char_widths is empty, e.g. word_spans from extract_words). ~keep
            span.font_size * 0.50 * char_count as f32
        } else {
            return false;
        };
        // Use absolute gap (bbox_w - expected) rather than a ratio so that
        // 5-char spans like "12.11" (gap ≈ 1.1×fs) are caught along with
        // 4-char spans like "1.10" (gap ≈ 1.4×fs). 1.0×font_size is a safe
        // lower bound: normal text rarely has >1em of hidden whitespace. ~keep
        let gap = span.bbox.width - expected_width;
        span.font_size > 0.0 && gap > span.font_size * 1.0
    }

    /// When a CID font's glyph iteration produces fewer advance-width entries than
    /// `decode_text_to_unicode` produces unicode chars, `char_widths.len()` < char count.
    /// This indicates two concatenated text runs stored in one Tj operator (e.g. "Theorem1.7"
    /// where "Theorem" widths come from the font's glyph table and "1.7" doesn't have
    /// matching glyph entries). Return the byte offset at which to insert a space,
    /// or None if no split is appropriate.
    pub(crate) fn char_widths_boundary_split(span: &TextSpan) -> Option<usize> {
        let cw_len = span.char_widths.len();
        if cw_len == 0 {
            return None;
        }
        let char_count = span.text.chars().count();
        if cw_len >= char_count {
            return None;
        }
        let (boundary_byte, boundary_char) = span.text.char_indices().nth(cw_len)?;
        let prev_char = span.text[..boundary_byte].chars().next_back()?;
        if boundary_char == ' ' || prev_char == ' ' {
            return None;
        }
        // Non-ASCII chars at the boundary are encoding artifacts (e.g. Polish diacritics
        // in Latin-2 / CP1250 fonts producing one fewer char_width entry). Only split
        // when the boundary char is ASCII, indicating a genuine text-run concatenation. ~keep
        if !boundary_char.is_ascii() {
            return None;
        }
        // Split at letter→digit boundary (e.g. "Theorem1.7") or lower→upper ASCII
        // case boundary (e.g. "BigText" from concatenated CID runs "Big"+"Text").
        // Upper→lower transitions are excluded: a ligature spanning an upper→lower
        // boundary within a compound word (e.g. "officeMax" with "fl" ligature)
        // would otherwise produce a false split. ~keep
        if (prev_char.is_alphabetic() && boundary_char.is_ascii_digit())
            || (prev_char.is_ascii_lowercase() && boundary_char.is_ascii_uppercase())
        {
            Some(boundary_byte)
        } else {
            None
        }
    }

    /// Merge subscript and superscript spans into their base span.
    ///
    /// In math-heavy untagged PDFs, subscript glyphs (e.g. the "1" in "k₁") are
    /// stored as separate `TextSpan` entries at a slightly lower/higher baseline than
    /// the base character, and non-adjacent in reading order. The text assembly loop
    /// emits them as isolated tokens ("k … 1") rather than the expected word ("k1").
    ///
    /// A span is classified as a subscript/superscript when ALL of the following hold:
    ///  - 1–3 ASCII alphanumeric chars (digit or letter, no punctuation)
    ///  - font_size < 85 % of the page's maximum font size
    ///  - There exists a preceding "base" span whose right edge (x + width) is within
    ///    ±0.6 × sub_fs of the subscript's left edge (x-adjacent)
    ///  - The vertical offset between base and sub is in [8 %, 85 %] of base_fs
    ///    (distinguishes true sub/superscripts from same-line small caps)
    ///
    /// Matched subscript/superscript spans have their text appended to the base
    /// are removed from `spans`.
    pub(super) fn merge_sub_superscript_spans(spans: &mut Vec<TextSpan>) {
        let n = spans.len();
        if n < 2 {
            return;
        }
        let max_fs = spans.iter().map(|s| s.font_size).fold(0f32, f32::max);
        if max_fs <= 0.0 {
            return;
        }

        // Item 5b (M4): an INDEX CLUSTER is a comma-joined run of digits that the
        // producer set as a single subscript/superscript — an F-statistic's
        // degrees of freedom (`4,176` in `F4,176`) or a multi-affiliation marker
        // (`1,2`). These exceed the 3-char limit and contain a comma, so the plain
        // sub-char gate rejected them, stranding `F`, `4`, `176` as separate
        // tokens. Recognised here so the comma cluster merges back into its base. ~keep
        let is_index_cluster = |t: &str| -> bool {
            t.chars().count() >= 3
                && t.contains(',')
                && t.chars().all(|c| c.is_ascii_digit() || c == ',')
                && !t.starts_with(',')
                && !t.ends_with(',')
        };

        let mut to_merge: Vec<(usize, usize)> = Vec::new();
        let mut already_sub: std::collections::HashSet<usize> = std::collections::HashSet::new();

        for i in 0..n {
            let sub = &spans[i];
            // Char-count gate (not byte-count): U+00B2/B3/B9 are 2-byte
            // UTF-8 sequences and U+2070..U+209F are 3-byte, so the
            // earlier byte-length check would have dropped a legitimate
            // 3-digit Unicode subscript like "₁₂₃" (9 bytes). ~keep
            if sub.text.is_empty() || (sub.text.chars().count() > 3 && !is_index_cluster(&sub.text)) {
                continue;
            }
            // Accept the raw ASCII the extractor produces AND the
            // already-substituted Unicode super/subscript codepoints
            // (apply_super_sub_script_substitutions runs upstream).
            // Without the U+00B2/B3/B9 + U+2070..U+209F gate, a
            // chemistry formula like "H₂O" would lose the subscript
            // span from this merge, leaving "H ₂ O" in the output. ~keep
            let is_sub_char = |c: char| {
                c.is_ascii_alphanumeric()
                    || matches!(c, '\u{00B2}' | '\u{00B3}' | '\u{00B9}')
                    || ('\u{2070}'..='\u{209F}').contains(&c)
            };
            // M4 (item 5c): a span the producer explicitly raised/lowered with the
            // Text Rise operator (ISO 32000-1 §9.3.7 `Ts`) is an authoritative
            // sub/superscript even when it is NOT shrunk and is not in the ASCII /
            // Unicode sub-glyph set (e.g. a math operator superscript). `text_rise`
            // is stored as the Ts/font-size ratio, so |ratio| ≥ 0.10 marks a real
            // shift. Such a span bypasses the charset and font-size gates below; the
            // x/y proximity gates in the base search still apply, so a genuinely
            // detached different-row marker is not over-merged. ~keep
            let ts_flagged = sub.text_rise.abs() >= 0.10;
            if !ts_flagged && !sub.text.chars().all(is_sub_char) && !is_index_cluster(&sub.text) {
                continue;
            }
            // Must be clearly smaller than the dominant font on this page (unless
            // the producer flagged it via Ts). ~keep
            if !ts_flagged && sub.font_size >= max_fs * 0.80 {
                continue;
            }
            let sub_fs = sub.font_size;
            let sub_x = sub.bbox.x;
            let sub_y = sub.bbox.y;

            // A purely NUMERIC sub-run (digits, optionally comma-joined) at a
            // base's advance edge is an inline super/subscript even when its
            // bbox shares the base's baseline. Some producers raise a glyph with
            // a small font but emit it on the SAME text-line baseline (the
            // visual rise lives in the glyph's own bbox, not the line's), so the
            // extractor records y_diff_abs ≈ 0. The 12 %-of-em vertical lower
            // bound (which screens out same-line small caps) would then strand
            // the marker — e.g. the isotope label `123` in `[123I]FP-CIT`, or an
            // author-affiliation marker `1,2`. Small caps are never bare digits,
            // so dropping the lower bound for numeric subs is safe; the smaller-
            // font, x-edge, valid-base, and upper-y gates still apply. ~keep
            let sub_is_numeric = sub.text.chars().all(|c| c.is_ascii_digit() || c == ',') && !ts_flagged;

            let search_limit = 30.min(i);
            let mut best: Option<(usize, f32)> = None;

            for j in (i.saturating_sub(search_limit)..i).rev() {
                if already_sub.contains(&j) {
                    continue;
                }
                let base = &spans[j];
                // Base must be at least 25 % larger than the sub (sub_fs ≤ 0.80×base_fs),
                // UNLESS the producer flagged the sub via Ts (then it may be the same
                // size as its base — the rise itself, not the size, marks it). ~keep
                if !ts_flagged && base.font_size < sub_fs * 1.25 {
                    continue;
                }
                // Base span must be a valid subscript host:
                //   • 1-char bases (single math variable: k, γ, ρ, H, ∆, …)
                //   • 2-char bases that are NOT two lowercase-ASCII letters
                //     (accepts "Pr", "εp", "ρε" but rejects "of", "to")
                //   • longer bases ENDING in an acronym — a run of ≥2 trailing
                //     uppercase ASCII letters (e.g. a wide body span
                //     "…activation of VPAC", or "CA1"'s "CA"). Receptor/region
                //     names (VPAC, CA, PAC, GABA, NMDA, …) carry a subscript on
                //     their trailing acronym, but the producer emits the whole
                //     wrapped line as one span, so the ≤2-char gate stranded the
                //     subscript and the row-band sort glued it onto a later word
                //     ("…of VPAC receptors … pyramidal1"). The subscript text is
                //     appended to the base's END, which is exactly the acronym, so
                //     it reconstructs "VPAC1". The x-edge gate below still requires
                //     the sub to sit at the base's advance edge, and the trailing
                //     run being UPPERCASE keeps ordinary prose (which ends in a
                //     lowercase letter or punctuation) from ever matching.
                // Multi-char lowercase-only strings like "and", "let", "sup"
                // are English words or common operators; their adjacent digit
                // spans are handled by the assembly loop and char_widths_boundary_split. ~keep
                let chars: Vec<char> = base.text.chars().collect();
                let ends_in_acronym = || {
                    let trailing_upper = chars.iter().rev().take_while(|c| c.is_ascii_uppercase()).count();
                    trailing_upper >= 2
                };
                let is_valid_base = match chars.len() {
                    1 => true,
                    2 => chars.iter().any(|c| !c.is_ascii_lowercase()),
                    _ => ends_in_acronym(),
                };
                if !is_valid_base {
                    continue;
                }
                if Self::needs_numeric_script_boundary(base, sub) {
                    continue;
                }
                let base_right = base.bbox.x + base.bbox.width;
                let x_dist = sub_x - base_right;
                let y_diff_abs = (base.bbox.y - sub_y).abs();

                // Use em-relative x_dist thresholds.
                // Real sub/superscript glyphs land within ±[−0.1×base_fs, 0.25×base_fs]
                // of the base's advance edge; absolute bounds were wrong for non-12pt fonts. ~keep
                let base_fs = base.font_size.max(1.0);
                let x_lo = -0.1 * base_fs;
                let x_hi = 0.25 * base_fs;
                if x_dist < x_lo || x_dist > x_hi {
                    continue;
                }
                // Vertical offset must be in the sub/superscript range.
                // Lower bound 12 % of base_fs ensures same-line small caps are excluded.
                // Upper bound 75 % excludes large line-to-line y differences (e.g.
                // author affiliation numbers on a different baseline row).
                // Numeric subs (digits/commas) may sit on the base baseline, so
                // skip the small-caps lower bound for them; all other subs keep it. ~keep
                let y_lo = if sub_is_numeric { 0.0 } else { base.font_size * 0.12 };
                if y_diff_abs < y_lo || y_diff_abs > base.font_size * 0.75 {
                    continue;
                }
                let score = x_dist.abs();
                if best.is_none() || score < best.unwrap().1 {
                    best = Some((j, score));
                }
            }

            if let Some((base_idx, _)) = best {
                to_merge.push((base_idx, i));
                already_sub.insert(i);
            }
        }

        if to_merge.is_empty() {
            return;
        }

        // Collect (base_idx, sub_idx, sub_text, sub_right_edge, sub_char_widths, sub_fs)
        // before mutating spans. ~keep
        let ops: Vec<(usize, usize, String, f32, Vec<f32>, f32)> = to_merge
            .iter()
            .map(|pair| {
                let (bi, si) = *pair;
                let sub = &spans[si];
                (
                    bi,
                    si,
                    sub.text.clone(),
                    sub.bbox.x + sub.bbox.width,
                    sub.char_widths.clone(),
                    sub.font_size,
                )
            })
            .collect();

        // Apply: append sub text to base; extend bbox and char_widths to cover the sub.
        //
        // Extending bbox: the assembly loop uses span widths for gap calculations — keeping
        // the original width would make the gap to the following span appear too large.
        //
        // Extending char_widths: char_widths_boundary_split fires whenever cw_len < char_count.
        // After merging sub text, char_count grows but cw_len stays the same, which would
        // cause the split to re-separate the merged token (e.g. "k1" → "k 1"). Adding
        // estimated widths for the sub characters prevents this. ~keep
        for (base_idx, _, sub_text, sub_right, sub_cw, sub_fs) in &ops {
            let base = &mut spans[*base_idx];
            base.text.push_str(sub_text);
            let base_right = base.bbox.x + base.bbox.width;
            if *sub_right > base_right {
                base.bbox.width = sub_right - base.bbox.x;
            }
            if !base.char_widths.is_empty() {
                let sub_char_count = sub_text.chars().count();
                if !sub_cw.is_empty() {
                    base.char_widths.extend_from_slice(sub_cw);
                } else {
                    // Estimate sub char widths at 0.50 em per character. ~keep
                    let w = sub_fs * 0.50;
                    for _ in 0..sub_char_count {
                        base.char_widths.push(w);
                    }
                }
            }
        }

        let to_remove: std::collections::HashSet<usize> = ops.iter().map(|(_, si, _, _, _, _)| *si).collect();
        let mut idx = 0usize;
        spans.retain(|_| {
            let keep = !to_remove.contains(&idx);
            idx += 1;
            keep
        });
    }

    /// Append span text to `out`, splitting merged runs for cleaner word tokenisation.
    /// Priority 0: spans whose text is entirely `\n`/`\r` are line-break signals.
    /// Priority 1: column-spanning decimal (nougat_018 sailing tables).
    /// Priority 2: char_widths boundary split (pdfa_004 CID-font merge artifacts).
    #[inline]
    pub(crate) fn push_span_text(out: &mut String, span: &TextSpan) {
        // A span whose entire text is one or more newline/CR characters is a
        // ToUnicode line-break signal. Treat it as a logical newline separator rather
        // than emitting the raw control characters verbatim as visible content. ~keep
        if !span.text.is_empty() && span.text.chars().all(|c| c == '\n' || c == '\r') {
            if !out.ends_with('\n') {
                out.push('\n');
            }
            return;
        }
        if Self::is_column_spanning_decimal(span) {
            let dot = span.text.find('.').unwrap();
            Self::push_str_without_soft_hyphens(out, &span.text[..dot]);
            out.push(' ');
            Self::push_str_without_soft_hyphens(out, &span.text[dot + 1..]);
        } else if let Some(split) = Self::char_widths_boundary_split(span) {
            Self::push_str_without_soft_hyphens(out, &span.text[..split]);
            out.push(' ');
            Self::push_str_without_soft_hyphens(out, &span.text[split..]);
        } else {
            Self::push_str_without_soft_hyphens(out, &span.text);
        }
    }

    /// Append `s` to `out`, dropping U+00AD (SOFT HYPHEN). Per ISO 32000-1
    /// §14.8.2.2.3 a soft hyphen only marks a discretionary line-break point —
    /// it is never meaningful rendered content, so it must not survive into
    /// flat-text output regardless of whether it sits at a line boundary (the
    /// PDF's own line wrap is not preserved here) or mid-word within a span
    /// whose glyphs were positioned individually.
    #[inline]
    fn push_str_without_soft_hyphens(out: &mut String, s: &str) {
        if s.contains('\u{00AD}') {
            out.extend(s.chars().filter(|&c| c != '\u{00AD}'));
        } else {
            out.push_str(s);
        }
    }

    /// Append a span's text to the structure-tree assembly, reversing a
    /// PURE-RTL run (every non-space char is an Arabic/Hebrew letter, no Latin)
    /// from visual to logical order. The tagged/struct-tree path collapses each
    /// run to a single span and never reaches `reverse_rtl_visual_order_runs`,
    /// so visually-stored RTL (e.g. issue10301 Hebrew "גבא") otherwise leaked
    /// out reversed. A single-direction run's logical order is just its reverse,
    /// so no glyph geometry is needed for the pure-RTL case.
    pub(super) fn push_span_text_bidi(out: &mut String, span: &TextSpan, rtl_run: bool) {
        use crate::text::rtl_detector::is_rtl_text;
        // A span whose glyphs were drawn right-to-left (logical storage — the
        // producer positioned each glyph individually at decreasing x, ISO
        // 32000-1 §14.8.2.3.3 method 1; detected by `detect_rtl_draw_direction`)
        // already carries its CHARACTERS in LOGICAL order. The visual→logical
        // character reversal below assumes VISUAL storage and would corrupt
        // them, so emit the text verbatim — the letters are already correct.
        // (Word ORDER within such a span is left as the reading-order sort
        // produced it: a producer may emit words logically or visually within
        // the same document, so a blanket word reversal would corrupt the
        // logical ones.) Visual storage — the default — is never flagged and
        // keeps the character-reversal below, so it stays byte-identical. ~keep
        if span.rtl_draw_logical {
            Self::push_span_text(out, span);
            return;
        }
        let mut rtl = 0usize;
        let mut has_latin = false;
        for c in span.text.chars() {
            if c.is_whitespace() {
                continue;
            }
            if c.is_ascii_alphabetic() {
                has_latin = true;
                break;
            }
            if is_rtl_text(c as u32) {
                rtl += 1;
            }
        }
        if rtl >= 2 && !has_latin {
            let mut tmp = span.clone();
            // Strip producer-inserted SPACEs that fall *between two Arabic
            // letters* inside a single show string. ISO 32000-1 §14.8.2.3.3
            // states a reverse-order show string "shall not contain interior
            // SPACEs" — a word break is signalled by a SPACE at the string
            // boundary (here, a separate span), never inside it. Arabic is
            // cursive, so an interior space splits letters that the script
            // joins; it is never a word boundary in the pure-text
            // representation (§14.8.2.5). Restricted to Arabic (cursive): a
            // non-cursive script such as Hebrew can legitimately carry a
            // space-separated pair in one show string, so it is left alone. ~keep
            tmp.text = Self::reverse_rtl_keeping_marks(&Self::strip_interior_arabic_spaces(&span.text))
                .replace(Self::RTL_WORD_BOUNDARY, " ");
            Self::push_span_text(out, &tmp);
        } else if rtl_run && Self::is_reversible_rtl_neutral_span(&span.text) {
            // A neutral-only span (separator / terminator punctuation plus
            // spaces — no strong letters and no digits) embedded in a pure-RTL
            // run carries its glyphs in *visual* (content-stream draw) order.
            // Per UAX #9 the neutrals inherit the surrounding right-to-left
            // direction (rules N1/N2), so their logical order is the reverse of
            // the visual sequence: a visual "<space><comma>" drawn between two
            // Hebrew words becomes "<comma><space>", re-attaching the comma to
            // the preceding word. The pure-RTL words around it are reversed by
            // the branch above; without this the punctuation stayed stranded on
            // the wrong side of the inter-word space. ~keep
            let mut tmp = span.clone();
            tmp.text = span.text.chars().rev().collect();
            Self::push_span_text(out, &tmp);
        } else if rtl_run && Self::is_reversible_rtl_numeric_span(&span.text) {
            // A neutral+numeric span (e.g. a Hebrew-context " ,2009-" or " 600-")
            // embedded in a pure-RTL run carries its glyphs in *visual*
            // (content-stream draw) order. Reverse it to logical order while
            // keeping each digit run forward (UAX #9 rule L2): visual " ,2009-"
            // → logical "-2009, ", re-attaching the hyphen to the number and the
            // comma to the preceding word, without ever flipping 2009 → 9002. ~keep
            let mut tmp = span.clone();
            tmp.text = crate::text::bidi::reverse_rtl_keep_numbers(&span.text);
            Self::push_span_text(out, &tmp);
        } else {
            Self::push_span_text(out, span);
        }
    }

    /// Whether `text` is a neutral+numeric span eligible for number-preserving
    /// RTL visual→logical reversal in [`push_span_text_bidi`]: every non-space
    /// char is a [reorderable neutral](Self::is_rtl_reorderable_neutral), an
    /// ASCII hyphen-minus, or a digit (ASCII / Arabic-Indic U+0660–0669 /
    /// Extended Arabic-Indic U+06F0–06F9); it contains **exactly one** maximal
    /// digit run (so a date range `2009-2010` or an ORCID is never reversed),
    /// at least one movable neutral/hyphen, and the number-preserving reversal
    /// actually changes it (else the cheaper verbatim path is byte-identical).
    fn is_reversible_rtl_numeric_span(text: &str) -> bool {
        let is_digit = |c: char| {
            c.is_ascii_digit() || ('\u{0660}'..='\u{0669}').contains(&c) || ('\u{06F0}'..='\u{06F9}').contains(&c)
        };
        let mut has_movable = false;
        let mut digit_runs = 0usize;
        let mut in_digit = false;
        for c in text.chars() {
            if is_digit(c) {
                if !in_digit {
                    digit_runs += 1;
                    in_digit = true;
                }
                continue;
            }
            in_digit = false;
            if c.is_whitespace() {
                continue;
            }
            if c == '-' || Self::is_rtl_reorderable_neutral(c) {
                has_movable = true;
                continue;
            }
            return false;
        }
        digit_runs == 1 && has_movable && crate::text::bidi::reverse_rtl_keep_numbers(text) != text
    }

    /// Remove ASCII SPACE (U+0020) characters that sit *between two Arabic
    /// letters* within a single show string — producer-inserted spurious
    /// spaces that split a cursive word (e.g. `قِ ل` inside `القِطّ`).
    ///
    /// Per ISO 32000-1 §14.8.2.3.3 a show string "shall not contain interior
    /// SPACEs"; a genuine word break is a SPACE at a string boundary (a
    /// separate span in this pipeline). Combining marks between the space and
    /// its neighbouring base letter are seen through, so a mark sitting next to
    /// the space does not hide the Arabic letter on that side. Leading and
    /// trailing spaces (real word-break candidates) and spaces flanked by
    /// anything other than two Arabic letters are preserved verbatim, so the
    /// fast path returns the input unchanged when there is nothing to strip.
    pub(super) fn strip_interior_arabic_spaces(text: &str) -> String {
        use crate::text::rtl_detector::{is_arabic_letter, is_rtl_diacritic};
        if !text.contains(' ') {
            return text.to_string();
        }
        // First non-mark char in `it` is an Arabic letter? (marks are seen
        // through so a diacritic next to the space does not hide its base.) ~keep
        fn arabic_letter_past_marks<'a>(it: impl Iterator<Item = &'a char>) -> bool {
            for &c in it {
                if is_rtl_diacritic(c as u32) {
                    continue;
                }
                return is_arabic_letter(c as u32);
            }
            false
        }
        let chars: Vec<char> = text.chars().collect();
        let qualifying: Vec<usize> = (0..chars.len())
            .filter(|&i| {
                chars[i] == ' '
                    && arabic_letter_past_marks(chars[..i].iter().rev())
                    && arabic_letter_past_marks(chars[i + 1..].iter())
            })
            .collect();
        if qualifying.is_empty() {
            return text.to_string();
        }
        // SHATTER case (§14.8.2.3.3: a show-string must not contain interior
        // SPACEs). When a space sits between a MAJORITY of adjacent Arabic-letter
        // pairs, the producer exploded one cursive word into separate glyphs
        // (e.g. `فصيلة` drawn as `ة لي ص ف`); every interior space is spurious, so
        // strip them all. The density test (qualifying ≥ half the inter-letter
        // gaps) tells this apart from ordinary multi-word text, whose spaces are
        // sparse real word breaks (the right_to_left_01 class) — those stay. ~keep
        let arabic_letters = chars.iter().filter(|&&c| is_arabic_letter(c as u32)).count();
        let gaps = arabic_letters.saturating_sub(1).max(1);
        if qualifying.len() >= 2 && qualifying.len() * 2 >= gaps {
            let drop: std::collections::HashSet<usize> = qualifying.iter().copied().collect();
            return chars
                .iter()
                .enumerate()
                .filter_map(|(i, &c)| (!drop.contains(&i)).then_some(c))
                .collect();
        }
        // Sparse case: a lone spurious cursive-join space. A span with several
        // sparse Arabic-flanked spaces is ordinary multi-word text whose spaces
        // are real word breaks — leave them intact. ~keep
        if qualifying.len() != 1 {
            return text.to_string();
        }
        let drop = qualifying[0];
        // Joining-type discriminator (§14.8.2.3.3). The cursive join already
        // breaks AFTER a right-joining-only letter (ا د ذ ر ز و …), so a space
        // there renders identically whether it is a genuine word break or a
        // producer artefact — the two are indistinguishable. Stripping it would
        // risk concatenating two real words (`دار اب` → `داراب`). Only a space
        // after a dual-joining letter unambiguously broke a join that should
        // not break, so restrict the strip to that case and keep the space when
        // the preceding base letter (seen past any combining marks) is
        // right-joining. ~keep
        let preceding_right_joining = chars[..drop]
            .iter()
            .rev()
            .find(|&&c| !is_rtl_diacritic(c as u32))
            .is_some_and(|&c| crate::text::rtl_detector::is_right_joining_arabic(c as u32));
        if preceding_right_joining {
            return text.to_string();
        }
        chars
            .iter()
            .enumerate()
            .filter_map(|(i, &c)| (i != drop).then_some(c))
            .collect()
    }

    /// Emit the inter-line newline(s) between two vertically separated spans in
    /// the struct-order assembler. A normal line gap maps to one to three
    /// newlines proportional to the vertical distance (`y_diff / line_height`,
    /// clamped) so multi-line paragraph spacing survives. When `single_break`
    /// is set — two consecutive cells of a tagged table on different rows — a
    /// single newline is emitted instead: table rows are stacked block rows,
    /// not free-leading paragraphs (ISO 32000-1 §14.8.4.3.4), and the geometric
    /// row pitch (~1.7× leading) would otherwise insert a spurious blank line
    /// between every row.
    pub(super) fn push_line_breaks(
        text: &mut String,
        prev: &TextSpan,
        span: &TextSpan,
        y_diff: f32,
        single_break: bool,
    ) {
        if single_break {
            text.push('\n');
            return;
        }
        let font_size = prev.font_size.max(span.font_size).max(10.0);
        let line_height = font_size * 1.2;
        let num_breaks = (y_diff / line_height).round() as usize;
        for _ in 0..num_breaks.clamp(1, 3) {
            text.push('\n');
        }
    }

    /// Whether every span in this marked-content element is part of a *pure*
    /// right-to-left run: at least one Arabic/Hebrew letter is present and no
    /// Latin letter is. Mirrors the gating in [`order_mcid_spans`] (the branch
    /// that sorts pure-RTL spans right-to-left). Used to decide whether
    /// neutral-only punctuation spans inside the run must be reversed from
    /// visual to logical order by [`push_span_text_bidi`].
    pub(super) fn mcid_run_is_pure_rtl(spans: &[crate::layout::TextSpan]) -> bool {
        use crate::text::rtl_detector::is_rtl_text;
        let has_rtl = spans.iter().any(|s| s.text.chars().any(|c| is_rtl_text(c as u32)));
        let has_latin = spans.iter().any(|s| s.text.chars().any(|c| c.is_ascii_alphabetic()));
        has_rtl && !has_latin
    }

    /// Is `c` a direction-neutral punctuation mark whose order inside an RTL
    /// run is a pure transposition — safe to reverse with the surrounding RTL
    /// neutrals? Restricted to separators and terminators (comma, full stop,
    /// semicolon, colon, exclamation, question, and their Arabic/Hebrew
    /// equivalents). Deliberately excludes paired brackets and quotation marks
    /// (which need UAX #9 L4 mirroring, handled elsewhere), digits, and any
    /// character that anchors an embedded left-to-right sub-run.
    fn is_rtl_reorderable_neutral(c: char) -> bool {
        matches!(
            c,
            ',' | '.'
                | ';'
                | ':'
                | '!'
                | '?'
                | '\u{05BE}'
                | '\u{05C3}'
                | '\u{060C}'
                | '\u{061B}'
                | '\u{061F}'
                | '\u{06D4}'
        )
    }

    /// Whether `text` is a neutral-only span eligible for the RTL visual→logical
    /// reversal in [`push_span_text_bidi`]: every character is whitespace or a
    /// [reorderable neutral](Self::is_rtl_reorderable_neutral), it contains at
    /// least one such punctuation mark, and it is at least two characters long
    /// (so there is an order to fix). A lone punctuation glyph or a bare space
    /// run reverses to itself and is left untouched.
    pub(super) fn is_reversible_rtl_neutral_span(text: &str) -> bool {
        let mut has_punct = false;
        let mut count = 0usize;
        for c in text.chars() {
            count += 1;
            if c.is_whitespace() {
                continue;
            }
            if Self::is_rtl_reorderable_neutral(c) {
                has_punct = true;
                continue;
            }
            return false;
        }
        has_punct && count >= 2
    }

    /// Reverse a pure-RTL run from visual to logical order while keeping each
    /// Arabic/Hebrew combining mark attached to its base letter.
    ///
    /// A naive `chars().rev()` reverses by Unicode scalar value, so a base
    /// letter's diacritics (which follow it in logical order — kasra/shadda
    /// U+0650/U+0651, Hebrew points U+05B0..) jump *in front* of the base and
    /// float off as standalone marks. Grouping each base char with the
    /// combining marks that trail it, then reversing the group order (each
    /// group's internal order preserved), keeps marks bound to their base.
    pub(super) fn reverse_rtl_keeping_marks(text: &str) -> String {
        use crate::text::rtl_detector::is_rtl_diacritic;
        let mut groups: Vec<Vec<char>> = Vec::new();
        for c in text.chars() {
            if is_rtl_diacritic(c as u32) && !groups.is_empty() {
                groups.last_mut().unwrap().push(c);
            } else {
                groups.push(vec![c]);
            }
        }
        groups.iter().rev().flatten().collect()
    }
}
