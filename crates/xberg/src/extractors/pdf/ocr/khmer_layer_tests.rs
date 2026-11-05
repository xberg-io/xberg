use super::*;
use crate::types::OcrConfidence;

fn word(text: &str, left: u32, width: u32, confidence: f64, language: &str) -> OcrElement {
    OcrElement::new(
        text,
        OcrBoundingGeometry::Rectangle {
            left,
            top: 10,
            width,
            height: 20,
        },
        OcrConfidence {
            detection: None,
            recognition: confidence,
        },
    )
    .with_level(OcrElementLevel::Word)
    .with_metadata("word_language", serde_json::json!(language))
}

fn read(content: &str, words: Vec<OcrElement>) -> ExtractedDocument {
    ExtractedDocument {
        content: content.to_string(),
        ocr_elements: Some(words),
        ..Default::default()
    }
}

/// The letter and digit runs of `text`, for the check that no token of the first read is lost.
fn tokens(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(str::to_string)
        .collect()
}

#[test]
fn khmer_words_go_after_the_low_confidence_latin_letters_read_in_their_place() {
    let mut base = read(
        "fiunm Invoice",
        vec![word("fiunm", 0, 90, 0.30, "eng"), word("Invoice", 100, 70, 0.95, "eng")],
    );
    let khmer = read(
        "សួស្តី Invoice",
        vec![word("សួស្តី", 0, 90, 0.85, "khm"), word("Invoice", 100, 70, 0.95, "eng")],
    );
    assert_eq!(merge_khmer_words(&mut base, &khmer), 1);
    assert_eq!(base.content, "fiunm សួស្តី Invoice");
}

#[test]
fn a_khmer_word_whose_confidence_does_not_beat_the_latin_word_is_not_taken() {
    let mut base = read("Total", vec![word("Total", 0, 60, 0.92, "eng")]);
    let khmer = read("តូ", vec![word("តូ", 0, 60, 0.70, "khm")]);
    assert_eq!(merge_khmer_words(&mut base, &khmer), 0);
    assert_eq!(base.content, "Total");
}

#[test]
fn no_word_of_the_first_read_is_removed_when_khmer_words_are_taken() {
    let mut base = read(
        "fiu 1042 Total",
        vec![
            word("fiu", 0, 40, 0.20, "eng"),
            word("1042", 45, 40, 0.20, "eng"),
            word("Total", 90, 40, 0.30, "eng"),
        ],
    );
    let khmer = read("ភាសា", vec![word("ភាសា", 0, 130, 0.90, "khm")]);
    assert_eq!(merge_khmer_words(&mut base, &khmer), 1);
    assert_eq!(base.content, "fiu 1042 Total ភាសា");
    for token in tokens("fiu 1042 Total") {
        assert!(tokens(&base.content).contains(&token), "{token} was lost");
    }
}

#[test]
fn two_groups_on_one_line_keep_their_order() {
    let mut base = read(
        "ab cd",
        vec![word("ab", 0, 40, 0.20, "eng"), word("cd", 50, 40, 0.20, "eng")],
    );
    let khmer = read(
        "ក ខ",
        vec![word("ក", 0, 40, 0.90, "khm"), word("ខ", 50, 40, 0.90, "khm")],
    );
    assert_eq!(merge_khmer_words(&mut base, &khmer), 2);
    assert_eq!(base.content, "ab ក cd ខ");
}

#[test]
fn words_at_different_places_never_merge() {
    let mut base = read("fiunm", vec![word("fiunm", 0, 50, 0.10, "eng")]);
    let khmer = read("ភាសា", vec![word("ភាសា", 500, 50, 0.90, "khm")]);
    assert_eq!(merge_khmer_words(&mut base, &khmer), 0);
    assert_eq!(base.content, "fiunm");
}

#[test]
fn a_word_the_second_read_did_not_read_with_khmer_is_not_taken() {
    let mut base = read("fiunm", vec![word("fiunm", 0, 50, 0.10, "eng")]);
    let khmer = read("ភាសា", vec![word("ភាសា", 0, 50, 0.90, "eng")]);
    assert_eq!(merge_khmer_words(&mut base, &khmer), 0);
    assert_eq!(base.content, "fiunm");
}

fn word_at(text: &str, left: u32, top: u32, confidence: f64, language: &str) -> OcrElement {
    let mut element = word(text, left, 60, confidence, language);
    element.geometry = OcrBoundingGeometry::Rectangle {
        left,
        top,
        width: 60,
        height: 20,
    };
    element
}

fn structured(paragraphs: &[&str]) -> crate::types::internal::InternalDocument {
    use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
    let mut document = InternalDocument::new("pdf");
    for text in paragraphs {
        document.push_element(InternalElement::text(
            ElementKind::OcrText {
                level: OcrElementLevel::Block,
            },
            *text,
            0,
        ));
    }
    document
}

fn paragraph_texts(read: &ExtractedDocument) -> Vec<String> {
    let document = read.ocr_internal_document.as_ref().expect("structured document");
    document.elements.iter().map(|element| element.text.clone()).collect()
}

#[test]
fn a_line_the_noise_filter_dropped_gets_its_khmer_words_before_the_next_line() {
    let mut base = read(
        "Invoice 1042",
        vec![
            word_at("fiunm", 0, 10, 0.20, "eng"),
            word_at("Invoice", 0, 60, 0.95, "eng"),
            word_at("1042", 70, 60, 0.95, "eng"),
        ],
    );
    let khmer = read("", vec![word_at("ភាសា", 0, 10, 0.90, "khm")]);
    assert_eq!(merge_khmer_words(&mut base, &khmer), 1);
    assert_eq!(base.content, "ភាសា\nInvoice 1042");
}

#[test]
fn a_dropped_line_after_a_kept_line_gets_its_khmer_words_after_it() {
    let mut base = read(
        "Invoice",
        vec![
            word_at("Invoice", 0, 10, 0.95, "eng"),
            word_at("fiunm", 0, 60, 0.20, "eng"),
        ],
    );
    let khmer = read("", vec![word_at("ភាសា", 0, 60, 0.90, "khm")]);
    assert_eq!(merge_khmer_words(&mut base, &khmer), 1);
    assert_eq!(base.content, "Invoice\nភាសា");
}

#[test]
fn two_dropped_lines_after_a_kept_line_keep_their_order() {
    let mut base = read(
        "Invoice",
        vec![
            word_at("Invoice", 0, 10, 0.95, "eng"),
            word_at("fiunm", 0, 60, 0.20, "eng"),
            word_at("xyz", 0, 110, 0.20, "eng"),
        ],
    );
    let khmer = read(
        "",
        vec![word_at("ភាសា", 0, 60, 0.90, "khm"), word_at("ខ្មែរ", 0, 110, 0.90, "khm")],
    );
    assert_eq!(merge_khmer_words(&mut base, &khmer), 2);
    assert_eq!(base.content, "Invoice\nភាសា\nខ្មែរ");
}

#[test]
fn a_page_whose_whole_text_was_dropped_gets_its_khmer_words_as_a_paragraph() {
    let mut base = read("", vec![word_at("fiunm", 0, 10, 0.20, "eng")]);
    base.ocr_internal_document = Some(structured(&[]));
    let khmer = read("", vec![word_at("ភាសា", 0, 10, 0.90, "khm")]);
    assert_eq!(merge_khmer_words(&mut base, &khmer), 1);
    assert_eq!(base.content, "ភាសា");
    assert_eq!(paragraph_texts(&base), vec!["ភាសា"]);
}

#[test]
fn the_structured_document_takes_the_same_khmer_words_as_the_content() {
    let mut base = read(
        "Title\n\nfiunm Invoice",
        vec![
            word_at("Title", 0, 0, 0.95, "eng"),
            word_at("fiunm", 0, 40, 0.30, "eng"),
            word_at("Invoice", 100, 40, 0.95, "eng"),
        ],
    );
    base.ocr_internal_document = Some(structured(&["Title", "fiunm Invoice"]));
    let khmer = read("", vec![word_at("សួស្តី", 0, 40, 0.85, "khm")]);
    assert_eq!(merge_khmer_words(&mut base, &khmer), 1);
    assert_eq!(base.content, "Title\n\nfiunm សួស្តី Invoice");
    assert_eq!(paragraph_texts(&base), vec!["Title", "fiunm សួស្តី Invoice"]);
}

#[test]
fn a_content_that_is_not_the_structured_paragraphs_gets_its_khmer_words_as_a_last_paragraph() {
    let mut base = read("| fiunm |", vec![word_at("fiunm", 0, 10, 0.30, "eng")]);
    base.ocr_internal_document = Some(structured(&["fiunm"]));
    let khmer = read("", vec![word_at("ភាសា", 0, 10, 0.90, "khm")]);
    assert_eq!(merge_khmer_words(&mut base, &khmer), 1);
    assert_eq!(base.content, "| fiunm |\n\nភាសា");
    assert_eq!(paragraph_texts(&base), vec!["fiunm", "ភាសា"]);
}

#[test]
fn a_base_word_matches_only_a_whole_token_of_the_text() {
    let mut base = read(
        "Fern",
        vec![word_at("Fe", 0, 10, 0.20, "eng"), word_at("Fern", 200, 10, 0.95, "eng")],
    );
    let khmer = read("", vec![word_at("ភាសា", 0, 10, 0.90, "khm")]);
    assert_eq!(merge_khmer_words(&mut base, &khmer), 1);
    assert_eq!(base.content, "ភាសា Fern");
}

#[test]
fn a_khmer_layer_needs_twenty_khmer_letters() {
    let ocr = OcrConfig::default();
    assert!(!wants_khmer_read(&ocr, "ភាសា English text only"));
}

#[test]
fn configured_khmer_or_another_backend_gets_no_second_read() {
    let layer = "ភាសាខ្មែរ គឺជាភាសាផ្លូវការ នៃប្រទេសកម្ពុជា";
    let khmer = OcrConfig {
        language: vec!["eng".to_string(), "khm".to_string()],
        ..Default::default()
    };
    assert!(!wants_khmer_read(&khmer, layer));
    let paddle = OcrConfig {
        backend: "paddleocr".to_string(),
        ..Default::default()
    };
    assert!(!wants_khmer_read(&paddle, layer));
}

#[test]
fn the_khmer_language_goes_to_the_field_tesseract_reads() {
    let outer = with_khmer_language(&OcrConfig {
        language: vec!["deu".to_string()],
        ..Default::default()
    });
    assert_eq!(outer.effective_tesseract_language(), vec!["deu", "khm"]);
    let nested = with_khmer_language(&OcrConfig {
        tesseract_config: Some(crate::types::TesseractConfig {
            language: vec!["fra".to_string()],
            ..Default::default()
        }),
        ..Default::default()
    });
    assert_eq!(nested.effective_tesseract_language(), vec!["fra", "khm"]);
    assert_eq!(nested.language, vec!["eng"], "the outer field keeps its value");
}

struct CountingBackend {
    name: &'static str,
    reads: std::sync::atomic::AtomicUsize,
}

impl crate::plugins::Plugin for CountingBackend {
    fn name(&self) -> &str {
        self.name
    }
    fn version(&self) -> String {
        "1.0.0".to_string()
    }
    fn initialize(&self) -> crate::Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> crate::Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl crate::plugins::OcrBackend for CountingBackend {
    fn backend_type(&self) -> crate::plugins::OcrBackendType {
        crate::plugins::OcrBackendType::Custom
    }
    fn supports_language(&self, _: &str) -> bool {
        true
    }
    async fn process_image(&self, _: &[u8], _: &OcrConfig) -> crate::Result<ExtractedDocument> {
        self.reads.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(ExtractedDocument::default())
    }
}

async fn reads_of_one_page(name: &'static str, second_khmer_read: bool) -> usize {
    let backend = std::sync::Arc::new(CountingBackend {
        name,
        reads: std::sync::atomic::AtomicUsize::new(0),
    });
    let shared: std::sync::Arc<dyn crate::plugins::OcrBackend> = backend.clone();
    let ocr = OcrConfig::default();
    let policy = crate::extractors::image::SparseRetryPolicy {
        whole_page_raster: false,
        allow_standard_retry: false,
    };
    read_page_with_khmer_layer(
        &shared,
        std::sync::Arc::new(Vec::new()),
        (&ocr, &ocr),
        policy,
        None,
        second_khmer_read,
    )
    .await
    .expect("read");
    backend.reads.load(std::sync::atomic::Ordering::SeqCst)
}

#[tokio::test]
async fn only_a_tesseract_stage_of_a_khmer_document_reads_a_page_twice() {
    assert_eq!(reads_of_one_page("tesseract", true).await, 2);
    assert_eq!(
        reads_of_one_page("paddle-ocr", true).await,
        1,
        "a Paddle stage keeps one read"
    );
    assert_eq!(
        reads_of_one_page("tesseract", false).await,
        1,
        "a document with no Khmer layer keeps one read"
    );
}
