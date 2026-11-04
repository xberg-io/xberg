//! Cached orientation recovery for low-confidence implicit PDF OCR.

use std::sync::Arc;

use crate::cancellation::CancellationToken;
use crate::core::config::{ExtractionConfig, OcrConfig, OcrQualityThresholds};
use crate::doc_orientation::detector::tesseract_detector;
use crate::extractors::security::SecurityBudget;
use crate::plugins::{ConfidenceSemantics, OcrBackend};
use crate::types::ExtractedDocument;

use super::document::{processed_ocr_layout_dimensions, resolved_ocr_correction_degrees};
use super::scoring::{OcrRecognitionNoiseDecision, mean_text_conf_of, ocr_recognition_noise_decision, word_count_of};

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
        if result.content.trim().is_empty()
            || !result.formulas.is_empty()
            || !resolved_ocr_correction_degrees(&result.metadata).is_some_and(|angle| angle != 0)
        {
            return false;
        }
        let table_text;
        let text = if result.tables.is_empty() {
            &result.content
        } else {
            let Some(text) = complete_table_text(result) else {
                return false;
            };
            table_text = text;
            &table_text
        };
        let mean_confidence = mean_text_conf_of(&result.metadata.additional);
        let noise = ocr_recognition_noise_decision(text, &self.thresholds, confidence, mean_confidence);
        !noise.low_confidence && !noise.fragmented_noise && mean_confidence.is_some()
    }
}

fn complete_table_text(result: &ExtractedDocument) -> Option<String> {
    const MIN_ROWS: usize = 3;
    const MIN_COLUMNS: usize = 2;
    let [table] = result.tables.as_slice() else {
        return None;
    };
    let first = table.cells.first()?;
    let (width, height) = processed_ocr_layout_dimensions(&result.metadata)?;
    let bbox = table.bounding_box?;
    if !(0.0 <= bbox.x0
        && bbox.x0 < bbox.x1
        && bbox.x1 <= f64::from(width)
        && 0.0 <= bbox.y0
        && bbox.y0 < bbox.y1
        && bbox.y1 <= f64::from(height))
        || table.cells.len() < MIN_ROWS
        || first.len() < MIN_COLUMNS
        || !table
            .cells
            .iter()
            .all(|row| row.len() == first.len() && row.iter().all(|cell| !cell.trim().is_empty()))
    {
        return None;
    }
    let words = table.cells.iter().flatten().flat_map(|cell| cell.split_whitespace());
    if usize::try_from(word_count_of(&result.metadata.additional)?).ok()? != words.clone().count()
        || (table.markdown.trim() != result.content.trim() && !words.clone().eq(result.content.split_whitespace()))
    {
        return None;
    }
    Some(words.collect::<Vec<_>>().join(" "))
}

pub(super) fn scale_recovered_table_to_raster(result: &mut ExtractedDocument, raster: (u32, u32)) {
    let Some((width, height)) = processed_ocr_layout_dimensions(&result.metadata) else {
        return;
    };
    let [table] = result.tables.as_mut_slice() else {
        return;
    };
    if let Some(bbox) = table.bounding_box.as_mut() {
        let scale_x = f64::from(raster.0) / f64::from(width);
        let scale_y = f64::from(raster.1) / f64::from(height);
        bbox.x0 *= scale_x;
        bbox.x1 *= scale_x;
        bbox.y0 *= scale_y;
        bbox.y1 *= scale_y;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ocr_metadata_keys::{OCR_AUTO_ROTATED_METADATA_KEY, OCR_ORIENTATION_DEGREES_METADATA_KEY};

    #[test]
    fn recovery_requires_proved_rotation_and_rejects_unverified_tables() {
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

    #[test]
    fn recovery_accepts_only_one_complete_rectangular_table_with_exact_ordered_text() {
        let retry = OrientationRetry::new(&ExtractionConfig::default(), OcrQualityThresholds::default());
        let confidence = ConfidenceSemantics::Legibility { scale_max: 100.0 };
        let mut candidate = ExtractedDocument {
            content: "Cash 100 31,290 Rent 107 77,078 Fuel 114 71,433".into(),
            tables: vec![crate::types::Table {
                cells: vec![
                    vec!["Cash 100".into(), "31,290".into()],
                    vec!["Rent 107".into(), "77,078".into()],
                    vec!["Fuel 114".into(), "71,433".into()],
                ],
                bounding_box: Some(crate::types::BoundingBox {
                    x0: 20.0,
                    y0: 40.0,
                    x1: 180.0,
                    y1: 160.0,
                }),
                ..Default::default()
            }],
            ..Default::default()
        };
        let metadata = &mut candidate.metadata.additional;
        metadata.insert(
            crate::ocr_metadata_keys::OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY.into(),
            200.into(),
        );
        metadata.insert(
            crate::ocr_metadata_keys::OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY.into(),
            200.into(),
        );
        metadata.insert("word_count".into(), 9.into());
        metadata.insert("mean_text_conf".into(), 94.into());
        metadata.insert(OCR_AUTO_ROTATED_METADATA_KEY.into(), true.into());
        metadata.insert(OCR_ORIENTATION_DEGREES_METADATA_KEY.into(), 270.into());
        assert!(retry.accepts(&candidate, confidence));

        let mut markdown_table = candidate.clone();
        markdown_table.tables[0].markdown =
            "| Cash 100 | 31,290 |\n| --- | --- |\n| Rent 107 | 77,078 |\n| Fuel 114 | 71,433 |".into();
        markdown_table.content = markdown_table.tables[0].markdown.clone();
        assert!(retry.accepts(&markdown_table, confidence));
        markdown_table.content.push_str("\nUnclaimed words");
        assert!(!retry.accepts(&markdown_table, confidence));

        let mut mismatched_text = candidate.clone();
        mismatched_text.content = "Rent 107 77,078 Cash 100 31,290 Fuel 114 71,433".into();
        assert!(!retry.accepts(&mismatched_text, confidence));

        let mut sparse_table = candidate.clone();
        sparse_table.tables[0].cells[1][1].clear();
        sparse_table.content = "Cash 100 31,290 Rent 107 Fuel 114 71,433".into();
        assert!(!retry.accepts(&sparse_table, confidence));

        let mut duplicate = candidate.clone();
        duplicate.tables.push(candidate.tables[0].clone());
        assert!(!retry.accepts(&duplicate, confidence));

        let mut off_page = candidate.clone();
        off_page.tables[0].bounding_box.as_mut().unwrap().y1 = 201.0;
        assert!(!retry.accepts(&off_page, confidence));

        scale_recovered_table_to_raster(&mut candidate, (100, 50));
        let bbox = candidate.tables[0].bounding_box.unwrap();
        assert_eq!((bbox.x0, bbox.y0, bbox.x1, bbox.y1), (10.0, 10.0, 90.0, 40.0));
    }
}
