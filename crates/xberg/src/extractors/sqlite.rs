//! SQLite and GeoPackage tabular extraction.

use std::collections::HashSet;
use std::io::Cursor;

use async_trait::async_trait;
use rusqlite::config::DbConfig;
use rusqlite::types::ValueRef;
use rusqlite::{Connection, MAIN_DB};

use crate::cancellation::CancellationToken;
use crate::core::config::ExtractionConfig;
use crate::core::mime::{GEOPACKAGE_MIME_TYPE, SQLITE_MIME_TYPE};
use crate::error::XbergError;
use crate::extractors::security::{SecurityBudget, SecurityError, SecurityLimits};
use crate::plugins::{InternalDocumentExtractor, Plugin};
use crate::types::Table;
use crate::types::internal::InternalDocument;
use crate::types::internal_builder::InternalDocumentBuilder;
use crate::{Result, rendering};

const SQLITE_PROGRESS_HANDLER_OPS: i32 = 1_000;
const SQLITE_MAGIC: &[u8; 16] = b"SQLite format 3\0";
const SQLITE_APPLICATION_ID_OFFSET: usize = 68;
const SQLITE_APPLICATION_ID_LENGTH: usize = size_of::<u32>();
const GEOPACKAGE_APPLICATION_ID: &[u8; SQLITE_APPLICATION_ID_LENGTH] = b"GPKG";
const GEOPACKAGE_LEGACY_APPLICATION_ID: &[u8; SQLITE_APPLICATION_ID_LENGTH] = b"GP10";

#[derive(Debug)]
struct TableSpec {
    name: String,
    without_rowid: bool,
}

#[derive(Debug)]
struct ColumnSpec {
    name: String,
    primary_key_position: i64,
}

#[derive(Debug)]
struct TableColumns {
    visible: Vec<ColumnSpec>,
    all_names: HashSet<String>,
}

#[derive(Debug)]
struct ExtractedTable {
    name: String,
    table: Table,
}

/// Extracts user tables from SQLite databases and GeoPackage containers.
#[cfg_attr(alef, alef(skip))]
pub struct SqliteExtractor;

impl SqliteExtractor {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl Default for SqliteExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for SqliteExtractor {
    fn name(&self) -> &str {
        "sqlite-extractor"
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
        "SQLite and GeoPackage tabular extraction"
    }

    fn author(&self) -> &str {
        "Xberg Team"
    }
}

#[async_trait]
impl InternalDocumentExtractor for SqliteExtractor {
    async fn extract_content(
        &self,
        content: &[u8],
        _mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        let limits = config.security_limits.clone().unwrap_or_default();
        enforce_input_size(content.len(), &limits)?;
        ensure_sqlite_magic(content)?;
        check_cancelled(config.cancel_token.as_ref())?;

        let geopackage = has_geopackage_application_id(content);
        let owned_content = content.to_vec();
        let cancel_token = config.cancel_token.clone();
        let span = tracing::Span::current();
        tokio::task::spawn_blocking(move || {
            let _guard = span.entered();
            extract_database(owned_content, geopackage, &limits, cancel_token)
        })
        .await
        .map_err(|error| XbergError::parsing(format!("SQLite extraction task failed: {error}")))?
    }

    fn supported_mime_types(&self) -> &[&str] {
        &[SQLITE_MIME_TYPE, GEOPACKAGE_MIME_TYPE, "application/x-sqlite3"]
    }

    fn priority(&self) -> i32 {
        50
    }
}

fn enforce_input_size(content_size: usize, limits: &SecurityLimits) -> Result<()> {
    if content_size > limits.max_content_size {
        return Err(SecurityError::ContentTooLarge {
            size: content_size,
            max: limits.max_content_size,
        }
        .into());
    }
    Ok(())
}

fn ensure_sqlite_magic(content: &[u8]) -> Result<()> {
    if content.starts_with(SQLITE_MAGIC) {
        return Ok(());
    }
    Err(XbergError::parsing(
        "SQLite input does not contain the SQLite format 3 header",
    ))
}

fn has_geopackage_application_id(content: &[u8]) -> bool {
    let Some(application_id) =
        content.get(SQLITE_APPLICATION_ID_OFFSET..SQLITE_APPLICATION_ID_OFFSET + SQLITE_APPLICATION_ID_LENGTH)
    else {
        return false;
    };
    application_id == GEOPACKAGE_APPLICATION_ID || application_id == GEOPACKAGE_LEGACY_APPLICATION_ID
}

fn extract_database(
    content: Vec<u8>,
    geopackage: bool,
    limits: &SecurityLimits,
    cancel_token: Option<CancellationToken>,
) -> Result<InternalDocument> {
    check_cancelled(cancel_token.as_ref())?;
    let mut connection = open_database(content, cancel_token.as_ref())?;
    configure_connection(&connection, cancel_token.as_ref())?;
    if let Some(token) = cancel_token.clone() {
        install_progress_handler(&mut connection, move || token.is_cancelled())
            .map_err(|error| sqlite_error("install the cancellation handler for", error, cancel_token.as_ref()))?;
    }

    let mut budget = SecurityBudget::from_limits(limits);
    let table_specs = user_table_specs(&connection, geopackage, &mut budget, cancel_token.as_ref())?;
    let mut tables = Vec::with_capacity(table_specs.len());
    for spec in table_specs {
        check_cancelled(cancel_token.as_ref())?;
        tables.push(extract_table(&connection, spec, &mut budget, cancel_token.as_ref())?);
    }
    Ok(build_document(tables, geopackage))
}

fn open_database(content: Vec<u8>, cancel_token: Option<&CancellationToken>) -> Result<Connection> {
    let content_size = content.len();
    let mut connection = Connection::open_in_memory()
        .map_err(|error| sqlite_error("open an in-memory connection for", error, cancel_token))?;
    connection
        .deserialize_read_exact(MAIN_DB, Cursor::new(content), content_size, true)
        .map_err(|error| sqlite_error("deserialize", error, cancel_token))?;
    Ok(connection)
}

fn configure_connection(connection: &Connection, cancel_token: Option<&CancellationToken>) -> Result<()> {
    for (setting, enabled) in [
        (DbConfig::SQLITE_DBCONFIG_DEFENSIVE, true),
        (DbConfig::SQLITE_DBCONFIG_TRUSTED_SCHEMA, false),
        (DbConfig::SQLITE_DBCONFIG_ENABLE_TRIGGER, false),
        (DbConfig::SQLITE_DBCONFIG_ENABLE_VIEW, false),
        (DbConfig::SQLITE_DBCONFIG_DQS_DML, false),
        (DbConfig::SQLITE_DBCONFIG_DQS_DDL, false),
    ] {
        connection
            .set_db_config(setting, enabled)
            .map_err(|error| sqlite_error("apply defensive configuration to", error, cancel_token))?;
    }
    connection
        .execute_batch("PRAGMA query_only = ON; PRAGMA trusted_schema = OFF;")
        .map_err(|error| sqlite_error("make read-only", error, cancel_token))?;
    Ok(())
}

fn install_progress_handler(
    connection: &mut Connection,
    should_cancel: impl FnMut() -> bool + Send + 'static,
) -> rusqlite::Result<()> {
    connection.progress_handler(SQLITE_PROGRESS_HANDLER_OPS, Some(should_cancel))
}

fn user_table_specs(
    connection: &Connection,
    geopackage: bool,
    budget: &mut SecurityBudget,
    cancel_token: Option<&CancellationToken>,
) -> Result<Vec<TableSpec>> {
    let mut statement = connection
        .prepare(
            "SELECT name, wr FROM pragma_table_list \
             WHERE schema = 'main' AND type = 'table' ORDER BY name COLLATE BINARY",
        )
        .map_err(|error| sqlite_error("prepare the table scan for", error, cancel_token))?;
    let mut rows = statement
        .query([])
        .map_err(|error| sqlite_error("scan the tables in", error, cancel_token))?;
    let mut tables = Vec::new();
    while let Some(row) = rows
        .next()
        .map_err(|error| sqlite_error("read the table schema from", error, cancel_token))?
    {
        check_cancelled(cancel_token)?;
        let name: String = row
            .get(0)
            .map_err(|error| sqlite_error("read a table name from", error, cancel_token))?;
        if is_sqlite_internal_table(&name) {
            continue;
        }
        budget.step()?;
        budget.check_entity(&name)?;
        budget.account_text(name.len())?;
        if geopackage && is_container_metadata_table(&name) {
            continue;
        }
        let without_rowid: bool = row
            .get(1)
            .map_err(|error| sqlite_error("read table properties from", error, cancel_token))?;
        tables.push(TableSpec { name, without_rowid });
    }
    Ok(tables)
}

fn is_sqlite_internal_table(name: &str) -> bool {
    name.to_ascii_lowercase().starts_with("sqlite_")
}

fn is_container_metadata_table(name: &str) -> bool {
    let lowercase = name.to_ascii_lowercase();
    lowercase.starts_with("gpkg_") || lowercase.starts_with("rtree_")
}

fn extract_table(
    connection: &Connection,
    spec: TableSpec,
    budget: &mut SecurityBudget,
    cancel_token: Option<&CancellationToken>,
) -> Result<ExtractedTable> {
    let columns = table_columns(connection, &spec, budget, cancel_token)?;
    let column_names: Vec<String> = columns.visible.iter().map(|column| column.name.clone()).collect();
    let projection = columns
        .visible
        .iter()
        .map(|column| quote_identifier(&column.name))
        .collect::<Result<Vec<_>>>()?
        .join(", ");
    let order_by = table_order_by(&spec, &columns)?;
    let query = format!(
        "SELECT {projection} FROM {} ORDER BY {order_by}",
        quote_identifier(&spec.name)?
    );
    let mut statement = connection
        .prepare(&query)
        .map_err(|error| sqlite_error("prepare a user-table query for", error, cancel_token))?;
    account_row(&column_names, budget)?;
    let mut cells = vec![column_names.clone()];
    let mut rows = statement
        .query([])
        .map_err(|error| sqlite_error("query a user table in", error, cancel_token))?;
    while let Some(row) = rows
        .next()
        .map_err(|error| sqlite_error("read a user-table row from", error, cancel_token))?
    {
        check_cancelled(cancel_token)?;
        budget.step()?;
        let values = extract_row(row, column_names.len(), cancel_token)?;
        account_row(&values, budget)?;
        cells.push(values);
    }
    let markdown = rendering::common::render_table_markdown(&cells);
    budget.account_text(markdown.len())?;
    Ok(ExtractedTable {
        name: spec.name,
        table: Table {
            cells,
            markdown,
            page_number: 0,
            columns: Some(column_names),
            ..Default::default()
        },
    })
}

fn table_columns(
    connection: &Connection,
    spec: &TableSpec,
    budget: &mut SecurityBudget,
    cancel_token: Option<&CancellationToken>,
) -> Result<TableColumns> {
    let mut statement = connection
        .prepare("SELECT name, pk, hidden FROM pragma_table_xinfo(?1) ORDER BY cid")
        .map_err(|error| sqlite_error("prepare a column scan for", error, cancel_token))?;
    let columns = statement
        .query_map([&spec.name], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?))
        })
        .map_err(|error| sqlite_error("inspect columns in", error, cancel_token))?;
    let mut visible_columns = Vec::new();
    let mut all_names = HashSet::new();
    for column in columns {
        check_cancelled(cancel_token)?;
        let (name, primary_key_position, hidden) =
            column.map_err(|error| sqlite_error("read a column from", error, cancel_token))?;
        budget.step()?;
        budget.check_entity(&name)?;
        budget.account_text(name.len())?;
        all_names.insert(name.to_ascii_lowercase());
        if hidden == 0 {
            visible_columns.push(ColumnSpec {
                name,
                primary_key_position,
            });
        }
    }
    if visible_columns.is_empty() {
        return Err(XbergError::parsing(format!(
            "SQLite table {:?} has no extractable columns",
            spec.name
        )));
    }
    Ok(TableColumns {
        visible: visible_columns,
        all_names,
    })
}

fn table_order_by(spec: &TableSpec, columns: &TableColumns) -> Result<String> {
    if !spec.without_rowid {
        for alias in ["rowid", "_rowid_", "oid"] {
            if !columns.all_names.contains(alias) {
                return Ok(alias.to_string());
            }
        }
        return columns
            .visible
            .iter()
            .map(|column| quote_identifier(&column.name))
            .collect::<Result<Vec<_>>>()
            .map(|columns| columns.join(", "));
    }
    let mut primary_key_columns: Vec<&ColumnSpec> = columns
        .visible
        .iter()
        .filter(|column| column.primary_key_position > 0)
        .collect();
    primary_key_columns.sort_by_key(|column| column.primary_key_position);
    if primary_key_columns.is_empty() {
        return Err(XbergError::parsing(format!(
            "SQLite WITHOUT ROWID table {:?} has no primary-key columns",
            spec.name
        )));
    }
    primary_key_columns
        .into_iter()
        .map(|column| quote_identifier(&column.name))
        .collect::<Result<Vec<_>>>()
        .map(|columns| columns.join(", "))
}

fn quote_identifier(identifier: &str) -> Result<String> {
    if identifier.contains('\0') {
        return Err(XbergError::parsing("SQLite identifier contains a NUL byte"));
    }
    Ok(format!("\"{}\"", identifier.replace('"', "\"\"")))
}

fn extract_row(
    row: &rusqlite::Row<'_>,
    column_count: usize,
    cancel_token: Option<&CancellationToken>,
) -> Result<Vec<String>> {
    let mut values = Vec::with_capacity(column_count);
    for index in 0..column_count {
        let value = row
            .get_ref(index)
            .map_err(|error| sqlite_error("read a cell from", error, cancel_token))?;
        values.push(value_to_string(value));
    }
    Ok(values)
}

fn value_to_string(value: ValueRef<'_>) -> String {
    match value {
        ValueRef::Null => String::new(),
        ValueRef::Integer(value) => value.to_string(),
        ValueRef::Real(value) => value.to_string(),
        ValueRef::Text(value) => String::from_utf8_lossy(value).into_owned(),
        ValueRef::Blob(value) => format!("[BLOB: {} bytes]", value.len()),
    }
}

fn account_row(row: &[String], budget: &mut SecurityBudget) -> Result<()> {
    budget.add_cells(row.len())?;
    for value in row {
        budget.check_entity(value)?;
        budget.account_text(value.len())?;
    }
    Ok(())
}

fn build_document(tables: Vec<ExtractedTable>, geopackage: bool) -> InternalDocument {
    let mut builder = InternalDocumentBuilder::new(if geopackage { "geopackage" } else { "sqlite" });
    builder.set_mime_type(if geopackage {
        GEOPACKAGE_MIME_TYPE
    } else {
        SQLITE_MIME_TYPE
    });
    for extracted in tables {
        builder.push_heading(2, &extracted.name, None, None);
        builder.push_table(extracted.table, None, None);
    }
    builder.build()
}

fn check_cancelled(cancel_token: Option<&CancellationToken>) -> Result<()> {
    if cancel_token.is_some_and(CancellationToken::is_cancelled) {
        return Err(XbergError::Cancelled);
    }
    Ok(())
}

fn sqlite_error(action: &str, error: rusqlite::Error, cancel_token: Option<&CancellationToken>) -> XbergError {
    if cancel_token.is_some_and(CancellationToken::is_cancelled)
        && error.sqlite_error_code() == Some(rusqlite::ffi::ErrorCode::OperationInterrupted)
    {
        return XbergError::Cancelled;
    }
    XbergError::Parsing {
        message: format!("Failed to {action} SQLite database: {error}"),
        source: Some(Box::new(error)),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    #[test]
    fn should_quote_identifiers_and_reject_nul_bytes() {
        assert_eq!(quote_identifier("odd\"name").unwrap(), "\"odd\"\"name\"");
        assert!(quote_identifier("bad\0name").is_err());
    }

    #[test]
    fn should_reject_truncated_geopackage_application_ids_without_panicking() {
        let mut truncated_header = SQLITE_MAGIC.to_vec();
        truncated_header.resize(SQLITE_APPLICATION_ID_OFFSET + SQLITE_APPLICATION_ID_LENGTH - 1, 0);

        assert!(!has_geopackage_application_id(SQLITE_MAGIC));
        assert!(!has_geopackage_application_id(&truncated_header));
    }

    #[test]
    fn should_interrupt_sqlite_work_through_the_progress_handler() {
        let mut connection = Connection::open_in_memory().expect("in-memory SQLite should open");
        let token = CancellationToken::new();
        token.cancel();
        let callback_count = Arc::new(AtomicUsize::new(0));
        let observed_count = Arc::clone(&callback_count);
        install_progress_handler(&mut connection, move || {
            observed_count.fetch_add(1, Ordering::Relaxed);
            token.is_cancelled()
        })
        .expect("the progress handler should install");

        let error = connection
            .query_row(
                "WITH RECURSIVE numbers(value) AS (\
                    VALUES(1) UNION ALL SELECT value + 1 FROM numbers WHERE value < 1000000\
                 ) SELECT sum(value) FROM numbers",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect_err("the cancelled progress handler should interrupt the query");

        assert_eq!(
            error.sqlite_error_code(),
            Some(rusqlite::ffi::ErrorCode::OperationInterrupted)
        );
        assert!(callback_count.load(Ordering::Relaxed) > 0);
    }

    #[test]
    fn should_report_the_plugin_contract() {
        let extractor = SqliteExtractor::new();
        assert_eq!(extractor.name(), "sqlite-extractor");
        assert_eq!(extractor.priority(), 50);
        assert_eq!(
            extractor.supported_mime_types(),
            &[SQLITE_MIME_TYPE, GEOPACKAGE_MIME_TYPE, "application/x-sqlite3"]
        );
    }
}
