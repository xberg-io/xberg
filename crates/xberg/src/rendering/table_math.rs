//! Private provenance for equations inside table-cell strings.

#[cfg(any(feature = "office", test))]
use std::ops::Range;

use ahash::AHashMap;
use serde::{Deserialize, Serialize};

#[cfg(any(feature = "office", test))]
use crate::types::document_structure::AnnotationKind;
use crate::types::document_structure::TextAnnotation;
use crate::types::internal_builder::InternalDocumentBuilder;

pub(crate) const TABLE_CELL_MATH_ATTRIBUTE: &str = "xberg:internal:table-cell-math";

#[cfg(any(feature = "office", test))]
type MathCell = (String, Vec<(Range<usize>, bool)>);
type CellAnnotations = AHashMap<(usize, usize), Vec<TextAnnotation>>;

#[derive(Serialize, Deserialize)]
struct CellMathAnnotation {
    row: usize,
    col: usize,
    annotation: TextAnnotation,
}

#[cfg(any(feature = "office", test))]
pub(crate) fn split_cell_grid(grid: Vec<Vec<MathCell>>) -> (Vec<Vec<String>>, Option<String>) {
    let mut annotations = Vec::new();
    let cells = grid
        .into_iter()
        .enumerate()
        .map(|(row, values)| {
            values
                .into_iter()
                .enumerate()
                .map(|(col, (text, spans))| {
                    annotations.extend(spans.into_iter().map(|(range, display)| CellMathAnnotation {
                        row,
                        col,
                        annotation: TextAnnotation {
                            start: range.start as u32,
                            end: range.end as u32,
                            kind: AnnotationKind::math(display),
                        },
                    }));
                    text
                })
                .collect()
        })
        .collect();
    let encoded = (!annotations.is_empty())
        .then(|| serde_json::to_string(&annotations).expect("table math annotations contain only serializable values"));
    (cells, encoded)
}

pub(crate) fn read_annotations(
    attributes: Option<&AHashMap<String, String>>,
    cells: &[Vec<String>],
) -> CellAnnotations {
    let Some(encoded) = attributes.and_then(|values| values.get(TABLE_CELL_MATH_ATTRIBUTE)) else {
        return AHashMap::new();
    };
    // ~keep A cached private attribute is untrusted; malformed provenance must leave cell text literal.
    let Ok(records) = serde_json::from_str::<Vec<CellMathAnnotation>>(encoded) else {
        return AHashMap::new();
    };
    let mut annotations = AHashMap::new();
    for record in records {
        let Some(text) = cells.get(record.row).and_then(|row| row.get(record.col)) else {
            return AHashMap::new();
        };
        if !super::common::valid_math_annotation(text, &record.annotation) {
            return AHashMap::new();
        }
        annotations
            .entry((record.row, record.col))
            .or_insert_with(Vec::new)
            .push(record.annotation);
    }
    annotations
}

pub(crate) fn render_table_markdown(cells: &[Vec<String>], encoded: Option<&str>) -> String {
    let Some(encoded) = encoded else {
        return super::common::render_table_markdown(cells);
    };
    let mut builder = InternalDocumentBuilder::new("docx");
    let element_index = builder.push_table_from_cells(cells, None, None);
    builder.merge_attribute(element_index, TABLE_CELL_MATH_ATTRIBUTE, encoded);
    super::render_markdown(&builder.build())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encoded_inline_cell(text: &str, range: Range<usize>) -> (Vec<Vec<String>>, String) {
        let (cells, encoded) = split_cell_grid(vec![vec![(text.to_string(), vec![(range, false)])]]);
        (cells, encoded.unwrap())
    }

    #[test]
    fn should_create_math_nodes_only_from_provenance_gh2256() {
        let text = "É $cash$ $\\alpha$";
        let start = "É $cash$ ".len();
        let (cells, encoded) = encoded_inline_cell(text, start..text.len());
        let mut builder = InternalDocumentBuilder::new("docx");
        let element = builder.push_table_from_cells(&cells, None, None);
        builder.set_table_math_annotations(element, encoded);
        let document = builder.build();
        let arena = comrak::Arena::new();
        let ast = super::super::comrak_bridge::build_comrak_ast(&document, &arena);
        let math: Vec<_> = ast
            .descendants()
            .filter_map(|node| match &node.data.borrow().value {
                comrak::nodes::NodeValue::Math(value) => Some(value.literal.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(math, ["\\alpha"]);
        let rendered = super::super::render_markdown(&document);
        let parsed_arena = comrak::Arena::new();
        let parsed = comrak::parse_document(&parsed_arena, &rendered, &super::super::markdown::comrak_options());
        let reparsed_math: Vec<_> = parsed
            .descendants()
            .filter_map(|node| match &node.data.borrow().value {
                comrak::nodes::NodeValue::Math(value) => Some(value.literal.clone()),
                _ => None,
            })
            .collect();
        assert!(reparsed_math.iter().any(|literal| literal == "\\alpha"), "{rendered}");
        assert!(document.elements[0].public_attributes().is_none());
        let restored = serde_json::from_str(&serde_json::to_string(&document).unwrap()).unwrap();
        assert_eq!(
            super::super::render_markdown(&document),
            super::super::render_markdown(&restored)
        );
    }

    #[test]
    fn should_keep_a_pipe_in_cell_math_inside_its_cell() {
        let text = "$|x|$";
        let (cells, encoded) = encoded_inline_cell(text, 0..text.len());
        let mut builder = InternalDocumentBuilder::new("docx");
        let element = builder.push_table_from_cells(
            &[
                vec![cells[0][0].clone(), "G".to_string()],
                vec!["H".to_string(), "b".to_string()],
            ],
            None,
            None,
        );
        builder.set_table_math_annotations(element, encoded);
        let rendered = super::super::render_markdown(&builder.build());
        let arena = comrak::Arena::new();
        let parsed = comrak::parse_document(&arena, &rendered, &super::super::markdown::comrak_options());
        let cells = parsed
            .descendants()
            .filter(|node| matches!(node.data.borrow().value, comrak::nodes::NodeValue::TableCell))
            .count();
        assert_eq!(cells, 4, "{rendered}");
        assert!(rendered.contains("$\\|x\\|$"), "{rendered}");
        let second_row_cell = parsed
            .descendants()
            .filter(|node| matches!(node.data.borrow().value, comrak::nodes::NodeValue::TableCell))
            .nth(3)
            .and_then(|cell| cell.first_child())
            .map(|child| format!("{:?}", child.data.borrow().value));
        assert!(
            second_row_cell.is_some_and(|value| value.contains("\"b\"")),
            "{rendered}"
        );
    }

    #[test]
    fn should_reject_invalid_cached_cell_math_provenance_gh2256() {
        let text = "é $x$";
        let (cells, encoded) = encoded_inline_cell(text, "é ".len()..text.len());
        let mut attributes = AHashMap::new();
        attributes.insert(TABLE_CELL_MATH_ATTRIBUTE.to_string(), encoded.clone());
        assert_eq!(read_annotations(Some(&attributes), &cells).len(), 1);
        let valid: serde_json::Value = serde_json::from_str(&encoded).unwrap();
        for (field, invalid) in [("row", 1), ("col", 1), ("start", 1), ("end", 99)] {
            let mut value = valid.clone();
            value[0]["annotation"][field] = invalid.into();
            if field == "row" || field == "col" {
                value[0][field] = invalid.into();
            }
            attributes.insert(TABLE_CELL_MATH_ATTRIBUTE.to_string(), value.to_string());
            assert!(read_annotations(Some(&attributes), &cells).is_empty(), "{field}");
        }
        attributes.insert(TABLE_CELL_MATH_ATTRIBUTE.to_string(), "not JSON".to_string());
        assert!(read_annotations(Some(&attributes), &cells).is_empty());
    }

    #[test]
    fn should_not_treat_plain_dollar_text_as_cell_math_gh2256() {
        let cells = vec![vec!["$\\alpha$ and $5".to_string()]];
        assert!(read_annotations(None, &cells).is_empty());
        assert_eq!(
            render_table_markdown(&cells, None),
            super::super::common::render_table_markdown(&cells)
        );
    }
}
