//! Column detection, gutters, and running-artifact identification.
//!
//! Split out of the parent's single 18,544-line `impl PdfDocument`, which made
//! `document.rs` 1.2 MiB and tripped the 500 KiB file-safety limit. A child module's
//! `impl` is the same inherent impl and sees the parent's private items unchanged. ~keep

// TODO(xberg-io/xberg#1567): 6 cyclomatic-complexity and 19 size/complexity findings
// in this file, currently excluded via the quality-debt baseline in alef.toml. Splitting
// these needs compiler-in-the-loop verification, not a mechanical pass. Delete this
// note and the file's baseline entry together once it goes green. Help wanted.

use super::*;

impl PdfDocument {
    /// Heuristic: does this page have two or more vertical text columns?
    ///
    /// Used by `extract_spans` to decide whether to pay the XY-cut cost
    /// (correct but slower on large pages) or stick with the cheap row-
    /// aware sort. The check bins span X-centers into a small histogram
    /// and looks for two dense bands separated by a gutter whose spans
    /// vertically overlap with each other — that's the defining shape
    /// of a multi-column layout (newspaper / academic / dashboard) as
    /// opposed to sparse side-notes that flank a single column.
    ///
    /// False negatives (missed multi-column page) just mean we use the
    /// old reading order. False positives (single column routed through
    /// XY-cut) cost a bit of CPU but produce the same or better result.
    /// Both sides degrade gracefully.
    /// True when the page splits into side-by-side columns separated by a clean
    /// vertical gutter that no text span crosses.
    ///
    /// This is the small-page companion to the histogram detector in
    /// [`Self::is_multi_column_page`]: a two-column page with only a handful of
    /// wrapped lines per column (a short article, a synthetic fixture) carries
    /// too few spans for a projection histogram to classify, yet the gutter is
    /// perfectly unambiguous. We recover it directly:
    ///
    /// 1. Drop spans whose width exceeds 60 % of the content width — full-bleed
    ///    headings/footers legitimately straddle the gutter and must not veto it
    ///    (the recursive XY-Cut handles them with a horizontal cut first).
    /// 2. Sweep the remaining boxes left-to-right merging their X extents; a
    ///    forward jump of ≥ `MIN_GUTTER_PT` between the running right edge and
    ///    the next box's left edge is an empty channel that no span crosses.
    /// 3. Accept only when ≥ 2 spans sit on each side (genuine columns, not a
    ///    stray indent or page number) and the two sides' vertical ranges
    ///    overlap (columns sit beside each other, ruling out stacked blocks).
    fn has_clean_column_gutter(spans: &[crate::layout::TextSpan]) -> bool {
        /// Minimum empty-channel width. Real column gutters run ≥ 18pt; ordinary
        /// inter-word/inter-cell gaps are both narrower and crossed by spans on
        /// other lines, so they never survive the sweep.
        const MIN_GUTTER_PT: f32 = 18.0;

        let mut boxes: Vec<(f32, f32, f32, f32)> = spans
            .iter()
            .filter(|s| {
                !s.text.trim().is_empty()
                    && s.bbox.x.is_finite()
                    && s.bbox.y.is_finite()
                    && s.bbox.width.is_finite()
                    && s.bbox.height.is_finite()
                    && s.bbox.width > 0.0
            })
            .map(|s| (s.bbox.x, s.bbox.x + s.bbox.width, s.bbox.y, s.bbox.y + s.bbox.height))
            .collect();
        if boxes.len() < 4 {
            return false;
        }

        let content_min_x = boxes.iter().map(|b| b.0).fold(f32::INFINITY, f32::min);
        let content_max_x = boxes.iter().map(|b| b.1).fold(f32::NEG_INFINITY, f32::max);
        let content_w = content_max_x - content_min_x;
        if content_w < 100.0 {
            return false;
        }

        boxes.retain(|b| (b.1 - b.0) <= 0.6 * content_w);
        if boxes.len() < 4 {
            return false;
        }
        boxes.sort_by(|a, b| crate::utils::safe_float_cmp(a.0, b.0));

        // Grid-row guard. A two-column body has exactly one text run per column
        // on a line, so a row carries at most one wide internal gap (the
        // gutter). A table / form row instead has several cells, i.e. two or
        // more wide internal gaps. Group the (heading-excluded) boxes into rows
        // by baseline and, when the majority of multi-box rows carry ≥ 2
        // significant internal gaps, treat the page as a grid and bail — a
        // single wide middle gap on a 2×N cell grid would otherwise read as a
        // lone gutter. Mirrors the grid-row discriminator on the histogram path. ~keep
        const MIN_GAP_PT: f32 = 6.0;
        let mut rows: std::collections::BTreeMap<i32, Vec<(f32, f32)>> = std::collections::BTreeMap::new();
        for &(x0, x1, _y0, y1) in &boxes {
            rows.entry(y1.round() as i32).or_default().push((x0, x1));
        }
        let (mut multi_gap_rows, mut counted_rows) = (0usize, 0usize);
        for cells in rows.values() {
            if cells.len() < 2 {
                continue;
            }
            let mut s = cells.clone();
            s.sort_by(|a, b| crate::utils::safe_float_cmp(a.0, b.0));
            let gaps = s.windows(2).filter(|w| w[1].0 - w[0].1 >= MIN_GAP_PT).count();
            counted_rows += 1;
            if gaps >= 2 {
                multi_gap_rows += 1;
            }
        }
        if counted_rows > 0 && multi_gap_rows * 2 >= counted_rows {
            return false;
        }

        // Sweep-merge X extents and collect EVERY ≥ MIN_GUTTER_PT forward jump
        // (a vertical channel no span crosses). A genuine two-column body has
        // exactly ONE such corridor — the gutter; the lines inside each column
        // overlap horizontally, so a column's own extents merge into a single
        // contiguous run. A short-cell table (numeric grid, form) instead leaves
        // a corridor between every cell column, so two or more qualifying gaps
        // means a grid / multi-region layout, not two columns — reject it
        // (matching the grid-row discriminator on the histogram path). ~keep
        let mut cover_right = boxes[0].1;
        let mut gutter_splits: Vec<usize> = Vec::new();
        for i in 1..boxes.len() {
            if boxes[i].0 - cover_right >= MIN_GUTTER_PT {
                gutter_splits.push(i);
            }
            cover_right = cover_right.max(boxes[i].1);
        }
        if gutter_splits.len() != 1 {
            return false;
        }

        let (left, right) = boxes.split_at(gutter_splits[0]);
        if left.len() < 2 || right.len() < 2 {
            return false;
        }
        // Vertical ranges of the two sides must overlap — otherwise the
        // "columns" are vertically stacked blocks (e.g. a body block above a
        // sidebar), which read fine row-aware. ~keep
        let l_y0 = left.iter().map(|b| b.2).fold(f32::INFINITY, f32::min);
        let l_y1 = left.iter().map(|b| b.3).fold(f32::NEG_INFINITY, f32::max);
        let r_y0 = right.iter().map(|b| b.2).fold(f32::INFINITY, f32::min);
        let r_y1 = right.iter().map(|b| b.3).fold(f32::NEG_INFINITY, f32::max);
        let overlap = l_y1.min(r_y1) - l_y0.max(r_y0);
        let min_height = (l_y1 - l_y0).min(r_y1 - r_y0);
        min_height > 0.0 && overlap > 0.5 * min_height
    }

    /// Gutter X for a page that is genuinely **two-column PROSE**, or
    /// `None`. Content-balance discriminator (corpus-measured): rejects forms
    /// (`label:value`), TOCs (`title…page#`), tables and N-up — all of which
    /// share a clean gutter but must read row-wise. A real two-column body has
    /// full-length text on both sides of the gutter.
    /// Measure a single central vertical gutter (a column-separating whitespace
    /// corridor) as a PURE geometric read. Returns the gutter's mid-X when the
    /// page has EXACTLY ONE corridor ≥ `MIN_GUTTER_PT` wide near mid-page
    /// (`0.30..=0.70` of content width); `None` for single-column, multi-corridor
    /// (grid/table/form), off-centre, or too-narrow pages — so a caller that
    /// gates on `Some` is byte-identical on all of those.
    ///
    /// Shared by the marginalia pre-filter (Item 2) and the topological
    /// union-find gutter veto (Item 4). Deliberately NOT a refactor of
    /// `prose_two_column_gutter` / `has_clean_column_gutter`: those use different
    /// corridor thresholds (12 / 18) and additional structural guards, and
    /// unifying them is high blast radius for no benefit. This is a separate,
    /// conservative 18 pt central-corridor probe.
    #[allow(dead_code)]
    pub(super) fn measure_single_central_gutter(spans: &[crate::layout::TextSpan]) -> Option<f32> {
        const MIN_GUTTER_PT: f32 = 18.0;
        let body: Vec<&crate::layout::TextSpan> = spans
            .iter()
            .filter(|s| {
                !s.text.trim().is_empty() && s.bbox.width > 0.0 && s.bbox.x.is_finite() && s.bbox.width.is_finite()
            })
            .collect();
        if body.len() < 8 {
            return None;
        }
        let cmin = body.iter().map(|s| s.bbox.x).fold(f32::INFINITY, f32::min);
        let cmax = body
            .iter()
            .map(|s| s.bbox.x + s.bbox.width)
            .fold(f32::NEG_INFINITY, f32::max);
        let content_w = cmax - cmin;
        if content_w < 100.0 {
            return None;
        }
        // Exclude full-width spanning rows (headings/footers) so they don't mask
        // the corridor (same exclusion as the prose/clean-gutter sweeps). ~keep
        let mut boxes: Vec<(f32, f32)> = body
            .iter()
            .filter(|s| s.bbox.width <= 0.6 * content_w)
            .map(|s| (s.bbox.x, s.bbox.x + s.bbox.width))
            .collect();
        if boxes.len() < 8 {
            return None;
        }
        boxes.sort_by(|a, b| crate::utils::safe_float_cmp(a.0, b.0));
        let mut cover = boxes[0].1;
        let (mut corridors, mut gutter_x) = (0usize, 0.0f32);
        for &(l, r) in &boxes[1..] {
            if l - cover >= MIN_GUTTER_PT {
                corridors += 1;
                gutter_x = (cover + l) * 0.5;
            }
            cover = cover.max(r);
        }
        if corridors != 1 || !(0.30..=0.70).contains(&((gutter_x - cmin) / content_w)) {
            return None;
        }
        Some(gutter_x)
    }

    /// Valley-DEPTH central gutter probe. Like `measure_single_central_gutter`,
    /// returns the mid-X of a single central column-separating corridor, but uses
    /// a 2-D span-PROJECTION density (the emptiest vertical channel over the whole
    /// Y-extent) instead of a 1-D running-cover scan. This finds gutters that the
    /// cover scan misses because a full-width header/footer that is NOT quite wide
    /// enough to be band-excluded (it spans, say, 0.55 of the content width) jumps
    /// the running cover past the corridor; the projection only counts spans that
    /// actually straddle a given x, so a single bridging line is absorbed by the
    /// tolerance. It also catches the TIGHT (≈ 10–14 pt) real gutters of dense
    /// two-column journal bodies, below the conservative 18 pt cover threshold.
    ///
    /// A true gutter is a vertical band of near-zero straddle density; a phantom
    /// word/indent gap has moderate density (many lines carry text there). Returns
    /// the gutter mid-X only when EXACTLY ONE such near-empty central corridor of
    /// real width exists; `None` otherwise. Used (OR-ed with the cover scan) as
    /// the topological union-find gutter veto, so it can only PREVENT a
    /// cross-gutter union — never create one — keeping non-2-column pages
    /// byte-identical.
    pub(super) fn density_central_gutter(spans: &[crate::layout::TextSpan]) -> Option<f32> {
        let finite = |s: &crate::layout::TextSpan| {
            !s.text.trim().is_empty() && s.bbox.width > 0.0 && s.bbox.x.is_finite() && s.bbox.width.is_finite()
        };
        let body: Vec<&crate::layout::TextSpan> = spans.iter().filter(|s| finite(s)).collect();
        if body.len() < 12 {
            return None;
        }
        let cmin = body.iter().map(|s| s.bbox.x).fold(f32::INFINITY, f32::min);
        let cmax = body
            .iter()
            .map(|s| s.bbox.x + s.bbox.width)
            .fold(f32::NEG_INFINITY, f32::max);
        let content_w = cmax - cmin;
        // A normal PDF page is at most a few thousand points wide. A
        // degenerate CTM can inflate span x-coordinates by orders of
        // magnitude, which would otherwise drive the fine-resolution scan
        // below (bounded to a >=0.5pt step) into an effectively unbounded
        // loop. Same hazard, same bound as
        // `pipeline::reading_order::xycut::MAX_PROJECTION_SIZE`. ~keep
        const MAX_CONTENT_EXTENT: f32 = 100_000.0;
        if !content_w.is_finite() || !(100.0..=MAX_CONTENT_EXTENT).contains(&content_w) {
            return None;
        }
        // Column-content spans only (exclude true full-width bands). A real
        // gutter is invisible under titles/abstracts/footers, so they must not
        // count toward straddle density. ~keep
        let band_w = 0.6 * content_w;
        let col_idx: Vec<usize> = (0..spans.len())
            .filter(|&i| finite(&spans[i]) && spans[i].bbox.width <= band_w)
            .collect();
        if col_idx.len() < 12 {
            return None;
        }
        // Scan the central band; "empty" tolerates ~1 % stray straddlers (a rare
        // long token or a header just under the band-exclusion width). ~keep
        let lo = cmin + 0.30 * content_w;
        let hi = cmin + 0.70 * content_w;
        let step = (content_w / 400.0).clamp(0.5, 3.0);
        let empty_max = (0.01 * col_idx.len() as f32).ceil() as usize;
        let straddle_at = |x: f32| -> usize {
            col_idx
                .iter()
                .filter(|&&i| spans[i].bbox.x + 2.0 < x && spans[i].bbox.x + spans[i].bbox.width - 2.0 > x)
                .count()
        };
        // Find ALL near-empty corridors and the widest one; require EXACTLY ONE
        // (a 3-column grid has two, and must stay row-aware). ~keep
        let (mut corridors, mut best_w, mut best_mid) = (0usize, 0.0f32, f32::NAN);
        let (mut run_start, mut in_run) = (lo, false);
        let mut x = lo;
        let close = |run_start: f32, end: f32, corridors: &mut usize, best_w: &mut f32, best_mid: &mut f32| {
            let w = end - run_start;
            if w >= 6.0 {
                *corridors += 1;
                if w > *best_w {
                    *best_w = w;
                    *best_mid = (run_start + end) * 0.5;
                }
            }
        };
        while x <= hi {
            if straddle_at(x) <= empty_max {
                if !in_run {
                    run_start = x;
                    in_run = true;
                }
            } else if in_run {
                close(run_start, x, &mut corridors, &mut best_w, &mut best_mid);
                in_run = false;
            }
            x += step;
        }
        if in_run {
            close(run_start, hi, &mut corridors, &mut best_w, &mut best_mid);
        }
        if corridors != 1 || !best_mid.is_finite() {
            return None;
        }
        // Balanced columns: each side carries a real share of the column spans
        // (rejects a single column beside a sparse margin rail). ~keep
        let (mut left_idx, mut right_idx): (Vec<usize>, Vec<usize>) = (Vec::new(), Vec::new());
        for &i in &col_idx {
            let s = &spans[i];
            if s.bbox.x + s.bbox.width * 0.5 < best_mid {
                left_idx.push(i);
            } else {
                right_idx.push(i);
            }
        }
        let n = left_idx.len() + right_idx.len();
        if n == 0 || (left_idx.len() * 4 < n) || (right_idx.len() * 4 < n) {
            return None;
        }
        // Class gate, mirroring `classifier_column_gutter`'s (load-bearing there
        // too): a tight, balanced corridor is not by itself proof of a two-column
        // PROSE body -- a label:value form (narrow label column, wide value
        // column) reads exactly the same corridor shape. Reject when either side
        // classifies as Table/Form; this probe has no reference-list/short-line
        // carve-out to preserve, so unlike `prose_two_column_gutter` there is no
        // balance-test fallback here. ~keep
        use crate::layout::RegionClass;
        let is_table_or_form = |c| matches!(c, RegionClass::Table | RegionClass::Form);
        if is_table_or_form(crate::layout::classify_region(spans, &left_idx))
            || is_table_or_form(crate::layout::classify_region(spans, &right_idx))
        {
            return None;
        }
        Some(best_mid)
    }

    /// Characters-per-text-line density for a set of spans (≈ chars per line).
    /// Lines are counted by clustering span upper edges (`bbox.bottom()`, larger
    /// y) with a `med_h * 0.6` gap. A page-number rail or a form's value column
    /// is text-SPARSE (a few chars per line); genuine prose columns and metadata
    /// sidebars are text-DENSE. Shared by the topological side-by-side gate
    /// (Item 1) and the marginalia sparsity gate (Item 2) — same formula the
    /// `topological_block_order` `char_density` closure uses.
    #[allow(dead_code)]
    pub(super) fn block_char_density(spans: &[&crate::layout::TextSpan], med_h: f32) -> f32 {
        if spans.is_empty() {
            return 0.0;
        }
        let med_h = med_h.max(1.0);
        let mut ys: Vec<f32> = spans.iter().map(|s| s.bbox.bottom()).collect();
        ys.sort_by(|p, q| crate::utils::safe_float_cmp(*p, *q));
        let mut lines = 1usize;
        for w in ys.windows(2) {
            if (w[1] - w[0]).abs() > med_h * 0.6 {
                lines += 1;
            }
        }
        let chars: usize = spans.iter().map(|s| s.text.trim().chars().count()).sum();
        chars as f32 / lines as f32
    }

    /// Detect a marginalia column (Item 2 / M2): a narrow, sparse, body-aligned
    /// numeric rail at the extreme left or right of the page — manuscript line
    /// numbers (`118 119 120 …`), a folio rail. Returns the indices of the rail
    /// spans (into `spans`) so the caller can lift them OUT of the body before
    /// geometric column dispatch (a rail otherwise injects a spurious second
    /// corridor / sparse block that disqualifies prose/topo detection) and
    /// re-append them at the end of the reading order.
    ///
    /// Tight 7-gate conjunction so it is a strict no-op (`None`) on ordinary
    /// pages and never lifts a genuine narrow first column (which is text-DENSE
    /// and multi-word → fails the sparsity + numeric-shape gates). `None` keeps
    /// the caller byte-identical.
    pub(super) fn lift_marginalia_column(spans: &[crate::layout::TextSpan]) -> Option<Vec<usize>> {
        use crate::utils::safe_float_cmp;
        let texties: Vec<usize> = (0..spans.len())
            .filter(|&i| {
                !spans[i].text.trim().is_empty()
                    && spans[i].bbox.x.is_finite()
                    && spans[i].bbox.width.is_finite()
                    && spans[i].bbox.width > 0.0
            })
            .collect();
        if texties.len() < 12 {
            return None;
        }
        let median = |mut v: Vec<f32>| -> Option<f32> {
            if v.is_empty() {
                return None;
            }
            v.sort_by(|a, b| safe_float_cmp(*a, *b));
            Some(v[v.len() / 2].max(1.0))
        };
        let med_fs = median(
            texties
                .iter()
                .filter(|&&i| spans[i].text.trim().chars().count() >= 2 && spans[i].font_size > 0.0)
                .map(|&i| spans[i].font_size)
                .collect(),
        )?;
        let med_h = median(
            texties
                .iter()
                .map(|&i| spans[i].bbox.height.abs())
                .filter(|h| h.is_finite() && *h > 0.0)
                .collect(),
        )?;
        let cmin = texties.iter().map(|&i| spans[i].bbox.x).fold(f32::INFINITY, f32::min);
        let cmax = texties
            .iter()
            .map(|&i| spans[i].bbox.x + spans[i].bbox.width)
            .fold(f32::NEG_INFINITY, f32::max);
        let content_w = cmax - cmin;
        if content_w < 100.0 {
            return None;
        }
        let xband = 3.0 * med_fs;

        // M2 targets LEFT-margin manuscript line-number rails (the documented
        // mechanism — "narrow left-margin numerals woven into the prose stream").
        // A right-margin narrow numeric column is predominantly a TOC/table-of-
        // contents PAGE-NUMBER reference that pairs 1:1 with its entry row;
        // lifting it would regroup the page numbers away from their entries (a
        // reorder that hurts TOC pages, observed on CFR Title 36). So only the
        // left rail is considered. The symmetric right-side geometry below is
        // retained so a future TOC-discriminating gate can re-enable it safely. ~keep
        for left_side in core::iter::once(true) {
            let in_band = |i: usize| -> bool {
                let l = spans[i].bbox.x;
                let r = spans[i].bbox.x + spans[i].bbox.width;
                if left_side {
                    r <= cmin + xband
                } else {
                    l >= cmax - xband
                }
            };
            let strip: Vec<usize> = texties.iter().copied().filter(|&i| in_band(i)).collect();
            if strip.len() < 3 {
                continue;
            }
            let strip_set: std::collections::HashSet<usize> = strip.iter().copied().collect();
            let body: Vec<usize> = texties.iter().copied().filter(|i| !strip_set.contains(i)).collect();
            if body.len() < 8 {
                continue;
            }

            let strip_refs: Vec<&crate::layout::TextSpan> = strip.iter().map(|&i| &spans[i]).collect();
            if Self::block_char_density(&strip_refs, med_h) >= 4.0 {
                continue;
            }

            // Gate 7: at least 3 rail lines (a recurring rail, not a stray number). ~keep
            let mut ys: Vec<f32> = strip.iter().map(|&i| spans[i].bbox.bottom()).collect();
            ys.sort_by(|p, q| safe_float_cmp(*p, *q));
            let lines = 1 + ys.windows(2).filter(|w| (w[1] - w[0]).abs() > med_h * 0.6).count();
            if lines < 3 {
                continue;
            }

            // Gate 6: NUMERIC-SHAPE — ≥70% pure digits or ≤3-char tokens. This is
            // the discriminator vs a real narrow prose column (multi-word lines). ~keep
            let numeric = strip
                .iter()
                .filter(|&&i| {
                    let t = spans[i].text.trim();
                    (!t.is_empty() && t.chars().all(|c| c.is_ascii_digit())) || t.chars().count() <= 3
                })
                .count();
            if (numeric as f32) < 0.70 * strip.len() as f32 {
                continue;
            }

            // Gate 4: DETACHED — a clear ≥18 pt empty gutter between the rail's
            // inner edge and the body's outer edge (the rail is geometrically
            // separate, not just the first words of body lines). ~keep
            let (strip_inner, body_outer) = if left_side {
                (
                    strip
                        .iter()
                        .map(|&i| spans[i].bbox.x + spans[i].bbox.width)
                        .fold(f32::NEG_INFINITY, f32::max),
                    body.iter().map(|&i| spans[i].bbox.x).fold(f32::INFINITY, f32::min),
                )
            } else {
                (
                    strip.iter().map(|&i| spans[i].bbox.x).fold(f32::INFINITY, f32::min),
                    body.iter()
                        .map(|&i| spans[i].bbox.x + spans[i].bbox.width)
                        .fold(f32::NEG_INFINITY, f32::max),
                )
            };
            let gutter = if left_side {
                body_outer - strip_inner
            } else {
                strip_inner - body_outer
            };
            if gutter < 18.0 {
                continue;
            }

            // Gate 5: BODY-ALIGNED — the rail runs ALONGSIDE the body (Y-overlap
            // > half the rail height), not above/below it. ~keep
            let (sy0, sy1) = strip.iter().fold((f32::INFINITY, f32::NEG_INFINITY), |(a, b), &i| {
                (a.min(spans[i].bbox.y), b.max(spans[i].bbox.y + spans[i].bbox.height))
            });
            let (by0, by1) = body.iter().fold((f32::INFINITY, f32::NEG_INFINITY), |(a, b), &i| {
                (a.min(spans[i].bbox.y), b.max(spans[i].bbox.y + spans[i].bbox.height))
            });
            let overlap = sy1.min(by1) - sy0.max(by0);
            if overlap <= 0.5 * (sy1 - sy0).max(1.0) {
                continue;
            }

            return Some(strip);
        }
        None
    }

    pub(super) fn prose_two_column_gutter(spans: &[crate::layout::TextSpan]) -> Option<f32> {
        let body: Vec<&crate::layout::TextSpan> = spans
            .iter()
            .filter(|s| {
                !s.text.trim().is_empty() && s.bbox.width > 0.0 && s.bbox.x.is_finite() && s.bbox.width.is_finite()
            })
            .collect();
        if body.len() < 8 {
            return None;
        }
        let cmin = body.iter().map(|s| s.bbox.x).fold(f32::INFINITY, f32::min);
        let cmax = body
            .iter()
            .map(|s| s.bbox.x + s.bbox.width)
            .fold(f32::NEG_INFINITY, f32::max);
        let content_w = cmax - cmin;
        if content_w < 100.0 {
            return None;
        }
        // Exactly one clean corridor near mid-page (bridge-excluded). 0 = single
        // column; ≥2 = grid/form/table. ~keep
        let mut boxes: Vec<(f32, f32)> = body
            .iter()
            .filter(|s| s.bbox.width <= 0.6 * content_w)
            .map(|s| (s.bbox.x, s.bbox.x + s.bbox.width))
            .collect();
        if boxes.len() < 8 {
            return None;
        }
        boxes.sort_by(|a, b| crate::utils::safe_float_cmp(a.0, b.0));
        let mut cover = boxes[0].1;
        let (mut corridors, mut gutter_x) = (0usize, 0.0f32);
        for &(l, r) in &boxes[1..] {
            if l - cover >= 12.0 {
                corridors += 1;
                gutter_x = (cover + l) * 0.5;
            }
            cover = cover.max(r);
        }
        if corridors != 1 || !(0.30..=0.70).contains(&((gutter_x - cmin) / content_w)) {
            return None;
        }
        // Column count via left-edge clustering. The coverage sweep above counts
        // *one* corridor whenever a single wide span in a column reaches past the
        // next column's start, hiding the real inter-column gap — so a two-page
        // spread (4 columns) collapses to its spread midline and reads as a clean
        // 2-column page, whose halves then each merge two real columns into an
        // interleaved row-major mess. Cluster the (non-full-width) span left edges
        // and require EXACTLY two significant column starts; anything else
        // (single column, 3+ columns, N-up spread) is rejected. ~keep
        {
            let mut lefts: Vec<f32> = boxes.iter().map(|&(l, _)| l).collect();
            lefts.sort_by(|a, b| crate::utils::safe_float_cmp(*a, *b));
            let clust_gap = 0.08 * content_w;
            let mut counts: Vec<usize> = Vec::new();
            let mut run = 1usize;
            let mut prev = lefts[0];
            for &v in &lefts[1..] {
                if v - prev > clust_gap {
                    counts.push(run);
                    run = 0;
                }
                run += 1;
                prev = v;
            }
            counts.push(run);
            // A "significant" column start carries ≥15% of the column-eligible
            // spans (a hanging-indent continuation merges into its start cluster
            // because its offset is far below clust_gap). ~keep
            let min_sig = (0.15 * lefts.len() as f32).ceil() as usize;
            let sig = counts.iter().filter(|&&c| c >= min_sig).count();
            if sig != 2 {
                return None;
            }
        }
        // Per-column region classification. The genuine discriminator between a
        // two-column PROSE/REFERENCE body (read column-major) and a table / form /
        // TOC that merely has one central gap (read row-wise) is the STRUCTURE of
        // each column, not a cross-gutter row-balance ratio. A cross-gutter
        // row-alignment gate measures alignment that ragged reference lists and
        // dense results columns do not have, so those were wrongly rejected and
        // fell to a row-major interleave. Classifying each half on its own
        // structure admits them while still rejecting tables/forms (which classify
        // as Table/Form). See `examples/classify_probe.rs`. ~keep
        let body_side = |want_left: bool| -> Vec<usize> {
            spans
                .iter()
                .enumerate()
                .filter(|(_, s)| {
                    !s.text.trim().is_empty()
                        && s.bbox.width > 0.0
                        && s.bbox.x.is_finite()
                        && s.bbox.width.is_finite()
                        && ((s.bbox.x + s.bbox.width * 0.5 < gutter_x) == want_left)
                })
                .map(|(i, _)| i)
                .collect()
        };
        let left_class = crate::layout::classify_region(spans, &body_side(true));
        let right_class = crate::layout::classify_region(spans, &body_side(false));
        if left_class.is_reorderable_column() && right_class.is_reorderable_column() {
            return Some(gutter_x);
        }
        // Fallback: the cross-gutter content-balance test. The per-column
        // classifier (above) admits ragged reference lists / dense results columns
        // the balance test rejected, but it also REJECTS some genuine balanced
        // two-column PROSE the balance test accepted — short, ragged verse/body
        // lines on a narrow-gutter page (a reference-Bible / two-column page with a
        // full-width title). Without this they fall off the
        // column-major path and the first row interleaves across the gutter. Tried
        // only AFTER the classifier (academic pages keep the classifier path) and
        // behind the same corridor + two-column-start preamble, so single-column / ~keep
        // grid / N-up pages never reach it.
        if Self::two_column_rows_balanced(spans, gutter_x) {
            return Some(gutter_x);
        }
        None
    }

    /// Cross-gutter content-balance test: true when spanning rows carry
    /// substantial text on BOTH sides of `gutter_x` (prose), not a short
    /// right-hand value / page number (form / TOC). Fallback for
    /// `prose_two_column_gutter` after the per-column classifier declines.
    fn two_column_rows_balanced(spans: &[crate::layout::TextSpan], gutter_x: f32) -> bool {
        let mut ordered: Vec<&crate::layout::TextSpan> = spans
            .iter()
            .filter(|s| {
                !s.text.trim().is_empty() && s.bbox.width > 0.0 && s.bbox.x.is_finite() && s.bbox.width.is_finite()
            })
            .collect();
        ordered.sort_by(|a, b| crate::utils::safe_float_cmp(b.bbox.y, a.bbox.y));
        let (mut total, mut spanning, mut short_r) = (0usize, 0usize, 0usize);
        let (mut lefts, mut rights): (Vec<usize>, Vec<usize>) = (Vec::new(), Vec::new());
        let mut i = 0;
        while i < ordered.len() {
            let y0 = ordered[i].bbox.y;
            let (mut lc, mut rc) = (0usize, 0usize);
            while i < ordered.len() && (ordered[i].bbox.y - y0).abs() <= 3.0 {
                let s = ordered[i];
                let n = s.text.trim().chars().count();
                if s.bbox.x + s.bbox.width * 0.5 < gutter_x {
                    lc += n;
                } else {
                    rc += n;
                }
                i += 1;
            }
            total += 1;
            if lc > 0 && rc > 0 {
                spanning += 1;
                lefts.push(lc);
                rights.push(rc);
                if rc < 15 {
                    short_r += 1;
                }
            }
        }
        if total < 6 || spanning == 0 || (spanning as f32) < 0.60 * total as f32 {
            return false;
        }
        if (short_r as f32) > 0.30 * spanning as f32 {
            return false;
        }
        let med = |v: &mut [usize]| -> f32 {
            v.sort_unstable();
            v[v.len() / 2] as f32
        };
        let (ml, mr) = (med(&mut lefts), med(&mut rights));
        mr >= 25.0 && (0.45..=2.2).contains(&(mr / ml.max(1.0)))
    }

    /// Robust classifier-gated two-column detector for bodies the clean corridor
    /// sweep (`prose_two_column_gutter`) and `is_multi_column_page`
    /// MISS — ragged reference lists and dense results columns. Their lines do
    /// not leave the single perfectly-clean empty corridor those detectors
    /// require (long entries occasionally bridge, ragged tails create extra
    /// gaps), so the page currently reads row-major (interleaved). This is the
    /// real-academic M1/M3 deficit.
    ///
    /// Strategy: find the emptiest vertical corridor in the central band, require
    /// it to be near-empty (a genuine gutter), require BALANCED + TALL columns on
    /// both sides (rejects single-column + margin note, and short side captions),
    /// and accept ONLY when both halves classify as reorderable (Prose/Reference)
    /// — so tables, forms, and single-column pages are rejected. Proven on the 5
    /// corpus discriminator PDFs (see `examples/classify_probe.rs`). Returns the
    /// gutter X on accept, else `None` (caller keeps prior behaviour).
    pub(super) fn classifier_column_gutter(spans: &[crate::layout::TextSpan]) -> Option<f32> {
        let finite = |s: &crate::layout::TextSpan| {
            !s.text.trim().is_empty()
                && s.bbox.width > 0.0
                && s.bbox.x.is_finite()
                && s.bbox.width.is_finite()
                && s.bbox.y.is_finite()
        };
        let body: Vec<&crate::layout::TextSpan> = spans.iter().filter(|s| finite(s)).collect();
        if body.len() < 16 {
            return None;
        }
        let cmin = body.iter().map(|s| s.bbox.x).fold(f32::INFINITY, f32::min);
        let cmax = body
            .iter()
            .map(|s| s.bbox.x + s.bbox.width)
            .fold(f32::NEG_INFINITY, f32::max);
        let content_w = cmax - cmin;
        // Same degenerate-CTM hazard and bound as `density_central_gutter`
        // above / `pipeline::reading_order::xycut::MAX_PROJECTION_SIZE`: the
        // fine-resolution scan below steps at >=0.5pt, so an unbounded
        // `content_w` would make the loop below run effectively forever. ~keep
        const MAX_CONTENT_EXTENT: f32 = 100_000.0;
        if !content_w.is_finite() || !(100.0..=MAX_CONTENT_EXTENT).contains(&content_w) {
            return None;
        }
        let ymin = body.iter().map(|s| s.bbox.y).fold(f32::INFINITY, f32::min);
        let ymax = body.iter().map(|s| s.bbox.y).fold(f32::NEG_INFINITY, f32::max);
        let body_h = (ymax - ymin).max(1.0);

        // COLUMN-CONTENT spans = those NOT spanning most of the content width.
        // Full-width spans (titles, the abstract block, section headings, running
        // footers) are BANDS, excluded from gutter detection and classification:
        // counting them would (a) hide the corridor on a mixed page whose top is a
        // full-width title/abstract and bottom is two columns (every paper's
        // page 1), and (b) pollute the per-column class. The corridor, balance,
        // height, and class gates all operate on column-content spans;
        // `reorder_column_major_with_bands` re-emits the bands at their own Y. ~keep
        let band_w = 0.6 * content_w;
        let col_idx: Vec<usize> = (0..spans.len())
            .filter(|&i| finite(&spans[i]) && spans[i].bbox.width <= band_w)
            .collect();
        if col_idx.len() < 16 {
            return None;
        }

        // Scan the central band [0.30, 0.70] at fine resolution and find the
        // WIDEST near-empty vertical corridor — the real inter-column gutter —
        // then place the gutter at its midpoint. Picking the widest run (not just
        // any minimal-straddle point) is load-bearing for hanging-indent
        // reference columns: a ragged-ref page has TWO empty corridors — the true
        // gutter between the columns, and a narrow decoy between the right
        // column's hanging entry numbers and its indented text. The decoy is
        // narrower, so the widest-run rule lands the gutter correctly between the
        // columns (otherwise the entry numbers fall into the left column). A
        // single-column body has NO wide empty central corridor (its lines are
        // full-width → excluded above → too few column spans), so it returns None. ~keep
        let lo = cmin + 0.30 * content_w;
        let hi = cmin + 0.70 * content_w;
        let step = (content_w / 400.0).clamp(0.5, 3.0);
        // "Empty" tolerates a few stray straddlers (noise / a rare long token). ~keep
        let empty_max = (0.01 * col_idx.len() as f32).ceil() as usize;
        let straddle_at = |x: f32| -> usize {
            col_idx
                .iter()
                .filter(|&&i| spans[i].bbox.x + 2.0 < x && spans[i].bbox.x + spans[i].bbox.width - 2.0 > x)
                .count()
        };
        let (mut best_lo, mut best_hi) = (f32::NAN, f32::NAN);
        let (mut run_start, mut in_run, mut best_w) = (lo, false, 0.0f32);
        let mut x = lo;
        while x <= hi {
            if straddle_at(x) <= empty_max {
                if !in_run {
                    run_start = x;
                    in_run = true;
                }
            } else if in_run {
                let w = x - run_start;
                if w > best_w {
                    best_w = w;
                    best_lo = run_start;
                    best_hi = x;
                }
                in_run = false;
            }
            x += step;
        }
        if in_run {
            let w = hi - run_start;
            if w > best_w {
                best_w = w;
                best_lo = run_start;
                best_hi = hi;
            }
        }
        // Require a corridor of real width (a genuine gutter, not a glyph gap). ~keep
        if !best_lo.is_finite() || best_w < 6.0 {
            return None;
        }
        let gutter = (best_lo + best_hi) * 0.5;

        let (mut left_idx, mut right_idx): (Vec<usize>, Vec<usize>) = (Vec::new(), Vec::new());
        let (mut ly0, mut ly1) = (f32::INFINITY, f32::NEG_INFINITY);
        let (mut ry0, mut ry1) = (f32::INFINITY, f32::NEG_INFINITY);
        for &i in &col_idx {
            let s = &spans[i];
            if s.bbox.x + s.bbox.width * 0.5 < gutter {
                left_idx.push(i);
                ly0 = ly0.min(s.bbox.y);
                ly1 = ly1.max(s.bbox.y);
            } else {
                right_idx.push(i);
                ry0 = ry0.min(s.bbox.y);
                ry1 = ry1.max(s.bbox.y);
            }
        }
        let nb = left_idx.len() + right_idx.len();
        if nb == 0 {
            return None;
        }
        // Each side carries a real share of the column content (rejects
        // 1 col + margin note). ~keep
        if (left_idx.len() as f32) < 0.30 * nb as f32 || (right_idx.len() as f32) < 0.30 * nb as f32 {
            return None;
        }
        // Both columns must be tall and of comparable height — they sit BESIDE
        // each other. This rejects a short side-caption/figure-label beside a tall
        // body column, while allowing a mixed page where the columns occupy only
        // the lower portion below a full-width title/abstract (so the floor is
        // 0.4·body_h, not 0.5). `body_h` spans the whole page (title included). ~keep
        let (lext, rext) = (ly1 - ly0, ry1 - ry0);
        if lext < 0.4 * body_h || rext < 0.4 * body_h || lext.min(rext) < 0.5 * lext.max(rext) {
            return None;
        }
        // Class gate (load-bearing). NEITHER half may be Table/Form — that is the
        // hard table/form rejection (tables classify Table via mean_chars<10,
        // label/value pages classify Form). AND at least one half must be clearly
        // Prose/Reference, to anchor that this really is a text body. A `Mixed`
        // half is admitted alongside a Prose/Reference half: a dense results
        // column often classifies Mixed (figures, equations, and inline-citation
        // fragments lower its wide-line ratio below the Prose threshold), but it
        // is NOT a table (those are Table, not Mixed), so column-major reading is
        // still correct. Two Mixed halves (no clear prose anchor) stay rejected. ~keep
        use crate::layout::RegionClass;
        let lc = crate::layout::classify_region(spans, &left_idx);
        let rc = crate::layout::classify_region(spans, &right_idx);
        let is_table_or_form = |c| matches!(c, RegionClass::Table | RegionClass::Form);
        if is_table_or_form(lc) || is_table_or_form(rc) {
            return None;
        }
        if !(lc.is_reorderable_column() || rc.is_reorderable_column()) {
            return None;
        }
        Some(gutter)
    }

    /// Reorder a confirmed two-column-prose page **column-major with band
    /// separation**. Walks rows top→bottom; a row containing a span that
    /// *crosses* the gutter is a full-width band (title, section heading,
    /// footer) and is emitted at its vertical position, between the column runs
    /// around it. Column runs are flushed left-column-then-right-column. This is
    /// the §14.8.3 layout model: full-width BLSEs interleave with columns by
    /// block position, so a mid-body heading is NOT split across the gutter.
    pub(super) fn reorder_column_major_with_bands(spans: &mut Vec<crate::layout::TextSpan>, gutter_x: f32) {
        use crate::layout::TextSpan;
        // A genuine full-width BAND (title/heading/footer that spans both
        // columns) extends meaningfully on BOTH sides of the gutter. Require an
        // 8pt overhang each side so a column item whose bbox merely *clips* the
        // gutter by a few points — e.g. a hanging reference number ("42.") at the
        // right column's left edge, or a wrapped line reaching just past the
        // gutter — is NOT mistaken for a band and pulled out of its column. ~keep
        let crosses = |s: &TextSpan| s.bbox.x < gutter_x - 8.0 && s.bbox.x + s.bbox.width > gutter_x + 8.0;
        let mut src = std::mem::take(spans);
        // Top→bottom, then left→right within a row. Quantize Y to the row band
        // (`row_aware_span_cmp`) so sub-point baseline jitter between spans on
        // the SAME visual line (font-metric rounding, a superscript citation's
        // slightly different Y) cannot invert their X order: a 0.001pt Y
        // difference under a raw `safe_float_cmp` would sort a mid-line span
        // ahead of the line's left-edge span, scrambling the line
        // (PMC8129076 "phase and amplitude of clock-controlled genes83. Thus,
        // it is clear" — the "83" citation lifts ". Thus" onto a 0.001pt-higher
        // baseline). The downstream row grouping already uses a 3pt tolerance; ~keep
        // matching it here keeps the two consistent.
        src.sort_by(|a, b| crate::utils::row_aware_span_cmp(a.bbox.y, a.bbox.x, b.bbox.y, b.bbox.x));
        let mut out: Vec<TextSpan> = Vec::with_capacity(src.len());
        let mut col_buf: Vec<TextSpan> = Vec::new();
        let flush = |buf: &mut Vec<TextSpan>, out: &mut Vec<TextSpan>| {
            if buf.is_empty() {
                return;
            }
            // A full line-height, used to decide when a block sits clearly
            // *below* the opposite column rather than beside it. ~keep
            let mut heights: Vec<f32> = buf.iter().map(|s| s.bbox.height).collect();
            heights.sort_by(|a, b| crate::utils::safe_float_cmp(*a, *b));
            let line_h = heights.get(heights.len() / 2).copied().unwrap_or(10.0).max(1.0);
            let (mut left, mut right): (Vec<TextSpan>, Vec<TextSpan>) = std::mem::take(buf)
                .into_iter()
                .partition(|s| s.bbox.x + s.bbox.width * 0.5 < gutter_x);
            // Row-banded (Y quantized to ROW_BAND_TOLERANCE_PT) so sub-point
            // baseline jitter on a single visual line cannot invert the X order
            // within that line; see the pre-sort note above. ~keep
            let by_yx =
                |a: &TextSpan, b: &TextSpan| crate::utils::row_aware_span_cmp(a.bbox.y, a.bbox.x, b.bbox.y, b.bbox.x);
            // Trailing-block peel: a block lying a full line-height BELOW the
            // entire opposite column is a bottom-spanning block (e.g. a
            // bottom-left References section), not a parallel column member, so
            // it must read AFTER both columns at its own y — not within its
            // column partition (which would print it before the whole opposite
            // column). oxide bbox.y is top-up (higher y = higher on page), so
            // "below" = smaller y. Only fires when the opposite column has real
            // content (>=2 spans) and the block clears its bottom by a line, so ~keep
            // balanced 2-col bodies (columns ending at ~equal y) are untouched.
            let bottom_y = |v: &[TextSpan]| v.iter().map(|s| s.bbox.y).fold(f32::INFINITY, f32::min);
            let right_bottom = bottom_y(&right);
            let left_bottom = bottom_y(&left);
            let mut trailing: Vec<TextSpan> = Vec::new();
            if right.len() >= 2 {
                left.retain(|s| {
                    let below = s.bbox.y < right_bottom - line_h;
                    if below {
                        trailing.push(s.clone());
                    }
                    !below
                });
            }
            if left.len() >= 2 {
                right.retain(|s| {
                    let below = s.bbox.y < left_bottom - line_h;
                    if below {
                        trailing.push(s.clone());
                    }
                    !below
                });
            }
            left.sort_by(by_yx);
            right.sort_by(by_yx);
            trailing.sort_by(by_yx);
            out.append(&mut left);
            out.append(&mut right);
            out.append(&mut trailing);
        };
        let mut i = 0;
        while i < src.len() {
            let y0 = src[i].bbox.y;
            let mut row: Vec<TextSpan> = Vec::new();
            while i < src.len() && (src[i].bbox.y - y0).abs() <= 3.0 {
                row.push(src[i].clone());
                i += 1;
            }
            if row.iter().any(crosses) {
                flush(&mut col_buf, &mut out);
                row.sort_by(|a, b| crate::utils::safe_float_cmp(a.bbox.x, b.bbox.x));
                out.append(&mut row);
            } else {
                col_buf.append(&mut row);
            }
        }
        flush(&mut col_buf, &mut out);
        *spans = out;
    }

    /// True when the page's multi-column geometric signal is explained by a
    /// detected TABLE rather than a genuine two-column text body.
    ///
    /// Used by the geometric reading-order dispatch: when the genuine
    /// two-column branches (topological / prose-gutter / classifier) all
    /// declined yet `is_multi_column_page` is still true, the page is either a
    /// single-column body with a data table (whose column-aligned cells trip the
    /// detector) or a two-column body the column branches missed. We only want
    /// to override the multi-column gate (and apply the row-aware band sort) in
    /// the FIRST case.
    ///
    /// Discriminator: cluster the per-line left edges of the spans OUTSIDE the
    /// detected table regions (the surrounding prose). A single-column body has
    /// ONE dominant left edge there; a genuine two-column body has two. We
    /// require a strong single dominant left-edge cluster (≥ 70% of non-table
    /// lines), so a two-column page — whose non-table prose still splits into two
    /// left-edge clusters — is rejected. Spans inside the table contribute their
    /// own column-aligned left edges and are deliberately excluded.
    pub(super) fn multicol_signal_is_tabular(
        spans: &[crate::layout::TextSpan],
        tables: &[crate::structure::table_extractor::Table],
    ) -> bool {
        // Expand each table bbox slightly upward to absorb header rows the
        // spatial extractor often leaves just above the captured cell grid. ~keep
        let in_table = |s: &crate::layout::TextSpan| -> bool {
            let cx = s.bbox.x + s.bbox.width * 0.5;
            let cy = s.bbox.y + s.bbox.height * 0.5;
            tables.iter().any(|t| {
                t.bbox.is_some_and(|b| {
                    cx >= b.x - 2.0 && cx <= b.x + b.width + 2.0 && cy >= b.y - 2.0 && cy <= b.y + b.height + 14.0
                })
            })
        };
        let outside: Vec<&crate::layout::TextSpan> = spans
            .iter()
            .filter(|s| !s.text.trim().is_empty() && s.bbox.width > 0.0 && !in_table(s))
            .collect();
        if outside.len() < 8 {
            // The table is essentially the whole page; the row-aware band sort
            // linearises it correctly, so treat the signal as tabular. ~keep
            return true;
        }
        let mut by_band: std::collections::BTreeMap<i32, f32> = std::collections::BTreeMap::new();
        for s in &outside {
            let band = (s.bbox.y / 2.0).round() as i32;
            let e = by_band.entry(band).or_insert(f32::INFINITY);
            *e = e.min(s.bbox.x);
        }
        let mut lefts: Vec<f32> = by_band.values().copied().filter(|v| v.is_finite()).collect();
        if lefts.len() < 6 {
            return true;
        }
        lefts.sort_by(|a, b| crate::utils::safe_float_cmp(*a, *b));
        // Cluster left edges with a 12pt gap (≈ one indent); the largest cluster
        // is the body's left margin. ~keep
        let mut clusters: Vec<usize> = Vec::new();
        let mut run = 1usize;
        let mut prev = lefts[0];
        for &v in &lefts[1..] {
            if v - prev > 12.0 {
                clusters.push(run);
                run = 0;
            }
            run += 1;
            prev = v;
        }
        clusters.push(run);
        let total = lefts.len();
        let top = *clusters.iter().max().unwrap_or(&0);
        // Strong single dominant left edge ⇒ single-column prose ⇒ the
        // multi-column signal came from the table. ~keep
        top as f32 >= 0.70 * total as f32
    }

    /// Mid-X of the page's column gutter, for callers that need the value and
    /// not just `is_multi_column_page`'s yes/no.
    ///
    /// Tries the same three detectors, in the same order, that the text path
    /// in `text_assembly.rs` uses, plus the density probe that finds the tight
    /// (10-14 pt) gutters the cover scan misses, and last `line_corridor_gutter`
    /// (GH#1809) for the lopsided pages all three decline on. Each returns `None` on
    /// single-column, grid/form/table and off-centre pages (see the
    /// `detect_column_gutter_rejects_*` tests), so a caller that gates on
    /// `Some` is unchanged on all of those.
    ///
    /// This is a DIFFERENT question from `is_multi_column_page`'s, and the two are
    /// NOT gated on each other: `page_reading_order_inner` (`pipeline/page_order.rs`)
    /// calls this unconditionally and seeds the context with whatever it returns,
    /// then hands the page to `StructureTreeStrategy`, whose default fallback for any
    /// untagged page (no `/StructTreeRoot`, the common case) is `XYCutStrategy` --
    /// regardless of what `is_multi_column_page` would have said. So this function's
    /// 12pt corridor floor (`prose_two_column_gutter`) really is looser than
    /// `is_multi_column_page`'s two gates (`has_clean_column_gutter`'s 18pt,
    /// `has_bimodal_line_starts`'s 30pt) on a path that does not consult
    /// `is_multi_column_page` at all first. This is accepted, not unified, for the
    /// same reason `measure_single_central_gutter`'s doc comment gives for not
    /// merging its own 18pt with this function's 12pt: the callers that consume
    /// `Some` here act only through `same_line_span_belongs_to_other_column`, gated
    /// on two spans' tops agreeing to 1pt (see `detect_column_gutter_rejects_*`
    /// below for the None-contract this and the sub-detectors keep instead) -- a
    /// false-positive 12-30pt gutter costs one heading-run merge decision on that
    /// page, not a reordering of its content. ~keep
    ///
    /// Consumed by the XY-cut's heading-run pre-pass, which needs to tell a
    /// heading opening the other column apart from a second `Tj` segment of
    /// the same heading line (GH#1757). Every XY-cut entry point on an output
    /// path must pass this, not `None`: `find_heading_runs` runs about a dozen
    /// times per page from several call sites, and threading only one of them
    /// leaves the defect reachable through the others. ~keep
    pub(crate) fn detect_column_gutter(spans: &[crate::layout::TextSpan]) -> Option<f32> {
        Self::prose_two_column_gutter(spans)
            .or_else(|| Self::density_central_gutter(spans))
            .or_else(|| Self::classifier_column_gutter(spans))
            // GH#1809: a fourth question the three detectors above never ask. All three
            // are MASS detectors: left-edge clusters each holding a share of the page's
            // spans, central density mass, or a whole-page classifier verdict. A page
            // whose right column fills only its top third presents none of that and gets
            // `None` three times, although the corridor between its two columns is empty
            // from the top of the page to the bottom. ~keep
            .or_else(|| Self::line_corridor_gutter(spans))
    }

    /// The page's sole empty x-corridor with a real population of spans on each flank,
    /// or `None`.
    ///
    /// GH#1809. Unlike the three detectors `detect_column_gutter` tries first, this asks
    /// nothing about MASS -- how much of the page a side holds, or how its spans cluster
    /// -- only whether an x-interval is empty of ink and how many spans flank it. It
    /// declines on a single column (no corridor), on a grid or table (a census of the
    /// page's own rows, below), and on a label/value form (its one corridor is wider
    /// than a gutter can be relative to the content it splits). ~keep
    pub(super) fn line_corridor_gutter(spans: &[crate::layout::TextSpan]) -> Option<f32> {
        // Matches `prose_two_column_gutter`'s own 12pt corridor floor, and doubles as the
        // gap width the grid-row census counts: `pdf::native::text::line_has_grid_row_gaps`
        // resolves to 11.9pt at A4 width, so the two agree on every gap either would
        // classify on a real page. ~keep
        const MIN_GUTTER_PTS: f32 = 12.0;
        // A corridor wider than this share of the content it splits is not a gutter but
        // the space inside a label/value form -- the reporter's own corridor is 41pt of
        // 521pt of content, a form's runs to ~230pt of the same width. ~keep
        const MAX_GUTTER_CONTENT_FRACTION: f32 = 0.25;
        // A span this wide relative to the content it sits in is full-width furniture (a
        // running header, a banner) and straddles any real corridor by construction --
        // the same bridge exclusion `prose_two_column_gutter` applies at its own 60%. ~keep
        const FURNITURE_CONTENT_FRACTION: f32 = 0.6;
        const MIN_SPANS_PER_SIDE: usize = 4;
        // A line whose ink opens this many internal gaps is a multi-column TABLE ROW,
        // never a body line -- `line_has_grid_row_gaps`' own threshold (GH#1742/#1756).
        // One such line is a kerning accident; two that agree are a grid, and on a grid
        // this detector declines OUTRIGHT rather than merely skipping those rows. Merely
        // skipping them is not enough, measured: with GH#1756's 27 journal-table rows
        // skipped, its page still leaves exactly one qualifying corridor (291.0..306.6,
        // the genuine gutter) and the detector answers `Some(298.8)` where
        // `detect_column_gutter_also_declines_on_the_sparse_table_rows_pages_gh1756`
        // requires `None`. Declining on the census instead keeps every grid page -- that
        // one, and the 38-row ACN financial table -- on the behaviour it has today. ~keep
        const MIN_GRID_ROW_GAP_COUNT: usize = 4;
        const MIN_GRID_ROWS_FOR_A_TABLE: usize = 2;

        let lines = Self::corridor_lines(spans);
        let grid_rows = lines
            .iter()
            .filter(|line| {
                Self::corridor_line_gap_count(spans, line.as_slice(), MIN_GUTTER_PTS) >= MIN_GRID_ROW_GAP_COUNT
            })
            .count();
        if grid_rows >= MIN_GRID_ROWS_FOR_A_TABLE {
            return None;
        }

        let mut extents: Vec<(f32, f32)> = lines
            .iter()
            .flatten()
            .map(|&index| (spans[index].bbox.left(), spans[index].bbox.right()))
            .filter(|&(left, right)| left.is_finite() && right.is_finite() && right > left)
            .collect();
        if extents.len() < MIN_SPANS_PER_SIDE * 2 {
            return None;
        }
        let content_min = extents.iter().map(|&(left, _)| left).fold(f32::INFINITY, f32::min);
        let content_max = extents
            .iter()
            .map(|&(_, right)| right)
            .fold(f32::NEG_INFINITY, f32::max);
        let content_width = content_max - content_min;
        if !(content_width.is_finite() && content_width > 0.0) {
            return None;
        }
        extents.retain(|&(left, right)| right - left < content_width * FURNITURE_CONTENT_FRACTION);
        if extents.len() < MIN_SPANS_PER_SIDE * 2 {
            return None;
        }
        Self::sole_corridor_midpoint(
            &mut extents,
            MIN_GUTTER_PTS,
            content_width * MAX_GUTTER_CONTENT_FRACTION,
            MIN_SPANS_PER_SIDE,
        )
    }

    /// `spans`' non-blank indices grouped into baseline runs, each sorted left to right.
    ///
    /// Mirrors `pdf::native::text::group_into_lines`, tolerance included: a new line
    /// opens when a span's `y` differs from its group's ANCHOR by more than the
    /// tolerance, so gradual drift across a page does not chain into one line. ~keep
    fn corridor_lines(spans: &[crate::layout::TextSpan]) -> Vec<Vec<usize>> {
        const LINE_Y_TOLERANCE_PTS: f32 = 0.5;

        let mut order: Vec<usize> = (0..spans.len()).collect();
        order.retain(|&index| !spans[index].text.trim().is_empty());
        order.sort_by(|&a, &b| {
            spans[b]
                .bbox
                .y
                .total_cmp(&spans[a].bbox.y)
                .then_with(|| spans[a].bbox.x.total_cmp(&spans[b].bbox.x))
        });
        let mut lines: Vec<Vec<usize>> = Vec::new();
        let mut anchor_y = f32::NAN;
        for index in order {
            let y = spans[index].bbox.y;
            if lines.is_empty() || (anchor_y - y).abs() > LINE_Y_TOLERANCE_PTS {
                anchor_y = y;
                lines.push(Vec::new());
            }
            lines.last_mut().expect("just pushed above").push(index);
        }
        for line in &mut lines {
            line.sort_by(|&a, &b| spans[a].bbox.x.total_cmp(&spans[b].bbox.x));
        }
        lines
    }

    /// How many gaps of at least `min_gutter` `line`'s ink opens between its own spans.
    ///
    /// A zero-width span counts and holds its x: the empty cell of a table row is marked
    /// with a U+200B, which `trim` does not remove, and dropping it would close the gap
    /// that makes the row a grid row (GH#1756). ~keep
    fn corridor_line_gap_count(spans: &[crate::layout::TextSpan], line: &[usize], min_gutter: f32) -> usize {
        let mut gaps = 0usize;
        let mut running_right = f32::NEG_INFINITY;
        for &index in line {
            let (left, right) = (spans[index].bbox.left(), spans[index].bbox.right());
            if running_right.is_finite() && left - running_right >= min_gutter {
                gaps += 1;
            }
            running_right = running_right.max(right);
        }
        gaps
    }

    /// The midpoint of the one corridor at least `min_gutter` and at most `max_gutter`
    /// wide that `extents` leaves, when there is exactly one and both flanks carry at
    /// least `min_per_side` spans.
    ///
    /// Exactly one is the discriminator, not the widest: a page with several qualifying
    /// corridors is a form or a grid, and picking one of them would place a gutter inside
    /// its content (GH#1809). ~keep
    fn sole_corridor_midpoint(
        extents: &mut [(f32, f32)],
        min_gutter: f32,
        max_gutter: f32,
        min_per_side: usize,
    ) -> Option<f32> {
        extents.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut corridors: Vec<(f32, f32)> = Vec::new();
        let mut running_right = extents.first()?.1;
        for &(left, right) in &extents[1..] {
            if left - running_right >= min_gutter {
                corridors.push((running_right, left));
            }
            running_right = running_right.max(right);
        }
        // Every corridor counts toward the "exactly one" test, `max_gutter` is applied
        // only afterwards: a second corridor is evidence of a form whatever its width, so
        // measuring the width first would let a form through on its narrow corridor while
        // its wide one was discarded unseen. ~keep
        if corridors.len() != 1 {
            return None;
        }
        let (left, right) = corridors[0];
        if right - left > max_gutter {
            return None;
        }
        let left_count = extents
            .iter()
            .filter(|&&(_, extent_right)| extent_right <= left)
            .count();
        let right_count = extents.iter().filter(|&&(extent_left, _)| extent_left >= right).count();
        (left_count >= min_per_side && right_count >= min_per_side).then_some((left + right) / 2.0)
    }

    pub(super) fn is_multi_column_page(spans: &[crate::layout::TextSpan]) -> bool {
        // Clean-gutter detector (handles short pages the histogram gates below
        // reject for lack of spans). A genuine empty vertical channel that no
        // span crosses, with multi-line content of overlapping vertical extent
        // on both sides, is the unambiguous geometric signature of side-by-side
        // columns — recoverable for untagged pages only from layout (XY-Cut,
        // ISO 32000-1 §9.4, since there is no logical-structure hint). ~keep
        if Self::has_clean_column_gutter(spans) {
            return true;
        }

        if spans.len() < 12 {
            return false;
        }

        // Primary detector: line-start-X bimodality.
        //
        // The span-center histogram further down is noisy for word-level
        // spans (every X position has many word starts on multi-word
        // body-text lines). The reliable signal is the X position at
        // which each *line* begins — a two-column body has a strong
        // peak at the left-column-start X plus a strong peak at the
        // right-column-start X, with a clear empty gutter between
        // them. We cluster spans into lines by Y (1pt tolerance), pick
        // the leftmost X per line, and look for ≥ 2 peaks separated by
        // a gutter of ≥ 30pt with zero line-starts in it. ~keep
        if Self::has_bimodal_line_starts(spans) {
            return true;
        }

        let mut x_centers: Vec<f32> = spans.iter().map(|s| s.bbox.x + s.bbox.width * 0.5).collect();
        x_centers.sort_by(|a, b| crate::utils::safe_float_cmp(*a, *b));

        // Degenerate CTM guard: drop centers more than MAX_EXTENT from the
        // median so a rogue span ~1e16 doesn't explode the histogram. ~keep
        const MAX_EXTENT_FROM_MEDIAN: f32 = 5_000.0;
        let median = x_centers[x_centers.len() / 2];
        x_centers.retain(|c| (*c - median).abs() <= MAX_EXTENT_FROM_MEDIAN);
        if x_centers.len() < 12 {
            return false;
        }

        let min = *x_centers.first().unwrap();
        let max = *x_centers.last().unwrap();
        let width = max - min;
        if width < 100.0 {
            return false;
        }

        // Bin into 40 buckets; find peaks (≥ mean × 1.5) separated by at
        // least one empty bucket. ~keep
        const BUCKETS: usize = 40;
        let bucket_width = width / BUCKETS as f32;
        if bucket_width <= 0.0 {
            return false;
        }
        let mut hist = [0usize; BUCKETS];
        for c in &x_centers {
            let idx = (((c - min) / bucket_width) as usize).min(BUCKETS - 1);
            hist[idx] += 1;
        }

        let total: usize = hist.iter().sum();
        let mean = total as f32 / BUCKETS as f32;
        let threshold = (mean * 1.5).max(3.0);

        let mut peaks = 0usize;
        let mut in_peak = false;
        for &count in &hist {
            if count as f32 >= threshold {
                if !in_peak {
                    peaks += 1;
                    in_peak = true;
                }
            } else if count == 0 {
                in_peak = false;
            }
        }

        if peaks < 2 {
            return false;
        }

        // Confirmation: the peaks must have vertical overlap. If one "column"
        // is a footer and the other is the body, they don't interact — row-
        // aware is fine. Split spans into left-half vs right-half and check ~keep
        // their Y ranges overlap.
        let mid_x = (min + max) / 2.0;
        let mut left_y_min = f32::INFINITY;
        let mut left_y_max = f32::NEG_INFINITY;
        let mut right_y_min = f32::INFINITY;
        let mut right_y_max = f32::NEG_INFINITY;
        for s in spans {
            let cx = s.bbox.x + s.bbox.width * 0.5;
            if (cx - median).abs() > MAX_EXTENT_FROM_MEDIAN {
                continue;
            }
            let y_top = s.bbox.y + s.bbox.height;
            if cx < mid_x {
                left_y_min = left_y_min.min(s.bbox.y);
                left_y_max = left_y_max.max(y_top);
            } else {
                right_y_min = right_y_min.min(s.bbox.y);
                right_y_max = right_y_max.max(y_top);
            }
        }
        let left_span = (left_y_max - left_y_min).max(0.0);
        let right_span = (right_y_max - right_y_min).max(0.0);
        let overlap = left_y_max.min(right_y_max) - left_y_min.max(right_y_min);
        let min_span = left_span.min(right_span);
        if !(min_span > 0.0 && overlap > 0.5 * min_span) {
            return false;
        }

        // Require each half to contain enough spans to represent genuine body
        // text columns. Copyright pages, title pages, and other sparse layouts
        // can produce two X-center peaks with only 2–7 spans per "column" —
        // these are not true multi-column body text. ~keep
        let left_count = spans
            .iter()
            .filter(|s| {
                let cx = s.bbox.x + s.bbox.width * 0.5;
                (cx - median).abs() <= MAX_EXTENT_FROM_MEDIAN && cx < mid_x
            })
            .count();
        let right_count = spans.len() - left_count;
        if left_count.min(right_count) < 15 {
            return false;
        }

        // Font-aware column-shape gate.
        //
        // Real two-column body text has tight column-edge alignment:
        // most spans on each side share one dominant X position
        // (the column start), with a handful of indented or
        // section-header outliers. Scattered-fragment layouts spread
        // their spans evenly across many X positions on each side.
        //
        // Measure the fraction of side-spans that fall into the
        // largest X-cluster (cluster gap = `dominant_em`). Body text
        // typically scores ≥ 0.5; scattered layouts score < 0.4.
        // Reject pages where either side fails the threshold so ~keep
        // XY-cut doesn't mis-route scattered content as multi-column.
        let stats = crate::layout::PageFontStats::from_spans(spans);
        let cluster_gap = stats.dominant_em.max(4.0);
        let dominant_cluster_fraction = |take: &dyn Fn(f32) -> bool| -> f32 {
            let mut xs: Vec<f32> = spans
                .iter()
                .filter(|s| {
                    let cx = s.bbox.x + s.bbox.width * 0.5;
                    (cx - median).abs() <= MAX_EXTENT_FROM_MEDIAN && take(cx)
                })
                .map(|s| s.bbox.x)
                .collect();
            let total = xs.len();
            if total == 0 {
                return 0.0;
            }
            xs.sort_by(|a, b| crate::utils::safe_float_cmp(*a, *b));
            let mut best = 1usize;
            let mut current = 1usize;
            let mut last = xs[0];
            for &x in &xs[1..] {
                if x - last <= cluster_gap {
                    current += 1;
                    if current > best {
                        best = current;
                    }
                } else {
                    current = 1;
                }
                last = x;
            }
            best as f32 / total as f32
        };
        const MIN_DOMINANT_FRACTION: f32 = 0.5;
        let left_frac = dominant_cluster_fraction(&|cx| cx < mid_x);
        let right_frac = dominant_cluster_fraction(&|cx| cx >= mid_x);
        if left_frac >= MIN_DOMINANT_FRACTION && right_frac >= MIN_DOMINANT_FRACTION {
            return true;
        }

        // Additive accept path (no change to the gate above): shared-baseline
        // two-column bodies — academic references / bibliographies — read
        // left+right on the SAME Y line, so the row-aware sort interleaves
        // them. Their word-granular left edges scatter, so the dominant-
        // cluster gate above misses them. But they exhibit ONE persistent
        // vertical gutter corridor (the signal poppler/MuPDF use, independent
        // of line length). Detect it via within-line gap projection, prose-
        // guarded so numeric / short-cell tables — which also reach here —
        // stay on the row-aware path. ~keep
        Self::has_persistent_gutter_corridor(spans, median, MAX_EXTENT_FROM_MEDIAN)
    }

    /// Detect a single persistent vertical gutter corridor across the page —
    /// the geometric fingerprint of a two-column prose body whose columns
    /// share Y baselines (so `has_bimodal_line_starts` and the dominant-
    /// cluster gate both miss it). Mirrors `detect_narrow_gutter_prose`
    /// (`src/pipeline/reading_order/xycut.rs`) at the document-routing layer.
    ///
    /// Table-safe by construction. Long-line bodies
    /// (`mean non-whitespace chars per line > 20`) keep the original
    /// concentration / coverage / centre accept path. Short-line bodies
    /// (verse / lexicon editions) are admitted only under stricter,
    /// length-independent guards a numeric / short-cell table cannot satisfy:
    /// higher concentration and coverage, left/right column char-mass balance,
    /// and a grid-row signal (a multi-cell table has ≥ 2 wide gaps on most
    /// rows; a two-column body has one gutter). Full-width display-math /
    /// heading rows are excluded from the gutter-coverage denominator so a
    /// minority of them does not veto an otherwise two-column page.
    pub(super) fn has_persistent_gutter_corridor(
        spans: &[crate::layout::TextSpan],
        median: f32,
        max_extent: f32,
    ) -> bool {
        // Group spans into lines by rounded Y baseline; carry left/right
        // extents for gap projection and char count for the prose guard. ~keep
        let mut lines: std::collections::BTreeMap<i32, (Vec<(f32, f32)>, usize)> = std::collections::BTreeMap::new();
        let mut x_min = f32::MAX;
        let mut x_max = f32::MIN;
        for s in spans {
            let cx = s.bbox.x + s.bbox.width * 0.5;
            if (cx - median).abs() > max_extent {
                continue;
            }
            let y_key = (s.bbox.y + s.bbox.height).round() as i32;
            let entry = lines.entry(y_key).or_default();
            entry.0.push((s.bbox.x, s.bbox.x + s.bbox.width));
            entry.1 += s.text.chars().filter(|c| !c.is_whitespace()).count();
            x_min = x_min.min(s.bbox.x);
            x_max = x_max.max(s.bbox.x + s.bbox.width);
        }
        let region_width = x_max - x_min;
        if lines.len() < 12 || region_width < 200.0 {
            return false;
        }

        let total_chars: usize = lines.values().map(|(_, c)| *c).sum();
        let mean_chars = total_chars as f32 / lines.len() as f32;

        // Largest within-line gap per line (≥ 6 pt suppresses word spacing);
        // record the gap midpoint X. Also flag full-width lines with no internal
        // gutter (display equations, full-width headings) so they neither support
        // nor veto the corridor — they are excluded from the coverage denominator
        // (Part 1b: display-math robustness, arxiv_math). ~keep
        const MIN_GAP_PT: f32 = 6.0;
        let mut gap_positions: Vec<f32> = Vec::new();
        let mut full_width_lines = 0usize;
        let mut multi_gap_lines = 0usize;
        for (line_spans, _) in lines.values() {
            if line_spans.is_empty() {
                continue;
            }
            let mut sorted = line_spans.clone();
            sorted.sort_by(|a, b| crate::utils::safe_float_cmp(a.0, b.0));
            let line_left = sorted.first().map(|s| s.0).unwrap_or(0.0);
            let line_right = sorted.last().map(|s| s.1).unwrap_or(0.0);
            let mut largest_gap = 0.0_f32;
            let mut largest_mid = 0.0_f32;
            let mut significant_gaps = 0usize;
            for w in sorted.windows(2) {
                let gap = w[1].0 - w[0].1;
                if gap >= MIN_GAP_PT {
                    significant_gaps += 1;
                }
                if gap > largest_gap {
                    largest_gap = gap;
                    largest_mid = (w[0].1 + w[1].0) * 0.5;
                }
            }
            if (line_right - line_left) >= region_width * 0.9 && largest_gap < MIN_GAP_PT {
                full_width_lines += 1;
            }
            // A line with two or more wide internal gaps is a grid row (≥ 3
            // cells), not a two-column body line (one gutter). Used by the
            // short-line table discriminator below. ~keep
            if significant_gaps >= 2 {
                multi_gap_lines += 1;
            }
            if largest_gap >= MIN_GAP_PT {
                gap_positions.push(largest_mid);
            }
        }
        if gap_positions.len() < 12 {
            return false;
        }
        let eff_lines = lines.len().saturating_sub(full_width_lines).max(1);

        // Cluster gap midpoints (10 pt radius); find the dominant corridor. ~keep
        const CLUSTER_RADIUS_PT: f32 = 10.0;
        gap_positions.sort_by(|a, b| crate::utils::safe_float_cmp(*a, *b));
        let mut best_size = 0usize;
        let mut best_center = 0.0_f32;
        let mut left = 0usize;
        let mut right = 0usize;
        let mut prefix: Vec<f32> = Vec::with_capacity(gap_positions.len() + 1);
        prefix.push(0.0);
        for &x in &gap_positions {
            prefix.push(prefix.last().unwrap() + x);
        }
        for &pivot in &gap_positions {
            while left < gap_positions.len() && gap_positions[left] < pivot - CLUSTER_RADIUS_PT {
                left += 1;
            }
            while right < gap_positions.len() && gap_positions[right] <= pivot + CLUSTER_RADIUS_PT {
                right += 1;
            }
            let count = right - left;
            if count > best_size {
                best_size = count;
                best_center = (prefix[right] - prefix[left]) / count as f32;
            }
        }

        // Gutter must sit near the page centre (0.30–0.70). A true two-column
        // body splits down the middle; a table's dominant gap (label column vs
        // data, or one of several cell boundaries) sits off-centre. ~keep
        let gutter_offset = best_center - x_min;
        let centre_ok = gutter_offset >= region_width * 0.30 && gutter_offset <= region_width * 0.70;
        if best_size < 16 || !centre_ok {
            return false;
        }

        if mean_chars > 20.0 {
            // Long-line two-column prose (the accept path, unchanged
            // except the coverage denominator now excludes display rows):
            // concentration ≥ 62 %, coverage ≥ 50 % of (effective) lines. ~keep
            return best_size * 50 >= gap_positions.len() * 31 && best_size * 2 >= eff_lines;
        }

        // Short-line bodies (verse / lexicon / dictionary editions): the
        // raw `mean_chars` floor used to reject these along with short-cell
        // tables. Admit them only under STRICTER, length-independent guards a
        // short-cell table cannot satisfy (Part 1a). ~keep
        let strict_concentration = best_size * 10 >= gap_positions.len() * 7;
        let strict_coverage = best_size * 5 >= eff_lines * 3;
        if !(strict_concentration && strict_coverage) {
            return false;
        }
        // Column char-mass balance: each side of the gutter must carry ≥ 35 % of
        // the non-whitespace characters. A narrow label / verse-number column
        // paired with wide data is lopsided and rejected. ~keep
        let (mut left_chars, mut right_chars) = (0usize, 0usize);
        for s in spans {
            let cx = s.bbox.x + s.bbox.width * 0.5;
            if (cx - median).abs() > max_extent {
                continue;
            }
            let n = s.text.chars().filter(|c| !c.is_whitespace()).count();
            if cx < best_center {
                left_chars += n;
            } else {
                right_chars += n;
            }
        }
        let total = (left_chars + right_chars).max(1) as f32;
        if (left_chars as f32) < total * 0.35 || (right_chars as f32) < total * 0.35 {
            return false;
        }
        // Grid-row discriminator: a two-column body has ONE wide gap per line
        // (the gutter); a multi-cell numeric table has ≥ 2 wide gaps on most
        // rows (cell boundaries). Reject when the majority of lines are grid
        // rows — this is what keeps short-cell tables off the XY-cut path
        // without the raw `mean_chars` floor that also blocked short verse. ~keep
        multi_gap_lines * 2 <= eff_lines
    }

    /// RW-1: reading order for a **narrow-sidebar + wide-body** page (the MDPI /
    /// academic first-page layout: a full-width title band on top, a narrow left
    /// metadata column — Citation / Editor / Received / copyright — beside a wide
    /// body column). `is_multi_column_page` misreads this as two balanced columns
    /// and the XY-cut then slices the full-width title along the body gutter
    /// (§14.8.3: a block-level full-width element must NOT be column-assigned).
    ///
    /// Returns `Some(reordered_spans)` only when the layout is *confidently* this
    /// shape — emit order **band (title) + body, merged top→bottom, then the
    /// sidebar last** (the gold puts the title whole on top, then the body; the
    /// metadata sidebar is publisher furniture read last). Returns `None`
    /// otherwise so the normal XY-cut / row-aware path is unchanged. The gate is
    /// deliberately tight (gutter left-of-centre + a narrow left column + a wide
    /// right column + a full-width band near the top) so balanced two-column and
    /// single-column pages never reach it.
    pub(super) fn sidebar_body_reading_order(
        spans: &[crate::layout::TextSpan],
    ) -> Option<Vec<crate::layout::TextSpan>> {
        use crate::utils::safe_float_cmp;
        if spans.len() < 30 {
            return None;
        }
        let x_min = spans.iter().map(|s| s.bbox.left()).fold(f32::MAX, f32::min);
        let x_max = spans.iter().map(|s| s.bbox.right()).fold(f32::MIN, f32::max);
        let y_min = spans.iter().map(|s| s.bbox.top()).fold(f32::MAX, f32::min);
        let y_max = spans.iter().map(|s| s.bbox.bottom()).fold(f32::MIN, f32::max);
        let width = (x_max - x_min).max(1.0);
        let height = (y_max - y_min).max(1.0);
        if !(width.is_finite() && height.is_finite()) {
            return None;
        }

        let mut order: Vec<usize> = (0..spans.len()).collect();
        order.sort_by(|&a, &b| {
            safe_float_cmp(spans[b].bbox.bottom(), spans[a].bbox.bottom())
                .then_with(|| safe_float_cmp(spans[a].bbox.left(), spans[b].bbox.left()))
        });
        struct Line {
            top_y: f32,
            min_left: f32,
            max_right: f32,
            members: Vec<usize>,
        }
        let mut lines: Vec<Line> = Vec::new();
        for &i in &order {
            let s = &spans[i];
            let h = (s.bbox.bottom() - s.bbox.top()).abs().max(1.0);
            match lines.last_mut() {
                Some(l) if (l.top_y - s.bbox.bottom()).abs() <= h * 0.6 => {
                    l.min_left = l.min_left.min(s.bbox.left());
                    l.max_right = l.max_right.max(s.bbox.right());
                    l.members.push(i);
                }
                _ => lines.push(Line {
                    top_y: s.bbox.bottom(),
                    min_left: s.bbox.left(),
                    max_right: s.bbox.right(),
                    members: vec![i],
                }),
            }
        }
        if lines.len() < 12 {
            return None;
        }

        // Find the body-column left edge: the most common line-start that sits
        // well right of the page left margin (excludes the sidebar/title cluster
        // anchored at x_min). The gutter is just left of it. ~keep
        let body_left = {
            const BIN: f32 = 5.0;
            let nbins = ((width / BIN).ceil() as usize).clamp(1, 4096);
            let mut hist = vec![0usize; nbins];
            for l in &lines {
                if l.min_left > x_min + width * 0.10 {
                    let b = (((l.min_left - x_min) / BIN) as usize).min(nbins - 1);
                    hist[b] += 1;
                }
            }
            let (peak, &cnt) = hist.iter().enumerate().max_by_key(|&(_, &c)| c)?;
            if cnt < 5 {
                return None;
            }
            x_min + peak as f32 * BIN
        };
        // Gutter must be left-of-centre (a narrow sidebar, not a centred 2-col). ~keep
        let gutter = body_left - width * 0.02;
        if !(gutter > x_min + width * 0.12 && gutter < x_min + width * 0.45) {
            return None;
        }

        // A full-width TITLE/heading row is typeset as many narrow word spans, so
        // no single span satisfies the per-span band test below; its leftmost words
        // (right edge ≤ gutter) would be miswept into the SIDEBAR and emitted last,
        // shattering the title (e.g. an MDPI first page where the title sits in a
        // large font across the full width, above the metadata sidebar + body).
        // Detect these per LINE: a baseline line whose member words FLOW
        // CONTINUOUSLY across the gutter (collective extent crosses it, ≥40% of the
        // page width, and no large internal gap straddling the gutter) is a true
        // full-width band — its words are evenly spaced, unlike a sidebar-label +
        // body-line that merely SHARE a baseline (e.g. "Accepted: 1 March 2021"
        // next to "1. Introduction"), which leaves a wide empty gutter corridor
        // between the two columns. Members of such band lines are forced into the
        // BAND group so the whole title row stays together at its vertical slot.
        // The straddling-gap gate keeps shared sidebar/body baselines split. ~keep
        let mut band_line_members: std::collections::HashSet<usize> = std::collections::HashSet::new();
        const MAX_STRADDLE_GAP_FRAC: f32 = 0.06;
        // The title/author band sits ABOVE the body column (the body starts at the
        // affiliations/abstract). Restrict band promotion to lines above the
        // topmost PURE-BODY line (a line whose words all begin at/right of the
        // gutter, with no left-of-gutter member). Below that Y the page is the
        // two-column sidebar+body region, where a wide crossing line is a
        // sidebar-label + body-line sharing a baseline (e.g. "Switzerland." next to
        // "cancer, atrial fibrillation…") whose tight gutter corridor would
        // otherwise pass the straddle-gap gate and wrongly glue the sidebar inline. ~keep
        // Larger bottom-y == higher on the page (PDF user space). ~keep
        let body_top_y = lines
            .iter()
            .filter(|l| l.min_left >= gutter)
            .map(|l| l.top_y)
            .fold(f32::NEG_INFINITY, f32::max);
        for line in &lines {
            if !(line.min_left < gutter && line.max_right > gutter && (line.max_right - line.min_left) > width * 0.40) {
                continue;
            }
            if body_top_y.is_finite() && line.top_y < body_top_y {
                continue;
            }
            let mut xs: Vec<(f32, f32)> = line
                .members
                .iter()
                .map(|&i| (spans[i].bbox.left(), spans[i].bbox.right()))
                .collect();
            xs.sort_by(|a, b| safe_float_cmp(a.0, b.0));
            let mut max_straddle_gap = 0.0f32;
            let mut prev_right = f32::NEG_INFINITY;
            for &(l, r) in &xs {
                if prev_right.is_finite() && prev_right < gutter && l > gutter {
                    max_straddle_gap = max_straddle_gap.max(l - prev_right);
                }
                prev_right = prev_right.max(r);
            }
            if max_straddle_gap < width * MAX_STRADDLE_GAP_FRAC {
                for &i in &line.members {
                    band_line_members.insert(i);
                }
            }
        }

        // Classify each SPAN by the gutter. A publisher-metadata sidebar and the
        // body usually SHARE baselines (the metadata column interleaves with body
        // lines by Y), so a per-line cluster would fuse them into one full-width
        // line and hide the sidebar — classify per span instead. BAND = a span
        // genuinely spanning the gutter (a wide full-width title/heading), or a
        // member of a continuous full-width band LINE (above). SIDEBAR = a span
        // entirely left of the gutter. BODY = everything at/right of it. ~keep
        let mut band: Vec<usize> = Vec::new();
        let mut sidebar: Vec<usize> = Vec::new();
        let mut body: Vec<usize> = Vec::new();
        for (i, s) in spans.iter().enumerate() {
            let l = s.bbox.left();
            let r = s.bbox.right();
            if band_line_members.contains(&i) || (l < gutter && r > gutter && (r - l) > width * 0.40) {
                band.push(i);
            } else if r <= gutter {
                sidebar.push(i);
            } else {
                body.push(i);
            }
        }
        let line_count = |v: &[usize]| -> usize {
            let mut ys: Vec<f32> = v.iter().map(|&i| spans[i].bbox.bottom()).collect();
            ys.sort_by(|a, b| safe_float_cmp(*a, *b));
            ys.dedup_by(|a, b| (*a - *b).abs() <= 2.0);
            ys.len()
        };
        if line_count(&sidebar) < 5 || line_count(&body) < 8 {
            return None;
        }
        let col_width = |v: &[usize]| -> f32 {
            let lo = v.iter().map(|&i| spans[i].bbox.left()).fold(f32::MAX, f32::min);
            let hi = v.iter().map(|&i| spans[i].bbox.right()).fold(f32::MIN, f32::max);
            (hi - lo).max(0.0)
        };
        let sw = col_width(&sidebar);
        let bw = col_width(&body);
        if sw >= width * 0.45 || sw >= bw * 0.70 {
            return None;
        }
        // ANTI-FORM discriminator. A bare narrow left column is geometrically
        // indistinguishable from a label:value form (Name:/Address:/Date:) or a
        // verse/margin-note page, and these PDFs carry NO background tint to anchor
        // the sidebar. The reliable signal is semantic: a publisher-metadata
        // sidebar carries recognisable furniture labels that never head a form
        // field or a body column. Require >=2 DISTINCT labels so ordinary narrow
        // columns and forms never engage this reordering. ~keep
        let side_text: String = {
            let mut t = String::new();
            for &i in &sidebar {
                t.push_str(&spans[i].text.to_lowercase());
                t.push(' ');
            }
            t
        };
        const FURNITURE: [&str; 12] = [
            "citation",
            "received",
            "accepted",
            "published",
            "copyright",
            "licensee",
            "academic editor",
            "publisher",
            "doi.org",
            "issn",
            "creative commons",
            "open access",
        ];
        let furniture_hits = FURNITURE.iter().filter(|k| side_text.contains(**k)).count();
        if furniture_hits < 2 {
            return None;
        }

        // Emit: band + body merged top→bottom (title stays on top, body flows,
        // any mid-body full-width element keeps its vertical slot), then the
        // sidebar furniture last. Spans within a line read left→right. ~keep
        let mut main: Vec<usize> = band;
        main.extend(body);
        let key = |idx: &usize| {
            let s = &spans[*idx];
            (s.bbox.bottom(), s.bbox.left())
        };
        main.sort_by(|a, b| {
            let (ay, ax) = key(a);
            let (by, bx) = key(b);
            safe_float_cmp(by, ay).then_with(|| safe_float_cmp(ax, bx))
        });
        sidebar.sort_by(|a, b| {
            let (ay, ax) = key(a);
            let (by, bx) = key(b);
            safe_float_cmp(by, ay).then_with(|| safe_float_cmp(ax, bx))
        });
        let mut out: Vec<crate::layout::TextSpan> = Vec::with_capacity(spans.len());
        for i in main.into_iter().chain(sidebar) {
            out.push(spans[i].clone());
        }
        Some(out)
    }

    /// Order spans by a topological sort over text BLOCKS (a precede relation),
    /// for pages with genuine side-by-side regions (a two-column body, a
    /// two-column footer, a sidebar beside the body) that a flat row-aware (y,x)
    /// sort interleaves row-by-row (splicing one region's line into the other's).
    /// Returns `None` for any page WITHOUT two horizontally-disjoint,
    /// vertically-overlapping blocks (single-column and simple stacked layouts),
    /// so their output stays byte-identical.
    ///
    /// Coordinate convention (see `row_aware_span_cmp`): larger Y = higher on the
    /// page, read first; `bottom()` is a span's UPPER edge, `top()` its LOWER edge.
    pub(super) fn topological_block_order(spans: &[crate::layout::TextSpan]) -> Option<Vec<crate::layout::TextSpan>> {
        use crate::utils::safe_float_cmp;
        if spans.len() < 8 {
            return None;
        }
        let hi = |s: &crate::layout::TextSpan| s.bbox.bottom();
        let lo = |s: &crate::layout::TextSpan| s.bbox.top();
        let med_h = {
            let mut hs: Vec<f32> = spans
                .iter()
                .map(|s| (hi(s) - lo(s)).abs())
                .filter(|h| h.is_finite() && *h > 0.0)
                .collect();
            if hs.is_empty() {
                return None;
            }
            hs.sort_by(|a, b| safe_float_cmp(*a, *b));
            hs[hs.len() / 2].max(1.0)
        };

        // Item 4 (M3): measure the page's single central column gutter (if any).
        // Used below to forbid a same-line union ACROSS the gutter regardless of
        // the measured gap: on dense two-column pages with tight leading, an
        // over-wide advance can make a cross-gutter gap < med_h, fusing the two
        // columns into one block so the side_by_side gate then declines and the
        // page falls to a row-major interleave. `None` (single-column /
        // multi-corridor / off-centre) ⇒ the predicate is byte-identical. ~keep
        let gutter_x = Self::measure_single_central_gutter(spans).or_else(|| Self::density_central_gutter(spans));

        // --- Union-find: connect spans in the same text region. Two spans join
        // iff they are on the same line and horizontally adjacent (a normal word
        // gap, NOT a column gutter), OR vertically stacked with overlapping X and
        // a small inter-line gap. A column gutter (≥ ~1 em of whitespace) never
        // connects, so left/right columns become separate blocks even when their
        // lines share Y bands. --- ~keep
        let n = spans.len();
        let mut parent: Vec<usize> = (0..n).collect();
        fn find(parent: &mut [usize], mut x: usize) -> usize {
            while parent[x] != x {
                parent[x] = parent[parent[x]];
                x = parent[x];
            }
            x
        }
        // Index spans by reading order so each only tests a local window. ~keep
        let mut ord: Vec<usize> = (0..n).collect();
        ord.sort_by(|&a, &b| {
            safe_float_cmp(hi(&spans[b]), hi(&spans[a]))
                .then_with(|| safe_float_cmp(spans[a].bbox.left(), spans[b].bbox.left()))
        });
        let x_overlap = |a: &crate::layout::TextSpan, b: &crate::layout::TextSpan| -> f32 {
            (a.bbox.right().min(b.bbox.right()) - a.bbox.left().max(b.bbox.left())).max(0.0)
        };
        for (p, &i) in ord.iter().enumerate() {
            let si = &spans[i];
            for &j in ord.iter().skip(p + 1).take(40) {
                let sj = &spans[j];
                let dy_centers = ((hi(si) + lo(si)) - (hi(sj) + lo(sj))).abs() * 0.5;
                let same_line = dy_centers < med_h * 0.5;
                let connect = if same_line {
                    // Horizontal neighbour: gap below ~1 em (word space), not a gutter. ~keep
                    let gap = (si.bbox.left().max(sj.bbox.left())) - (si.bbox.right().min(sj.bbox.right()));
                    // Item 4 (M3): never join two spans that straddle the measured
                    // central gutter (one wholly left of it, the other wholly
                    // right), independent of `gap` — a tight-leading over-wide
                    // advance can otherwise make the cross-gutter gap < med_h and
                    // fuse the columns. Purely subtractive: it can only PREVENT a
                    // union, never create one, so `gutter_x == None` is byte-identical. ~keep
                    let crosses_gutter = gutter_x.is_some_and(|gx| {
                        (si.bbox.right() <= gx && sj.bbox.left() >= gx)
                            || (sj.bbox.right() <= gx && si.bbox.left() >= gx)
                    });
                    !crosses_gutter && gap < med_h * 1.0
                } else {
                    let vgap = (lo(si).min(lo(sj)) - hi(si).max(hi(sj))).abs();
                    let near = (lo(si) - hi(sj)).abs().min((lo(sj) - hi(si)).abs());
                    x_overlap(si, sj) > med_h * 0.3 && near < med_h * 1.5 && vgap < med_h * 6.0
                };
                if connect {
                    let (ri, rj) = (find(&mut parent, i), find(&mut parent, j));
                    if ri != rj {
                        parent[ri] = rj;
                    }
                }
            }
        }

        // --- Build blocks from the union-find components. A BTreeMap (not a
        // HashMap) keys the components by root index so `into_values()` is
        // DETERMINISTIC — HashMap iteration order is randomized per run, which
        // would make the block order (and thus the extracted text) flaky for
        // pages where two blocks tie on the seed sort key. --- ~keep
        use std::collections::BTreeMap;
        let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for i in 0..n {
            let r = find(&mut parent, i);
            groups.entry(r).or_default().push(i);
        }
        struct Block {
            x0: f32,
            x1: f32,
            y_hi: f32,
            y_lo: f32,
            members: Vec<usize>,
        }
        let blocks: Vec<Block> = groups
            .into_values()
            .map(|members| {
                let mut b = Block {
                    x0: f32::MAX,
                    x1: f32::MIN,
                    y_hi: f32::MIN,
                    y_lo: f32::MAX,
                    members,
                };
                for &i in &b.members {
                    let s = &spans[i];
                    b.x0 = b.x0.min(s.bbox.left());
                    b.x1 = b.x1.max(s.bbox.right());
                    b.y_hi = b.y_hi.max(hi(s));
                    b.y_lo = b.y_lo.min(lo(s));
                }
                b
            })
            .collect();
        if blocks.len() < 2 {
            return None;
        }

        // A STRUCTURED/TABULAR page (a chess-move table, a data grid, a TOC with a
        // page-number rail) shatters into many tiny fragment blocks the union-find
        // cannot coalesce — isolated cells, page numbers, single-letter column
        // heads — interleaved with the column runs. Flowing multi-column prose and
        // a sidebar+body do not: they are a handful of big blocks. So when fragment
        // blocks (< 4 spans) outnumber the substantial ones, the page is tabular
        // and must stay row-aware, NOT be read column-major. ~keep
        let fragments = blocks.iter().filter(|b| b.members.len() < 4).count();
        if fragments > blocks.len() - fragments {
            return None;
        }

        // Item 4 follow-up (M3): if the page has a clean central column gutter but
        // the union-find STILL produced a block that fuses the two columns across
        // it, the topological emit would interleave them (the block's spans get
        // sorted row-aware within the block). This happens on dense two-column
        // bodies where a producer-malformed full-width fragment, or a chain of
        // vertical unions through a wide line, bridges the columns despite the
        // same-line cross-gutter veto. A correct two-column block decomposition
        // has NO block straddling the gutter with substantial content on BOTH
        // sides. When one exists, bail to None so the dispatch falls through to
        // the band-aware column-major reader (`classifier_column_gutter` /
        // `reorder_column_major_with_bands`), which separates the columns and
        // re-emits full-width bands at their own Y. Gated on a measured gutter, so
        // pages without one (the common case) are byte-identical. ~keep
        if let Some(gx) = gutter_x {
            let fused = blocks.iter().any(|b| {
                if b.x0 >= gx - med_h || b.x1 <= gx + med_h {
                    return false;
                }
                let mut l = 0usize;
                let mut r = 0usize;
                for &i in &b.members {
                    let s = &spans[i];
                    if s.bbox.right() <= gx {
                        l += 1;
                    } else if s.bbox.left() >= gx {
                        r += 1;
                    }
                }
                // Substantial content on BOTH sides ⇒ fused columns, not a band. ~keep
                l >= 4 && r >= 4
            });
            if fused {
                return None;
            }
        }

        // --- GATE: require ≥2 blocks that are horizontally DISJOINT yet overlap
        // in Y (genuine side-by-side regions). Single-column / stacked layouts
        // have none, so they return None and stay byte-identical. --- ~keep
        let y_ov = |a: &Block, b: &Block| (a.y_hi.min(b.y_hi) - a.y_lo.max(b.y_lo)) > med_h * 0.5;
        let x_disjoint = |a: &Block, b: &Block| a.x1 <= b.x0 || b.x1 <= a.x0;
        // Character density per block (≈ chars per text line). A page-number
        // column in a TOC, or the value column of a label:value form/table, is
        // text-SPARSE (a few chars per line); genuine prose columns and a
        // publisher metadata sidebar are text-DENSE. Used to reject row-paired
        // tables/TOCs/forms (which must read row-wise, NOT column-major). ~keep
        let char_density = |b: &Block| -> f32 {
            let mut ys: Vec<f32> = b.members.iter().map(|&i| hi(&spans[i])).collect();
            ys.sort_by(|p, q| safe_float_cmp(*p, *q));
            let mut lines = 1usize;
            for w in ys.windows(2) {
                if (w[1] - w[0]).abs() > med_h * 0.6 {
                    lines += 1;
                }
            }
            let chars: usize = b.members.iter().map(|&i| spans[i].text.trim().chars().count()).sum();
            chars as f32 / lines as f32
        };
        // Number of baseline rows a block spans (same Y-clustering as char_density). ~keep
        let block_lines = |b: &Block| -> usize {
            let mut ys: Vec<f32> = b.members.iter().map(|&i| hi(&spans[i])).collect();
            ys.sort_by(|p, q| safe_float_cmp(*p, *q));
            let mut lines = 1usize;
            for w in ys.windows(2) {
                if (w[1] - w[0]).abs() > med_h * 0.6 {
                    lines += 1;
                }
            }
            lines
        };

        // Both side-by-side blocks must be SUBSTANTIAL, text-DENSE, multi-line
        // regions that overlap over several lines — a genuine 2-column body/footer
        // or a sidebar+body. Incidental overlaps (a drop cap, a page number, a
        // margin note, a fragmented poem line) involve tiny blocks or a sliver of
        // Y-overlap; row-paired tables/TOCs/forms have a text-sparse value column.
        // Neither must engage the reorder, or single-column poetry, decorated
        // pages, and TOCs scramble. ~keep
        let side_by_side = blocks.iter().enumerate().any(|(i, a)| {
            blocks.iter().skip(i + 1).any(|b| {
                x_disjoint(a, b)
                    && a.members.len() >= 8
                    && b.members.len() >= 8
                    && (a.y_hi.min(b.y_hi) - a.y_lo.max(b.y_lo)) > med_h * 3.0
                    // Each side must be a genuine MULTI-LINE column (≥ 4 rows). A
                    // single-column page whose body happens to end in just a
                    // couple of lines can have a wide intra-line word gap (a
                    // sentence space after a period) split those lines into two
                    // x-disjoint blocks that otherwise pass this gate and emit as
                    // fake columns (alice_old "Looking-Glass House" p.226). A real
                    // two-column body / sidebar spans many rows. ~keep
                    && block_lines(a) >= 4
                    && block_lines(b) >= 4
                    && char_density(a) >= 12.0
                    && char_density(b) >= 12.0
                    // The two side-by-side blocks must be the page's DOMINANT
                    // content (≥ half the spans). A genuine 2-column body or
                    // sidebar+body lives in two big blocks; a table / chess
                    // diagram / dense diagram fragments into many small blocks
                    // that the union-find cannot coalesce, so the dominant pair
                    // never reaches half — leaving such pages on the row-aware path. ~keep
                    && (a.members.len() + b.members.len()) * 2 >= n
            })
        });
        if !side_by_side {
            return None;
        }

        // --- Topological order (two precede rules). A precedes B if they
        // overlap in X and A is above B (vertical stack), OR A is left of B and
        // they overlap in Y (side-by-side columns: left first). DFS with a visited
        // guard appends a block only after all its predecessors, and terminates on
        // any rule cycle. --- ~keep
        let nb = blocks.len();
        let before = |a: &Block, b: &Block| -> bool {
            let x_ov = (a.x1.min(b.x1) - a.x0.max(b.x0)) > med_h * 0.3;
            if x_ov && a.y_hi > b.y_hi && a.y_lo > b.y_lo {
                return true;
            }
            if a.x1 <= b.x0 && y_ov(a, b) {
                return true;
            }
            false
        };
        // Kahn's algorithm over the `before` relation. The previous
        // iterative DFS re-pushed every unvisited predecessor each time a
        // node was expanded (no on-stack marking), which is exponential in
        // stack growth on block graphs with heavy fan-in — a dense
        // equation page produced tens of gigabytes of stack and an OOM
        // kill. Kahn's is O(V^2) for the edge scan and O(V+E) after,
        // visits each block exactly once, and terminates unconditionally;
        // ready blocks are drained in reading order (top-left first) for
        // a stable result, matching the old seed order. ~keep
        let mut result_blocks: Vec<usize> = Vec::with_capacity(nb);
        let mut preds: Vec<Vec<usize>> = vec![Vec::new(); nb];
        let mut indegree: Vec<usize> = vec![0; nb];
        for a in 0..nb {
            for b in 0..nb {
                if a != b && before(&blocks[a], &blocks[b]) {
                    preds[a].push(b);
                    indegree[b] += 1;
                }
            }
        }
        let seed_order = |a: usize, b: usize| {
            safe_float_cmp(blocks[b].y_hi, blocks[a].y_hi).then_with(|| safe_float_cmp(blocks[a].x0, blocks[b].x0))
        };
        // Kept sorted in REVERSE reading order so pop() takes the
        // top-left-most ready block. ~keep
        let mut ready: Vec<usize> = (0..nb).filter(|&i| indegree[i] == 0).collect();
        ready.sort_by(|&a, &b| seed_order(b, a));
        let mut emitted = vec![false; nb];
        while let Some(bi) = ready.pop() {
            // `ready` is kept sorted with the NEXT block last (reverse
            // reading order), so pop() takes the top-left-most. ~keep
            if emitted[bi] {
                continue;
            }
            emitted[bi] = true;
            result_blocks.push(bi);
            let mut newly_ready = false;
            for &succ in &preds[bi] {
                indegree[succ] -= 1;
                if indegree[succ] == 0 {
                    ready.push(succ);
                    newly_ready = true;
                }
            }
            if newly_ready {
                ready.sort_by(|&a, &b| seed_order(b, a));
            }
        }
        // The `before` relation is acyclic by construction (edges strictly
        // decrease y within a band or strictly increase x across columns),
        // but guard against float pathologies leaving blocks unemitted:
        // append any remainder in reading order rather than dropping text. ~keep
        if result_blocks.len() < nb {
            let mut rest: Vec<usize> = (0..nb).filter(|&i| !emitted[i]).collect();
            rest.sort_by(|&a, &b| seed_order(a, b));
            result_blocks.extend(rest);
        }

        let mut out: Vec<crate::layout::TextSpan> = Vec::with_capacity(n);
        for &bi in &result_blocks {
            let mut members = blocks[bi].members.clone();
            members.sort_by(|&a, &b| {
                safe_float_cmp(hi(&spans[b]), hi(&spans[a]))
                    .then_with(|| safe_float_cmp(spans[a].bbox.left(), spans[b].bbox.left()))
            });
            for i in members {
                out.push(spans[i].clone());
            }
        }
        if out.len() == n { Some(out) } else { None }
    }

    /// True if the spans cluster into lines whose leftmost X positions
    /// form ≥ 2 distinct peaks separated by a clear gutter.
    ///
    /// Body-level word spans fill the X axis continuously, so the
    /// span-center histogram cannot tell two-column body text apart
    /// from a single-column page with varied line lengths. The line-
    /// start histogram does: in two-column body text most lines start
    /// at one of two X positions (left-column-start or right-column-
    /// start), and the wide gutter between the columns produces a
    /// long zero-count stretch.
    fn has_bimodal_line_starts(spans: &[crate::layout::TextSpan]) -> bool {
        const Y_BAND: f32 = 2.0;
        const BIN_PT: f32 = 5.0;
        const MIN_PEAK_COUNT: usize = 4;
        const MIN_GUTTER_PT: f32 = 30.0;

        if spans.len() < 24 {
            return false;
        }

        let mut lines: Vec<(f32, f32)> = Vec::new();
        let mut sorted = spans.to_vec();
        sorted.sort_by(|a, b| {
            crate::utils::safe_float_cmp(b.bbox.y, a.bbox.y)
                .then_with(|| crate::utils::safe_float_cmp(a.bbox.x, b.bbox.x))
        });

        let mut current_y: Option<f32> = None;
        let mut current_xmin: f32 = f32::INFINITY;
        for s in &sorted {
            match current_y {
                Some(y) if (y - s.bbox.y).abs() <= Y_BAND => {
                    current_xmin = current_xmin.min(s.bbox.x);
                }
                _ => {
                    if let Some(y) = current_y
                        && current_xmin.is_finite()
                    {
                        lines.push((y, current_xmin));
                    }
                    current_y = Some(s.bbox.y);
                    current_xmin = s.bbox.x;
                }
            }
        }
        if let Some(y) = current_y
            && current_xmin.is_finite()
        {
            lines.push((y, current_xmin));
        }
        if lines.len() < 16 {
            return false;
        }

        let xmin = lines.iter().map(|(_, x)| *x).fold(f32::INFINITY, f32::min);
        let xmax = lines.iter().map(|(_, x)| *x).fold(f32::NEG_INFINITY, f32::max);
        if !(xmin.is_finite() && xmax.is_finite()) || xmax - xmin < MIN_GUTTER_PT {
            return false;
        }
        let bin_count = (((xmax - xmin) / BIN_PT).ceil() as usize).max(1);
        if bin_count > 4096 {
            return false;
        }
        let mut hist = vec![0usize; bin_count];
        for (_, x) in &lines {
            let idx = (((x - xmin) / BIN_PT) as usize).min(bin_count - 1);
            hist[idx] += 1;
        }

        let mut peaks: Vec<usize> = Vec::new();
        let mut in_peak = false;
        let mut peak_start = 0usize;
        for (i, &c) in hist.iter().enumerate() {
            if c >= MIN_PEAK_COUNT {
                if !in_peak {
                    peak_start = i;
                    in_peak = true;
                }
            } else if c == 0 && in_peak {
                peaks.push((peak_start + i.saturating_sub(1)) / 2);
                in_peak = false;
            }
        }
        if in_peak {
            peaks.push((peak_start + hist.len() - 1) / 2);
        }
        if peaks.len() < 2 {
            return false;
        }

        let gutter_bins = (MIN_GUTTER_PT / BIN_PT) as usize;
        for w in peaks.windows(2) {
            let a = w[0];
            let b = w[1];
            if b <= a {
                continue;
            }
            let zeros = hist[a + 1..b].iter().filter(|&&c| c == 0).count();
            if zeros >= gutter_bins {
                return true;
            }
        }
        false
    }

    /// Numeric value 0–9 of a folio (page-number) digit, or `None` if `c` is
    /// not one. Scoped to the decimal-digit blocks that actually appear as
    /// page folios: ASCII, Arabic-Indic, Extended Arabic-Indic (Persian/Urdu),
    /// Devanagari, and full-width. Deliberately narrower than
    /// `char::is_numeric()` (which also matches `½`, `①`, superscripts) and
    /// wider than `char::is_ascii_digit()`. CJK ideographic numerals
    /// (`一二三…`) are intentionally excluded — they are not Unicode `Nd`, and
    /// collapsing them would over-normalize real headings (`第一章` → `第#章`).
    ///
    /// `char::to_digit(10)` cannot stand in here: it is ASCII-only and returns
    /// `None` for `'٥'` / `'५'` / `'５'`, so each block is mapped to its zero
    /// code point directly.
    fn folio_digit_value(c: char) -> Option<u32> {
        let cp = c as u32;
        let base = match cp {
            0x0030..=0x0039 => 0x0030,
            0x0660..=0x0669 => 0x0660,
            0x06F0..=0x06F9 => 0x06F0,
            0x0966..=0x096F => 0x0966,
            0xFF10..=0xFF19 => 0xFF10,
            _ => return None,
        };
        Some(cp - base)
    }

    /// Unicode-aware decimal-digit predicate for page folios. See
    /// [`Self::folio_digit_value`] for the supported blocks and rationale.
    fn is_folio_digit(c: char) -> bool {
        Self::folio_digit_value(c).is_some()
    }

    /// Normalize a span's text for cross-page signature matching.
    /// Collapses whitespace and replaces digit runs with `#` so that page
    /// numbers ("Page 1 of 10", "Page 2 of 10") collapse to one signature.
    /// Non-Latin folio digits (Arabic-Indic, Persian, Devanagari, full-width)
    /// collapse too, so folios paginated in those scripts share one signature.
    pub(super) fn normalize_artifact_signature(text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        let mut in_digit_run = false;
        let mut last_was_space = true;
        for c in text.chars() {
            if Self::is_folio_digit(c) {
                if !in_digit_run {
                    out.push('#');
                    in_digit_run = true;
                }
                last_was_space = false;
            } else if c.is_whitespace() {
                if !last_was_space {
                    out.push(' ');
                    last_was_space = true;
                }
                in_digit_run = false;
            } else {
                out.push(c);
                last_was_space = false;
                in_digit_run = false;
            }
        }
        out.trim().to_string()
    }

    /// Item 6B (M5): does a running-band literal look like a CONSTANT-text
    /// pagination / citation string — a DOI, a journal volume/issue/article
    /// reference, or a journal URL host — accompanied by a digit? Such strings
    /// recur identically on every page (so the varying-literal gate never catches
    /// them) yet are furniture that leaks into the body. The gate is deliberately
    /// narrow: it requires a recognised citation/URL token AND a digit, so a
    /// repeated facility name, document title, or ordinary sentence is NEVER
    /// matched (miss-rather-than-drop — a false positive deletes real content).
    pub(super) fn looks_like_stable_pagination(literal: &str) -> bool {
        let l = literal.to_ascii_lowercase();
        // The digit gate is script-aware (a non-Latin folio digit still
        // qualifies); the citation/URL keyword tokens below remain English-
        // only by design — keyword universality is tracked separately. ~keep
        if !l.chars().any(Self::is_folio_digit) {
            return false;
        }
        if l.contains("doi.org") || l.contains("doi:") || l.contains("/doi/") {
            return true;
        }
        // Journal volume/issue/article reference. NB: "no." is deliberately
        // EXCLUDED — it also matches government-form control numbers like
        // "OMB No. 1545-0115", which are form content, not running furniture. ~keep
        if ["volume", "vol.", "article", "issue"].iter().any(|kw| l.contains(kw)) {
            return true;
        }
        l.contains("www.") && (l.contains(".org") || l.contains(".com") || l.contains(".net"))
    }

    /// A page is treated as vertical-writing (CJK tategaki, 縦書き) when a
    /// majority of its non-empty text spans were rendered in WMode 1. The
    /// writing mode comes from the PDF's own `/WMode` (captured on each span
    /// via `GraphicsState::text_wmode`), so this is authoritative — a
    /// horizontal page (WMode 0) is never misclassified, and its
    /// running-header/footer detection is unchanged.
    pub(super) fn page_is_vertical(spans: &[crate::layout::TextSpan]) -> bool {
        let mut vertical = 0usize;
        let mut total = 0usize;
        for s in spans {
            if s.text.trim().is_empty() {
                continue;
            }
            total += 1;
            if s.wmode == 1 {
                vertical += 1;
            }
        }
        total > 0 && vertical * 2 > total
    }

    /// Is `bbox` inside the candidate running-header/footer band for a page of
    /// the given dimensions? Horizontal pages use the top/bottom 12% strips.
    /// Vertical-writing (tategaki) pages *additionally* use the left/right 12%
    /// strips — the outer edge where CJK vertical folios and running heads
    /// conventionally sit, rather than across the top/bottom edge. The side
    /// strips are additive (the top/bottom test still applies), so this only
    /// ever widens detection, never narrows it.
    pub(super) fn in_chrome_band(
        bbox: &crate::geometry::Rect,
        page_width: f32,
        page_height: f32,
        vertical: bool,
    ) -> bool {
        let vband = page_height * 0.12;
        if bbox.y < vband || bbox.y + bbox.height > page_height - vband {
            return true;
        }
        if vertical {
            let hband = page_width * 0.12;
            if bbox.x < hband || bbox.x + bbox.width > page_width - hband {
                return true;
            }
        }
        false
    }

    /// Ensure running-artifact signatures are computed (once) and return a
    /// clone for matching. The computation scans every page's raw spans,
    /// collects normalized text that appears in the top/bottom 12% band (and,
    /// on vertical-writing pages, the left/right 12% band), and keeps entries
    /// that recur on >=50% of pages.
    /// Article threads for this document, parsed once and shared.
    /// [`crate::structure::parse_article_threads`] walks the entire page tree,
    /// and reading-order resolution asks for them on every page.
    pub(crate) fn cached_article_threads(&self) -> std::sync::Arc<Vec<crate::structure::ArticleThread>> {
        if let Some(cached) = self.article_threads_cache.lock_or_recover().as_ref() {
            return std::sync::Arc::clone(cached);
        }
        let threads = std::sync::Arc::new(crate::structure::parse_article_threads(self));
        *self.article_threads_cache.lock_or_recover() = Some(std::sync::Arc::clone(&threads));
        threads
    }

    fn ensure_running_artifact_signatures(&self) -> Result<std::sync::Arc<std::collections::HashMap<String, usize>>> {
        {
            let guard = self.running_artifact_signatures.lock_or_recover();
            if let Some(ref map) = *guard {
                // Shared by reference: this runs once per page and the map is
                // document-wide, so cloning it here scaled with page count. ~keep
                return Ok(std::sync::Arc::clone(map));
            }
        }
        let page_count = self.page_count()?;
        if page_count < 2 {
            let empty = std::sync::Arc::new(std::collections::HashMap::new());
            *self.running_artifact_signatures.lock_or_recover() = Some(std::sync::Arc::clone(&empty));
            return Ok(empty);
        }

        // (count of distinct pages seeing the signature, first page it appeared on).
        // `first_seen_any` tracks the earliest page a signature appeared on
        // regardless of body-content — so if the cover page is all-chrome
        // (no body text), it still registers as "first seen" and gets its
        // title kept by the per-page mark_running_artifact_spans exemption. ~keep
        let mut occurrences: std::collections::HashMap<String, (usize, usize)> = std::collections::HashMap::new();
        let mut first_seen_any: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        // Track distinct literal texts per signature. A signature whose digits
        // are stable across every page (i.e. the literal text never changes) is
        // NOT a page-number-containing header — it is substantive content that
        // happens to repeat. Only suppress signatures where the literal text
        // varies (at least two distinct forms) meaning digits change per page. ~keep
        let mut literal_variants: std::collections::HashMap<String, std::collections::HashSet<String>> =
            std::collections::HashMap::new();
        for pi in 0..page_count {
            let spans = match self.extract_spans_raw(pi) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let (page_width, page_height) = match self.get_page_media_box(pi) {
                Ok((_, _, w, h)) if h > 0.0 => (w, h),
                _ => continue,
            };
            let vertical = Self::page_is_vertical(&spans);
            // Require that the page has CONTENT outside the chrome band(s)
            // before counting band spans as candidate artifacts. Otherwise, a
            // page consisting only of a title near the top would have its own
            // title classified as a "running header" across all pages. (For
            // horizontal pages this is identical to the prior top/bottom test.) ~keep
            let has_body_content = spans.iter().any(|s| {
                !s.text.trim().is_empty() && !Self::in_chrome_band(&s.bbox, page_width, page_height, vertical)
            });
            // Collect per-page unique signatures from the chrome bands.
            // Runs even when there's no body content so `first_seen_any`
            // registers the cover page even if it's all-chrome. ~keep
            let mut seen_this_page: std::collections::HashMap<String, String> = std::collections::HashMap::new();
            for s in spans.iter() {
                let trimmed = s.text.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if !Self::in_chrome_band(&s.bbox, page_width, page_height, vertical) {
                    continue;
                }
                let sig = Self::normalize_artifact_signature(trimmed);
                if sig.is_empty() || sig.chars().count() < 2 {
                    continue;
                }
                seen_this_page.entry(sig).or_insert_with(|| trimmed.to_string());
            }
            // Track first-seen across ALL pages (even body-content-skipped) ~keep
            for sig in seen_this_page.keys() {
                first_seen_any.entry(sig.clone()).or_insert(pi);
            }
            // Track literal variants — if the literal text for a signature
            // differs across pages, the digits are varying (page numbers). ~keep
            for (sig, literal) in &seen_this_page {
                literal_variants.entry(sig.clone()).or_default().insert(literal.clone());
            }
            if !has_body_content {
                continue;
            }
            // Count only pages with body content for the recurrence threshold ~keep
            for sig in seen_this_page.into_keys() {
                let entry = occurrences.entry(sig).or_insert((0, pi));
                entry.0 += 1;
                if pi < entry.1 {
                    entry.1 = pi;
                }
            }
        }
        let threshold = (page_count as f32 * 0.5).ceil() as usize;
        let signatures: std::collections::HashMap<String, usize> = occurrences
            .into_iter()
            .filter(|(sig, (count, _))| {
                let variants = literal_variants.get(sig).map(|s| s.len()).unwrap_or(0);
                // Varying-literal path (page numbers / dates): the digits change per
                // page. Recurs on >=50% of body pages. ~keep
                if *count >= threshold.max(2) && variants >= 2 {
                    return true;
                }
                // Item 6B (M5): CONSTANT-literal pagination/citation (DOI, volume/
                // article, journal URL + digit). The literal never changes, so the
                // varying-literal gate above misses it. Require a STRICTER >=60%
                // recurrence AND the narrow citation/URL shape gate, so substantive
                // repeated content (facility names, titles) is never suppressed. ~keep
                let strict = (page_count as f32 * 0.6).ceil() as usize;
                *count >= strict.max(2)
                    && variants < 2
                    && literal_variants
                        .get(sig)
                        .and_then(|s| s.iter().next())
                        .is_some_and(|lit| Self::looks_like_stable_pagination(lit))
            })
            .map(|(sig, _)| {
                // Use the earliest page the signature appeared on — which
                // may be a body-content-skipped cover page that `occurrences`
                // didn't count toward the threshold but `first_seen_any` did. ~keep
                let first = first_seen_any.get(&sig).copied().unwrap_or(0);
                (sig, first)
            })
            .collect();
        let signatures = std::sync::Arc::new(signatures);
        *self.running_artifact_signatures.lock_or_recover() = Some(std::sync::Arc::clone(&signatures));
        Ok(signatures)
    }

    /// Mark spans near the top/bottom of the page whose normalized text
    /// matches a cached running-artifact signature by setting
    /// `artifact_type` to Pagination.
    /// A bare page number (e.g. " 1 ", "12") varies per page, so it
    /// never matches a repeated-text signature and leaks into the body. Treat
    /// a short pure-digit token (1..=9999) as a page-number candidate — only
    /// applied inside the top/bottom margin band by the caller, so ordinary
    /// numerals in body text are never affected.
    pub(super) fn is_bare_page_number_text(trimmed: &str) -> bool {
        // Bound by character count, not byte length: non-Latin folio digits are
        // 2–3 UTF-8 bytes each, so a byte cap would reject "۱۲۳" outright. ~keep
        if trimmed.is_empty() || trimmed.chars().count() > 4 {
            return false;
        }
        // Fold the (script-aware) digits to a value directly; `parse::<u32>`
        // and `char::to_digit` are ASCII-only and reject non-Latin folios. ~keep
        let mut value: u32 = 0;
        for c in trimmed.chars() {
            match Self::folio_digit_value(c) {
                Some(d) => value = value * 10 + d,
                None => return false,
            }
        }
        (1..=9999).contains(&value)
    }

    pub(super) fn mark_running_artifact_spans(
        &self,
        page_index: usize,
        spans: &mut [crate::layout::TextSpan],
    ) -> Result<()> {
        let (_, _, page_width, page_height) = match self.get_page_media_box(page_index) {
            Ok(mb) => mb,
            Err(_) => return Ok(()),
        };
        if page_height <= 0.0 {
            return Ok(());
        }
        let vertical = Self::page_is_vertical(spans);
        // Snapshot baselines of every non-blank span, so the bare-page-number
        // rule can require a candidate to stand ALONE on its line: a
        // digit adjacent to other text — e.g. the "8" in "8th" — is content,
        // not a page number. ~keep
        let occupied_baselines: Vec<f32> = spans
            .iter()
            .filter(|s| !s.text.trim().is_empty())
            .map(|s| s.bbox.y)
            .collect();
        // Signature set may be empty (no repeated headers/footers); the
        // bare-page-number rule below still runs. ~keep
        let signatures = self.ensure_running_artifact_signatures()?;
        for s in spans.iter_mut() {
            if s.artifact_type.is_some() {
                continue;
            }
            if !Self::in_chrome_band(&s.bbox, page_width, page_height, vertical) {
                continue;
            }
            let trimmed = s.text.trim();
            if trimmed.is_empty() {
                continue;
            }
            // Standalone page-number chrome in the margin band — only
            // when the digit is ISOLATED on its line (no other text span
            // within ~one line height), so digits embedded in words/runs are
            // never dropped. ~keep ~keep
            if Self::is_bare_page_number_text(trimmed) {
                let line_tol = s.font_size.max(6.0);
                let on_line = occupied_baselines
                    .iter()
                    .filter(|&&oy| (oy - s.bbox.y).abs() < line_tol)
                    .count();
                if on_line <= 1 {
                    s.artifact_type = Some(crate::extractors::text::ArtifactType::Pagination(
                        crate::extractors::text::PaginationSubtype::PageNumber,
                    ));
                }
                continue;
            }
            if signatures.is_empty() {
                continue;
            }
            let sig = Self::normalize_artifact_signature(trimmed);
            if let Some(&first_seen_on) = signatures.get(&sig) {
                // Keep the first appearance — it's usually the document
                // cover-page title that got classified as chrome only
                // because later pages repeat it as a running header (B3). ~keep ~keep
                if page_index == first_seen_on {
                    continue;
                }
                s.artifact_type = Some(crate::extractors::text::ArtifactType::Pagination(
                    crate::extractors::text::PaginationSubtype::Other,
                ));
            }
        }
        Ok(())
    }
}
