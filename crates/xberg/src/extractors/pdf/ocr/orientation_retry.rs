//! Cached orientation recovery for low-confidence implicit PDF OCR.

use std::sync::Arc;

use crate::cancellation::CancellationToken;
use crate::core::config::{ExtractionConfig, OcrConfig, OcrQualityThresholds};
use crate::doc_orientation::detector::tesseract_detector;
use crate::extractors::security::SecurityBudget;
use crate::plugins::{ConfidenceSemantics, OcrBackend};
use crate::types::ExtractedDocument;

use super::document::resolved_ocr_correction_degrees;
use super::scoring::{OcrRecognitionNoiseDecision, mean_text_conf_of, ocr_recognition_noise_decision};

pub(super) struct OrientationRetry {
    thresholds: OcrQualityThresholds,
    attempts: parking_lot::Mutex<SecurityBudget>,
    cached_model_ready: tokio::sync::OnceCell<bool>,
}

impl OrientationRetry {
    pub(super) fn new(config: &ExtractionConfig, thresholds: OcrQualityThresholds) -> Self {
        Self {
            thresholds,
            attempts: parking_lot::Mutex::new(SecurityBudget::from_config(config)),
            cached_model_ready: tokio::sync::OnceCell::new(),
        }
    }

    fn noise(&self, result: &ExtractedDocument, confidence: ConfidenceSemantics) -> OcrRecognitionNoiseDecision {
        ocr_recognition_noise_decision(
            &result.content,
            &self.thresholds,
            confidence,
            mean_text_conf_of(&result.metadata.additional),
        )
    }

    fn has_text_only(result: &ExtractedDocument) -> bool {
        !result.content.trim().is_empty() && result.tables.is_empty() && result.formulas.is_empty()
    }

    pub(super) async fn after_ocr(
        &self,
        backend: &Arc<dyn OcrBackend>,
        raster: Arc<Vec<u8>>,
        config: &OcrConfig,
        original: crate::Result<ExtractedDocument>,
        cancel: Option<&CancellationToken>,
    ) -> crate::Result<ExtractedDocument> {
        let original = original?;
        let confidence = backend.confidence_semantics();
        if !Self::has_text_only(&original)
            || !self.noise(&original, confidence).low_confidence
            || self.attempts.lock().step().is_err()
        {
            return Ok(original);
        }
        let cancelled = || cancel.is_some_and(CancellationToken::is_cancelled);
        if cancelled() {
            return Err(crate::XbergError::Cancelled);
        }
        let ready = self
            .cached_model_ready
            .get_or_init(|| async {
                tokio::task::spawn_blocking(|| tesseract_detector().prepare_cached())
                    .await
                    .unwrap_or(false)
            })
            .await;
        if !ready {
            return Ok(original);
        }
        if cancelled() {
            return Err(crate::XbergError::Cancelled);
        }

        let mut rotated_config = config.clone();
        rotated_config.auto_rotate = true;
        let retry = backend.process_image_owned(raster, &rotated_config).await;
        if cancelled() {
            return Err(crate::XbergError::Cancelled);
        }
        match retry {
            Ok(candidate) if self.accepts(&candidate, confidence) => Ok(candidate),
            Err(crate::XbergError::Cancelled) => Err(crate::XbergError::Cancelled),
            _ => Ok(original),
        }
    }

    fn accepts(&self, result: &ExtractedDocument, confidence: ConfidenceSemantics) -> bool {
        if !Self::has_text_only(result)
            || !resolved_ocr_correction_degrees(&result.metadata).is_some_and(|angle| angle != 0)
        {
            return false;
        }
        let noise = self.noise(result, confidence);
        !noise.low_confidence && !noise.fragmented_noise && mean_text_conf_of(&result.metadata.additional).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ocr_metadata_keys::{OCR_AUTO_ROTATED_METADATA_KEY, OCR_ORIENTATION_DEGREES_METADATA_KEY};

    #[test]
    fn recovery_requires_proved_rotation_confident_text_and_no_tables() {
        let retry = OrientationRetry::new(&ExtractionConfig::default(), OcrQualityThresholds::default());
        let confidence = ConfidenceSemantics::Legibility { scale_max: 100.0 };
        let mut candidate = ExtractedDocument {
            content: "A recovered page of ordinary text".to_string(),
            ..Default::default()
        };
        let metadata = &mut candidate.metadata.additional;
        metadata.insert("mean_text_conf".into(), 94.into());
        metadata.insert(OCR_AUTO_ROTATED_METADATA_KEY.into(), true.into());
        metadata.insert(OCR_ORIENTATION_DEGREES_METADATA_KEY.into(), 0.into());
        assert!(!retry.accepts(&candidate, confidence));

        candidate
            .metadata
            .additional
            .insert(OCR_ORIENTATION_DEGREES_METADATA_KEY.into(), 270.into());
        assert!(retry.accepts(&candidate, confidence));

        candidate.metadata.additional.insert("mean_text_conf".into(), 67.into());
        assert!(!retry.accepts(&candidate, confidence));
        candidate.metadata.additional.insert("mean_text_conf".into(), 94.into());
        candidate.tables.push(crate::types::Table::default());
        assert!(!retry.accepts(&candidate, confidence));
    }
}
