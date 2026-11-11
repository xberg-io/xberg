use super::*;

fn word(text: &str, column: f32, row: f32) -> PlacedWord {
    PlacedWord {
        text: text.to_owned(),
        place: LineBox {
            left: column * 60.0,
            right: column * 60.0 + 50.0,
            top: row * 20.0,
            bottom: row * 20.0 + 12.0,
        },
        confidence: 0.4,
    }
}

fn inked(_: &LineBox) -> bool {
    true
}

fn page(words: &[(&str, f32, f32)]) -> Vec<PlacedWord> {
    words
        .iter()
        .map(|(text, column, row)| word(text, *column, *row))
        .collect()
}

const AGREEING: [(&str, f32, f32); 4] = [
    ("Alpha", 0.0, 0.0),
    ("Beta", 1.0, 0.0),
    ("Gamma", 2.0, 0.0),
    ("Delta", 3.0, 0.0),
];

fn with(extra: &[(&'static str, f32, f32)]) -> Vec<(&'static str, f32, f32)> {
    AGREEING.iter().chain(extra).copied().collect()
}

#[test]
fn a_letter_misread_as_a_digit_takes_the_layer_spelling() {
    let layer = page(&with(&[("B0X92Q", 0.0, 1.0)]));
    let read = page(&with(&[("BOX92Q", 0.0, 1.0)]));
    let planned = plan(&layer, &read, &mut inked, &|| false);
    assert_eq!(planned.corrections, vec![(4, "B0X92Q".to_owned())]);
    assert!(planned.additions.is_empty());
}

#[test]
fn an_edge_character_the_read_dropped_returns_from_the_layer() {
    let layer = page(&with(&[("AB1234", 0.0, 1.0)]));
    let read = page(&with(&[("B1234", 0.0, 1.0)]));
    assert_eq!(
        plan(&layer, &read, &mut inked, &|| false).corrections,
        vec![(4, "AB1234".to_owned())]
    );
}

#[test]
fn a_layer_word_with_no_read_word_at_its_place_is_added_once() {
    let layer = page(&with(&[("7", 1.0, 5.0)]));
    let read = page(&AGREEING);
    let planned = plan(&layer, &read, &mut inked, &|| false);
    assert_eq!(planned.additions, vec![4]);
    assert!(planned.corrections.is_empty());
}

#[test]
fn a_different_word_at_the_same_place_stays_as_read() {
    let layer = page(&with(&[("Alpine", 0.0, 1.0), ("Coordinates", 1.0, 1.0)]));
    let read = page(&with(&[("Zebra", 0.0, 1.0), ("Quartz", 1.0, 1.0)]));
    assert_eq!(plan(&layer, &read, &mut inked, &|| false), Plan::default());
}

#[test]
fn a_distance_above_two_is_a_different_word() {
    assert!(!is_misreading("ABCDEF", "ABCXYZ"));
    assert!(is_misreading("BOX92Q", "B0X92Q"));
    assert!(is_misreading("SSTOU9", "S5T0U9"));
    assert!(is_misreading("rnodern", "modern"));
    assert!(!is_misreading("ab", "ac"));
    assert!(!is_misreading("word,", "word"));
    assert!(!is_misreading("form", "from"));
    assert!(!is_misreading("financial", "\u{FB01}nancial"));
    assert!(!is_misreading("certificate", "certicate"));
    assert!(!is_misreading("1995", "1996"));
    assert!(!is_misreading("AB1995", "AB1996"));
    assert!(!is_misreading("cert", "certs"));
}

#[test]
fn a_read_word_partly_over_a_layer_word_blocks_an_addition() {
    let layer = page(&with(&[("Orphan", 0.0, 1.0)]));
    let mut shifted = word("Orphan", 0.0, 1.0);
    shifted.place.left += 40.0;
    shifted.place.right += 40.0;
    let mut read = page(&AGREEING);
    read.push(shifted);
    assert!(plan(&layer, &read, &mut inked, &|| false).additions.is_empty());
}

#[test]
fn a_layer_that_disagrees_with_the_read_changes_nothing() {
    let layer = page(&[
        ("Alpha", 0.0, 0.0),
        ("Beta", 1.0, 0.0),
        ("Gamma", 2.0, 0.0),
        ("Delta", 3.0, 0.0),
        ("AB1234", 0.0, 1.0),
        ("7", 1.0, 5.0),
    ]);
    let read = page(&[
        ("Orion", 0.0, 0.0),
        ("Vega", 1.0, 0.0),
        ("Sirius", 2.0, 0.0),
        ("Rigel", 3.0, 0.0),
        ("B1234", 0.0, 1.0),
    ]);
    assert_eq!(plan(&layer, &read, &mut inked, &|| false), Plan::default());
}

#[test]
fn undecodable_layer_words_are_never_recovered() {
    let layer = page(&with(&[("\u{E041}\u{E042}", 0.0, 1.0), ("a\u{FFFD}b", 1.0, 1.0)]));
    let read = page(&AGREEING);
    assert_eq!(plan(&layer, &read, &mut inked, &|| false), Plan::default());
}

#[test]
fn a_word_the_page_does_not_draw_is_not_recovered() {
    let layer = page(&with(&[("B0X92Q", 0.0, 1.0), ("7", 1.0, 5.0)]));
    let read = page(&with(&[("BOX92Q", 0.0, 1.0)]));
    assert_eq!(plan(&layer, &read, &mut |_| false, &|| false), Plan::default());
}

#[test]
fn a_misread_spelling_repeated_without_a_layer_word_is_not_rewritten() {
    let layer = page(&with(&[("B0X92Q", 0.0, 1.0), ("BOX92Q", 0.0, 2.0)]));
    let read = page(&with(&[("BOX92Q", 0.0, 1.0), ("BOX92Q", 0.0, 2.0)]));
    let planned = plan(&layer, &read, &mut inked, &|| false);
    assert!(planned.corrections.is_empty());
}

#[test]
fn a_confident_read_word_is_never_corrected() {
    let layer = page(&with(&[("B0X92Q", 0.0, 1.0)]));
    let mut read = page(&with(&[("BOX92Q", 0.0, 1.0)]));
    read[4].confidence = 0.95;
    assert_eq!(plan(&layer, &read, &mut inked, &|| false), Plan::default());
}

#[test]
fn a_cancelled_plan_is_empty() {
    let layer = page(&with(&[("B0X92Q", 0.0, 1.0)]));
    let read = page(&with(&[("BOX92Q", 0.0, 1.0)]));
    assert_eq!(plan(&layer, &read, &mut inked, &|| true), Plan::default());
}

#[test]
fn a_word_with_a_copy_of_its_text_beside_it_is_not_added_again() {
    let layer = page(&with(&[("7", 1.0, 5.0)]));
    let mut shifted = word("7", 1.0, 5.0);
    shifted.place.top += 30.0;
    shifted.place.bottom += 30.0;
    let mut read = page(&AGREEING);
    read.push(shifted);
    assert!(plan(&layer, &read, &mut inked, &|| false).additions.is_empty());
}

#[test]
fn a_token_inside_markup_is_not_rewritten() {
    let mut read = read_of("**BOX92Q** and [BOX92Q](u)", &[("BOX92Q", 0, 0)]);
    assert!(!apply(
        &mut read,
        &[("BOX92Q".to_owned(), "B0X92Q".to_owned())],
        &[],
        &|| false
    ));
    assert_eq!(read.content, "**BOX92Q** and [BOX92Q](u)");
}

#[test]
fn a_table_whose_markdown_and_cells_disagree_keeps_the_misread_token() {
    let mut read = read_of("| BOX92Q |", &[("BOX92Q", 0, 0)]);
    read.tables.push(crate::types::Table {
        cells: vec![vec!["BOX92Q".into()]],
        markdown: "| BOX92Q | BOX92Q |".into(),
        page_number: 1,
        bounding_box: None,
        table_id: None,
        columns: None,
        cell_styles: Vec::new(),
    });
    assert!(!apply(
        &mut read,
        &[("BOX92Q".to_owned(), "B0X92Q".to_owned())],
        &[],
        &|| false
    ));
    assert_eq!(read.content, "| BOX92Q |");
}

#[test]
fn an_added_word_beside_markup_lands_beside_the_unmarked_neighbour() {
    let mut read = read_of(
        "Title\n\n**Left** Right",
        &[("Title", 0, 0), ("**Left**", 0, 100), ("Right", 200, 100)],
    );
    assert!(apply(&mut read, &[], &[added("Mid", 100, 100)], &|| false));
    assert_eq!(read.content, "Title\n\n**Left** Mid Right");
}

#[test]
fn an_added_word_is_placed_among_the_document_elements_by_its_reading_position() {
    use crate::types::internal::InternalDocument;
    let block = |text: &str, top: f64| {
        let mut element = InternalElement::text(
            ElementKind::OcrText {
                level: OcrElementLevel::Block,
            },
            text,
            0,
        );
        element.bbox = Some(crate::types::BoundingBox {
            x0: 0.0,
            y0: top,
            x1: 60.0,
            y1: top + 20.0,
        });
        element
    };
    let mut read = read_of("Title\n\nBody", &[("Title", 0, 0), ("Body", 0, 300)]);
    let mut document = InternalDocument::new("ocr");
    document.elements = vec![block("Title", 0.0), block("Body", 300.0)];
    read.ocr_internal_document = Some(document);
    assert!(apply(&mut read, &[], &[added("Mid", 0, 150)], &|| false));
    let texts: Vec<&str> = read
        .ocr_internal_document
        .as_ref()
        .unwrap()
        .elements
        .iter()
        .map(|e| e.text.as_str())
        .collect();
    assert_eq!(texts, ["Title", "Mid", "Body"]);
}

#[test]
fn replace_token_keeps_longer_tokens_whole() {
    let mut text = "BOX92Q ABOX92Q BOX92Q2 | BOX92Q | BOX92Q.".to_owned();
    assert_eq!(count_token(&text, "BOX92Q"), 2);
    assert_eq!(replace_token(&mut text, "BOX92Q", "B0X92Q"), 2);
    assert_eq!(text, "B0X92Q ABOX92Q BOX92Q2 | B0X92Q | BOX92Q.");
}

fn element(text: &str, left: u32, top: u32) -> OcrElement {
    OcrElement {
        text: text.to_owned(),
        geometry: OcrBoundingGeometry::Rectangle {
            left,
            top,
            width: 60,
            height: 20,
        },
        confidence: OcrConfidence {
            detection: None,
            recognition: 0.9,
        },
        level: OcrElementLevel::Word,
        rotation: None,
        page_number: 1,
        parent_id: None,
        backend_metadata: Default::default(),
    }
}

fn read_of(content: &str, words: &[(&str, u32, u32)]) -> ExtractedDocument {
    ExtractedDocument {
        content: content.to_owned(),
        ocr_elements: Some(
            words
                .iter()
                .map(|(text, left, top)| element(text, *left, *top))
                .collect(),
        ),
        ..Default::default()
    }
}

fn added(text: &str, left: u32, top: u32) -> AddedWord {
    (text.to_owned(), (left, top, 60, 20))
}

#[test]
fn a_correction_rewrites_the_token_everywhere_the_read_holds_it() {
    let mut read = read_of(
        "Alpha BOX92Q\n\nBeta",
        &[("Alpha", 0, 0), ("BOX92Q", 100, 0), ("Beta", 0, 100)],
    );
    assert!(apply(
        &mut read,
        &[("BOX92Q".to_owned(), "B0X92Q".to_owned())],
        &[],
        &|| false
    ));
    assert_eq!(read.content, "Alpha B0X92Q\n\nBeta");
    assert_eq!(read.ocr_elements.as_ref().unwrap()[1].text, "B0X92Q");
}

#[test]
fn a_correction_is_skipped_when_the_text_holds_the_token_a_different_number_of_times() {
    let mut read = read_of("BOX92Q and BOX92Q", &[("BOX92Q", 0, 0)]);
    assert!(!apply(
        &mut read,
        &[("BOX92Q".to_owned(), "B0X92Q".to_owned())],
        &[],
        &|| false
    ));
    assert_eq!(read.content, "BOX92Q and BOX92Q");
}

#[test]
fn an_added_word_becomes_its_own_block_between_the_blocks_above_and_below_it() {
    let mut read = read_of(
        "Title\n\n100\n\nLabels",
        &[("Title", 0, 0), ("100", 400, 100), ("Labels", 0, 300)],
    );
    assert!(apply(&mut read, &[], &[added("80", 200, 200)], &|| false));
    assert_eq!(read.content, "Title\n\n100\n\n80\n\nLabels");
    assert_eq!(read.ocr_elements.as_ref().unwrap().len(), 4);
}

#[test]
fn an_added_word_below_the_last_block_goes_last() {
    let mut read = read_of("Title\n\nBody", &[("Title", 0, 0), ("Body", 0, 100)]);
    assert!(apply(&mut read, &[], &[added("7", 300, 900)], &|| false));
    assert_eq!(read.content, "Title\n\nBody\n\n7");
}

#[test]
fn an_added_word_joins_the_line_it_stands_on() {
    let mut read = read_of(
        "Title\n\nLeft Right",
        &[("Title", 0, 0), ("Left", 0, 100), ("Right", 200, 100)],
    );
    assert!(apply(&mut read, &[], &[added("Mid", 100, 100)], &|| false));
    assert_eq!(read.content, "Title\n\nLeft Mid Right");
}

#[test]
fn an_added_word_is_skipped_when_the_blocks_do_not_run_down_the_page() {
    let mut read = read_of(
        "Left top\n\nLeft bottom\n\nRight top\n\nRight bottom",
        &[
            ("Left", 0, 0),
            ("top", 60, 0),
            ("bottom", 0, 500),
            ("Right", 600, 0),
            ("top2", 660, 0),
        ],
    );
    assert!(!apply(&mut read, &[], &[added("9", 300, 250)], &|| false));
    assert_eq!(read.content, "Left top\n\nLeft bottom\n\nRight top\n\nRight bottom");
}

#[test]
fn an_added_word_inside_a_table_is_skipped() {
    let mut read = read_of("Title\n\n| A | B |", &[("Title", 0, 0), ("A", 0, 100), ("B", 100, 100)]);
    read.tables.push(crate::types::Table {
        cells: vec![vec!["A".into(), "B".into()]],
        markdown: "| A | B |".into(),
        page_number: 1,
        bounding_box: Some(crate::types::BoundingBox {
            x0: 0.0,
            y0: 90.0,
            x1: 400.0,
            y1: 300.0,
        }),
        table_id: None,
        columns: None,
        cell_styles: Vec::new(),
    });
    assert!(!apply(&mut read, &[], &[added("9", 200, 150)], &|| false));
    assert_eq!(read.content, "Title\n\n| A | B |");
}
