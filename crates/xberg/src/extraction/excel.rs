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
//! ```ignore
//! use xberg::extraction::excel::read_excel_file;
//!
//! # fn example() -> xberg::Result<()> {
//! let workbook = read_excel_file("data.xlsx")?;
//!
//! println!("Sheet count: {}", workbook.sheets.len());
//! for sheet in &workbook.sheets {
//!     println!("Sheet: {}", sheet.name);
//! }
//! # Ok(())
//! # }
//! ```
use calamine::{Data, DataRef, Range, Reader, open_workbook_auto};
use std::collections::HashMap;
use std::fmt::Write as FmtWrite;
use std::io::{Cursor, Read, Seek};
use std::path::Path;

use crate::error::{Result, XbergError};
use crate::extraction::capacity;
use crate::types::revisions::{DocumentRevision, RevisionDelta, RevisionKind};
use crate::types::{ExcelSheet, ExcelWorkbook};

/// Maximum number of cells in a Range's bounding box before we consider it pathological.
/// This threshold is set to prevent OOM when processing files with sparse data at extreme
/// positions (e.g., Excel Solver files that have cells at A1 and XFD1048575).
///
/// 100 million cells at ~64 bytes each = ~6.4 GB, which is a reasonable upper limit.
const MAX_BOUNDING_BOX_CELLS: u64 = 100_000_000;

#[cfg(feature = "office")]
use crate::extraction::office_metadata::{
    extract_core_properties, extract_custom_properties, extract_xlsx_app_properties,
};
#[cfg(feature = "office")]
use serde_json::Value;

pub(crate) fn read_excel_file(file_path: &str) -> Result<ExcelWorkbook> {
    let lower_path = file_path.to_lowercase();

    #[cfg(feature = "office")]
    let office_metadata = if lower_path.ends_with(".xlsx")
        || lower_path.ends_with(".xlsm")
        || lower_path.ends_with(".xlam")
        || lower_path.ends_with(".xltm")
    {
        extract_xlsx_office_metadata_from_file(file_path).ok()
    } else {
        None
    };

    #[cfg(not(feature = "office"))]
    let office_metadata: Option<HashMap<String, String>> = None;

    // For standard XLSX-format files, use specialized handler with OOM protection
    if lower_path.ends_with(".xlsx") || lower_path.ends_with(".xlsm") || lower_path.ends_with(".xltm") {
        let file = std::fs::File::open(file_path)?;
        let workbook = calamine::Xlsx::new(std::io::BufReader::new(file))
            .map_err(|e| XbergError::parsing(format!("Failed to parse XLSX: {}", e)))?;
        let mut result = process_xlsx_workbook(workbook, office_metadata)?;
        result.revisions = extract_xlsx_revisions_from_file(file_path);
        return Ok(result);
    }

    // For .xlam (Excel add-in), try XLSX parsing but gracefully return empty workbook on failure
    if lower_path.ends_with(".xlam") {
        let file = std::fs::File::open(file_path)?;
        match calamine::Xlsx::new(std::io::BufReader::new(file)) {
            Ok(workbook) => {
                return process_xlsx_workbook(workbook, office_metadata);
            }
            Err(_) => {
                // .xlam files may not contain proper workbook data - return empty workbook
                return Ok(ExcelWorkbook {
                    sheets: vec![],
                    metadata: office_metadata.unwrap_or_default(),
                    revisions: None,
                });
            }
        }
    }

    // For .xla (legacy add-in), try XLS parsing but gracefully return empty workbook on failure
    if lower_path.ends_with(".xla") {
        let file = std::fs::File::open(file_path)?;
        match calamine::Xls::new(std::io::BufReader::new(file)) {
            Ok(workbook) => {
                return process_workbook(workbook, office_metadata);
            }
            Err(_) => {
                return Ok(ExcelWorkbook {
                    sheets: vec![],
                    metadata: office_metadata.unwrap_or_default(),
                    revisions: None,
                });
            }
        }
    }

    // For .xlsb (binary spreadsheet), use XLSB parser with error propagation
    if lower_path.ends_with(".xlsb") {
        let file = std::fs::File::open(file_path)?;
        let workbook = calamine::Xlsb::new(std::io::BufReader::new(file))
            .map_err(|e| XbergError::parsing(format!("Failed to parse XLSB: {}", e)))?;
        return process_workbook(workbook, office_metadata);
    }

    // For other formats, use open_workbook_auto
    let workbook = match open_workbook_auto(Path::new(file_path)) {
        Ok(wb) => wb,
        Err(calamine::Error::Io(io_err)) => {
            if io_err.kind() == std::io::ErrorKind::InvalidData {
                return Err(XbergError::parsing(format!(
                    "Cannot detect Excel file format: {}",
                    io_err
                )));
            }
            // Real IO error - bubble up unchanged ~keep
            return Err(io_err.into());
        }
        Err(e) => return Err(XbergError::parsing(format!("Failed to parse Excel file: {}", e))),
    };

    process_workbook(workbook, office_metadata)
}

pub(crate) fn read_excel_bytes(data: &[u8], file_extension: &str) -> Result<ExcelWorkbook> {
    #[cfg(feature = "office")]
    let office_metadata = match file_extension.to_lowercase().as_str() {
        ".xlsx" | ".xlsm" | ".xlam" | ".xltm" => extract_xlsx_office_metadata_from_bytes(data).ok(),
        _ => None,
    };

    #[cfg(not(feature = "office"))]
    let office_metadata: Option<HashMap<String, String>> = None;

    match file_extension.to_lowercase().as_str() {
        // Standard XLSX-format files: propagate errors
        ".xlsx" | ".xlsm" | ".xltm" => {
            let cursor = Cursor::new(data);
            let workbook =
                calamine::Xlsx::new(cursor).map_err(|e| XbergError::parsing(format!("Failed to parse XLSX: {}", e)))?;
            let mut result = process_xlsx_workbook(workbook, office_metadata)?;
            result.revisions = extract_xlsx_revisions_from_bytes(data);
            Ok(result)
        }
        // Exotic format: .xlam (Excel add-in) - may not contain proper workbook data
        ".xlam" => {
            let cursor = Cursor::new(data);
            match calamine::Xlsx::new(cursor) {
                Ok(workbook) => process_xlsx_workbook(workbook, office_metadata),
                Err(_) => {
                    // .xlam files may not contain proper workbook data - return empty workbook
                    Ok(ExcelWorkbook {
                        sheets: vec![],
                        metadata: office_metadata.unwrap_or_default(),
                        revisions: None,
                    })
                }
            }
        }
        // Standard XLS format: propagate errors
        ".xls" => {
            let cursor = Cursor::new(data);
            let workbook =
                calamine::Xls::new(cursor).map_err(|e| XbergError::parsing(format!("Failed to parse XLS: {}", e)))?;
            process_workbook(workbook, office_metadata)
        }
        // Exotic format: .xla (legacy add-in) - may not contain proper workbook data
        ".xla" => {
            let cursor = Cursor::new(data);
            match calamine::Xls::new(cursor) {
                Ok(workbook) => process_workbook(workbook, office_metadata),
                Err(_) => {
                    // .xla files may not contain proper workbook data - return empty workbook
                    Ok(ExcelWorkbook {
                        sheets: vec![],
                        metadata: office_metadata.unwrap_or_default(),
                        revisions: None,
                    })
                }
            }
        }
        // Standard XLSB format (binary spreadsheet): propagate errors
        ".xlsb" => {
            let cursor = Cursor::new(data);
            let workbook =
                calamine::Xlsb::new(cursor).map_err(|e| XbergError::parsing(format!("Failed to parse XLSB: {}", e)))?;
            process_workbook(workbook, office_metadata)
        }
        // Standard OpenDocument format
        ".ods" => {
            let cursor = Cursor::new(data);
            let workbook =
                calamine::Ods::new(cursor).map_err(|e| XbergError::parsing(format!("Failed to parse ODS: {}", e)))?;
            process_workbook(workbook, office_metadata)
        }
        _ => Err(XbergError::parsing(format!(
            "Unsupported file extension: {}",
            file_extension
        ))),
    }
}

/// Process XLSX workbooks with special handling for pathological sparse files.
///
/// This function uses calamine's `worksheet_cells_reader()` API to detect sheets with
/// extreme bounding boxes BEFORE allocating memory for the full Range. This prevents
/// OOM when processing files like Excel Solver files that have cells at both A1 and
/// XFD1048575, creating a bounding box of ~17 billion cells.
fn process_xlsx_workbook<RS: Read + Seek>(
    mut workbook: calamine::Xlsx<RS>,
    office_metadata: Option<HashMap<String, String>>,
) -> Result<ExcelWorkbook> {
    let sheet_names = workbook.sheet_names();
    let mut sheets = Vec::with_capacity(sheet_names.len());

    for name in &sheet_names {
        // Use worksheet_cells_reader to stream cells and detect pathological bounding boxes
        match process_xlsx_sheet_safe(&mut workbook, name) {
            Ok(sheet) => sheets.push(sheet),
            Err(e) => {
                // Log but don't fail - continue with other sheets
                tracing::warn!("Failed to process sheet '{}': {}", name, e);
            }
        }
    }

    let metadata = extract_metadata(&workbook, &sheet_names, office_metadata);
    Ok(ExcelWorkbook {
        sheets,
        metadata,
        revisions: None,
    })
}

/// Process a single XLSX sheet safely by pre-checking the bounding box.
///
/// This function streams cells to compute the actual bounding box without allocating
/// a full Range, then only creates the Range if the bounding box is within safe limits.
fn process_xlsx_sheet_safe<RS: Read + Seek>(workbook: &mut calamine::Xlsx<RS>, sheet_name: &str) -> Result<ExcelSheet> {
    // First pass: stream cells to compute actual bounding box and collect cell data
    let (cells, row_min, row_max, col_min, col_max) = {
        let mut cell_reader = workbook
            .worksheet_cells_reader(sheet_name)
            .map_err(|e| XbergError::parsing(format!("Failed to read sheet '{}': {}", sheet_name, e)))?;

        let mut cells: Vec<((u32, u32), Data)> = Vec::new();
        let mut row_min = u32::MAX;
        let mut row_max = 0u32;
        let mut col_min = u32::MAX;
        let mut col_max = 0u32;

        // Stream through all cells, tracking bounds
        while let Ok(Some(cell)) = cell_reader.next_cell() {
            let (row, col) = cell.get_position();
            row_min = row_min.min(row);
            row_max = row_max.max(row);
            col_min = col_min.min(col);
            col_max = col_max.max(col);

            // Convert DataRef to owned Data
            let data: Data = match cell.get_value() {
                DataRef::Empty => Data::Empty,
                DataRef::String(s) => Data::String(s.clone()),
                DataRef::SharedString(s) => Data::String(s.to_string()),
                DataRef::Float(f) => Data::Float(*f),
                DataRef::Int(i) => Data::Int(*i),
                DataRef::Bool(b) => Data::Bool(*b),
                DataRef::DateTime(dt) => Data::DateTime(*dt),
                DataRef::DateTimeIso(s) => Data::DateTimeIso(s.clone()),
                DataRef::DurationIso(s) => Data::DurationIso(s.clone()),
                DataRef::Error(e) => Data::Error(e.clone()),
            };
            cells.push(((row, col), data));
        }
        (cells, row_min, row_max, col_min, col_max)
    }; // cell_reader is dropped here, releasing the borrow

    // Check if sheet is empty
    if cells.is_empty() {
        return Ok(ExcelSheet {
            name: sheet_name.to_owned(),
            markdown: format!("## {}\n\n*Empty sheet*", sheet_name),
            row_count: 0,
            col_count: 0,
            cell_count: 0,
            table_cells: None,
        });
    }

    // Calculate bounding box size
    let bb_rows = (row_max - row_min + 1) as u64;
    let bb_cols = (col_max - col_min + 1) as u64;
    let bb_cells = bb_rows.saturating_mul(bb_cols);

    // Check for pathological bounding box
    if bb_cells > MAX_BOUNDING_BOX_CELLS {
        // Sheet has sparse data at extreme positions - process directly from cells
        return process_sparse_sheet_from_cells(sheet_name, cells, row_min, row_max, col_min, col_max);
    }

    // Safe to create a Range - bounding box is within limits
    // Use calamine's normal worksheet_range which will create the Range
    let range = workbook
        .worksheet_range(sheet_name)
        .map_err(|e| XbergError::parsing(format!("Failed to parse sheet '{}': {}", sheet_name, e)))?;

    Ok(process_sheet(sheet_name, &range))
}

/// Process a sparse sheet directly from collected cells without creating a full Range.
///
/// This is used when the bounding box would exceed MAX_BOUNDING_BOX_CELLS.
/// Instead of creating a dense Range, we generate a markdown pipe table from the sparse cells.
fn process_sparse_sheet_from_cells(
    sheet_name: &str,
    cells: Vec<((u32, u32), Data)>,
    row_min: u32,
    row_max: u32,
    col_min: u32,
    col_max: u32,
) -> Result<ExcelSheet> {
    let cell_count = cells.len();
    let bb_rows = (row_max - row_min + 1) as usize;
    let bb_cols = (col_max - col_min + 1) as usize;

    // Collect unique columns and rows that actually contain data
    let mut col_set = std::collections::BTreeSet::new();
    let mut row_set = std::collections::BTreeSet::new();
    let mut cell_map: HashMap<(u32, u32), &Data> = HashMap::with_capacity(cells.len());

    for ((row, col), data) in &cells {
        if !matches!(data, Data::Empty) {
            col_set.insert(*col);
            row_set.insert(*row);
            cell_map.insert((*row, *col), data);
        }
    }

    let cols: Vec<u32> = col_set.into_iter().collect();
    let rows: Vec<u32> = row_set.into_iter().collect();

    if cols.is_empty() || rows.is_empty() {
        let markdown = format!("## {}\n\n*Empty sheet*", sheet_name);
        return Ok(ExcelSheet {
            name: sheet_name.to_owned(),
            markdown,
            row_count: bb_rows,
            col_count: bb_cols,
            cell_count,
            table_cells: None,
        });
    }

    // Limit output to avoid huge tables
    const MAX_OUTPUT_ROWS: usize = 1000;
    const MAX_OUTPUT_COLS: usize = 50;
    let display_rows = rows.len().min(MAX_OUTPUT_ROWS);
    let display_cols = cols.len().min(MAX_OUTPUT_COLS);

    let mut markdown = String::with_capacity(500 + cell_count * 20);
    let mut table_cells: Vec<Vec<String>> = Vec::with_capacity(display_rows + 1);

    write!(markdown, "## {}\n\n", sheet_name).expect("write to String cannot fail");

    // First row of actual data is treated as the header row
    // Build header
    let first_row = rows[0];
    let mut header_cells = Vec::with_capacity(display_cols);
    markdown.push_str("| ");
    for (i, &col) in cols.iter().take(display_cols).enumerate() {
        if i > 0 {
            markdown.push_str(" | ");
        }
        let cell_str = cell_map
            .get(&(first_row, col))
            .map(|d| format_cell_to_string(d))
            .unwrap_or_default();
        if cell_str.contains('|') || cell_str.contains('\\') {
            escape_markdown_into(&mut markdown, &cell_str);
        } else {
            markdown.push_str(&cell_str);
        }
        header_cells.push(cell_str);
    }
    markdown.push_str(" |\n");
    table_cells.push(header_cells);

    // Separator row
    markdown.push_str("| ");
    for i in 0..display_cols {
        if i > 0 {
            markdown.push_str(" | ");
        }
        markdown.push_str("---");
    }
    markdown.push_str(" |\n");

    // Data rows
    for &row in rows.iter().skip(1).take(display_rows - 1) {
        let mut row_cells_vec = Vec::with_capacity(display_cols);
        markdown.push_str("| ");
        for (i, &col) in cols.iter().take(display_cols).enumerate() {
            if i > 0 {
                markdown.push_str(" | ");
            }
            let cell_str = cell_map
                .get(&(row, col))
                .map(|d| format_cell_to_string(d))
                .unwrap_or_default();
            if cell_str.contains('|') || cell_str.contains('\\') {
                escape_markdown_into(&mut markdown, &cell_str);
            } else {
                markdown.push_str(&cell_str);
            }
            row_cells_vec.push(cell_str);
        }
        markdown.push_str(" |\n");
        table_cells.push(row_cells_vec);
    }

    if rows.len() > MAX_OUTPUT_ROWS || cols.len() > MAX_OUTPUT_COLS {
        write!(
            markdown,
            "\n*Truncated: showing {}x{} of {}x{} cells*\n",
            display_rows,
            display_cols,
            rows.len(),
            cols.len()
        )
        .expect("write to String cannot fail");
    }

    Ok(ExcelSheet {
        name: sheet_name.to_owned(),
        markdown,
        row_count: bb_rows,
        col_count: bb_cols,
        cell_count,
        table_cells: Some(table_cells),
    })
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

    Ok(ExcelWorkbook {
        sheets,
        metadata,
        revisions: None,
    })
}

#[inline]
fn process_sheet(name: &str, range: &Range<Data>) -> ExcelSheet {
    let (rows, cols) = range.get_size();
    let cell_count = range.used_cells().count();

    // Fix for issue #331: Use actual cell count instead of declared dimensions
    // to avoid OOM on sparse sheets with extreme dimensions (e.g., Excel Solver files).
    // Declared dimensions can claim A1:XFD1048575 (~17T cells) while actual data is minimal.
    let estimated_capacity = 50 + (cols * 20) + (cell_count * 12);

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
    // Fix for issue #331: Protect against extreme declared dimensions.
    // Excel Solver files can declare A1:XFD1048575 (1M+ rows) but only have ~26 actual cells.
    // Calling range.rows().collect() would iterate ALL declared rows causing OOM.
    const MAX_REASONABLE_ROWS: usize = 100_000; // Cap at 100K rows for safety

    let (declared_rows, _declared_cols) = range.get_size();

    // If declared rows exceed reasonable limit, skip processing to avoid OOM
    if declared_rows > MAX_REASONABLE_ROWS {
        let actual_cell_count = range.used_cells().count();

        // If actual data is minimal compared to declared size, it's a sparse/pathological file
        if actual_cell_count < 10_000 {
            // Return minimal output instead of OOM
            let result_capacity = 100 + sheet_name.len();
            let mut result = String::with_capacity(result_capacity);
            write!(
                result,
                "## {}\n\n*Sheet has extreme declared dimensions ({} rows) with minimal actual data ({} cells). Skipping to prevent OOM.*",
                sheet_name, declared_rows, actual_cell_count
            ).unwrap();
            return (result, Vec::new());
        }
    }

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

        if cell_str.contains('|') || cell_str.contains('\\') {
            escape_markdown_into(&mut markdown, &cell_str);
        } else {
            markdown.push_str(&cell_str);
        }
        header_cells.push(cell_str);
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
            let cell_str = if let Some(cell) = row.get(i) {
                let cell_str = format_cell_to_string(cell);

                if cell_str.contains('|') || cell_str.contains('\\') {
                    escape_markdown_into(&mut markdown, &cell_str);
                } else {
                    markdown.push_str(&cell_str);
                }
                cell_str
            } else {
                String::new()
            };
            row_cells.push(cell_str);
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
///
/// Float values that are whole numbers (e.g. 1.0, 42.0) are formatted without the
/// trailing decimal point (e.g. "1", "42") so that numeric ground-truth comparisons
/// produce correct F1 scores.  Rust's default `{}` formatter already does this for
/// `f64`, so we simply delegate to it for every float case.
#[inline]
fn format_cell_to_string(data: &Data) -> String {
    match data {
        Data::Empty => String::new(),
        Data::String(s) => s.clone(),
        Data::Float(f) => format!("{}", f),
        Data::Int(i) => format!("{}", i),
        Data::Bool(b) => {
            if *b {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        Data::DateTime(dt) => {
            // `as_datetime()` requires the calamine "chrono" feature which is not enabled;
            // use `to_ymd_hms_milli()` instead (available with the "dates" feature).
            let (year, month, day, hour, min, sec, _milli) = dt.to_ymd_hms_milli();
            format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", year, month, day, hour, min, sec)
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

/// Convert an Excel workbook to plain text (space-separated cells, one row per line).
///
/// Each sheet is separated by a blank line. Sheet names are included as headers.
/// This produces text suitable for quality scoring against ground truth.
#[cfg_attr(alef, alef(skip))]
pub fn excel_to_text(workbook: &ExcelWorkbook) -> String {
    let mut result = String::new();

    for (i, sheet) in workbook.sheets.iter().enumerate() {
        if i > 0 {
            result.push_str("\n\n");
        }

        if let Some(cells) = &sheet.table_cells {
            for (row_idx, row) in cells.iter().enumerate() {
                if row_idx > 0 {
                    result.push('\n');
                }
                let line: String = row
                    .iter()
                    .map(|cell| cell.trim())
                    .filter(|cell| !cell.is_empty())
                    .collect::<Vec<_>>()
                    .join(" ");
                result.push_str(&line);
            }
        }
    }

    result
}

/// Render all sheets in an [`ExcelWorkbook`] as a single Markdown string.
///
/// Sheets are separated by double newlines; each sheet's pre-rendered markdown
/// is trimmed of trailing whitespace before joining.
#[cfg_attr(alef, alef(skip))]
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
        ZipArchive::new(file).map_err(|e| XbergError::parsing(format!("Failed to open ZIP archive: {}", e)))?;

    extract_xlsx_office_metadata_from_archive(&mut archive)
}

#[cfg(feature = "office")]
fn extract_xlsx_office_metadata_from_bytes(data: &[u8]) -> Result<HashMap<String, String>> {
    use zip::ZipArchive;

    let cursor = Cursor::new(data);
    let mut archive =
        ZipArchive::new(cursor).map_err(|e| XbergError::parsing(format!("Failed to open ZIP archive: {}", e)))?;

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

// ── xl/revisions parsing ──────────────────────────────────────────────────────

/// Extract revision headers from an in-memory `.xlsx`/`.xlsm`/`.xltm` blob.
///
/// Returns `None` when `xl/revisions/revisionHeaders.xml` is absent (the
/// common case for modern files that don't use legacy shared-workbook mode).
/// Returns `Some(vec![])` when the file exists but contains no `<header>`
/// elements. On any parse error the function logs a warning and returns `None`
/// so that the rest of extraction succeeds.
fn extract_xlsx_revisions_from_bytes(data: &[u8]) -> Option<Vec<DocumentRevision>> {
    let cursor = Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor).ok()?;
    extract_xlsx_revisions_from_archive(&mut archive)
}

/// Extract revision headers from an `.xlsx`/`.xlsm`/`.xltm` file on disk.
///
/// Same semantics as [`extract_xlsx_revisions_from_bytes`].
fn extract_xlsx_revisions_from_file(file_path: &str) -> Option<Vec<DocumentRevision>> {
    let file = std::fs::File::open(file_path).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;
    extract_xlsx_revisions_from_archive(&mut archive)
}

/// Core revision-header parser shared by the file and bytes paths.
fn extract_xlsx_revisions_from_archive<R: Read + Seek>(
    archive: &mut zip::ZipArchive<R>,
) -> Option<Vec<DocumentRevision>> {
    const HEADERS_PATH: &str = "xl/revisions/revisionHeaders.xml";

    // Check presence without consuming; if absent this is not a shared workbook.
    let xml_bytes = {
        let mut entry = match archive.by_name(HEADERS_PATH) {
            Ok(e) => e,
            Err(zip::result::ZipError::FileNotFound) => return None,
            Err(e) => {
                tracing::warn!(
                    path = HEADERS_PATH,
                    error = %e,
                    "failed to open xl/revisions/revisionHeaders.xml"
                );
                return None;
            }
        };
        let mut buf = Vec::new();
        if entry.read_to_end(&mut buf).is_err() {
            return None;
        }
        buf
    };

    match parse_revision_headers_xml(&xml_bytes) {
        Ok(revisions) => Some(revisions),
        Err(e) => {
            tracing::warn!(error = %e, "failed to parse xl/revisions/revisionHeaders.xml");
            None
        }
    }
}

/// Parse `xl/revisions/revisionHeaders.xml` and emit one `DocumentRevision`
/// per `<header>` element.
///
/// Each header carries a `guid` (→ `revision_id`), `userName` (→ `author`),
/// and `dateTime` (→ `timestamp`). `anchor` and `delta` are empty for v1;
/// per-cell log parsing (`revisionLog*.xml`) is a future follow-up.
///
/// `RevisionKind::FormatChange` is used as the closest available variant
/// because the header file does not distinguish what *kind* of changes the
/// revision contains — that information is in the per-revision log file.
fn parse_revision_headers_xml(xml_bytes: &[u8]) -> Result<Vec<DocumentRevision>> {
    let xml_str = crate::text::utf8_validation::from_utf8(xml_bytes)
        .map_err(|e| XbergError::parsing(format!("invalid UTF-8 in revisionHeaders.xml: {e}")))?;

    let doc = roxmltree::Document::parse(xml_str)
        .map_err(|e| XbergError::parsing(format!("failed to parse revisionHeaders.xml: {e}")))?;

    // Spreadsheet ML namespace for revision headers
    const SPREADSHEETML_NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";

    let mut revisions = Vec::new();
    for node in doc.descendants() {
        if !node.has_tag_name((SPREADSHEETML_NS, "header")) {
            continue;
        }
        let revision_id = node
            .attribute("guid")
            .unwrap_or("")
            .trim_matches(|c| c == '{' || c == '}')
            .to_string();
        let revision_id = if revision_id.is_empty() {
            format!("xlsx-rev-{}", revisions.len())
        } else {
            revision_id
        };
        let author = node.attribute("userName").filter(|s| !s.is_empty()).map(str::to_string);
        let timestamp = node.attribute("dateTime").filter(|s| !s.is_empty()).map(str::to_string);
        revisions.push(DocumentRevision {
            revision_id,
            author,
            timestamp,
            // The header file does not tell us what kind of change was made;
            // FormatChange is the closest neutral variant. Per-cell kind data
            // requires parsing the corresponding revisionLog*.xml (follow-up).
            kind: RevisionKind::FormatChange,
            anchor: None,
            delta: RevisionDelta::default(),
        });
    }

    Ok(revisions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_cell_to_string_basic() {
        assert_eq!(format_cell_to_string(&Data::Empty), "");
        assert_eq!(format_cell_to_string(&Data::String("test".to_owned())), "test");
        // Whole-number floats must NOT include a trailing ".0" – ground-truth files
        // use plain integers and f1_numeric scoring requires an exact token match.
        assert_eq!(format_cell_to_string(&Data::Float(42.0)), "42");
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
        // 49353.5 in Excel serial date (1900 epoch) ≈ 2035-03-22 12:00:00
        let dt = Data::DateTime(ExcelDateTime::new(49353.5, ExcelDateTimeType::DateTime, false));
        let result = format_cell_to_string(&dt);
        assert!(!result.is_empty());
        // Result should look like an ISO-style datetime string
        assert!(result.contains('-'), "Expected datetime string, got: {}", result);
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
        // Whole-number floats should be formatted without a trailing ".0"
        let result = format_cell_to_string(&Data::Float(100.0));
        assert_eq!(result, "100");
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

    // ── xl/revisions tests ────────────────────────────────────────────────────

    /// Build a minimal in-memory `.xlsx` zip that contains a synthetic
    /// `xl/revisions/revisionHeaders.xml` with the given `<header>` elements.
    ///
    /// `headers` is a slice of `(guid, user_name, date_time)` tuples.
    fn make_xlsx_with_revision_headers(headers: &[(&str, &str, &str)]) -> Vec<u8> {
        use std::io::Write;
        use zip::write::{SimpleFileOptions, ZipWriter};

        let mut buffer = Vec::new();
        {
            let mut zip = ZipWriter::new(Cursor::new(&mut buffer));
            let opts = SimpleFileOptions::default();

            zip.start_file("[Content_Types].xml", opts).unwrap();
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="xml" ContentType="application/xml"/>
  <Default Extension="rels"
    ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Override PartName="/xl/workbook.xml"
    ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
</Types>"#,
            )
            .unwrap();

            zip.start_file("_rels/.rels", opts).unwrap();
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1"
    Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument"
    Target="xl/workbook.xml"/>
</Relationships>"#,
            )
            .unwrap();

            zip.start_file("xl/workbook.xml", opts).unwrap();
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
          xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheets>
    <sheet name="Sheet1" sheetId="1" r:id="rId1"/>
  </sheets>
</workbook>"#,
            )
            .unwrap();

            zip.start_file("xl/_rels/workbook.xml.rels", opts).unwrap();
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1"
    Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet"
    Target="worksheets/sheet1.xml"/>
</Relationships>"#,
            )
            .unwrap();

            zip.start_file("xl/worksheets/sheet1.xml", opts).unwrap();
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData/>
</worksheet>"#,
            )
            .unwrap();

            // Build revisionHeaders.xml
            let mut headers_xml = String::from(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<headers xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#,
            );
            for (guid, user, dt) in headers {
                use std::fmt::Write as FmtWrite;
                let _ = write!(
                    headers_xml,
                    r#"<header guid="{{{guid}}}" dateTime="{dt}" userName="{user}" maxSheetId="1"/>"#,
                );
            }
            headers_xml.push_str("\n</headers>");

            zip.start_file("xl/revisions/revisionHeaders.xml", opts).unwrap();
            zip.write_all(headers_xml.as_bytes()).unwrap();

            let _ = zip.finish().unwrap();
        }
        buffer
    }

    #[test]
    fn should_return_none_revisions_when_xl_revisions_absent() {
        // A plain xlsx without xl/revisions/ must yield revisions = None.
        use std::io::Write;
        use zip::write::{SimpleFileOptions, ZipWriter};

        let mut buffer = Vec::new();
        {
            let mut zip = ZipWriter::new(Cursor::new(&mut buffer));
            let opts = SimpleFileOptions::default();

            zip.start_file("[Content_Types].xml", opts).unwrap();
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="xml" ContentType="application/xml"/>
</Types>"#,
            )
            .unwrap();

            zip.start_file("xl/workbook.xml", opts).unwrap();
            zip.write_all(br#"<?xml version="1.0"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheets/></workbook>"#).unwrap();

            let _ = zip.finish().unwrap();
        }

        let result = extract_xlsx_revisions_from_bytes(&buffer);
        assert!(result.is_none(), "expected None when xl/revisions/ is absent");
    }

    #[test]
    fn should_parse_two_revision_headers_with_correct_fields() {
        let xlsx = make_xlsx_with_revision_headers(&[
            ("11111111-2222-3333-4444-555555555555", "Alice", "2024-01-15T09:00:00Z"),
            ("AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE", "Bob", "2024-01-16T14:30:00Z"),
        ]);

        let revisions = extract_xlsx_revisions_from_bytes(&xlsx)
            .expect("revisions should be Some when xl/revisions/revisionHeaders.xml is present");

        assert_eq!(revisions.len(), 2, "expected 2 revisions from 2 headers");

        // First header
        assert_eq!(
            revisions[0].revision_id, "11111111-2222-3333-4444-555555555555",
            "guid should be stored without braces"
        );
        assert_eq!(revisions[0].author.as_deref(), Some("Alice"));
        assert_eq!(revisions[0].timestamp.as_deref(), Some("2024-01-15T09:00:00Z"));
        assert!(
            matches!(revisions[0].kind, RevisionKind::FormatChange),
            "kind should be FormatChange for v1 headers"
        );
        assert!(revisions[0].anchor.is_none(), "anchor should be None for v1");
        assert!(
            revisions[0].delta.content.is_empty(),
            "delta.content should be empty for v1"
        );

        // Second header
        assert_eq!(revisions[1].revision_id, "AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE");
        assert_eq!(revisions[1].author.as_deref(), Some("Bob"));
        assert_eq!(revisions[1].timestamp.as_deref(), Some("2024-01-16T14:30:00Z"));
    }

    #[test]
    fn should_return_some_empty_vec_when_headers_xml_exists_but_has_no_header_elements() {
        // File with xl/revisions/revisionHeaders.xml that has no <header> children.
        use std::io::Write;
        use zip::write::{SimpleFileOptions, ZipWriter};

        let mut buffer = Vec::new();
        {
            let mut zip = ZipWriter::new(Cursor::new(&mut buffer));
            let opts = SimpleFileOptions::default();
            zip.start_file("[Content_Types].xml", opts).unwrap();
            zip.write_all(b"<?xml version=\"1.0\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"/>").unwrap();

            zip.start_file("xl/revisions/revisionHeaders.xml", opts).unwrap();
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<headers xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"/>
"#,
            )
            .unwrap();
            let _ = zip.finish().unwrap();
        }

        let revisions = extract_xlsx_revisions_from_bytes(&buffer).expect("revisions should be Some when file exists");
        assert!(revisions.is_empty(), "expected empty vec when no <header> elements");
    }

    #[test]
    fn should_surface_revisions_in_full_xlsx_extraction() {
        // Round-trip: build an xlsx with 1 revision header and verify
        // read_excel_bytes populates workbook.revisions.
        let xlsx = make_xlsx_with_revision_headers(&[(
            "DEADBEEF-0000-0000-0000-000000000001",
            "Carol",
            "2024-03-01T12:00:00Z",
        )]);

        let workbook = read_excel_bytes(&xlsx, ".xlsx").expect("should parse workbook");
        let revisions = workbook
            .revisions
            .as_ref()
            .expect("revisions should be Some after full extraction");
        assert_eq!(revisions.len(), 1);
        assert_eq!(revisions[0].author.as_deref(), Some("Carol"));
    }
}
