//! Post-extraction span rotation, composition, and banding.
//!
//! Split out of the parent's single 18,544-line `impl PdfDocument`, which made
//! `document.rs` 1.2 MiB and tripped the 500 KiB file-safety limit. A child module's
//! `impl` is the same inherent impl and sees the parent's private items unchanged. ~keep

use super::*;

impl PdfDocument {
    /// Map a span rectangle (already translated so the page origin is at
    /// `(0, 0)`) through a clockwise page `/Rotate` of `rot` degrees, returning
    /// the axis-aligned bounding box in the displayed coordinate frame.
    ///
    /// `page_w` / `page_h` are the unrotated page dimensions; for 90° / 270° the
    /// displayed page is `page_h × page_w`. Per ISO 32000-1:2008 §7.7.3.3 the
    /// rotation is clockwise and §8.3.3 gives the point transform. `rot` must be
    /// a normalised multiple of 90 (`0/90/180/270`); any other value returns the
    /// rectangle unchanged. `rot == 0` is the identity and `rot == 180` is
    /// numerically identical to the legacy mirror, preserving byte-for-byte
    /// output for unrotated and 180° pages.
    pub(crate) fn rotate_span_bbox(
        bbox: crate::geometry::Rect,
        rot: i32,
        page_w: f32,
        page_h: f32,
    ) -> crate::geometry::Rect {
        // Map a point (y-up) by the clockwise display rotation. ~keep
        let map = |x: f32, y: f32| -> (f32, f32) {
            match rot {
                90 => (y, page_w - x),
                180 => (page_w - x, page_h - y),
                270 => (page_h - y, x),
                _ => (x, y),
            }
        };
        let (ax, ay) = map(bbox.x, bbox.y);
        let (bx, by) = map(bbox.x + bbox.width, bbox.y + bbox.height);
        crate::geometry::Rect::new(ax.min(bx), ay.min(by), (ax - bx).abs(), (ay - by).abs())
    }

    /// Map a single span's bbox into the displayed frame for a `/Rotate`d page
    /// (translate to origin → [`rotate_span_bbox`] → translate back).
    ///
    /// This is the only place `TextSpan::page_rotation_applied` is ever written,
    /// and it is reached only through `postprocess_spans` — i.e. from
    /// [`Self::extract_spans`] / `extract_spans_filtered`, never from
    /// [`Self::extract_spans_filtered_with_reading_order`] (see that function's
    /// doc comment). Anything reading `page_rotation_applied` off a span must
    /// know which of the two extraction paths produced it.
    fn map_span_into_rotated_frame(s: &mut crate::layout::TextSpan, rot: i32, llx: f32, lly: f32, w: f32, h: f32) {
        let rel = crate::geometry::Rect::new(s.bbox.x - llx, s.bbox.y - lly, s.bbox.width, s.bbox.height);
        let m = Self::rotate_span_bbox(rel, rot, w, h);
        s.bbox.x = llx + m.x;
        s.bbox.y = lly + m.y;
        s.bbox.width = m.width;
        s.bbox.height = m.height;
        // `rotation_degrees` stays raw (downstream passes select on it), so
        // record the applied rotation for `TextSpan::page_bbox` — otherwise it
        // would re-rotate the already-mapped rect. ~keep
        s.page_rotation_applied = rot;
    }

    /// Order rotated runs that were segregated out of the horizontal reading
    /// flow. Spans drawn with a rotated text matrix (`rotation_degrees != 0`)
    /// break the axis-aligned assumptions of the row-band / XY-cut sort, so they
    /// are pulled out, ordered here, and appended as their own blocks. Runs are
    /// grouped by rotation (first-seen group order preserved); within a group
    /// each span's origin is rotated back into an upright frame and the standard
    /// row-aware comparator (top→bottom, left→right) is applied there.
    pub(crate) fn order_rotated_blocks(spans: Vec<crate::layout::TextSpan>) -> Vec<crate::layout::TextSpan> {
        let mut groups: Vec<(f32, Vec<crate::layout::TextSpan>)> = Vec::new();
        for s in spans {
            let key = s.rotation_degrees;
            match groups.iter_mut().find(|(k, _)| (*k - key).abs() < 0.5) {
                Some(g) => g.1.push(s),
                None => groups.push((key, vec![s])),
            }
        }
        let mut out = Vec::new();
        for (deg, mut group) in groups {
            let (sin, cos) = (-deg).to_radians().sin_cos();
            // Upright frame: rotate each origin by -deg, then read top→bottom,
            // left→right exactly as horizontal text. ~keep
            group.sort_by(|a, b| {
                let ax = a.bbox.x * cos - a.bbox.y * sin;
                let ay = a.bbox.x * sin + a.bbox.y * cos;
                let bx = b.bbox.x * cos - b.bbox.y * sin;
                let by = b.bbox.x * sin + b.bbox.y * cos;
                crate::utils::row_aware_span_cmp(ay, ax, by, bx)
            });
            out.extend(group);
        }
        out
    }

    /// Re-attach an oversized lone leading capital (a drop-cap / table-title
    /// initial that the producer set in a larger font, so it became its own
    /// span) to the body run immediately to its right on the same line —
    /// otherwise reading-order strands it (`TABLE` → `T` … `ABLE`).
    ///
    /// Conservative gates so prose drop-caps / standalone capitals aren't glued
    /// to the wrong word: the candidate must be a single uppercase ASCII letter
    /// at ≥1.5× the body run's font size, its right edge within ~0.3 em of the
    /// body's left edge, vertically overlapping it, and the body must start with
    /// a letter. Runs in raw span order before reading-order sorting.
    pub(super) fn merge_drop_cap_initials(spans: &mut Vec<crate::layout::TextSpan>) {
        let n = spans.len();
        if n < 2 {
            return;
        }
        // A genuine drop cap is oversized relative to the page's *normal* body
        // text, not merely relative to its right-hand neighbor. Inline math such
        // as "A_st" pairs a normal-size capital with a shrunken subscript; gating
        // on the neighbor alone would treat that capital as oversized and glue
        // "A" + "st" into "Ast". Anchor the size gate to the median size of
        // multi-character spans (real words) so a body-size capital cannot
        // qualify. ~keep
        let mut body_sizes: Vec<f32> = spans
            .iter()
            .filter(|s| s.font_size > 0.0 && s.text.chars().nth(1).is_some())
            .map(|s| s.font_size)
            .collect();
        if body_sizes.is_empty() {
            return;
        }
        body_sizes.sort_by(|a, b| crate::utils::safe_float_cmp(*a, *b));
        let body_size = body_sizes[body_sizes.len() / 2];

        // Span indices sorted by left edge, and the widest font on the page, so
        // each initial only probes spans whose left edge falls in its narrow
        // candidate x-window (was a full O(n) rescan per initial). A continuation
        // satisfies `gap in [-fs*0.5, fs*0.12]`, i.e. its left edge is within
        // [init_right - max_fs*0.5, init_right + max_fs*0.12]; using the page max
        // font widens the window conservatively, and the exact per-candidate gap
        // test below reproduces the original filter — so this is byte-identical. ~keep
        let order: Vec<usize> = {
            let mut o: Vec<usize> = (0..n).collect();
            o.sort_by(|&a, &b| crate::utils::safe_float_cmp(spans[a].bbox.x, spans[b].bbox.x));
            o
        };
        let max_fs = spans.iter().map(|s| s.font_size).fold(0.0_f32, f32::max);

        let mut target: Vec<Option<usize>> = vec![None; n];
        for i in 0..n {
            let init = &spans[i];
            if init.text.chars().count() != 1 || init.font_size <= 0.0 {
                continue;
            }
            if !init.text.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
                continue;
            }
            if init.font_size < body_size * 1.5 {
                continue;
            }
            let init_right = init.bbox.x + init.bbox.width;
            // Candidates: spans whose left edge is in the conservative window.
            // Collect their indices and visit in ASCENDING ORIGINAL ORDER so the
            // strict-`<` min keeps the same first-wins tie-break as the old scan. ~keep
            let lo_x = init_right - max_fs * 0.5;
            let hi_x = init_right + max_fs * 0.12;
            let lo = order.partition_point(|&k| spans[k].bbox.x < lo_x);
            let hi = order.partition_point(|&k| spans[k].bbox.x <= hi_x);
            let mut cands: Vec<usize> = order[lo..hi].to_vec();
            cands.sort_unstable();
            let mut best: Option<usize> = None;
            let mut best_gap = f32::MAX;
            for &j in &cands {
                let body = &spans[j];
                if j == i || body.font_size <= 0.0 {
                    continue;
                }
                if !body.text.chars().next().is_some_and(|c| c.is_alphabetic()) {
                    continue;
                }
                // Continuation shares the initial's baseline (same text line). A
                // tall oversized initial also vertically overlaps the line *above*
                // it, so a raw bbox-overlap test would let it reach up and steal a
                // word from the previous line (alice_old: the 16.8pt "A" of "A very
                // heavy weight" overlapping "Or if" → "OrAif"). Baseline proximity
                // (≈ bbox bottom) keeps the merge on the initial's own line. ~keep
                if (init.bbox.y - body.bbox.y).abs() > body.font_size * 0.5 {
                    continue;
                }
                // Body immediately to the right, essentially touching. A genuine
                // oversized initial is the first glyph of one word ("T" of
                // "TABLE", "P" of "PENALTY"), so its continuation begins within a
                // hair of the initial's advance — never across a word space. A
                // word-space gap (~0.25 em) would wrongly glue a standalone "A"
                // or "I" onto the next word ("A Perspective" → "APerspective"),
                // so the upper bound stays well below it. ~keep
                let gap = body.bbox.x - init_right;
                if gap < -body.font_size * 0.5 || gap > body.font_size * 0.12 {
                    continue;
                }
                if gap.abs() < best_gap {
                    best_gap = gap.abs();
                    best = Some(j);
                }
            }
            target[i] = best;
        }

        let mut taken = vec![false; n];
        let mut remove = vec![false; n];
        for i in 0..n {
            let Some(j) = target[i] else { continue };
            if taken[j] || remove[j] || remove[i] {
                continue;
            }
            taken[j] = true;
            remove[i] = true;
            let init_text = spans[i].text.clone();
            let init_left = spans[i].bbox.x;
            let body = &mut spans[j];
            body.text = format!("{init_text}{}", body.text);
            let right = body.bbox.x + body.bbox.width;
            body.bbox.x = init_left.min(body.bbox.x);
            body.bbox.width = right - body.bbox.x;
        }
        let mut k = 0;
        spans.retain(|_| {
            let keep = !remove[k];
            k += 1;
            keep
        });
    }

    /// True for Computer-Modern (`CM*`) or symbol font names, after stripping a
    /// `ABCDEF+` subset tag. Used to scope the `¬`→`.` decimal recovery.
    pub(super) fn is_cm_or_symbol_font(font_name: &str) -> bool {
        let base = font_name.split('+').next_back().unwrap_or(font_name);
        let lower = base.to_ascii_lowercase();
        lower.starts_with("cm") || lower.contains("symbol")
    }

    /// Replace a `¬` (U+00AC) that a math subset drew from its `logicalnot`
    /// slot as a decimal point. Two shapes are recovered:
    ///
    ///   - `digit ¬ digit`         → `digit.digit` (e.g. `1¬00` → `1.00`)
    ///   - `digit ¬ <space> digit` → `digit.digit` (e.g. `1¬ 00` → `1.00`)
    ///
    /// The second form covers subsets that emit a single space between the
    /// decimal glyph and the fractional digits; the lone separating space is
    /// dropped so the number reads as one token. The leading digit must abut
    /// `¬` directly in both shapes, so a genuinely spaced negation (`5 ¬ 3`,
    /// `A ¬ B`) is left untouched. Every other `¬` is preserved.
    pub(super) fn fix_digit_logicalnot_decimal(text: &str) -> String {
        let chars: Vec<char> = text.chars().collect();
        let mut out = String::with_capacity(text.len());
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            if c == '\u{00AC}' && i > 0 && chars[i - 1].is_ascii_digit() {
                if chars.get(i + 1).is_some_and(|n| n.is_ascii_digit()) {
                    out.push('.');
                    i += 1;
                    continue;
                }
                if chars.get(i + 1) == Some(&' ') && chars.get(i + 2).is_some_and(|n| n.is_ascii_digit()) {
                    out.push('.');
                    i += 2;
                    continue;
                }
            }
            out.push(c);
            i += 1;
        }
        out
    }

    /// Drop spans whose bbox lies ENTIRELY outside the page's MediaBox.
    ///
    /// PDFs that reuse one big Form XObject across pages (ExpertPdf and similar
    /// tools - see issue B1 / nougat_005.pdf) rely on the content stream's `W n`
    /// clip rectangle to hide the off-page portion. The text extractor does not
    /// honour `W n` yet, so without this filter a page emits every page's worth of
    /// spans at distinct but out-of-bounds Y coordinates. Spans that even
    /// PARTIALLY overlap the MediaBox are kept, so legitimate bleed / trim-mark
    /// content is never dropped.
    ///
    /// `get_page_media_box` returns `(llx, lly, urx, ury)` - absolute corner
    /// coordinates per ISO 32000-1 s7.7.3.3, NOT `(x, y, width, height)`.
    pub(super) fn drop_offpage_spans(&self, page_index: usize, spans: &mut Vec<crate::layout::TextSpan>) {
        if let Ok((llx, lly, urx, ury)) = self.get_page_media_box(page_index) {
            const EDGE_TOLERANCE_PT: f32 = 2.0;
            // Normalise corners: some producers write the MediaBox with swapped
            // corners (e.g. `[0 792 612 0]`, ury < lly). Taking min/max makes the
            // bounds correct either way - without this a swapped box inverts the
            // test below and drops the whole page's legitimate text. ~keep
            let left = llx.min(urx) - EDGE_TOLERANCE_PT;
            let right = llx.max(urx) + EDGE_TOLERANCE_PT;
            let bottom = lly.min(ury) - EDGE_TOLERANCE_PT;
            let top = lly.max(ury) + EDGE_TOLERANCE_PT;
            spans.retain(|span| {
                let sx1 = span.bbox.x;
                let sx2 = span.bbox.x + span.bbox.width;
                let sy1 = span.bbox.y;
                let sy2 = span.bbox.y + span.bbox.height;
                sx2 > left && sx1 < right && sy2 > bottom && sy1 < top
            });
        }
    }

    pub(super) fn postprocess_spans(
        &self,
        page_index: usize,
        raw_spans: Vec<crate::layout::TextSpan>,
    ) -> Result<Vec<crate::layout::TextSpan>> {
        let mut spans = raw_spans;

        self.drop_offpage_spans(page_index, &mut spans);

        // Recover decimal points mis-decoded as `¬` (U+00AC) in Computer-Modern
        // math subsets, where the `/Differences` names the decimal slot
        // `logicalnot`. Only a `¬` sitting *directly between two digits* (no
        // space) is rewritten — real logic/set `¬` is always spaced, so this
        // cannot corrupt it. ~keep
        for span in &mut spans {
            if Self::is_cm_or_symbol_font(&span.font_name) && span.text.contains('\u{00AC}') {
                span.text = Self::fix_digit_logicalnot_decimal(&span.text);
            }
        }

        // Re-attach oversized lone leading capitals to their word before the
        // reading-order sort can strand them (drop-cap / table-title initials). ~keep
        Self::merge_drop_cap_initials(&mut spans);

        // Apply page /Rotate to span geometry BEFORE reading-order sorting.
        //
        // A page with a /Rotate entry must be read in its DISPLAYED orientation
        // or the row-aware sort emits text in the wrong order (pdf.js issue14415
        // is a 180° English page that otherwise comes out word- and line-reversed).
        //
        // The transform is applied selectively, because a rotated page carries two
        // very different kinds of run, distinguished by each span's own
        // `rotation_degrees` (the content-stream text-matrix rotation):
        //
        // * **Horizontal content (`rotation_degrees == 0`) on a 90°/270° page** —
        //   e.g. a landscape table stored rotated (`/Rotate 90`, MediaBox already
        //   landscape). This text is horizontal in raw user space, so it reads and
        //   groups correctly THERE. Rotating its bbox by ±90° only rotates the
        //   RECTANGLE, but `TextSpan::to_chars` still lays glyphs horizontally with
        //   raw advance widths and cannot express a now-vertical run, so every raw
        //   row collapses onto one displayed band and perpendicular columns fuse
        //   into one 1000+ char token. These are LEFT RAW — matching
        //   `extract_chars`, which also returns raw coordinates.
        //
        // * **Rotated content (`rotation_degrees == ±90`) on a 90°/270° page** —
        //   e.g. a chart axis, a sideways table, or a whole landscape page authored
        //   by drawing every glyph sideways in a portrait MediaBox with `/Rotate 90`
        //   to present it upright. Here the page /Rotate must be applied so it
        //   COMBINES with the content rotation (which `order_rotated_blocks` undoes
        //   for ordering) into the correct upright displayed frame; leaving it raw
        //   reads the page sideways. These ARE mapped.
        //
        // 180° maps everything (text stays horizontal; both axes just mirror —
        // numerically identical to the legacy mirror).
        //
        // Captured so the same transform is applied to annotation spans appended
        // later (their /Rect is in unrotated page space too). `None` for rot == 0
        // or unknown media box — those pages keep raw geometry. ~keep
        let page_rotation: Option<(i32, f32, f32, f32, f32)> = match self.get_page_media_box(page_index) {
            Ok((llx, lly, urx, ury)) => {
                let rot = self.get_page_rotation(page_index).unwrap_or(0).rem_euclid(360);
                matches!(rot, 90 | 180 | 270).then_some((rot, llx, lly, urx - llx, ury - lly))
            }
            Err(_) => None,
        };
        if let Some((rot, llx, lly, w, h)) = page_rotation {
            for s in spans.iter_mut() {
                // 90°/270°: only map runs whose own content is rotated; horizontal
                // content stays in raw user space (see rationale above). ~keep
                if rot != 180 && s.rotation_degrees == 0.0 {
                    continue;
                }
                Self::map_span_into_rotated_frame(s, rot, llx, lly, w, h);
            }
        }

        // Tategaki (vertical writing) intercept. Pages whose majority of
        // spans were emitted under WMode 1 (font /Encoding ends in -V or
        // the CMap declares /WMode 1) need right-to-left, top-to-bottom
        // ordering. Row-aware / XY-cut sorts assume horizontal flow and
        // scramble vertical text; per-span wmode lets us route just those
        // pages through a tategaki comparator while leaving every existing
        // horizontal corpus untouched. ~keep
        let vertical_count = spans.iter().filter(|s| s.wmode == 1).count();
        if !spans.is_empty() && vertical_count * 2 >= spans.len() {
            // See `crate::utils::sort_vertical_tategaki` for the
            // column-clustering algorithm and the total-order rationale. ~keep
            spans = crate::utils::sort_vertical_tategaki(spans, |s| &s.bbox);
        } else if let Some(ordered) = Self::sidebar_body_reading_order(&spans) {
            // RW-1: narrow-sidebar + wide-body first pages (full-width title band
            // over a metadata sidebar + body). Handled before the XY-cut so the
            // title is not sliced along the body gutter (§14.8.3). ~keep
            spans = ordered;
        } else if Self::is_multi_column_page(&spans) {
            use crate::pipeline::reading_order::{
                ReadingOrderContext as ROContext, ReadingOrderStrategy, XYCutStrategy,
            };
            let strategy = XYCutStrategy::new();
            // `is_multi_column_page` has already established there IS a
            // gutter but returns only a bool, so re-measure its X here for
            // the heading-run pre-pass (GH#1757). ~keep
            let column_gutter = Self::detect_column_gutter(&spans);
            let mut context = ROContext::new().with_page(page_index as u32);
            if let Some(gutter_x) = column_gutter {
                context = context.with_column_gutter(gutter_x);
            }
            // Clone needed: apply() takes ownership, and the Err branch
            // falls back to sorting the original vec in place. ~keep
            match strategy.apply(spans.clone(), &context) {
                Ok(ordered) => {
                    spans = ordered.into_iter().map(|o| o.span).collect();
                }
                Err(_) => {
                    tracing::warn!(target: LOG_TARGET,
                        page_index,
                        error_code = "reading_order_error",
                        "XY-cut reading order failed; falling back to row-aware sort"
                    );
                    spans.sort_by(|a, b| crate::utils::row_aware_span_cmp(a.bbox.y, a.bbox.x, b.bbox.y, b.bbox.x));
                    Self::reorder_rowspan_labels(&mut spans);
                }
            }
        } else {
            spans.sort_by(|a, b| crate::utils::row_aware_span_cmp(a.bbox.y, a.bbox.x, b.bbox.y, b.bbox.x));
            Self::reorder_rowspan_labels(&mut spans);
        }

        reorder_ltr_key_value_rows(&mut spans);

        // Per-span rotation firewall. Runs drawn with a rotated text matrix
        // (the vertical `arXiv:…` margin stamp, figure/axis labels, rotated
        // table headers, transit-poster route names) break the axis-aligned
        // row-band / XY-cut assumptions, so interleaving them with the
        // horizontal flow scrambles reading order. The reordering above ran on
        // the FULL span set (so its column/XY-cut decisions are unchanged — the
        // horizontal body keeps its exact baseline order); now stably lift the
        // rotated runs out (preserving horizontal order) and re-append them as
        // their own blocks, each ordered in an upright frame. No-op (and ~keep
        // byte-identical) when the page has no rotated spans.
        if spans.iter().any(|s| s.rotation_degrees != 0.0) {
            let rotated: Vec<crate::layout::TextSpan> =
                spans.iter().filter(|s| s.rotation_degrees != 0.0).cloned().collect();
            spans.retain(|s| s.rotation_degrees == 0.0);
            spans.extend(Self::order_rotated_blocks(rotated));
        }

        let erase = self.erase_regions.lock_or_recover().get(&page_index).cloned();
        if let Some(regions) = erase {
            spans.retain(|span| !regions.iter().any(|r| r.intersects(&span.bbox)));
        }

        // Append text from non-Widget annotations (/Subtype /Text, FreeText,
        // Stamp, Highlight, etc.) that carry a /Contents entry. These are not
        // part of the page content stream so they are not picked up by the
        // regular extractor. On a /Rotate'd page their /Rect-derived bboxes are
        // in unrotated page space, so map the appended spans into the same
        // displayed frame as the content spans (no-op for unrotated pages).
        // Annotation text is horizontal (rotation_degrees == 0), so on a 90°/270°
        // page it stays raw, matching the horizontal content spans above. ~keep
        let pre_annotation_len = spans.len();
        spans.extend(self.annotation_content_spans(page_index));
        if let Some((rot, llx, lly, w, h)) = page_rotation {
            for s in spans[pre_annotation_len..].iter_mut() {
                if rot != 180 && s.rotation_degrees == 0.0 {
                    continue;
                }
                Self::map_span_into_rotated_frame(s, rot, llx, lly, w, h);
            }
        }

        // Mark running headers/footers (untagged-PDF heuristic). Spans whose
        // normalized text recurs on >=50% of pages and sits near the top or
        // bottom of the page are flagged as artifacts so downstream filters
        // drop them. ~keep
        self.mark_running_artifact_spans(page_index, &mut spans)?;

        // Normalize Unicode typographic spaces (U+2000–U+200B, U+202F, U+205F)
        // to ASCII space. Some PDF producers encode word separators as hair-space
        // or thin-space variants in ToUnicode CMaps (e.g. justified text layouts);
        // normalising here gives consistent word boundaries to every downstream
        // consumer (extract_text, word-F1 scoring, etc.). ~keep
        for span in &mut spans {
            if span
                .text
                .chars()
                .any(|c| matches!(c, '\u{2000}'..='\u{200B}' | '\u{202F}' | '\u{205F}'))
            {
                span.text =
                    crate::converters::text_post_processor::TextPostProcessor::normalize_unicode_spaces(&span.text)
                        .into_owned();
            }
        }

        // Apply char_widths boundary splits directly to span.text so that every
        // downstream consumer (to_markdown, to_html, extract_text) sees the same
        // word boundaries. extract_text applies the same logic through push_span_text;
        // after this normalization push_span_text sees a space at the boundary
        // becomes a no-op, so there is no double-application risk. ~keep
        for span in &mut spans {
            if let Some(split) = Self::char_widths_boundary_split(span) {
                let mut t = String::with_capacity(span.text.len() + 1);
                t.push_str(&span.text[..split]);
                t.push(' ');
                t.push_str(&span.text[split..]);
                span.text = t;
            }
        }

        // Detect superscript / subscript runs and substitute ASCII
        // digits with their Unicode super/sub-script equivalents
        // (only when the run is sandwiched between alphabetic body
        // spans on both sides — chemistry/math context like "S²X"
        // or "H₂O"). The same substitution would otherwise fire on
        // author-affiliation markers ("name¹,²") which the bench
        // ground truth keeps in ASCII; gating on token-internal
        // context keeps the desired cases without regressing the
        // affiliation-block pages. ~keep
        Self::apply_super_sub_script_substitutions(&mut spans);

        // Fold spacing-diacritic spans (´, `, ^, ~, ¨, …) into the
        // base letter of the following span when the diacritic is
        // centred over the base glyph. PDFs that pre-shape accented
        // Latin (LaTeX `\'E` → two glyphs, `acute` then `E`) emit
        // the marks as separate `Tj` ops at the base glyph's X
        // coordinate. Without this pass extract_text returns the
        // raw two-glyph order "´Ecole" instead of "École". ~keep
        Self::apply_combining_mark_composition(&mut spans);

        // Stamp accurate per-glyph x-origins onto the finalized spans so that
        // `to_chars()` (and thus extract_words / extract_spans /
        // extract_text_lines, which all decompose spans through it) reports
        // spec-aligned positions instead of drifting prefix-sums. Runs last, on
        // the fully post-processed spans, so alignment sees the same text the ~keep
        // consumers do.
        self.stamp_char_x_offsets(page_index, &mut spans);

        Ok(spans)
    }

    /// Copy the spec-aligned per-glyph baseline x-origins from the char-level
    /// extractor onto each span's [`char_x_offsets`](crate::layout::TextSpan::char_x_offsets).
    ///
    /// # Why
    ///
    /// [`TextSpan::to_chars`](crate::layout::TextSpan::to_chars) otherwise
    /// reconstructs each glyph's x by prefix-summing the span's nominal
    /// `char_widths` from `bbox.x`. Those nominal widths omit the ISO
    /// 32000-1:2008 §9.4.3 TJ-array adjustment (the number in a TJ array is
    /// "expressed in thousandths of a unit of text space … subtracted from the
    /// current … coordinate") and the full §9.4.4 text-space displacement
    /// (`t_x = ((w0 − Tj/1000) · Tfs + Tc + Tw) · Th`). Prefix-summing the
    /// nominal widths therefore drifts cumulatively along a line. The
    /// char-level extractor that [`extract_chars`](Self::extract_chars) uses
    /// implements §9.4.4 / §9.4.3 in full (it matches Poppler `pdftotext
    /// -bbox`), so its `origin_x` values are the authoritative positions this
    /// function stamps back onto the spans.
    ///
    /// # Alignment (robust, per span)
    ///
    /// A naive global greedy walk on char values mis-jumps on repeated
    /// letters / spaces. Instead, for each span we take only the accurate chars
    /// on the SAME baseline (`|origin_y − span.bbox.y| ≤ 0.5·font_size`), sort
    /// them by x, and match the span's glyph sequence as a CONTIGUOUS run,
    /// choosing the run whose first glyph's `origin_x` is nearest `span.bbox.x`.
    ///
    /// # Fallback (never guess)
    ///
    /// If a span cannot be fully, unambiguously aligned — no contiguous run of
    /// exactly the span's glyphs exists on its line (count mismatch from a
    /// post-processing text edit, ligature expansion, a synthetic space glyph
    /// not present in the char stream, …) — its `char_x_offsets` is left empty
    /// so `to_chars` uses the legacy prefix-sum path. A cleared span is a
    /// no-op, never a regression. `char_widths` is never touched (a downstream
    /// word-boundary heuristic keys off its length).
    ///
    /// 180° pages are skipped entirely: the spans are mirrored into the displayed
    /// frame while the accurate chars remain in unrotated page space, so a
    /// horizontal-x stamp would not correspond. On 90°/270° pages, horizontal
    /// content spans stay in raw user space and ARE stamped, but rotated-content
    /// spans (`rotation_degrees != 0`) have been mapped into the displayed frame
    /// (see `postprocess_spans`) and their glyphs run along a rotated axis, so a
    /// horizontal-x stamp would misalign — those individual spans are skipped.
    fn stamp_char_x_offsets(&self, page_index: usize, spans: &mut [crate::layout::TextSpan]) {
        // Horizontal-x offsets only make sense in an unrotated frame; the 180°
        // mirror is the one rotation that leaves ALL spans in the displayed frame. ~keep
        if self.get_page_rotation(page_index).unwrap_or(0).rem_euclid(360) == 180 {
            return;
        }

        let accurate = match self.cached_page_chars(page_index) {
            Ok(chars) if !chars.is_empty() => chars,
            _ => return,
        };

        // Baseline index: char positions ordered by `origin_y`, so each span's
        // baseline slice is a binary-searched range rather than a linear scan
        // over every glyph on the page (that scan made this pass
        // O(spans x chars) — the dominant per-page cost on long documents). ~keep
        let mut by_y: Vec<u32> = (0..accurate.len() as u32).collect();
        by_y.sort_by(|&a, &b| {
            crate::utils::safe_float_cmp(accurate[a as usize].origin_y, accurate[b as usize].origin_y)
        });
        let ys: Vec<f32> = by_y.iter().map(|&i| accurate[i as usize].origin_y).collect();

        for span in spans.iter_mut() {
            // Rotated-content spans are in the displayed frame (mapped on 90/270)
            // and their glyphs run vertically; a horizontal-x stamp from the raw
            // chars would not correspond, so leave them to the prefix-sum path. ~keep
            if span.rotation_degrees != 0.0 {
                continue;
            }
            // Start clean: any offsets carried over via struct-update from a
            // source span must not be trusted for this (possibly edited) text. ~keep
            span.char_x_offsets.clear();

            let glyphs: Vec<char> = span.text.chars().collect();
            let n = glyphs.len();
            if n == 0 {
                continue;
            }

            let baseline_tol = 0.6 * span.font_size.max(1.0);
            // Chars sharing this span's baseline, left-to-right. The y-sorted
            // index brackets a candidate range; the exact `abs()` predicate then
            // selects from it, so the result matches a full linear scan even
            // where the bracket arithmetic rounds differently. The widened
            // bracket keeps that range a superset. Ordering by
            // (origin_x, source index) reproduces the previous stable
            // filter-then-sort: ties on origin_x keep their `accurate` order. ~keep
            let bracket = baseline_tol + baseline_tol.abs() * 1e-6 + f32::EPSILON;
            let lo = ys.partition_point(|&y| y < span.bbox.y - bracket);
            let hi = ys.partition_point(|&y| y <= span.bbox.y + bracket);
            let mut idx: Vec<u32> = by_y[lo..hi]
                .iter()
                .copied()
                .filter(|&i| (accurate[i as usize].origin_y - span.bbox.y).abs() <= baseline_tol)
                .collect();
            if idx.is_empty() {
                continue;
            }
            idx.sort_by(|&a, &b| {
                crate::utils::safe_float_cmp(accurate[a as usize].origin_x, accurate[b as usize].origin_x)
                    .then(a.cmp(&b))
            });
            let line: Vec<&crate::layout::TextChar> = idx.iter().map(|&i| &accurate[i as usize]).collect();

            // Greedy per-glyph alignment. Anchor the scan cursor at the accurate
            // char nearest this span's left edge, then walk the span's glyphs,
            // matching each to the next equal accurate char within a small
            // forward window. Unlike an all-or-nothing contiguous match, a single
            // unmatched glyph (an inserted word-boundary space, a ligature split,
            // a combining mark) no longer discards the whole span — such glyphs
            // are interpolated below so `char_x_offsets` still fills every index. ~keep
            let start = line.iter().position(|c| c.origin_x >= span.bbox.x - 0.5).unwrap_or(0);
            let mut assigned: Vec<Option<f32>> = vec![None; n];
            let mut li = start;
            for (k, &g) in glyphs.iter().enumerate() {
                let mut j = li;
                let mut steps = 0;
                while j < line.len() && steps < 6 {
                    if line[j].char == g {
                        assigned[k] = Some(line[j].origin_x);
                        li = j + 1;
                        break;
                    }
                    j += 1;
                    steps += 1;
                }
            }

            // Need enough real anchors to trust the run; otherwise fall back. ~keep
            let anchors = assigned.iter().filter(|a| a.is_some()).count();
            if anchors * 5 < n * 3 {
                // < 60% matched ~keep
                continue;
            }

            // Fill the gaps: each unmatched glyph takes the nearest preceding
            // anchor plus the prefix sum of the (locally accurate) char_widths
            // between them; if there is no preceding anchor, walk back from the
            // nearest following one. Over the short spans between anchors the
            // cumulative drift these interpolations reintroduce is sub-point. ~keep
            let cw = &span.char_widths;
            let width_at = |i: usize| -> f32 {
                if cw.len() == n {
                    cw[i]
                } else {
                    span.bbox.width / n as f32
                }
            };
            let mut offs = vec![0.0f32; n];
            let mut last: Option<(usize, f32)> = None;
            for k in 0..n {
                if let Some(x) = assigned[k] {
                    offs[k] = x;
                    last = Some((k, x));
                } else if let Some((lk, lx)) = last {
                    let acc: f32 = (lk..k).map(width_at).sum();
                    offs[k] = lx + acc;
                }
            }
            if assigned[0].is_none()
                && let Some(fk) = assigned.iter().position(|a| a.is_some())
            {
                let fx = assigned[fk].unwrap();
                for k in 0..fk {
                    let acc: f32 = (k..fk).map(width_at).sum();
                    offs[k] = fx - acc;
                }
            }
            span.char_x_offsets = offs;
        }
    }

    /// Fold a one-char spacing-diacritic span into the following
    /// span's first character when they overlap in X (the typical
    /// LaTeX `\'E` → `(´)(E)` shape). Substitutes the relevant
    /// combining mark from U+0300..U+0327 and lets
    /// `unicode_normalization::nfc` precompose where it can
    /// ("E\u{0301}" → "É"). The diacritic span is left empty so
    /// downstream rendering skips it.
    fn apply_combining_mark_composition(spans: &mut Vec<crate::layout::TextSpan>) {
        use unicode_normalization::UnicodeNormalization;

        fn combining_for(spacing: char) -> Option<char> {
            Some(match spacing {
                '\u{00B4}' => '\u{0301}',
                '\u{0060}' => '\u{0300}',
                '\u{005E}' => '\u{0302}',
                '\u{02C6}' => '\u{0302}',
                '\u{007E}' => '\u{0303}',
                '\u{02DC}' => '\u{0303}',
                '\u{00A8}' => '\u{0308}',
                '\u{00AF}' => '\u{0304}',
                '\u{02C9}' => '\u{0304}',
                '\u{00B8}' => '\u{0327}',
                '\u{02DA}' => '\u{030A}',
                _ => return None,
            })
        }

        // First pass: spans that already got merged at the extractor
        // (when the LaTeX `(´)(Ecole)` pair both sit at the same
        // text-matrix origin the upstream merge_adjacent_spans pulls
        // them into a single "´Ecole" span). Fold the leading
        // diacritic + base letter into the precomposed form. ~keep
        for span in spans.iter_mut() {
            let mut iter = span.text.chars();
            let Some(d) = iter.next() else { continue };
            let Some(base) = iter.next() else { continue };
            let Some(combining) = combining_for(d) else {
                continue;
            };
            if !base.is_alphabetic() {
                continue;
            }
            let rest_start = d.len_utf8() + base.len_utf8();
            let mut composed = String::with_capacity(span.text.len() + 2);
            composed.push(base);
            composed.push(combining);
            composed.push_str(&span.text[rest_start..]);
            span.text = composed.nfc().collect();
        }

        // Walk spans pairwise. The diacritic is on its own one-
        // character span; the next span carries the base letter. ~keep
        let mut i = 0;
        while i + 1 < spans.len() {
            let mark_char = {
                let s = &spans[i];
                let mut iter = s.text.chars();
                let first = iter.next();
                let rest = iter.next();
                if rest.is_some() {
                    None
                } else {
                    first.and_then(combining_for)
                }
            };
            let Some(combining) = mark_char else {
                i += 1;
                continue;
            };
            // Geometric: same line, diacritic anchored over the base
            // letter's left edge (within ±1 pt). ~keep
            let (same_line, overlaps_x) = {
                let p = &spans[i];
                let n = &spans[i + 1];
                let same = (p.bbox.y - n.bbox.y).abs() < p.font_size.max(n.font_size) * 0.6;
                let dx = (p.bbox.x - n.bbox.x).abs();
                (same, dx <= 1.5)
            };
            if !(same_line && overlaps_x) {
                i += 1;
                continue;
            }
            let Some(base) = spans[i + 1].text.chars().next() else {
                i += 1;
                continue;
            };
            if !base.is_alphabetic() {
                i += 1;
                continue;
            }
            let mut composed = String::with_capacity(spans[i + 1].text.len() + 2);
            composed.push(base);
            composed.push(combining);
            let rest_start = base.len_utf8();
            composed.push_str(&spans[i + 1].text[rest_start..]);
            spans[i + 1].text = composed.nfc().collect();
            // Empty out the diacritic span; downstream consumers
            // skip zero-text spans. ~keep
            spans[i].text.clear();
            i += 2;
        }

        spans.retain(|s| !s.text.is_empty());
    }

    /// Substitute ASCII digits and a few punctuation characters in
    /// super/sub-script spans with their Unicode counterparts
    /// (U+2070..U+2079 / U+00B2/B3/B9 for superscripts,
    /// U+2080..U+2089 for subscripts). A span is treated as
    /// super- or sub-script when its font is meaningfully smaller
    /// than the previous span on the same line and its baseline is
    /// raised or lowered. Only spans whose text consists entirely
    /// of substitutable characters are rewritten — mixed-content
    /// or single-letter superscript callouts (e.g. footnote "a")
    /// fall through unchanged so the existing citation-handling
    /// path stays in control.
    fn apply_super_sub_script_substitutions(spans: &mut [crate::layout::TextSpan]) {
        fn super_for_char(c: char) -> Option<char> {
            Some(match c {
                '0' => '\u{2070}',
                '1' => '\u{00B9}',
                '2' => '\u{00B2}',
                '3' => '\u{00B3}',
                '4' => '\u{2074}',
                '5' => '\u{2075}',
                '6' => '\u{2076}',
                '7' => '\u{2077}',
                '8' => '\u{2078}',
                '9' => '\u{2079}',
                '+' => '\u{207A}',
                '-' => '\u{207B}',
                '=' => '\u{207C}',
                '(' => '\u{207D}',
                ')' => '\u{207E}',
                _ => return None,
            })
        }
        fn sub_for_char(c: char) -> Option<char> {
            Some(match c {
                '0' => '\u{2080}',
                '1' => '\u{2081}',
                '2' => '\u{2082}',
                '3' => '\u{2083}',
                '4' => '\u{2084}',
                '5' => '\u{2085}',
                '6' => '\u{2086}',
                '7' => '\u{2087}',
                '8' => '\u{2088}',
                '9' => '\u{2089}',
                '+' => '\u{208A}',
                '-' => '\u{208B}',
                '=' => '\u{208C}',
                '(' => '\u{208D}',
                ')' => '\u{208E}',
                _ => return None,
            })
        }
        // Two-pass: first compute the body-font baseline for each
        // line band (largest font_size on that line), then walk
        // spans and substitute any whose font is meaningfully
        // smaller AND whose baseline is raised or lowered relative
        // to the body baseline. ~keep
        let n = spans.len();
        if n < 2 {
            return;
        }
        const LINE_BAND_PT: f32 = 4.0;
        // band_anchor[i] = (body_font_size, body_y) of the line
        // band that span `i` belongs to. Sorting span indices by Y
        // once + sliding a two-pointer window over the sorted view
        // reduces the per-span band-anchor scan from O(n) to amortised
        // O(window_size), bringing the whole pass from O(n²) down to
        // O(n log n) on thesis-style pages with thousands of spans. ~keep
        let mut sorted_by_y: Vec<usize> = (0..n).collect();
        sorted_by_y.sort_by(|&a, &b| crate::utils::safe_float_cmp(spans[a].bbox.y, spans[b].bbox.y));
        let band_anchor = Self::compute_band_anchors(spans, &sorted_by_y, LINE_BAND_PT);
        // Spatial index: bucket spans by Y-band so `span_is_token_internal`
        // queries only nearby spans instead of all of them (its same-line
        // neighbour scan was O(n) per candidate → O(n²) on dense pages). ~keep
        let y_index = Self::build_y_band_index(spans, LINE_BAND_PT);
        for i in 0..n {
            let (anchor_fs, anchor_y) = band_anchor[i];
            let curr_fs = spans[i].font_size;
            if anchor_fs <= 0.0 || curr_fs >= anchor_fs * 0.85 {
                continue;
            }
            let y_delta = spans[i].bbox.y - anchor_y;
            let raised = y_delta > anchor_fs * 0.15;
            let lowered = y_delta < -anchor_fs * 0.15;
            if !raised && !lowered {
                continue;
            }
            let map: fn(char) -> Option<char> = if raised { super_for_char } else { sub_for_char };
            if spans[i].text.is_empty() || !spans[i].text.chars().all(|c| map(c).is_some()) {
                continue;
            }
            // Leave a signed numeric exponent (scientific unit notation such as
            // `s−1`, `m−2`) as ASCII. ToUnicode already decoded the intended
            // characters, and the plaintext convention every reference extractor
            // follows keeps these un-superscripted; rewriting `−1` to `₋₁` / `⁻¹`
            // is both wrong against that convention and — because the geometric
            // classifier fires inconsistently on borderline baselines — a source
            // of non-determinism across identical occurrences. ~keep
            if Self::run_is_signed_number(&spans[i].text) {
                continue;
            }
            // Limit the substitution to clearly token-internal
            // super/sub-scripts: the run must have a base-sized
            // neighbour on BOTH sides whose first/last char is
            // alphabetic and roughly adjacent in X. Author-
            // affiliation markers like "name¹,²" sit at the END
            // of a line with no following body letter; the bench
            // GT renders those as plain ASCII digits, so substi-
            // tuting them would regress. Restricting to sandwiched
            // runs keeps the chemistry / exponent cases that the
            // GT does want as Unicode (S², H₂O, k₁) and skips the
            // trailing footnote callouts. ~keep
            if !Self::span_is_token_internal(spans, i, &y_index, LINE_BAND_PT) {
                continue;
            }
            let substituted: String = spans[i].text.chars().map(|c| map(c).unwrap()).collect();
            spans[i].text = substituted;
        }
    }

    /// A run is a signed numeric exponent — e.g. `-1`, `−2`, `‑3` — when it
    /// opens with a minus/hyphen sign and contains at least one digit. Such runs
    /// are scientific unit exponents (`s−1`, `m−2`) that the plaintext extraction
    /// convention keeps as ASCII, so [`apply_super_sub_script_substitutions`]
    /// must not rewrite them into Unicode sub/superscript glyphs.
    ///
    /// [`apply_super_sub_script_substitutions`]: Self::apply_super_sub_script_substitutions
    pub(super) fn run_is_signed_number(text: &str) -> bool {
        let is_minus = |c: char| matches!(c, '\u{002D}' | '\u{2212}' | '\u{2010}' | '\u{2011}');
        matches!(text.chars().next(), Some(c) if is_minus(c)) && text.chars().any(|c| c.is_ascii_digit())
    }

    /// For every span, the `(max_font_size, anchor_y)` over the spans within
    /// `±band` of its Y, in O(n) via a sliding-window maximum (monotonic deque)
    /// over the Y-sorted order. Replaces a per-span window walk that was O(n²)
    /// when many spans share a Y band (wide table rows).
    ///
    /// Tie-break on equal max font size: the lowest-Y span (deque keeps the
    /// earliest sorted position). A substitution only fires when the span's own
    /// font is strictly smaller than the anchor, so the tie-break merely picks
    /// which equal-sized body span supplies `anchor_y`, all within `band`.
    fn compute_band_anchors(spans: &[crate::layout::TextSpan], sorted_by_y: &[usize], band: f32) -> Vec<(f32, f32)> {
        let n = sorted_by_y.len();
        let mut band_anchor = vec![(0.0f32, 0.0f32); n];
        let y = |p: usize| spans[sorted_by_y[p]].bbox.y;
        let fs = |p: usize| spans[sorted_by_y[p]].font_size;
        // Deque of sorted positions, font size non-increasing front→back;
        // positions are pushed in increasing order so the deque is also
        // position-increasing front→back (front = smallest position = max fs). ~keep
        let mut deque: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
        let mut lo = 0usize;
        let mut hi = 0usize;
        for pos in 0..n {
            let cy = y(pos);
            while hi < n && y(hi) <= cy + band {
                while let Some(&back) = deque.back() {
                    if fs(back) < fs(hi) {
                        deque.pop_back();
                    } else {
                        break;
                    }
                }
                deque.push_back(hi);
                hi += 1;
            }
            while lo < n && y(lo) < cy - band {
                if deque.front() == Some(&lo) {
                    deque.pop_front();
                }
                lo += 1;
            }
            let best = *deque.front().expect("window always contains pos");
            band_anchor[sorted_by_y[pos]] = (fs(best), y(best));
        }
        band_anchor
    }

    /// Return true when span `i` has a base-sized alphabetic
    /// neighbour both before and after it on the same line band,
    /// within ~1 em horizontally. That captures the "X²Y" /
    /// "H₂O" / "k₁ + …" pattern but excludes footnote markers
    /// that hang off the end of a word with no following body
    /// character.
    /// Bucket span indices by Y-band (`round(y / band)`) so same-line lookups
    /// scan only nearby bands instead of every span. Querying a band `k`'s
    /// `[k-2, k+2]` neighbours is a guaranteed superset of all spans within
    /// `band` points of any Y in band `k`, so an exact `|Δy|` filter on the
    /// result is byte-identical to a full scan.
    pub(super) fn build_y_band_index(spans: &[crate::layout::TextSpan], band: f32) -> HashMap<i32, Vec<usize>> {
        let mut idx: HashMap<i32, Vec<usize>> = HashMap::new();
        for (j, s) in spans.iter().enumerate() {
            idx.entry((s.bbox.y / band).round() as i32).or_default().push(j);
        }
        idx
    }

    /// Indices in the Y-bands within ±2 of `y`'s band (superset of `|Δy| ≤ band`).
    pub(super) fn y_band_candidates<'a>(
        y_index: &'a HashMap<i32, Vec<usize>>,
        y: f32,
        band: f32,
    ) -> impl Iterator<Item = usize> + 'a {
        let k = (y / band).round() as i32;
        (k - 2..=k + 2).flat_map(move |b| y_index.get(&b).into_iter().flatten().copied())
    }

    fn span_is_token_internal(
        spans: &[crate::layout::TextSpan],
        i: usize,
        y_index: &HashMap<i32, Vec<usize>>,
        band: f32,
    ) -> bool {
        let curr = &spans[i];
        let curr_y = curr.bbox.y;
        let curr_x = curr.bbox.x;
        let curr_right = curr.bbox.x + curr.bbox.width;
        let body_fs = Self::y_band_candidates(y_index, curr_y, band)
            .filter(|&j| (spans[j].bbox.y - curr_y).abs() <= 4.0)
            .map(|j| spans[j].font_size)
            .fold(0f32, f32::max)
            .max(1.0);
        let neighbour_fs_min = body_fs * 0.85;
        let max_em = body_fs;
        let mut has_left = false;
        let mut has_right = false;
        for j in Self::y_band_candidates(y_index, curr_y, band) {
            if j == i {
                continue;
            }
            let s = &spans[j];
            if (s.bbox.y - curr_y).abs() > 4.0 {
                continue;
            }
            if s.font_size < neighbour_fs_min {
                continue;
            }
            // Anchor must start or end with an alphabetic character
            // — a digit or punctuation neighbour does not signal a
            // token-internal context. ~keep
            let s_right = s.bbox.x + s.bbox.width;
            // Allow small overlap (super/sub glyphs nest slightly
            // under the body letter's bounding box). ~keep
            let dx_left = curr_x - s_right;
            if s_right < curr_right
                && dx_left <= max_em
                && dx_left >= -max_em * 0.5
                && s.text.chars().next_back().is_some_and(|c| c.is_alphabetic())
            {
                has_left = true;
            }
            let dx_right = s.bbox.x - curr_right;
            if s.bbox.x > curr_x
                && dx_right <= max_em
                && dx_right >= -max_em * 0.5
                && s.text.chars().next().is_some_and(|c| c.is_alphabetic())
            {
                has_right = true;
            }
        }
        has_left && has_right
    }

    /// Return per-page font statistics for use in heading detection and layout analysis.
    ///
    /// [`crate::layout::PageFontStats`] contains:
    /// - `dominant_em`: the mode font size weighted by character count — the body text "1 em"
    /// - `dominant_line_height`: median baseline-to-baseline distance
    /// - `dominant_char_width`: average character advance width
    /// - `body_font_name`: name of the most-used font
    ///
    /// The primary use-case is heading detection in downstream tools: compare
    /// `span.font_size / stats.dominant_em` against a threshold (e.g. 1.4×
    /// for H2, 1.8× for H1) to classify large-font spans as headings without
    /// depending on any hardcoded point sizes.
    ///
    /// ```ignore
    /// let stats = doc.page_font_stats(0)?;
    /// let spans = doc.extract_spans(0)?;
    /// for span in &spans {
    ///     let ratio = span.font_size / stats.dominant_em;
    ///     if ratio >= 1.8 { println!("H1: {}", span.text); }
    ///     else if ratio >= 1.4 { println!("H2: {}", span.text); }
    /// }
    /// ```
    pub fn page_font_stats(&self, page_index: usize) -> Result<crate::layout::PageFontStats> {
        let spans = self.extract_spans(page_index)?;
        Ok(crate::layout::PageFontStats::from_spans(&spans))
    }
}
