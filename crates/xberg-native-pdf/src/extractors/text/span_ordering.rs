//! Span deduplication, column detection, and reading-order sorting.
//!
//! Split out of the parent's single 5,806-line `impl TextExtractor`, which made
//! `extractors/text.rs` 673 KiB — over the repository's 500 KiB file-safety limit.
//! A child module's `impl` is the same inherent impl and sees the parent's private
//! items unchanged. ~keep

use super::*;

impl<'doc> TextExtractor<'doc> {
    pub(super) fn sort_spans_preserving_script_geometry(&mut self) {
        // Cached Form XObjects can reuse source sequence numbers. Use temporary
        // indices during sorting, then restore source IDs and raw baselines before
        // deduplication or merging can consume their geometry (GH#2275). ~keep
        let original_geometry = self.preserve_script_baselines.then(|| {
            self.spans
                .iter_mut()
                .enumerate()
                .map(|(index, span)| {
                    let original = (span.sequence, span.bbox.y);
                    span.sequence = index;
                    original
                })
                .collect::<Vec<_>>()
        });
        self.snap_superscript_baselines();
        self.sort_spans_by_reading_order();
        if let Some(original_geometry) = original_geometry {
            for span in &mut self.spans {
                let (sequence, baseline) = original_geometry[span.sequence];
                span.sequence = sequence;
                span.bbox.y = baseline;
            }
        }
    }

    /// Deduplicate overlapping characters on the same line.
    ///
    /// Some PDFs render text multiple times at slightly different X positions
    /// (e.g., for bold effect or shadowing). This causes garbled text output when
    /// all renders are extracted. We keep only one character when multiple chars
    /// at nearly the same position exist.
    ///
    /// Heuristic: If two consecutive characters on the same line (Y rounded to
    /// integer) overlap by a fraction of their own advance width, keep only the
    /// first one.
    ///
    /// The threshold is expressed as a fraction of the glyph's `advance_width`
    /// (see [`Self::DEDUP_OVERLAP_RATIO`]) rather than an absolute point
    /// value. Real rendering duplicates (stroke+fill, bold shadow,
    /// outline+fill) sit at nearly identical positions — well under 30 % of
    /// one advance apart. Legitimate adjacent doublets of narrow glyphs
    /// (`ll`, `rr`, `II`, `ii` at small font sizes) are separated by one
    /// full advance; an absolute threshold of e.g. 2 pt would wrongly
    /// collapse them on fonts where a narrow glyph's advance drops below
    /// ~2 pt (e.g. Helvetica at ≤ 9 pt).
    ///
    /// Capped at [`Self::DEDUP_OVERLAP_CAP_PT`] to preserve the existing
    /// behaviour for pathologically oversized advance values, and falls
    /// back to `bbox.width` when `advance_width` is missing from the font
    /// dictionary.
    pub(super) fn deduplicate_overlapping_chars(&mut self) {
        if self.chars.is_empty() {
            return;
        }

        let before = self.chars.len();
        let mut prev_y_rounded: Option<i32> = None;
        let mut prev_x: Option<f32> = None;
        let mut prev_char: Option<char> = None;

        // Retained in place: the predicate only looks back at the previously
        // KEPT glyph, which `retain`'s in-order visit preserves. Building a
        // second Vec instead deep-cloned every glyph — and `TextChar` owns a
        // `font_name` String, so that was one malloc per glyph, per page. ~keep
        self.chars.retain(|ch| {
            let y_rounded = ch.bbox.y.round() as i32;
            let x = ch.bbox.x;

            let should_skip =
                if let (Some(prev_y), Some(prev_x_val), Some(prev_ch)) = (prev_y_rounded, prev_x, prev_char) {
                    let ref_width = if ch.advance_width > 0.0 {
                        ch.advance_width
                    } else if ch.bbox.width > 0.0 {
                        ch.bbox.width
                    } else {
                        Self::DEDUP_OVERLAP_CAP_PT
                    };
                    let threshold = (ref_width * Self::DEDUP_OVERLAP_RATIO).min(Self::DEDUP_OVERLAP_CAP_PT);
                    ch.char == prev_ch && y_rounded == prev_y && (x - prev_x_val).abs() < threshold
                } else {
                    false
                };

            if !should_skip {
                prev_y_rounded = Some(y_rounded);
                prev_x = Some(x);
                prev_char = Some(ch.char);
            } else {
                tracing::trace!(target: LOG_TARGET,
                    "Deduplicating overlapping char '{}' at X={:.1}, Y={:.1} (too close to previous)",
                    ch.char,
                    x,
                    ch.bbox.y
                );
            }
            !should_skip
        });

        tracing::debug!(target: LOG_TARGET,
            "Deduplicated {} overlapping characters ({} -> {} chars)",
            before - self.chars.len(),
            before,
            self.chars.len()
        );
    }

    /// Snap super/subscript glyph spans onto the baseline of an
    /// adjacent base span so downstream row-aware sorting keeps
    /// them inline.
    ///
    /// PDF §9.3.7 defines text rise (`Ts`) as a per-text-state
    /// vertical offset added to the rendering position; the
    /// resulting glyphs sit above (super) or below (sub) the
    /// surrounding baseline. The raw extracted bbox preserves
    /// that offset, so sorting by Y descending interprets a
    /// superscript line of affiliation markers (`1,2 ★ 3,4 …`)
    /// as a row that precedes the author names that they actually
    /// annotate. Snapping each candidate's Y to the matched base
    /// puts them back in the same Y-band.
    ///
    /// A span is a snap candidate when:
    /// - its font_size is < 85 % of a nearby larger-font span,
    /// - its Y is above that base by ≤ 50 % of the base's font_size
    ///   (or below it by the same — covers subscript too), and
    /// - its X falls between the base's right edge and one base
    ///   font_size beyond (the position a superscript would
    ///   appear when typeset directly after the base).
    pub(super) fn snap_superscript_baselines(&mut self) {
        let n = self.spans.len();
        if n < 2 {
            return;
        }

        // Snapshot the read-side fields we need so the borrow checker
        // lets us mutate `self.spans[i].bbox.y` inside the loop. ~keep
        let snapshot: Vec<(f32, f32, f32, f32)> = self
            .spans
            .iter()
            .map(|s| (s.bbox.x, s.bbox.y, s.bbox.width, s.font_size))
            .collect();

        // A valid base candidate `j` always has `y_offset = sy - by` in
        // `[0, bfs*0.5]` (see the gates below), so `by` lies in
        // `[sy - bfs*0.5, sy] ⊆ [sy - max_fs*0.5, sy]`. Sort span indices by
        // Y once and, per candidate, binary-search that Y-window instead of
        // rescanning all spans — this turns the previous O(n²) double loop
        // (which hung for >30 s on archive.org / Google-Books pages whose
        // invisible hOCR layer emits thousands of spans) into roughly
        // O(n log n + n·window). The window is a strict superset of the
        // acceptable bases, so the result is identical to the full scan. ~keep
        let max_fs = snapshot.iter().map(|s| s.3).fold(0.0f32, f32::max);
        let max_half_em = max_fs * 0.5;
        let mut by_order: Vec<usize> = (0..n).collect();
        by_order.sort_by(|&a, &b| crate::utils::safe_float_cmp(snapshot[a].1, snapshot[b].1));
        let ys_sorted: Vec<f32> = by_order.iter().map(|&idx| snapshot[idx].1).collect();

        let search = SnapBaseSearch {
            snapshot: &snapshot,
            by_order: &by_order,
            ys_sorted: &ys_sorted,
            max_half_em,
        };
        for i in 0..n {
            if snapshot[i].3 <= 0.0 {
                continue;
            }
            if let Some(by) = search.base_y_for(i) {
                self.spans[i].bbox.y = by;
            }
        }
    }

    /// Sort extracted text spans by reading order (top-to-bottom, left-to-right).
    pub(super) fn sort_spans_by_reading_order(&mut self) {
        if self.spans.is_empty() {
            return;
        }

        // Vertical-mode (tategaki) routing. Each span carries the writing
        // mode it was emitted under (`wmode == 1` for vertical text). When
        // the page is *predominantly* vertical we apply column-aware
        // top-to-bottom + right-to-left ordering. When the page is
        // predominantly horizontal we fall through to the existing
        // horizontal sort; the rare mixed-mode case stays governed by the
        // dominant mode here. Per-span wmode is preserved on every span
        // either way, so downstream consumers (export, search) can still
        // distinguish them. ~keep
        let vertical_count = self.spans.iter().filter(|s| s.wmode == 1).count();
        let total = self.spans.len();
        if total > 0 && vertical_count * 2 >= total {
            tracing::debug!(target: LOG_TARGET,
                "Reading order: {}/{} spans are vertical — using tategaki sort",
                vertical_count,
                total
            );
            self.sort_spans_vertical_tategaki();
            return;
        }

        let columns = self.detect_span_columns();

        tracing::debug!(target: LOG_TARGET,
            "Column detection: found {} columns from {} spans",
            columns.len(),
            self.spans.len()
        );
        for (i, (left, right)) in columns.iter().enumerate() {
            tracing::trace!(target: LOG_TARGET,
                "  Column {}: X range [{:.1}, {:.1}] (width: {:.1})",
                i,
                left,
                right,
                right - left
            );
        }

        if columns.len() <= 1 {
            tracing::debug!(target: LOG_TARGET, "Using simple Y-then-X sorting (single column)");
            self.simple_sort_spans();
        } else {
            tracing::debug!(target: LOG_TARGET, "Using column-aware sorting ({} columns)", columns.len());
            self.sort_spans_by_columns(&columns);
        }
    }

    /// Sort spans in vertical writing (tategaki) order: right-to-left
    /// across columns, top-to-bottom within each column. See
    /// `crate::utils::sort_vertical_tategaki` for the column-clustering
    /// algorithm and the total-order rationale.
    fn sort_spans_vertical_tategaki(&mut self) {
        self.spans = crate::utils::sort_vertical_tategaki(std::mem::take(&mut self.spans), |s| &s.bbox);
    }

    /// Simple Y-then-X sorting for single-column layouts.
    fn simple_sort_spans(&mut self) {
        self.spans.sort_by(|a, b| {
            let a_y_rounded = a.bbox.y.round() as i32;
            let b_y_rounded = b.bbox.y.round() as i32;

            match b_y_rounded.cmp(&a_y_rounded) {
                std::cmp::Ordering::Equal => crate::utils::safe_float_cmp(a.bbox.x, b.bbox.x),
                other => other,
            }
        });
    }

    /// Detect columns by analyzing X-coordinate distribution.
    ///
    /// Returns column boundaries as (left_x, right_x) pairs, sorted left-to-right.
    pub(super) fn detect_span_columns(&self) -> Vec<(f32, f32)> {
        if self.spans.is_empty() {
            return vec![];
        }

        let min_x = self.spans.iter().map(|s| s.bbox.x).fold(f32::INFINITY, f32::min);
        let max_x = self
            .spans
            .iter()
            .map(|s| s.bbox.x + s.bbox.width)
            .fold(f32::NEG_INFINITY, f32::max);

        let page_width = max_x - min_x;

        let bins = 100;
        let bin_width = page_width / bins as f32;
        let mut histogram = vec![0; bins];

        for span in &self.spans {
            let start_bin = ((span.bbox.x - min_x) / bin_width) as usize;
            let end_bin = ((span.bbox.x + span.bbox.width - min_x) / bin_width) as usize;

            for i in start_bin..=end_bin.min(bins - 1) {
                histogram[i] += 1;
            }
        }

        let avg_density: f32 = histogram.iter().sum::<i32>() as f32 / bins as f32;
        let gap_threshold = (avg_density * 0.2).max(1.0); // 20% of average or at least 1 ~keep

        let mut gaps = vec![];
        let mut in_gap = false;
        let mut gap_start = 0;

        for (i, &count) in histogram.iter().enumerate() {
            if count as f32 <= gap_threshold {
                if !in_gap {
                    gap_start = i;
                    in_gap = true;
                }
            } else if in_gap {
                // End of gap - record if significant
                // Use 2% of page width OR absolute 15pt minimum (catches narrow column gutters)
                // ~keep
                let gap_width = (i - gap_start) as f32 * bin_width;
                if gap_width > (page_width * 0.02).max(15.0) {
                    let gap_x = min_x + gap_start as f32 * bin_width;
                    gaps.push(gap_x);
                }
                in_gap = false;
            }
        }

        if gaps.is_empty() {
            return vec![(min_x, max_x)];
        }

        let mut columns = vec![];
        let mut left = min_x;

        for gap_x in gaps {
            columns.push((left, gap_x));
            left = gap_x;
        }
        columns.push((left, max_x));

        tracing::debug!(target: LOG_TARGET, "Detected {} columns: {:?}", columns.len(), columns);

        columns
    }

    /// Sort spans by column-aware reading order.
    ///
    /// Process columns left-to-right, and within each column, top-to-bottom.
    pub(super) fn sort_spans_by_columns(&mut self, columns: &[(f32, f32)]) {
        let mut column_spans: Vec<Vec<TextSpan>> = vec![vec![]; columns.len()];

        for span in self.spans.drain(..) {
            let span_center_x = span.bbox.x + span.bbox.width / 2.0;

            let col_idx = columns
                .iter()
                .position(|&(left, right)| span_center_x >= left && span_center_x <= right)
                .unwrap_or(0);

            column_spans[col_idx].push(span);
        }

        for col_spans in &mut column_spans {
            col_spans.sort_by(|a, b| {
                let a_y_rounded = a.bbox.y.round() as i32;
                let b_y_rounded = b.bbox.y.round() as i32;

                match b_y_rounded.cmp(&a_y_rounded) {
                    std::cmp::Ordering::Equal => crate::utils::safe_float_cmp(a.bbox.x, b.bbox.x),
                    other => other,
                }
            });
        }

        for col_spans in column_spans {
            self.spans.extend(col_spans);
        }
    }

    /// Deduplicate overlapping text spans on the same line.
    ///
    /// Uses hybrid geometric + content-based deduplication:
    /// - Geometric check (same Y, X within a fraction of the span's per-glyph
    ///   advance) — catches identical positions
    /// - Content check (same text, same line Y, different X) — catches
    ///   duplicates across columns
    ///
    /// The geometric threshold is expressed as a fraction of the span's
    /// per-glyph width (bbox.width / char_count), capped by
    /// [`Self::DEDUP_OVERLAP_CAP_PT`] and scaled by
    /// [`Self::DEDUP_OVERLAP_RATIO`]. An absolute threshold would wrongly
    /// collapse legitimate single-glyph spans of adjacent narrow glyphs
    /// (`ll`, `rr`, `II`, `ii` at small font sizes) in PDFs that emit text
    /// glyph-by-glyph with kerning.
    ///
    /// Geometric check — require BOTH position AND text to match. Split out
    /// of `deduplicate_overlapping_spans` unchanged, for file size. ~keep
    fn is_geometric_duplicate_span(
        span: &TextSpan,
        y_rounded: i32,
        x: f32,
        prev_y_rounded: Option<i32>,
        prev_x: Option<f32>,
        prev_text: Option<&str>,
    ) -> bool {
        let (Some(prev_y), Some(prev_x_val), Some(prev_txt)) = (prev_y_rounded, prev_x, prev_text) else {
            return false;
        };
        let char_count = span.text.chars().count().max(1) as f32;
        let per_glyph_width = (span.bbox.width / char_count).max(0.1);
        let threshold = (per_glyph_width * Self::DEDUP_OVERLAP_RATIO).min(Self::DEDUP_OVERLAP_CAP_PT);
        y_rounded == prev_y && (x - prev_x_val).abs() < threshold && span.text == prev_txt
    }

    /// Content-based deduplication — require positions to OVERLAP. Split out
    /// of `deduplicate_overlapping_spans` unchanged, for file size. ~keep
    fn is_content_duplicate_span(
        span: &TextSpan,
        seen_content: &std::collections::HashMap<String, (f32, f32)>,
    ) -> bool {
        if span.text.len() < 5 {
            return false;
        }
        let Some((prev_x_val, prev_y_val)) = seen_content.get(&span.text) else {
            return false;
        };
        let y_diff = (span.bbox.y - prev_y_val).abs();
        let x_diff = (span.bbox.x - prev_x_val).abs();

        // Only dedup when spans overlap geometrically (X within 5pt)
        // NOT when they're at different positions on the same line ~keep
        let same_line = y_diff < 2.0;
        let overlapping_position = x_diff < 5.0;

        same_line && overlapping_position
    }

    pub(super) fn deduplicate_overlapping_spans(&mut self) {
        if self.spans.is_empty() {
            return;
        }

        // Same-text overlapping spans from stroke+fill render
        // passes. Maps (newspaper / poster) frequently draw every label
        // twice — once stroked for outline, once filled — and both passes
        // land at essentially the same CTM. Without this up-front filter,
        // the merge step later concatenates them into "EverestEverest" /
        // "CentralCentral". We bucket by lowercased text and compare each
        // new span's bbox against prior entries via IoU; any later span
        // whose bbox overlaps an earlier one by >= 70 % is dropped. ~keep
        self.dedup_stroke_fill_overlap();

        let old_len = self.spans.len();
        let spans = std::mem::take(&mut self.spans);
        let mut deduplicated = Vec::with_capacity(old_len);
        let mut prev_y_rounded: Option<i32> = None;
        let mut prev_x: Option<f32> = None;
        let mut prev_text: Option<String> = None;
        let mut seen_content: std::collections::HashMap<String, (f32, f32)> = std::collections::HashMap::new();

        let mut geometric_skips = 0;
        let mut content_skips = 0;

        for span in spans {
            let y_rounded = span.bbox.y.round() as i32;
            let x = span.bbox.x;

            let geometric_duplicate =
                Self::is_geometric_duplicate_span(&span, y_rounded, x, prev_y_rounded, prev_x, prev_text.as_deref());
            let content_duplicate = Self::is_content_duplicate_span(&span, &seen_content);

            if geometric_duplicate {
                geometric_skips += 1;
            } else if content_duplicate {
                content_skips += 1;
            } else {
                prev_y_rounded = Some(y_rounded);
                prev_x = Some(x);
                prev_text = Some(span.text.clone());

                if span.text.len() >= 5 {
                    seen_content.insert(span.text.clone(), (span.bbox.x, span.bbox.y));
                }
                deduplicated.push(span);
            }
        }

        tracing::debug!(target: LOG_TARGET,
            "Deduplicated {} spans (geometric: {}, content: {}) ({} -> {} spans)",
            geometric_skips + content_skips,
            geometric_skips,
            content_skips,
            old_len,
            deduplicated.len()
        );

        self.spans = deduplicated;
    }

    /// Drop same-text spans whose bounding boxes overlap heavily with an
    /// earlier span. This is the canonical stroke+fill pattern on maps,
    /// posters, and marketing materials: a label is drawn twice (once
    /// stroked for the outline, once filled for the glyph) at identical
    /// positions. Both passes surface as distinct spans; without this
    /// filter the downstream merge pass concatenates them.
    ///
    /// Keyed by lowercased text + rounded (x, y) bucket to make the
    /// lookup O(1) without quadratic bbox comparisons on large pages.
    /// The actual overlap check falls through to a real IoU on collision.
    ///
    /// Intersection-over-union of two bounding boxes. Split out of
    /// `dedup_stroke_fill_overlap` unchanged, for file size. ~keep
    fn bbox_iou(a: &crate::geometry::Rect, b: &crate::geometry::Rect) -> f32 {
        let ix1 = a.x.max(b.x);
        let iy1 = a.y.max(b.y);
        let ix2 = (a.x + a.width).min(b.x + b.width);
        let iy2 = (a.y + a.height).min(b.y + b.height);
        if ix2 <= ix1 || iy2 <= iy1 {
            return 0.0;
        }
        let inter = (ix2 - ix1) * (iy2 - iy1);
        let area_a = a.width * a.height;
        let area_b = b.width * b.height;
        let union = area_a + area_b - inter;
        if union > 0.0 { inter / union } else { 0.0 }
    }

    /// True if `b` has an IoU-duplicate (>= `threshold`) among the bboxes
    /// stored in `grid`'s 3x3 cell neighbourhood around `(cx, cy)`. A
    /// partner with IoU >= 0.7 is within ≈0.176·width, so it always falls in
    /// this neighbourhood — querying it finds every match a full scan would.
    /// Saturating bounds: a span with an extreme/out-of-page bbox can push
    /// cx/cy to the i32 limits, where `cx + 1` would overflow in an
    /// overflow-checked build (observed on 1008.3918v2.pdf). Split out of
    /// `dedup_stroke_fill_overlap` unchanged, for file size. ~keep
    fn grid_has_iou_duplicate(
        grid: &std::collections::HashMap<(i32, i32), Vec<crate::geometry::Rect>>,
        cx: i32,
        cy: i32,
        b: &crate::geometry::Rect,
        threshold: f32,
    ) -> bool {
        for gx in cx.saturating_sub(1)..=cx.saturating_add(1) {
            for gy in cy.saturating_sub(1)..=cy.saturating_add(1) {
                let Some(others) = grid.get(&(gx, gy)) else {
                    continue;
                };
                if others.iter().any(|other| Self::bbox_iou(b, other) >= threshold) {
                    return true;
                }
            }
        }
        false
    }

    fn dedup_stroke_fill_overlap(&mut self) {
        use std::collections::HashMap;

        if self.spans.len() < 2 {
            return;
        }
        let old_len = self.spans.len();
        let spans = std::mem::take(&mut self.spans);
        // Bucket each text key into a coarse (cx,cy) grid instead of one flat
        // Vec (O(k²) when a short label repeats N times). The IoU ≥ 0.7 test is
        // unchanged: a partner with that overlap is within ≈0.176·width, so it
        // falls in this cell or a neighbour — querying the 3×3 neighbourhood
        // finds every match the full scan would. Only runs for text ≥ 2 chars
        // (shorter candidates rely on downstream positional dedup). ~keep
        const CELL: f32 = 16.0;
        type Grid = HashMap<(i32, i32), Vec<crate::geometry::Rect>>;
        let mut seen: HashMap<String, Grid> = HashMap::new();
        let mut kept: Vec<TextSpan> = Vec::with_capacity(old_len);
        let mut skipped = 0usize;
        for span in spans {
            let trimmed = span.text.trim();
            if trimmed.chars().count() < 2 {
                kept.push(span);
                continue;
            }
            let key = trimmed.to_ascii_lowercase();
            let b = span.bbox;
            let cx = ((b.x + b.width * 0.5) / CELL).floor() as i32;
            let cy = ((b.y + b.height * 0.5) / CELL).floor() as i32;
            let is_dup = seen
                .get(&key)
                .is_some_and(|grid| Self::grid_has_iou_duplicate(grid, cx, cy, &b, 0.7));
            if is_dup {
                skipped += 1;
            } else {
                seen.entry(key).or_default().entry((cx, cy)).or_default().push(b);
                kept.push(span);
            }
        }
        if skipped > 0 {
            tracing::debug!(target: LOG_TARGET, "Stroke+fill dedup: dropped {skipped} duplicate spans of {old_len}");
        }
        self.spans = kept;
    }

    /// Merge adjacent text spans on the same line to reconstruct complete words.
    ///
    /// PDF content streams often break words into multiple Tj operators for precise
    /// kerning/positioning. This causes word fragmentation like "Intr oduction" instead
    /// of "Introduction". We merge spans that are:
    /// - On the same line (Y coordinates within 1pt)
    /// - Very close horizontally (gap < 3pt, approximately average char width)
    ///
    /// Mark spans whose RTL glyphs were drawn **right-to-left** — the producer
    /// stored the text in LOGICAL order and positioned each glyph individually at
    /// decreasing x (ISO 32000-1 §14.8.2.3.3 method 1). Such spans' characters are
    /// already logical and must NOT be character-reversed by the structure-path
    /// `push_span_text_bidi`. VISUAL storage (glyphs drawn left-to-right) is never
    /// marked, so it keeps the default character-reversal and stays byte-identical.
    ///
    /// MUST run on the raw stream order (right after the content stream is parsed),
    /// before `sort_spans_by_reading_order`, which reorders the spans into
    /// left-to-right and erases the draw direction.
    ///
    /// The draw direction is the only signal that separates logical-stored RTL
    /// from visual-stored RTL when both use base-form characters with no Arabic
    /// presentation forms and no `/ReversedChars` (the two are otherwise
    /// indistinguishable yet need opposite treatment).
    pub(super) fn detect_rtl_draw_direction(&mut self) {
        use crate::text::rtl_detector::is_rtl_text;
        fn is_rtl_span(s: &TextSpan) -> bool {
            let mut rtl = false;
            for c in s.text.chars() {
                if c.is_ascii_alphabetic() {
                    return false;
                }
                if is_rtl_text(c as u32) {
                    rtl = true;
                }
            }
            rtl
        }
        let n = self.spans.len();
        // Index of the previous RTL span in stream order; a pure-whitespace span
        // between two RTL glyphs (a word break) does not break the run. ~keep
        let mut prev: Option<usize> = None;
        for i in 0..n {
            if self.spans[i].text.chars().all(char::is_whitespace) {
                continue;
            }
            if !is_rtl_span(&self.spans[i]) {
                prev = None;
                continue;
            }
            if let Some(p) = prev {
                let same_line =
                    (self.spans[i].bbox.y - self.spans[p].bbox.y).abs() < self.spans[p].font_size.max(1.0) * 0.6;
                // The incoming glyph sits to the LEFT of the previous one on the
                // same baseline ⇒ right-to-left placement ⇒ logical storage. ~keep
                if same_line && self.spans[i].bbox.x < self.spans[p].bbox.x - 0.5 {
                    self.spans[i].rtl_draw_logical = true;
                    self.spans[p].rtl_draw_logical = true;
                }
            }
            prev = Some(i);
        }
    }

    /// Per-line bimodal word-gap thresholds for the narrow-space rescue.
    ///
    /// The fixed intra-word kerning guard in `should_insert_space`
    /// (0.75× the space-glyph advance) suppresses genuine but *narrow* word
    /// gaps on condensed/tracked lines — a bold heading or a running footer
    /// typeset with NO space glyph, whose inter-word gaps are ~0.18 em, just
    /// under the guard. A fixed magnitude cannot separate a 0.18 em word gap
    /// from ~0.15 em intra-word kerning. But within one line the intra-word
    /// glyph gaps cluster near zero (tight/slightly-overlapping side-bearings)
    /// while the inter-word gaps form a distinct larger cluster: a clean
    /// bimodal split that pins the word boundary *regardless of absolute
    /// magnitude*.
    ///
    /// This walks the content-order span list, groups it into baseline runs,
    /// and for each run whose inter-span gaps are clearly bimodal returns the
    /// gap value separating the two clusters (indexed per span). Spans on
    /// unimodal or too-short lines get `None` and keep the default guard. The
    /// merge loop uses a returned threshold only to *rescue* a suppressed word
    /// gap — it never removes a space the default logic already inserts.
    pub(super) fn bimodal_line_gap_thresholds(spans: &[TextSpan]) -> Vec<Option<f32>> {
        let n = spans.len();
        let mut out = vec![None; n];
        let mut i = 0;
        while i < n {
            let mut j = i;
            while j + 1 < n && (spans[j].bbox.y - spans[j + 1].bbox.y).abs() < 1.0 {
                j += 1;
            }
            if j > i {
                let fs = spans[i..=j].iter().map(|s| s.font_size).fold(0.0f32, f32::max).max(1.0);
                // ALL consecutive gaps (intra-word gaps are near-zero or
                // slightly negative, so they must be kept, not filtered) — but
                // ONLY between glyphs sharing a baseline. A super/subscript sits
                // at a baseline shift (~0.15 em) and its horizontal gap to the
                // base is the same ~0.10 em magnitude as a condensed footer's
                // word gap; including it would let the narrow-gap rescue split a
                // math subscript from its variable (`λᵢ` → `λ i`), which the
                // advance-aware extractors correctly do NOT do. Excluding
                // baseline-shifted pairs keeps the footer word gap (same
                // baseline) while leaving dense math untouched. ~keep
                let gaps: Vec<f32> = (i..j)
                    .filter(|&k| (spans[k].bbox.y - spans[k + 1].bbox.y).abs() < fs * 0.04)
                    .map(|k| spans[k + 1].bbox.x - (spans[k].bbox.x + spans[k].bbox.width))
                    .collect();
                if let Some(split) = Self::bimodal_gap_split(&gaps, fs) {
                    for slot in out.iter_mut().take(j + 1).skip(i) {
                        *slot = Some(split);
                    }
                }
            }
            i = j + 1;
        }
        out
    }

    /// Given the consecutive inter-span gaps of one baseline run, return the
    /// threshold separating an intra-word cluster from an inter-word cluster
    /// when the distribution is clearly bimodal, else `None`. `fs` is the
    /// run's font size; all bounds are expressed as em fractions so headings
    /// and body calibrate independently.
    pub(super) fn bimodal_gap_split(gaps: &[f32], fs: f32) -> Option<f32> {
        if gaps.len() < 3 {
            return None;
        }
        let mut sorted = gaps.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        // Return the LOWEST cluster border, not the widest jump: walking the
        // sorted gaps from the bottom, the first jump that leaves the intra-word
        // cluster for a real word gap. A qualifying border needs
        //   * an intra-word-sized low side (< 0.10 em — kerning, tight
        //     side-bearings, or overlap),
        //   * a high side that is a real (if narrow) word gap (>= 0.09 em) —
        //     reaching the ~0.10 em gaps of condensed running footers that
        //     pymupdf/pdfplumber's fixed thresholds miss (an explicit positive
        //     advance IS a word-boundary signal, ISO 32000-1 §9.4.4), and
        //   * a real separation between them (>= 0.08 em), not a smooth spread.
        // Taking the LOWEST such border handles a *multi-level* condensed line —
        // tight intra-word gaps, a narrow ~0.10 em word gap, AND a wide real
        // space glyph — splitting at every level above intra-word, matching the
        // advance-aware extractors (pdfminer, poppler). A single-word line (all
        // gaps low) yields no qualifying border and returns None. The caller
        // feeds only SAME-BASELINE gaps, so a math subscript gap of the same
        // magnitude (which sits at a baseline shift) never enters this
        // distribution and is not split. ~keep
        for w in sorted.windows(2) {
            let (lo, hi) = (w[0], w[1]);
            if lo < fs * 0.10 && hi >= fs * 0.09 && (hi - lo) >= fs * 0.08 {
                return Some((lo + hi) * 0.5);
            }
        }
        None
    }
}

/// Y-sorted view of the span geometry `snap_superscript_baselines` searches.
///
/// `snapshot` holds `(x, y, width, font_size)` per span in span order;
/// `by_order` is the span indices sorted by Y and `ys_sorted` the matching Y
/// values, so a candidate's admissible base window is a contiguous slice.
struct SnapBaseSearch<'a> {
    snapshot: &'a [(f32, f32, f32, f32)],
    by_order: &'a [usize],
    ys_sorted: &'a [f32],
    max_half_em: f32,
}

impl SnapBaseSearch<'_> {
    /// Closest base candidate (in Y) that satisfies the super/subscript
    /// geometry for span `i`. Ties break on the smallest `|y_offset|` so a
    /// candidate sandwiched between two body lines snaps onto the nearer one.
    fn base_y_for(&self, i: usize) -> Option<f32> {
        let (sx, sy, _sw, sfs) = self.snapshot[i];
        let mut best_base_y: Option<f32> = None;
        let mut best_abs_offset = f32::MAX;
        // Candidates have `by ∈ [sy - max_half_em, sy]`; restrict the scan
        // to that contiguous slice of the Y-sorted index. ~keep
        let lo = self.ys_sorted.partition_point(|&y| y < sy - self.max_half_em);
        let hi = self.ys_sorted.partition_point(|&y| y <= sy);
        for &j in &self.by_order[lo..hi] {
            if i == j {
                continue;
            }
            let (bx, by, bw, bfs) = self.snapshot[j];
            if bfs <= sfs * 1.15 {
                continue;
            }
            let y_offset = sy - by;
            let half_em = bfs * 0.5;
            if y_offset.abs() > half_em {
                continue;
            }
            // Skip subscripts (lowered glyphs). The document-level
            // pass `apply_super_sub_script_substitutions` needs to
            // see them at their original lowered baseline so it can
            // substitute ASCII digits with U+2080..U+2089 (e.g.
            // H2O -> H\u{2082}O). Snapping them onto the base
            // baseline would defeat that substitution. ~keep
            if y_offset < 0.0 {
                continue;
            }
            // X adjacency: the candidate's left edge must sit
            // near the base's right edge — within one base
            // font_size to the right and a small slack to the
            // left for kerning. Combining diacritics are
            // excluded by the size-ratio gate above (they
            // typically share font_size with their base
            // letter, failing `bfs > sfs * 1.15`). ~keep
            let base_right = bx + bw;
            let dx = sx - base_right;
            if dx < -bfs * 0.25 || dx > bfs {
                continue;
            }
            let abs_off = y_offset.abs();
            if abs_off < best_abs_offset {
                best_abs_offset = abs_off;
                best_base_y = Some(by);
            }
        }
        best_base_y
    }
}

#[cfg(test)]
mod script_geometry_tests {
    use super::*;

    fn source_spans(script_y: f32, text_rise: f32, source_sequence: usize) -> Vec<TextSpan> {
        [
            ("base", 100.0, 700.0, 10.0, 12.0, 1),
            ("2", 110.0, script_y, 4.0, 8.0, source_sequence),
            ("figure label", 100.0, 500.0, 40.0, 8.0, source_sequence),
            ("caption", 100.0, 300.0, 40.0, 10.0, source_sequence),
        ]
        .into_iter()
        .map(|(text, x, y, width, font_size, sequence)| TextSpan {
            text: text.to_string(),
            bbox: crate::geometry::Rect::new(x, y, width, font_size),
            font_size,
            font_name: format!("source-{text}"),
            is_italic: text == "caption",
            primary_detected: text == "figure label",
            mirrored: text == "caption",
            page_rotation_applied: if text == "figure label" { 90 } else { 0 },
            sequence,
            text_rise: if text == "2" { text_rise } else { 0.0 },
            char_x_offsets: vec![x],
            char_widths: vec![width],
            ..Default::default()
        })
        .collect()
    }

    fn span_multiset(spans: &[TextSpan]) -> Vec<(String, bool, i32)> {
        let mut records = spans
            .iter()
            .map(|span| {
                (
                    serde_json::to_string(span).unwrap(),
                    span.mirrored,
                    span.page_rotation_applied,
                )
            })
            .collect::<Vec<_>>();
        records.sort();
        records
    }

    fn assert_preserved_after_sort(original: Vec<TextSpan>) {
        let mut extractor = TextExtractor::new();
        extractor.preserve_script_baselines = true;
        extractor.spans = original.clone();
        extractor.spans.reverse();
        extractor.sort_spans_preserving_script_geometry();
        assert_eq!(extractor.spans.len(), 4);
        assert_eq!(span_multiset(&extractor.spans), span_multiset(&original));
        assert_eq!(
            extractor
                .spans
                .iter()
                .map(|span| span.text.as_str())
                .collect::<Vec<_>>(),
            ["base", "2", "figure label", "caption"]
        );
    }

    #[test]
    fn should_preserve_each_source_span_when_form_and_page_sequences_collide() {
        for (script_y, text_rise, sequence) in [
            (703.0, 0.0, 0),
            (700.0, 0.375, 0),
            (703.0, 0.0, usize::MAX),
            (700.0, 0.375, usize::MAX),
        ] {
            let original = source_spans(script_y, text_rise, sequence);
            assert_eq!(original.iter().filter(|span| span.sequence == sequence).count(), 3);
            assert_preserved_after_sort(original);
        }
    }

    #[test]
    fn should_preserve_unique_source_sequences_and_geometry_after_reordering() {
        for (script_y, text_rise) in [(703.0, 0.0), (700.0, 0.375)] {
            let mut original = source_spans(script_y, text_rise, 0);
            for (span, sequence) in original.iter_mut().zip([usize::MAX, 7, 2, 99]) {
                span.sequence = sequence;
            }
            assert_preserved_after_sort(original);
        }
    }

    #[test]
    fn should_keep_default_script_snapping_with_colliding_source_sequences() {
        let mut extractor = TextExtractor::new();
        extractor.spans = source_spans(703.0, 0.0, usize::MAX);
        extractor.sort_spans_preserving_script_geometry();
        let coordinates = extractor
            .spans
            .iter()
            .map(|span| (span.text.as_str(), span.bbox.y))
            .collect::<Vec<_>>();
        assert_eq!(
            coordinates,
            [
                ("base", 700.0),
                ("2", 700.0),
                ("figure label", 500.0),
                ("caption", 300.0)
            ]
        );
        assert_eq!(
            extractor.spans.iter().map(|span| span.sequence).collect::<Vec<_>>(),
            [1, usize::MAX, usize::MAX, usize::MAX]
        );
    }
}
