//! Comprehensive Markdown Extractor Tests
//!
//! This test suite uses Pandoc as a baseline for validating markdown extraction capabilities.
//! It tests:
//! - YAML frontmatter metadata extraction (both standard and extended fields)
//! - Table extraction from various markdown table formats
//! - Complex formatting and structure preservation
//! - Comparison with Pandoc's metadata extraction capabilities

use std::path::PathBuf;

use xberg::core::config::{ExtractInput, ExtractionConfig, OutputFormat};
use xberg::extractors::markdown::MarkdownExtractor;
use xberg::plugins::DocumentExtractor;
use xberg::types::ExtractedDocument;

mod helpers;
use helpers::extract_bytes_document;

fn markdown_fixture_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test_documents/markdown")
        .join(relative)
}

fn read_markdown_fixture(relative: &str) -> Vec<u8> {
    let path = markdown_fixture_path(relative);
    std::fs::read(&path).unwrap_or_else(|err| panic!("Failed to read markdown fixture {}: {}", path.display(), err))
}

async fn extract_markdown(content: &[u8], mime_type: &str) -> xberg::Result<ExtractedDocument> {
    let extractor = MarkdownExtractor;
    let input = ExtractInput::from_bytes(content.to_vec(), mime_type.to_string(), None);
    extractor.extract(input, &ExtractionConfig::default()).await
}

/// Test comprehensive YAML frontmatter with all Pandoc-recognized fields
#[tokio::test]
async fn test_pandoc_baseline_yaml_fields() {
    let markdown_with_yaml = b"---\ntitle: Test Document\nauthor: John Doe\ndate: 2024-01-15\nkeywords:\n  - markdown\n  - testing\n  - rust\ndescription: A comprehensive test document\nabstract: This is an abstract\nsubject: Testing Subject\ncategory: Documentation\ntags:\n  - important\n  - draft\nlanguage: en\nversion: 1.0.0\n---\n\n# Content\n\nThis is the main content.";

    let result = extract_markdown(markdown_with_yaml, "text/markdown")
        .await
        .expect("Should extract markdown with frontmatter");

    assert_eq!(result.metadata.title.as_deref(), Some("Test Document"));
    assert_eq!(result.metadata.created_by.as_deref(), Some("John Doe"));
    assert_eq!(result.metadata.created_at, Some("2024-01-15".to_string()));

    assert!(result.metadata.keywords.is_some());
    let keywords = result.metadata.keywords.as_ref().unwrap();
    assert!(keywords.contains(&"markdown".to_string()));
    assert!(keywords.contains(&"testing".to_string()));
    assert!(keywords.contains(&"rust".to_string()));

    assert_eq!(result.metadata.abstract_text.as_deref(), Some("This is an abstract"));

    assert_eq!(result.metadata.subject, Some("Testing Subject".to_string()));

    assert_eq!(result.metadata.category.as_deref(), Some("Documentation"));

    assert!(result.metadata.tags.is_some());
    let tags = result.metadata.tags.as_ref().unwrap();
    assert!(tags.contains(&"important".to_string()));
    assert!(tags.contains(&"draft".to_string()));

    assert_eq!(result.metadata.language.as_deref(), Some("en"));

    assert_eq!(result.metadata.document_version.as_deref(), Some("1.0.0"));
}

/// Test table extraction from pipe-format markdown tables
#[tokio::test]
async fn test_extract_simple_pipe_tables() {
    let markdown = b"# Tables Example\n\n| Header 1 | Header 2 | Header 3 |\n|----------|----------|----------|\n| Row1Col1 | Row1Col2 | Row1Col3 |\n| Row2Col1 | Row2Col2 | Row2Col3 |";

    let result = extract_markdown(markdown, "text/markdown")
        .await
        .expect("Should extract tables");

    assert!(!result.tables.is_empty(), "Should extract at least one table");
    let table = &result.tables[0];

    assert_eq!(table.cells.len(), 3, "Should have 3 rows (header + 2 data rows)");
    assert_eq!(table.cells[0].len(), 3, "Should have 3 columns");

    assert_eq!(table.cells[0][0], "Header 1");
    assert_eq!(table.cells[0][1], "Header 2");
    assert_eq!(table.cells[0][2], "Header 3");

    assert_eq!(table.cells[1][0], "Row1Col1");
    assert_eq!(table.cells[2][0], "Row2Col1");

    assert!(table.markdown.contains("Header 1"));
    assert!(table.markdown.contains("Row1Col1"));
    assert!(table.markdown.contains("---"));
}

/// Test extraction of grid tables (as found in comprehensive.md)
#[tokio::test]
async fn test_extract_grid_tables() {
    let markdown = b"# Grid Table Example\n\n+--------+--------+\n| Cell 1 | Cell 2 |\n+========+========+\n| Cell 3 | Cell 4 |\n+--------+--------+\n| Cell 5 | Cell 6 |\n+--------+--------+";

    let result = extract_markdown(markdown, "text/markdown")
        .await
        .expect("Should extract grid tables");

    let _ = result.tables;
}

/// Test extraction of tables with multiple blocks in cells
#[tokio::test]
async fn test_extract_complex_table_cells() {
    let markdown = b"# Complex Table\n\n| Header 1 | Header 2 |\n|----------|----------|\n| - bullet 1<br/>- bullet 2 | Simple text |\n| **Bold** *italic* | `code` |";

    let result = extract_markdown(markdown, "text/markdown")
        .await
        .expect("Should extract tables with complex formatting");

    assert!(!result.tables.is_empty());
    assert!(!result.content.is_empty());
}

/// Test multiline table from tables.markdown
#[tokio::test]
async fn test_pandoc_style_multiline_table() {
    let markdown = b"Simple table with caption:\n\n    Right Left    Center  Default\n  ------- ------ -------- ---------\n       12 12        12    12\n      123 123      123    123\n        1 1         1     1\n\n  : Demonstration of simple table syntax.";

    let result = extract_markdown(markdown, "text/markdown")
        .await
        .expect("Should extract pandoc-style tables");

    assert!(result.content.contains("12") || result.content.contains("Demonstration"));
}

/// Test YAML frontmatter with author as list (Pandoc style)
#[tokio::test]
async fn test_pandoc_author_list() {
    let markdown = b"% Title\n% Author One; Author Two; Author Three\n\n# Content\n\nBody text.";

    let result = extract_markdown(markdown, "text/markdown")
        .await
        .expect("Should extract markdown");

    assert!(!result.content.is_empty());
}

/// Test YAML with array keywords field (Pandoc format)
#[tokio::test]
async fn test_keywords_array_extraction() {
    let markdown =
        b"---\ntitle: Document\nkeywords:\n  - rust\n  - markdown\n  - pandoc\n---\n\n# Main Content\n\nText here.";

    let result = extract_markdown(markdown, "text/markdown")
        .await
        .expect("Should extract keywords array");

    assert!(result.metadata.keywords.is_some());
    let keywords = result.metadata.keywords.as_ref().unwrap();
    assert!(keywords.contains(&"rust".to_string()));
    assert!(keywords.contains(&"markdown".to_string()));
    assert!(keywords.contains(&"pandoc".to_string()));
}

/// Test complex formatting in content (links, code, emphasis)
#[tokio::test]
async fn test_complex_markdown_formatting() {
    let markdown = b"# Document\n\nThis is a paragraph with [links](http://example.com) and `code blocks`.\n\n## Subsection\n\n- **Bold text**\n- *Italic text*\n- ***Bold italic***\n\n```rust\nfn main() {\n    println!(\"Hello, world!\");\n}\n```";

    let result = extract_markdown(markdown, "text/markdown")
        .await
        .expect("Should extract complex markdown");

    assert!(result.content.contains("links"));
    assert!(result.content.contains("code blocks"));
    assert!(result.content.contains("Bold text"));
    assert!(result.content.contains("println"));
}

/// Test extraction of raw HTML and LaTeX in markdown
#[tokio::test]
async fn test_raw_content_extraction() {
    let markdown = b"# Document\n\nSome text.\n\n<div>Raw HTML</div>\n\nMore text.\n\n\\\\begin{equation}\nx = y\n\\\\end{equation}";

    let result = extract_markdown(markdown, "text/markdown")
        .await
        .expect("Should extract raw content");

    assert!(!result.content.is_empty());
}

/// Test comprehensive.md from test_documents
#[tokio::test]
async fn test_comprehensive_md_extraction() {
    let markdown = read_markdown_fixture("comprehensive.md");

    let result = extract_markdown(&markdown, "text/markdown")
        .await
        .expect("Should extract comprehensive.md");

    assert!(!result.content.is_empty());

    let _has_title_or_author =
        result.metadata.additional.contains_key("title") || result.metadata.additional.contains_key("author");

    assert!(result.content.contains("Additional markdown reader tests") || result.content.contains("markdown"));

    let _ = result.tables;
}

/// Test tables.markdown from test_documents
#[tokio::test]
async fn test_tables_markdown_extraction() {
    let markdown = read_markdown_fixture("tables.markdown");

    let result = extract_markdown(&markdown, "text/markdown")
        .await
        .expect("Should extract tables.markdown");

    assert!(!result.content.is_empty());

    assert!(result.content.contains("Right") || result.content.contains("Left") || result.content.contains("table"));
}

/// Test empty YAML frontmatter handling
#[tokio::test]
async fn test_empty_frontmatter() {
    let markdown = b"---\n---\n\n# Main Title\n\nContent here.";

    let result = extract_markdown(markdown, "text/markdown")
        .await
        .expect("Should handle empty frontmatter");

    assert!(result.content.contains("Main Title"));
    assert!(result.content.contains("Content here"));
}

/// Test malformed YAML frontmatter fallback
#[tokio::test]
async fn test_malformed_frontmatter_graceful_fallback() {
    let markdown = b"---\ninvalid: yaml: syntax: here:\n---\n\nContent here.";

    let result = extract_markdown(markdown, "text/markdown")
        .await
        .expect("Should handle malformed YAML gracefully");

    assert!(!result.content.is_empty());
}

/// Test metadata field extraction for standard YAML fields
#[tokio::test]
async fn test_standard_yaml_metadata_fields() {
    let markdown =
        b"---\ntitle: Standard Fields Test\nauthor: Test Author\ndate: 2024-12-06\n---\n\n# Content\n\nTest body.";

    let result = extract_markdown(markdown, "text/markdown")
        .await
        .expect("Should extract standard fields");

    assert_eq!(result.metadata.title.as_deref(), Some("Standard Fields Test"));
    assert_eq!(result.metadata.created_by.as_deref(), Some("Test Author"));
    assert_eq!(result.metadata.created_at, Some("2024-12-06".to_string()));
}

/// Test extraction of description field (maps to subject)
#[tokio::test]
async fn test_description_to_subject_mapping() {
    let markdown = b"---\ntitle: Test\ndescription: This is the document description\n---\n\nContent.";

    let result = extract_markdown(markdown, "text/markdown")
        .await
        .expect("Should extract description");

    assert_eq!(
        result.metadata.subject,
        Some("This is the document description".to_string())
    );
}

/// Test multi-line title extraction from YAML
#[tokio::test]
async fn test_multiline_title_in_yaml() {
    let markdown = b"---\ntitle: |\n  This is a\n  multi-line title\nauthor: Test\n---\n\n# Content\n\nBody.";

    let result = extract_markdown(markdown, "text/markdown")
        .await
        .expect("Should extract multiline title");

    let title = result.metadata.title.as_deref();
    assert!(title.is_some());
}

/// Test table page numbering
#[tokio::test]
async fn test_table_page_numbering() {
    let markdown = b"# Document\n\n| A | B |\n|---|---|\n| 1 | 2 |\n\nSome text between tables.\n\n| X | Y |\n|---|---|\n| 3 | 4 |";

    let result = extract_markdown(markdown, "text/markdown")
        .await
        .expect("Should extract multiple tables");

    assert_eq!(result.tables.len(), 2);
    assert_eq!(result.tables[0].page_number, 1);
    assert_eq!(result.tables[1].page_number, 1);
}

/// Test unicode content extraction
#[tokio::test]
async fn test_unicode_markdown_extraction() {
    let markdown = "---\ntitle: Unicode Test\nauthor: 日本人\n---\n\n# こんにちは\n\nThis document has:\n- 中文 (Chinese)\n- 日本語 (Japanese)\n- Русский (Russian)\n- العربية (Arabic)".as_bytes();

    let result = extract_markdown(markdown, "text/markdown")
        .await
        .expect("Should extract unicode content");

    assert!(result.content.contains("こんにちは") || result.content.contains("Chinese"));
}

/// Test YAML list to comma-separated conversion for keywords
#[tokio::test]
async fn test_keywords_list_comma_separation() {
    let markdown = b"---\nkeywords:\n  - first\n  - second\n  - third\n---\n\nContent.";

    let result = extract_markdown(markdown, "text/markdown")
        .await
        .expect("Should extract keywords list");

    assert!(result.metadata.keywords.is_some());
    let keywords = result.metadata.keywords.as_ref().unwrap();
    assert!(keywords.contains(&"first".to_string()));
    assert!(keywords.contains(&"second".to_string()));
    assert!(keywords.contains(&"third".to_string()));
}

/// Test extraction without any frontmatter
#[tokio::test]
async fn test_no_frontmatter_extraction() {
    let markdown = b"# Document Title\n\nJust a document without frontmatter.\n\n## Section\n\nWith content.";

    let result = extract_markdown(markdown, "text/markdown")
        .await
        .expect("Should extract markdown without frontmatter");

    assert!(result.content.contains("Document Title"));
    assert!(result.content.contains("document") || result.content.contains("Section"));

    let title = result.metadata.title.as_deref();
    assert_eq!(title, Some("Document Title"));
}

/// Test code block extraction
#[tokio::test]
async fn test_code_block_extraction() {
    let markdown = b"# Code Examples\n\n```rust\nfn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n```\n\n```python\ndef add(a, b):\n    return a + b\n```";

    let result = extract_markdown(markdown, "text/markdown")
        .await
        .expect("Should extract code blocks");

    assert!(result.content.contains("add"));
    assert!(result.content.contains("return"));
}

/// Test extraction with various mime types
#[tokio::test]
async fn test_supported_mime_types() {
    let markdown = b"# Test\n\nContent.";

    for mime_type in &["text/markdown", "text/x-markdown", "text/x-gfm", "text/x-commonmark"] {
        let result = extract_markdown(markdown, mime_type)
            .await
            .unwrap_or_else(|_| panic!("Should support {}", mime_type));

        assert_eq!(result.mime_type, *mime_type);
        assert!(result.content.contains("Test"));
    }
}

/// Test that metadata extraction handles nested YAML structures
/// (Currently not fully supported - documents what's missing)
#[tokio::test]
async fn test_nested_yaml_awareness() {
    let markdown = b"---\ntitle: Test\nmetadata:\n  organization: Test Corp\n  location:\n    city: San Francisco\n    state: CA\n---\n\nContent.";

    let result = extract_markdown(markdown, "text/markdown")
        .await
        .expect("Should extract document");

    let title = result.metadata.title.as_deref();
    assert_eq!(title, Some("Test"));
}

/// Test extraction with special characters in metadata
#[tokio::test]
async fn test_special_characters_in_metadata() {
    let markdown = b"---\ntitle: \"Document: Part 1 & 2\"\nauthor: O'Brien\nkeywords: \"C++, C#, F#\"\n---\n\nContent.";

    let result = extract_markdown(markdown, "text/markdown")
        .await
        .expect("Should extract with special characters");

    let title = result.metadata.title.as_deref();
    assert!(title.is_some());
    assert!(title.expect("Operation failed").contains("&") || title.expect("Operation failed").contains("Part"));
}

// ── Quarto / R Markdown / pandoc-flavored markdown (issue #1203) ──────────────

/// Extract through the full pipeline (registry routing + output-format renderer)
/// so the rendered `content` reflects the chosen `OutputFormat`.
async fn extract_markdown_as(
    content: &[u8],
    mime_type: &str,
    format: OutputFormat,
) -> xberg::Result<ExtractedDocument> {
    let config = ExtractionConfig {
        output_format: format,
        ..Default::default()
    };
    extract_bytes_document(content, mime_type, &config).await
}

/// A Quarto `.qmd` is a Markdown superset: YAML frontmatter plus executable code
/// cells (` ```{python} `) that xberg renders as clean fenced code — never executed.
#[tokio::test]
async fn test_quarto_qmd_extraction_end_to_end() {
    let qmd = b"---\ntitle: Quarto Report\nauthor: Jane Doe\n---\n\n# Analysis\n\nSome prose before the cell.\n\n```{python}\nimport pandas as pd\nprint('hello')\n```\n\nMore prose.\n\n```{r, echo=FALSE}\nsummary(cars)\n```\n";

    let result = extract_markdown_as(qmd, "text/x-quarto", OutputFormat::Markdown)
        .await
        .expect("should extract Quarto document");

    assert_eq!(result.metadata.title.as_deref(), Some("Quarto Report"));
    // Executable cell braces are stripped to the bare kernel name.
    assert!(
        result.content.contains("```python"),
        "python cell renders with clean language tag"
    );
    assert!(
        result.content.contains("```r"),
        "r cell renders with clean language tag"
    );
    assert!(
        !result.content.contains("```{python}"),
        "brace-wrapped language must be normalized"
    );
    assert!(
        !result.content.contains("echo=FALSE"),
        "chunk options are dropped from the language tag"
    );
    // Code body is preserved verbatim; nothing is executed.
    assert!(result.content.contains("import pandas as pd"));
    assert!(result.content.contains("summary(cars)"));
}

/// R Markdown `.Rmd` has the same structure as Quarto and routes to the same extractor.
#[tokio::test]
async fn test_r_markdown_rmd_extraction_end_to_end() {
    let rmd = b"---\ntitle: RMarkdown Doc\n---\n\n# Section\n\n```{r}\nx <- c(1, 2, 3)\nmean(x)\n```\n";

    let result = extract_markdown_as(rmd, "text/x-r-markdown", OutputFormat::Markdown)
        .await
        .expect("should extract R Markdown document");

    assert_eq!(result.metadata.title.as_deref(), Some("RMarkdown Doc"));
    assert!(
        result.content.contains("```r"),
        "r cell renders with clean language tag"
    );
    assert!(!result.content.contains("```{r}"));
    assert!(result.content.contains("mean(x)"));
}

/// Pandoc-flavored constructs (definition lists, math, task lists) parse through the
/// full pulldown-cmark option set.
#[tokio::test]
async fn test_pandoc_flavored_markdown_end_to_end() {
    let pandoc = b"# Pandoc\n\nTerm 1\n\n:   Definition of term 1.\n\nInline math $E = mc^2$ and a display block:\n\n$$\\int_0^1 x^2 dx$$\n\n- [x] done task\n- [ ] pending task\n";

    let result = extract_markdown_as(pandoc, "text/x-pandoc", OutputFormat::Markdown)
        .await
        .expect("should extract pandoc-flavored markdown");

    // Definition list, math, and task list all survive extraction.
    assert!(
        result.content.contains("Definition of term 1"),
        "definition list body present"
    );
    assert!(result.content.contains("\\int"), "display math present");
    assert!(
        result.content.contains("done task") && result.content.contains("pending task"),
        "task list items present"
    );
}
