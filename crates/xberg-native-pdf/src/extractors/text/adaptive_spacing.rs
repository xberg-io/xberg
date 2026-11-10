//! Adaptive TJ-threshold estimation from the page's gap distribution.
//!
//! Split out of the parent's single 5,806-line `impl TextExtractor`, which made
//! `extractors/text.rs` 673 KiB — over the repository's 500 KiB file-safety limit.
//! A child module's `impl` is the same inherent impl and sees the parent's private
//! items unchanged. ~keep

use super::*;

pub(super) const TJ_OFFSET_HISTORY_LIMIT: usize = 10_000;

impl<'doc> TextExtractor<'doc> {
    /// Calculate adaptive TJ offset threshold based on font size and text justification.
    ///
    /// When `use_adaptive_tj_threshold` is enabled, this method calculates the TJ offset
    /// threshold dynamically using the formula:
    ///
    /// ```text
    /// adaptive_threshold = -(space_width * font_size * margin_ratio) / 1000
    /// ```
    ///
    /// Where `margin_ratio` is adjusted based on justified vs normal text detection:
    /// - **Justified text** (high CV > 0.5): Uses 3× the normal ratio (conservative)
    ///   to prevent false space insertions from arbitrary TJ offsets
    /// - **Normal text** (low CV ≤ 0.5): Uses the default ratio (aggressive)
    ///
    /// # Adaptive Threshold Enhancement
    ///
    /// Per ISO 32000-1:2008 Section 9.4.4, justified text uses arbitrary TJ offsets to
    /// distribute whitespace. This method detects justified text through statistical
    /// analysis (coefficient of variation) and adapts the threshold accordingly.
    ///
    /// # Fallback Behavior
    ///
    /// If adaptive thresholds are disabled, this method returns the static
    /// `space_insertion_threshold` from the configuration.
    ///
    /// # PDF Spec Compliance
    ///
    /// Per Section 9.10: "Determining word boundaries is not specified by PDF."
    /// This method uses only spec-defined TJ values and geometric positions.
    pub(super) fn calculate_adaptive_tj_threshold(&self) -> f32 {
        if !self.config.use_adaptive_tj_threshold {
            return self.config.space_insertion_threshold;
        }

        self.adaptive_tj_threshold(self.analyze_tj_distribution())
    }

    pub(super) fn calculate_tj_threshold_after_offset(&self, offset: f32) -> f32 {
        if !self.config.use_adaptive_tj_threshold || self.tj_offset_history.len() >= TJ_OFFSET_HISTORY_LIMIT {
            return self.calculate_adaptive_tj_threshold();
        }
        let offset = f64::from(offset);
        let distribution = Self::tj_distribution_from_stats(
            self.tj_offset_history.len() + 1,
            self.tj_sum + offset,
            self.tj_sum_sq + offset * offset,
        );
        self.adaptive_tj_threshold(distribution)
    }

    fn adaptive_tj_threshold(&self, (is_justified, cv): (bool, f32)) -> f32 {
        let state = self.state_stack.current();

        let font_size = state.font_size;

        let space_width_units = state
            .font_name
            .as_ref()
            .and_then(|name| self.fonts.get(name))
            .map(|font| font.get_space_glyph_width())
            .unwrap_or(250.0); // Fallback: Times-Roman typical space width ~keep

        let margin_ratio = if is_justified {
            self.config.word_margin_ratio * 3.0
        } else {
            self.config.word_margin_ratio
        };

        // Calculate threshold: negative offset required to trigger space insertion
        // Normalized by 1000 (PDF spec font units are 1/1000em) ~keep
        let adaptive_threshold = -((space_width_units * font_size * margin_ratio) / 1000.0);

        tracing::trace!(target: LOG_TARGET,
            "TJ threshold: {} (justified={}, cv={:.2}, margin_ratio={:.3}, ISO 32000-1 §9.4.4)",
            adaptive_threshold,
            is_justified,
            cv,
            margin_ratio
        );

        adaptive_threshold
    }

    /// Analyze TJ offset distribution to detect justified vs normal text.
    ///
    /// This method performs statistical analysis on collected TJ offsets to determine
    /// if the document uses justified alignment. Justified text has high variance in TJ
    /// offsets (to distribute whitespace), while normally-spaced text has low variance.
    ///
    /// # Returns
    ///
    /// A tuple `(is_justified: bool, coefficient_of_variation: f32)` where:
    /// - `is_justified`: true if CV > 0.5 (high variance = justified text)
    /// - `coefficient_of_variation`: standard deviation / mean (normalized spread)
    ///
    /// # Algorithm
    ///
    /// Per ISO 32000-1:2008 Section 9.4.4, TJ array offsets are in font-relative units
    /// (1/1000 of text space). The distribution is analyzed as:
    ///
    /// 1. Calculate mean of all TJ offsets
    /// 2. Calculate variance: average of squared deviations from mean
    /// 3. Calculate standard deviation: sqrt(variance)
    /// 4. Calculate coefficient of variation: std_dev / |mean|
    ///
    /// # Thresholds
    ///
    /// - CV > 0.5: Justified text (high variance in offsets)
    /// - CV ≤ 0.5: Normal text (consistent spacing)
    ///
    /// # PDF Spec Compliance
    ///
    /// Per ISO 32000-1:2008 Section 9.10 ("Extraction of Text Content"):
    /// "Determining word boundaries is not specified by PDF." This method uses only
    /// spec-defined TJ offset values to infer text characteristics, not semantic assumptions.
    pub(super) fn analyze_tj_distribution(&self) -> (bool, f32) {
        let n = self.tj_offset_history.len();
        if n == 0 {
            return (false, 0.0);
        }

        // Use the accumulators when current; recompute from the slice if the
        // history was replaced wholesale (same sum order → same result). ~keep
        let (sum, sum_sq) = if self.tj_stats_len == n {
            (self.tj_sum, self.tj_sum_sq)
        } else {
            let mut s = 0.0f64;
            let mut sq = 0.0f64;
            for &x in &self.tj_offset_history {
                let x = x as f64;
                s += x;
                sq += x * x;
            }
            (s, sq)
        };

        Self::tj_distribution_from_stats(n, sum, sum_sq)
    }

    fn tj_distribution_from_stats(n: usize, sum: f64, sum_sq: f64) -> (bool, f32) {
        let nf = n as f64;
        let mean = sum / nf;
        // Variance from accumulators: E[x²] − E[x]². Clamp to ≥0 to absorb
        // floating-point cancellation when the spread is tiny. ~keep
        let variance = ((sum_sq / nf) - mean * mean).max(0.0);
        let std_dev = variance.sqrt();

        // Coefficient of variation (normalized spread); guard zero mean. ~keep
        let cv = if mean.abs() > 0.001 {
            (std_dev / mean.abs()) as f32
        } else {
            0.0
        };

        let is_justified = cv > 0.5;

        tracing::trace!(target: LOG_TARGET,
            "TJ distribution analysis: mean={:.2}, std_dev={:.2}, cv={:.2}, justified={}",
            mean,
            std_dev,
            cv,
            is_justified
        );

        (is_justified, cv)
    }
}
