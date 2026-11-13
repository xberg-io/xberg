//! Retry standalone images whose colored backgrounds may hide text from global Otsu.

use super::{merge_primary_warnings_into_selected, usable_word_confidences};
use crate::cancellation::CancellationToken;
use crate::core::config::OcrConfig;
use crate::extractors::security::SecurityLimits;
use crate::plugins::OcrBackend;
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
    backend: Option<&crate::ocr::tesseract_backend::TesseractBackend>,
    bytes: Arc<Vec<u8>>,
    mime_type: &str,
    (original, effective): (&OcrConfig, &OcrConfig),
    cancel: Option<&CancellationToken>,
    primary: ExtractedDocument,
) -> crate::Result<ExtractedDocument> {
    let Some(backend) = backend.filter(|_| mime_type == "image/png" && original.tesseract_config.is_none()) else {
        return Ok(primary);
    };
    let defaults = SecurityLimits::default();
    let limits = effective.security_limits.as_ref().unwrap_or(&defaults);
    retry_if_colored(
        cancel,
        || has_colored_background(&bytes, limits),
        || async {
            let mut config = effective.clone();
            config
                .tesseract_config
                .get_or_insert_with(Default::default)
                .thresholding_method = 2;
            backend.process_image_owned(Arc::clone(&bytes), &config).await
        },
        primary,
    )
    .await
}

fn check_cancelled(cancel: Option<&CancellationToken>) -> crate::Result<()> {
    if cancel.is_some_and(CancellationToken::is_cancelled) {
        return Err(crate::XbergError::Cancelled);
    }
    Ok(())
}

async fn retry_if_colored(
    cancel: Option<&CancellationToken>,
    probe: impl FnOnce() -> bool,
    read: impl AsyncFnOnce() -> crate::Result<ExtractedDocument>,
    primary: ExtractedDocument,
) -> crate::Result<ExtractedDocument> {
    check_cancelled(cancel)?;
    if usable_word_confidences(&primary).len() > MAX_PRIMARY_WORDS {
        return Ok(primary);
    }
    let colored = probe();
    check_cancelled(cancel)?;
    if !colored {
        return Ok(primary);
    }
    let result = read().await;
    check_cancelled(cancel)?;
    match result {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{OcrBoundingGeometry, OcrConfidence, OcrElement};

    fn document(text: &str) -> ExtractedDocument {
        ExtractedDocument {
            content: text.into(),
            ocr_elements: Some(
                text.split_whitespace()
                    .map(|word| OcrElement {
                        text: word.into(),
                        geometry: OcrBoundingGeometry::Rectangle {
                            left: 0,
                            top: 0,
                            width: 10,
                            height: 10,
                        },
                        confidence: OcrConfidence {
                            detection: None,
                            recognition: 0.95,
                        },
                        level: OcrElementLevel::Word,
                        rotation: None,
                        page_number: 1,
                        parent_id: None,
                        backend_metadata: Default::default(),
                    })
                    .collect(),
            ),
            ..Default::default()
        }
    }

    const CALENDAR: &[u8] = include_bytes!("../../tests/fixtures/ocr_color_calendar_2959.png");
    const KNOWN: &str = "Monday Tuesday Wednesday Thursday";

    #[tokio::test]
    async fn colored_probe_keeps_the_primary_when_more_words_displace_known_text() {
        for (candidate, expected) in [
            (KNOWN, KNOWN),
            ("Monday Harbor Dredge Quarry Ridge Library Hours Orchard Layout", KNOWN),
            (
                "Monday Tuesday Wednesday Thursday Harbor Dredge Quarry Ridge",
                "Monday Tuesday Wednesday Thursday Harbor Dredge Quarry Ridge",
            ),
        ] {
            let actual = retry_if_colored(
                None,
                || has_colored_background(CALENDAR, &SecurityLimits::default()),
                async || Ok(document(candidate)),
                document(KNOWN),
            )
            .await
            .unwrap();
            assert_eq!(actual.content, expected, "candidate: {candidate}");
        }
    }

    #[test]
    fn known_confident_words_must_meet_the_three_quarter_boundary() {
        let primary = document(KNOWN);
        let at_boundary = document("Monday Tuesday Wednesday Harbor Dredge Quarry Ridge Library");
        let below_boundary = document("Monday Tuesday Harbor Dredge Quarry Ridge Library Hours");
        assert!(improves(&primary, &at_boundary));
        assert!(!improves(&primary, &below_boundary));
    }

    #[test]
    fn the_known_good_public_fixture_qualifies_for_the_color_probe() {
        let image = include_bytes!("../../tests/fixtures/ocr_color_library_2959.png");
        assert!(has_colored_background(image, &SecurityLimits::default()));
    }

    #[tokio::test]
    async fn cancellation_before_the_probe_prevents_all_retry_work() {
        let token = CancellationToken::new();
        token.cancel();
        let result = retry_if_colored(
            Some(&token),
            || panic!("cancelled before the color probe"),
            async || panic!("cancelled before OCR"),
            document(KNOWN),
        )
        .await;
        assert!(matches!(result, Err(crate::XbergError::Cancelled)));
    }

    #[tokio::test]
    async fn cancellation_during_the_probe_prevents_ocr() {
        let token = CancellationToken::new();
        let result = retry_if_colored(
            Some(&token),
            || {
                token.cancel();
                true
            },
            async || panic!("cancelled before OCR"),
            document(KNOWN),
        )
        .await;
        assert!(matches!(result, Err(crate::XbergError::Cancelled)));
    }

    #[tokio::test]
    async fn cancellation_during_ocr_discards_the_candidate() {
        let token = CancellationToken::new();
        let result = retry_if_colored(
            Some(&token),
            || true,
            async || {
                token.cancel();
                Ok(document("Monday Tuesday Wednesday Thursday Harbor Dredge Quarry Ridge"))
            },
            document(KNOWN),
        )
        .await;
        assert!(matches!(result, Err(crate::XbergError::Cancelled)));
    }
}
