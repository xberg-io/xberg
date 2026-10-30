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
    /// Bounding box of the table's position. Only populated when position data is
    /// available from the producing extractor.
    ///
    /// The coordinate space depends on how the table was produced, and callers must
    /// know which route produced a given `Table` before interpreting this field:
    ///
    /// - Tables extracted from a PDF's native content, and tables recognized on a
    ///   scanned PDF page that went through xberg's OCR pipeline (`--force-ocr` /
    ///   `--ocr-scanned-pages`), are in **PDF points with a bottom-left origin**
    ///   (x0=left, y0=bottom, x1=right, y1=top; y increases upward). For the OCR
    ///   case, the pipeline rescales the backend's raw pixel output into this space
    ///   before it reaches `Table::bounding_box` — see
    ///   `rescale_ocr_bboxes_to_page_points` in `extractors::pdf::ocr`.
    /// - Tables detected by OCR on a standalone image with no backing PDF page (for
    ///   example extracting a bare PNG/JPEG/TIFF) are in **raster pixel coordinates
    ///   with a top-left origin** (x0=left, y0=top, x1=right, y1=bottom; y increases
    ///   downward) — the same convention the OCR backend (Tesseract, PaddleOCR,
    ///   candle-based backends) or the layout detector reported them in. There is no
    ///   PDF page geometry to rescale into for this case, so the raw pixel box is
    ///   passed through unchanged.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[cfg_attr(feature = "api", schema(nullable = false))]
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
    ///
    /// Today, same-page fragments of one physical table are already merged
    /// into a single `tables[]` entry before ids are assigned (see PDF table
    /// stitching), so in practice `table_id` is unique per entry rather than
    /// shared across several. A table split across a page boundary is
    /// intentionally *not* linked — its per-page pieces get separate ids.
    /// Sharing one id across page-boundary fragments is a known possible
    /// future extension, not implemented yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table_id: Option<String>,

    /// Paragraph styles carried by individual cells, for the cells that have one.
    ///
    /// Sparse and flat on purpose. A DOCX banner row -- row 0, one cell spanning the grid,
    /// styled `Heading1`..`Heading6` -- is what Word's navigation pane and a `TOC` field treat
    /// as the document outline, but as a table cell it reached consumers as anonymous text
    /// (GH#1587). `cells` keeps the bare text: prefixing it with `#` would put a markdown
    /// heading inside a table cell, which is invalid where it lands and would change text every
    /// existing consumer already reads. This list is the signal instead.
    ///
    /// Entries are only emitted for cells that actually carry a style, so an ordinary table
    /// serialises exactly as it did before. Indices are into `cells`. ~keep
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cell_styles: Vec<TableCellStyle>,

    /// Declared column header labels for this fragment.
    ///
    /// These may describe the first row of `cells` or be inherited from a sibling
    /// fragment (see `table_id`) whose header row is absent here, so a single
    /// fragment is interpretable in isolation. `None` when no column headers
    /// could be determined.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub columns: Option<Vec<String>>,
}

impl Table {
    pub(crate) fn has_header_row(&self) -> bool {
        let (Some(columns), Some(first_row)) = (self.columns.as_ref(), self.cells.first()) else {
            return false;
        };
        !columns.is_empty()
            && first_row.len() == columns.len()
            && columns.iter().any(|column| !column.trim().is_empty())
            && first_row
                .iter()
                .zip(columns)
                .all(|(cell, column)| cell.trim() == column.trim())
    }
}

/// The paragraph style a single table cell's text carries, located by grid position.
///
/// Flat rather than a nested `Vec<Vec<Option<..>>>`: the nested shape marshals badly across the
/// FFI bindings, and the data is sparse anyway. See [`Table::cell_styles`]. ~keep
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct TableCellStyle {
    /// Zero-indexed row of the cell this style belongs to.
    pub row: u32,
    /// Zero-indexed column of the cell this style belongs to.
    pub col: u32,
    /// Outline level 1-6 when the style resolves to a heading, otherwise `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heading_level: Option<u8>,
    /// Human-readable style name, e.g. `heading 2`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_name: Option<String>,
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
    fn physical_header_requires_matching_nonblank_declared_cells() {
        type HeaderCase<'a> = (&'a [&'a str], Option<&'a [&'a str]>, bool);
        let cases: &[HeaderCase<'_>] = &[
            (&["A", "B"], None, false),
            (&[], Some(&[]), false),
            (&[" ", ""], Some(&["", " "]), false),
            (&["A", "B"], Some(&["Known A", "Known B"]), false),
            (&["A", "B"], Some(&["A"]), false),
            (&["A"], Some(&["A", "B"]), false),
            (&["A", "B"], Some(&["A", "B"]), true),
            (&[" A ", " B"], Some(&["A", "B "]), true),
            (&["", "B"], Some(&["", "B"]), true),
        ];
        for (first_row, columns, expected) in cases {
            let table = Table {
                cells: vec![first_row.iter().map(|cell| (*cell).to_owned()).collect()],
                columns: columns.map(|columns| columns.iter().map(|column| (*column).to_owned()).collect()),
                ..Default::default()
            };
            assert_eq!(table.has_header_row(), *expected, "{table:?}");
        }
        assert!(
            !Table {
                columns: Some(vec!["A".to_owned()]),
                ..Default::default()
            }
            .has_header_row()
        );
    }

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
