use super::{Word, interleaving, retry_until_stop, source_order_improves};
use crate::cancellation::CancellationToken;
use std::future::{pending, poll_fn};
use std::time::{Duration, Instant};

#[tokio::test]
async fn stopped_or_expired_retry_never_polls_the_backend() {
    for deadline in [true, false] {
        let token = CancellationToken::new();
        if deadline {
            token.set_deadline(Instant::now());
        } else {
            token.request_stop();
        }
        let result = retry_until_stop(Some(&token), poll_fn(|_| panic!("retry polled after stop"))).await;
        assert_eq!(result.unwrap_or("baseline"), "baseline");
    }
}

#[tokio::test]
async fn a_stalled_retry_yields_the_baseline_after_stop_or_deadline() {
    for deadline in [true, false] {
        let token = CancellationToken::new();
        let result = tokio::time::timeout(
            Duration::from_secs(1),
            retry_until_stop(Some(&token), async {
                if deadline {
                    token.set_deadline(Instant::now());
                } else {
                    token.request_stop();
                }
                pending().await
            }),
        )
        .await
        .expect("the stalled backend must not hold the completed baseline");
        assert_eq!(result.unwrap_or("baseline"), "baseline");
    }
}

#[test]
fn deleting_real_source_words_cannot_count_as_improved_order() {
    let passage = [
        "The", "old", "stone", "bridge", "crosses", "the", "river", "below", "the", "mill", "at", "dawn",
    ];
    let mut old = Vec::new();
    for (row, word) in passage.into_iter().enumerate() {
        let top = 35 * row as u32;
        for left in (60..=540).step_by(20) {
            old.push(Word {
                text: "retained",
                bounds: [left, top, left + 12, top + 10],
                confidence: 0.95,
            });
        }
        old.push(Word {
            text: word,
            bounds: [560, top, 572, top + 10],
            confidence: 0.95,
        });
        for left in (674..=914).step_by(20) {
            old.push(Word {
                text: "retained",
                bounds: [left, top, left + 12, top + 10],
                confidence: 0.95,
            });
        }
    }
    let frame = (1000, 500);
    let jumps = interleaving(&old, frame);
    assert!(jumps >= 12);
    let deleted: Vec<_> = old.iter().copied().filter(|word| word.bounds[0] != 560).collect();
    assert_eq!(old.len() - deleted.len(), 12);
    assert!(deleted.len() * 100 >= old.len() * 92);
    assert!(interleaving(&deleted, frame) * 100 <= jumps * 45);
    assert!(!source_order_improves(&old, &deleted, frame));
    let mut reordered = old.clone();
    reordered.sort_by_key(|word| (word.bounds[0] >= 674, word.bounds[1], word.bounds[0]));
    assert!(source_order_improves(&old, &reordered, frame));
}

pub(super) const FRAME: (u32, u32) = (1000, 500);

fn source_page() -> Vec<Word<'static>> {
    let passage = [
        "The", "old", "stone", "bridge", "crosses", "the", "river", "below", "the", "mill", "at", "dawn",
    ];
    let mut words = Vec::new();
    for (row, token) in passage.into_iter().enumerate() {
        let top = 35 * row as u32;
        for left in (60..=560).step_by(20).chain((674..=914).step_by(20)) {
            words.push(Word {
                text: if left == 60 { token } else { "retained" },
                bounds: [left, top, left + 12, top + 10],
                confidence: 0.95,
            });
        }
    }
    words
}

pub(super) fn page_read(psm: i32, content: &str, words: &[Word<'_>]) -> crate::types::ExtractedDocument {
    use crate::ocr_metadata_keys::{OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY, OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY};
    use crate::types::{ExtractedDocument, FormatMetadata, OcrConfidence, OcrElement, OcrElementLevel, OcrMetadata};
    let mut read = ExtractedDocument {
        content: content.to_string(),
        ocr_elements: Some(
            words
                .iter()
                .map(|word| {
                    OcrElement::new(
                        word.text,
                        super::rectangle(word.bounds),
                        OcrConfidence::from_tesseract(95.0),
                    )
                    .with_level(OcrElementLevel::Word)
                })
                .collect(),
        ),
        ocr_internal_document: Some(Default::default()),
        ..Default::default()
    };
    read.metadata.format = Some(FormatMetadata::Ocr(OcrMetadata {
        psm,
        ..Default::default()
    }));
    read.metadata
        .additional
        .insert(OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY.into(), FRAME.0.into());
    read.metadata
        .additional
        .insert(OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY.into(), FRAME.1.into());
    read
}

#[tokio::test]
async fn an_eligible_deadline_cut_hands_off_primary_and_blocks_cache_publication() {
    use crate::core::extractor::cacheability;
    for stopped_before in [false, true] {
        for deadline in [false, true] {
            cacheability::scope(async {
                let token = CancellationToken::new();
                let stop = || {
                    if deadline {
                        token.set_deadline(Instant::now());
                    } else {
                        token.request_stop();
                    }
                };
                assert!(cacheability::can_publish_outer_result());
                if stopped_before {
                    stop();
                }
                let words = source_page();
                let primary = page_read(11, "completed primary", &words);
                let (read, witness) = tokio::time::timeout(
                    Duration::from_secs(1),
                    super::retry_order_read(primary, "tesseract", Some(&token), || async {
                        assert!(!stopped_before, "a stopped operation started an optional OCR read");
                        stop();
                        pending().await
                    }),
                )
                .await
                .expect("stalled optional read must release the completed page")
                .expect("soft deadline keeps the successful primary");
                assert!(witness.is_none());
                assert_eq!(read.content, "completed primary");
                assert_eq!(read.ocr_elements.as_ref().unwrap().len(), words.len());
                assert_eq!(read.processing_warnings.len(), 1);
                assert!(token.result_is_partial());
                assert!(!cacheability::can_publish_outer_result());
            })
            .await;
        }
    }
}

#[test]
fn adoption_discards_the_candidate_when_the_deadline_expires_between_phases() {
    let words = source_page();
    let primary = page_read(11, "completed primary", &words);
    let mut reordered = words;
    reordered.sort_by_key(|word| (word.bounds[0] >= 674, word.bounds[1], word.bounds[0]));

    for stop_at in [None, Some(1), Some(2), Some(3), Some(4)] {
        let token = CancellationToken::new();
        let candidate = page_read(3, "reordered candidate", &reordered);
        let mut checks = 0;
        let selected = super::adopt(&primary, candidate, "tesseract", FRAME, || {
            checks += 1;
            if stop_at == Some(checks) {
                token.set_deadline(Instant::now());
            }
            super::stop_retry(Some(&token))
        });
        assert_eq!(checks, stop_at.unwrap_or(4));
        assert_eq!(
            selected.as_ref().unwrap_or(&primary).content,
            if stop_at.is_some() {
                "completed primary"
            } else {
                "reordered candidate"
            }
        );
    }
}

#[tokio::test]
async fn rejects_reordered_read_that_loses_a_source_passage_at_adoption() {
    let words = source_page();
    let mut reordered = words.clone();
    reordered.sort_by_key(|w| (w.bounds[0] >= 674, w.bounds[1], w.bounds[0]));
    assert!(interleaving(&words, FRAME) >= 12);
    assert_eq!(interleaving(&reordered, FRAME), 0);
    let accepted = super::retry_read(page_read(11, "primary", &words), "tesseract", None, || async {
        Ok(page_read(3, "source-complete retry", &reordered))
    })
    .await
    .unwrap();
    assert_eq!(accepted.content, "source-complete retry");
    reordered.retain(|word| word.bounds[0] != 60);
    assert_eq!(words.len() - reordered.len(), 12);
    assert!(reordered.len() * 100 >= words.len() * 92);
    assert_eq!(interleaving(&reordered, FRAME), 0);
    let baseline = super::retry_read(page_read(11, "primary", &words), "tesseract", None, || async {
        Ok(page_read(3, "retry with deleted passage", &reordered))
    })
    .await
    .unwrap();
    assert_eq!(baseline.content, "primary");
    assert_eq!(baseline.ocr_elements.as_ref().unwrap().len(), words.len());
    assert!(baseline.processing_warnings.is_empty());
}

#[tokio::test]
async fn explicit_cancellation_during_retry_stays_an_error() {
    let token = CancellationToken::new();
    let words = source_page();
    let result = super::retry_read(page_read(11, "primary", &words), "tesseract", Some(&token), || async {
        token.cancel();
        pending().await
    })
    .await;
    assert!(matches!(result, Err(crate::XbergError::Cancelled)));
    assert!(!token.result_is_partial());
}

#[test]
fn occurrence_matching_preserves_duplicate_footprints_and_split_punctuation() {
    use super::source_occurrences_preserved;
    let word = |text, left, right| Word {
        text,
        bounds: [left, 0, right, 10],
        confidence: 0.95,
    };
    let duplicated = [word("same", 0, 10), word("same", 20, 30)];
    assert!(source_occurrences_preserved(
        &duplicated,
        &[duplicated[1], duplicated[0]]
    ));
    assert!(!source_occurrences_preserved(
        &duplicated,
        &[duplicated[1], duplicated[1]]
    ));

    let old = [word("can't", 0, 14)];
    assert!(source_occurrences_preserved(
        &old,
        &[word("ca", 0, 5), word("n't", 6, 14)]
    ));
    assert!(!source_occurrences_preserved(
        &old,
        &[word("ca", 0, 5), word("nt", 6, 14)]
    ));
}

fn structured_read(psm: i32, words: &[Word<'_>], first_row_split: Option<u32>) -> crate::types::ExtractedDocument {
    use crate::types::{
        OcrConfidence, OcrElement, OcrElementLevel,
        internal::{ElementKind, InternalElement},
    };

    let mut read = page_read(psm, "", words);
    let doc = read.ocr_internal_document.as_mut().unwrap();
    let public = read.ocr_elements.as_mut().unwrap();
    let original = std::mem::take(public);
    let mut offset = 0;
    for row in words.chunk_by(|a, b| {
        let crosses_split =
            a.bounds[1] == 0 && first_row_split.is_some_and(|split| a.bounds[0] < split && b.bounds[0] >= split);
        a.bounds[1] == b.bounds[1] && (a.bounds[0] >= 674) == (b.bounds[0] >= 674) && !crosses_split
    }) {
        let text = row.iter().map(|word| word.text).collect::<Vec<_>>().join(" ");
        let bounds = super::union(row.iter().map(|word| word.bounds)).unwrap();
        let mut block = InternalElement::text(ElementKind::Paragraph, text.clone(), 0);
        block.bbox = Some(super::bbox(bounds));
        doc.elements.push(block);
        public.push(
            OcrElement::new(text, super::rectangle(bounds), OcrConfidence::from_tesseract(95.0))
                .with_level(OcrElementLevel::Line),
        );
        public.extend_from_slice(&original[offset..offset + row.len()]);
        offset += row.len();
    }
    read.content = doc
        .elements
        .iter()
        .map(|block| block.text.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");
    read
}

#[tokio::test]
async fn restores_exact_source_occurrences_to_blocks_lines_and_words_or_keeps_primary() {
    use crate::types::OcrElementLevel;

    let mut old = source_page();
    let missing = [[80, 0, 92, 10], [100, 0, 112, 10], [80, 35, 92, 45]];
    for word in old.iter_mut().filter(|word| missing.contains(&word.bounds)) {
        word.text = "same!";
    }
    let mut reordered = old.clone();
    reordered.sort_by_key(|word| (word.bounds[0] >= 674, word.bounds[1], word.bounds[0]));
    let expected = structured_read(3, &reordered, None);
    reordered.retain(|word| !missing.contains(&word.bounds));
    let primary = structured_read(11, &old, None);
    let retry = structured_read(3, &reordered, None);
    let result = super::retry_read(primary.clone(), "tesseract", None, || async { Ok(retry.clone()) })
        .await
        .unwrap();
    assert_eq!(result.content, expected.content);
    assert_eq!(
        serde_json::to_value(&result.ocr_elements).unwrap(),
        serde_json::to_value(&expected.ocr_elements).unwrap()
    );
    assert_eq!(
        result.ocr_internal_document.as_ref().unwrap().elements[0].text,
        expected.ocr_internal_document.as_ref().unwrap().elements[0].text
    );
    let result_words = super::inventory(&result).unwrap();
    assert!(super::source_occurrences_preserved(&old, &result_words.words));

    for stop_at in 1..=4 {
        let token = CancellationToken::new();
        let mut checks = 0;
        let adopted = super::adopt(&primary, retry.clone(), "tesseract", FRAME, || {
            checks += 1;
            if checks == stop_at {
                token.set_deadline(Instant::now());
            }
            super::stop_retry(Some(&token))
        });
        assert_eq!(checks, stop_at);
        assert!(adopted.is_none());
    }

    let crossed = structured_read(3, &reordered, Some(120));
    let crossed_result = super::retry_read(primary.clone(), "tesseract", None, || async { Ok(crossed) })
        .await
        .unwrap();
    assert_eq!(crossed_result.content, primary.content);
    assert_eq!(
        serde_json::to_value(&crossed_result.ocr_elements).unwrap(),
        serde_json::to_value(&primary.ocr_elements).unwrap()
    );

    let mut ambiguous = retry;
    let elements = ambiguous.ocr_elements.as_mut().unwrap();
    let first_line = elements
        .iter()
        .find(|element| element.level == OcrElementLevel::Line)
        .unwrap()
        .clone();
    elements.push(first_line);
    let result = super::retry_read(primary.clone(), "tesseract", None, || async { Ok(ambiguous) })
        .await
        .unwrap();
    assert_eq!(result.content, primary.content);
    assert_eq!(
        serde_json::to_value(&result.ocr_elements).unwrap(),
        serde_json::to_value(&primary.ocr_elements).unwrap()
    );
}

#[tokio::test]
async fn restores_whole_line_from_unique_source_between_two_matched_column_neighbors() {
    let mut original = source_page();
    let missing = |word: &Word<'_>| word.bounds[1] == 35 && word.bounds[0] < 674;
    for word in original.iter_mut().filter(|word| missing(word)) {
        word.text = "verbatim!";
    }
    let mut reordered = original.clone();
    reordered.sort_by_key(|word| (word.bounds[0] >= 674, word.bounds[1], word.bounds[0]));
    let expected = structured_read(3, &reordered, None);
    reordered.retain(|word| !missing(word));
    let primary = structured_read(11, &original, None);
    let retry = structured_read(3, &reordered, None);
    let restored = super::retry_read(primary.clone(), "tesseract", None, || async { Ok(retry.clone()) })
        .await
        .unwrap();
    assert_eq!(restored.content, expected.content);
    assert_eq!(
        serde_json::to_value(&restored.ocr_elements).unwrap(),
        serde_json::to_value(&expected.ocr_elements).unwrap()
    );
    let words = super::inventory(&restored).unwrap();
    assert!(super::source_occurrences_preserved(&original, &words.words));

    let mut duplicate_owner = primary;
    let doc = duplicate_owner.ocr_internal_document.as_mut().unwrap();
    let repeated = doc
        .elements
        .iter()
        .find(|block| block.text.starts_with("verbatim!"))
        .unwrap()
        .clone();
    doc.elements.push(repeated);
    assert_whole_line_retry_keeps_primary(duplicate_owner, retry).await;
}

#[tokio::test]
async fn whole_line_retry_abstains_when_the_nearby_lower_line_is_in_another_column() {
    let removed_source_line = |word: &Word<'_>| word.bounds[1] == 70 && word.bounds[0] < 674;
    let mut original = source_page();
    let last_top = original.last().unwrap().bounds[1];
    let last_row: Vec<_> = original
        .iter()
        .copied()
        .filter(|word| word.bounds[1] == last_top)
        .collect();
    for shift in [35, 70] {
        original.extend(last_row.iter().copied().map(|mut word| {
            word.bounds[1] += shift;
            word.bounds[3] += shift;
            word
        }));
    }
    original.retain(|word| !removed_source_line(word));
    let mut reordered = original.clone();
    reordered.sort_by_key(|word| (word.bounds[0] >= 674, word.bounds[1], word.bounds[0]));
    reordered.retain(|word| word.bounds[1] != 35 || word.bounds[0] >= 674);
    assert!(super::interleaving(&original, FRAME) >= 12);
    assert_eq!(super::interleaving(&reordered, FRAME), 0);
    assert!(reordered.len() * 100 >= original.len() * 92);
    assert_whole_line_retry_keeps_primary(
        structured_read(11, &original, None),
        structured_read(3, &reordered, None),
    )
    .await;
}

async fn assert_whole_line_retry_keeps_primary(
    primary: crate::types::ExtractedDocument,
    retry: crate::types::ExtractedDocument,
) {
    let expected_content = primary.content.clone();
    let expected_elements = serde_json::to_value(&primary.ocr_elements).unwrap();
    let read_started = std::cell::Cell::new(false);
    let result = super::retry_read(primary, "tesseract", None, || async {
        read_started.set(true);
        Ok(retry)
    })
    .await
    .unwrap();
    assert!(read_started.get(), "the matched control must reach retry adoption");
    assert_eq!(result.content, expected_content);
    assert_eq!(serde_json::to_value(&result.ocr_elements).unwrap(), expected_elements);
    assert!(result.processing_warnings.is_empty());
}

#[tokio::test]
async fn transient_order_retry_error_keeps_primary_and_invalidates_cache() {
    use crate::core::extractor::cacheability;
    let words = source_page();
    let primary = page_read(11, "completed primary", &words);
    cacheability::scope(async {
        assert!(cacheability::can_publish_outer_result());
        let (result, witness) = super::retry_order_read(primary.clone(), "tesseract", None, || async {
            Err(crate::XbergError::Plugin {
                message: "temporary backend failure".into(),
                plugin_name: "tesseract".into(),
            })
        })
        .await
        .unwrap();
        assert_eq!(result.content, primary.content);
        assert!(result.processing_warnings.is_empty());
        assert!(witness.is_none());
        assert!(!cacheability::can_publish_outer_result());
    })
    .await;
    cacheability::scope(async {
        let (result, witness) = super::retry_order_read(primary.clone(), "tesseract", None, || async {
            Ok(page_read(3, "ordinary no-op", &words))
        })
        .await
        .unwrap();
        assert_eq!(result.content, primary.content);
        assert!(witness.is_none());
        assert!(cacheability::can_publish_outer_result());
    })
    .await;
}

#[test]
fn a_forced_read_without_layout_detection_admits_the_column_retry() {
    use crate::core::config::{ExtractionConfig, OcrConfig};
    let ocr = OcrConfig::default();
    let forced = ExtractionConfig {
        force_ocr: true,
        ..Default::default()
    };
    assert!(super::request_admits_retry(&forced, &ocr));
    assert!(super::request_admits_retry(&ExtractionConfig::default(), &ocr));
}

#[test]
fn a_caller_chosen_segmentation_or_page_set_refuses_the_column_retry() {
    use crate::core::config::{ExtractionConfig, OcrConfig};
    let config = ExtractionConfig::default();
    let mut explicit_psm = OcrConfig::default();
    explicit_psm.tesseract_config.get_or_insert_default().psm = Some(11);
    assert!(!super::request_admits_retry(&config, &explicit_psm));
    let selected_pages = ExtractionConfig {
        force_ocr_pages: Some(vec![1]),
        ..Default::default()
    };
    assert!(!super::request_admits_retry(&selected_pages, &OcrConfig::default()));
    let german = OcrConfig {
        language: vec!["deu".to_string()],
        ..Default::default()
    };
    assert!(!super::request_admits_retry(&config, &german));
}
