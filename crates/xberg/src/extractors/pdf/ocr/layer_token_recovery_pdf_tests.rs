use super::*;
use crate::core::config::OcrQualityThresholds;
use crate::extractors::pdf::ocr::document::ocr_confirms_layer;
use crate::ocr_metadata_keys::{OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY, OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY};

/// The reproducer page of GH#2952 with a painted line whose layer is wrong: `financial` extracts
/// with the ligature U+FB01 for its f, which the extractor expands to fi, and `AB1995` extracts as `AB1996`.
const WRONG_LAYER_PDF: &[u8] = include_bytes!("../../../../tests/fixtures/ocr_layer_tokens_2952/wrong.pdf");
const PAGE: (f32, f32) = (612.0, 792.0);
const SCALE: f32 = 2.0;
const WORKING_BYTES: usize = 64 * 1024 * 1024;

/// A read of the page that has every layer word as the layer spells it, except where `reading`
/// gives another spelling for a layer token, each with `confidence`.
fn read_of(pdf: &PdfDocument, reading: &[(&str, &str)], confidence: f64) -> ExtractedDocument {
    read_where(pdf, reading, confidence, |_| true)
}

/// As `read_of`, but only the layer words that `kept` accepts are read at all.
fn read_where(
    pdf: &PdfDocument,
    reading: &[(&str, &str)],
    confidence: f64,
    kept: impl Fn(&str) -> bool,
) -> ExtractedDocument {
    let raster_height = PAGE.1 * SCALE;
    let words: Vec<OcrElement> = pdf
        .extract_text_lines(0)
        .unwrap()
        .iter()
        .flat_map(|line| &line.words)
        .filter(|word| is_decodable(&word.text) && kept(&word.text))
        .map(|word| {
            let text = reading
                .iter()
                .find(|(layer, _)| *layer == word.text)
                .map_or(word.text.as_str(), |(_, read)| read);
            OcrElement {
                text: text.to_owned(),
                level: OcrElementLevel::Word,
                geometry: OcrBoundingGeometry::Rectangle {
                    left: (word.bbox.x * SCALE) as u32,
                    top: (raster_height - (word.bbox.y + word.bbox.height) * SCALE) as u32,
                    width: (word.bbox.width * SCALE) as u32,
                    height: (word.bbox.height * SCALE) as u32,
                },
                confidence: OcrConfidence {
                    detection: None,
                    recognition: confidence,
                },
                backend_metadata: [("backend".to_owned(), serde_json::json!("tesseract-iterator"))].into(),
                ..Default::default()
            }
        })
        .collect();
    let mut metadata = crate::types::Metadata::default();
    metadata.additional.insert(
        OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY.into(),
        serde_json::json!((PAGE.0 * SCALE) as u32),
    );
    metadata.additional.insert(
        OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY.into(),
        serde_json::json!((PAGE.1 * SCALE) as u32),
    );
    ExtractedDocument {
        content: words
            .iter()
            .map(|word| word.text.as_str())
            .collect::<Vec<_>>()
            .join(" "),
        ocr_elements: Some(words),
        metadata,
        ..Default::default()
    }
}

/// The page's layer as the page decision sees it: the words of its lines.
fn layer_text(pdf: &PdfDocument) -> String {
    pdf.extract_text_lines(0)
        .unwrap()
        .iter()
        .flat_map(|line| line.words.iter().map(|word| word.text.clone()))
        .collect::<Vec<_>>()
        .join(" ")
}

/// A page long enough for the decision's quality check: the layer and a paragraph the read holds too.
const PROSE: &str = "The regional office reviewed the quarterly figures and found that deliveries \
    stayed on schedule across the northern depots while the southern routes needed extra vehicles \
    during the winter months because snow closed several roads and the drivers waited at the \
    border crossing for inspections that took longer than planned this year";

fn recover(pdf: &PdfDocument, read: &mut ExtractedDocument) -> bool {
    recover_with_layer(pdf, read, Some(&layer_text(pdf)))
}

fn recover_with_layer(pdf: &PdfDocument, read: &mut ExtractedDocument, page_layer: Option<&str>) -> bool {
    recover_page(
        pdf,
        0,
        read,
        page_layer,
        InkProofLimits {
            thresholds: &OcrQualityThresholds::default(),
            max_working_bytes: WORKING_BYTES,
            cancelled: &|| false,
        },
    )
}

fn tokens(read: &ExtractedDocument) -> Vec<&str> {
    read.content.split_whitespace().collect()
}

#[test]
fn a_layer_with_a_ligature_form_and_a_shifted_digit_never_replaces_a_correct_low_confidence_read() {
    let pdf = PdfDocument::from_bytes(WRONG_LAYER_PDF.to_vec()).unwrap();
    let layer: Vec<String> = pdf
        .extract_text_lines(0)
        .unwrap()
        .iter()
        .flat_map(|line| line.words.iter().map(|word| word.text.clone()))
        .collect();
    assert!(
        layer.contains(&"fiinancial".to_owned()) && layer.contains(&"AB1996".to_owned()),
        "{layer:?}"
    );
    let mut read = read_of(&pdf, &[("fiinancial", "financial"), ("AB1996", "AB1995")], 0.3);
    let before = read.content.clone();
    recover(&pdf, &mut read);
    assert!(
        tokens(&read).contains(&"financial") && tokens(&read).contains(&"AB1995"),
        "{}",
        read.content
    );
    assert!(!read.content.contains("AB1996"), "{}", read.content);
    assert_eq!(
        before.split_whitespace().filter(|token| *token == "financial").count(),
        tokens(&read).iter().filter(|token| **token == "financial").count()
    );
}

#[test]
fn a_layer_with_a_dropped_glyph_never_replaces_a_correct_low_confidence_read() {
    let pdf = PdfDocument::from_bytes(WRONG_LAYER_PDF.to_vec()).unwrap();
    let mut read = read_of(&pdf, &[("document", "documents")], 0.3);
    recover(&pdf, &mut read);
    assert!(
        tokens(&read).contains(&"documents") && !tokens(&read).contains(&"document"),
        "{}",
        read.content
    );
}

#[test]
fn a_low_confidence_confusable_misreading_takes_the_layer_spelling() {
    let pdf = PdfDocument::from_bytes(WRONG_LAYER_PDF.to_vec()).unwrap();
    let mut read = read_of(&pdf, &[("B0X92Q", "BOX92Q")], 0.3);
    assert!(recover(&pdf, &mut read));
    assert!(
        tokens(&read).contains(&"B0X92Q") && !tokens(&read).contains(&"BOX92Q"),
        "{}",
        read.content
    );
}

#[test]
fn a_confident_confusable_misreading_keeps_the_read() {
    let pdf = PdfDocument::from_bytes(WRONG_LAYER_PDF.to_vec()).unwrap();
    let mut read = read_of(&pdf, &[("B0X92Q", "BOX92Q")], 0.95);
    recover(&pdf, &mut read);
    assert!(
        tokens(&read).contains(&"BOX92Q") && !tokens(&read).contains(&"B0X92Q"),
        "{}",
        read.content
    );
}

#[test]
fn a_read_that_does_not_confirm_the_layer_gets_no_word_from_it() {
    let pdf = PdfDocument::from_bytes(WRONG_LAYER_PDF.to_vec()).unwrap();
    let digits_only = |token: &str| token.chars().all(|character| character.is_ascii_digit());
    let mut read = read_where(&pdf, &[], 0.9, digits_only);
    let before = read.content.clone();
    assert!(before.split_whitespace().count() >= MIN_LOCATED_WORDS, "{before}");
    assert!(!recover(&pdf, &mut read));
    assert_eq!(read.content, before);
}

#[test]
fn identifiers_misread_with_digits_and_case_take_the_layer_spelling_at_low_confidence() {
    // `SSTOU9` was already corrected before the case rule; it guards that the rule keeps it.
    let pdf = PdfDocument::from_bytes(WRONG_LAYER_PDF.to_vec()).unwrap();
    let mut read = read_of(&pdf, &[("S5T0U9", "SSTOU9"), ("K2Q5W0", "K2Q5wo")], 0.43);
    assert!(recover(&pdf, &mut read));
    let found = tokens(&read);
    assert!(
        found.contains(&"S5T0U9") && found.contains(&"K2Q5W0"),
        "{}",
        read.content
    );
    assert!(
        !found.contains(&"SSTOU9") && !found.contains(&"K2Q5wo"),
        "{}",
        read.content
    );
}

#[test]
fn the_same_identifiers_stay_as_read_at_the_confidence_floor() {
    let pdf = PdfDocument::from_bytes(WRONG_LAYER_PDF.to_vec()).unwrap();
    let mut read = read_of(&pdf, &[("S5T0U9", "SSTOU9"), ("K2Q5W0", "K2Q5wo")], 0.75);
    recover(&pdf, &mut read);
    let found = tokens(&read);
    assert!(
        found.contains(&"SSTOU9") && found.contains(&"K2Q5wo"),
        "{}",
        read.content
    );
}

#[test]
fn a_chart_page_whose_read_dropped_a_few_values_gets_them_back_and_still_confirms_its_layer() {
    let pdf = PdfDocument::from_bytes(WRONG_LAYER_PDF.to_vec()).unwrap();
    let layer = format!("{} {PROSE}", layer_text(&pdf));
    let thresholds = OcrQualityThresholds::default();
    let not_a_value = |token: &str| !token.chars().all(|character| character.is_ascii_digit());
    let mut read = read_where(&pdf, &[], 0.9, not_a_value);
    read.content = format!("{} {PROSE}", read.content);
    assert!(ocr_confirms_layer(&layer, &read.content, &thresholds));
    assert!(recover_with_layer(&pdf, &mut read, Some(&layer)), "{}", read.content);
    assert!(tokens(&read).contains(&"7"), "{}", read.content);
    assert!(ocr_confirms_layer(&layer, &read.content, &thresholds));
}

#[test]
fn a_number_that_would_make_the_read_confirm_the_layer_is_not_recovered() {
    let pdf = PdfDocument::from_bytes(WRONG_LAYER_PDF.to_vec()).unwrap();
    let base = layer_text(&pdf);
    let thresholds = OcrQualityThresholds::default();
    let not_a_value = |token: &str| !token.chars().all(|character| character.is_ascii_digit());
    let mut read = read_where(&pdf, &[], 0.9, not_a_value);
    read.content = format!("{} {PROSE}", read.content);
    let words = |text: &str| -> std::collections::HashSet<String> {
        text.split(|c: char| !c.is_alphanumeric())
            .filter(|word| !word.is_empty())
            .map(str::to_lowercase)
            .collect()
    };
    let held = words(&read.content).len();
    let numbers = words(&base)
        .iter()
        .filter(|word| word.chars().all(|c| c.is_ascii_digit()))
        .count();
    // Filler numbers that the page's words never include put the read just under the decision's
    // share of all words; one recovered number takes it over.
    let mut layer = format!("{base} {PROSE}");
    for value in 0..(held - numbers + 1) {
        layer.push_str(&format!(" {}", 5000 + value));
    }
    let before = read.content.clone();
    assert!(!ocr_confirms_layer(&layer, &before, &thresholds));
    let mut full = read.clone();
    full.content = format!("{} 7", before);
    assert!(
        ocr_confirms_layer(&layer, &full.content, &thresholds),
        "setup: one number must flip it"
    );
    recover_with_layer(&pdf, &mut read, Some(&layer));
    assert!(
        !ocr_confirms_layer(&layer, &read.content, &thresholds),
        "{}",
        read.content
    );
    assert!(!tokens(&read).contains(&"7"), "{}", read.content);
}
