//! Private script annotations for plain table-cell strings.

use ahash::AHashMap;
use serde::{Deserialize, Serialize};

use crate::types::{AnnotationKind, TextAnnotation};

pub(crate) const TABLE_CELL_SCRIPTS_ATTRIBUTE: &str = "xberg:internal:table-cell-scripts";

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct CellScriptAnnotation {
    pub row: usize,
    pub col: usize,
    pub annotation: TextAnnotation,
}

pub(crate) fn read_annotations(
    attributes: Option<&AHashMap<String, String>>,
    cells: &[Vec<String>],
) -> AHashMap<(usize, usize), Vec<TextAnnotation>> {
    let Some(encoded) = attributes.and_then(|values| values.get(TABLE_CELL_SCRIPTS_ATTRIBUTE)) else {
        return AHashMap::new();
    };
    // ~keep Cached internal metadata must not turn malformed ranges or literal text into formatting.
    let Ok(records) = serde_json::from_str::<Vec<CellScriptAnnotation>>(encoded) else {
        return AHashMap::new();
    };
    let mut annotations = AHashMap::new();
    for record in records {
        let Some(text) = cells.get(record.row).and_then(|row| row.get(record.col)) else {
            return AHashMap::new();
        };
        let annotation = record.annotation;
        if !matches!(annotation.kind, AnnotationKind::Superscript | AnnotationKind::Subscript)
            || annotation.start >= annotation.end
            || text.get(annotation.start as usize..annotation.end as usize).is_none()
        {
            return AHashMap::new();
        }
        annotations
            .entry((record.row, record.col))
            .or_insert_with(Vec::new)
            .push(annotation);
    }
    annotations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_reject_invalid_cached_table_script_ranges_gh2275() {
        let cells = vec![vec!["é109".to_string()]];
        let valid = CellScriptAnnotation {
            row: 0,
            col: 0,
            annotation: crate::types::builder::superscript(4, 5),
        };
        let encoded = serde_json::to_string(&[valid]).unwrap();
        let mut attributes = AHashMap::from_iter([(TABLE_CELL_SCRIPTS_ATTRIBUTE.to_string(), encoded.clone())]);
        assert_eq!(read_annotations(Some(&attributes), &cells).len(), 1);
        let valid: serde_json::Value = serde_json::from_str(&encoded).unwrap();
        for (field, invalid) in [("row", 1), ("col", 1), ("start", 1), ("end", 99), ("end", 4)] {
            let mut value = valid.clone();
            if field == "row" || field == "col" {
                value[0][field] = invalid.into();
            } else {
                value[0]["annotation"][field] = invalid.into();
            }
            attributes.insert(TABLE_CELL_SCRIPTS_ATTRIBUTE.to_string(), value.to_string());
            assert!(
                read_annotations(Some(&attributes), &cells).is_empty(),
                "{field}={invalid}"
            );
        }
        attributes.insert(TABLE_CELL_SCRIPTS_ATTRIBUTE.to_string(), "not JSON".to_string());
        assert!(read_annotations(Some(&attributes), &cells).is_empty());
    }
}
