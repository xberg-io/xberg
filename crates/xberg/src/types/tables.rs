//! Table-related types for document extraction.

use super::extraction::BoundingBox;
use serde::{Deserialize, Serialize};

/// Extracted table structure.
///
/// Represents a table detected and extracted from a document (PDF, image, etc.).
/// Tables are converted to both structured cell data and Markdown format.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct Table {
    /// Table cells as a 2D vector (rows × columns)
    pub cells: Vec<Vec<String>>,
    /// Markdown representation of the table
    pub markdown: String,
    /// Page number where the table was found (1-indexed)
    pub page_number: u32,
    /// Bounding box of the table on the page (PDF coordinates: x0=left, y0=bottom, x1=right, y1=top).
    /// Only populated for PDF-extracted tables when position data is available.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub bounding_box: Option<BoundingBox>,

    /// Stable identifier shared by every `tables[]` entry that represents a
    /// fragment of the same physical table.
    ///
    /// Assigned deterministically by the extraction pipeline (e.g. a
    /// sequential `"table-N"` in document order); never derived from
    /// randomness or wall-clock time, so the same input document always
    /// produces the same ids. Consumers can use it to reconcile the markdown
    /// blocks in `content` / `pages[].content` / `chunks[].content` with the
    /// structured entries in `tables[]`. `None` when the extractor did not
    /// assign one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table_id: Option<String>,

    /// Header cells for this fragment, i.e. the first row of `cells`.
    ///
    /// Populated even when this fragment's own header row was merged away or
    /// physically lives in a sibling fragment (see `table_id`), so a single
    /// fragment is interpretable in isolation. `None` when no header row
    /// could be determined.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub columns: Option<Vec<String>>,
}

/// Individual table cell with content and optional styling.
///
/// Future extension point for rich table support with cell-level metadata.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct TableCell {
    /// Cell content as text
    pub content: String,
    /// Row span (number of rows this cell spans)
    #[serde(default = "default_span")]
    pub row_span: u32,
    /// Column span (number of columns this cell spans)
    #[serde(default = "default_span")]
    pub col_span: u32,
    /// Whether this is a header cell
    #[serde(default)]
    pub is_header: bool,
}

fn default_span() -> u32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_with_bounding_box_serialization() {
        let table = Table {
            cells: vec![
                vec!["A".to_string(), "B".to_string()],
                vec!["C".to_string(), "D".to_string()],
            ],
            markdown: "| A | B |\n|---|---|\n| C | D |".to_string(),
            page_number: 1,
            bounding_box: Some(BoundingBox {
                x0: 50.0,
                y0: 100.0,
                x1: 500.0,
                y1: 700.0,
            }),
            ..Default::default()
        };

        let json = serde_json::to_string(&table).unwrap();
        assert!(json.contains("\"bounding_box\""));
        assert!(json.contains("\"x0\":50.0"));
        assert!(json.contains("\"y1\":700.0"));

        let deserialized: Table = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.page_number, 1);
        assert!(deserialized.bounding_box.is_some());
        let bbox = deserialized.bounding_box.unwrap();
        assert_eq!(bbox.x0, 50.0);
        assert_eq!(bbox.y0, 100.0);
        assert_eq!(bbox.x1, 500.0);
        assert_eq!(bbox.y1, 700.0);
    }

    #[test]
    fn test_table_without_bounding_box_serialization() {
        let table = Table {
            cells: vec![vec!["X".to_string()]],
            markdown: "| X |".to_string(),
            page_number: 2,
            bounding_box: None,
            ..Default::default()
        };

        let json = serde_json::to_string(&table).unwrap();
        assert!(!json.contains("bounding_box"));

        let deserialized: Table = serde_json::from_str(&json).unwrap();
        assert!(deserialized.bounding_box.is_none());
    }

    #[test]
    fn test_table_deserialization_without_bounding_box_field() {
        let json = r#"{"cells":[["A","B"]],"markdown":"| A | B |","page_number":1}"#;
        let table: Table = serde_json::from_str(json).unwrap();
        assert!(table.bounding_box.is_none());
        assert_eq!(table.page_number, 1);
    }

    #[test]
    fn test_table_bounding_box_clone_and_debug() {
        let table = Table {
            cells: vec![],
            markdown: String::new(),
            page_number: 1,
            bounding_box: Some(BoundingBox {
                x0: 10.0,
                y0: 20.0,
                x1: 30.0,
                y1: 40.0,
            }),
            ..Default::default()
        };

        let cloned = table.clone();
        assert_eq!(cloned.bounding_box, table.bounding_box);

        let debug = format!("{:?}", table);
        assert!(debug.contains("bounding_box"));
    }

    #[test]
    fn test_table_bounding_box_values_preserved() {
        let original = Table {
            cells: vec![
                vec!["Header1".to_string(), "Header2".to_string()],
                vec!["Val1".to_string(), "Val2".to_string()],
            ],
            markdown: "| Header1 | Header2 |\n|---|---|\n| Val1 | Val2 |".to_string(),
            page_number: 3,
            bounding_box: Some(BoundingBox {
                x0: 72.0,
                y0: 200.5,
                x1: 540.0,
                y1: 600.75,
            }),
            ..Default::default()
        };

        let json_value = serde_json::to_value(&original).unwrap();
        let deserialized: Table = serde_json::from_value(json_value).unwrap();

        assert_eq!(deserialized.cells, original.cells);
        assert_eq!(deserialized.markdown, original.markdown);
        assert_eq!(deserialized.page_number, original.page_number);
        assert_eq!(deserialized.bounding_box, original.bounding_box);
    }

    #[test]
    fn test_table_id_and_columns_serialize_when_present() {
        let table = Table {
            cells: vec![
                vec!["Name".to_string(), "Age".to_string()],
                vec!["Alice".to_string(), "30".to_string()],
            ],
            markdown: "| Name | Age |\n|---|---|\n| Alice | 30 |".to_string(),
            page_number: 1,
            table_id: Some("table-1".to_string()),
            columns: Some(vec!["Name".to_string(), "Age".to_string()]),
            ..Default::default()
        };

        let json = serde_json::to_string(&table).unwrap();
        assert!(json.contains("\"table_id\":\"table-1\""));
        assert!(json.contains("\"columns\":[\"Name\",\"Age\"]"));

        let deserialized: Table = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.table_id.as_deref(), Some("table-1"));
        assert_eq!(deserialized.columns, Some(vec!["Name".to_string(), "Age".to_string()]));
    }

    #[test]
    fn test_table_id_and_columns_omitted_when_absent() {
        let table = Table {
            cells: vec![vec!["X".to_string()]],
            markdown: "| X |".to_string(),
            page_number: 1,
            ..Default::default()
        };

        let json = serde_json::to_string(&table).unwrap();
        assert!(!json.contains("table_id"));
        assert!(!json.contains("columns"));

        let deserialized: Table = serde_json::from_str(&json).unwrap();
        assert!(deserialized.table_id.is_none());
        assert!(deserialized.columns.is_none());
    }

    #[test]
    fn test_table_deserialization_without_table_id_or_columns_fields() {
        let json = r#"{"cells":[["A","B"]],"markdown":"| A | B |","page_number":1}"#;
        let table: Table = serde_json::from_str(json).unwrap();
        assert!(table.table_id.is_none());
        assert!(table.columns.is_none());
    }
}
