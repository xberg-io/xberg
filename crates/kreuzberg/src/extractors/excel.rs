//! Excel spreadsheet extractor.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extractors::SyncExtractor;
use crate::extractors::security::SecurityBudget;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::internal::InternalDocument;
use crate::types::internal_builder::InternalDocumentBuilder;
use crate::types::page::PageContent;
use crate::types::tables::Table;
use crate::types::{ExcelMetadata, Metadata};
use ahash::AHashMap;
use async_trait::async_trait;
use std::borrow::Cow;
use std::path::Path;
use std::sync::Arc;

/// Validate an Excel workbook against security limits.
///
/// Iterates sheets and their cells, enforcing `max_table_cells` and
/// `max_content_size` via the provided [`SecurityBudget`].
fn validate_workbook_budget(workbook: &crate::types::ExcelWorkbook, budget: &mut SecurityBudget) -> crate::Result<()> {
    for sheet in &workbook.sheets {
        if let Some(ref cells) = sheet.table_cells {
            let row_count = cells.len();
            let col_count = cells.iter().map(|r| r.len()).max().unwrap_or(0);
            budget.add_cells(row_count.saturating_mul(col_count))?;
            for row in cells {
                for cell in row {
                    budget.account_text(cell.len())?;
                }
            }
        }
    }
    Ok(())
}

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
#[cfg_attr(alef, alef(skip))]
pub struct ExcelExtractor;

impl Default for ExcelExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl ExcelExtractor {
    pub(crate) fn new() -> Self {
        Self
    }

    /// Build an `InternalDocument` from the workbook.
    ///
    /// Each sheet becomes a table preceded by an H2 heading with the sheet name (when
    /// non-empty). Additionally, `prebuilt_pages` is set to `Some(Vec<PageContent>)` with
    /// one entry per sheet so that `ExtractionResult.pages` is always `Some` for Excel.
    ///
    /// Empty sheets still produce a `PageContent` entry so the page index aligns with the
    /// sheet index. The top-level `content` remains the concatenation of all per-sheet
    /// content, preserving backward compat for callers that do not read `pages`.
    fn build_internal_document(workbook: &crate::types::ExcelWorkbook) -> InternalDocument {
        let mut builder = InternalDocumentBuilder::new("excel");
        let mut pages: Vec<PageContent> = Vec::with_capacity(workbook.sheets.len());

        for (sheet_index, sheet) in workbook.sheets.iter().enumerate() {
            let page_number = (sheet_index + 1) as u32;

            if let Some(ref cells) = sheet.table_cells
                && !cells.is_empty()
            {
                if !sheet.name.is_empty() {
                    builder.push_heading(2, &sheet.name, None, None);
                }
                builder.push_table_from_cells(cells, Some(page_number), None);

                // Build per-sheet content: heading (when named) + markdown table.
                let page_content = if sheet.name.is_empty() {
                    sheet.markdown.clone()
                } else {
                    format!("## {}\n\n{}", sheet.name, sheet.markdown)
                };

                // Wrap the sheet table in Arc for zero-copy sharing.
                let arc_table = Arc::new(Table {
                    cells: cells.clone(),
                    markdown: sheet.markdown.clone(),
                    page_number,
                    bounding_box: None,
                });

                pages.push(PageContent {
                    page_number,
                    content: page_content,
                    tables: vec![arc_table],
                    image_indices: Vec::new(),
                    hierarchy: None,
                    is_blank: Some(false),
                    layout_regions: None,
                    speaker_notes: None,
                    section_name: if sheet.name.is_empty() {
                        None
                    } else {
                        Some(sheet.name.clone())
                    },
                });
            } else {
                // Empty sheet: emit a PageContent so page index == sheet index.
                pages.push(PageContent {
                    page_number,
                    content: if sheet.name.is_empty() {
                        String::new()
                    } else {
                        format!("## {}", sheet.name)
                    },
                    tables: Vec::new(),
                    image_indices: Vec::new(),
                    hierarchy: None,
                    is_blank: Some(true),
                    layout_regions: None,
                    speaker_notes: None,
                    section_name: if sheet.name.is_empty() {
                        None
                    } else {
                        Some(sheet.name.clone())
                    },
                });
            }
        }

        let mut doc = builder.build();
        doc.prebuilt_pages = Some(pages);
        doc
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
        let sheet_count = workbook.sheets.len() as u32;
        let excel_metadata = ExcelMetadata {
            sheet_count: Some(sheet_count),
            sheet_names: Some(sheet_names),
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
                "title" | "subject" | "created_by" | "creator" | "modified_by" | "created_at" | "modified_at"
                | "keywords" | "language" => {}
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
    fn extract_sync(&self, content: &[u8], mime_type: &str, config: &ExtractionConfig) -> Result<InternalDocument> {
        let _span = tracing::debug_span!("extract_excel", element_count = tracing::field::Empty,).entered();

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
        let mut budget = SecurityBudget::from_config(config);
        validate_workbook_budget(&workbook, &mut budget)?;
        let mut doc = Self::workbook_to_internal_document(&workbook);
        doc.mime_type = mime_type.to_string();
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
                if config.cancel_token.as_ref().map(|t| t.is_cancelled()).unwrap_or(false) {
                    return Err(crate::error::KreuzbergError::Cancelled);
                }
                crate::extraction::excel::read_excel_bytes(content, extension)?
            }
        };

        let mut budget = SecurityBudget::from_config(config);
        validate_workbook_budget(&workbook, &mut budget)?;
        let mut doc = Self::workbook_to_internal_document(&workbook);
        doc.mime_type = mime_type.to_string();
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
        doc.mime_type = mime_type.to_string();
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
    use crate::types::{ExcelSheet, ExcelWorkbook};
    use std::collections::HashMap;

    fn make_sheet(name: &str, cells: Option<Vec<Vec<String>>>) -> ExcelSheet {
        let (markdown, row_count, col_count, cell_count) = match cells.as_ref() {
            Some(c) if !c.is_empty() => {
                let rows = c.len();
                let cols = c.iter().map(|r| r.len()).max().unwrap_or(0);
                let count = c.iter().flat_map(|r| r.iter()).filter(|s| !s.is_empty()).count();
                // Build a minimal markdown table for test assertions
                let mut md = String::new();
                for (i, row) in c.iter().enumerate() {
                    md.push('|');
                    for cell in row {
                        md.push(' ');
                        md.push_str(cell);
                        md.push_str(" |");
                    }
                    md.push('\n');
                    if i == 0 && c.len() > 1 {
                        md.push('|');
                        for _ in row {
                            md.push_str(" --- |");
                        }
                        md.push('\n');
                    }
                }
                (md, rows, cols, count)
            }
            _ => (String::new(), 0, 0, 0),
        };
        ExcelSheet {
            name: name.to_string(),
            markdown,
            row_count,
            col_count,
            cell_count,
            table_cells: cells,
        }
    }

    fn make_workbook(sheets: Vec<ExcelSheet>) -> ExcelWorkbook {
        ExcelWorkbook {
            sheets,
            metadata: HashMap::new(),
        }
    }

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

    #[test]
    fn test_prebuilt_pages_always_some() {
        // Even an empty workbook sets prebuilt_pages to Some(vec![]) so callers
        // can rely on .is_some() to detect that this format supports paging.
        let workbook = make_workbook(vec![]);
        let doc = ExcelExtractor::build_internal_document(&workbook);
        assert!(doc.prebuilt_pages.is_some());
        assert_eq!(doc.prebuilt_pages.unwrap().len(), 0);
    }

    #[test]
    fn test_single_sheet_produces_one_page() {
        let cells = vec![
            vec!["Name".to_string(), "Value".to_string()],
            vec!["A".to_string(), "1".to_string()],
        ];
        let workbook = make_workbook(vec![make_sheet("Sheet1", Some(cells))]);
        let doc = ExcelExtractor::build_internal_document(&workbook);

        let pages = doc.prebuilt_pages.as_ref().unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].page_number, 1);
        assert_eq!(pages[0].section_name.as_deref(), Some("Sheet1"));
        assert!(pages[0].content.contains("Sheet1"));
        assert_eq!(pages[0].tables.len(), 1);
        assert_eq!(pages[0].is_blank, Some(false));
    }

    #[test]
    fn test_single_sheet_content_matches_flat_content() {
        // The concatenation of per-sheet page content should equal what the flat
        // extraction produces (heading text + table rows visible in elements).
        let cells = vec![
            vec!["Col1".to_string(), "Col2".to_string()],
            vec!["r1".to_string(), "r2".to_string()],
        ];
        let workbook = make_workbook(vec![make_sheet("Data", Some(cells))]);
        let doc = ExcelExtractor::build_internal_document(&workbook);

        let pages = doc.prebuilt_pages.as_ref().unwrap();
        assert_eq!(pages.len(), 1);
        // Page content must start with the heading
        assert!(pages[0].content.starts_with("## Data"));
        // Page content must include the table markdown
        assert!(pages[0].content.contains("Col1"));
        assert!(pages[0].content.contains("r1"));
    }

    #[test]
    fn test_multi_sheet_workbook_produces_one_page_per_sheet() {
        let sheet1_cells = vec![
            vec!["A".to_string(), "B".to_string()],
            vec!["1".to_string(), "2".to_string()],
        ];
        let sheet2_cells = vec![vec!["X".to_string()], vec!["99".to_string()]];
        let sheet3_cells = vec![vec!["P".to_string(), "Q".to_string(), "R".to_string()]];
        let workbook = make_workbook(vec![
            make_sheet("First", Some(sheet1_cells)),
            make_sheet("Second", Some(sheet2_cells)),
            make_sheet("Third", Some(sheet3_cells)),
        ]);
        let doc = ExcelExtractor::build_internal_document(&workbook);

        let pages = doc.prebuilt_pages.as_ref().unwrap();
        assert_eq!(pages.len(), 3);

        // Page numbers are 1-indexed and match sheet order
        assert_eq!(pages[0].page_number, 1);
        assert_eq!(pages[1].page_number, 2);
        assert_eq!(pages[2].page_number, 3);

        // Section names match sheet names
        assert_eq!(pages[0].section_name.as_deref(), Some("First"));
        assert_eq!(pages[1].section_name.as_deref(), Some("Second"));
        assert_eq!(pages[2].section_name.as_deref(), Some("Third"));

        // Each page's content is only that sheet's markdown (not other sheets)
        assert!(pages[0].content.contains("First"));
        assert!(!pages[0].content.contains("Second"));
        assert!(pages[1].content.contains("Second"));
        assert!(!pages[1].content.contains("First"));
        assert!(pages[2].content.contains("Third"));

        // Each page has exactly one table
        assert_eq!(pages[0].tables.len(), 1);
        assert_eq!(pages[1].tables.len(), 1);
        assert_eq!(pages[2].tables.len(), 1);

        // Tables carry the right cells
        assert_eq!(pages[0].tables[0].cells[0][0], "A");
        assert_eq!(pages[1].tables[0].cells[0][0], "X");
        assert_eq!(pages[2].tables[0].cells[0][0], "P");
    }

    #[test]
    fn test_empty_sheet_in_middle_produces_page_at_correct_index() {
        // An empty sheet must still emit a PageContent so page index == sheet index.
        let sheet1_cells = vec![vec!["A".to_string()]];
        let sheet3_cells = vec![vec!["C".to_string()]];
        let workbook = make_workbook(vec![
            make_sheet("First", Some(sheet1_cells)),
            make_sheet("Empty", None), // empty — no table_cells
            make_sheet("Third", Some(sheet3_cells)),
        ]);
        let doc = ExcelExtractor::build_internal_document(&workbook);

        let pages = doc.prebuilt_pages.as_ref().unwrap();
        assert_eq!(pages.len(), 3);

        assert_eq!(pages[0].page_number, 1);
        assert_eq!(pages[1].page_number, 2);
        assert_eq!(pages[2].page_number, 3);

        // Empty sheet has no tables and is marked blank
        assert_eq!(pages[1].tables.len(), 0);
        assert_eq!(pages[1].is_blank, Some(true));
        assert_eq!(pages[1].section_name.as_deref(), Some("Empty"));

        // Non-empty sheets are not blank
        assert_eq!(pages[0].is_blank, Some(false));
        assert_eq!(pages[2].is_blank, Some(false));
    }

    #[test]
    fn test_empty_sheet_cells_vec_produces_page_at_correct_index() {
        // table_cells = Some(vec![]) (present but empty) is treated the same as None.
        let workbook = make_workbook(vec![
            make_sheet("HasData", Some(vec![vec!["x".to_string()]])),
            make_sheet("EmptyCells", Some(vec![])),
        ]);
        let doc = ExcelExtractor::build_internal_document(&workbook);

        let pages = doc.prebuilt_pages.as_ref().unwrap();
        assert_eq!(pages.len(), 2);
        assert_eq!(pages[1].page_number, 2);
        assert_eq!(pages[1].tables.len(), 0);
        assert_eq!(pages[1].is_blank, Some(true));
    }

    #[test]
    fn test_sheet_order_preserved() {
        // Natural workbook sheet order must be reflected in page order.
        let workbook = make_workbook(vec![
            make_sheet("Z", Some(vec![vec!["z".to_string()]])),
            make_sheet("A", Some(vec![vec!["a".to_string()]])),
            make_sheet("M", Some(vec![vec!["m".to_string()]])),
        ]);
        let doc = ExcelExtractor::build_internal_document(&workbook);

        let pages = doc.prebuilt_pages.as_ref().unwrap();
        assert_eq!(pages[0].section_name.as_deref(), Some("Z"));
        assert_eq!(pages[1].section_name.as_deref(), Some("A"));
        assert_eq!(pages[2].section_name.as_deref(), Some("M"));
    }
}
