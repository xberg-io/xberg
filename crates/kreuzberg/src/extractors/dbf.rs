//! dBASE (.dbf) extractor.
//!
//! Reads records from dBASE files and formats them as a markdown table.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::internal::InternalDocument;
use crate::types::internal_builder::InternalDocumentBuilder;
use crate::types::metadata::{DbfFieldInfo, DbfMetadata, FormatMetadata, Metadata};
use async_trait::async_trait;
use std::io::Cursor;

/// Extractor for dBASE (.dbf) files.
///
/// Reads all records and formats them as a markdown table with
/// column headers derived from field names.
pub struct DbfExtractor;

impl DbfExtractor {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl Default for DbfExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for DbfExtractor {
    fn name(&self) -> &str {
        "dbf-extractor"
    }

    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    fn initialize(&self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    fn description(&self) -> &str {
        "dBASE (.dbf) table extraction"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

fn field_value_to_string(value: &dbase::FieldValue) -> String {
    match value {
        dbase::FieldValue::Character(Some(s)) => s.trim().to_string(),
        dbase::FieldValue::Numeric(Some(n)) => n.to_string(),
        dbase::FieldValue::Logical(Some(b)) => b.to_string(),
        dbase::FieldValue::Date(Some(d)) => format!("{}-{:02}-{:02}", d.year(), d.month(), d.day()),
        dbase::FieldValue::Float(Some(f)) => f.to_string(),
        dbase::FieldValue::Integer(i) => i.to_string(),
        dbase::FieldValue::Currency(c) => format!("{c:.2}"),
        dbase::FieldValue::Double(d) => d.to_string(),
        dbase::FieldValue::Memo(s) => s.trim().to_string(),
        _ => String::new(),
    }
}

/// Parsed dBASE data: field names, field types, and rows of string values.
struct DbfParsed {
    field_names: Vec<String>,
    field_types: Vec<String>,
    rows: Vec<Vec<String>>,
    record_count: usize,
}

/// Map a dbase FieldType to a descriptive string.
fn field_type_name(value: &dbase::FieldValue) -> &'static str {
    match value {
        dbase::FieldValue::Character(_) => "Character",
        dbase::FieldValue::Numeric(_) => "Numeric",
        dbase::FieldValue::Logical(_) => "Logical",
        dbase::FieldValue::Date(_) => "Date",
        dbase::FieldValue::Float(_) => "Float",
        dbase::FieldValue::Integer(_) => "Integer",
        dbase::FieldValue::Currency(_) => "Currency",
        dbase::FieldValue::Double(_) => "Double",
        dbase::FieldValue::Memo(_) => "Memo",
        _ => "Unknown",
    }
}

/// Parse a dBASE file once, returning field names, types, and row data.
fn parse_dbf(content: &[u8]) -> Result<DbfParsed> {
    let cursor = Cursor::new(content);
    let mut reader = dbase::Reader::new(cursor)
        .map_err(|e| crate::KreuzbergError::parsing(format!("Failed to open dBASE file: {e}")))?;

    let field_names: Vec<String> = reader.fields().iter().map(|f| f.name().to_string()).collect();

    let records = reader
        .iter_records()
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| crate::KreuzbergError::parsing(format!("Failed to read dBASE records: {e}")))?;

    let record_count = records.len();

    // Detect field types from the first record and build rows simultaneously
    let mut field_types: Vec<String> = vec!["Unknown".to_string(); field_names.len()];
    let mut rows: Vec<Vec<String>> = Vec::with_capacity(records.len());
    let mut first_row = true;

    for record in records {
        let mut row = Vec::with_capacity(field_names.len());
        for (col_idx, (_, v)) in record.into_iter().enumerate() {
            if first_row && col_idx < field_types.len() {
                field_types[col_idx] = field_type_name(&v).to_string();
            }
            row.push(field_value_to_string(&v));
        }
        rows.push(row);
        first_row = false;
    }

    Ok(DbfParsed {
        field_names,
        field_types,
        rows,
        record_count,
    })
}

fn build_dbf_internal_document(parsed: &DbfParsed) -> InternalDocument {
    let mut builder = InternalDocumentBuilder::new("dbf");

    if parsed.field_names.is_empty() {
        return builder.build();
    }

    let mut table_rows: Vec<Vec<String>> = Vec::with_capacity(parsed.rows.len() + 1);
    table_rows.push(parsed.field_names.clone());
    table_rows.extend(parsed.rows.iter().cloned());

    builder.push_table_from_cells(&table_rows, None, None);
    builder.build()
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl DocumentExtractor for DbfExtractor {
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        let parsed = parse_dbf(content)?;

        let fields: Vec<DbfFieldInfo> = parsed
            .field_names
            .iter()
            .zip(parsed.field_types.iter())
            .map(|(name, ftype)| DbfFieldInfo {
                name: name.clone(),
                field_type: ftype.clone(),
            })
            .collect();

        let dbf_metadata = DbfMetadata {
            record_count: parsed.record_count,
            field_count: parsed.field_names.len(),
            fields,
        };

        let mut doc = build_dbf_internal_document(&parsed);
        doc.mime_type = mime_type.to_string();

        doc.metadata = Metadata {
            format: Some(FormatMetadata::Dbf(dbf_metadata)),
            ..Default::default()
        };

        Ok(doc)
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/x-dbf", "application/dbase"]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dbf_extractor_plugin_interface() {
        let extractor = DbfExtractor::new();
        assert_eq!(extractor.name(), "dbf-extractor");
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert_eq!(extractor.priority(), 50);
        assert_eq!(
            extractor.supported_mime_types(),
            &["application/x-dbf", "application/dbase"]
        );
    }

    #[test]
    fn test_dbf_extractor_initialize_shutdown() {
        let extractor = DbfExtractor::new();
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }
}
