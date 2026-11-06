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

#[test]
fn geometry_and_raster_checks_share_the_same_iteration_budget() {
    let source = source();
    let defaults = SecurityLimits::default();
    let standalone = certify_figure_caption(&source.table, &source.elements, FRAME, &defaults).unwrap();
    let (_, post_crop) = standalone.paint.crop_work(FRAME, &defaults).unwrap();
    let joint_charge = defaults.max_iterations - post_crop;
    let tight = SecurityLimits {
        max_iterations: joint_charge - 1,
        ..defaults
    };
    let separately_valid = certify_figure_caption(&source.table, &source.elements, FRAME, &tight).unwrap();
    let (crop, _) = standalone.paint.crop_work(FRAME, &tight).unwrap();
    assert!(crop.right > crop.left);
    assert!(separately_valid.paint.crop_work(FRAME, &tight).is_none());
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

fn assert_exact_lines(source: &Source, frame: (u32, u32)) {
    let proof = certify_figure_caption(&source.table, &source.elements, frame, &SecurityLimits::default())
        .expect("complete caption rows with exact cell and word ownership");
    let expected: Vec<_> = source
        .elements
        .iter()
        .filter(|element| element.level == OcrElementLevel::Line)
        .map(|line| line.text.as_str())
        .collect();
    assert_eq!(proof.lines(), expected);
    assert_eq!(
        proof.lines().iter().flat_map(|line| line.split_whitespace()).count(),
        85
    );
}

#[test]
fn wrapped_caption_ownership_is_independent_of_the_label_and_punctuation() {
    for (label, number) in [("Fig.", "4:"), ("Fig", "4:"), ("Table", "4:"), ("Figure", "4")] {
        let mut source = source();
        let original = "Figure 4:";
        let replacement = format!("{label} {number}");
        assert_eq!(source.table.cells[0][0], original);
        assert_eq!(source.elements[1].text, "Figure");
        assert_eq!(source.elements[2].text, "4:");
        source.table.cells[0][0].clone_from(&replacement);
        source.elements[0].text = source.elements[0].text.replacen(original, &replacement, 1);
        source.elements[1].text = label.to_owned();
        source.elements[2].text = number.to_owned();
        assert_exact_lines(&source, FRAME);
    }

    let mut source = source();
    let last_cell = &mut source.table.cells[5][0];
    assert!(last_cell.ends_with("(C)."));
    last_cell.pop();
    let last_line = source
        .elements
        .iter_mut()
        .rfind(|element| element.level == OcrElementLevel::Line)
        .unwrap();
    assert!(last_line.text.ends_with("(C)."));
    last_line.text.pop();
    let last_word = source.elements.last_mut().unwrap();
    assert_eq!(last_word.text, "(C).");
    last_word.text.pop();
    assert_exact_lines(&source, FRAME);
}

#[test]
fn accepts_a_short_full_line_edge_at_equivalent_raster_scales() {
    for scale in [1, 2, 3] {
        let mut source = source();
        let shortened_line = source
            .elements
            .iter()
            .enumerate()
            .filter(|(_, element)| element.level == OcrElementLevel::Line)
            .nth(2)
            .unwrap()
            .0;
        let shortened_word = source.elements[shortened_line + 1..]
            .iter()
            .position(|element| element.level == OcrElementLevel::Line)
            .unwrap()
            + shortened_line;
        assert_eq!(source.elements[shortened_word].text, "(B),");
        for (index, element) in source.elements.iter_mut().enumerate() {
            let OcrBoundingGeometry::Rectangle {
                left,
                top,
                width,
                height,
            } = &mut element.geometry
            else {
                panic!("axis-aligned source geometry");
            };
            if index == shortened_line || index == shortened_word {
                *width -= 5;
            }
            *left *= scale;
            *top *= scale;
            *width *= scale;
            *height *= scale;
        }
        let region = source.table.bounding_box.as_mut().unwrap();
        region.left *= scale;
        region.top *= scale;
        region.right *= scale;
        region.bottom *= scale;
        assert_exact_lines(&source, (FRAME.0 * scale, FRAME.1 * scale));
    }
}

#[test]
fn keeps_staggered_unruled_cells_without_an_intact_intervening_line() {
    let mut source = source();
    for (row, word_count) in [(2, 4), (3, 8)] {
        let cells = &mut source.table.cells[row];
        assert!(cells[1..].iter().all(String::is_empty));
        let words: Vec<_> = cells[0].split_whitespace().collect();
        let first = words[..word_count].join(" ");
        let second = words[word_count..].join(" ");
        cells[0] = first;
        cells[1] = second;
    }
    assert!(!accepts(&source));
}

#[test]
fn keeps_a_cell_boundary_without_a_whole_word_crossing_its_gap() {
    let mut source = source();
    let word = source
        .elements
        .iter_mut()
        .find(|element| element.level == OcrElementLevel::Word && element.text == "guarantees")
        .unwrap();
    let OcrBoundingGeometry::Rectangle { left, width, .. } = &mut word.geometry else {
        panic!("axis-aligned source word");
    };
    assert_eq!((*left, *left + *width), (572, 750));
    *width = 620 - *left;
    assert!(!accepts(&source));
}
