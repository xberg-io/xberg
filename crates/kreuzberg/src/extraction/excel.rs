//! Excel and spreadsheet extraction functions.
//!
//! This module provides Excel file parsing using the `calamine` library.
//! Supports both modern Office Open XML formats (.xlsx, .xlsm, .xlam, .xltm, .xlsb)
//! and legacy binary formats (.xls, .xla), as well as OpenDocument spreadsheets (.ods).
//!
//! # Features
//!
//! - **Multiple formats**: XLSX, XLSM, XLS, XLSB, ODS
//! - **Sheet extraction**: Reads all sheets from workbook
//! - **Markdown conversion**: Converts spreadsheet data to Markdown tables
//! - **Office metadata**: Extracts core properties, custom properties (when `office` feature enabled)
//! - **Error handling**: Distinguishes between format errors and true I/O errors
//!
//! # Example
//!
//! ```rust
//! use kreuzberg::extraction::excel::read_excel_file;
//!
//! # fn example() -> kreuzberg::Result<()> {
//! let workbook = read_excel_file("data.xlsx")?;
//!
//! println!("Sheet count: {}", workbook.sheets.len());
//! for sheet in &workbook.sheets {
//!     println!("Sheet: {}", sheet.name);
//! }
//! # Ok(())
//! # }
//! ```
use calamine::{Data, Range, Reader, open_workbook_auto};
use std::collections::HashMap;
use std::fmt::Write as FmtWrite;
use std::io::Cursor;
use std::path::Path;

use crate::error::{KreuzbergError, Result};
use crate::extraction::capacity;
use crate::types::{ExcelSheet, ExcelWorkbook};

#[cfg(feature = "office")]
use crate::extraction::office_metadata::{
    extract_core_properties, extract_custom_properties, extract_xlsx_app_properties,
};
#[cfg(feature = "office")]
use serde_json::Value;

pub fn read_excel_file(file_path: &str) -> Result<ExcelWorkbook> {
    #[cfg(feature = "office")]
    let office_metadata = if file_path.to_lowercase().ends_with(".xlsx")
        || file_path.to_lowercase().ends_with(".xlsm")
        || file_path.to_lowercase().ends_with(".xlam")
        || file_path.to_lowercase().ends_with(".xltm")
    {
        extract_xlsx_office_metadata_from_file(file_path).ok()
    } else {
        None
    };

    #[cfg(not(feature = "office"))]
    let office_metadata: Option<HashMap<String, String>> = None;

    // We analyze the error and only wrap format errors, letting real IO errors bubble up ~keep
    let workbook = match open_workbook_auto(Path::new(file_path)) {
        Ok(wb) => wb,
        Err(calamine::Error::Io(io_err)) => {
            if io_err.kind() == std::io::ErrorKind::InvalidData {
                return Err(KreuzbergError::parsing(format!(
                    "Cannot detect Excel file format: {}",
                    io_err
                )));
            }
            // Real IO error - bubble up unchanged ~keep
            return Err(io_err.into());
        }
        Err(e) => return Err(KreuzbergError::parsing(format!("Failed to parse Excel file: {}", e))),
    };

    process_workbook(workbook, office_metadata)
}

pub fn read_excel_bytes(data: &[u8], file_extension: &str) -> Result<ExcelWorkbook> {
    #[cfg(feature = "office")]
    let office_metadata = match file_extension.to_lowercase().as_str() {
        ".xlsx" | ".xlsm" | ".xlam" | ".xltm" => extract_xlsx_office_metadata_from_bytes(data).ok(),
        _ => None,
    };

    #[cfg(not(feature = "office"))]
    let office_metadata: Option<HashMap<String, String>> = None;

    let cursor = Cursor::new(data);

    match file_extension.to_lowercase().as_str() {
        ".xlsx" | ".xlsm" | ".xlam" | ".xltm" => {
            let workbook = calamine::Xlsx::new(cursor)
                .map_err(|e| KreuzbergError::parsing(format!("Failed to parse XLSX: {}", e)))?;
            process_workbook(workbook, office_metadata)
        }
        ".xls" | ".xla" => {
            let workbook = calamine::Xls::new(cursor)
                .map_err(|e| KreuzbergError::parsing(format!("Failed to parse XLS: {}", e)))?;
            process_workbook(workbook, office_metadata)
        }
        ".xlsb" => {
            let workbook = calamine::Xlsb::new(cursor)
                .map_err(|e| KreuzbergError::parsing(format!("Failed to parse XLSB: {}", e)))?;
            process_workbook(workbook, office_metadata)
        }
        ".ods" => {
            let workbook = calamine::Ods::new(cursor)
                .map_err(|e| KreuzbergError::parsing(format!("Failed to parse ODS: {}", e)))?;
            process_workbook(workbook, office_metadata)
        }
        _ => Err(KreuzbergError::parsing(format!(
            "Unsupported file extension: {}",
            file_extension
        ))),
    }
}

fn process_workbook<RS, R>(mut workbook: R, office_metadata: Option<HashMap<String, String>>) -> Result<ExcelWorkbook>
where
    RS: std::io::Read + std::io::Seek,
    R: Reader<RS>,
{
    let sheet_names = workbook.sheet_names();

    let mut sheets = Vec::with_capacity(sheet_names.len());

    for name in &sheet_names {
        if let Ok(range) = workbook.worksheet_range(name) {
            sheets.push(process_sheet(name, &range));
        }
    }

    let metadata = extract_metadata(&workbook, &sheet_names, office_metadata);

    Ok(ExcelWorkbook { sheets, metadata })
}

#[inline]
fn process_sheet(name: &str, range: &Range<Data>) -> ExcelSheet {
    let (rows, cols) = range.get_size();
    let cell_count = range.used_cells().count();

    let estimated_capacity = 50 + (cols * 20) + (rows * cols * 12);

    if rows == 0 || cols == 0 {
        let markdown = format!("## {}\n\n*Empty sheet*", name);
        ExcelSheet {
            name: name.to_owned(),
            markdown,
            row_count: rows,
            col_count: cols,
            cell_count,
            table_cells: None,
        }
    } else {
        let (markdown, table_cells) = generate_markdown_and_cells(name, range, estimated_capacity);
        ExcelSheet {
            name: name.to_owned(),
            markdown,
            row_count: rows,
            col_count: cols,
            cell_count,
            table_cells: Some(table_cells),
        }
    }
}

/// Generate both markdown and extracted cells in a single pass.
///
/// This function produces both the markdown representation and the structured
/// cell data simultaneously, avoiding the expensive markdown re-parsing that
/// was previously done in `sheets_to_tables()`.
///
/// Returns (markdown, table_cells) where table_cells is a 2D vector of strings.
fn generate_markdown_and_cells(sheet_name: &str, range: &Range<Data>, capacity: usize) -> (String, Vec<Vec<String>>) {
    let rows: Vec<_> = range.rows().collect();
    if rows.is_empty() {
        let result_capacity = 50 + sheet_name.len();
        let mut result = String::with_capacity(result_capacity);
        write!(result, "## {}\n\n*No data*", sheet_name).unwrap();
        return (result, Vec::new());
    }

    let header = &rows[0];
    let header_len = header.len();
    let row_count = rows.len();

    let table_capacity = capacity::estimate_table_markdown_capacity(row_count, header_len);

    let mut exact_size = 16 + sheet_name.len();

    exact_size += 2 + (header_len * 2);
    exact_size += header_len * 10;

    exact_size += 5 + (header_len * 5);

    exact_size += (row_count - 1) * (5 + header_len * 15);

    let mut markdown = String::with_capacity(exact_size.max(table_capacity).max(capacity));
    let mut cells: Vec<Vec<String>> = Vec::with_capacity(row_count);

    write!(markdown, "## {}\n\n", sheet_name).unwrap();

    let mut header_cells = Vec::with_capacity(header_len);
    markdown.push_str("| ");
    for (i, cell) in header.iter().enumerate() {
        if i > 0 {
            markdown.push_str(" | ");
        }
        let cell_str = format_cell_to_string(cell);
        header_cells.push(cell_str.clone());

        if cell_str.contains('|') || cell_str.contains('\\') {
            escape_markdown_into(&mut markdown, &cell_str);
        } else {
            markdown.push_str(&cell_str);
        }
    }
    markdown.push_str(" |\n");
    cells.push(header_cells);

    markdown.push_str("| ");
    for i in 0..header_len {
        if i > 0 {
            markdown.push_str(" | ");
        }
        markdown.push_str("---");
    }
    markdown.push_str(" |\n");

    for row in rows.iter().skip(1) {
        let mut row_cells = Vec::with_capacity(header_len);
        markdown.push_str("| ");
        for i in 0..header_len {
            if i > 0 {
                markdown.push_str(" | ");
            }
            if let Some(cell) = row.get(i) {
                let cell_str = format_cell_to_string(cell);
                row_cells.push(cell_str.clone());

                if cell_str.contains('|') || cell_str.contains('\\') {
                    escape_markdown_into(&mut markdown, &cell_str);
                } else {
                    markdown.push_str(&cell_str);
                }
            } else {
                row_cells.push(String::new());
            }
        }
        markdown.push_str(" |\n");
        cells.push(row_cells);
    }

    (markdown, cells)
}

/// Convert a Data cell to its string representation.
///
/// This helper function is shared between markdown generation and cell extraction
/// to ensure byte-identical output.
#[inline]
fn format_cell_to_string(data: &Data) -> String {
    match data {
        Data::Empty => String::new(),
        Data::String(s) => s.clone(),
        Data::Float(f) => {
            if f.fract() == 0.0 {
                format!("{:.1}", f)
            } else {
                format!("{}", f)
            }
        }
        Data::Int(i) => format!("{}", i),
        Data::Bool(b) => {
            if *b {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        Data::DateTime(dt) => {
            if let Some(datetime) = dt.as_datetime() {
                format!("{}", datetime.format("%Y-%m-%d %H:%M:%S"))
            } else {
                format!("{:?}", dt)
            }
        }
        Data::Error(e) => format!("#ERR: {:?}", e),
        Data::DateTimeIso(s) => s.clone(),
        Data::DurationIso(s) => format!("DURATION: {}", s),
    }
}

#[inline]
fn escape_markdown_into(buffer: &mut String, s: &str) {
    for ch in s.chars() {
        match ch {
            '|' => buffer.push_str("\\|"),
            '\\' => buffer.push_str("\\\\"),
            _ => buffer.push(ch),
        }
    }
}

fn extract_metadata<RS, R>(
    workbook: &R,
    sheet_names: &[String],
    office_metadata: Option<HashMap<String, String>>,
) -> HashMap<String, String>
where
    RS: std::io::Read + std::io::Seek,
    R: Reader<RS>,
{
    let mut metadata = HashMap::with_capacity(4);

    let sheet_count = sheet_names.len();
    metadata.insert("sheet_count".to_owned(), sheet_count.to_string());

    let sheet_names_str = if sheet_count <= 5 {
        sheet_names.join(", ")
    } else {
        let mut result = String::with_capacity(100);
        for (i, name) in sheet_names.iter().take(5).enumerate() {
            if i > 0 {
                result.push_str(", ");
            }
            result.push_str(name);
        }
        write!(result, ", ... ({} total)", sheet_count).unwrap();
        result
    };
    metadata.insert("sheet_names".to_owned(), sheet_names_str);

    let _workbook_metadata = workbook.metadata();

    if let Some(office_meta) = office_metadata {
        for (key, value) in office_meta {
            metadata.insert(key, value);
        }
    }

    metadata
}

pub fn excel_to_markdown(workbook: &ExcelWorkbook) -> String {
    let total_capacity: usize = workbook.sheets.iter().map(|sheet| sheet.markdown.len() + 2).sum();

    let mut result = String::with_capacity(total_capacity);

    for (i, sheet) in workbook.sheets.iter().enumerate() {
        if i > 0 {
            result.push_str("\n\n");
        }
        let sheet_content = sheet.markdown.trim_end();
        result.push_str(sheet_content);
    }

    result
}

#[cfg(feature = "office")]
fn extract_xlsx_office_metadata_from_file(file_path: &str) -> Result<HashMap<String, String>> {
    use std::fs::File;
    use zip::ZipArchive;

    // OSError/RuntimeError must bubble up - system errors need user reports ~keep
    let file = File::open(file_path)?;

    let mut archive =
        ZipArchive::new(file).map_err(|e| KreuzbergError::parsing(format!("Failed to open ZIP archive: {}", e)))?;

    extract_xlsx_office_metadata_from_archive(&mut archive)
}

#[cfg(feature = "office")]
fn extract_xlsx_office_metadata_from_bytes(data: &[u8]) -> Result<HashMap<String, String>> {
    use zip::ZipArchive;

    let cursor = Cursor::new(data);
    let mut archive =
        ZipArchive::new(cursor).map_err(|e| KreuzbergError::parsing(format!("Failed to open ZIP archive: {}", e)))?;

    extract_xlsx_office_metadata_from_archive(&mut archive)
}

#[cfg(feature = "office")]
fn extract_xlsx_office_metadata_from_archive<R: std::io::Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
) -> Result<HashMap<String, String>> {
    let mut metadata = HashMap::new();

    if let Ok(core) = extract_core_properties(archive) {
        if let Some(title) = core.title {
            metadata.insert("title".to_string(), title);
        }
        if let Some(creator) = core.creator {
            metadata.insert("creator".to_string(), creator.clone());
            metadata.insert("created_by".to_string(), creator);
        }
        if let Some(subject) = core.subject {
            metadata.insert("subject".to_string(), subject);
        }
        if let Some(keywords) = core.keywords {
            metadata.insert("keywords".to_string(), keywords);
        }
        if let Some(description) = core.description {
            metadata.insert("description".to_string(), description);
        }
        if let Some(modified_by) = core.last_modified_by {
            metadata.insert("modified_by".to_string(), modified_by);
        }
        if let Some(created) = core.created {
            metadata.insert("created_at".to_string(), created);
        }
        if let Some(modified) = core.modified {
            metadata.insert("modified_at".to_string(), modified);
        }
        if let Some(revision) = core.revision {
            metadata.insert("revision".to_string(), revision);
        }
        if let Some(category) = core.category {
            metadata.insert("category".to_string(), category);
        }
        if let Some(content_status) = core.content_status {
            metadata.insert("content_status".to_string(), content_status);
        }
        if let Some(language) = core.language {
            metadata.insert("language".to_string(), language);
        }
    }

    if let Ok(app) = extract_xlsx_app_properties(archive) {
        if !app.worksheet_names.is_empty() {
            metadata.insert("worksheet_names".to_string(), app.worksheet_names.join(", "));
        }
        if let Some(company) = app.company {
            metadata.insert("organization".to_string(), company);
        }
        if let Some(application) = app.application {
            metadata.insert("application".to_string(), application);
        }
        if let Some(app_version) = app.app_version {
            metadata.insert("application_version".to_string(), app_version);
        }
    }

    if let Ok(custom) = extract_custom_properties(archive) {
        for (key, value) in custom {
            let value_str = match value {
                Value::String(s) => s,
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                Value::Null => "null".to_string(),
                Value::Array(_) | Value::Object(_) => value.to_string(),
            };
            metadata.insert(format!("custom_{}", key), value_str);
        }
    }

    Ok(metadata)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_cell_to_string_basic() {
        assert_eq!(format_cell_to_string(&Data::Empty), "");
        assert_eq!(format_cell_to_string(&Data::String("test".to_owned())), "test");
        assert_eq!(format_cell_to_string(&Data::Float(42.0)), "42.0");
        assert_eq!(format_cell_to_string(&Data::Int(100)), "100");
        assert_eq!(format_cell_to_string(&Data::Bool(true)), "true");
    }

    #[test]
    fn test_escape_markdown_into() {
        let mut buffer = String::with_capacity(50);

        escape_markdown_into(&mut buffer, "normal text");
        assert_eq!(buffer, "normal text");

        buffer.clear();
        escape_markdown_into(&mut buffer, "text|with|pipes");
        assert_eq!(buffer, "text\\|with\\|pipes");

        buffer.clear();
        escape_markdown_into(&mut buffer, "back\\slash");
        assert_eq!(buffer, "back\\\\slash");
    }

    #[test]
    fn test_capacity_optimization() {
        let buffer = String::with_capacity(100);
        assert!(buffer.capacity() >= 100);
    }

    #[test]
    fn test_format_cell_value_datetime() {
        use calamine::{ExcelDateTime, ExcelDateTimeType};
        let dt = Data::DateTime(ExcelDateTime::new(49353.5, ExcelDateTimeType::DateTime, false));
        let result = format_cell_to_string(&dt);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_format_cell_value_error() {
        use calamine::CellErrorType;
        let result = format_cell_to_string(&Data::Error(CellErrorType::Div0));
        assert!(result.contains("#ERR"));
    }

    #[test]
    fn test_format_cell_value_datetime_iso() {
        let result = format_cell_to_string(&Data::DateTimeIso("2024-01-01T10:30:00".to_owned()));
        assert_eq!(result, "2024-01-01T10:30:00");
    }

    #[test]
    fn test_format_cell_value_duration_iso() {
        let result = format_cell_to_string(&Data::DurationIso("PT1H30M".to_owned()));
        assert_eq!(result, "DURATION: PT1H30M");
    }

    #[test]
    fn test_escape_markdown_combined() {
        let mut buffer = String::new();
        escape_markdown_into(&mut buffer, "text|with|pipes\\and\\slashes");
        assert_eq!(buffer, "text\\|with\\|pipes\\\\and\\\\slashes");
    }

    #[test]
    fn test_escape_markdown_no_special_chars() {
        let mut buffer = String::new();
        escape_markdown_into(&mut buffer, "plain text");
        assert_eq!(buffer, "plain text");
    }

    #[test]
    fn test_process_sheet_empty() {
        let range: Range<Data> = Range::empty();
        let sheet = process_sheet("EmptySheet", &range);

        assert_eq!(sheet.name, "EmptySheet");
        assert_eq!(sheet.row_count, 0);
        assert_eq!(sheet.col_count, 0);
        assert_eq!(sheet.cell_count, 0);
        assert!(sheet.markdown.contains("Empty sheet"));
    }

    #[test]
    fn test_process_sheet_single_cell() {
        let mut range: Range<Data> = Range::new((0, 0), (0, 0));
        range.set_value((0, 0), Data::String("Single Cell".to_owned()));

        let sheet = process_sheet("Sheet1", &range);

        assert_eq!(sheet.name, "Sheet1");
        assert_eq!(sheet.row_count, 1);
        assert_eq!(sheet.col_count, 1);
        assert_eq!(sheet.cell_count, 1);
        assert!(sheet.markdown.contains("Single Cell"));
    }

    #[test]
    fn test_process_sheet_with_data() {
        let mut range: Range<Data> = Range::new((0, 0), (2, 1));
        range.set_value((0, 0), Data::String("Name".to_owned()));
        range.set_value((0, 1), Data::String("Age".to_owned()));
        range.set_value((1, 0), Data::String("Alice".to_owned()));
        range.set_value((1, 1), Data::Int(30));
        range.set_value((2, 0), Data::String("Bob".to_owned()));
        range.set_value((2, 1), Data::Int(25));

        let sheet = process_sheet("People", &range);

        assert_eq!(sheet.name, "People");
        assert_eq!(sheet.row_count, 3);
        assert_eq!(sheet.col_count, 2);
        assert!(sheet.markdown.contains("Name"));
        assert!(sheet.markdown.contains("Age"));
        assert!(sheet.markdown.contains("Alice"));
        assert!(sheet.markdown.contains("30"));
    }

    #[test]
    fn test_generate_markdown_and_cells_empty() {
        let range: Range<Data> = Range::empty();
        let (markdown, cells) = generate_markdown_and_cells("Test", &range, 100);

        assert!(markdown.contains("## Test"));
        assert!(cells.is_empty());
    }

    #[test]
    fn test_generate_markdown_and_cells_with_data() {
        let mut range: Range<Data> = Range::new((0, 0), (1, 2));
        range.set_value((0, 0), Data::String("Col1".to_owned()));
        range.set_value((0, 1), Data::String("Col2".to_owned()));
        range.set_value((0, 2), Data::String("Col3".to_owned()));
        range.set_value((1, 0), Data::String("A".to_owned()));
        range.set_value((1, 1), Data::String("B".to_owned()));
        range.set_value((1, 2), Data::String("C".to_owned()));

        let (markdown, cells) = generate_markdown_and_cells("Sheet1", &range, 200);

        assert!(markdown.contains("## Sheet1"));
        assert!(markdown.contains("Col1"));
        assert!(markdown.contains("---"));
        assert_eq!(cells.len(), 2);
    }

    #[test]
    fn test_generate_markdown_and_cells_sparse() {
        let mut range: Range<Data> = Range::new((0, 0), (2, 2));
        range.set_value((0, 0), Data::String("A".to_owned()));
        range.set_value((0, 1), Data::String("B".to_owned()));
        range.set_value((0, 2), Data::String("C".to_owned()));
        range.set_value((1, 0), Data::String("X".to_owned()));
        range.set_value((1, 2), Data::String("Z".to_owned()));

        let (markdown, cells) = generate_markdown_and_cells("Sparse", &range, 200);

        assert!(markdown.contains("X"));
        assert!(markdown.contains("Z"));
        assert_eq!(cells.len(), 3);
    }

    #[test]
    fn test_format_cell_value_float_integer() {
        let result = format_cell_to_string(&Data::Float(100.0));
        assert_eq!(result, "100.0");
    }

    #[test]
    fn test_format_cell_value_float_decimal() {
        let result = format_cell_to_string(&Data::Float(12.3456));
        assert_eq!(result, "12.3456");
    }

    #[test]
    fn test_format_cell_value_bool_false() {
        let result = format_cell_to_string(&Data::Bool(false));
        assert_eq!(result, "false");
    }

    #[test]
    fn test_format_cell_escape_pipe() {
        let mut buffer = String::new();
        escape_markdown_into(&mut buffer, "value|with|pipes");
        assert_eq!(buffer, "value\\|with\\|pipes");
    }

    #[test]
    fn test_format_cell_escape_backslash() {
        let mut buffer = String::new();
        escape_markdown_into(&mut buffer, "path\\to\\file");
        assert_eq!(buffer, "path\\\\to\\\\file");
    }

    #[test]
    fn test_markdown_table_structure() {
        let mut range: Range<Data> = Range::new((0, 0), (2, 1));
        range.set_value((0, 0), Data::String("H1".to_owned()));
        range.set_value((0, 1), Data::String("H2".to_owned()));
        range.set_value((1, 0), Data::String("A".to_owned()));
        range.set_value((1, 1), Data::String("B".to_owned()));

        let (markdown, _cells) = generate_markdown_and_cells("Test", &range, 100);

        let lines: Vec<&str> = markdown.lines().collect();
        assert!(lines[0].contains("## Test"));
        assert!(lines[2].starts_with("| "));
        assert!(lines[3].contains("---"));
        assert!(lines[4].starts_with("| "));
    }

    #[test]
    fn test_process_sheet_metadata() {
        let mut range: Range<Data> = Range::new((0, 0), (9, 4));
        for row in 0..10 {
            for col in 0..5 {
                range.set_value((row, col), Data::String(format!("R{}C{}", row, col)));
            }
        }

        let sheet = process_sheet("Data", &range);

        assert_eq!(sheet.row_count, 10);
        assert_eq!(sheet.col_count, 5);
        assert_eq!(sheet.cell_count, 50);
    }
}
