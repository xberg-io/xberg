use super::certify_figure_caption;
use crate::extractors::security::SecurityLimits;
use crate::types::formats::OcrTable;
use crate::types::{OcrBoundingGeometry, OcrElement, OcrElementLevel};

const FRAME: (u32, u32) = (2586, 3346);

#[derive(serde::Deserialize)]
struct Source {
    table: OcrTable,
    elements: Vec<OcrElement>,
}

fn source() -> Source {
    serde_json::from_str(include_str!("../../tests/fixtures/ocr_caption_2530/p9-iterator.json"))
        .expect("exact bounded OCR excerpt")
}

fn accepts(source: &Source) -> bool {
    certify_figure_caption(&source.table, &source.elements, FRAME, &SecurityLimits::default()).is_some()
}

#[test]
fn certifies_the_exact_docling_caption_from_all_its_owned_words() {
    let source = source();
    let proof = certify_figure_caption(&source.table, &source.elements, FRAME, &SecurityLimits::default())
        .expect("all six intact source lines are unjustifiably split into cells");
    assert_eq!(proof.lines().len(), 6);
    assert_eq!(proof.lines().first().copied(), Some(source.elements[0].text.as_str()));
    assert_eq!(proof.lines().last().copied(), Some("fields of each table cell (C)."));
    assert_eq!(
        proof
            .lines()
            .iter()
            .map(|line| line.split_whitespace().count())
            .sum::<usize>(),
        85
    );
}

fn set_gutter(source: &mut Source, right_start: u32) {
    let mut offset = 0;
    for cells in source.table.cells.iter_mut().take(5) {
        let line = &source.elements[offset];
        let row = line.text.split_whitespace().collect::<Vec<_>>();
        let OcrBoundingGeometry::Rectangle { left: start, width, .. } = line.geometry else {
            panic!("axis-aligned synthetic source line")
        };
        let end = start + width;
        *cells = vec![row[..2].join(" "), row[2..].join(" "), String::new()];
        let word_count = row.len();
        let next = offset + word_count + 1;
        for (index, element) in source.elements[offset + 1..next].iter_mut().enumerate() {
            let OcrBoundingGeometry::Rectangle { left, width, .. } = &mut element.geometry else {
                panic!("axis-aligned synthetic source word")
            };
            (*left, *width) = match index {
                0 => (start, 35),
                1 => (start + 45, 550 - (start + 45)),
                _ => {
                    let span = (end - right_start) / (word_count - 2) as u32;
                    let start = right_start + (index - 2) as u32 * span;
                    let word_width = if index == word_count - 1 {
                        end - start
                    } else {
                        span - 10
                    };
                    (start, word_width)
                }
            };
        }
        offset = next;
    }
}

#[test]
fn keeps_a_grid_with_a_recurring_gutter_even_with_figure_like_text() {
    for right_start in [563, 650] {
        let mut source = source();
        set_gutter(&mut source, right_start);
        assert!(!accepts(&source), "stable gutter at x550–{right_start}");
    }
}

#[test]
fn keeps_a_grid_when_two_physical_lines_belong_to_one_wrapped_cell() {
    let mut source = source();
    let continuation = source.table.cells.remove(2).join(" ");
    source.table.cells[1][1].push(' ');
    source.table.cells[1][1].push_str(continuation.trim());
    assert!(!accepts(&source));
}

#[test]
fn refuses_duplicate_line_owners_and_missing_word_occurrences() {
    let mut duplicated = source();
    duplicated.elements.push(duplicated.elements[0].clone());
    assert!(!accepts(&duplicated));

    let mut missing = source();
    let first_word = missing
        .elements
        .iter()
        .position(|element| element.level == OcrElementLevel::Word)
        .unwrap();
    let OcrBoundingGeometry::Rectangle { top, .. } = &mut missing.elements[first_word].geometry else {
        panic!("exact source word rectangle")
    };
    *top = 2000;
    assert!(
        !accepts(&missing),
        "an identical word outside the table cannot fill its missing occurrence"
    );
}

#[test]
fn refuses_mismatched_literal_order_and_source_frame() {
    let mut source = source();
    source.table.cells[0].swap(0, 1);
    assert!(!accepts(&source));
    let source = self::source();
    assert!(
        certify_figure_caption(
            &source.table,
            &source.elements,
            (2127, FRAME.1),
            &SecurityLimits::default(),
        )
        .is_none()
    );
}

#[test]
fn refuses_evidence_above_the_bounded_work_budget() {
    let source = source();
    let tiny = SecurityLimits {
        max_iterations: 1,
        ..SecurityLimits::default()
    };
    assert!(certify_figure_caption(&source.table, &source.elements, FRAME, &tiny).is_none());
}

#[test]
fn refuses_more_than_the_fixed_capture_limit_even_with_a_generous_caller_budget() {
    let mut source = source();
    let word = source.elements[1].clone();
    for index in 0..(1025 - source.elements.len()) {
        let mut outside = word.clone();
        outside.geometry = OcrBoundingGeometry::Rectangle {
            left: (index % 250) as u32 * 10,
            top: 1000 + (index / 250) as u32 * 10,
            width: 4,
            height: 5,
        };
        source.elements.push(outside);
    }
    assert_eq!(source.elements.len(), 1025);
    assert!(!accepts(&source));
}
