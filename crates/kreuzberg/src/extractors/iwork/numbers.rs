//! Apple Numbers (.numbers) extractor.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extractors::iwork::{dedup_text, extract_metadata_from_zip, extract_text_from_proto, read_iwa_file};
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::internal::InternalDocument;
use crate::types::internal_builder::InternalDocumentBuilder;
use async_trait::async_trait;

/// Apple Numbers spreadsheet extractor.
///
/// Supports `.numbers` files (modern iWork format, 2013+).
///
/// Extracts cell string values and sheet names from the IWA container:
/// ZIP → Snappy → protobuf text fields. Output is formatted as plain text
/// with one text token per line (representing cell values and labels).
pub struct NumbersExtractor;

impl NumbersExtractor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NumbersExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for NumbersExtractor {
    fn name(&self) -> &str {
        "iwork-numbers-extractor"
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
        "Apple Numbers (.numbers) text extraction via IWA container parser"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

/// Parsed Numbers data: per-sheet cell values and optional metadata.
struct NumbersData {
    /// Each entry is (sheet_name, cell_values).
    sheets: Vec<(String, Vec<String>)>,
    /// Metadata extracted from the ZIP archive.
    metadata: crate::types::metadata::Metadata,
}

/// Parse a Numbers ZIP and extract all text from IWA files.
///
/// Numbers stores its content across many IWA files:
/// - `Index/CalculationEngine.iwa` — formula cells and sheet data
/// - `Index/Document.iwa` — document structure and sheet names
/// - `tables/DataStore.iwa` — table cell string values
///
/// We group IWA paths by table/sheet and extract cell values per-sheet so the
/// output can be structured as tables rather than flat paragraphs.
fn parse_numbers(content: &[u8]) -> Result<NumbersData> {
    let iwa_paths = super::collect_iwa_paths(content)?;
    let metadata = extract_metadata_from_zip(content);

    // Separate table-related IWA files from other IWA files.
    // Numbers stores table cell data under `Index/Tables/` or in paths containing `Table`
    // and document structure in `Index/Document.iwa`.
    let mut table_paths: Vec<&String> = Vec::new();
    let mut other_paths: Vec<&String> = Vec::new();

    for path in &iwa_paths {
        if path.contains("Table") || path.contains("DataStore") {
            table_paths.push(path);
        } else {
            other_paths.push(path);
        }
    }

    // If there were no table-specific paths, treat all paths as table data
    // (older Numbers formats may not use the Table/ prefix).
    if table_paths.is_empty() {
        table_paths = iwa_paths.iter().collect();
        other_paths.clear();
    }

    // Extract table cell values
    let mut table_texts: Vec<String> = Vec::new();
    for path in &table_paths {
        match read_iwa_file(content, path) {
            Ok(decompressed) => {
                let texts = extract_text_from_proto(&decompressed);
                table_texts.extend(texts);
            }
            Err(_) => {
                tracing::debug!("Skipping IWA file (decompression failed): {path}");
            }
        }
    }

    // Extract any additional text from non-table IWA files (labels, sheet names, etc.)
    let mut other_texts: Vec<String> = Vec::new();
    for path in &other_paths {
        match read_iwa_file(content, path) {
            Ok(decompressed) => {
                let texts = extract_text_from_proto(&decompressed);
                other_texts.extend(texts);
            }
            Err(_) => {
                tracing::debug!("Skipping IWA file (decompression failed): {path}");
            }
        }
    }

    let table_deduped = dedup_text(table_texts);
    let other_deduped = dedup_text(other_texts);

    // Filter out noise tokens (common in spreadsheet binary data)
    let filter = |texts: Vec<String>| -> Vec<String> {
        texts
            .into_iter()
            .filter(|s| s.len() >= 2 && s.chars().any(|c| c.is_alphanumeric()))
            .collect()
    };

    let table_filtered = filter(table_deduped);
    let other_filtered = filter(other_deduped);

    // Build per-sheet data. Without full protobuf schema we cannot reliably
    // determine sheet boundaries, so we emit one sheet for table data and one
    // for any remaining text labels.
    let mut sheets = Vec::new();
    if !table_filtered.is_empty() {
        sheets.push(("Sheet Data".to_string(), table_filtered));
    }
    if !other_filtered.is_empty() {
        sheets.push(("Document Info".to_string(), other_filtered));
    }

    Ok(NumbersData { sheets, metadata })
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl DocumentExtractor for NumbersExtractor {
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        let data = {
            #[cfg(feature = "tokio-runtime")]
            if crate::core::batch_mode::is_batch_mode() {
                if config.cancel_token.as_ref().map(|t| t.is_cancelled()).unwrap_or(false) {
                    return Err(crate::error::KreuzbergError::Cancelled);
                }
                let content_owned = content.to_vec();
                let span = tracing::Span::current();
                tokio::task::spawn_blocking(move || {
                    let _guard = span.entered();
                    parse_numbers(&content_owned)
                })
                .await
                .map_err(|e| crate::error::KreuzbergError::parsing(format!("Numbers extraction task failed: {e}")))??
            } else {
                parse_numbers(content)?
            }

            #[cfg(not(feature = "tokio-runtime"))]
            parse_numbers(content)?
        };

        let mut doc = build_numbers_internal_document(&data);
        doc.mime_type = std::borrow::Cow::Owned(mime_type.to_string());
        Ok(doc)
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/x-iwork-numbers-sffnumbers"]
    }

    fn priority(&self) -> i32 {
        50
    }
}

/// Build an `InternalDocument` from extracted Numbers data.
///
/// Outputs cell values as tables (one per sheet) rather than flat paragraphs,
/// reflecting the spreadsheet nature of the .numbers format.
fn build_numbers_internal_document(data: &NumbersData) -> InternalDocument {
    let mut builder = InternalDocumentBuilder::new("numbers");

    // Apply any metadata we could extract from the ZIP archive.
    if data.metadata.title.is_some() || data.metadata.authors.is_some() {
        builder.set_metadata(data.metadata.clone());
    }

    for (sheet_name, cell_values) in &data.sheets {
        if cell_values.is_empty() {
            continue;
        }

        builder.push_heading(1, sheet_name, None, None);

        // Build a single-column table from the cell values. Without full
        // protobuf schema knowledge we cannot reliably determine column
        // boundaries, so each value gets its own row in a single-column table.
        let cells: Vec<Vec<String>> = cell_values.iter().map(|v| vec![v.clone()]).collect();
        builder.push_table_from_cells(&cells, None, None);
    }

    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_numbers_extractor_plugin_interface() {
        let extractor = NumbersExtractor::new();
        assert_eq!(extractor.name(), "iwork-numbers-extractor");
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[test]
    fn test_numbers_extractor_supported_mime_types() {
        let extractor = NumbersExtractor::new();
        let types = extractor.supported_mime_types();
        assert!(types.contains(&"application/x-iwork-numbers-sffnumbers"));
    }
}
