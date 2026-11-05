//! Cached orientation recovery for low-confidence implicit PDF OCR.

use std::sync::Arc;

use crate::cancellation::CancellationToken;
use crate::core::config::{ExtractionConfig, OcrConfig, OcrQualityThresholds};
use crate::doc_orientation::detector::tesseract_detector;
use crate::extractors::security::{SecurityBudget, SecurityLimits};
use crate::plugins::{ConfidenceSemantics, OcrBackend};
use crate::table_core::{HocrWord, detect_rows, find_row_index};
use crate::types::{BoundingBox, ExtractedDocument, OcrElement, OcrElementConfig, OcrElementLevel};

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

    async fn prepare_retry(
        &self,
        cancel: Option<&CancellationToken>,
        probe: impl std::future::Future<Output = bool>,
    ) -> crate::Result<bool> {
        check_cancelled(cancel)?;
        if self.attempts.lock().step().is_err() {
            return Ok(false);
        }
        let ready = self
            .cached_model_ready
            .get_or_init(|| async {
                tokio::task::spawn_blocking(|| tesseract_detector().prepare_cached())
                    .await
                    .unwrap_or(false)
            })
            .await;
        check_cancelled(cancel)?;
        if !ready {
            return Ok(false);
        }
        let has_evidence = probe.await;
        check_cancelled(cancel)?;
        Ok(has_evidence)
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
        let empty = original.content.trim().is_empty();
        if !original.tables.is_empty()
            || !original.formulas.is_empty()
            || (!empty && !self.noise(&original, confidence).low_confidence)
        {
            return Ok(original);
        }
        let probe = async {
            if !empty {
                return true;
            }
            let raster = Arc::clone(&raster);
            let limits = config.security_limits.clone().unwrap_or_default();
            tokio::task::spawn_blocking(move || raster_has_glyphs(&raster, &limits))
                .await
                .unwrap_or(false)
        };
        if !self.prepare_retry(cancel, probe).await? {
            return Ok(original);
        }

        let mut rotated_config = config.clone();
        rotated_config.auto_rotate = true;
        rotated_config.element_config = Some(OcrElementConfig {
            include_elements: true,
            min_level: OcrElementLevel::Word,
            ..Default::default()
        });
        let row_threshold = rotated_config.tesseract_config.as_ref().map_or_else(
            || crate::types::TesseractConfig::default().table_row_threshold_ratio,
            |config| config.table_row_threshold_ratio,
        );
        let retry = backend.process_image_owned(raster, &rotated_config).await;
        check_cancelled(cancel)?;
        match retry {
            Ok(mut candidate) if self.accepts(&candidate, confidence, row_threshold) => {
                let evidence = candidate.ocr_elements.take().unwrap_or_default();
                candidate.ocr_elements = config
                    .element_config
                    .as_ref()
                    .map(|options| options.select_elements(&evidence))
                    .filter(|elements| !elements.is_empty());
                Ok(candidate)
            }
            Err(crate::XbergError::Cancelled) => Err(crate::XbergError::Cancelled),
            _ => Ok(original),
        }
    }

    fn accepts(&self, result: &ExtractedDocument, confidence: ConfidenceSemantics, row_threshold: f64) -> bool {
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
            let Some(text) = complete_table_text(result, row_threshold) else {
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

fn check_cancelled(cancel: Option<&CancellationToken>) -> crate::Result<()> {
    if cancel.is_some_and(CancellationToken::is_cancelled) {
        return Err(crate::XbergError::Cancelled);
    }
    Ok(())
}

const GLYPH_SAMPLE_EDGE: u32 = 512;
// RGB thumbnail, luminance and foreground masks, and spare capacity for the glyph estimator's
// flood-fill stack, component heights and letter bodies. At most 20% of pixels are visited
// by the flood fill, so six usize slots per sample cover all three vectors and reallocations. ~keep
const GLYPH_PROBE_WORKSPACE: usize =
    GLYPH_SAMPLE_EDGE as usize * GLYPH_SAMPLE_EDGE as usize * (5 + 6 * size_of::<usize>());

fn raster_has_glyphs(raster: &[u8], limits: &SecurityLimits) -> bool {
    let Ok(rgb) = crate::extraction::image_decode::decode_standard_rgb8_with_additional_live_bytes_and_security_limits(
        raster,
        limits,
        GLYPH_PROBE_WORKSPACE as u64,
    ) else {
        return false;
    };
    let (width, height) = rgb.dimensions();
    let longest = width.max(height);
    let sample = if longest > GLYPH_SAMPLE_EDGE {
        let shrink = |length| ((u64::from(length) * u64::from(GLYPH_SAMPLE_EDGE)) / u64::from(longest)).max(1) as u32;
        image::imageops::thumbnail(&rgb, shrink(width), shrink(height))
    } else {
        rgb
    };
    crate::image::text_height::median_glyph_height(sample.as_raw(), sample.width() as usize, sample.height() as usize)
        .is_some()
}

fn complete_table_text(result: &ExtractedDocument, row_threshold: f64) -> Option<String> {
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
    let evidence = result.ocr_elements.as_deref()?;
    if !table_matches_iterator_rows(&table.cells, evidence, bbox, row_threshold) {
        return None;
    }
    Some(words.collect::<Vec<_>>().join(" "))
}

fn iterator_word(element: &OcrElement, bbox: BoundingBox) -> Option<HocrWord> {
    if element.backend_metadata.get("backend")?.as_str()? != "tesseract-iterator" {
        return None;
    }
    let (left, top, width, height) = element.geometry.to_aabb();
    let center_x = f64::from(left) + f64::from(width) / 2.0;
    let center_y = f64::from(top) + f64::from(height) / 2.0;
    (width > 0 && height > 0 && (bbox.x0..=bbox.x1).contains(&center_x) && (bbox.y0..=bbox.y1).contains(&center_y))
        .then(|| HocrWord {
            text: element.text.clone(),
            left,
            top,
            width,
            height,
            confidence: element.confidence.recognition,
        })
}

fn table_matches_iterator_rows(
    cells: &[Vec<String>],
    elements: &[OcrElement],
    bbox: BoundingBox,
    row_threshold: f64,
) -> bool {
    // Independently order the raw words by geometry. Tesseract's iterator may emit entire columns first. ~keep
    let Some(words) = elements
        .iter()
        .filter(|element| element.level == OcrElementLevel::Word)
        .map(|element| iterator_word(element, bbox))
        .collect::<Option<Vec<_>>>()
    else {
        return false;
    };
    let row_positions = detect_rows(&words, row_threshold);
    if row_positions.len() != cells.len() {
        return false;
    }
    let mut rows = vec![Vec::new(); row_positions.len()];
    for word in &words {
        let Some(row) = find_row_index(&row_positions, word) else {
            return false;
        };
        rows[row].push(word);
    }
    rows.into_iter().zip(cells).all(|(mut raw, cells)| {
        raw.sort_unstable_by_key(|word| word.left);
        raw.windows(2).all(|pair| pair[0].left < pair[1].left)
            && raw
                .iter()
                .flat_map(|word| word.text.split_whitespace())
                .eq(cells.iter().flat_map(|cell| cell.split_whitespace()))
    })
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

    #[tokio::test]
    async fn cancelled_retry_does_not_probe() {
        let retry = OrientationRetry::new(&ExtractionConfig::default(), OcrQualityThresholds::default());
        let cancel = CancellationToken::new();
        cancel.cancel();
        assert!(matches!(
            retry
                .prepare_retry(Some(&cancel), async { panic!("cancelled retry probed") })
                .await,
            Err(crate::XbergError::Cancelled)
        ));
        assert!(retry.cached_model_ready.get().is_none());
    }

    #[tokio::test]
    async fn exhausted_retry_does_not_probe() {
        let limits = SecurityLimits {
            max_iterations: 0,
            ..Default::default()
        };
        let exhausted = OrientationRetry::new(
            &ExtractionConfig {
                security_limits: Some(limits),
                ..Default::default()
            },
            OcrQualityThresholds::default(),
        );
        assert!(
            !exhausted
                .prepare_retry(None, async { panic!("exhausted retry probed") })
                .await
                .unwrap()
        );
        assert!(exhausted.cached_model_ready.get().is_none());
    }

    #[tokio::test]
    async fn cold_retry_does_not_probe() {
        let retry = OrientationRetry::new(&ExtractionConfig::default(), OcrQualityThresholds::default());
        retry
            .cached_model_ready
            .set(false)
            .expect("model cache starts uninitialized");
        assert!(
            !retry
                .prepare_retry(None, async { panic!("cold retry probed") })
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn cancellation_during_the_probe_prevents_retry() {
        let retry = OrientationRetry::new(&ExtractionConfig::default(), OcrQualityThresholds::default());
        retry
            .cached_model_ready
            .set(true)
            .expect("model cache starts uninitialized");
        let cancel = CancellationToken::new();
        let result = retry
            .prepare_retry(Some(&cancel), async {
                cancel.cancel();
                true
            })
            .await;
        assert!(matches!(result, Err(crate::XbergError::Cancelled)));
    }

    #[test]
    fn empty_read_requires_glyphs_not_blank_or_ruled_paper() {
        let limits = SecurityLimits::default();
        let mut raster = image::RgbImage::from_pixel(120, 120, image::Rgb([255; 3]));
        let has_glyphs = |raster: &image::RgbImage| {
            let mut bytes = std::io::Cursor::new(Vec::new());
            raster
                .write_to(&mut bytes, image::ImageFormat::Png)
                .expect("PNG encodes");
            raster_has_glyphs(bytes.get_ref(), &limits)
        };
        assert!(!has_glyphs(&raster));

        for line in (12..120).step_by(12) {
            for x in 8..112 {
                raster.put_pixel(x, line, image::Rgb([0; 3]));
                raster.put_pixel(x, line + 1, image::Rgb([0; 3]));
            }
        }
        assert!(!has_glyphs(&raster));
        assert!(!has_glyphs(&image::imageops::rotate90(&raster)));

        for left in (12..100).step_by(12) {
            for y in 5..10 {
                for x in left..left + 3 {
                    raster.put_pixel(x, y, image::Rgb([0; 3]));
                }
            }
        }
        assert!(has_glyphs(&raster));
    }

    #[test]
    fn empty_raster_probe_accounts_for_workspace_beyond_decoded_rgb() {
        let mut raster = image::RgbImage::from_pixel(120, 120, image::Rgb([255; 3]));
        for left in (12..100).step_by(12) {
            for y in 5..10 {
                for x in left..left + 3 {
                    raster.put_pixel(x, y, image::Rgb([0; 3]));
                }
            }
        }
        let mut encoded = std::io::Cursor::new(Vec::new());
        raster
            .write_to(&mut encoded, image::ImageFormat::Png)
            .expect("PNG encodes");
        let bytes = encoded.into_inner();
        let required = bytes.len() + raster.as_raw().len() + GLYPH_PROBE_WORKSPACE;
        let mut limits = SecurityLimits {
            max_content_size: required,
            ..Default::default()
        };
        assert!(raster_has_glyphs(&bytes, &limits));
        limits.max_content_size -= 1;
        assert!(!raster_has_glyphs(&bytes, &limits));
    }

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
        assert!(!retry.accepts(&candidate, confidence, 0.5));

        candidate
            .metadata
            .additional
            .insert(OCR_ORIENTATION_DEGREES_METADATA_KEY.into(), 270.into());
        assert!(retry.accepts(&candidate, confidence, 0.5));

        candidate.metadata.additional.insert("mean_text_conf".into(), 67.into());
        assert!(!retry.accepts(&candidate, confidence, 0.5));
        candidate.metadata.additional.insert("mean_text_conf".into(), 94.into());
        candidate.tables.push(crate::types::Table::default());
        assert!(!retry.accepts(&candidate, confidence, 0.5));
    }

    fn table_iterator_evidence() -> Vec<OcrElement> {
        let mut evidence = Vec::new();
        for (row, text) in [
            ["Cash", "100", "31,290"],
            ["Rent", "107", "77,078"],
            ["Fuel", "114", "71,433"],
        ]
        .into_iter()
        .enumerate()
        {
            for (column, token) in text.into_iter().enumerate() {
                evidence.push(OcrElement {
                    text: token.into(),
                    level: OcrElementLevel::Word,
                    geometry: crate::types::OcrBoundingGeometry::Rectangle {
                        left: 30 + 25 * column as u32,
                        top: 50 + 30 * row as u32,
                        width: 10,
                        height: 10,
                    },
                    backend_metadata: std::collections::HashMap::from([(
                        "backend".into(),
                        "tesseract-iterator".into(),
                    )]),
                    ..Default::default()
                });
            }
        }
        evidence.reverse();
        evidence
    }

    fn table_candidate() -> ExtractedDocument {
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
        candidate.ocr_elements = Some(table_iterator_evidence());
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
        candidate
    }

    #[test]
    fn recovery_requires_exact_ordered_iterator_tokens_for_flat_or_markdown_tables() {
        let retry = OrientationRetry::new(&ExtractionConfig::default(), OcrQualityThresholds::default());
        let confidence = ConfidenceSemantics::Legibility { scale_max: 100.0 };
        let candidate = table_candidate();
        assert!(retry.accepts(&candidate, confidence, 0.5));

        let mut markdown_table = candidate.clone();
        markdown_table.tables[0].markdown =
            "| Cash 100 | 31,290 |\n| --- | --- |\n| Rent 107 | 77,078 |\n| Fuel 114 | 71,433 |".into();
        markdown_table.content = markdown_table.tables[0].markdown.clone();
        assert!(retry.accepts(&markdown_table, confidence, 0.5));

        let mut substituted = markdown_table.clone();
        substituted.tables[0].cells[0][1] = "31,299".into();
        substituted.tables[0].markdown = substituted.tables[0].markdown.replace("31,290", "31,299");
        substituted.content = substituted.tables[0].markdown.clone();
        assert!(
            !retry.accepts(&substituted, confidence, 0.5),
            "rendered markup cannot vouch for its own cells"
        );

        markdown_table.content.push_str("\nUnclaimed words");
        assert!(!retry.accepts(&markdown_table, confidence, 0.5));

        let mut mismatched_text = candidate.clone();
        mismatched_text.content = "Rent 107 77,078 Cash 100 31,290 Fuel 114 71,433".into();
        assert!(!retry.accepts(&mismatched_text, confidence, 0.5));
    }

    #[test]
    fn recovery_rejects_sparse_duplicate_and_off_page_tables() {
        let retry = OrientationRetry::new(&ExtractionConfig::default(), OcrQualityThresholds::default());
        let confidence = ConfidenceSemantics::Legibility { scale_max: 100.0 };
        let mut candidate = table_candidate();
        let mut sparse_table = candidate.clone();
        sparse_table.tables[0].cells[1][1].clear();
        sparse_table.content = "Cash 100 31,290 Rent 107 Fuel 114 71,433".into();
        assert!(!retry.accepts(&sparse_table, confidence, 0.5));

        let mut duplicate = candidate.clone();
        duplicate.tables.push(candidate.tables[0].clone());
        assert!(!retry.accepts(&duplicate, confidence, 0.5));

        let mut off_page = candidate.clone();
        off_page.tables[0].bounding_box.as_mut().unwrap().y1 = 201.0;
        assert!(!retry.accepts(&off_page, confidence, 0.5));

        scale_recovered_table_to_raster(&mut candidate, (100, 50));
        let bbox = candidate.tables[0].bounding_box.unwrap();
        assert_eq!((bbox.x0, bbox.y0, bbox.x1, bbox.y1), (10.0, 10.0, 90.0, 40.0));
    }
}
