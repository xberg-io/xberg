//! Confidence scoring for extraction outputs.
//!
//! Combines three signals into a single threshold-able score:
//! - `text_coverage` — fraction of pages with usable text (caller supplies from
//!   pdf-level analysis, or 1.0 for non-PDF text formats),
//! - `ocr_aggregate` — recognition confidence averaged over every OCR'd word, weighted by word
//!   count; sourced from `ocr_elements` when OCR ran on embedded images, or from
//!   `PageContent.ocr_confidence` for the page-level OCR route when `ocr_elements` is absent,
//! - `schema_compliance` — outcome of JSON validation against the caller's schema.

use serde::{Deserialize, Serialize};

use crate::types::extraction::ExtractedDocument;
use crate::types::ocr_elements::{OcrElement, element_level_rank};
use crate::types::page::PageContent;

/// Schema-validation outcome surfaced as one of three buckets.
///
/// Fold into the combined confidence score without leaking internal validation
/// error types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum SchemaCompliance {
    /// Every batch validated against the schema.
    AllValid,
    /// At least one batch validated; at least one did not.
    PartialValid,
    /// No batch validated.
    AllInvalid,
}

impl SchemaCompliance {
    /// Map the compliance bucket to a scalar weight in `[0, 1]`.
    pub fn score(self) -> f32 {
        match self {
            SchemaCompliance::AllValid => 1.0,
            SchemaCompliance::PartialValid => 0.5,
            SchemaCompliance::AllInvalid => 0.0,
        }
    }
}

/// Input signals for confidence scoring.
///
/// Caller fills these from the extraction result and the LLM response.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ConfidenceSignals {
    /// Fraction of pages with usable text in `[0, 1]`.
    pub text_coverage: f32,
    /// OCR recognition confidence, averaged over every recognized word and weighted by word
    /// count rather than by element or page count; `None` when OCR did not run.
    pub ocr_aggregate: Option<f32>,
    /// Schema-validation result of the merged output.
    pub schema_compliance: SchemaCompliance,
}

impl ConfidenceSignals {
    /// Build `ConfidenceSignals` from an `ExtractedDocument`.
    ///
    /// * `result` — The extraction result whose `ocr_elements` are inspected.
    /// * `schema_compliance` — Caller-supplied schema validation outcome.
    /// * `text_coverage` — Caller-supplied fraction of pages with usable text
    ///   (e.g. 1.0 for native text formats, value from PDF analysis for PDFs).
    ///
    /// The `ocr_aggregate` is the word-count-weighted mean recognition confidence, taken from
    /// `ocr_elements` when present ([`Self::ocr_aggregate_from_elements`]) or, for the
    /// page-level OCR route, from `PageContent.ocr_confidence`
    /// ([`Self::ocr_aggregate_from_pages`]). Both use the same weighting, so the field carries
    /// one statistic regardless of which route populated it. `None` when neither source has a
    /// recognized word.
    pub fn from_extraction_result(
        result: &ExtractedDocument,
        schema_compliance: SchemaCompliance,
        text_coverage: f32,
    ) -> Self {
        let ocr_aggregate =
            Self::ocr_aggregate_from_elements(result).or_else(|| Self::ocr_aggregate_from_pages(result));

        Self {
            text_coverage,
            ocr_aggregate,
            schema_compliance,
        }
    }

    /// Word-count-weighted mean OCR recognition confidence from `ocr_elements`, the field
    /// `apply_public_element_policy` populates. Its only two call sites are both embedded-image
    /// OCR (`extraction/image_ocr.rs`, `extractors/pdf/mod.rs`'s `ocr_inline_images` path), so
    /// this alone is `None` for the page-level OCR route (issue #1677);
    /// [`Self::ocr_aggregate_from_pages`] covers that route.
    ///
    /// Weighted by each element's own word count (`text.split_whitespace().count()`), not
    /// counted flat per element. PaddleOCR's native elements are `Line`-level
    /// (`OcrElementLevel::Line`), and the default `OcrElementConfig::min_level` passes lines
    /// through unfiltered, so a flat per-element mean would silently change meaning whenever
    /// line lengths vary. Weighting by word count keeps the statistic "mean confidence per
    /// recognized word" regardless of the element granularity a backend reports, which is also
    /// what [`Self::ocr_aggregate_from_pages`] computes. Narrows [`Self::ocr_confidence_from_elements`]
    /// to `f32`; callers that need the underlying word count use that fn directly.
    fn ocr_aggregate_from_elements(result: &ExtractedDocument) -> Option<f32> {
        let elements = result.ocr_elements.as_deref()?;
        Self::ocr_confidence_from_elements(elements).map(|(mean, _total_words)| mean as f32)
    }

    /// Word-count-weighted mean OCR recognition confidence for the page-level OCR route (issue
    /// #1677), which reports per-page confidence (`PageContent.ocr_confidence`) but never
    /// populates `ocr_elements`. Uses the SAME weighting [`Self::ocr_aggregate_from_elements`]
    /// does, so the two routes agree on what `ocr_aggregate` means regardless of which one
    /// populated it. Narrows [`Self::ocr_confidence_from_pages`] to `f32`; callers that need the
    /// underlying word count use that fn directly.
    fn ocr_aggregate_from_pages(result: &ExtractedDocument) -> Option<f32> {
        let pages = result.pages.as_deref()?;
        Self::ocr_confidence_from_pages(pages).map(|(mean, _total_words)| mean as f32)
    }

    /// Word-count-weighted mean OCR recognition confidence from a slice of `ocr_elements`, paired
    /// with the total recognized word count the mean was computed over. `pub(crate)` so
    /// `text::quality_processor` can apply its own evidence floor to the same fold instead of
    /// recomputing it (issue #1694); [`Self::ocr_aggregate_from_elements`] is the `f32`,
    /// floor-free wrapper `ocr_aggregate` itself uses. `None` when no element carries a word.
    ///
    /// Only the finest element level that carries a word is folded: a backend that reports words
    /// together with the lines holding them would otherwise count every word twice.
    pub(crate) fn ocr_confidence_from_elements(elements: &[OcrElement]) -> Option<(f64, u64)> {
        let word_count = |element: &OcrElement| element.text.split_whitespace().count() as u64;
        let finest_rank = elements
            .iter()
            .filter(|element| word_count(element) > 0)
            .map(|element| element_level_rank(element.level))
            .min()?;
        Self::word_count_weighted_mean(
            elements
                .iter()
                .filter(|element| element_level_rank(element.level) == finest_rank)
                .map(|element| (element.confidence.recognition, word_count(element))),
        )
    }

    /// Word-count-weighted mean OCR recognition confidence from a slice of `PageContent`, paired
    /// with the total recognized word count the mean was computed over. `pub(crate)` for the same
    /// reason as [`Self::ocr_confidence_from_elements`] (issue #1694); [`Self::ocr_aggregate_from_pages`]
    /// is the `f32`, floor-free wrapper `ocr_aggregate` itself uses. A page whose backend reports
    /// no calibrated legibility scale (`score: None`) is skipped rather than treated as zero
    /// confidence. `None` when no page carries a word.
    pub(crate) fn ocr_confidence_from_pages(pages: &[PageContent]) -> Option<(f64, u64)> {
        Self::word_count_weighted_mean(
            pages
                .iter()
                .filter_map(|page| page.ocr_confidence.as_ref())
                .filter_map(|confidence| confidence.score.map(|score| (score, u64::from(confidence.word_count)))),
        )
    }

    /// Fold `(recognition_score, word_count)` pairs into their word-count-weighted mean, paired
    /// with the total word count folded. Shared by [`Self::ocr_confidence_from_elements`] and
    /// [`Self::ocr_confidence_from_pages`] so the two routes cannot drift onto different
    /// weightings. `None` when no pair carries a word.
    ///
    /// ~keep: this is the one fold both `ocr_aggregate` and the quality-score cap build on, and
    /// they read different amounts of trust into the same mean. `ocr_aggregate` reports
    /// whenever any word was recognized; the quality-score cap applies its own, stricter
    /// evidence floor on top of this fn's `total_words` before trusting the mean as a ceiling
    /// (issue #1694). That is deliberate, not drift: `ocr_aggregate` is a diagnostic value read
    /// on its own, while the cap silently lowers a score callers otherwise trust at face value
    /// and needs more evidence before it will do that.
    fn word_count_weighted_mean(pairs: impl Iterator<Item = (f64, u64)>) -> Option<(f64, u64)> {
        let (weighted_sum, total_words) = pairs.fold((0.0_f64, 0_u64), |(sum, words), (score, word_count)| {
            (sum + score * word_count as f64, words + word_count)
        });

        (total_words > 0).then(|| (weighted_sum / total_words as f64, total_words))
    }
}

/// Tunable weights for the confidence scoring formula.
///
/// Defaults picked by inspection; callers tune them via config.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ConfidenceWeights {
    /// Weight assigned to `text_coverage`. Default 0.30.
    pub text_coverage: f32,
    /// Weight assigned to `ocr_aggregate` when OCR ran.
    ///
    /// Default 0.30 — folds into `text_coverage` weight when OCR did not run.
    pub ocr_aggregate: f32,
    /// Weight assigned to `schema_compliance`. Default 0.40.
    pub schema_compliance: f32,
}

impl Default for ConfidenceWeights {
    fn default() -> Self {
        Self {
            text_coverage: 0.30,
            ocr_aggregate: 0.30,
            schema_compliance: 0.40,
        }
    }
}

impl ConfidenceWeights {
    /// Validate that weights sum to approximately 1.0.
    pub fn is_normalized(&self) -> bool {
        let sum = self.text_coverage + self.ocr_aggregate + self.schema_compliance;
        (sum - 1.0).abs() < 0.01
    }
}

/// Combined confidence on `[0, 1]`.
///
/// When OCR did not run, the `ocr_aggregate` weight folds into `text_coverage`
/// so the weighted sum still totals 1.0.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct ExtractionConfidence {
    /// Fraction of pages with a usable text layer.
    pub text_coverage: f32,
    /// OCR recognition confidence, word-count-weighted across every recognized word, when OCR
    /// ran; `None` when it did not.
    pub ocr_aggregate: Option<f32>,
    /// Whether the merged output validates against the preset schema.
    pub schema_compliance: SchemaCompliance,
    /// Weighted blend in `[0, 1]`.  The value compared against the fallback threshold.
    pub combined: f32,
}

/// Score a [`ConfidenceSignals`] triple into an [`ExtractionConfidence`] using
/// the supplied weights.
///
/// When `signals.ocr_aggregate` is `None`, the OCR weight folds into
/// `text_coverage` so the weighted sum still totals 1.0.
pub fn score_confidence(signals: ConfidenceSignals, weights: ConfidenceWeights) -> ExtractionConfidence {
    let schema_score = signals.schema_compliance.score();
    let combined = match signals.ocr_aggregate {
        Some(ocr) => {
            signals.text_coverage * weights.text_coverage
                + ocr * weights.ocr_aggregate
                + schema_score * weights.schema_compliance
        }
        None => {
            let merged_text_weight = weights.text_coverage + weights.ocr_aggregate;
            signals.text_coverage * merged_text_weight + schema_score * weights.schema_compliance
        }
    };
    ExtractionConfidence {
        text_coverage: signals.text_coverage,
        ocr_aggregate: signals.ocr_aggregate,
        schema_compliance: signals.schema_compliance,
        combined: combined.clamp(0.0, 1.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::extraction::ExtractedDocument;
    use crate::types::ocr_elements::{OcrBoundingGeometry, OcrConfidence, OcrElement};

    fn make_ocr_element(recognition: f64) -> OcrElement {
        make_ocr_element_with_text(recognition, "word")
    }

    /// Builds an OCR element with a given recognized text, so its word count (used to weight
    /// `ocr_aggregate_from_elements`) can be controlled directly, the way a PaddleOCR `Line`
    /// element's word count varies with how many words are on that line.
    fn make_ocr_element_with_text(recognition: f64, text: &str) -> OcrElement {
        OcrElement {
            text: text.to_string(),
            geometry: OcrBoundingGeometry::default(),
            confidence: OcrConfidence {
                detection: None,
                recognition,
            },
            ..Default::default()
        }
    }

    #[test]
    fn from_extraction_result_computes_mean_recognition_confidence() {
        let result = ExtractedDocument {
            ocr_elements: Some(vec![
                make_ocr_element(0.7),
                make_ocr_element(0.8),
                make_ocr_element(0.9),
            ]),
            ..Default::default()
        };

        let signals = ConfidenceSignals::from_extraction_result(&result, SchemaCompliance::AllValid, 1.0);

        let ocr_agg = signals.ocr_aggregate.expect("should have ocr_aggregate");
        assert!((ocr_agg - 0.8).abs() < 0.001, "expected mean ~0.8, got {}", ocr_agg);
        assert_eq!(signals.schema_compliance, SchemaCompliance::AllValid);
        assert!((signals.text_coverage - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn from_extraction_result_no_ocr_elements_returns_none() {
        let result = ExtractedDocument {
            ocr_elements: None,
            ..Default::default()
        };

        let signals = ConfidenceSignals::from_extraction_result(&result, SchemaCompliance::AllInvalid, 0.5);

        assert!(signals.ocr_aggregate.is_none());
        assert_eq!(signals.schema_compliance, SchemaCompliance::AllInvalid);
    }

    #[test]
    fn from_extraction_result_empty_ocr_elements_returns_none() {
        let result = ExtractedDocument {
            ocr_elements: Some(vec![]),
            ..Default::default()
        };

        let signals = ConfidenceSignals::from_extraction_result(&result, SchemaCompliance::PartialValid, 0.8);

        assert!(signals.ocr_aggregate.is_none());
    }

    #[test]
    fn from_extraction_result_single_element_mean_equals_element_confidence() {
        let result = ExtractedDocument {
            ocr_elements: Some(vec![make_ocr_element(0.95)]),
            ..Default::default()
        };

        let signals = ConfidenceSignals::from_extraction_result(&result, SchemaCompliance::AllValid, 0.9);

        let ocr_agg = signals.ocr_aggregate.expect("should have ocr_aggregate");
        assert!((ocr_agg as f64 - 0.95).abs() < 0.001, "expected ~0.95, got {}", ocr_agg);
    }

    // xberg#1677 finding 1: PaddleOCR's native elements are Line-level, and the default
    // `OcrElementConfig::min_level` passes lines through unfiltered, so a flat mean over
    // elements silently becomes "confidence per line" rather than "confidence per word"
    // whenever line lengths vary. `ocr_aggregate_from_elements` must weight by each element's
    // own word count, matching what `ocr_aggregate_from_pages` already does.
    #[test]
    fn from_extraction_result_weights_elements_by_word_count_not_flat_per_element() {
        let result = ExtractedDocument {
            ocr_elements: Some(vec![
                make_ocr_element_with_text(0.9, "one"),
                make_ocr_element_with_text(0.9, "two"),
                make_ocr_element_with_text(0.1, "three words here"),
            ]),
            ..Default::default()
        };

        let signals = ConfidenceSignals::from_extraction_result(&result, SchemaCompliance::AllValid, 1.0);

        // Word-count-weighted: (0.9*1 + 0.9*1 + 0.1*3) / 5 = 0.42. A flat per-element mean
        // would give (0.9 + 0.9 + 0.1) / 3 = 0.6333, which is the bug this pins.
        let ocr_agg = signals.ocr_aggregate.expect("should have ocr_aggregate");
        assert!(
            (ocr_agg - 0.42).abs() < 0.001,
            "expected word-count-weighted 0.42, got {ocr_agg}"
        );
    }

    #[test]
    fn default_weights_are_normalized() {
        let w = ConfidenceWeights::default();
        assert!(w.is_normalized(), "default weights should sum to 1.0");
    }

    #[test]
    fn schema_compliance_scores() {
        assert_eq!(SchemaCompliance::AllValid.score(), 1.0);
        assert_eq!(SchemaCompliance::PartialValid.score(), 0.5);
        assert_eq!(SchemaCompliance::AllInvalid.score(), 0.0);
    }

    #[test]
    fn all_signals_high_produces_high_confidence() {
        let signals = ConfidenceSignals {
            text_coverage: 0.95,
            ocr_aggregate: Some(0.90),
            schema_compliance: SchemaCompliance::AllValid,
        };
        let conf = score_confidence(signals, ConfidenceWeights::default());
        assert!(conf.combined > 0.85, "all high signals should yield high confidence");
    }

    #[test]
    fn all_signals_low_produces_low_confidence() {
        let signals = ConfidenceSignals {
            text_coverage: 0.10,
            ocr_aggregate: Some(0.05),
            schema_compliance: SchemaCompliance::AllInvalid,
        };
        let conf = score_confidence(signals, ConfidenceWeights::default());
        assert!(conf.combined < 0.20, "all low signals should yield low confidence");
    }

    #[test]
    fn schema_invalid_dominates_despite_high_text_and_ocr() {
        let signals = ConfidenceSignals {
            text_coverage: 0.95,
            ocr_aggregate: Some(0.95),
            schema_compliance: SchemaCompliance::AllInvalid,
        };
        let conf = score_confidence(signals, ConfidenceWeights::default());
        assert!(
            conf.combined > 0.4 && conf.combined < 0.65,
            "schema_invalid dominates: got {}",
            conf.combined
        );
    }

    #[test]
    fn no_ocr_folds_weight_to_text_coverage() {
        let signals = ConfidenceSignals {
            text_coverage: 0.8,
            ocr_aggregate: None,
            schema_compliance: SchemaCompliance::AllValid,
        };
        let conf = score_confidence(signals, ConfidenceWeights::default());
        assert!(
            (conf.combined - 0.88).abs() < 0.01,
            "no OCR: combined should be ~0.88, got {}",
            conf.combined
        );
    }

    #[test]
    fn no_ocr_low_text_coverage_still_recovers_with_valid_schema() {
        let signals = ConfidenceSignals {
            text_coverage: 0.2,
            ocr_aggregate: None,
            schema_compliance: SchemaCompliance::AllValid,
        };
        let conf = score_confidence(signals, ConfidenceWeights::default());
        assert!(
            (conf.combined - 0.52).abs() < 0.01,
            "low text but valid schema: combined should be ~0.52, got {}",
            conf.combined
        );
    }

    #[test]
    fn confidence_clamps_to_valid_range() {
        let signals = ConfidenceSignals {
            text_coverage: 1.5,
            ocr_aggregate: Some(1.5),
            schema_compliance: SchemaCompliance::AllValid,
        };
        let conf = score_confidence(signals, ConfidenceWeights::default());
        assert!(
            conf.combined >= 0.0 && conf.combined <= 1.0,
            "confidence should be clamped to [0,1], got {}",
            conf.combined
        );
    }

    #[test]
    fn partial_schema_compliance_scores_midway() {
        let signals_all = ConfidenceSignals {
            text_coverage: 1.0,
            ocr_aggregate: None,
            schema_compliance: SchemaCompliance::AllValid,
        };
        let signals_partial = ConfidenceSignals {
            text_coverage: 1.0,
            ocr_aggregate: None,
            schema_compliance: SchemaCompliance::PartialValid,
        };
        let signals_none = ConfidenceSignals {
            text_coverage: 1.0,
            ocr_aggregate: None,
            schema_compliance: SchemaCompliance::AllInvalid,
        };

        let conf_all = score_confidence(signals_all, ConfidenceWeights::default());
        let conf_partial = score_confidence(signals_partial, ConfidenceWeights::default());
        let conf_none = score_confidence(signals_none, ConfidenceWeights::default());

        assert!(conf_all.combined > conf_partial.combined);
        assert!(conf_partial.combined > conf_none.combined);
    }

    #[test]
    fn should_return_zero_combined_when_all_signals_are_zero_with_ocr() {
        let signals = ConfidenceSignals {
            text_coverage: 0.0,
            ocr_aggregate: Some(0.0),
            schema_compliance: SchemaCompliance::AllInvalid,
        };
        let result = score_confidence(signals, ConfidenceWeights::default());
        assert_eq!(result.combined, 0.0);
    }

    #[test]
    fn should_return_one_combined_when_all_signals_are_max_with_ocr() {
        let signals = ConfidenceSignals {
            text_coverage: 1.0,
            ocr_aggregate: Some(1.0),
            schema_compliance: SchemaCompliance::AllValid,
        };
        let result = score_confidence(signals, ConfidenceWeights::default());
        assert_eq!(result.combined, 1.0);
    }

    #[test]
    fn should_compute_exact_combined_for_mixed_realistic_signals_with_ocr() {
        let signals = ConfidenceSignals {
            text_coverage: 0.6,
            ocr_aggregate: Some(0.7),
            schema_compliance: SchemaCompliance::PartialValid,
        };
        let result = score_confidence(signals, ConfidenceWeights::default());
        let expected: f32 = 0.6 * 0.30 + 0.7 * 0.30 + 0.5 * 0.40;
        assert_eq!(result.combined, expected, "combined should be exactly {expected}");
    }

    #[test]
    fn should_compute_exact_combined_for_mixed_signals_without_ocr() {
        let signals = ConfidenceSignals {
            text_coverage: 0.75,
            ocr_aggregate: None,
            schema_compliance: SchemaCompliance::AllValid,
        };
        let result = score_confidence(signals, ConfidenceWeights::default());
        let expected: f32 = 0.75 * 0.60 + 1.0 * 0.40;
        assert_eq!(result.combined, expected, "combined should be exactly {expected}");
    }

    #[test]
    fn should_produce_different_combined_when_weights_are_overridden() {
        let signals = ConfidenceSignals {
            text_coverage: 0.9,
            ocr_aggregate: Some(0.2),
            schema_compliance: SchemaCompliance::AllValid,
        };
        let default_result = score_confidence(signals, ConfidenceWeights::default());
        let custom_weights = ConfidenceWeights {
            text_coverage: 0.50,
            ocr_aggregate: 0.10,
            schema_compliance: 0.40,
        };
        let custom_result = score_confidence(signals, custom_weights);

        let expected_default: f32 = 0.9 * 0.30 + 0.2 * 0.30 + 1.0 * 0.40;
        let expected_custom: f32 = 0.9 * 0.50 + 0.2 * 0.10 + 1.0 * 0.40;

        assert_eq!(
            default_result.combined, expected_default,
            "default weights: expected {expected_default}"
        );
        assert_eq!(
            custom_result.combined, expected_custom,
            "custom weights: expected {expected_custom}"
        );
        assert_ne!(
            default_result.combined, custom_result.combined,
            "custom weights must produce a different combined score"
        );
    }

    #[test]
    fn should_have_exact_default_weight_fields() {
        let w = ConfidenceWeights::default();
        assert_eq!(w.text_coverage, 0.30, "default text_coverage weight should be 0.30");
        assert_eq!(w.ocr_aggregate, 0.30, "default ocr_aggregate weight should be 0.30");
        assert_eq!(
            w.schema_compliance, 0.40,
            "default schema_compliance weight should be 0.40"
        );
    }

    #[test]
    fn should_is_normalized_return_false_when_weights_do_not_sum_to_one() {
        let w = ConfidenceWeights {
            text_coverage: 0.50,
            ocr_aggregate: 0.50,
            schema_compliance: 0.50,
        };
        assert!(!w.is_normalized(), "weights summing to 1.5 should not be normalized");
    }

    #[test]
    fn should_is_normalized_return_false_when_weights_sum_below_one() {
        let w = ConfidenceWeights {
            text_coverage: 0.10,
            ocr_aggregate: 0.10,
            schema_compliance: 0.10,
        };
        assert!(!w.is_normalized(), "weights summing to 0.3 should not be normalized");
    }

    #[test]
    fn should_thread_signal_fields_into_extraction_confidence() {
        let signals = ConfidenceSignals {
            text_coverage: 0.55,
            ocr_aggregate: Some(0.65),
            schema_compliance: SchemaCompliance::PartialValid,
        };
        let result = score_confidence(signals, ConfidenceWeights::default());
        assert_eq!(result.text_coverage, 0.55);
        assert_eq!(result.ocr_aggregate, Some(0.65));
        assert_eq!(result.schema_compliance, SchemaCompliance::PartialValid);
    }

    #[test]
    fn should_set_ocr_aggregate_to_none_in_confidence_when_signals_have_none() {
        let signals = ConfidenceSignals {
            text_coverage: 0.8,
            ocr_aggregate: None,
            schema_compliance: SchemaCompliance::AllValid,
        };
        let result = score_confidence(signals, ConfidenceWeights::default());
        assert!(result.ocr_aggregate.is_none());
    }

    #[test]
    fn should_clamp_combined_to_zero_when_inputs_are_negative() {
        let signals = ConfidenceSignals {
            text_coverage: -1.0,
            ocr_aggregate: Some(-1.0),
            schema_compliance: SchemaCompliance::AllInvalid,
        };
        let result = score_confidence(signals, ConfidenceWeights::default());
        assert_eq!(result.combined, 0.0, "negative inputs must clamp to 0.0");
    }

    #[test]
    fn should_clamp_combined_to_one_when_inputs_exceed_one() {
        let signals = ConfidenceSignals {
            text_coverage: 2.0,
            ocr_aggregate: Some(2.0),
            schema_compliance: SchemaCompliance::AllValid,
        };
        let result = score_confidence(signals, ConfidenceWeights::default());
        assert_eq!(result.combined, 1.0, "inputs > 1.0 must clamp to 1.0");
    }

    #[test]
    fn should_serialize_schema_compliance_variants_with_snake_case_names() {
        let all_valid = serde_json::to_string(&SchemaCompliance::AllValid).unwrap();
        let partial_valid = serde_json::to_string(&SchemaCompliance::PartialValid).unwrap();
        let all_invalid = serde_json::to_string(&SchemaCompliance::AllInvalid).unwrap();

        assert_eq!(all_valid, r#""all_valid""#);
        assert_eq!(partial_valid, r#""partial_valid""#);
        assert_eq!(all_invalid, r#""all_invalid""#);
    }

    #[test]
    fn should_deserialize_schema_compliance_from_snake_case_names() {
        let all_valid: SchemaCompliance = serde_json::from_str(r#""all_valid""#).unwrap();
        let partial_valid: SchemaCompliance = serde_json::from_str(r#""partial_valid""#).unwrap();
        let all_invalid: SchemaCompliance = serde_json::from_str(r#""all_invalid""#).unwrap();

        assert_eq!(all_valid, SchemaCompliance::AllValid);
        assert_eq!(partial_valid, SchemaCompliance::PartialValid);
        assert_eq!(all_invalid, SchemaCompliance::AllInvalid);
    }

    #[test]
    fn should_round_trip_extraction_confidence_through_json() {
        let signals = ConfidenceSignals {
            text_coverage: 0.8,
            ocr_aggregate: Some(0.75),
            schema_compliance: SchemaCompliance::AllValid,
        };
        let original = score_confidence(signals, ConfidenceWeights::default());
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: ExtractionConfidence = serde_json::from_str(&json).unwrap();
        assert_eq!(original, deserialized);
    }

    /// GH#1624's decisive assertion: with every other signal held identical, a failed
    /// structured extraction (`AllInvalid`) must score strictly below a successful one
    /// (`AllValid`) -- not just differently, and not by an unverified margin.
    #[test]
    fn should_score_all_invalid_strictly_below_all_valid_with_identical_other_signals() {
        let text_coverage = 0.8;
        let ocr_aggregate = Some(0.7);
        let weights = ConfidenceWeights::default();

        let successful = score_confidence(
            ConfidenceSignals {
                text_coverage,
                ocr_aggregate,
                schema_compliance: SchemaCompliance::AllValid,
            },
            weights,
        );
        let failed = score_confidence(
            ConfidenceSignals {
                text_coverage,
                ocr_aggregate,
                schema_compliance: SchemaCompliance::AllInvalid,
            },
            weights,
        );

        let expected_successful: f32 = text_coverage * weights.text_coverage
            + 0.7 * weights.ocr_aggregate
            + SchemaCompliance::AllValid.score() * weights.schema_compliance;
        let expected_failed: f32 = text_coverage * weights.text_coverage
            + 0.7 * weights.ocr_aggregate
            + SchemaCompliance::AllInvalid.score() * weights.schema_compliance;

        assert_eq!(successful.combined, expected_successful);
        assert_eq!(failed.combined, expected_failed);
        assert!(
            failed.combined < successful.combined,
            "failed ({}) must score strictly below successful ({})",
            failed.combined,
            successful.combined
        );
    }

    #[test]
    fn should_round_trip_extraction_confidence_with_no_ocr_through_json() {
        let signals = ConfidenceSignals {
            text_coverage: 0.5,
            ocr_aggregate: None,
            schema_compliance: SchemaCompliance::PartialValid,
        };
        let original = score_confidence(signals, ConfidenceWeights::default());
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: ExtractionConfidence = serde_json::from_str(&json).unwrap();
        assert_eq!(original, deserialized);
        assert!(deserialized.ocr_aggregate.is_none());
    }

    // xberg#1677: the page-level OCR route never populates `ocr_elements` (only
    // `apply_public_element_policy`'s two embedded-image call sites do), but it reports a
    // per-page confidence. These pin the fallback and the claim that it agrees with the
    // elements-based formula.
    fn ocrd_page(page_number: u32, score: Option<f64>, word_count: u32) -> crate::types::PageContent {
        crate::types::PageContent {
            page_number,
            content: String::new(),
            tables: Vec::new(),
            image_indices: Vec::new(),
            image_preprocessing: None,
            hierarchy: None,
            is_blank: None,
            layout_regions: None,
            speaker_notes: None,
            section_name: None,
            sheet_name: None,
            ocr_confidence: Some(crate::types::page::PageOcrConfidence {
                score,
                word_count,
                backend: "tesseract".to_string(),
            }),
        }
    }

    #[test]
    fn from_extraction_result_falls_back_to_page_confidence_when_no_elements() {
        let result = ExtractedDocument {
            ocr_elements: None,
            pages: Some(vec![ocrd_page(1, Some(0.9), 100), ocrd_page(2, Some(0.5), 300)]),
            ..Default::default()
        };

        let signals = ConfidenceSignals::from_extraction_result(&result, SchemaCompliance::AllValid, 1.0);

        // (0.9*100 + 0.5*300) / 400 = 0.6, the same word-count-weighted mean issue #1669's
        // quality_score fix used for this exact data shape.
        let ocr_agg = signals
            .ocr_aggregate
            .expect("page-route fallback must populate ocr_aggregate");
        assert!((ocr_agg - 0.6).abs() < 1e-6, "expected 0.6, got {ocr_agg}");
    }

    #[test]
    fn from_extraction_result_agrees_with_the_elements_formula_word_for_word() {
        // Three pages, each carrying exactly one word's worth of confidence, must produce
        // the identical aggregate an `ocr_elements` list of the same three words would:
        // both are "the mean over every recognized word".
        let via_pages = ExtractedDocument {
            ocr_elements: None,
            pages: Some(vec![
                ocrd_page(1, Some(0.7), 1),
                ocrd_page(2, Some(0.8), 1),
                ocrd_page(3, Some(0.9), 1),
            ]),
            ..Default::default()
        };
        let via_elements = ExtractedDocument {
            ocr_elements: Some(vec![
                make_ocr_element(0.7),
                make_ocr_element(0.8),
                make_ocr_element(0.9),
            ]),
            ..Default::default()
        };

        let pages_signals = ConfidenceSignals::from_extraction_result(&via_pages, SchemaCompliance::AllValid, 1.0);
        let elements_signals =
            ConfidenceSignals::from_extraction_result(&via_elements, SchemaCompliance::AllValid, 1.0);

        assert_eq!(
            pages_signals.ocr_aggregate, elements_signals.ocr_aggregate,
            "the page-route fallback must agree with the embedded-image route's own formula"
        );
    }

    #[test]
    fn from_extraction_result_agrees_with_elements_formula_when_line_lengths_vary() {
        // Same reproduction as the flat-vs-weighted element test above, but this time compared
        // against the page route for the same underlying word-level confidences: two 1-word
        // lines at 0.9 and one 3-word line at 0.1. Both routes must land on the same aggregate,
        // 0.42, regardless of which one filled it.
        let via_elements = ExtractedDocument {
            ocr_elements: Some(vec![
                make_ocr_element_with_text(0.9, "one"),
                make_ocr_element_with_text(0.9, "two"),
                make_ocr_element_with_text(0.1, "three words here"),
            ]),
            ..Default::default()
        };
        let via_pages = ExtractedDocument {
            ocr_elements: None,
            pages: Some(vec![
                ocrd_page(1, Some(0.9), 1),
                ocrd_page(2, Some(0.9), 1),
                ocrd_page(3, Some(0.1), 3),
            ]),
            ..Default::default()
        };

        let elements_signals =
            ConfidenceSignals::from_extraction_result(&via_elements, SchemaCompliance::AllValid, 1.0);
        let pages_signals = ConfidenceSignals::from_extraction_result(&via_pages, SchemaCompliance::AllValid, 1.0);

        assert_eq!(
            elements_signals.ocr_aggregate, pages_signals.ocr_aggregate,
            "the two routes must agree even when element/line word counts vary"
        );
    }

    #[test]
    fn from_extraction_result_prefers_elements_over_pages_when_both_present() {
        let result = ExtractedDocument {
            ocr_elements: Some(vec![
                make_ocr_element(0.7),
                make_ocr_element(0.8),
                make_ocr_element(0.9),
            ]),
            pages: Some(vec![ocrd_page(1, Some(0.1), 500)]),
            ..Default::default()
        };

        let signals = ConfidenceSignals::from_extraction_result(&result, SchemaCompliance::AllValid, 1.0);

        let ocr_agg = signals.ocr_aggregate.expect("must have ocr_aggregate");
        assert!(
            (ocr_agg - 0.8).abs() < 0.001,
            "elements must take priority over the page fallback, got {ocr_agg}"
        );
    }

    #[test]
    fn from_extraction_result_pages_with_no_score_do_not_count_as_zero_confidence() {
        let result = ExtractedDocument {
            ocr_elements: None,
            pages: Some(vec![ocrd_page(1, None, 500)]),
            ..Default::default()
        };

        let signals = ConfidenceSignals::from_extraction_result(&result, SchemaCompliance::AllValid, 1.0);

        assert!(
            signals.ocr_aggregate.is_none(),
            "an uncalibrated backend's None score must not be treated as zero confidence"
        );
    }

    fn leveled_element(recognition: f64, text: &str, level: crate::types::OcrElementLevel) -> OcrElement {
        OcrElement {
            level,
            ..make_ocr_element_with_text(recognition, text)
        }
    }

    /// Tesseract reports each line before its words, the line holding the words' mean.
    #[test]
    fn ocr_confidence_from_elements_counts_tesseract_words_once() {
        use crate::types::OcrElementLevel::{Line, Word};
        let elements = [
            leveled_element(0.75, "Total due", Line),
            leveled_element(0.8, "42", Line),
            leveled_element(0.9, "Total", Word),
            leveled_element(0.6, "due", Word),
            leveled_element(0.8, "42", Word),
        ];

        let (mean, total_words) = ConfidenceSignals::ocr_confidence_from_elements(&elements).unwrap();

        assert_eq!(total_words, 3);
        assert!((mean - 2.3 / 3.0).abs() < 1e-9);
    }

    /// PaddleOCR scores a line separately from the words it splits the line into.
    #[test]
    fn ocr_confidence_from_elements_folds_paddle_words_not_their_lines() {
        use crate::types::OcrElementLevel::{Line, Word};
        let elements = [
            leveled_element(0.2, "Total due", Line),
            leveled_element(0.9, "Total", Word),
            leveled_element(0.7, "due", Word),
        ];

        let (mean, total_words) = ConfidenceSignals::ocr_confidence_from_elements(&elements).unwrap();

        assert_eq!(total_words, 2);
        assert!((mean - 0.8).abs() < 1e-9);
    }

    #[test]
    fn ocr_confidence_from_elements_folds_lines_when_no_word_is_present() {
        use crate::types::OcrElementLevel::{Block, Line};
        let elements = [
            leveled_element(0.9, "Total due 42", Block),
            leveled_element(0.6, "Total due", Line),
            leveled_element(0.3, "42", Line),
        ];

        let (mean, total_words) = ConfidenceSignals::ocr_confidence_from_elements(&elements).unwrap();

        assert_eq!(total_words, 3);
        assert!((mean - 0.5).abs() < 1e-9);
    }
}
