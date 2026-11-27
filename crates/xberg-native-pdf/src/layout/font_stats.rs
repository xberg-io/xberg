//! Per-page font statistics derived from extracted text spans.
//!
//! Every layout heuristic in the multi-column / reading-order pipeline
//! used to depend on hardcoded absolute thresholds (a 100 pt minimum
//! column width, a 6 pt column-boundary gutter, etc.). That breaks on
//! documents whose body font isn't 12 pt — a 6 pt fax sets one set of
//! gutter sizes, a 24 pt poster sets another, and a hardcoded threshold
//! is necessarily wrong on at least one of them.
//!
//! [`PageFontStats`] holds the four measurements every downstream
//! threshold should be derived from. They are computed once per page in
//! a single pass over the spans and passed by reference into the
//! layout heuristics that need them.
//!
//! ```ignore
//! use xberg_native_pdf::layout::{PageFontStats, TextSpan};
//! let stats = PageFontStats::from_spans(&spans);
//! let body_em = stats.dominant_em;          // ≈ font size of the body text
//! let line_h  = stats.dominant_line_height;  // baseline-to-baseline distance
//! ```

use super::TextSpan;

/// Per-page font statistics. Computed in O(n) over the spans.
#[derive(Debug, Clone, PartialEq)]
pub struct PageFontStats {
    /// Mode of `font_size` weighted by character count. The "1 em"
    /// for the page's body text. Defaults to 12.0 on an empty page.
    pub dominant_em: f32,

    /// Median baseline-to-baseline distance for adjacent same-column
    /// same-font spans. Approximates the page's set leading. Defaults
    /// to `1.2 × dominant_em` when fewer than two same-font spans are
    /// available to derive it.
    pub dominant_line_height: f32,

    /// Average advance width per character in the dominant font. Used
    /// to convert "N characters wide" thresholds into points. Defaults
    /// to `0.5 × dominant_em` (Helvetica's per-em mean width).
    pub dominant_char_width: f32,

    /// Identifier of the font used by the largest share of characters
    /// on the page. Used by region-distinctness heuristics (caption
    /// vs body) to detect font-discontinuity boundaries.
    pub body_font_name: String,
}

impl Default for PageFontStats {
    fn default() -> Self {
        Self {
            dominant_em: 12.0,
            dominant_line_height: 14.4,
            dominant_char_width: 6.0,
            body_font_name: String::new(),
        }
    }
}

impl PageFontStats {
    /// Compute font statistics from a slice of spans.
    ///
    /// Empty input → returns [`PageFontStats::default`].
    pub fn from_spans(spans: &[TextSpan]) -> Self {
        if spans.is_empty() {
            return Self::default();
        }

        // Mode of font_size weighted by character count.
        // Bin font sizes into 0.25 pt buckets so 11.97 / 12.00 / 12.03
        // (which all came from the same nominal 12 pt body) collapse to
        // one bucket. ~keep
        let mut size_buckets: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
        let mut font_buckets: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for s in spans {
            let chars = s.text.chars().count();
            if chars == 0 || !s.font_size.is_finite() || s.font_size <= 0.0 {
                continue;
            }
            // Quantize to 0.25 pt buckets. Use u32 to avoid silent saturation
            // for large font sizes (e.g. poster / display fonts at 200+ pt). ~keep
            let bucket = (s.font_size * 4.0).round() as u32;
            *size_buckets.entry(bucket).or_insert(0) += chars;
            *font_buckets.entry(s.font_name.as_str()).or_insert(0) += chars;
        }
        if size_buckets.is_empty() {
            return Self::default();
        }

        // Both modes are picked with an explicit tie-break, NOT `max_by_key`.
        // `max_by_key` returns the LAST maximal element, and over a `HashMap`
        // "last" is whichever bucket the per-process-randomized iteration order
        // happens to visit last. When the top two buckets carry the SAME
        // character count — two font sizes each covering half the page, a page
        // set in two fonts of equal mass — the winner was a coin flip per
        // process, so `dominant_em` and `body_font_name` differed between runs
        // of the same binary on the same page. `dominant_em` then feeds the
        // column-shape gate in `is_multi_column_page` (`cluster_gap`), which
        // selects the reading-order branch in `assemble_text_from_spans`, so
        // the flip reordered the whole page's extracted text.
        //
        // Ties resolve to the SMALLER size bucket and the lexicographically
        // smaller font name. Smaller-wins is the conservative direction for the
        // size: `dominant_em` is meant to be the BODY em, headings are the
        // larger size, and a tighter `cluster_gap` makes the multi-column gate
        // stricter rather than looser. Same idiom as the mode pick in
        // `crate::pipeline::converters` and the cmap pick in
        // `crate::extractors::text`.
        // ~keep
        let dominant_em = {
            let (bucket, _) = size_buckets
                .iter()
                .max_by(|(a_bucket, a_count), (b_bucket, b_count)| {
                    a_count.cmp(b_count).then_with(|| b_bucket.cmp(a_bucket))
                })
                .expect("size_buckets non-empty checked above");
            (*bucket as f32) / 4.0
        };

        let body_font_name = font_buckets
            .iter()
            .max_by(|(a_name, a_count), (b_name, b_count)| a_count.cmp(b_count).then_with(|| b_name.cmp(a_name)))
            .map(|(name, _)| (*name).to_string())
            .unwrap_or_default();

        let dominant_line_height =
            compute_line_height(spans, &body_font_name, dominant_em).unwrap_or(dominant_em * 1.2);

        let dominant_char_width = compute_char_width(spans, &body_font_name, dominant_em);

        Self {
            dominant_em,
            dominant_line_height,
            dominant_char_width,
            body_font_name,
        }
    }
}

/// Mean rendered width of one character across the body-font spans, falling
/// back to half an em when no span qualifies.
fn compute_char_width(spans: &[TextSpan], body_font: &str, dominant_em: f32) -> f32 {
    let mut total_width = 0.0_f64;
    let mut total_chars = 0_usize;
    let dominant_size_min = dominant_em - 0.25;
    let dominant_size_max = dominant_em + 0.25;
    for s in spans {
        if s.font_name == body_font
            && s.font_size >= dominant_size_min
            && s.font_size <= dominant_size_max
            && s.bbox.width > 0.0
        {
            let chars = s.text.chars().count();
            if chars > 0 {
                total_width += s.bbox.width as f64;
                total_chars += chars;
            }
        }
    }
    if total_chars > 0 {
        (total_width / total_chars as f64) as f32
    } else {
        dominant_em * 0.5
    }
}

fn compute_line_height(spans: &[TextSpan], body_font: &str, dominant_em: f32) -> Option<f32> {
    let mut ys: Vec<f32> = spans
        .iter()
        .filter(|s| s.font_name == body_font && (s.font_size - dominant_em).abs() < 0.5 && s.bbox.y.is_finite())
        .map(|s| s.bbox.y)
        .collect();

    if ys.len() < 4 {
        return None;
    }

    // Sort descending (highest y = top of page first in PDF coords). ~keep
    ys.sort_by(|a, b| crate::utils::safe_float_cmp(*b, *a));

    // Deduplicate baselines within 0.5 pt: multiple spans on the same
    // text line (bold run, mixed font) would otherwise inject zero-gaps. ~keep
    ys.dedup_by(|later, earlier| (*earlier - *later).abs() < 0.5);

    // Measure consecutive baseline gaps. Accept only values in the
    // plausible set-leading range [0.5em, 3em]: narrower gaps are
    // subscript/inline-math overlaps; wider are paragraph or section
    // breaks. This filter makes the column-grouping heuristic
    // unnecessary — cross-column gaps are almost always > 3em.
    // ~keep
    let mut gaps: Vec<f32> = ys
        .windows(2)
        .map(|w| w[0] - w[1])
        .filter(|&g| g >= dominant_em * 0.5 && g <= dominant_em * 3.0)
        .collect();

    if gaps.is_empty() {
        return None;
    }
    gaps.sort_by(|a, b| crate::utils::safe_float_cmp(*a, *b));
    Some(gaps[gaps.len() / 2])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Rect;

    fn span(text: &str, x: f32, y: f32, font: &str, size: f32) -> TextSpan {
        TextSpan {
            text: text.to_string(),
            bbox: Rect::new(x, y, text.chars().count() as f32 * size * 0.5, size),
            font_name: font.into(),
            font_size: size,
            ..Default::default()
        }
    }

    #[test]
    fn empty_spans_returns_default() {
        let stats = PageFontStats::from_spans(&[]);
        assert_eq!(stats.dominant_em, 12.0);
        assert_eq!(stats.dominant_line_height, 14.4);
        assert_eq!(stats.dominant_char_width, 6.0);
        assert!(stats.body_font_name.is_empty());
    }

    #[test]
    fn single_uniform_block_picks_size_and_font() {
        let mut spans = Vec::new();
        let mut y = 720.0;
        for i in 0..20 {
            spans.push(span(&format!("body line {i:02}"), 72.0, y, "Helvetica", 12.0));
            y -= 14.4;
        }
        let stats = PageFontStats::from_spans(&spans);
        assert_eq!(stats.dominant_em, 12.0);
        assert_eq!(stats.body_font_name, "Helvetica");
        assert!(
            (stats.dominant_line_height - 14.4).abs() < 0.01,
            "expected ~14.4, got {}",
            stats.dominant_line_height
        );
        assert!(
            (stats.dominant_char_width - 6.0).abs() < 0.01,
            "expected ~6.0, got {}",
            stats.dominant_char_width
        );
    }

    // A tie in the size histogram must resolve to the SMALLER bucket, every
    // run. Both sizes carry exactly 50 characters, so `max_by_key` over the
    // backing `HashMap` would return whichever bucket the per-process-
    // randomized iteration order visited last — this test failed roughly half
    // the time before the explicit tie-break.
    // ~keep
    #[test]
    fn size_histogram_tie_resolves_to_smaller_bucket() {
        let mut spans = Vec::new();
        let mut y = 720.0;
        for _ in 0..10 {
            spans.push(span("abcde", 72.0, y, "Helvetica", 10.0));
            y -= 12.0;
        }
        for _ in 0..10 {
            spans.push(span("abcde", 300.0, y, "Helvetica", 14.0));
            y -= 16.0;
        }
        let stats = PageFontStats::from_spans(&spans);
        assert_eq!(
            stats.dominant_em, 10.0,
            "50 chars at 10 pt vs 50 chars at 14 pt must resolve to the smaller em"
        );
    }

    // Same defect on the font histogram: equal character mass in two fonts
    // must resolve to the lexicographically smaller name. ~keep
    #[test]
    fn font_histogram_tie_resolves_to_lexicographically_smaller_name() {
        let mut spans = Vec::new();
        let mut y = 720.0;
        for _ in 0..10 {
            spans.push(span("abcde", 72.0, y, "Helvetica", 12.0));
            spans.push(span("vwxyz", 300.0, y, "Arial", 12.0));
            y -= 14.4;
        }
        let stats = PageFontStats::from_spans(&spans);
        assert_eq!(
            stats.body_font_name, "Arial",
            "equal character mass in two fonts must resolve to the smaller name"
        );
    }

    // The tie-break must not disturb the ordinary case: a clear winner is
    // still the clear winner, whichever side of the tie-break it sits on. ~keep
    #[test]
    fn unique_mode_is_unaffected_by_tie_break_direction() {
        let mut spans = Vec::new();
        let mut y = 720.0;
        // Larger size holds strictly more character mass, so it must win even
        // though ties resolve toward the smaller bucket. ~keep
        for _ in 0..4 {
            spans.push(span("ab", 72.0, y, "Helvetica", 9.0));
            y -= 11.0;
        }
        for _ in 0..10 {
            spans.push(span("abcdefghij", 300.0, y, "Helvetica", 18.0));
            y -= 21.0;
        }
        let stats = PageFontStats::from_spans(&spans);
        assert_eq!(stats.dominant_em, 18.0);
    }

    #[test]
    fn mode_not_mean_when_outliers_present() {
        // 30 chars of 12 pt body + 1 char of 72 pt heading.
        // Mean would be skewed up; mode-by-char-count stays at 12. ~keep
        let mut spans: Vec<TextSpan> = (0..15)
            .map(|i| span("two body line", 72.0, 720.0 - i as f32 * 14.4, "Helvetica", 12.0))
            .collect();
        spans.push(span("X", 72.0, 720.0, "Helvetica", 72.0));
        let stats = PageFontStats::from_spans(&spans);
        assert_eq!(stats.dominant_em, 12.0);
    }

    #[test]
    fn body_font_is_majority_by_char_count() {
        let mut spans: Vec<TextSpan> = (0..15)
            .map(|i| span("body line text", 72.0, 720.0 - i as f32 * 14.4, "Helvetica", 12.0))
            .collect();
        spans.push(span("Fig 1", 200.0, 400.0, "Times", 9.0));
        spans.push(span("Fig 2", 200.0, 300.0, "Times", 9.0));
        let stats = PageFontStats::from_spans(&spans);
        assert_eq!(stats.body_font_name, "Helvetica");
    }

    #[test]
    fn quantized_size_bucket_collapses_near_duplicates() {
        let spans = vec![
            span("aaaaa", 72.0, 720.0, "Helvetica", 11.97),
            span("bbbbb", 72.0, 706.0, "Helvetica", 12.00),
            span("ccccc", 72.0, 692.0, "Helvetica", 12.03),
            span("ddddd", 72.0, 678.0, "Helvetica", 12.00),
        ];
        let stats = PageFontStats::from_spans(&spans);
        assert!(
            (stats.dominant_em - 12.0).abs() < 0.05,
            "expected 12.0, got {}",
            stats.dominant_em
        );
    }

    #[test]
    fn line_height_falls_back_to_1_2_em_when_unmeasurable() {
        let spans = vec![span("solo", 72.0, 720.0, "Helvetica", 12.0)];
        let stats = PageFontStats::from_spans(&spans);
        assert!(
            (stats.dominant_line_height - 14.4).abs() < 0.01,
            "fallback expected, got {}",
            stats.dominant_line_height
        );
    }

    #[test]
    fn char_width_uses_dominant_font_only() {
        // Body 12 pt Helvetica with char-width 6.0; an outlier Times
        // span at 9 pt should NOT pull the dominant_char_width away. ~keep
        let mut spans: Vec<TextSpan> = (0..15)
            .map(|i| span("body line", 72.0, 720.0 - i as f32 * 14.4, "Helvetica", 12.0))
            .collect();
        spans.push(span("CAPTION CAPTION CAPTION", 200.0, 400.0, "Times", 9.0));
        let stats = PageFontStats::from_spans(&spans);
        assert!(
            (stats.dominant_char_width - 6.0).abs() < 0.01,
            "expected ~6.0, got {}",
            stats.dominant_char_width
        );
    }

    #[test]
    fn wide_column_line_height_measured_correctly() {
        // Spans across a 500pt-wide column. The old x-center sweep with
        // bucket_w = 72pt would split this into ~7 buckets, each with
        // 2-3 spans, producing unreliable gap measurements. The new
        // y-only approach handles it correctly regardless of span width.
        // ~keep
        let mut spans = Vec::new();
        let mut y = 720.0f32;
        for i in 0..20 {
            let w = 10.0 + (i as f32 * 20.0);
            spans.push(TextSpan {
                text: format!("line {i:02}"),
                bbox: crate::geometry::Rect::new(72.0, y, w, 12.0),
                font_name: "Helvetica".into(),
                font_size: 12.0,
                ..Default::default()
            });
            y -= 14.4;
        }
        let stats = PageFontStats::from_spans(&spans);
        assert!(
            (stats.dominant_line_height - 14.4).abs() < 0.5,
            "wide-column line height must be ~14.4, got {}",
            stats.dominant_line_height
        );
    }

    #[test]
    fn two_column_line_height_measured_correctly() {
        // Two columns at x=72 and x=320, same 14.4 pt leading.
        // Cross-column gaps would be paragraph-break sized (> 3em) and
        // must NOT pollute the line-height measurement. ~keep
        let mut spans = Vec::new();
        for col_x in [72.0_f32, 320.0] {
            let mut y = 720.0;
            for i in 0..10 {
                spans.push(TextSpan {
                    text: format!("col{col_x:.0} line {i:02}"),
                    bbox: crate::geometry::Rect::new(col_x, y, 100.0, 12.0),
                    font_name: "Helvetica".into(),
                    font_size: 12.0,
                    ..Default::default()
                });
                y -= 14.4;
            }
        }
        let stats = PageFontStats::from_spans(&spans);
        assert!(
            (stats.dominant_line_height - 14.4).abs() < 0.5,
            "two-column line height must be ~14.4, got {}",
            stats.dominant_line_height
        );
    }

    #[test]
    fn ignores_non_finite_or_zero_size() {
        let spans = vec![
            span("ok", 72.0, 720.0, "Helvetica", 12.0),
            span("zero", 72.0, 700.0, "Helvetica", 0.0),
            TextSpan {
                text: "nan".into(),
                bbox: Rect::new(72.0, 680.0, 10.0, 12.0),
                font_name: "Helvetica".into(),
                font_size: f32::NAN,
                ..Default::default()
            },
        ];
        let stats = PageFontStats::from_spans(&spans);
        assert_eq!(stats.dominant_em, 12.0);
    }
}
