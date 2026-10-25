//! Table-related types for document extraction.

use super::document_structure::TextAnnotation;
use super::extraction::BoundingBox;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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

    /// Header cells for this fragment, i.e. the first row of `cells`.
    ///
    /// Populated even when this fragment's own header row was merged away or
    /// physically lives in a sibling fragment (see `table_id`), so a single
    /// fragment is interpretable in isolation. `None` when no header row
    /// could be determined.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub columns: Option<Vec<String>>,

    /// Inline annotations on individual cells' text, for the cells that have one.
    ///
    /// Sparse and flat, like [`Table::cell_styles`]. `cells` keeps its text: an equation in a DOCX
    /// cell is written into it as `$...$`, the same text a paragraph keeps, and an entry here marks
    /// which bytes of it are math, so a renderer writes math instead of escaped dollars without
    /// guessing from the text. Only a producer that knows what a span is emits an entry; today that
    /// is DOCX equations.
    ///
    /// Entries are only emitted for annotated cells, so an ordinary table serialises exactly as it
    /// did before. Indices are into `cells`, byte ranges into that cell's text. A pass that rewrites
    /// a cell's text must drop or move that cell's entries. ~keep
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cell_annotations: Vec<TableCellAnnotation>,
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

/// An inline annotation on one table cell's text, located by grid position.
///
/// One entry per annotation, not a per-cell grid: the nested grid shape marshals badly across the
/// FFI bindings, and the data is sparse. The annotation is the same [`TextAnnotation`] a paragraph
/// carries, with byte offsets into `cells[row][col]`. See [`Table::cell_annotations`]. ~keep
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct TableCellAnnotation {
    /// Zero-indexed row of the annotated cell.
    pub row: u32,
    /// Zero-indexed column of the annotated cell.
    pub col: u32,
    /// The annotation, with byte offsets into the cell's text.
    pub annotation: TextAnnotation,
}

impl Table {
    /// Drop the annotations of the cell at `(row, col)`, whose text a pass has rewritten.
    pub(crate) fn drop_cell_annotations(&mut self, row: usize, col: usize) {
        self.cell_annotations
            .retain(|entry| (entry.row as usize, entry.col as usize) != (row, col));
    }

    /// `cell_annotations` grouped by cell, so a reader of every cell does not rescan the list.
    pub(crate) fn annotations_by_cell(&self) -> BTreeMap<(u32, u32), Vec<TextAnnotation>> {
        let mut by_cell: BTreeMap<(u32, u32), Vec<TextAnnotation>> = BTreeMap::new();
        for entry in &self.cell_annotations {
            by_cell
                .entry((entry.row, entry.col))
                .or_default()
                .push(entry.annotation.clone());
        }
        by_cell
    }
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

    fn math_entry(row: u32, col: u32, end: u32) -> TableCellAnnotation {
        TableCellAnnotation {
            row,
            col,
            annotation: TextAnnotation {
                start: 0,
                end,
                kind: crate::types::AnnotationKind::math(false),
            },
        }
    }

    #[test]
    fn should_serialise_a_table_without_math_unchanged() {
        let plain = Table {
            cells: vec![vec!["$5".to_string()]],
            ..Default::default()
        };
        let json = serde_json::to_string(&plain).unwrap();
        assert!(!json.contains("cell_annotations"), "{json}");

        let old = r#"{"cells":[["$x$"]],"markdown":"","page_number":1}"#;
        assert!(serde_json::from_str::<Table>(old).unwrap().cell_annotations.is_empty());

        let annotated = Table {
            cells: vec![vec!["$x$".to_string()]],
            cell_annotations: vec![math_entry(0, 0, 3)],
            ..Default::default()
        };
        let value = serde_json::to_value(&annotated).unwrap();
        assert_eq!(
            value["cell_annotations"],
            serde_json::json!([{"row": 0, "col": 0, "annotation": {"start": 0, "end": 3,
                "kind": {"annotation_type": "custom", "name": "xberg.math", "value": "inline"}}}])
        );
        let restored: Table = serde_json::from_value(value).unwrap();
        assert_eq!(restored.cell_annotations, annotated.cell_annotations);
    }

    #[test]
    fn should_drop_only_the_rewritten_cells_annotations() {
        let mut table = Table {
            cell_annotations: vec![
                math_entry(0, 0, 3),
                math_entry(0, 1, 3),
                math_entry(0, 0, 5),
                math_entry(1, 0, 3),
            ],
            ..Default::default()
        };
        table.drop_cell_annotations(0, 0);
        assert_eq!(table.cell_annotations, [math_entry(0, 1, 3), math_entry(1, 0, 3)]);
        assert_eq!(
            table.annotations_by_cell().keys().copied().collect::<Vec<_>>(),
            [(0, 1), (1, 0)]
        );
    }
}
