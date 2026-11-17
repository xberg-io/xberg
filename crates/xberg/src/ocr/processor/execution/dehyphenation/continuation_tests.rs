use super::*;
use crate::ocr::hocr_parser::HOCR_BLOCK_ID_ATTRIBUTE;
use crate::types::{AnnotationKind, OcrElementLevel, TextAnnotation};

const PRECEDING: u32 = 0;
const MARKER: u32 = 1;
const PRIOR: u32 = 2;
const SUFFIX: u32 = 3;
const FOLLOWING: u32 = 4;
const BETWEEN: u32 = 5;
const RIGHT_PEER: u32 = 6;

fn sparse_body() -> (Vec<InternalElement>, Vec<HocrWordOccurrence>) {
    let rows = [
        ("! Earlier body words occupy the same column", [400, 267, 1301, 292]),
        ("x", [377, 317, 385, 325]),
        ("This example still ends with recog-", [401, 309, 1399, 334]),
        ("nition", [400, 342, 452, 358]),
        ("The independent spelling is recognition", [400, 380, 1335, 405]),
        ("A shorter line continues below", [400, 421, 1100, 446]),
        ("A different witness-word reaches the same edge", [400, 462, 1399, 487]),
    ];
    let mut elements = Vec::new();
    let mut words = Vec::new();
    for (text, [left, top, right, bottom]) in rows {
        let element = elements.len() as u32;
        let mut block = InternalElement::text(
            ElementKind::OcrText {
                level: OcrElementLevel::Block,
            },
            text,
            0,
        );
        block.page = Some(1);
        block.attributes = Some(
            [(HOCR_BLOCK_ID_ATTRIBUTE.to_owned(), element.to_string())]
                .into_iter()
                .collect(),
        );
        elements.push(block);
        let mut offset = 0;
        let tokens: Vec<_> = text.split(' ').collect();
        let body_count = (tokens.len() - usize::from(element == PRECEDING)) as u32;
        for (index, token) in tokens.into_iter().enumerate() {
            let bounds = if element == PRECEDING && index == 0 {
                [374, 275, 384, 283]
            } else {
                let index = index as u32 - u32::from(element == PRECEDING);
                [
                    left + (right - left) * index / body_count,
                    top,
                    left + (right - left) * (index + 1) / body_count,
                    bottom,
                ]
            };
            words.push(HocrWordOccurrence {
                id: words.len(),
                element,
                bytes: offset..offset + token.len(),
                bounds,
            });
            offset += token.len() + 1;
        }
    }
    (elements, words)
}

fn shift_line(words: &mut [HocrWordOccurrence], line: u32, dx: i32, dy: i32) {
    for word in words.iter_mut().filter(|word| word.element == line) {
        let [left, top, right, bottom] = word.bounds;
        word.bounds = [
            left.checked_add_signed(dx).unwrap(),
            top.checked_add_signed(dy).unwrap(),
            right.checked_add_signed(dx).unwrap(),
            bottom.checked_add_signed(dy).unwrap(),
        ];
    }
}

#[test]
fn sparse_terminal_wrap_joins_with_bracketed_body_geometry() {
    let (mut elements, words) = sparse_body();
    let mut expected = elements.clone();
    expected[PRIOR as usize].text = "This example still ends with recognition".to_owned();
    expected[SUFFIX as usize].text.clear();

    assert!(join_scanned_wraps(&mut elements, &words, 1700, true));
    assert_eq!(elements, expected);
}

#[test]
fn sparse_terminal_wrap_rejects_unproved_body_geometry() {
    for (line, dx, dy) in [
        (PRECEDING, 12, 0),
        (MARKER, 0, 25),
        (SUFFIX, -6, 0),
        (FOLLOWING, 0, -16),
        (BETWEEN, 12, 0),
        (RIGHT_PEER, 0, 24),
    ] {
        let (mut elements, mut words) = sparse_body();
        shift_line(&mut words, line, dx, dy);
        let expected = elements.clone();

        assert!(
            !join_scanned_wraps(&mut elements, &words, 1700, true),
            "line {line}, shift {dx}, {dy}"
        );
        assert_eq!(elements, expected);
    }
}

#[test]
fn sparse_terminal_wrap_stops_at_a_right_lane_overrun() {
    for line in [PRECEDING, BETWEEN] {
        let (mut elements, mut words) = sparse_body();
        let last = words.iter_mut().rfind(|word| word.element == line).unwrap();
        last.bounds[2] = 1550;
        let expected = elements.clone();

        assert!(
            !join_scanned_wraps(&mut elements, &words, 1700, true),
            "line {line} overruns the body lane"
        );
        assert_eq!(elements, expected);
    }
}

#[test]
fn sparse_terminal_wrap_requires_spelling_and_unclaimed_ownership() {
    for control in [
        "missing spelling",
        "lexical compound",
        "missing block",
        "same block",
        "annotation",
        "not sparse",
    ] {
        let (mut elements, words) = sparse_body();
        match control {
            "missing spelling" => {
                let line = &mut elements[FOLLOWING as usize].text;
                *line = line.replace("recognition", "publication");
            }
            "lexical compound" => {
                let line = &mut elements[RIGHT_PEER as usize].text;
                *line = line.replace("witness-word", "recog-nition");
            }
            "missing block" => elements[SUFFIX as usize].attributes = None,
            "same block" => elements[SUFFIX as usize].attributes = elements[PRIOR as usize].attributes.clone(),
            "annotation" => elements[SUFFIX as usize].annotations.push(TextAnnotation {
                start: 0,
                end: 6,
                kind: AnnotationKind::Bold,
            }),
            "not sparse" => (),
            _ => unreachable!(),
        }
        let expected = elements.clone();

        assert!(
            !join_scanned_wraps(&mut elements, &words, 1700, control != "not sparse"),
            "{control}"
        );
        assert_eq!(elements, expected);
    }
}
