//! Excel spreadsheet extractor.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extractors::SyncExtractor;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::internal::InternalDocument;
use crate::types::internal_builder::InternalDocumentBuilder;
use crate::types::{ExcelMetadata, Metadata};
use ahash::AHashMap;
use async_trait::async_trait;
use std::borrow::Cow;
use std::path::Path;

/// Excel spreadsheet extractor using calamine.
///
/// Supports: .xlsx, .xlsm, .xlam, .xltm, .xls, .xla, .xlsb, .ods
///
/// # Limitations
///
/// - **Hyperlinks**: calamine (v0.34) does not expose cell hyperlink data in its
///   public API. Excel files may contain hyperlinks via the `HYPERLINK()` formula
///   or via the relationships XML, but neither is accessible through the crate.
///   This would require either a calamine upstream change or manual OOXML parsing.
pub struct ExcelExtractor;

impl Default for ExcelExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl ExcelExtractor {
    pub fn new() -> Self {
        Self
    }

    /// Build an `InternalDocument` from the workbook.
    ///
    /// Each sheet becomes a table preceded by an H2 heading with the
    /// sheet name (when non-empty).
    fn build_internal_document(workbook: &crate::types::ExcelWorkbook) -> InternalDocument {
        let mut builder = InternalDocumentBuilder::new("excel");

        for (sheet_index, sheet) in workbook.sheets.iter().enumerate() {
            if let Some(ref cells) = sheet.table_cells
                && !cells.is_empty()
            {
                if !sheet.name.is_empty() {
                    builder.push_heading(2, &sheet.name, None, None);
                }
                builder.push_table_from_cells(cells, Some((sheet_index + 1) as u32), None);
            }
        }

        builder.build()
    }
}

impl Plugin for ExcelExtractor {
    fn name(&self) -> &str {
        "excel-extractor"
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
}

impl ExcelExtractor {
    /// Build an InternalDocument from a workbook with metadata.
    fn workbook_to_internal_document(workbook: &crate::types::ExcelWorkbook) -> InternalDocument {
        let mut doc = Self::build_internal_document(workbook);

        let sheet_names: Vec<String> = workbook.sheets.iter().map(|s| s.name.clone()).collect();
        let excel_metadata = ExcelMetadata {
            sheet_count: workbook.sheets.len(),
            sheet_names,
        };

        let mut additional = AHashMap::new();
        let wb_meta = &workbook.metadata;

        // Map office metadata to standard Metadata fields
        let title = wb_meta.get("title").cloned();
        let subject = wb_meta.get("subject").cloned();
        let created_by = wb_meta.get("created_by").or_else(|| wb_meta.get("creator")).cloned();
        let modified_by = wb_meta.get("modified_by").cloned();
        let created_at = wb_meta.get("created_at").cloned();
        let modified_at = wb_meta.get("modified_at").cloned();
        let authors = created_by.as_ref().map(|a| vec![a.clone()]);
        let keywords = wb_meta.get("keywords").map(|k| {
            k.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        });
        let language = wb_meta.get("language").cloned();

        // Put remaining metadata into additional map (excluding standard fields)
        for (key, value) in &workbook.metadata {
            match key.as_str() {
                "sheet_count" | "sheet_names" | "title" | "subject" | "created_by" | "creator" | "modified_by"
                | "created_at" | "modified_at" | "keywords" | "language" => {}
                _ => {
                    additional.insert(Cow::Owned(key.clone()), serde_json::json!(value));
                }
            }
        }

        doc.metadata = Metadata {
            title,
            subject,
            authors,
            keywords,
            language,
            created_at,
            modified_at,
            created_by,
            modified_by,
            format: Some(crate::types::FormatMetadata::Excel(excel_metadata)),
            additional,
            ..Default::default()
        };

        doc
    }
}

impl SyncExtractor for ExcelExtractor {
    fn extract_sync(&self, content: &[u8], mime_type: &str, _config: &ExtractionConfig) -> Result<InternalDocument> {
        let _span = tracing::debug_span!(
            "extract_excel",
            sheet_count = tracing::field::Empty,
            element_count = tracing::field::Empty,
        )
        .entered();

        let extension = match mime_type {
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => ".xlsx",
            "application/vnd.ms-excel.sheet.macroEnabled.12" => ".xlsm",
            "application/vnd.ms-excel.addin.macroEnabled.12" => ".xlam",
            "application/vnd.ms-excel.template.macroEnabled.12" => ".xltm",
            "application/vnd.ms-excel" => ".xls",
            "application/vnd.ms-excel.addin.macroEnabled" => ".xla",
            "application/vnd.ms-excel.sheet.binary.macroEnabled.12" => ".xlsb",
            "application/vnd.oasis.opendocument.spreadsheet" => ".ods",
            _ => ".xlsx",
        };

        let workbook = crate::extraction::excel::read_excel_bytes(content, extension)?;
        let mut doc = Self::workbook_to_internal_document(&workbook);
        doc.mime_type = Cow::Owned(mime_type.to_string());
        Ok(doc)
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl DocumentExtractor for ExcelExtractor {
    #[cfg_attr(feature = "otel", tracing::instrument(
        skip(self, content, config),
        fields(
            extractor.name = self.name(),
            content.size_bytes = content.len(),
        )
    ))]
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        let extension = match mime_type {
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => ".xlsx",
            "application/vnd.ms-excel.sheet.macroEnabled.12" => ".xlsm",
            "application/vnd.ms-excel.addin.macroEnabled.12" => ".xlam",
            "application/vnd.ms-excel.template.macroEnabled.12" => ".xltm",
            "application/vnd.ms-excel" => ".xls",
            "application/vnd.ms-excel.addin.macroEnabled" => ".xla",
            "application/vnd.ms-excel.sheet.binary.macroEnabled.12" => ".xlsb",
            "application/vnd.oasis.opendocument.spreadsheet" => ".ods",
            _ => ".xlsx",
        };

        let workbook = {
            #[cfg(feature = "tokio-runtime")]
            {
                if crate::core::batch_mode::is_batch_mode() {
                    if config.cancel_token.as_ref().map(|t| t.is_cancelled()).unwrap_or(false) {
                        return Err(crate::error::KreuzbergError::Cancelled);
                    }
                    let content_owned = content.to_vec();
                    let extension_owned = extension.to_string();
                    let span = tracing::Span::current();
                    tokio::task::spawn_blocking(move || {
                        let _guard = span.entered();
                        crate::extraction::excel::read_excel_bytes(&content_owned, &extension_owned)
                    })
                    .await
                    .map_err(|e| {
                        crate::error::KreuzbergError::parsing(format!("Excel extraction task failed: {}", e))
                    })??
                } else {
                    crate::extraction::excel::read_excel_bytes(content, extension)?
                }
            }
            #[cfg(not(feature = "tokio-runtime"))]
            {
                crate::extraction::excel::read_excel_bytes(content, extension)?
            }
        };

        let mut doc = Self::workbook_to_internal_document(&workbook);
        doc.mime_type = Cow::Owned(mime_type.to_string());
        Ok(doc)
    }

    #[cfg_attr(feature = "otel", tracing::instrument(
        skip(self, path, _config),
        fields(
            extractor.name = self.name(),
        )
    ))]
    async fn extract_file(&self, path: &Path, mime_type: &str, _config: &ExtractionConfig) -> Result<InternalDocument> {
        let path_str = path
            .to_str()
            .ok_or_else(|| crate::KreuzbergError::validation("Invalid file path".to_string()))?;

        let workbook = crate::extraction::excel::read_excel_file(path_str)?;
        let mut doc = Self::workbook_to_internal_document(&workbook);
        doc.mime_type = Cow::Owned(mime_type.to_string());
        Ok(doc)
    }

    fn supported_mime_types(&self) -> &[&str] {
        &[
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            "application/vnd.ms-excel.sheet.macroEnabled.12",
            "application/vnd.ms-excel.addin.macroEnabled.12",
            "application/vnd.ms-excel.template.macroEnabled.12",
            "application/vnd.ms-excel",
            "application/vnd.ms-excel.addin.macroEnabled",
            "application/vnd.ms-excel.sheet.binary.macroEnabled.12",
            "application/vnd.oasis.opendocument.spreadsheet",
            "application/vnd.openxmlformats-officedocument.spreadsheetml.template",
        ]
    }

    fn priority(&self) -> i32 {
        50
    }

    fn as_sync_extractor(&self) -> Option<&dyn SyncExtractor> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_excel_extractor_plugin_interface() {
        let extractor = ExcelExtractor::new();
        assert_eq!(extractor.name(), "excel-extractor");
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[test]
    fn test_excel_extractor_supported_mime_types() {
        let extractor = ExcelExtractor::new();
        let mime_types = extractor.supported_mime_types();
        assert_eq!(mime_types.len(), 9);
        assert!(mime_types.contains(&"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"));
        assert!(mime_types.contains(&"application/vnd.ms-excel"));
    }
}
