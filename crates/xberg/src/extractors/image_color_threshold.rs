//! Retry standalone images whose colored backgrounds may hide text from global Otsu.

use super::{merge_primary_warnings_into_selected, usable_word_confidences};
use crate::core::config::OcrConfig;
use crate::extractors::security::SecurityLimits;
use crate::types::{ExtractedDocument, OcrElementLevel};
use std::collections::HashMap;
use std::sync::Arc;

const MAX_COLOR_PROBE_BYTES: usize = 8 * 1024 * 1024;
const MAX_PRIMARY_WORDS: usize = 80;
const CONFIDENT_WORD: f64 = 0.70;

fn has_colored_background(bytes: &[u8], limits: &SecurityLimits) -> bool {
    let limits = SecurityLimits {
        max_content_size: limits.max_content_size.min(MAX_COLOR_PROBE_BYTES),
        ..limits.clone()
    };
    let Ok(rgb) = crate::extraction::image_decode::decode_standard_rgb8_with_security_limits(bytes, &limits) else {
        return false;
    };
    let (mut colored, mut light, mut total) = (0, 0, 0);
    for pixel in rgb.pixels().step_by(17) {
        let [red, green, blue] = pixel.0;
        let high = red.max(green).max(blue);
        let low = red.min(green).min(blue);
        colored += usize::from(low >= 100 && high - low >= 30);
        light += usize::from(low >= 240);
        total += 1;
    }
    total != 0 && colored * 100 >= total * 2 && light * 4 >= total
}

fn confident_words(result: &ExtractedDocument) -> HashMap<String, usize> {
    let mut words = HashMap::new();
    for element in result.ocr_elements.iter().flatten() {
        if element.level == OcrElementLevel::Word && element.confidence.recognition >= CONFIDENT_WORD {
            let word = element.text.trim_matches(|ch: char| !ch.is_alphanumeric());
            if !word.is_empty() {
                *words.entry(word.to_lowercase()).or_default() += 1;
            }
        }
    }
    words
}

fn improves(primary: &ExtractedDocument, candidate: &ExtractedDocument) -> bool {
    let primary = confident_words(primary);
    let candidate = confident_words(candidate);
    let before: usize = primary.values().sum();
    let after: usize = candidate.values().sum();
    let retained: usize = primary
        .iter()
        .map(|(word, count)| (*count).min(candidate.get(word).copied().unwrap_or(0)))
        .sum();
    after >= before + (before / 4).max(4) && retained * 4 >= before * 3
}

pub(super) async fn retry(
    backend: &dyn crate::plugins::OcrBackend,
    bytes: Arc<Vec<u8>>,
    config: &OcrConfig,
    limits: &SecurityLimits,
    primary: ExtractedDocument,
) -> crate::Result<ExtractedDocument> {
    if usable_word_confidences(&primary).len() > MAX_PRIMARY_WORDS || !has_colored_background(&bytes, limits) {
        return Ok(primary);
    }
    let mut config = config.clone();
    config
        .tesseract_config
        .get_or_insert_with(Default::default)
        .thresholding_method = 2;
    match backend.process_image_owned(bytes, &config).await {
        Ok(mut candidate) if improves(&primary, &candidate) => {
            merge_primary_warnings_into_selected(&primary, &mut candidate);
            Ok(candidate)
        }
        Err(error @ crate::XbergError::Cancelled) => Err(error),
        Err(error) => {
            tracing::warn!(%error, "Colored image OCR retry failed");
            Ok(primary)
        }
        Ok(_) => Ok(primary),
    }
}
