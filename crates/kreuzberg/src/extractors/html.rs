//! HTML document extractor.

use crate::Result;
use crate::core::config::{ExtractionConfig, OutputFormat};
use crate::extractors::SyncExtractor;
use crate::extractors::security::SecurityBudget;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::text::utf8_validation;
use crate::types::document_structure::TextAnnotation;
use crate::types::extraction::ExtractedImage;
use crate::types::internal::InternalDocument;
use crate::types::internal_builder::InternalDocumentBuilder;
use crate::types::uri::{Uri, classify_uri};
use crate::types::{HtmlMetadata, Metadata, Table};
use async_trait::async_trait;
use bytes::Bytes;
use html_to_markdown_rs::InlineImageFormat;
use std::borrow::Cow;
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
    pub(crate) fn new() -> Self {
        Self
    }
}

impl HtmlExtractor {
    /// Map html-to-markdown's `DocumentStructure` into kreuzberg's `InternalDocument`.
    ///
    /// Walks the flat node array from html-to-markdown and uses `InternalDocumentBuilder`
    /// to construct the equivalent kreuzberg representation. Skips `RawBlock` nodes
    /// (script/style content) and `MetadataBlock` nodes (handled by metadata extraction).
    ///
    /// `budget` enforces hostile-input limits (iteration count, nesting depth, entity
    /// length, cumulative content size, table cells). Any limit violation is converted
    /// into a `KreuzbergError::Security` via the `?` operator.
    fn map_document_structure(
        doc_structure: &html_to_markdown_rs::types::DocumentStructure,
        inject_placeholders: bool,
        budget: &mut SecurityBudget,
    ) -> crate::Result<InternalDocument> {
        let mut b = InternalDocumentBuilder::new("html");

        // Track which nodes are list containers so we can manage open/close
        // We need to walk top-level nodes and handle children via the tree structure.
        // html-to-markdown uses a flat array with parent/children indices.
        // We do a depth-first walk using the children structure.

        let root_indices: Vec<usize> = doc_structure
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.parent.is_none())
            .map(|(i, _)| i)
            .collect();

        Self::walk_nodes(doc_structure, &root_indices, &mut b, inject_placeholders, budget)?;

        Ok(b.build())
    }

    /// Recursively walk document nodes and push them into the builder.
    ///
    /// Calls `budget` validators on each node visited to enforce hostile-input limits.
    fn walk_nodes(
        doc: &html_to_markdown_rs::types::DocumentStructure,
        indices: &[usize],
        b: &mut InternalDocumentBuilder,
        inject_placeholders: bool,
        budget: &mut SecurityBudget,
    ) -> crate::Result<()> {
        use html_to_markdown_rs::types::NodeContent as HC;

        for &idx in indices {
            budget.step()?;

            let Some(node) = doc.nodes.get(idx) else {
                continue;
            };

            match &node.content {
                HC::Heading { level, text } => {
                    budget.account_text(text.len())?;
                    let elem_idx = b.push_heading(*level, text, None, None);
                    let annotations = map_annotations(&node.annotations);
                    push_link_uris_from_annotations(&annotations, text, b);
                    if !annotations.is_empty() {
                        b.set_annotations(elem_idx, annotations);
                    }
                }
                HC::Paragraph { text } => {
                    budget.account_text(text.len())?;
                    let annotations = map_annotations(&node.annotations);
                    push_link_uris_from_annotations(&annotations, text, b);
                    b.push_paragraph(text, annotations, None, None);
                }
                HC::List { ordered } => {
                    budget.enter()?;
                    b.push_list(*ordered);
                    let child_indices: Vec<usize> = node.children.iter().map(|&i| i as usize).collect();
                    Self::walk_nodes(doc, &child_indices, b, inject_placeholders, budget)?;
                    b.end_list();
                    budget.leave();
                }
                HC::ListItem { text } => {
                    budget.enter()?;
                    // Determine if parent is ordered
                    let ordered = node
                        .parent
                        .and_then(|p| doc.nodes.get(p as usize))
                        .map(|parent| matches!(parent.content, HC::List { ordered: true }))
                        .unwrap_or(false);
                    let annotations = map_annotations(&node.annotations);
                    push_link_uris_from_annotations(&annotations, text, b);
                    budget.account_text(text.len())?;
                    b.push_list_item(text, ordered, annotations, None, None);

                    // Recurse into children (e.g. nested lists inside this item)
                    if !node.children.is_empty() {
                        let child_indices: Vec<usize> = node.children.iter().map(|&i| i as usize).collect();
                        Self::walk_nodes(doc, &child_indices, b, inject_placeholders, budget)?;
                    }
                    budget.leave();
                }
                HC::Table { grid } => {
                    let cell_count = grid.cells.len();
                    budget.add_cells(cell_count)?;
                    // Convert grid to 2D cells for the builder
                    let mut cells = vec![vec![String::new(); grid.cols as usize]; grid.rows as usize];
                    for cell in &grid.cells {
                        if (cell.row as usize) < cells.len() && (cell.col as usize) < cells[0].len() {
                            cells[cell.row as usize][cell.col as usize] = cell.content.clone();
                        }
                    }
                    b.push_table_from_cells(&cells, None, None);
                }
                HC::Image { description, src, .. } => {
                    let text = description.as_deref().unwrap_or("");
                    if inject_placeholders && (!text.is_empty() || src.is_some()) {
                        let display = if let Some(src) = src {
                            if text.is_empty() {
                                format!("![]({})", src)
                            } else {
                                format!("![{}]({})", text, src)
                            }
                        } else {
                            text.to_string()
                        };
                        budget.account_text(display.len())?;
                        b.push_paragraph(&display, vec![], None, None);
                    }
                    // Always collect image URI reference regardless of inject_placeholders
                    if let Some(img_src) = src.as_ref().filter(|s| !s.is_empty()) {
                        b.push_uri(Uri::image(img_src.as_str(), description.clone()));
                    }
                }
                HC::Code { text, language } => {
                    budget.account_text(text.len())?;
                    b.push_code(text, language.as_deref(), None, None);
                }
                HC::Quote => {
                    budget.enter()?;
                    b.push_quote_start();
                    let child_indices: Vec<usize> = node.children.iter().map(|&i| i as usize).collect();
                    Self::walk_nodes(doc, &child_indices, b, inject_placeholders, budget)?;
                    b.push_quote_end();
                    budget.leave();
                }
                HC::DefinitionList => {
                    budget.enter()?;
                    // Walk children (DefinitionItem nodes)
                    let child_indices: Vec<usize> = node.children.iter().map(|&i| i as usize).collect();
                    Self::walk_nodes(doc, &child_indices, b, inject_placeholders, budget)?;
                    budget.leave();
                }
                HC::DefinitionItem { term, definition } => {
                    budget.account_text(term.len())?;
                    budget.account_text(definition.len())?;
                    b.push_definition_term(term, None);
                    b.push_definition_description(definition, None);
                }
                HC::Group { label, .. } => {
                    budget.enter()?;
                    b.push_group_start(label.as_deref(), None);
                    let child_indices: Vec<usize> = node.children.iter().map(|&i| i as usize).collect();
                    Self::walk_nodes(doc, &child_indices, b, inject_placeholders, budget)?;
                    b.push_group_end();
                    budget.leave();
                }
                // Skip RawBlock (script/style content) and MetadataBlock (handled by metadata extraction)
                HC::RawBlock { .. } | HC::MetadataBlock { .. } => {}
            }
        }

        Ok(())
    }
}

/// Map html-to-markdown annotations to kreuzberg annotations.
fn map_annotations(annotations: &[html_to_markdown_rs::types::TextAnnotation]) -> Vec<TextAnnotation> {
    annotations
        .iter()
        .map(|a| {
            use html_to_markdown_rs::types::AnnotationKind as AK;
            let kind = match &a.kind {
                AK::Bold => crate::types::document_structure::AnnotationKind::Bold,
                AK::Italic => crate::types::document_structure::AnnotationKind::Italic,
                AK::Underline => crate::types::document_structure::AnnotationKind::Underline,
                AK::Strikethrough => crate::types::document_structure::AnnotationKind::Strikethrough,
                AK::Code => crate::types::document_structure::AnnotationKind::Code,
                AK::Subscript => crate::types::document_structure::AnnotationKind::Subscript,
                AK::Superscript => crate::types::document_structure::AnnotationKind::Superscript,
                AK::Highlight => crate::types::document_structure::AnnotationKind::Highlight,
                AK::Link { url, title } => crate::types::document_structure::AnnotationKind::Link {
                    url: url.clone(),
                    title: title.clone(),
                },
            };
            TextAnnotation {
                start: a.start,
                end: a.end,
                kind,
            }
        })
        .collect()
}

/// Extract URIs from link annotations and push them into the builder.
fn push_link_uris_from_annotations(annotations: &[TextAnnotation], text: &str, b: &mut InternalDocumentBuilder) {
    for ann in annotations {
        if let crate::types::document_structure::AnnotationKind::Link { url, .. } = &ann.kind {
            if url.is_empty() {
                continue;
            }
            let label = if ann.start < ann.end && (ann.end as usize) <= text.len() {
                let slice = &text[ann.start as usize..ann.end as usize];
                if slice.is_empty() {
                    None
                } else {
                    Some(slice.to_string())
                }
            } else {
                None
            };
            b.push_uri(Uri {
                url: url.clone(),
                label,
                page: None,
                kind: classify_uri(url),
            });
        }
    }
}

/// Normalize markdown output from html-to-markdown-rs to comply with GFM lint rules.
///
/// html-to-markdown-rs may produce:
/// - Setext-style headings (`text\n===` or `text\n---`) instead of ATX (`# text`)
/// - Lines with trailing whitespace
/// - ATX headings without a preceding blank line
///
/// This function normalizes all three issues. Only called when `pre_rendered_content`
/// is about to be stored as the Markdown output of an HTML extraction.
pub(crate) fn normalize_html_markdown(raw: String) -> String {
    let lines: Vec<&str> = raw.lines().collect();
    // First pass: convert setext headings → ATX headings, strip trailing whitespace.
    // This produces a flat Vec<String> where setext pairs are collapsed to a single line.
    let mut pass1: Vec<String> = Vec::with_capacity(lines.len());
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let line_trimmed = line.trim_end(); // strip trailing whitespace

        // Check whether the *next* line is a setext heading underline.
        // setext H1: next line is all `=` with length >= 1
        // setext H2: next line is all `-` with length >= 1 (but not a table separator
        //            which starts after a `|`-prefixed line)
        if i + 1 < lines.len() {
            let next = lines[i + 1].trim();
            let is_setext_h1 = !next.is_empty() && next.chars().all(|c| c == '=');
            // Only treat as setext H2 if the current line is non-empty (to avoid
            // converting horizontal rules or list separators).
            let is_setext_h2 = !next.is_empty()
                && next.chars().all(|c| c == '-')
                && !line_trimmed.trim().is_empty()
                && !line_trimmed.trim().starts_with('|');

            if is_setext_h1 {
                // Emit as ATX H1 and skip the underline
                let heading_text = line_trimmed.trim();
                pass1.push(format!("# {heading_text}"));
                i += 2; // skip current line + underline
                continue;
            }
            if is_setext_h2 {
                // Emit as ATX H2 and skip the underline
                let heading_text = line_trimmed.trim();
                pass1.push(format!("## {heading_text}"));
                i += 2; // skip current line + underline
                continue;
            }
        }

        pass1.push(line_trimmed.to_string());
        i += 1;
    }

    // Second pass: ensure a blank line before every ATX heading that is not at
    // the start of the file and not already preceded by a blank line.
    let mut result = String::with_capacity(raw.len());
    for (idx, line) in pass1.iter().enumerate() {
        let is_atx_heading = line.starts_with('#');
        if is_atx_heading && idx > 0 {
            let prev = &pass1[idx - 1];
            if !prev.is_empty() {
                result.push('\n'); // inject blank line separator
            }
        }
        result.push_str(line);
        result.push('\n');
    }

    // Ensure single trailing newline (GFM rule)
    let trimmed_len = result.trim_end().len();
    if trimmed_len == 0 {
        return String::new();
    }
    result.truncate(trimmed_len);
    result.push('\n');
    result
}

/// Merge content filter settings into HTML conversion options.
///
/// When `content_filter` is `Some(...)`, adds `"header"` and/or `"footer"` to
/// `strip_tags` so `html-to-markdown-rs` removes those elements during conversion.
/// When `content_filter` is `None`, returns the options unchanged (preserving
/// current default behavior).
pub(crate) fn apply_content_filter_to_html_options(
    options: Option<html_to_markdown_rs::ConversionOptions>,
    content_filter: Option<&crate::core::config::ContentFilterConfig>,
) -> Option<html_to_markdown_rs::ConversionOptions> {
    let Some(filter) = content_filter else {
        return options;
    };

    let mut tags_to_strip: Vec<String> = Vec::new();
    if !filter.include_headers {
        tags_to_strip.push("header".to_string());
    }
    if !filter.include_footers {
        tags_to_strip.push("footer".to_string());
    }

    if tags_to_strip.is_empty() {
        return options;
    }

    let mut opts = options.unwrap_or_default();
    // Merge with any existing strip_tags rather than replacing them.
    for tag in tags_to_strip {
        if !opts.strip_tags.contains(&tag) {
            opts.strip_tags.push(tag);
        }
    }
    Some(opts)
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
    fn extract_sync(&self, content: &[u8], mime_type: &str, config: &ExtractionConfig) -> Result<InternalDocument> {
        let _span = tracing::debug_span!("extract_html", element_count = tracing::field::Empty,).entered();

        let html = utf8_validation::from_utf8(content)
            .map(|s| s.to_string())
            .unwrap_or_else(|_| String::from_utf8_lossy(content).into_owned());

        // Apply content filter to HTML conversion options: strip <header>/<footer>
        // elements when the corresponding flags are false.
        let html_options =
            apply_content_filter_to_html_options(config.html_options.clone(), config.content_filter.as_ref());

        let (content_text, html_metadata, table_data, doc_structure) =
            crate::extraction::html::convert_html_to_markdown_with_tables(
                &html,
                html_options,
                Some(config.output_format.clone()),
            )?;

        let mut budget = SecurityBudget::from_config(config);

        let mut tables: Vec<Table> = Vec::with_capacity(table_data.len());
        for (i, t) in table_data.into_iter().enumerate() {
            let grid = &t.grid;
            budget.add_cells(grid.cells.len())?;
            let mut cells = vec![vec![String::new(); grid.cols as usize]; grid.rows as usize];
            for cell in &grid.cells {
                if (cell.row as usize) < cells.len() && (cell.col as usize) < cells[0].len() {
                    cells[cell.row as usize][cell.col as usize] = cell.content.clone();
                }
            }
            tables.push(Table {
                cells,
                markdown: t.markdown,
                page_number: i + 1,
                bounding_box: None,
            });
        }

        // Extract standard metadata fields from HtmlMetadata before consuming into FormatMetadata
        let meta_title = html_metadata.as_ref().and_then(|m| m.title.clone());
        let meta_authors = html_metadata
            .as_ref()
            .and_then(|m| m.author.as_ref().map(|a| vec![a.clone()]));
        let meta_language = html_metadata.as_ref().and_then(|m| m.language.clone());
        let meta_subject = html_metadata.as_ref().and_then(|m| m.description.clone());
        let meta_keywords = html_metadata.as_ref().and_then(|m| {
            if m.keywords.is_empty() {
                None
            } else {
                Some(m.keywords.clone())
            }
        });

        let format_metadata = html_metadata.map(|m: HtmlMetadata| crate::types::FormatMetadata::Html(Box::new(m)));

        // Signal that the extractor already formatted the output so the pipeline
        // does not double-convert.
        // Markdown content is normalized to GFM: setext headings → ATX, trailing
        // whitespace removed. Djot is passed through as-is (different lint rules apply).
        let (pre_formatted, pre_rendered) = match config.output_format {
            OutputFormat::Markdown => {
                let normalized = normalize_html_markdown(content_text.clone());
                (Some("markdown".to_string()), Some(normalized))
            }
            OutputFormat::Djot => (Some("djot".to_string()), Some(content_text.clone())),
            _ => (None, None),
        };

        // Build InternalDocument from html-to-markdown's DocumentStructure.
        // If the structure has nodes, map them to InternalDocument elements.
        // Otherwise, fall back to a single paragraph with the converter's text output.
        let inject_placeholders = config
            .images
            .as_ref()
            .map(|img| img.inject_placeholders)
            .unwrap_or(true);

        let mut doc = if let Some(ref structure) = doc_structure {
            let mapped = Self::map_document_structure(structure, inject_placeholders, &mut budget)?;
            if mapped.elements.is_empty() && !content_text.is_empty() {
                // Structure collector didn't produce nodes (e.g. only images/lists which
                // aren't collected yet). Use the converter's text as a paragraph.
                let mut b = InternalDocumentBuilder::new("html");
                b.push_paragraph(&content_text, vec![], None, None);
                b.build()
            } else {
                mapped
            }
        } else if !content_text.is_empty() {
            let mut b = InternalDocumentBuilder::new("html");
            b.push_paragraph(&content_text, vec![], None, None);
            b.build()
        } else {
            InternalDocumentBuilder::new("html").build()
        };

        doc.metadata = Metadata {
            title: meta_title,
            authors: meta_authors,
            language: meta_language,
            subject: meta_subject,
            keywords: meta_keywords,
            output_format: pre_formatted,
            format: format_metadata,
            ..Default::default()
        };
        doc.mime_type = std::borrow::Cow::Owned(mime_type.to_string());
        doc.pre_rendered_content = pre_rendered;

        // Add tables to InternalDocument
        for table in tables {
            doc.push_table(table);
        }

        // Extract inline images when image extraction is configured
        let should_extract_images = config.images.as_ref().map(|i| i.extract_images).unwrap_or(false);

        if should_extract_images {
            let image_html_options =
                apply_content_filter_to_html_options(config.html_options.clone(), config.content_filter.as_ref());
            let inline_images = crate::extraction::html::extract_html_inline_images(&html, image_html_options)?;

            for (i, img) in inline_images.into_iter().enumerate() {
                let (width, height) = img.dimensions.map_or((None, None), |(w, h)| (Some(w), Some(h)));
                let format: Cow<'static, str> = match img.format {
                    InlineImageFormat::Png => Cow::Borrowed("png"),
                    InlineImageFormat::Jpeg => Cow::Borrowed("jpeg"),
                    InlineImageFormat::Gif => Cow::Borrowed("gif"),
                    InlineImageFormat::Bmp => Cow::Borrowed("bmp"),
                    InlineImageFormat::Webp => Cow::Borrowed("webp"),
                    InlineImageFormat::Svg => Cow::Borrowed("svg"),
                    InlineImageFormat::Other(ref s) => Cow::Owned(s.clone()),
                };

                let extracted = ExtractedImage {
                    data: Bytes::from(img.data),
                    format,
                    image_index: i,
                    page_number: None,
                    width,
                    height,
                    colorspace: None,
                    bits_per_component: None,
                    is_mask: false,
                    description: img.description,
                    ocr_result: None,
                    bounding_box: None,
                    source_path: None,
                };
                doc.push_image(extracted);
            }
        }

        _span.record("element_count", doc.elements.len());

        Ok(doc)
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl DocumentExtractor for HtmlExtractor {
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        self.extract_sync(content, mime_type, config)
    }

    #[cfg(feature = "tokio-runtime")]
    #[cfg_attr(feature = "otel", tracing::instrument(
        skip(self, path, config),
        fields(
            extractor.name = self.name(),
        )
    ))]
    async fn extract_file(&self, path: &Path, mime_type: &str, config: &ExtractionConfig) -> Result<InternalDocument> {
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

    /// Helper to extract tables from HTML using the unified converter.
    fn extract_tables(html: &str) -> Vec<Table> {
        let (_, _, table_data, _): (String, _, Vec<html_to_markdown_rs::types::TableData>, _) =
            crate::extraction::html::convert_html_to_markdown_with_tables(html, None, None).unwrap();
        table_data
            .into_iter()
            .enumerate()
            .map(|(i, t)| {
                let grid = &t.grid;
                let mut cells = vec![vec![String::new(); grid.cols as usize]; grid.rows as usize];
                for cell in &grid.cells {
                    if (cell.row as usize) < cells.len() && (cell.col as usize) < cells[0].len() {
                        cells[cell.row as usize][cell.col as usize] = cell.content.clone();
                    }
                }
                Table {
                    cells,
                    markdown: t.markdown,
                    page_number: i + 1,
                    bounding_box: None,
                }
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
        let result =
            crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);

        // Tables come from document structure extraction (single source now)
        assert!(!result.tables.is_empty(), "Should have at least one table");
        // Verify table content
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
        let result =
            crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);

        assert_eq!(result.mime_type, "text/html");
        assert!(
            result.content.contains("Test Page"),
            "Should contain heading text: {}",
            result.content
        );
        assert!(
            result.content.contains("emphasis"),
            "Should contain emphasis text: {}",
            result.content
        );
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
        let result =
            crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);

        // Content should already be in djot format
        assert_eq!(result.mime_type, "text/html");
        let original_content = result.content.clone();

        // Simulate pipeline format application
        let pipeline_result = crate::core::pipeline::apply_output_format(result.clone(), OutputFormat::Djot);

        // Content should be identical - no re-conversion should occur
        assert_eq!(pipeline_result.content, original_content);
        assert_eq!(pipeline_result.mime_type, "text/html");
    }

    #[test]
    fn test_map_document_structure_basic() {
        let html = "<h1>Title</h1><p>Hello world.</p>";
        let (_, _, _, doc_structure) =
            crate::extraction::html::convert_html_to_markdown_with_tables(html, None, None).unwrap();
        let doc_structure = doc_structure.expect("should have document structure");
        let mut budget = SecurityBudget::with_defaults();
        let doc = HtmlExtractor::map_document_structure(&doc_structure, true, &mut budget).unwrap();
        assert!(!doc.elements.is_empty(), "Should have elements");
    }

    #[tokio::test]
    async fn test_extract_sync_plain_text_has_content() {
        let html = r#"<h1>Title</h1><p>Hello world</p>"#;
        let extractor = HtmlExtractor::new();
        let config = ExtractionConfig::default(); // Plain text
        let result = extractor
            .extract_bytes(html.as_bytes(), "text/html", &config)
            .await
            .unwrap();
        // Check that InternalDocument has elements
        assert!(
            !result.elements.is_empty(),
            "InternalDocument should have elements, got: {:?}",
            result.elements.len()
        );
        let content = result.content();
        assert!(
            content.contains("Title"),
            "Content should contain heading: '{}'",
            content
        );
    }

    #[test]
    fn test_no_css_or_script_leaking() {
        let html = r#"
        <html>
            <head>
                <style>body { color: red; } .hidden { display: none; }</style>
                <script>alert('xss');</script>
                <script type="application/ld+json">{"@type": "Article"}</script>
            </head>
            <body>
                <h1>Clean Content</h1>
                <p>This should be the only content.</p>
            </body>
        </html>
        "#;

        let doc_structure = {
            let (_, _, _, ds) =
                crate::extraction::html::convert_html_to_markdown_with_tables(html, None, None).unwrap();
            ds.expect("should have document structure")
        };
        let mut budget = SecurityBudget::with_defaults();
        let doc = HtmlExtractor::map_document_structure(&doc_structure, true, &mut budget).unwrap();

        // Check that no element contains CSS or script content
        for elem in &doc.elements {
            let text = elem.text.as_str();
            assert!(
                !text.contains("color: red"),
                "CSS should not leak into elements: {:?}",
                text
            );
            assert!(
                !text.contains("alert("),
                "Script should not leak into elements: {:?}",
                text
            );
            assert!(
                !text.contains("@type"),
                "JSON-LD should not leak into elements: {:?}",
                text
            );
        }
    }

    #[test]
    fn test_html_inject_placeholders_true() {
        let html = r#"<html><body><img src="test.png" alt="test image"></body></html>"#;
        let (_, _, _, doc_structure) =
            crate::extraction::html::convert_html_to_markdown_with_tables(html, None, None).unwrap();
        let doc_structure = doc_structure.expect("should have document structure");
        let mut budget = SecurityBudget::with_defaults();
        let doc = HtmlExtractor::map_document_structure(&doc_structure, true, &mut budget).unwrap();
        let content = doc.content();
        assert!(
            content.contains("!["),
            "inject_placeholders=true should produce image markdown placeholder, got: '{}'",
            content
        );
    }

    #[test]
    fn test_html_inject_placeholders_false() {
        let html = r#"<html><body><img src="test.png" alt="test image"></body></html>"#;
        let (_, _, _, doc_structure) =
            crate::extraction::html::convert_html_to_markdown_with_tables(html, None, None).unwrap();
        let doc_structure = doc_structure.expect("should have document structure");
        let mut budget = SecurityBudget::with_defaults();
        let doc = HtmlExtractor::map_document_structure(&doc_structure, false, &mut budget).unwrap();
        let content = doc.content();
        assert!(
            !content.contains("!["),
            "inject_placeholders=false should not produce image markdown placeholder, got: '{}'",
            content
        );
        // URI should still be collected
        assert!(
            !doc.uris.is_empty(),
            "Image URI should still be collected when inject_placeholders=false"
        );
        assert!(
            doc.uris.iter().any(|u| u.url.contains("test.png")),
            "Should have URI for test.png"
        );
    }

    // =========================================================================
    // normalize_html_markdown tests
    // =========================================================================

    #[test]
    fn test_normalize_converts_setext_h1_to_atx() {
        let input = "Title\n=====\n\nSome paragraph.\n".to_string();
        let output = normalize_html_markdown(input);
        assert!(output.contains("# Title"), "should convert setext H1, got: {output}");
        assert!(!output.contains("====="), "should remove underline, got: {output}");
    }

    #[test]
    fn test_normalize_converts_setext_h2_to_atx() {
        let input = "Subtitle\n--------\n\nSome paragraph.\n".to_string();
        let output = normalize_html_markdown(input);
        assert!(
            output.contains("## Subtitle"),
            "should convert setext H2, got: {output}"
        );
        assert!(!output.contains("--------"), "should remove underline, got: {output}");
    }

    #[test]
    fn test_normalize_strips_trailing_whitespace() {
        let input = "Line one   \nLine two\t\n".to_string();
        let output = normalize_html_markdown(input);
        for line in output.lines() {
            assert!(
                !line.ends_with(' ') && !line.ends_with('\t'),
                "line has trailing whitespace: {line:?}"
            );
        }
    }

    #[test]
    fn test_normalize_ensures_blank_line_before_atx_heading() {
        let input = "Some text\n# Heading\n".to_string();
        let output = normalize_html_markdown(input);
        let lines: Vec<&str> = output.lines().collect();
        // Find the heading line and assert the line before it is blank
        let heading_idx = lines.iter().position(|l| l.starts_with("# Heading")).unwrap();
        assert!(heading_idx > 0, "heading should not be at line 0");
        assert!(
            lines[heading_idx - 1].is_empty(),
            "blank line before ATX heading required, found: {:?}",
            lines[heading_idx - 1]
        );
    }

    #[test]
    fn test_normalize_atx_heading_at_file_start_no_blank_line_needed() {
        let input = "# Top Heading\n\nSome text.\n".to_string();
        let output = normalize_html_markdown(input.clone());
        // When heading is the very first line, no blank line should be prepended
        assert!(
            output.starts_with("# Top Heading"),
            "heading at file start should not have leading blank line, got: {output:?}"
        );
    }

    #[test]
    fn test_normalize_single_trailing_newline() {
        let input = "Content\n\n\n".to_string();
        let output = normalize_html_markdown(input);
        assert!(output.ends_with('\n'), "should end with newline");
        assert!(
            !output.ends_with("\n\n"),
            "should have exactly one trailing newline, got: {output:?}"
        );
    }

    #[test]
    fn test_normalize_does_not_convert_table_separator_as_setext_h2() {
        let input = "| Col1 | Col2 |\n|------|------|\n| A    | B    |\n".to_string();
        let output = normalize_html_markdown(input);
        assert!(
            output.contains("|------|"),
            "table separator should not be treated as setext H2, got: {output}"
        );
    }

    #[test]
    fn test_normalize_empty_input() {
        let input = String::new();
        let output = normalize_html_markdown(input);
        assert!(output.is_empty(), "empty input should produce empty output");
    }
}
