//! Confidence scoring for extraction outputs.
//!
//! Combines three signals into a single threshold-able score:
//! - `text_coverage` — fraction of pages with usable text (caller supplies from
//!   pdf-level analysis, or 1.0 for non-PDF text formats),
//! - `ocr_aggregate` — mean of `ocr_elements[].confidence.recognition` when OCR ran,
//! - `schema_compliance` — outcome of JSON validation against the caller's schema.

use serde::{Deserialize, Serialize};

use crate::types::extraction::ExtractionResult;

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
    /// Mean OCR per-element recognition confidence; `None` when OCR did not run.
    pub ocr_aggregate: Option<f32>,
    /// Schema-validation result of the merged output.
    pub schema_compliance: SchemaCompliance,
}

impl ConfidenceSignals {
    /// Build `ConfidenceSignals` from an `ExtractionResult`.
    ///
    /// * `result` — The extraction result whose `ocr_elements` are inspected.
    /// * `schema_compliance` — Caller-supplied schema validation outcome.
    /// * `text_coverage` — Caller-supplied fraction of pages with usable text
    ///   (e.g. 1.0 for native text formats, value from PDF analysis for PDFs).
    ///
    /// The `ocr_aggregate` is computed as the arithmetic mean of all
    /// `ocr_elements[].confidence.recognition` values.  When `ocr_elements` is
    /// `None` or empty the field is set to `None`.
    pub fn from_extraction_result(
        result: &ExtractionResult,
        schema_compliance: SchemaCompliance,
        text_coverage: f32,
    ) -> Self {
        let ocr_aggregate = result.ocr_elements.as_deref().and_then(|elements| {
            if elements.is_empty() {
                return None;
            }
            let sum: f64 = elements.iter().map(|e| e.confidence.recognition).sum();
            let mean = sum / elements.len() as f64;
            Some(mean as f32)
        });

        Self {
            text_coverage,
            ocr_aggregate,
            schema_compliance,
        }
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
    /// Mean OCR per-element recognition confidence when OCR ran; `None` when it did not.
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
            // No OCR ran — fold ocr weight into text_coverage so we still sum to 1.0.
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
    use crate::types::extraction::ExtractionResult;
    use crate::types::ocr_elements::{OcrBoundingGeometry, OcrConfidence, OcrElement};

    // -----------------------------------------------------------------------
    // ConfidenceSignals::from_extraction_result
    // -----------------------------------------------------------------------

    fn make_ocr_element(recognition: f64) -> OcrElement {
        OcrElement {
            text: "word".to_string(),
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
        // Three OCR elements with known recognition scores → mean should be (0.7 + 0.8 + 0.9) / 3 = 0.8
        let result = ExtractionResult {
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
        let result = ExtractionResult {
            ocr_elements: None,
            ..Default::default()
        };

        let signals = ConfidenceSignals::from_extraction_result(&result, SchemaCompliance::AllInvalid, 0.5);

        assert!(signals.ocr_aggregate.is_none());
        assert_eq!(signals.schema_compliance, SchemaCompliance::AllInvalid);
    }

    #[test]
    fn from_extraction_result_empty_ocr_elements_returns_none() {
        let result = ExtractionResult {
            ocr_elements: Some(vec![]),
            ..Default::default()
        };

        let signals = ConfidenceSignals::from_extraction_result(&result, SchemaCompliance::PartialValid, 0.8);

        assert!(signals.ocr_aggregate.is_none());
    }

    #[test]
    fn from_extraction_result_single_element_mean_equals_element_confidence() {
        let result = ExtractionResult {
            ocr_elements: Some(vec![make_ocr_element(0.95)]),
            ..Default::default()
        };

        let signals = ConfidenceSignals::from_extraction_result(&result, SchemaCompliance::AllValid, 0.9);

        let ocr_agg = signals.ocr_aggregate.expect("should have ocr_aggregate");
        assert!((ocr_agg as f64 - 0.95).abs() < 0.001, "expected ~0.95, got {}", ocr_agg);
    }

    // -----------------------------------------------------------------------
    // Ported cloud unit tests
    // -----------------------------------------------------------------------

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
        // 0.95*0.30 + 0.95*0.30 + 0.0*0.40 = 0.57
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
        // text weight becomes 0.60; combined = 0.8*0.60 + 1.0*0.40 = 0.88
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
        // combined = 0.2*0.60 + 1.0*0.40 = 0.52
        assert!(
            (conf.combined - 0.52).abs() < 0.01,
            "low text but valid schema: combined should be ~0.52, got {}",
            conf.combined
        );
    }

    #[test]
    fn confidence_clamps_to_valid_range() {
        let signals = ConfidenceSignals {
            text_coverage: 1.5, // impossible but let's verify clamping
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

    // -----------------------------------------------------------------------
    // Exact-value scoring tests (hand-computed expected values)
    // -----------------------------------------------------------------------

    #[test]
    fn should_return_zero_combined_when_all_signals_are_zero_with_ocr() {
        // text=0.0 * 0.30 + ocr=0.0 * 0.30 + schema_invalid=0.0 * 0.40 = 0.0
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
        // text=1.0 * 0.30 + ocr=1.0 * 0.30 + schema_valid=1.0 * 0.40 = 1.0
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
        // text=0.6 * 0.30 = 0.18
        // ocr=0.7 * 0.30  = 0.21
        // schema=PartialValid(0.5) * 0.40 = 0.20
        // combined = 0.59
        let signals = ConfidenceSignals {
            text_coverage: 0.6,
            ocr_aggregate: Some(0.7),
            schema_compliance: SchemaCompliance::PartialValid,
        };
        let result = score_confidence(signals, ConfidenceWeights::default());
        // f32 arithmetic: check within float precision
        let expected: f32 = 0.6 * 0.30 + 0.7 * 0.30 + 0.5 * 0.40;
        assert_eq!(result.combined, expected, "combined should be exactly {expected}");
    }

    #[test]
    fn should_compute_exact_combined_for_mixed_signals_without_ocr() {
        // merged_text_weight = 0.30 + 0.30 = 0.60
        // text=0.75 * 0.60 = 0.45
        // schema=AllValid(1.0) * 0.40 = 0.40
        // combined = 0.85
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
        // Default weights: text=0.30, ocr=0.30, schema=0.40
        // Override: text=0.50, ocr=0.10, schema=0.40
        // signals: text=0.9, ocr=Some(0.2), schema=AllValid(1.0)
        //
        // default: 0.9*0.30 + 0.2*0.30 + 1.0*0.40 = 0.27 + 0.06 + 0.40 = 0.73
        // custom:  0.9*0.50 + 0.2*0.10 + 1.0*0.40 = 0.45 + 0.02 + 0.40 = 0.87
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
        // The two must differ — this confirms weights actually affect the result.
        assert_ne!(
            default_result.combined, custom_result.combined,
            "custom weights must produce a different combined score"
        );
    }

    // -----------------------------------------------------------------------
    // Default weight field values
    // -----------------------------------------------------------------------

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

    // -----------------------------------------------------------------------
    // ExtractionConfidence field pass-through
    // -----------------------------------------------------------------------

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

    // -----------------------------------------------------------------------
    // Clamping boundary
    // -----------------------------------------------------------------------

    #[test]
    fn should_clamp_combined_to_zero_when_inputs_are_negative() {
        // Negative inputs are outside documented range but clamping must hold.
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

    // -----------------------------------------------------------------------
    // Serde round-trips
    // -----------------------------------------------------------------------

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
}
