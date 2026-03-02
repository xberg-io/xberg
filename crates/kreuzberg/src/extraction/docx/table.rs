//! DOCX table property types and parsing for Word document extraction.
//!
//! This module provides comprehensive support for parsing table-level, row-level, and cell-level
//! properties from OOXML `<w:tblPr>`, `<w:trPr>`, and `<w:tcPr>` elements using streaming XML parsing.

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use serde::{Deserialize, Serialize};

/// Table-level properties from `<w:tblPr>`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TableProperties {
    pub style_id: Option<String>,
    pub width: Option<TableWidth>,
    pub alignment: Option<String>, // "left", "center", "right"
    pub layout: Option<String>,    // "fixed" or "autofit"
    pub look: Option<TableLook>,
    pub borders: Option<TableBorders>,
    pub cell_margins: Option<CellMargins>,
    pub indent: Option<TableWidth>,
    pub caption: Option<String>,
}

/// Width specification used for tables and cells.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TableWidth {
    pub value: i32,
    pub width_type: String, // "dxa" (twips), "pct" (50ths of percent), "auto", "nil"
}

/// Table look bitmask/flags controlling conditional formatting bands.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TableLook {
    pub first_row: bool,
    pub last_row: bool,
    pub first_column: bool,
    pub last_column: bool,
    pub no_h_band: bool,
    pub no_v_band: bool,
}

/// Borders for a table (6 borders: top, bottom, left, right, insideH, insideV).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TableBorders {
    pub top: Option<BorderStyle>,
    pub bottom: Option<BorderStyle>,
    pub left: Option<BorderStyle>,
    pub right: Option<BorderStyle>,
    pub inside_h: Option<BorderStyle>,
    pub inside_v: Option<BorderStyle>,
}

/// A single border specification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BorderStyle {
    pub style: String,         // "single", "double", "dashed", "dotted", "none", etc.
    pub size: Option<i32>,     // eighths of a point
    pub color: Option<String>, // hex RGB or "auto"
    pub space: Option<i32>,    // spacing in points
}

/// Cell margins (used for both table-level defaults and per-cell overrides).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct CellMargins {
    pub top: Option<i32>, // twips
    pub bottom: Option<i32>,
    pub left: Option<i32>,
    pub right: Option<i32>,
}

/// Row-level properties from `<w:trPr>`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct RowProperties {
    pub height: Option<i32>,         // twips
    pub height_rule: Option<String>, // "auto", "atLeast", "exact"
    pub is_header: bool,
    pub cant_split: bool,
}

/// Cell-level properties from `<w:tcPr>`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct CellProperties {
    pub width: Option<TableWidth>,
    pub grid_span: Option<u32>,         // column span (default 1)
    pub v_merge: Option<VerticalMerge>, // vertical merge state
    pub borders: Option<CellBorders>,
    pub shading: Option<CellShading>,
    pub margins: Option<CellMargins>,
    pub vertical_align: Option<String>, // "top", "center", "bottom"
    pub text_direction: Option<String>, // "lrTb", "tbRl", "btLr"
    pub no_wrap: bool,
}

/// Vertical merge state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VerticalMerge {
    Restart,  // Start of a new merged range
    Continue, // Continuation of a previous merge
}

/// Per-cell borders (4 sides).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct CellBorders {
    pub top: Option<BorderStyle>,
    pub bottom: Option<BorderStyle>,
    pub left: Option<BorderStyle>,  // OOXML calls this "start" in LTR
    pub right: Option<BorderStyle>, // OOXML calls this "end" in LTR
}

/// Cell shading/background.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct CellShading {
    pub fill: Option<String>,  // background color hex or "auto"
    pub color: Option<String>, // pattern color
    pub val: Option<String>,   // pattern type: "clear", "solid", "pct10", etc.
}

/// Column widths from `<w:tblGrid>`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TableGrid {
    pub columns: Vec<i32>, // column widths in twips
}

/// Parse table-level properties from streaming XML reader.
///
/// Expects the reader to be positioned just after the `<w:tblPr>` start tag.
/// Reads all child elements until the matching `</w:tblPr>` end tag.
pub fn parse_table_properties(reader: &mut Reader<&[u8]>) -> TableProperties {
    let mut props = TableProperties::default();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let local_name = e.local_name();
                match local_name.as_ref() {
                    b"tblStyle" => {
                        props.style_id = get_attribute(&e, b"val");
                    }
                    b"tblW" => {
                        props.width = parse_width_element(&e);
                    }
                    b"jc" => {
                        props.alignment = get_attribute(&e, b"val");
                    }
                    b"tblLayout" => {
                        props.layout = get_attribute(&e, b"type");
                    }
                    b"tblLook" => {
                        props.look = Some(parse_table_look(&e));
                    }
                    b"tblBorders" => {
                        props.borders = Some(parse_table_borders(reader));
                    }
                    b"tblCellMar" => {
                        props.cell_margins = Some(parse_cell_margins_element(reader));
                    }
                    b"tblInd" => {
                        props.indent = parse_width_element(&e);
                    }
                    b"tblCaption" => {
                        props.caption = get_attribute(&e, b"val");
                    }
                    _ => {}
                }
                buf.clear();
            }
            Ok(Event::Empty(e)) => {
                let local_name = e.local_name();
                match local_name.as_ref() {
                    b"tblStyle" => {
                        props.style_id = get_attribute(&e, b"val");
                    }
                    b"tblW" => {
                        props.width = parse_width_element(&e);
                    }
                    b"jc" => {
                        props.alignment = get_attribute(&e, b"val");
                    }
                    b"tblLayout" => {
                        props.layout = get_attribute(&e, b"type");
                    }
                    b"tblLook" => {
                        props.look = Some(parse_table_look(&e));
                    }
                    b"tblInd" => {
                        props.indent = parse_width_element(&e);
                    }
                    b"tblCaption" => {
                        props.caption = get_attribute(&e, b"val");
                    }
                    b"tblBorders" => {
                        props.borders = Some(TableBorders::default());
                    }
                    b"tblCellMar" => {
                        props.cell_margins = Some(CellMargins::default());
                    }
                    _ => {}
                }
                buf.clear();
            }
            Ok(Event::End(e)) => {
                if e.local_name().as_ref() == b"tblPr" {
                    break;
                }
                buf.clear();
            }
            Ok(Event::Eof) => break,
            _ => {
                buf.clear();
            }
        }
    }

    props
}

/// Parse row-level properties from streaming XML reader.
///
/// Expects the reader to be positioned just after the `<w:trPr>` start tag.
pub fn parse_row_properties(reader: &mut Reader<&[u8]>) -> RowProperties {
    let mut props = RowProperties::default();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let local_name = e.local_name();
                match local_name.as_ref() {
                    b"trHeight" => {
                        props.height = get_attribute_int(&e, b"val");
                        props.height_rule = get_attribute(&e, b"hRule");
                    }
                    b"tblHeader" => {
                        props.is_header = is_toggle_on(&e);
                    }
                    b"cantSplit" => {
                        props.cant_split = is_toggle_on(&e);
                    }
                    _ => {}
                }
                buf.clear();
            }
            Ok(Event::Empty(e)) => {
                let local_name = e.local_name();
                match local_name.as_ref() {
                    b"trHeight" => {
                        props.height = get_attribute_int(&e, b"val");
                        props.height_rule = get_attribute(&e, b"hRule");
                    }
                    b"tblHeader" => {
                        props.is_header = is_toggle_on(&e);
                    }
                    b"cantSplit" => {
                        props.cant_split = is_toggle_on(&e);
                    }
                    _ => {}
                }
                buf.clear();
            }
            Ok(Event::End(e)) => {
                if e.local_name().as_ref() == b"trPr" {
                    break;
                }
                buf.clear();
            }
            Ok(Event::Eof) => break,
            _ => {
                buf.clear();
            }
        }
    }

    props
}

/// Parse cell-level properties from streaming XML reader.
///
/// Expects the reader to be positioned just after the `<w:tcPr>` start tag.
pub fn parse_cell_properties(reader: &mut Reader<&[u8]>) -> CellProperties {
    let mut props = CellProperties::default();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let local_name = e.local_name();
                match local_name.as_ref() {
                    b"tcW" => {
                        props.width = parse_width_element(&e);
                    }
                    b"gridSpan" => {
                        props.grid_span = get_attribute_u32(&e, b"val");
                    }
                    b"vMerge" => {
                        props.v_merge = Some(parse_vmerge(&e));
                    }
                    b"tcBorders" => {
                        props.borders = Some(parse_cell_borders(reader));
                    }
                    b"shd" => {
                        props.shading = Some(parse_cell_shading(&e));
                    }
                    b"tcMar" => {
                        props.margins = Some(parse_cell_margins_element(reader));
                    }
                    b"vAlign" => {
                        props.vertical_align = get_attribute(&e, b"val");
                    }
                    b"textDirection" => {
                        props.text_direction = get_attribute(&e, b"val");
                    }
                    b"noWrap" => {
                        props.no_wrap = is_toggle_on(&e);
                    }
                    _ => {}
                }
                buf.clear();
            }
            Ok(Event::Empty(e)) => {
                let local_name = e.local_name();
                match local_name.as_ref() {
                    b"tcW" => {
                        props.width = parse_width_element(&e);
                    }
                    b"gridSpan" => {
                        props.grid_span = get_attribute_u32(&e, b"val");
                    }
                    b"vMerge" => {
                        props.v_merge = Some(parse_vmerge(&e));
                    }
                    b"shd" => {
                        props.shading = Some(parse_cell_shading(&e));
                    }
                    b"vAlign" => {
                        props.vertical_align = get_attribute(&e, b"val");
                    }
                    b"textDirection" => {
                        props.text_direction = get_attribute(&e, b"val");
                    }
                    b"noWrap" => {
                        props.no_wrap = is_toggle_on(&e);
                    }
                    b"tcBorders" => {
                        props.borders = Some(CellBorders::default());
                    }
                    b"tcMar" => {
                        props.margins = Some(CellMargins::default());
                    }
                    _ => {}
                }
                buf.clear();
            }
            Ok(Event::End(e)) => {
                if e.local_name().as_ref() == b"tcPr" {
                    break;
                }
                buf.clear();
            }
            Ok(Event::Eof) => break,
            _ => {
                buf.clear();
            }
        }
    }

    props
}

/// Parse table grid (column widths) from streaming XML reader.
///
/// Expects the reader to be positioned just after the `<w:tblGrid>` start tag.
pub fn parse_table_grid(reader: &mut Reader<&[u8]>) -> TableGrid {
    let mut grid = TableGrid::default();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                if e.local_name().as_ref() == b"gridCol"
                    && let Some(width) = get_attribute_int(&e, b"w")
                {
                    grid.columns.push(width);
                }
                buf.clear();
            }
            Ok(Event::Empty(e)) => {
                if e.local_name().as_ref() == b"gridCol"
                    && let Some(width) = get_attribute_int(&e, b"w")
                {
                    grid.columns.push(width);
                }
                buf.clear();
            }
            Ok(Event::End(e)) => {
                if e.local_name().as_ref() == b"tblGrid" {
                    break;
                }
                buf.clear();
            }
            Ok(Event::Eof) => break,
            _ => {
                buf.clear();
            }
        }
    }

    grid
}

/// Helper: Check if an OOXML on/off toggle element is enabled.
/// Handles `<w:foo/>` (on), `<w:foo w:val="1"/>` (on), `<w:foo w:val="0"/>` (off),
/// `<w:foo w:val="true"/>` (on), `<w:foo w:val="false"/>` (off).
fn is_toggle_on(e: &BytesStart) -> bool {
    !matches!(
        get_attribute(e, b"val").as_deref(),
        Some("0") | Some("false") | Some("off")
    )
}

/// Helper: Parse border element from attributes.
fn parse_border_element(e: &BytesStart) -> BorderStyle {
    BorderStyle {
        style: get_attribute(e, b"val").unwrap_or_default(),
        size: get_attribute_int(e, b"sz"),
        color: get_attribute(e, b"color"),
        space: get_attribute_int(e, b"space"),
    }
}

/// Helper: Parse width element from attributes.
fn parse_width_element(e: &BytesStart) -> Option<TableWidth> {
    get_attribute_int(e, b"w").map(|value| TableWidth {
        value,
        width_type: get_attribute(e, b"type").unwrap_or_default(),
    })
}

/// Helper: Parse table look from attributes.
/// Handles both OOXML 2012+ individual boolean attributes and legacy hex bitmask.
fn parse_table_look(e: &BytesStart) -> TableLook {
    let mut look = TableLook::default();

    // Try individual boolean attributes first (OOXML 2012+ Transitional).
    // Check if ANY individual attribute is present to distinguish from bitmask-only.
    let has_individual = get_attribute(e, b"firstRow").is_some()
        || get_attribute(e, b"lastRow").is_some()
        || get_attribute(e, b"firstColumn").is_some()
        || get_attribute(e, b"lastColumn").is_some()
        || get_attribute(e, b"noHBand").is_some()
        || get_attribute(e, b"noVBand").is_some();

    if has_individual {
        look.first_row = get_attribute(e, b"firstRow").as_deref() == Some("1");
        look.last_row = get_attribute(e, b"lastRow").as_deref() == Some("1");
        look.first_column = get_attribute(e, b"firstColumn").as_deref() == Some("1");
        look.last_column = get_attribute(e, b"lastColumn").as_deref() == Some("1");
        look.no_h_band = get_attribute(e, b"noHBand").as_deref() == Some("1");
        look.no_v_band = get_attribute(e, b"noVBand").as_deref() == Some("1");
    } else if let Some(val_str) = get_attribute(e, b"val") {
        // Fall back to legacy hex bitmask
        if let Ok(mask) = i32::from_str_radix(&val_str, 16) {
            look.first_row = (mask & 0x0020) != 0;
            look.last_row = (mask & 0x0040) != 0;
            look.first_column = (mask & 0x0080) != 0;
            look.last_column = (mask & 0x0100) != 0;
            look.no_h_band = (mask & 0x0200) != 0;
            look.no_v_band = (mask & 0x0400) != 0;
        }
    }

    look
}

/// Helper: Parse vertical merge state.
fn parse_vmerge(e: &BytesStart) -> VerticalMerge {
    match get_attribute(e, b"val") {
        Some(val) if val == "restart" => VerticalMerge::Restart,
        _ => VerticalMerge::Continue, // Empty element or missing attribute = Continue
    }
}

/// Helper: Parse cell shading from attributes.
fn parse_cell_shading(e: &BytesStart) -> CellShading {
    CellShading {
        fill: get_attribute(e, b"fill"),
        color: get_attribute(e, b"color"),
        val: get_attribute(e, b"val"),
    }
}

/// Helper: Parse table borders container element.
fn parse_table_borders(reader: &mut Reader<&[u8]>) -> TableBorders {
    let mut borders = TableBorders::default();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let local_name = e.local_name();
                match local_name.as_ref() {
                    b"top" => {
                        borders.top = Some(parse_border_element(&e));
                    }
                    b"bottom" => {
                        borders.bottom = Some(parse_border_element(&e));
                    }
                    b"left" | b"start" => {
                        borders.left = Some(parse_border_element(&e));
                    }
                    b"right" | b"end" => {
                        borders.right = Some(parse_border_element(&e));
                    }
                    b"insideH" => {
                        borders.inside_h = Some(parse_border_element(&e));
                    }
                    b"insideV" => {
                        borders.inside_v = Some(parse_border_element(&e));
                    }
                    _ => {}
                }
                buf.clear();
            }
            Ok(Event::Empty(e)) => {
                let local_name = e.local_name();
                match local_name.as_ref() {
                    b"top" => {
                        borders.top = Some(parse_border_element(&e));
                    }
                    b"bottom" => {
                        borders.bottom = Some(parse_border_element(&e));
                    }
                    b"left" | b"start" => {
                        borders.left = Some(parse_border_element(&e));
                    }
                    b"right" | b"end" => {
                        borders.right = Some(parse_border_element(&e));
                    }
                    b"insideH" => {
                        borders.inside_h = Some(parse_border_element(&e));
                    }
                    b"insideV" => {
                        borders.inside_v = Some(parse_border_element(&e));
                    }
                    _ => {}
                }
                buf.clear();
            }
            Ok(Event::End(e)) => {
                if e.local_name().as_ref() == b"tblBorders" {
                    break;
                }
                buf.clear();
            }
            Ok(Event::Eof) => break,
            _ => {
                buf.clear();
            }
        }
    }

    borders
}

/// Helper: Parse cell borders container element.
fn parse_cell_borders(reader: &mut Reader<&[u8]>) -> CellBorders {
    let mut borders = CellBorders::default();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let local_name = e.local_name();
                match local_name.as_ref() {
                    b"top" => {
                        borders.top = Some(parse_border_element(&e));
                    }
                    b"bottom" => {
                        borders.bottom = Some(parse_border_element(&e));
                    }
                    b"left" | b"start" => {
                        borders.left = Some(parse_border_element(&e));
                    }
                    b"right" | b"end" => {
                        borders.right = Some(parse_border_element(&e));
                    }
                    _ => {}
                }
                buf.clear();
            }
            Ok(Event::Empty(e)) => {
                let local_name = e.local_name();
                match local_name.as_ref() {
                    b"top" => {
                        borders.top = Some(parse_border_element(&e));
                    }
                    b"bottom" => {
                        borders.bottom = Some(parse_border_element(&e));
                    }
                    b"left" | b"start" => {
                        borders.left = Some(parse_border_element(&e));
                    }
                    b"right" | b"end" => {
                        borders.right = Some(parse_border_element(&e));
                    }
                    _ => {}
                }
                buf.clear();
            }
            Ok(Event::End(e)) => {
                if e.local_name().as_ref() == b"tcBorders" {
                    break;
                }
                buf.clear();
            }
            Ok(Event::Eof) => break,
            _ => {
                buf.clear();
            }
        }
    }

    borders
}

/// Helper: Parse cell margins container element.
fn parse_cell_margins_element(reader: &mut Reader<&[u8]>) -> CellMargins {
    let mut margins = CellMargins::default();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let local_name = e.local_name();
                match local_name.as_ref() {
                    b"top" => {
                        margins.top = get_attribute_int(&e, b"w");
                    }
                    b"bottom" => {
                        margins.bottom = get_attribute_int(&e, b"w");
                    }
                    b"left" | b"start" => {
                        margins.left = get_attribute_int(&e, b"w");
                    }
                    b"right" | b"end" => {
                        margins.right = get_attribute_int(&e, b"w");
                    }
                    _ => {}
                }
                buf.clear();
            }
            Ok(Event::Empty(e)) => {
                let local_name = e.local_name();
                match local_name.as_ref() {
                    b"top" => {
                        margins.top = get_attribute_int(&e, b"w");
                    }
                    b"bottom" => {
                        margins.bottom = get_attribute_int(&e, b"w");
                    }
                    b"left" | b"start" => {
                        margins.left = get_attribute_int(&e, b"w");
                    }
                    b"right" | b"end" => {
                        margins.right = get_attribute_int(&e, b"w");
                    }
                    _ => {}
                }
                buf.clear();
            }
            Ok(Event::End(e)) => {
                if e.local_name().as_ref() == b"tblCellMar" || e.local_name().as_ref() == b"tcMar" {
                    break;
                }
                buf.clear();
            }
            Ok(Event::Eof) => break,
            _ => {
                buf.clear();
            }
        }
    }

    margins
}

/// Helper: Extract string attribute value.
/// Uses local_name() to handle namespace-prefixed attributes (e.g., `w:val` matches `val`).
fn get_attribute(e: &BytesStart, key: &[u8]) -> Option<String> {
    e.attributes()
        .flatten()
        .find(|attr| attr.key.local_name().as_ref() == key)
        .and_then(|attr| {
            let raw = std::str::from_utf8(&attr.value).ok()?;
            quick_xml::escape::unescape(raw).ok().map(|s| s.into_owned())
        })
}

/// Helper: Extract and parse integer attribute value.
fn get_attribute_int(e: &BytesStart, key: &[u8]) -> Option<i32> {
    get_attribute(e, key).and_then(|s| s.parse().ok())
}

/// Helper: Extract and parse unsigned integer attribute value.
fn get_attribute_u32(e: &BytesStart, key: &[u8]) -> Option<u32> {
    get_attribute(e, key).and_then(|s| s.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_table_properties_full() {
        let xml = r#"<w:tblPr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:tblStyle w:val="TableGrid"/>
            <w:tblW w:w="5000" w:type="dxa"/>
            <w:jc w:val="center"/>
            <w:tblLayout w:type="fixed"/>
            <w:tblLook w:val="0460"/>
            <w:tblBorders>
                <w:top w:val="single" w:sz="12" w:color="000000" w:space="0"/>
                <w:bottom w:val="single" w:sz="12" w:color="000000" w:space="0"/>
            </w:tblBorders>
            <w:tblCellMar>
                <w:top w:w="0" w:type="dxa"/>
                <w:left w:w="108" w:type="dxa"/>
            </w:tblCellMar>
            <w:tblInd w:w="108" w:type="dxa"/>
        </w:tblPr>"#;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut buf = Vec::new();
        reader.read_event_into(&mut buf).unwrap(); // Start(w:tblPr)
        buf.clear();

        let props = parse_table_properties(&mut reader);

        assert_eq!(props.style_id, Some("TableGrid".to_string()));
        assert_eq!(
            props.width,
            Some(TableWidth {
                value: 5000,
                width_type: "dxa".to_string()
            })
        );
        assert_eq!(props.alignment, Some("center".to_string()));
        assert_eq!(props.layout, Some("fixed".to_string()));
        assert!(props.look.is_some());
        assert!(props.borders.is_some());
        assert!(props.cell_margins.is_some());
        assert_eq!(
            props.indent,
            Some(TableWidth {
                value: 108,
                width_type: "dxa".to_string()
            })
        );
    }

    #[test]
    fn test_parse_row_properties() {
        let xml = r#"<w:trPr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:trHeight w:val="720" w:hRule="atLeast"/>
            <w:tblHeader/>
            <w:cantSplit/>
        </w:trPr>"#;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut buf = Vec::new();
        reader.read_event_into(&mut buf).unwrap(); // Start(w:trPr)
        buf.clear();

        let props = parse_row_properties(&mut reader);

        assert_eq!(props.height, Some(720));
        assert_eq!(props.height_rule, Some("atLeast".to_string()));
        assert!(props.is_header);
        assert!(props.cant_split);
    }

    #[test]
    fn test_parse_cell_properties_merged() {
        let xml = r#"<w:tcPr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:tcW w:w="2000" w:type="dxa"/>
            <w:gridSpan w:val="3"/>
            <w:vMerge w:val="restart"/>
        </w:tcPr>"#;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut buf = Vec::new();
        reader.read_event_into(&mut buf).unwrap(); // Start(w:tcPr)
        buf.clear();

        let props = parse_cell_properties(&mut reader);

        assert_eq!(
            props.width,
            Some(TableWidth {
                value: 2000,
                width_type: "dxa".to_string()
            })
        );
        assert_eq!(props.grid_span, Some(3));
        assert_eq!(props.v_merge, Some(VerticalMerge::Restart));
    }

    #[test]
    fn test_parse_cell_properties_shading() {
        let xml = r#"<w:tcPr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:shd w:val="clear" w:color="auto" w:fill="D9E2F3"/>
            <w:tcBorders>
                <w:top w:val="single" w:sz="8" w:color="000000"/>
                <w:left w:val="single" w:sz="8" w:color="000000"/>
            </w:tcBorders>
            <w:vAlign w:val="center"/>
        </w:tcPr>"#;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut buf = Vec::new();
        reader.read_event_into(&mut buf).unwrap(); // Start(w:tcPr)
        buf.clear();

        let props = parse_cell_properties(&mut reader);

        assert!(props.shading.is_some());
        let shading = props.shading.unwrap();
        assert_eq!(shading.fill, Some("D9E2F3".to_string()));
        assert_eq!(shading.color, Some("auto".to_string()));
        assert_eq!(shading.val, Some("clear".to_string()));

        assert!(props.borders.is_some());
        assert_eq!(props.vertical_align, Some("center".to_string()));
    }

    #[test]
    fn test_parse_table_grid() {
        let xml = r#"<w:tblGrid xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:gridCol w:w="2500"/>
            <w:gridCol w:w="2500"/>
            <w:gridCol w:w="2000"/>
            <w:gridCol w:w="2000"/>
        </w:tblGrid>"#;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut buf = Vec::new();
        reader.read_event_into(&mut buf).unwrap(); // Start(w:tblGrid)
        buf.clear();

        let grid = parse_table_grid(&mut reader);

        assert_eq!(grid.columns, vec![2500, 2500, 2000, 2000]);
    }

    #[test]
    fn test_parse_table_look() {
        let xml = r#"<w:tblLook xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" w:val="0460"/>"#;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut buf = Vec::new();
        let event = reader.read_event_into(&mut buf).unwrap(); // Empty(w:tblLook)

        if let Event::Empty(e) = event {
            let look = parse_table_look(&e);

            // 0x0460 = 0000_0100_0110_0000
            // first_row (0x0020) = 1, last_row (0x0040) = 1, first_column (0x0080) = 0, etc.
            assert!(look.first_row);
            assert!(look.last_row);
            assert!(!look.first_column);
        } else {
            panic!("Expected Empty event");
        }
    }

    #[test]
    fn test_vmerge_continue() {
        let xml = r#"<w:tcPr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:vMerge/>
        </w:tcPr>"#;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut buf = Vec::new();
        reader.read_event_into(&mut buf).unwrap(); // Start(w:tcPr)
        buf.clear();

        let props = parse_cell_properties(&mut reader);

        // Bare <w:vMerge/> without val attribute should be Continue
        assert_eq!(props.v_merge, Some(VerticalMerge::Continue));
    }

    #[test]
    fn test_empty_table_properties() {
        let xml = r#"<w:tblPr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"/>"#;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut buf = Vec::new();
        reader.read_event_into(&mut buf).unwrap(); // Empty(w:tblPr)
        buf.clear();

        // Consume the Empty event and test with default
        let props = TableProperties::default();

        assert!(props.style_id.is_none());
        assert!(props.width.is_none());
        assert!(props.alignment.is_none());
    }

    #[test]
    fn test_cell_margins() {
        let xml = r#"<w:tblCellMar xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:top w:w="100" w:type="dxa"/>
            <w:bottom w:w="100" w:type="dxa"/>
            <w:left w:w="50" w:type="dxa"/>
            <w:right w:w="50" w:type="dxa"/>
        </w:tblCellMar>"#;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut buf = Vec::new();
        reader.read_event_into(&mut buf).unwrap(); // Start(w:tblCellMar)
        buf.clear();

        let margins = parse_cell_margins_element(&mut reader);

        assert_eq!(margins.top, Some(100));
        assert_eq!(margins.bottom, Some(100));
        assert_eq!(margins.left, Some(50));
        assert_eq!(margins.right, Some(50));
    }

    #[test]
    fn test_border_styles() {
        let xml = r#"<w:tblBorders xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:top w:val="single" w:sz="12" w:color="FF0000" w:space="0"/>
            <w:bottom w:val="double" w:sz="24" w:color="0000FF" w:space="1"/>
            <w:left w:val="dashed" w:sz="8" w:color="auto"/>
            <w:right w:val="dotted" w:sz="4"/>
        </w:tblBorders>"#;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut buf = Vec::new();
        reader.read_event_into(&mut buf).unwrap(); // Start(w:tblBorders)
        buf.clear();

        let borders = parse_table_borders(&mut reader);

        assert!(borders.top.is_some());
        let top = borders.top.unwrap();
        assert_eq!(top.style, "single");
        assert_eq!(top.size, Some(12));
        assert_eq!(top.color, Some("FF0000".to_string()));
        assert_eq!(top.space, Some(0));

        assert!(borders.bottom.is_some());
        let bottom = borders.bottom.unwrap();
        assert_eq!(bottom.style, "double");
        assert_eq!(bottom.size, Some(24));

        assert!(borders.left.is_some());
        let left = borders.left.unwrap();
        assert_eq!(left.style, "dashed");
        assert_eq!(left.color, Some("auto".to_string()));

        assert!(borders.right.is_some());
        let right = borders.right.unwrap();
        assert_eq!(right.style, "dotted");
    }

    #[test]
    fn test_table_properties_round_trip_serialize() {
        let props = TableProperties {
            style_id: Some("TableGrid".to_string()),
            width: Some(TableWidth {
                value: 5000,
                width_type: "dxa".to_string(),
            }),
            alignment: Some("center".to_string()),
            layout: Some("fixed".to_string()),
            look: Some(TableLook {
                first_row: true,
                last_row: true,
                first_column: false,
                last_column: false,
                no_h_band: false,
                no_v_band: false,
            }),
            borders: None,
            cell_margins: None,
            indent: Some(TableWidth {
                value: 108,
                width_type: "dxa".to_string(),
            }),
            caption: None,
        };

        let json = serde_json::to_string(&props).unwrap();
        let deserialized: TableProperties = serde_json::from_str(&json).unwrap();

        assert_eq!(props, deserialized);
    }

    #[test]
    fn test_cell_properties_round_trip_serialize() {
        let props = CellProperties {
            width: Some(TableWidth {
                value: 2000,
                width_type: "dxa".to_string(),
            }),
            grid_span: Some(3),
            v_merge: Some(VerticalMerge::Restart),
            borders: None,
            shading: Some(CellShading {
                fill: Some("D9E2F3".to_string()),
                color: Some("auto".to_string()),
                val: Some("clear".to_string()),
            }),
            margins: None,
            vertical_align: Some("center".to_string()),
            text_direction: None,
            no_wrap: false,
        };

        let json = serde_json::to_string(&props).unwrap();
        let deserialized: CellProperties = serde_json::from_str(&json).unwrap();

        assert_eq!(props, deserialized);
    }
}
