use super::tests::page_break;
use super::*;
use crate::layout::{BBox, LayoutClass};
use crate::pdf::structure::types::LayoutHintClass;

fn region_hint(class_name: LayoutHintClass, bottom: f32, top: f32) -> LayoutHint {
    LayoutHint {
        class_name,
        confidence: 0.9,
        left: 50.0,
        bottom,
        right: 550.0,
        top,
    }
}

fn page_box(bottom: f64, top: f64) -> BoundingBox {
    BoundingBox {
        x0: 60.0,
        y0: bottom,
        x1: 540.0,
        y1: top,
    }
}

fn paragraph_at(page: u32, text: &str, bottom: f64, top: f64) -> InternalElement {
    let mut element = InternalElement::text(ElementKind::Paragraph, text, 0).with_page(page);
    element.bbox = Some(page_box(bottom, top));
    element
}

fn table_at(document: &mut InternalDocument, page: u32, id: &str, bottom: f64, top: f64) -> InternalElement {
    let table_index = document.push_table(crate::types::Table {
        cells: vec![vec![id.to_string()]],
        markdown: format!("| {id} |"),
        page_number: page,
        bounding_box: Some(page_box(bottom, top)),
        ..Default::default()
    });
    let mut element = InternalElement::text(ElementKind::Table { table_index }, "", 0).with_page(page);
    element.bbox = Some(page_box(bottom, top));
    element
}

fn region_result(page_index: usize, markdown: &str, hint: LayoutHint) -> RegionVlmResult {
    RegionVlmResult {
        page_index,
        markdown: markdown.to_string(),
        hint,
    }
}

fn texts(document: &InternalDocument) -> Vec<String> {
    document
        .elements
        .iter()
        .map(|element| match element.kind {
            ElementKind::Table { table_index } => {
                format!("table:{}", document.tables[table_index as usize].markdown)
            }
            ElementKind::PageBreak => "break".to_string(),
            _ => element.text.clone(),
        })
        .collect()
}

/// Page 1: a caption above a table region, the native table and a stray row read as text
/// inside it, a paragraph below. Page 2: a table at the same coordinates.
fn table_region_document() -> InternalDocument {
    use crate::types::internal::{Relationship, RelationshipKind, RelationshipTarget};

    let mut document = InternalDocument::default();
    let first = table_at(&mut document, 1, "native-1", 340.0, 580.0);
    let second = table_at(&mut document, 2, "native-2", 340.0, 580.0);
    document.elements.push(paragraph_at(1, "caption", 620.0, 640.0));
    document.elements.push(first);
    document
        .elements
        .push(paragraph_at(1, "row read as text", 310.0, 330.0));
    document.elements.push(paragraph_at(1, "below", 100.0, 200.0));
    document.elements.push(page_break());
    document.elements.push(second);
    document.relationships.push(Relationship {
        source: 0,
        target: RelationshipTarget::Index(1),
        kind: RelationshipKind::Caption,
    });
    document.relationships.push(Relationship {
        source: 3,
        target: RelationshipTarget::Index(5),
        kind: RelationshipKind::InternalLink,
    });
    document
}

#[test]
fn should_replace_native_content_inside_table_and_other_regions_with_the_answer() {
    use crate::types::internal::RelationshipTarget;

    for class_name in [LayoutHintClass::Table, LayoutHintClass::Other] {
        let mut document = table_region_document();

        inject_region_results(
            &mut document,
            vec![region_result(
                0,
                "VLM native-1 row read as text",
                region_hint(class_name, 300.0, 600.0),
            )],
        );

        assert_eq!(
            texts(&document),
            [
                "caption",
                "VLM native-1 row read as text",
                "below",
                "break",
                "table:| native-2 |"
            ],
            "{class_name:?}"
        );
        assert_eq!(document.tables.len(), 2, "only the elements change: {class_name:?}");
        assert_eq!(
            document.relationships.len(),
            1,
            "the caption's link to the removed table must go: {:?}",
            document.relationships
        );
        assert_eq!(document.relationships[0].source, 2);
        assert_eq!(document.relationships[0].target, RelationshipTarget::Index(4));
    }
}

#[test]
fn should_keep_native_content_beside_a_picture_answer() {
    let mut document = table_region_document();

    inject_region_results(
        &mut document,
        vec![region_result(
            0,
            "VLM FIGURE",
            region_hint(LayoutHintClass::Picture, 300.0, 600.0),
        )],
    );

    assert_eq!(
        texts(&document),
        [
            "caption",
            "VLM FIGURE",
            "table:| native-1 |",
            "row read as text",
            "below",
            "break",
            "table:| native-2 |"
        ]
    );
    assert_eq!(document.tables.len(), 2);
    assert_eq!(document.relationships.len(), 2);
}

#[test]
fn should_keep_elements_and_tables_mostly_outside_the_region() {
    let mut document = InternalDocument::default();
    let straddling_table = table_at(&mut document, 1, "straddling", 100.0, 580.0);
    document.elements.push(straddling_table);
    document.elements.push(paragraph_at(1, "half inside", 250.0, 350.0));
    document.elements.push(paragraph_at(1, "mostly inside", 270.0, 370.0));

    inject_region_results(
        &mut document,
        vec![region_result(
            0,
            "straddling half inside mostly inside",
            region_hint(LayoutHintClass::Table, 300.0, 600.0),
        )],
    );

    assert_eq!(
        texts(&document),
        [
            "table:| straddling |",
            "half inside",
            "straddling half inside mostly inside"
        ],
        "a table 58% inside and a paragraph 50% inside stay; one 70% inside goes"
    );
}

#[test]
fn should_replace_a_paragraph_whose_box_has_inverted_corners() {
    let mut inverted = paragraph_at(1, "inverted", 400.0, 500.0);
    inverted.bbox = Some(BoundingBox {
        x0: 540.0,
        y0: 500.0,
        x1: 60.0,
        y1: 400.0,
    });
    let mut document = InternalDocument::default();
    document.elements.push(inverted);
    document.elements.push(paragraph_at(1, "below", 100.0, 200.0));

    inject_region_results(
        &mut document,
        vec![region_result(
            0,
            "VLM inverted",
            region_hint(LayoutHintClass::Table, 300.0, 600.0),
        )],
    );

    assert_eq!(texts(&document), ["VLM inverted", "below"]);
}

#[test]
fn should_nest_a_replacing_answer_at_the_depth_of_what_it_replaced() {
    let mut nested = paragraph_at(1, "nested", 400.0, 500.0);
    nested.depth = 2;
    let mut document = InternalDocument::default();
    document.elements.push(nested);

    inject_region_results(
        &mut document,
        vec![
            region_result(0, "VLM nested", region_hint(LayoutHintClass::Table, 300.0, 600.0)),
            region_result(0, "VLM FIGURE", region_hint(LayoutHintClass::Picture, 300.0, 600.0)),
        ],
    );

    let depths: Vec<(&str, u16)> = document
        .elements
        .iter()
        .map(|element| (element.text.as_str(), element.depth))
        .collect();
    assert_eq!(depths, [("VLM nested", 2), ("VLM FIGURE", 0)]);
}

/// A table directly followed by `container`, whose start has no box: the answer must take the
/// table's place before the container, not land after its start.
fn table_followed_by(container: ElementKind, child: InternalElement, end: ElementKind) -> InternalDocument {
    let mut document = InternalDocument::default();
    let table = table_at(&mut document, 1, "native", 340.0, 580.0);
    document.elements.push(paragraph_at(1, "above", 650.0, 700.0));
    document.elements.push(table);
    document
        .elements
        .push(InternalElement::text(container, "", 0).with_page(1));
    document.elements.push(child);
    document.elements.push(InternalElement::text(end, "", 0));
    document.elements.push(paragraph_at(1, "below", 100.0, 150.0));
    document
}

fn kinds_and_texts(document: &InternalDocument) -> Vec<String> {
    document
        .elements
        .iter()
        .map(|element| format!("{}:{}", element.kind.discriminant(), element.text))
        .collect()
}

#[test]
fn should_put_a_replacing_answer_before_a_list_that_follows_the_table() {
    let mut document = table_followed_by(
        ElementKind::ListStart { ordered: false },
        element_at(ElementKind::ListItem { ordered: false }, 1, "one", 200.0, 220.0),
        ElementKind::ListEnd,
    );
    document.elements[2].page = None;

    inject_region_results(
        &mut document,
        vec![region_result(
            0,
            "VLM native",
            region_hint(LayoutHintClass::Table, 300.0, 600.0),
        )],
    );

    assert_eq!(
        kinds_and_texts(&document),
        [
            "paragraph:above",
            "paragraph:VLM native",
            "list_start:",
            "list_item:one",
            "list_end:",
            "paragraph:below"
        ]
    );
    assert_eq!(
        crate::rendering::render_markdown(&document),
        "above\n\nVLM native\n\n- one\n\nbelow\n"
    );
}

#[test]
fn should_put_a_replacing_answer_before_a_group_that_follows_the_table() {
    let mut document = table_followed_by(
        ElementKind::GroupStart,
        paragraph_at(1, "grouped", 200.0, 220.0),
        ElementKind::GroupEnd,
    );

    inject_region_results(
        &mut document,
        vec![region_result(
            0,
            "VLM native",
            region_hint(LayoutHintClass::Table, 300.0, 600.0),
        )],
    );

    assert_eq!(
        kinds_and_texts(&document),
        [
            "paragraph:above",
            "paragraph:VLM native",
            "group_start:",
            "paragraph:grouped",
            "group_end:",
            "paragraph:below"
        ]
    );
}

#[test]
fn should_count_no_share_for_a_box_without_area() {
    let flat = BoundingBox {
        x0: 100.0,
        y0: 400.0,
        x1: 200.0,
        y1: 400.0,
    };

    assert_eq!(
        contained_share(&flat, &region_hint(LayoutHintClass::Table, 300.0, 600.0)),
        0.0
    );
}

#[test]
fn should_keep_a_table_split_evenly_between_two_regions() {
    let mut document = InternalDocument::default();
    let table = table_at(&mut document, 1, "split", 340.0, 580.0);
    document.elements.push(table);

    inject_region_results(
        &mut document,
        vec![
            region_result(0, "UPPER split", region_hint(LayoutHintClass::Table, 460.0, 600.0)),
            region_result(0, "LOWER split", region_hint(LayoutHintClass::Table, 300.0, 460.0)),
        ],
    );
    assert_eq!(
        texts(&document),
        ["UPPER split", "table:| split |", "LOWER split"],
        "a table split evenly between two regions belongs to neither"
    );
}

#[test]
fn should_find_a_flat_table_element_by_its_table_page_and_box() {
    let mut document = InternalDocument::default();
    let table_index = document.push_table(crate::types::Table {
        markdown: "| flat |".to_string(),
        page_number: 1,
        bounding_box: Some(page_box(340.0, 580.0)),
        ..Default::default()
    });
    document
        .elements
        .push(InternalElement::text(ElementKind::Table { table_index }, "", 0));

    inject_region_results(
        &mut document,
        vec![region_result(
            0,
            "VLM TABLE",
            region_hint(LayoutHintClass::Table, 300.0, 600.0),
        )],
    );

    assert_eq!(texts(&document), ["VLM TABLE"]);
}

#[test]
fn should_place_an_answer_replacing_an_unpaged_table_element_after_its_page() {
    let mut document = InternalDocument::default();
    let table_index = document.push_table(crate::types::Table {
        markdown: "| flat |".to_string(),
        page_number: 1,
        bounding_box: Some(page_box(340.0, 580.0)),
        ..Default::default()
    });
    for (page, text) in [(1, "page one"), (2, "page two")] {
        document
            .elements
            .push(InternalElement::text(ElementKind::Paragraph, text, 0).with_page(page));
    }
    document
        .elements
        .push(InternalElement::text(ElementKind::Table { table_index }, "", 0));

    inject_region_results(
        &mut document,
        vec![region_result(
            0,
            "VLM TABLE",
            region_hint(LayoutHintClass::Table, 300.0, 600.0),
        )],
    );

    assert_eq!(
        texts(&document),
        ["page one", "VLM TABLE", "page two"],
        "the flat path pushes the table element at the end, which is not page one's place"
    );
}

#[test]
fn should_replace_only_the_paragraphs_whose_words_the_answer_carries() {
    let mut document = InternalDocument::default();
    let table = table_at(&mut document, 1, "native", 400.0, 580.0);
    document.elements.push(table);
    document.elements.push(paragraph_at(1, "North 42", 370.0, 390.0));
    document
        .elements
        .push(paragraph_at(1, "Item Total units", 340.0, 360.0));
    document
        .elements
        .push(paragraph_at(1, "Source: Statistics Office", 310.0, 330.0));
    let answer = "| Item | Total |\n| --- | --- |\n| North | 42 |\n| native | |";

    inject_region_results(
        &mut document,
        vec![region_result(
            0,
            answer,
            region_hint(LayoutHintClass::Table, 300.0, 600.0),
        )],
    );

    assert_eq!(
        texts(&document),
        [answer, "Item Total units", "Source: Statistics Office"],
        "a paragraph of under ten words goes only when the answer carries every one of them"
    );
}

#[test]
fn should_let_each_answer_word_stand_in_for_one_paragraph_only() {
    let mut document = InternalDocument::default();
    document.elements.push(paragraph_at(1, "Total 42", 400.0, 420.0));
    document.elements.push(paragraph_at(1, "total 42", 360.0, 380.0));

    inject_region_results(
        &mut document,
        vec![region_result(
            0,
            "| Total | 42 |",
            region_hint(LayoutHintClass::Table, 300.0, 600.0),
        )],
    );

    assert_eq!(texts(&document), ["| Total | 42 |", "total 42"]);
}

#[test]
fn should_keep_a_title_whose_words_are_only_the_tables_cells() {
    let mut document = InternalDocument::default();
    document.elements.push(paragraph_at(1, "Revenue", 560.0, 580.0));
    let table = table_at(&mut document, 1, "Revenue", 400.0, 540.0);
    document.elements.push(table);
    let answer = "| Revenue |\n| --- |";

    inject_region_results(
        &mut document,
        vec![region_result(
            0,
            answer,
            region_hint(LayoutHintClass::Table, 300.0, 600.0),
        )],
    );

    assert_eq!(
        texts(&document),
        ["Revenue", answer],
        "the table's cells spend the answer's words first"
    );
}

#[test]
fn should_keep_a_native_table_beside_a_degenerate_answer() {
    let mut document = InternalDocument::default();
    let table = table_at(&mut document, 1, "native", 340.0, 580.0);
    document.elements.push(table);
    document.elements.push(paragraph_at(1, "below", 100.0, 200.0));

    inject_region_results(
        &mut document,
        vec![region_result(
            0,
            "| 0000000000 |",
            region_hint(LayoutHintClass::Table, 300.0, 600.0),
        )],
    );

    assert_eq!(texts(&document), ["| 0000000000 |", "table:| native |", "below"]);
}

const CELL_WORDS: [&str; 10] = [
    "north", "south", "east", "west", "units", "total", "2023", "2024", "12", "40",
];

#[test]
fn should_keep_a_native_table_the_answer_misses_and_replace_a_paragraph_it_carries() {
    let mut document = InternalDocument::default();
    let table = table_at(&mut document, 1, "native", 500.0, 580.0);
    document.elements.push(table);
    document.elements.push(paragraph_at(1, "North 42", 400.0, 420.0));
    let answer = "| North | 42 |";

    inject_region_results(
        &mut document,
        vec![region_result(
            0,
            answer,
            region_hint(LayoutHintClass::Table, 300.0, 600.0),
        )],
    );

    assert_eq!(
        texts(&document),
        ["table:| native |", answer],
        "the answer takes the paragraph's slot"
    );
}

#[test]
fn should_replace_a_native_table_only_when_the_answer_carries_nine_tenths_of_its_cell_words() {
    for (carried, replaced) in [(9, true), (8, false)] {
        let mut document = InternalDocument::default();
        let table = table_at(&mut document, 1, "ten", 340.0, 580.0);
        document.tables[0].cells = CELL_WORDS
            .chunks(5)
            .map(|row| row.iter().map(ToString::to_string).collect())
            .collect();
        document.elements.push(table);
        let answer = CELL_WORDS[..carried].join(" | ");

        inject_region_results(
            &mut document,
            vec![region_result(
                0,
                &answer,
                region_hint(LayoutHintClass::Table, 300.0, 600.0),
            )],
        );

        let kept = (!replaced).then(|| "table:| ten |".to_string());
        let expected: Vec<String> = std::iter::once(answer).chain(kept).collect();
        assert_eq!(texts(&document), expected, "{carried} of 10 cell words");
    }
}

/// Injects `answer` over a native 3x4 table of `cells` and returns the resulting element texts.
fn texts_after_answer_over_table(cells: [[&str; 4]; 3], answer: &str) -> Vec<String> {
    let mut document = InternalDocument::default();
    let table = table_at(&mut document, 1, "native", 340.0, 580.0);
    document.tables[0].cells = cells
        .iter()
        .map(|row| row.iter().map(ToString::to_string).collect())
        .collect();
    document.elements.push(table);

    inject_region_results(
        &mut document,
        vec![region_result(
            0,
            answer,
            region_hint(LayoutHintClass::Table, 300.0, 600.0),
        )],
    );
    texts(&document)
}

#[test]
fn should_replace_a_native_table_with_a_compact_pipe_table_answer() {
    let cells = [
        ["item", "q1", "q2", "q3"],
        ["north", "10", "20", "30"],
        ["south", "11", "21", "31"],
    ];
    let answer = "|Item|Q1|Q2|Q3|\n|---|---|---|---|\n|North|10|20|30|\n|South|11|21|31|";

    assert_eq!(texts_after_answer_over_table(cells, answer), [answer]);
}

#[test]
fn should_replace_a_native_table_with_an_answer_that_breaks_cells_with_br_tags() {
    let cells = [
        ["item", "q1", "q2", "q3"],
        ["north america", "10", "20", "30"],
        ["south america", "11", "21", "31"],
    ];
    let answer = "| Item | Q1 | Q2 | Q3 |\n| --- | --- | --- | --- |\n| North<br>America | 10 | 20 | 30 |\n| South<BR/>America | 11 | 21 | 31 |";

    assert_eq!(texts_after_answer_over_table(cells, answer), [answer]);
}

#[test]
fn should_replace_a_native_table_with_an_html_table_answer() {
    let cells = [
        ["item", "q1", "q2", "q3"],
        ["north", "10", "20", "30"],
        ["south", "11", "21", "31"],
    ];
    let answer = "<table><tr><td>Item</td> <td>Q1</td> <td>Q2</td> <td>Q3</td></tr>\n<tr><td>North</td> <td>10</td> <td>20</td> <td>30</td></tr>\n<tr><td>South</td> <td>11</td> <td>21</td> <td>31</td></tr></table>";

    assert_eq!(texts_after_answer_over_table(cells, answer), [answer]);
}

/// PaddleOCR-VL answers a "Table Recognition:" prompt in OTSL, with no space between cells.
#[test]
fn should_replace_a_native_table_with_an_otsl_table_answer() {
    let cells = [
        ["item", "q1", "q2", "q3"],
        ["north", "10", "20", "30"],
        ["south", "11", "21", "31"],
    ];
    let answer = "<fcel>Item<fcel>Q1<fcel>Q2<fcel>Q3<nl><fcel>North<fcel>10<fcel>20<fcel>30<nl><fcel>South<fcel>11<fcel>21<fcel>31<nl>";

    assert_eq!(texts_after_answer_over_table(cells, answer), [answer]);
}

/// A degenerate answer can be a million `<` with no `>`; the word split must take linear time on it.
#[test]
fn should_split_an_answer_of_unclosed_angle_brackets_in_linear_time() {
    for unit in ["<", "<a"] {
        let answer = format!("{} north america", unit.repeat(1_000_000 / unit.len()));

        let started = std::time::Instant::now();
        let text = answer_text_for_words(&answer);
        let elapsed = started.elapsed();

        assert!(elapsed.as_secs() < 5, "{unit:?} run took {elapsed:?}");
        assert_eq!(text, answer, "no tag was closed, so nothing changes");
        let words = super::super::token_multiset([text.as_str()]);
        assert_eq!(words.get("america"), Some(&1));
        assert_eq!(words.get("north"), Some(&1));
    }
}

#[test]
fn should_replace_a_paragraph_missing_one_word_only_when_it_has_ten_or_more() {
    let long = "one two three four five six seven eight nine ten";
    let short = "red green blue cyan pink";
    let mut document = InternalDocument::default();
    document.elements.push(paragraph_at(1, long, 450.0, 550.0));
    document.elements.push(paragraph_at(1, short, 350.0, 420.0));
    let answer = "one two three four five six seven eight nine red green blue cyan";

    inject_region_results(
        &mut document,
        vec![region_result(
            0,
            answer,
            region_hint(LayoutHintClass::Table, 300.0, 600.0),
        )],
    );

    assert_eq!(
        texts(&document),
        [answer, short],
        "9 of 10 words is enough, 4 of 5 is not"
    );
}

#[test]
fn should_keep_images_inside_an_other_region() {
    let mut document = InternalDocument::default();
    let mut image = InternalElement::text(ElementKind::Image { image_index: 0 }, "", 0).with_page(1);
    image.bbox = Some(page_box(340.0, 580.0));
    document.elements.push(image);

    inject_region_results(
        &mut document,
        vec![region_result(
            0,
            "VLM LAYOUT",
            region_hint(LayoutHintClass::Other, 300.0, 600.0),
        )],
    );

    assert_eq!(document.elements.len(), 2);
    assert!(matches!(document.elements[1].kind, ElementKind::Image { .. }));
}

fn element_at(kind: ElementKind, page: u32, text: &str, bottom: f64, top: f64) -> InternalElement {
    let mut element = InternalElement::text(kind, text, 0).with_page(page);
    element.bbox = Some(page_box(bottom, top));
    element
}

#[test]
fn should_replace_only_tables_and_body_paragraphs_inside_the_region() {
    use crate::types::internal::{Relationship, RelationshipTarget};

    let mut document = InternalDocument::default();
    let table = table_at(&mut document, 1, "native", 460.0, 580.0);
    document.elements.push(table);
    document
        .elements
        .push(element_at(ElementKind::Paragraph, 1, "cell text", 430.0, 450.0));
    document.elements.push(element_at(
        ElementKind::Heading { level: 2 },
        1,
        "heading",
        400.0,
        420.0,
    ));
    document
        .elements
        .push(element_at(ElementKind::Paragraph, 1, "caption", 380.0, 395.0));
    document
        .elements
        .push(element_at(ElementKind::Formula, 1, "E = mc^2", 360.0, 375.0));
    document
        .elements
        .push(element_at(ElementKind::Paragraph, 1, "footer", 340.0, 355.0).with_layer(ContentLayer::Footer));
    document.elements.push(element_at(
        ElementKind::ListItem { ordered: false },
        1,
        "item",
        320.0,
        335.0,
    ));
    document.relationships.push(Relationship {
        source: 3,
        target: RelationshipTarget::Index(0),
        kind: RelationshipKind::Caption,
    });
    let answer = "native cell text heading caption E = mc^2 footer item";

    inject_region_results(
        &mut document,
        vec![region_result(
            0,
            answer,
            region_hint(LayoutHintClass::Table, 300.0, 600.0),
        )],
    );

    assert_eq!(
        texts(&document),
        [answer, "heading", "caption", "E = mc^2", "footer", "item"],
        "the answer carries every word, so only kind, layer and the caption link keep them"
    );
}

#[test]
fn should_drop_groups_the_answer_empties_and_break_around_it() {
    let hint = || region_hint(LayoutHintClass::Table, 300.0, 600.0);
    let group_start = || InternalElement::text(ElementKind::GroupStart, "", 0).with_page(2);
    let group_end = || InternalElement::text(ElementKind::GroupEnd, "", 0);

    let mut emptied = InternalDocument::default();
    emptied.elements.push(paragraph_at(1, "first", 100.0, 200.0));
    emptied.elements.push(page_break());
    emptied.elements.push(group_start());
    emptied.elements.push(group_start());
    emptied.elements.push(paragraph_at(2, "cell text", 340.0, 580.0));
    emptied.elements.push(group_end());
    emptied.elements.push(group_end());
    emptied.elements.push(page_break());
    emptied.elements.push(paragraph_at(3, "third", 100.0, 200.0));
    inject_region_results(&mut emptied, vec![region_result(1, "VLM cell text", hint())]);
    assert_eq!(texts(&emptied), ["first", "break", "VLM cell text", "break", "third"]);

    let mut kept = InternalDocument::default();
    kept.elements.push(group_start());
    kept.elements.push(element_at(
        ElementKind::Heading { level: 2 },
        2,
        "heading",
        560.0,
        580.0,
    ));
    kept.elements.push(paragraph_at(2, "cell text", 340.0, 540.0));
    kept.elements.push(group_end());
    inject_region_results(&mut kept, vec![region_result(1, "VLM cell text", hint())]);
    let kinds: Vec<_> = kept.elements.iter().map(|element| element.kind).collect();
    assert_eq!(
        kinds,
        [
            ElementKind::GroupStart,
            ElementKind::Heading { level: 2 },
            ElementKind::Paragraph,
            ElementKind::GroupEnd
        ]
    );
    assert_eq!(kept.elements[2].text, "VLM cell text");
}

#[test]
fn should_keep_one_break_on_each_side_of_a_page_the_answer_empties() {
    let hint = || region_hint(LayoutHintClass::Table, 300.0, 600.0);

    let mut middle = InternalDocument::default();
    let table = table_at(&mut middle, 2, "middle", 340.0, 580.0);
    middle.elements.push(paragraph_at(1, "first", 100.0, 200.0));
    middle.elements.push(page_break());
    middle.elements.push(table);
    middle.elements.push(page_break());
    middle.elements.push(paragraph_at(3, "third", 100.0, 200.0));
    inject_region_results(&mut middle, vec![region_result(1, "VLM middle", hint())]);
    assert_eq!(texts(&middle), ["first", "break", "VLM middle", "break", "third"]);
    assert_eq!(middle.elements[2].page, Some(2));

    let mut leading = InternalDocument::default();
    let table = table_at(&mut leading, 1, "leading", 340.0, 580.0);
    leading.elements.push(table);
    leading.elements.push(page_break());
    leading.elements.push(paragraph_at(2, "second", 100.0, 200.0));
    inject_region_results(&mut leading, vec![region_result(0, "VLM leading", hint())]);
    assert_eq!(texts(&leading), ["VLM leading", "break", "second"]);

    let mut trailing = InternalDocument::default();
    let table = table_at(&mut trailing, 2, "trailing", 340.0, 580.0);
    trailing.elements.push(paragraph_at(1, "first", 100.0, 200.0));
    trailing.elements.push(page_break());
    trailing.elements.push(table);
    inject_region_results(&mut trailing, vec![region_result(1, "VLM trailing", hint())]);
    assert_eq!(texts(&trailing), ["first", "break", "VLM trailing"]);
}

fn table_region_llm_config(base_url: String) -> LlmConfig {
    LlmConfig {
        model: "openai/gpt-4o-mini".to_string(),
        api_key: Some("test-key".to_string()),
        base_url: Some(base_url),
        timeout_secs: Some(10),
        max_retries: Some(0),
        ..Default::default()
    }
}

#[tokio::test]
async fn should_leave_native_tables_alone_when_the_vlm_answer_is_empty_or_fails() {
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let empty_answer = serde_json::json!({
        "id": "chatcmpl-test",
        "object": "chat.completion",
        "created": 0,
        "model": "openai/gpt-4o-mini",
        "choices": [{
            "index": 0,
            "message": { "role": "assistant", "content": "  \n " },
            "finish_reason": "stop"
        }],
        "usage": { "prompt_tokens": 10, "completion_tokens": 10, "total_tokens": 20 }
    });
    for response in [
        ResponseTemplate::new(200).set_body_json(empty_answer),
        ResponseTemplate::new(500),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST")).respond_with(response).mount(&server).await;
        let page_image = image::RgbImage::new(1_275, 1_650);
        let hints = vec![region_hint(LayoutHintClass::Table, 300.0, 600.0)];
        let detection = DetectionResult::new(
            1_275,
            1_650,
            vec![LayoutDetection::new(
                LayoutClass::Table,
                0.9,
                BBox::new(100.0, 400.0, 1_100.0, 1_000.0),
            )],
        );

        let results = extract_vlm_regions(
            &[page_image],
            &[hints],
            &[detection],
            &table_region_llm_config(server.uri()),
        )
        .await;
        let mut document = table_region_document();
        let before = texts(&document);
        inject_region_results(&mut document, results);

        assert_eq!(
            server.received_requests().await.map(|requests| requests.len()),
            Some(1),
            "the table region must reach the VLM"
        );
        assert_eq!(texts(&document), before);
        assert_eq!(document.tables.len(), 2);
        assert_eq!(document.relationships.len(), 2);
    }
}
