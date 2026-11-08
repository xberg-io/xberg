use super::{Rect, SourceElement, certify_figure_caption, certify_figure_caption_rows, follows_wrapped_caption_shape};
use crate::extractors::security::SecurityLimits;
use crate::types::formats::{OcrTable, OcrTableBoundingBox};
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

fn shift_left_edge(source: &mut Source, lines: &[usize], delta: u32) {
    let mut current = None;
    let mut first_word = false;
    for element in &mut source.elements {
        let shift = match element.level {
            OcrElementLevel::Line => {
                current = Some(current.map_or(0, |index| index + 1));
                first_word = true;
                lines.contains(&current.unwrap())
            }
            OcrElementLevel::Word => {
                std::mem::take(&mut first_word) && current.is_some_and(|index| lines.contains(&index))
            }
            _ => false,
        };
        if shift {
            let OcrBoundingGeometry::Rectangle { left, width, .. } = &mut element.geometry else {
                panic!("axis-aligned source geometry");
            };
            *left += delta;
            *width -= delta;
        }
    }
}

#[test]
fn refuses_middle_lines_that_start_beyond_the_margin_tolerance() {
    let line_height = 39;
    let tolerance = line_height / 6;
    // The first and second lines already differ by 1 px, so delta + 1 is the measured offset.
    let within = tolerance - 1;
    let mut source = source();
    shift_left_edge(&mut source, &[1, 2, 3], within);
    assert!(accepts(&source), "middle lines offset by exactly the tolerance");
    for delta in [tolerance, line_height] {
        let mut source = self::source();
        shift_left_edge(&mut source, &[1, 2, 3], delta);
        assert!(!accepts(&source), "middle lines {delta} px further right are ragged");
    }
}

fn synthetic_line(left: u32, top: u32, right: u32, height: u32) -> SourceElement<'static> {
    SourceElement {
        text: "",
        bounds: Rect {
            left,
            top,
            right,
            bottom: top + height,
        },
    }
}

#[test]
fn margin_tolerance_never_drops_below_one_pixel_on_small_rasters() {
    let block = |last_left| {
        vec![
            synthetic_line(10, 0, 100, 5),
            synthetic_line(11, 6, 100, 5),
            synthetic_line(10, 12, 99, 5),
            synthetic_line(last_left, 18, 40, 5),
        ]
    };
    assert!(
        follows_wrapped_caption_shape(&block(11)),
        "1 px off the margin on a 5 px line"
    );
    assert!(
        !follows_wrapped_caption_shape(&block(12)),
        "2 px off the margin on a 5 px line"
    );
}

#[test]
fn one_short_line_does_not_shrink_the_margin_tolerance_of_the_block() {
    let block = vec![
        synthetic_line(100, 0, 1000, 60),
        synthetic_line(108, 65, 1000, 12),
        synthetic_line(100, 80, 1000, 60),
        synthetic_line(100, 145, 400, 60),
    ];
    assert!(
        follows_wrapped_caption_shape(&block),
        "8 px is within a sixth of the 60 px median line"
    );
}

struct Synthetic {
    cells: Vec<Vec<String>>,
    elements: Vec<OcrElement>,
}

fn synthetic_block(last_line: &[(&'static str, u32, u32)]) -> Synthetic {
    type Word = (&'static str, u32, u32);
    let rows: [(&[Word], usize); 4] = [
        (&[("a", 100, 300), ("b", 310, 660), ("c", 680, 1000)], 2),
        (
            &[("x", 100, 400), ("y", 410, 640), ("z", 650, 760), ("t", 770, 1000)],
            0,
        ),
        (&[("a", 100, 300), ("b", 310, 720), ("c", 740, 1000)], 2),
        (last_line, 0),
    ];
    let mut cells = Vec::new();
    let mut elements = Vec::new();
    let element = |text: &str, level, left: u32, top: u32, right: u32| OcrElement {
        text: text.to_owned(),
        geometry: OcrBoundingGeometry::Rectangle {
            left,
            top,
            width: right - left,
            height: 40,
        },
        level,
        page_number: 1,
        ..OcrElement::default()
    };
    for (index, (words, split)) in rows.iter().enumerate() {
        let top = 100 + index as u32 * 50;
        let text = words.iter().map(|word| word.0).collect::<Vec<_>>().join(" ");
        let (left, right) = (words[0].1, words[words.len() - 1].2);
        elements.push(element(&text, OcrElementLevel::Line, left, top, right));
        for &(word, l, r) in *words {
            elements.push(element(word, OcrElementLevel::Word, l, top, r));
        }
        let texts: Vec<&str> = words.iter().map(|word| word.0).collect();
        cells.push(if *split == 0 {
            vec![texts.join(" "), String::new()]
        } else {
            vec![texts[..*split].join(" "), texts[*split..].join(" ")]
        });
    }
    Synthetic { cells, elements }
}

fn certifies(block: &Synthetic) -> bool {
    let bounds = OcrTableBoundingBox {
        left: 100,
        top: 100,
        right: 1000,
        bottom: 290,
    };
    certify_figure_caption_rows(
        &block.cells,
        1,
        bounds,
        &block.elements,
        (1200, 400),
        &SecurityLimits::default(),
    )
    .is_some()
}

#[test]
fn refuses_a_two_column_block_whose_last_line_is_justified_to_the_full_width() {
    let ragged_last = synthetic_block(&[("p", 100, 400)]);
    assert!(
        certifies(&ragged_last),
        "the same block with a short last line is a wrapped caption"
    );
    let justified_last = synthetic_block(&[("p", 100, 500), ("q", 510, 1000)]);
    assert!(!certifies(&justified_last));
}
