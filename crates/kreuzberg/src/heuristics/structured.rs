//! Structured-extraction call-mode heuristic.
//!
//! Decides whether a document should enter the structured-extraction pipeline
//! as text-only, vision-only, text+vision — or be skipped entirely.  This is
//! the chokepoint that keeps LLM calls minimal: pure-text invoices never see
//! the vision model.
//!
//! # Modules at a glance
//!
//! | Item | Description |
//! |------|-------------|
//! | [`StructuredCallMode`] | Runtime outcome enum (5 variants) |
//! | [`StructuredInput`] | Plain DTO; signals derived from a prior extraction |
//! | [`StructuredThresholds`] | Tunable thresholds with conservative defaults |
//! | [`choose_call_mode`] | Pure decision function |

use serde::{Deserialize, Serialize};

/// Outcome of the structured-extraction call-mode heuristic.
///
/// **Distinct from `crate::core::config::CallMode`** which has three variants
/// and governs extraction-engine behaviour.  This enum governs whether and how
/// an already-extracted document is sent to an LLM structured-extraction
/// pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum StructuredCallMode {
    /// Document is unsupported or not worth invoking the pipeline.
    Skip,
    /// Send extracted text only; no vision model call.
    TextOnly,
    /// Send page rasters only; no extracted text payload.
    VisionOnly,
    /// Fuse extracted text with page rasters in a single multimodal call.
    TextPlusVision,
    /// Try text-only first; escalate to vision on low confidence score.
    TextOnlyWithVisionFallback,
}

/// Signals consumed by the call-mode heuristic.
///
/// All fields derive from a prior kreuzberg extraction — no double-work.
/// This is a plain DTO; it intentionally has no dependency on internal
/// kreuzberg extraction types so it can be constructed from any source.
#[derive(Debug, Clone)]
pub struct StructuredInput {
    /// MIME type, canonicalised to lowercase by the caller.
    pub mime_type: String,
    /// Number of pages in the document.
    pub page_count: u32,
    /// Fraction of pages with a real text layer (0.0..=1.0).
    pub text_coverage: f64,
    /// Average extracted characters per page.
    pub avg_chars_per_page: f64,
    /// Count of embedded images (figures, photos, signatures) discovered.
    pub embedded_image_count: u32,
    /// When `true`, promote the result to at least [`StructuredCallMode::TextPlusVision`].
    pub user_force_vision: bool,
}

/// Thresholds for the structured-extraction call-mode heuristic.
///
/// All defaults are **conservative starting points**.  Deployments should
/// measure their own document corpus and override via their own config;
/// these values are chosen to be safe-by-default, not to be optimal for
/// any particular workload.
///
/// Construct custom thresholds with struct-update syntax:
/// ```rust
/// use kreuzberg::heuristics::StructuredThresholds;
/// let t = StructuredThresholds {
///     enable_vision_fallback: true,
///     ..StructuredThresholds::default()
/// };
/// ```
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct StructuredThresholds {
    /// PDFs with `text_coverage` strictly below this are treated as scanned.
    ///
    /// **Conservative default: 0.10** — deployments override via their own
    /// config after measuring their document corpus.
    pub scan_max_coverage: f64,
    /// PDFs with `text_coverage` at or above this AND zero embedded images
    /// route to [`StructuredCallMode::TextOnly`].
    ///
    /// **Conservative default: 0.90** — deployments override via their own
    /// config after measuring their document corpus.
    pub digital_min_coverage: f64,
    /// DOCX / HTML / text documents with `avg_chars_per_page` above this
    /// route to [`StructuredCallMode::TextOnly`].
    ///
    /// **Conservative default: 200.0** — deployments override via their own
    /// config after measuring their document corpus.
    pub docx_text_min_density: f64,
    /// When `true`, emit [`StructuredCallMode::TextOnlyWithVisionFallback`]
    /// instead of [`StructuredCallMode::TextOnly`] so the orchestrator can
    /// escalate to vision on low confidence.
    ///
    /// **Conservative default: `false`** — must be explicitly enabled per
    /// deployment after bench validation; deployments override via their own
    /// config.
    pub enable_vision_fallback: bool,
}

impl Default for StructuredThresholds {
    fn default() -> Self {
        Self {
            scan_max_coverage: 0.10,
            digital_min_coverage: 0.90,
            docx_text_min_density: 200.0,
            enable_vision_fallback: false,
        }
    }
}

/// Decide which call mode best fits this document.
///
/// Rules applied in order:
///
/// 1. `image/*` → [`StructuredCallMode::VisionOnly`] (no text layer to start from).
/// 2. `application/pdf` → [`StructuredCallMode::TextOnly`] regardless of
///    `text_coverage` or embedded image count.  Kreuzberg's OCR + text-layer
///    extraction produces text for scanned PDFs; the orchestrator's
///    post-call confidence gate handles any vision escalation actually needed.
/// 3. DOCX / `text/html` / `text/*` / `application/json` / `application/xml` /
///    `application/rtf` with `avg_chars_per_page > docx_text_min_density`
///    → [`StructuredCallMode::TextOnly`].
/// 4. Anything else → [`StructuredCallMode::Skip`].
///
/// After rule selection two post-rule promotions apply (in order):
///
/// - `user_force_vision` promotes `TextOnly` → `TextPlusVision`
///   (`Skip` stays `Skip` — caller meant to opt out).
/// - `enable_vision_fallback` promotes `TextOnly` →
///   `TextOnlyWithVisionFallback` (does **not** upgrade `TextPlusVision` or
///   `Skip`).
pub fn choose_call_mode(input: &StructuredInput, t: &StructuredThresholds) -> StructuredCallMode {
    let mime = input.mime_type.to_ascii_lowercase();
    let is_text_mime = mime.starts_with("text/")
        || mime == "application/json"
        || mime == "application/xml"
        || mime == "application/rtf";
    let is_text_bearing = mime == "application/pdf"
        || (is_docx_or_html(&mime) && input.avg_chars_per_page > t.docx_text_min_density)
        || (is_text_mime && input.avg_chars_per_page > t.docx_text_min_density);

    let raw = if mime.starts_with("image/") {
        StructuredCallMode::VisionOnly
    } else if is_text_bearing {
        StructuredCallMode::TextOnly
    } else {
        StructuredCallMode::Skip
    };

    let mode = if input.user_force_vision {
        match raw {
            StructuredCallMode::TextOnly => StructuredCallMode::TextPlusVision,
            other => other,
        }
    } else {
        raw
    };

    // When the thresholds enable it, promote TextOnly → TextOnlyWithVisionFallback
    // so the orchestrator runs the confidence-gated escalation path.  Does NOT
    // upgrade TextPlusVision (user already opted into vision) or Skip.
    if t.enable_vision_fallback && mode == StructuredCallMode::TextOnly {
        StructuredCallMode::TextOnlyWithVisionFallback
    } else {
        mode
    }
}

fn is_docx_or_html(mime: &str) -> bool {
    matches!(
        mime,
        "text/html" | "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t() -> StructuredThresholds {
        StructuredThresholds::default()
    }

    fn input(mime: &str) -> StructuredInput {
        StructuredInput {
            mime_type: mime.into(),
            page_count: 1,
            text_coverage: 0.0,
            avg_chars_per_page: 0.0,
            embedded_image_count: 0,
            user_force_vision: false,
        }
    }

    #[test]
    fn image_mime_chooses_vision_only() {
        assert_eq!(
            choose_call_mode(&input("image/png"), &t()),
            StructuredCallMode::VisionOnly
        );
        assert_eq!(
            choose_call_mode(&input("image/jpeg"), &t()),
            StructuredCallMode::VisionOnly
        );
    }

    #[test]
    fn pdf_low_coverage_chooses_text_only() {
        // Scanned-looking PDFs route to TextOnly — kreuzberg's OCR fills the
        // text layer for us. The orchestrator's confidence gate handles any
        // vision escalation.
        let mut i = input("application/pdf");
        i.text_coverage = 0.05;
        assert_eq!(choose_call_mode(&i, &t()), StructuredCallMode::TextOnly);
    }

    #[test]
    fn pdf_high_coverage_pure_text_chooses_text_only() {
        let mut i = input("application/pdf");
        i.text_coverage = 0.95;
        i.embedded_image_count = 0;
        assert_eq!(choose_call_mode(&i, &t()), StructuredCallMode::TextOnly);
    }

    #[test]
    fn pdf_high_coverage_with_images_chooses_text_only() {
        // Embedded images no longer escalate at the heuristic layer — the
        // orchestrator's TextOnlyWithVisionFallback path drives any vision pass.
        let mut i = input("application/pdf");
        i.text_coverage = 0.95;
        i.embedded_image_count = 3;
        assert_eq!(choose_call_mode(&i, &t()), StructuredCallMode::TextOnly);
    }

    #[test]
    fn pdf_mid_coverage_chooses_text_only() {
        let mut i = input("application/pdf");
        i.text_coverage = 0.5;
        assert_eq!(choose_call_mode(&i, &t()), StructuredCallMode::TextOnly);
    }

    #[test]
    fn docx_dense_text_chooses_text_only() {
        let mut i =
            input("application/vnd.openxmlformats-officedocument.wordprocessingml.document");
        i.avg_chars_per_page = 800.0;
        assert_eq!(choose_call_mode(&i, &t()), StructuredCallMode::TextOnly);
    }

    #[test]
    fn html_dense_text_chooses_text_only() {
        let mut i = input("text/html");
        i.avg_chars_per_page = 500.0;
        assert_eq!(choose_call_mode(&i, &t()), StructuredCallMode::TextOnly);
    }

    #[test]
    fn html_sparse_text_chooses_skip() {
        let mut i = input("text/html");
        i.avg_chars_per_page = 50.0;
        assert_eq!(choose_call_mode(&i, &t()), StructuredCallMode::Skip);
    }

    #[test]
    fn text_plain_dense_chooses_text_only() {
        let mut i = input("text/plain");
        i.avg_chars_per_page = 500.0;
        assert_eq!(choose_call_mode(&i, &t()), StructuredCallMode::TextOnly);
    }

    #[test]
    fn text_plain_sparse_chooses_skip() {
        let mut i = input("text/plain");
        i.avg_chars_per_page = 50.0;
        assert_eq!(choose_call_mode(&i, &t()), StructuredCallMode::Skip);
    }

    #[test]
    fn text_csv_dense_chooses_text_only() {
        let mut i = input("text/csv");
        i.avg_chars_per_page = 500.0;
        assert_eq!(choose_call_mode(&i, &t()), StructuredCallMode::TextOnly);
    }

    #[test]
    fn application_json_dense_chooses_text_only() {
        let mut i = input("application/json");
        i.avg_chars_per_page = 500.0;
        assert_eq!(choose_call_mode(&i, &t()), StructuredCallMode::TextOnly);
    }

    #[test]
    fn application_xml_dense_chooses_text_only() {
        let mut i = input("application/xml");
        i.avg_chars_per_page = 500.0;
        assert_eq!(choose_call_mode(&i, &t()), StructuredCallMode::TextOnly);
    }

    #[test]
    fn application_rtf_sparse_chooses_skip() {
        let mut i = input("application/rtf");
        i.avg_chars_per_page = 50.0;
        assert_eq!(choose_call_mode(&i, &t()), StructuredCallMode::Skip);
    }

    #[test]
    fn unsupported_mime_chooses_skip() {
        assert_eq!(
            choose_call_mode(&input("application/octet-stream"), &t()),
            StructuredCallMode::Skip
        );
    }

    #[test]
    fn user_force_vision_promotes_text_only_to_text_plus_vision() {
        let mut i = input("application/pdf");
        i.text_coverage = 0.95;
        i.user_force_vision = true;
        assert_eq!(
            choose_call_mode(&i, &t()),
            StructuredCallMode::TextPlusVision
        );
    }

    #[test]
    fn user_force_vision_does_not_promote_skip() {
        let mut i = input("application/octet-stream");
        i.user_force_vision = true;
        assert_eq!(choose_call_mode(&i, &t()), StructuredCallMode::Skip);
    }

    #[test]
    fn case_insensitive_mime_match() {
        assert_eq!(
            choose_call_mode(&input("IMAGE/PNG"), &t()),
            StructuredCallMode::VisionOnly
        );
    }

    #[test]
    fn enable_vision_fallback_promotes_text_only_to_fallback() {
        let mut i = input("application/pdf");
        i.text_coverage = 0.95;
        let thresholds = StructuredThresholds {
            enable_vision_fallback: true,
            ..StructuredThresholds::default()
        };
        assert_eq!(
            choose_call_mode(&i, &thresholds),
            StructuredCallMode::TextOnlyWithVisionFallback
        );
    }

    #[test]
    fn enable_vision_fallback_does_not_upgrade_text_plus_vision() {
        // user_force_vision wins first, producing TextPlusVision;
        // enable_vision_fallback must not demote it back to TextOnlyWithVisionFallback.
        let mut i = input("application/pdf");
        i.user_force_vision = true;
        let thresholds = StructuredThresholds {
            enable_vision_fallback: true,
            ..StructuredThresholds::default()
        };
        assert_eq!(
            choose_call_mode(&i, &thresholds),
            StructuredCallMode::TextPlusVision
        );
    }

    #[test]
    fn serde_round_trip_all_variants() {
        let variants = [
            StructuredCallMode::Skip,
            StructuredCallMode::TextOnly,
            StructuredCallMode::VisionOnly,
            StructuredCallMode::TextPlusVision,
            StructuredCallMode::TextOnlyWithVisionFallback,
        ];
        for variant in variants {
            let json = serde_json::to_string(&variant).expect("serialize");
            let decoded: StructuredCallMode = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(decoded, variant, "round-trip failed for {:?}", variant);
        }
    }

    #[test]
    fn serde_uses_snake_case_names() {
        assert_eq!(
            serde_json::to_string(&StructuredCallMode::Skip).unwrap(),
            r#""skip""#
        );
        assert_eq!(
            serde_json::to_string(&StructuredCallMode::TextOnly).unwrap(),
            r#""text_only""#
        );
        assert_eq!(
            serde_json::to_string(&StructuredCallMode::VisionOnly).unwrap(),
            r#""vision_only""#
        );
        assert_eq!(
            serde_json::to_string(&StructuredCallMode::TextPlusVision).unwrap(),
            r#""text_plus_vision""#
        );
        assert_eq!(
            serde_json::to_string(&StructuredCallMode::TextOnlyWithVisionFallback).unwrap(),
            r#""text_only_with_vision_fallback""#
        );
    }
}
