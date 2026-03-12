//! HTML document extractor.

use crate::Result;
use crate::core::config::{ExtractionConfig, OutputFormat};
use crate::extractors::SyncExtractor;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::text::utf8_validation;
use crate::types::{ExtractionResult, HtmlMetadata, Metadata, Table};
use async_trait::async_trait;
#[cfg(feature = "tokio-runtime")]
use std::path::Path;

/// HTML document extractor using html-to-markdown.
pub struct HtmlExtractor;

impl Default for HtmlExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl HtmlExtractor {
    pub fn new() -> Self {
        Self
    }
}

impl Plugin for HtmlExtractor {
    fn name(&self) -> &str {
        "html-extractor"
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

impl SyncExtractor for HtmlExtractor {
    fn extract_sync(&self, content: &[u8], mime_type: &str, config: &ExtractionConfig) -> Result<ExtractionResult> {
        let html = utf8_validation::from_utf8(content)
            .map(|s| s.to_string())
            .unwrap_or_else(|_| String::from_utf8_lossy(content).to_string());

        let (content_text, html_metadata, table_data) = crate::extraction::html::convert_html_to_markdown_with_tables(
            &html,
            config.html_options.clone(),
            Some(config.output_format),
        )?;

        let tables: Vec<Table> = table_data
            .into_iter()
            .enumerate()
            .map(|(i, t)| Table {
                cells: t.cells,
                markdown: t.markdown,
                page_number: i + 1,
                bounding_box: None,
            })
            .collect();

        let format_metadata = html_metadata.map(|m: HtmlMetadata| crate::types::FormatMetadata::Html(Box::new(m)));

        // Signal that the extractor already formatted the output so the pipeline
        // does not double-convert.
        let pre_formatted = match config.output_format {
            OutputFormat::Markdown => Some("markdown".to_string()),
            OutputFormat::Djot => Some("djot".to_string()),
            _ => None,
        };

        Ok(ExtractionResult {
            content: content_text,
            mime_type: mime_type.to_string().into(),
            metadata: Metadata {
                output_format: pre_formatted,
                format: format_metadata,
                ..Default::default()
            },
            pages: None,
            tables,
            detected_languages: None,
            chunks: None,
            images: None,
            djot_content: None,
            elements: None,
            ocr_elements: None,
            document: None,
            #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
            extracted_keywords: None,
            quality_score: None,
            processing_warnings: Vec::new(),
            annotations: None,
        })
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl DocumentExtractor for HtmlExtractor {
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
    ) -> Result<ExtractionResult> {
        self.extract_sync(content, mime_type, config)
    }

    #[cfg(feature = "tokio-runtime")]
    #[cfg_attr(feature = "otel", tracing::instrument(
        skip(self, path, config),
        fields(
            extractor.name = self.name(),
        )
    ))]
    #[cfg(feature = "tokio-runtime")]
    async fn extract_file(&self, path: &Path, mime_type: &str, config: &ExtractionConfig) -> Result<ExtractionResult> {
        let bytes = tokio::fs::read(path).await?;
        self.extract_bytes(&bytes, mime_type, config).await
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["text/html", "application/xhtml+xml"]
    }

    fn priority(&self) -> i32 {
        50
    }

    fn as_sync_extractor(&self) -> Option<&dyn crate::extractors::SyncExtractor> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to extract tables from HTML using the visitor-based converter.
    fn extract_tables(html: &str) -> Vec<Table> {
        let (_, _, table_data): (String, _, Vec<html_to_markdown_rs::TableData>) =
            crate::extraction::html::convert_html_to_markdown_with_tables(html, None, None).unwrap();
        table_data
            .into_iter()
            .enumerate()
            .map(|(i, t)| Table {
                cells: t.cells,
                markdown: t.markdown,
                page_number: i + 1,
                bounding_box: None,
            })
            .collect()
    }

    #[test]
    fn test_html_extractor_plugin_interface() {
        let extractor = HtmlExtractor::new();
        assert_eq!(extractor.name(), "html-extractor");
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[test]
    fn test_html_extractor_supported_mime_types() {
        let extractor = HtmlExtractor::new();
        let mime_types = extractor.supported_mime_types();
        assert_eq!(mime_types.len(), 2);
        assert!(mime_types.contains(&"text/html"));
        assert!(mime_types.contains(&"application/xhtml+xml"));
    }

    #[test]
    fn test_extract_html_tables_basic() {
        let html = r#"
            <table>
                <tr><th>Header1</th><th>Header2</th></tr>
                <tr><td>Row1Col1</td><td>Row1Col2</td></tr>
                <tr><td>Row2Col1</td><td>Row2Col2</td></tr>
            </table>
        "#;

        let tables = extract_tables(html);
        assert_eq!(tables.len(), 1);

        let table = &tables[0];
        assert_eq!(table.cells.len(), 3);
        assert_eq!(table.cells[0], vec!["Header1", "Header2"]);
        assert_eq!(table.cells[1], vec!["Row1Col1", "Row1Col2"]);
        assert_eq!(table.cells[2], vec!["Row2Col1", "Row2Col2"]);
        assert_eq!(table.page_number, 1);
        assert!(table.markdown.contains("Header1"));
        assert!(table.markdown.contains("Row1Col1"));
    }

    #[test]
    fn test_extract_html_tables_multiple() {
        let html = r#"
            <table>
                <tr><th>Table1</th></tr>
                <tr><td>Data1</td></tr>
            </table>
            <p>Some text</p>
            <table>
                <tr><th>Table2</th></tr>
                <tr><td>Data2</td></tr>
            </table>
        "#;

        let tables = extract_tables(html);
        assert_eq!(tables.len(), 2);
        assert_eq!(tables[0].page_number, 1);
        assert_eq!(tables[1].page_number, 2);
    }

    #[test]
    fn test_extract_html_tables_no_thead() {
        let html = r#"
            <table>
                <tr><td>Cell1</td><td>Cell2</td></tr>
                <tr><td>Cell3</td><td>Cell4</td></tr>
            </table>
        "#;

        let tables = extract_tables(html);
        assert_eq!(tables.len(), 1);

        let table = &tables[0];
        assert_eq!(table.cells.len(), 2);
        assert_eq!(table.cells[0], vec!["Cell1", "Cell2"]);
        assert_eq!(table.cells[1], vec!["Cell3", "Cell4"]);
    }

    #[test]
    fn test_extract_html_tables_empty() {
        let html = "<p>No tables here</p>";
        let tables = extract_tables(html);
        assert_eq!(tables.len(), 0);
    }

    #[test]
    fn test_extract_html_tables_with_nested_elements() {
        let html = r#"
            <table>
                <tr><th>Header <strong>Bold</strong></th></tr>
                <tr><td>Data with <em>emphasis</em></td></tr>
            </table>
        "#;

        let tables = extract_tables(html);
        assert_eq!(tables.len(), 1);

        let table = &tables[0];
        assert!(table.cells[0][0].contains("Header"));
        assert!(table.cells[0][0].contains("Bold"));
        assert!(table.cells[1][0].contains("Data with"));
        assert!(table.cells[1][0].contains("emphasis"));
    }

    #[test]
    fn test_extract_nested_html_tables() {
        let html = r#"
            <table>
                <tr>
                    <th>Category</th>
                    <th>Details &amp; Nested Data</th>
                </tr>
                <tr>
                    <td><strong>Project Alpha</strong></td>
                    <td>
                    <table>
                        <tr><th>Task ID</th><th>Status</th><th>Priority</th></tr>
                        <tr><td>001</td><td>Completed</td><td>High</td></tr>
                        <tr><td>002</td><td>In Progress</td><td>Medium</td></tr>
                    </table>
                    </td>
                </tr>
                <tr>
                    <td><strong>Project Beta</strong></td>
                    <td>No sub-tasks assigned yet.</td>
                </tr>
            </table>
        "#;

        let tables = extract_tables(html);

        // Should find at least 2 tables: outer + nested
        assert!(
            tables.len() >= 2,
            "Expected at least 2 tables (outer + nested), found {}",
            tables.len()
        );

        // Find the nested table (has Task ID header)
        let nested = tables
            .iter()
            .find(|t| {
                t.cells
                    .first()
                    .is_some_and(|row| row.iter().any(|c| c.contains("Task ID")))
            })
            .expect("Should find nested table with Task ID header");

        assert_eq!(nested.cells[0].len(), 3, "Nested table header should have 3 columns");
        assert!(nested.cells[0][0].contains("Task ID"));
        assert!(nested.cells[0][1].contains("Status"));
        assert!(nested.cells[0][2].contains("Priority"));
        assert_eq!(
            nested.cells.len(),
            3,
            "Nested table should have 3 rows (header + 2 data)"
        );
        assert!(nested.cells[1][0].contains("001"));
        assert!(nested.cells[1][1].contains("Completed"));
        assert!(nested.cells[2][0].contains("002"));
        assert!(nested.cells[2][1].contains("In Progress"));
    }

    #[tokio::test]
    async fn test_html_extractor_with_table() {
        let html = r#"
            <html>
                <body>
                    <h1>Test Page</h1>
                    <table>
                        <tr><th>Name</th><th>Age</th></tr>
                        <tr><td>Alice</td><td>30</td></tr>
                        <tr><td>Bob</td><td>25</td></tr>
                    </table>
                </body>
            </html>
        "#;

        let extractor = HtmlExtractor::new();
        let config = ExtractionConfig::default();
        let result = extractor
            .extract_bytes(html.as_bytes(), "text/html", &config)
            .await
            .unwrap();

        assert_eq!(result.tables.len(), 1);
        let table = &result.tables[0];
        assert_eq!(table.cells.len(), 3);
        assert_eq!(table.cells[0], vec!["Name", "Age"]);
        assert_eq!(table.cells[1], vec!["Alice", "30"]);
        assert_eq!(table.cells[2], vec!["Bob", "25"]);
    }

    #[tokio::test]
    async fn test_html_extractor_with_djot_output() {
        let html = r#"
        <html>
            <body>
                <h1>Test Page</h1>
                <p>Content with <strong>emphasis</strong>.</p>
            </body>
        </html>
    "#;

        let extractor = HtmlExtractor::new();
        let config = ExtractionConfig {
            output_format: OutputFormat::Djot,
            ..Default::default()
        };

        let result = extractor
            .extract_bytes(html.as_bytes(), "text/html", &config)
            .await
            .unwrap();

        assert_eq!(result.mime_type, "text/html");
        assert!(result.content.contains("# Test Page"));
        assert!(result.content.contains("*emphasis*")); // Djot strong syntax
    }

    #[tokio::test]
    async fn test_html_extractor_djot_double_conversion_prevention() {
        let html = r#"
        <html>
            <body>
                <h1>Test</h1>
                <p>Content with <strong>bold</strong> text.</p>
            </body>
        </html>
    "#;

        let extractor = HtmlExtractor::new();
        let config = ExtractionConfig {
            output_format: OutputFormat::Djot,
            ..Default::default()
        };

        let result = extractor
            .extract_bytes(html.as_bytes(), "text/html", &config)
            .await
            .unwrap();

        // Content should already be in djot format
        assert_eq!(result.mime_type, "text/html");
        let original_content = result.content.clone();

        // Simulate pipeline format application
        let mut pipeline_result = result.clone();
        crate::core::pipeline::apply_output_format(&mut pipeline_result, OutputFormat::Djot);

        // Content should be identical - no re-conversion should occur
        assert_eq!(pipeline_result.content, original_content);
        assert_eq!(pipeline_result.mime_type, "text/html");
    }
}
