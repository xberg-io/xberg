//! dBASE (.dbf) extractor.
//!
//! Reads records from dBASE files and formats them as a markdown table.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extractors::security::{SecurityError, SecurityLimits};
use crate::plugins::{InternalDocumentExtractor, Plugin};
use crate::types::internal::InternalDocument;
use crate::types::internal_builder::InternalDocumentBuilder;
use crate::types::metadata::{DbfFieldInfo, DbfMetadata, FormatMetadata, Metadata};
use async_trait::async_trait;
use std::io::{Cursor, Read, Seek};
use std::path::Path;
#[cfg_attr(alef, alef(skip))]
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
        "Xberg Team"
    }
}

fn field_value_to_string(value: &dbase::FieldValue) -> String {
    match value {
        dbase::FieldValue::Character(Some(s)) => s.trim().to_string(),
        dbase::FieldValue::Numeric(Some(n)) => n.to_string(),
        dbase::FieldValue::Logical(Some(b)) => b.to_string(),
        dbase::FieldValue::Date(Some(d)) => format!("{}-{:02}-{:02}", d.year(), d.month(), d.day()),
        dbase::FieldValue::DateTime(dt) => format_dbf_datetime(dt),
        dbase::FieldValue::Float(Some(f)) => f.to_string(),
        dbase::FieldValue::Integer(i) => i.to_string(),
        dbase::FieldValue::Currency(c) => format!("{c:.2}"),
        dbase::FieldValue::Double(d) => d.to_string(),
        dbase::FieldValue::Memo(s) => s.trim().to_string(),
        _ => String::new(),
    }
}

/// Render a dBASE `DateTime` field (#108). Previously unmatched in
/// `field_value_to_string`, so every `DateTime` field silently rendered as an
/// empty string instead of its date and time.
fn format_dbf_datetime(value: &dbase::DateTime) -> String {
    let date = value.date();
    let time = value.time();
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        date.year(),
        date.month(),
        date.day(),
        time.hours(),
        time.minutes(),
        time.seconds()
    )
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
        dbase::FieldValue::DateTime(_) => "DateTime",
        dbase::FieldValue::Float(_) => "Float",
        dbase::FieldValue::Integer(_) => "Integer",
        dbase::FieldValue::Currency(_) => "Currency",
        dbase::FieldValue::Double(_) => "Double",
        dbase::FieldValue::Memo(_) => "Memo",
    }
}

/// Parse a dBASE file once, returning field names, types, and row data.
///
/// Memo (`.dbt`/`.fpt`) fields cannot be resolved here: a memo field only
/// stores a block pointer into the sidecar file, and this entry point has no
/// filesystem access to find it. A document with memo fields but no sidecar
/// fails outright (matches the underlying `dbase` crate's behavior). Reading
/// from a real path goes through [`parse_dbf_with_memo`] instead (#109).
fn parse_dbf(content: &[u8]) -> Result<DbfParsed> {
    let reader = dbase::Reader::new(Cursor::new(content))
        .map_err(|e| crate::XbergError::parsing(format!("Failed to open dBASE file: {e}")))?;
    parse_dbf_records(reader)
}

/// Parse a dBASE file together with its memo sidecar, resolving `Memo` fields
/// to their actual text instead of failing on `MissingMemoFile` (#109).
fn parse_dbf_with_memo(content: &[u8], memo: &[u8]) -> Result<DbfParsed> {
    let reader = dbase::ReaderBuilder::new()
        .build_with_memo(Cursor::new(content), Cursor::new(memo))
        .map_err(|e| crate::XbergError::parsing(format!("Failed to open dBASE file with memo: {e}")))?;
    parse_dbf_records(reader)
}

/// One record's values in field-declaration order.
///
/// `dbase::Record` keys values by field name in a `HashMap`: iterating it does
/// not follow the header order, and two fields sharing a name collapse into one.
struct OrderedRecord(Vec<dbase::FieldValue>);

impl dbase::ReadableRecord for OrderedRecord {
    fn read_using<Source, MemoSource>(
        field_iterator: &mut dbase::FieldIterator<Source, MemoSource>,
    ) -> std::result::Result<Self, dbase::FieldError>
    where
        Source: Read + Seek,
        MemoSource: Read + Seek,
    {
        field_iterator
            .map(|field| field.map(|named| named.value))
            .collect::<std::result::Result<_, _>>()
            .map(Self)
    }
}

/// Shared record-reading logic for both the memo-less and memo-aware readers.
fn parse_dbf_records<T: Read + Seek>(mut reader: dbase::Reader<T>) -> Result<DbfParsed> {
    let field_names: Vec<String> = reader.fields().iter().map(|f| f.name().to_string()).collect();

    let records = reader
        .iter_records_as::<OrderedRecord>()
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| crate::XbergError::parsing(format!("Failed to read dBASE records: {e}")))?;

    let record_count = records.len();

    let mut field_types: Vec<String> = vec!["Unknown".to_string(); field_names.len()];
    let mut rows: Vec<Vec<String>> = Vec::with_capacity(records.len());
    let mut first_row = true;

    for OrderedRecord(values) in records {
        let mut row = Vec::with_capacity(field_names.len());
        for (col_idx, v) in values.into_iter().enumerate() {
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

/// Memo sidecar extensions tried, in order: dBASE III/IV (`.dbt`), FoxPro
/// (`.fpt`), and their uppercase forms (relevant on case-sensitive filesystems).
const MEMO_SIDECAR_EXTENSIONS: &[&str] = &["dbt", "DBT", "fpt", "FPT"];

/// Locate and read a `.dbf` file's memo sidecar next to it on disk, if any
/// (#109). Returns `Ok(None)` when no sidecar exists — the caller falls back
/// to memo-less parsing, matching prior behavior for `.dbf` files that never
/// had a memo file to begin with.
fn read_memo_sidecar(dbf_path: &Path, limits: &SecurityLimits) -> Result<Option<Vec<u8>>> {
    for extension in MEMO_SIDECAR_EXTENSIONS {
        let candidate = dbf_path.with_extension(extension);
        if !candidate.is_file() {
            continue;
        }
        let bytes = crate::core::io::open_file_bytes(&candidate)?;
        if bytes.len() > limits.max_content_size {
            return Err(SecurityError::ContentTooLarge {
                size: bytes.len(),
                max: limits.max_content_size,
            }
            .into());
        }
        return Ok(Some(bytes.to_vec()));
    }
    Ok(None)
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

/// Build the final `InternalDocument` (table + `DbfMetadata`) from parsed rows.
/// Shared by the bytes-only and path-based (memo-aware) entry points.
fn finish_dbf_document(parsed: &DbfParsed, mime_type: &str) -> InternalDocument {
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

    let mut doc = build_dbf_internal_document(parsed);
    doc.mime_type = mime_type.to_string();
    doc.metadata = Metadata {
        format: Some(FormatMetadata::Dbf(dbf_metadata)),
        ..Default::default()
    };
    doc
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl InternalDocumentExtractor for DbfExtractor {
    async fn extract_content(
        &self,
        content: &[u8],
        mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        let parsed = parse_dbf(content)?;
        Ok(finish_dbf_document(&parsed, mime_type))
    }

    /// Reads a `.dbf` file from disk and, when a `.dbt`/`.fpt` memo sidecar
    /// sits next to it, resolves `Memo` fields to their full text instead of
    /// failing on `MissingMemoFile` (#109). Falls back to memo-less parsing
    /// when there is no sidecar, matching `extract_content`'s behavior.
    async fn extract_path(&self, path: &Path, mime_type: &str, config: &ExtractionConfig) -> Result<InternalDocument> {
        let limits = config.security_limits.clone().unwrap_or_default();
        let dbf_bytes = crate::core::io::open_file_bytes(path)?;
        let memo_bytes = read_memo_sidecar(path, &limits)?;
        let parsed = match &memo_bytes {
            Some(memo) => parse_dbf_with_memo(&dbf_bytes, memo)?,
            None => parse_dbf(&dbf_bytes)?,
        };
        Ok(finish_dbf_document(&parsed, mime_type))
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/vnd.dbf", "application/x-dbf", "application/dbase"]
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
            &["application/vnd.dbf", "application/x-dbf", "application/dbase"]
        );
    }

    #[test]
    fn test_dbf_extractor_initialize_shutdown() {
        let extractor = DbfExtractor::new();
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    /// Record types for the round-trip fixtures below.
    ///
    /// `dbase_record!` expands to `Result<_, FieldError>`. The parent module has
    /// `use crate::Result` in scope — a one-parameter alias — which would shadow
    /// `std::result::Result` inside the expansion and fail to compile. Declaring the
    /// records in a module that does not glob-import the crate prelude keeps the
    /// macro's `Result` meaning the standard one.
    mod dbase_records {
        dbase::dbase_record! {
            pub struct DateTimeRecord { pub ts: dbase::DateTime }
        }

        dbase::dbase_record! {
            pub struct MemoIndexRecord { pub notes: String }
        }

        dbase::dbase_record! {
            pub struct StationRecord {
                pub name: String,
                pub riders: f64,
                pub open: bool,
                pub opened: dbase::Date,
            }
        }

        dbase::dbase_record! {
            pub struct TwoCodesRecord { pub first: String, pub second: String }
        }
    }
    use dbase_records::{DateTimeRecord, MemoIndexRecord, StationRecord, TwoCodesRecord};

    const STATION_COUNT: u32 = 20;

    fn write_dbf<R: dbase::WritableRecord>(table_info: dbase::TableInfo, records: &[R]) -> Vec<u8> {
        let mut cursor = Cursor::new(Vec::<u8>::new());
        {
            let mut file = dbase::File::create_new(&mut cursor, table_info).unwrap();
            file.append_records(records).unwrap();
        }
        cursor.into_inner()
    }

    /// Four fields of four different types, so a row read in any order other
    /// than the declared one is visible both in the cells and in the types.
    fn stations_dbf() -> Vec<u8> {
        let table_info = dbase::TableWriterBuilder::new()
            .add_character_field("NAME".try_into().unwrap(), 20)
            .add_numeric_field("RIDERS".try_into().unwrap(), 10, 0)
            .add_logical_field("OPEN".try_into().unwrap())
            .add_date_field("OPENED".try_into().unwrap())
            .build_table_info();
        let records: Vec<StationRecord> = (1..=STATION_COUNT)
            .map(|i| StationRecord {
                name: format!("station-{i}"),
                riders: f64::from(i * 100),
                open: i.is_multiple_of(2),
                opened: dbase::Date::new(i, 1, 2024).unwrap(),
            })
            .collect();
        write_dbf(table_info, &records)
    }

    fn expected_station_row(i: u32) -> Vec<String> {
        vec![
            format!("station-{i}"),
            (i * 100).to_string(),
            i.is_multiple_of(2).to_string(),
            format!("2024-01-{i:02}"),
        ]
    }

    /// Regression for #108: a `DateTime` field previously fell through to the
    /// catch-all `_ => String::new()` arm in `field_value_to_string` and
    /// `field_type_name`, rendering as an empty cell with type "Unknown".
    #[test]
    fn should_render_datetime_field_instead_of_dropping_it() {
        let date = dbase::Date::new(15, 3, 2024).unwrap();
        let time = dbase::Time::new(9, 30, 5).unwrap();
        let value = dbase::FieldValue::DateTime(dbase::DateTime::new(date, time));

        assert_eq!(field_value_to_string(&value), "2024-03-15 09:30:05");
        assert_eq!(field_type_name(&value), "DateTime");
    }

    #[test]
    fn should_parse_dbf_file_with_datetime_field() {
        let date = dbase::Date::new(15, 3, 2024).unwrap();
        let time = dbase::Time::new(9, 30, 5).unwrap();
        let dt = dbase::DateTime::new(date, time);

        let mut cursor = Cursor::new(Vec::<u8>::new());
        let table_info = dbase::TableWriterBuilder::new()
            .add_datetime_field("TS".try_into().unwrap())
            .build_table_info();
        {
            let mut file = dbase::File::create_new(&mut cursor, table_info).unwrap();
            file.append_records(&[DateTimeRecord { ts: dt }]).unwrap();
        }
        let dbf_bytes = cursor.into_inner();

        let parsed = parse_dbf(&dbf_bytes).unwrap();

        assert_eq!(parsed.field_types, vec!["DateTime".to_string()]);
        assert_eq!(parsed.rows, vec![vec!["2024-03-15 09:30:05".to_string()]]);
    }

    /// Builds a minimal dBase III (with-memo) `.dbf` + `.dbt` pair.
    ///
    /// The writer only supports Character fields, but the on-disk record
    /// layout is identical for `'C'` and `'M'` fields — only the type byte
    /// (field descriptor offset `32 + 11`, per the dBase III file format) and
    /// its interpretation differ. A Character field written as `"1"` already
    /// produces the space-padded ASCII block-index text dBase III expects for
    /// a memo pointer, so patching the version and type bytes turns a valid
    /// Character-field `.dbf` into a valid Memo-field one.
    fn dbf_with_memo_fixture(memo_text: &[u8]) -> (Vec<u8>, Vec<u8>) {
        let mut cursor = Cursor::new(Vec::<u8>::new());
        let table_info = dbase::TableWriterBuilder::new()
            .add_character_field("NOTES".try_into().unwrap(), 10)
            .build_table_info();
        {
            let mut file = dbase::File::create_new(&mut cursor, table_info).unwrap();
            file.append_records(&[MemoIndexRecord { notes: "1".to_string() }])
                .unwrap();
        }
        let mut dbf_bytes = cursor.into_inner();
        dbf_bytes[0] = 0x83; // dBase III WITH memo
        dbf_bytes[32 + 11] = b'M'; // field type: Character -> Memo ~keep

        // .dbt sidecar: a 6-byte header (next free block = 2, block size = 64)
        // padded to fill block 0, then block 1 holds the memo text terminated
        // by 0x1A.
        const BLOCK_SIZE: usize = 64;
        let mut dbt_bytes = vec![0_u8; BLOCK_SIZE * 2];
        dbt_bytes[0..4].copy_from_slice(&2_u32.to_le_bytes());
        dbt_bytes[4..6].copy_from_slice(&(BLOCK_SIZE as u16).to_le_bytes());
        dbt_bytes[BLOCK_SIZE..BLOCK_SIZE + memo_text.len()].copy_from_slice(memo_text);
        dbt_bytes[BLOCK_SIZE + memo_text.len()] = 0x1A;

        (dbf_bytes, dbt_bytes)
    }

    /// Regression for #109: a `.dbf` with a `Memo` field and its `.dbt`
    /// sidecar must resolve to the full memo text, not fail or render empty.
    #[test]
    fn should_resolve_memo_text_when_sidecar_bytes_are_supplied() {
        let (dbf_bytes, dbt_bytes) = dbf_with_memo_fixture(b"This is the long memo body.");

        let parsed = parse_dbf_with_memo(&dbf_bytes, &dbt_bytes).unwrap();

        assert_eq!(parsed.field_types, vec!["Memo".to_string()]);
        assert_eq!(parsed.rows, vec![vec!["This is the long memo body.".to_string()]]);
    }

    /// Without the sidecar, a `Memo` field cannot be resolved at all — this
    /// documents the existing (unchanged) failure mode of the bytes-only path,
    /// which has no way to discover a sidecar file.
    #[test]
    fn parse_dbf_without_memo_sidecar_fails_on_a_memo_field() {
        let (dbf_bytes, _dbt_bytes) = dbf_with_memo_fixture(b"unreachable");

        assert!(parse_dbf(&dbf_bytes).is_err());
    }

    /// Regression for #1968: `dbase::Record` stores a row in a freshly seeded
    /// `HashMap`, so walking it yielded each row's values in a different order
    /// from the header.
    #[test]
    fn should_keep_every_row_in_field_declaration_order() {
        let parsed = parse_dbf(&stations_dbf()).unwrap();

        assert_eq!(parsed.field_names, vec!["NAME", "RIDERS", "OPEN", "OPENED"]);
        let expected_rows: Vec<Vec<String>> = (1..=STATION_COUNT).map(expected_station_row).collect();
        assert_eq!(parsed.rows, expected_rows);
    }

    #[test]
    fn should_report_field_types_in_field_declaration_order() {
        let parsed = parse_dbf(&stations_dbf()).unwrap();

        assert_eq!(parsed.field_types, vec!["Character", "Numeric", "Logical", "Date"]);
    }

    /// Regression for #1968: neither the dBASE format nor the `dbase` reader
    /// rejects repeated field names, and a name-keyed record kept only one of
    /// the two values.
    #[test]
    fn should_keep_both_values_of_fields_sharing_a_name() {
        let table_info = dbase::TableWriterBuilder::new()
            .add_character_field("CODE".try_into().unwrap(), 10)
            .add_character_field("CODE".try_into().unwrap(), 10)
            .build_table_info();
        let dbf_bytes = write_dbf(
            table_info,
            &[TwoCodesRecord {
                first: "first".to_string(),
                second: "second".to_string(),
            }],
        );

        let parsed = parse_dbf(&dbf_bytes).unwrap();

        assert_eq!(parsed.field_names, vec!["CODE", "CODE"]);
        assert_eq!(parsed.rows, vec![vec!["first".to_string(), "second".to_string()]]);
    }

    #[cfg(feature = "tokio-runtime")]
    #[tokio::test]
    async fn should_align_extracted_cells_and_field_metadata_with_the_header() {
        let extractor = DbfExtractor::new();
        let doc = extractor
            .extract_content(&stations_dbf(), "application/x-dbf", &ExtractionConfig::default())
            .await
            .unwrap();

        let cells = &doc.tables[0].cells;
        assert_eq!(cells[0], vec!["NAME", "RIDERS", "OPEN", "OPENED"]);
        let expected_rows: Vec<Vec<String>> = (1..=STATION_COUNT).map(expected_station_row).collect();
        assert_eq!(cells[1..], expected_rows[..]);

        let Some(FormatMetadata::Dbf(metadata)) = &doc.metadata.format else {
            panic!("expected dBASE metadata, got {:?}", doc.metadata.format);
        };
        let fields: Vec<(&str, &str)> = metadata
            .fields
            .iter()
            .map(|field| (field.name.as_str(), field.field_type.as_str()))
            .collect();
        assert_eq!(
            fields,
            vec![
                ("NAME", "Character"),
                ("RIDERS", "Numeric"),
                ("OPEN", "Logical"),
                ("OPENED", "Date"),
            ]
        );
    }

    #[cfg(feature = "tokio-runtime")]
    #[tokio::test]
    async fn extract_path_resolves_memo_sidecar_next_to_the_dbf_file() {
        let (dbf_bytes, dbt_bytes) = dbf_with_memo_fixture(b"Sidecar-resolved memo text.");

        let dir = tempfile::tempdir().unwrap();
        let dbf_path = dir.path().join("report.dbf");
        std::fs::write(&dbf_path, &dbf_bytes).unwrap();
        std::fs::write(dir.path().join("report.dbt"), &dbt_bytes).unwrap();

        let extractor = DbfExtractor::new();
        let config = ExtractionConfig::default();
        let doc = extractor
            .extract_path(&dbf_path, "application/x-dbf", &config)
            .await
            .unwrap();

        // DBF rows are emitted as a table, so the memo text lands in the table's cells
        // (and its rendered markdown), not in an element's own `text`.
        let rendered: String = doc.tables.iter().map(|table| table.markdown.as_str()).collect();
        assert!(
            rendered.contains("Sidecar-resolved memo text."),
            "memo content from the .dbt sidecar must appear in the extracted document: {rendered:?}"
        );
    }

    #[cfg(feature = "tokio-runtime")]
    #[tokio::test]
    async fn extract_path_without_sidecar_falls_back_to_memo_less_parsing() {
        let content = b"not a dbf file";
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("plain.dbf");
        std::fs::write(&path, content).unwrap();

        let extractor = DbfExtractor::new();
        let config = ExtractionConfig::default();
        let result = extractor.extract_path(&path, "application/x-dbf", &config).await;

        assert!(result.is_err(), "a non-DBF file must still fail to parse");
    }
}
