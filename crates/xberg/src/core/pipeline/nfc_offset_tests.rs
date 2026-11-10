//! Byte offsets that index `content` follow it through Unicode normalization (#2851).

use super::*;
use crate::types::entity::{Entity, EntityCategory};
use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
use crate::types::{PageBoundary, PageStructure, PageUnitType};

/// "Cafe" and U+0301 are 6 bytes before NFC and "Café" is 5 after, so every offset past it moves.
const DECOMPOSED: &str = "Lyon, Cafe\u{0301} in Paris";
const COMPOSED: &str = "Lyon, Caf\u{00e9} in Paris";

fn location(content: &str, text: &str) -> Entity {
    let start = content.find(text).expect("the mention is in the content");
    Entity {
        category: EntityCategory::Location,
        text: text.to_string(),
        start: start as u32,
        end: (start + text.len()) as u32,
        confidence: None,
    }
}

fn mentions(result: &ExtractedDocument) -> Vec<Option<&str>> {
    let entities = result.entities.as_deref().unwrap_or_default();
    entities
        .iter()
        .map(|e| result.content.get(e.start as usize..e.end as usize))
        .collect()
}

#[test]
fn nfc_moves_entity_spans_with_the_text_they_name() {
    for formatted_content in [None, Some("# rendered".to_string())] {
        let mut result = ExtractedDocument {
            content: DECOMPOSED.to_string(),
            formatted_content,
            entities: Some(
                ["Lyon", "Cafe\u{0301}", "Paris"]
                    .map(|t| location(DECOMPOSED, t))
                    .to_vec(),
            ),
            ..Default::default()
        };

        normalize_nfc(&mut result);

        assert_eq!(result.content, COMPOSED);
        assert_eq!(
            mentions(&result),
            [Some("Lyon"), Some("Caf\u{00e9}"), Some("Paris")],
            "rendered output set: {}",
            result.formatted_content.is_some()
        );
    }
}

#[test]
fn nfc_leaves_an_entity_span_that_was_not_on_a_character_boundary_as_it_was() {
    let mut result = ExtractedDocument {
        content: DECOMPOSED.to_string(),
        entities: Some(vec![location(DECOMPOSED, "Paris")]),
        ..Default::default()
    };
    let inside_the_accent = DECOMPOSED.find('\u{0301}').unwrap() as u32 + 1;
    let entities = result.entities.as_mut().unwrap();
    entities[0].start = inside_the_accent;
    let before = entities.clone();

    normalize_nfc(&mut result);

    assert_eq!(result.content, COMPOSED);
    assert_eq!(result.entities.unwrap(), before);
}

/// A span that cuts a base letter from the mark that joins it lands inside the composed letter once
/// they compose. It grows to hold the composed letter at either end, and every span after it still
/// finds its text, whether the letter comes once or twice and whatever the order of the spans. Each
/// case joins for another reason: an accent, Hangul jamo cut after the first and after the second
/// one, and two Oriya vowel signs.
#[test]
fn nfc_grows_an_entity_span_that_splits_a_letter_from_the_mark_that_joins_it() {
    for (decomposed, cut_after, composed) in [
        ("e\u{0301}", 1, "\u{00e9}"),
        ("\u{1100}\u{1161}", 1, "\u{ac00}"),
        ("\u{1100}\u{1161}\u{11a8}", 2, "\u{ac01}"),
        ("\u{0b47}\u{0b3e}", 1, "\u{0b4b}"),
    ] {
        let content = format!("Lyon, {decomposed} in Paris, {decomposed} again");
        let first_cut = decomposed.chars().take(cut_after).map(char::len_utf8).sum::<usize>();
        let span = |start: usize, end: usize| Entity {
            start: start as u32,
            end: end as u32,
            ..location(&content, "Lyon")
        };
        let letters: Vec<usize> = content.match_indices(decomposed).map(|(at, _)| at).collect();
        let mut entities = vec![
            span(letters[0], letters[0] + first_cut),
            span(letters[0] + first_cut, letters[0] + decomposed.len()),
            location(&content, "Paris"),
            span(letters[1], letters[1] + first_cut),
            span(letters[1] + first_cut, letters[1] + decomposed.len()),
        ];
        let mut expected = vec![
            Some(composed),
            Some(composed),
            Some("Paris"),
            Some(composed),
            Some(composed),
        ];
        for reversed in [false, true] {
            if reversed {
                entities.reverse();
                expected.reverse();
            }
            let mut result = ExtractedDocument {
                content: content.clone(),
                entities: Some(entities.clone()),
                ..Default::default()
            };

            normalize_nfc(&mut result);

            assert_eq!(result.content, format!("Lyon, {composed} in Paris, {composed} again"));
            assert_eq!(mentions(&result), expected, "{decomposed:?}, reversed {reversed}");
        }
    }
}

/// A Middle-stage processor that tags "Paris" in the text it is given, as the NER step does.
#[cfg(feature = "tokio-runtime")]
struct ParisTagger;

#[cfg(feature = "tokio-runtime")]
impl crate::plugins::Plugin for ParisTagger {
    fn name(&self) -> &str {
        "nfc-offset-paris-tagger"
    }
    fn version(&self) -> String {
        "test".to_string()
    }
    fn initialize(&self) -> Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg(feature = "tokio-runtime")]
#[async_trait::async_trait]
impl crate::plugins::PostProcessor for ParisTagger {
    async fn process(&self, result: &mut ExtractedDocument, _config: &ExtractionConfig) -> Result<()> {
        result.entities = Some(vec![location(&result.content, "Paris")]);
        Ok(())
    }
    fn processing_stage(&self) -> crate::plugins::ProcessingStage {
        crate::plugins::ProcessingStage::Middle
    }
}

#[cfg(feature = "tokio-runtime")]
#[tokio::test]
#[serial_test::serial]
async fn entity_spans_of_a_post_processor_index_the_content_after_nfc() {
    let registry = crate::plugins::registry::get_post_processor_registry();
    registry.write().register(std::sync::Arc::new(ParisTagger)).unwrap();
    clear_processor_cache().unwrap();
    let mut doc = InternalDocument::new("plain");
    doc.mime_type = "text/plain".to_string();
    doc.push_element(InternalElement::text(ElementKind::Paragraph, DECOMPOSED, 0));

    let processed = run_pipeline(doc, &ExtractionConfig::default()).await;

    registry.write().remove("nfc-offset-paris-tagger").unwrap();
    clear_processor_cache().unwrap();
    let processed = processed.unwrap();
    assert_eq!(processed.content, COMPOSED);
    assert_eq!(mentions(&processed), [Some("Paris")]);
}

fn result_with_one_range(content: &str, byte_start: usize, byte_end: usize) -> ExtractedDocument {
    let mut result = ExtractedDocument {
        content: content.to_string(),
        ..Default::default()
    };
    result.metadata.pages = Some(PageStructure {
        total_count: 1,
        unit_type: PageUnitType::Page,
        boundaries: Some(vec![PageBoundary {
            byte_start,
            byte_end,
            page_number: 1,
        }]),
        pages: None,
    });
    result
}

/// The range ends between "e" and its accent: on a character boundary before NFC, inside "é" after.
#[test]
fn nfc_omits_a_page_range_that_splits_a_letter_from_its_accent_and_says_so() {
    let mut result = result_with_one_range("e\u{0301}ab", 0, 1);

    normalize_nfc(&mut result);

    assert_eq!(result.content, "\u{00e9}ab");
    assert!(result.metadata.pages.unwrap().boundaries.is_none());
    assert!(result.processing_warnings.iter().any(|w| w.source == "page_boundaries"));
}

/// The synchronous entry runs the same normalization step as the async one.
#[cfg(not(feature = "tokio-runtime"))]
#[test]
fn the_synchronous_pipeline_keeps_page_ranges_on_their_page_text_after_nfc() {
    let mut doc = InternalDocument::new("plain");
    doc.mime_type = "text/plain".to_string();
    for (page, text) in [(1, "First page"), (2, DECOMPOSED), (3, "Third page")] {
        doc.push_element(InternalElement::text(ElementKind::Paragraph, text, 0).with_page(page));
    }
    doc.metadata.pages = Some(PageStructure {
        total_count: 3,
        unit_type: PageUnitType::Page,
        boundaries: Some(
            (1..=3)
                .map(|page_number| PageBoundary {
                    byte_start: 0,
                    byte_end: 0,
                    page_number,
                })
                .collect(),
        ),
        pages: None,
    });

    let processed = run_pipeline_sync(doc, &ExtractionConfig::default()).unwrap();

    assert!(processed.content.contains(COMPOSED));
    let boundaries = processed.metadata.pages.unwrap().boundaries.unwrap();
    let pages = processed.pages.unwrap();
    assert_eq!(boundaries.len(), 3);
    for (boundary, page) in boundaries.iter().zip(&pages) {
        assert_eq!(
            processed.content.get(boundary.byte_start..boundary.byte_end),
            Some(page.content.as_str())
        );
    }
}
