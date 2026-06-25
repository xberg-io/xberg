//! Integration tests for the new rendering layer.
//!
//! These tests construct `InternalDocument` instances via the builder API,
//! run them through `derive_extraction_result`, and verify the rendered
//! output in each supported `OutputFormat`.
//!
//! Usage:
//!   cargo test -p xberg --test rendering_integration_tests

mod helpers;

use xberg::core::config::OutputFormat;
use xberg::extraction::derive::derive_extraction_result;
use xberg::types::document_structure::{AnnotationKind, TextAnnotation};
use xberg::types::internal_builder::InternalDocumentBuilder;

// ============================================================================
// Helpers
// ============================================================================

/// Build a rich document containing a heading, paragraph, list, code block,
/// and table — the structural elements every format must handle.
fn build_rich_document() -> xberg::types::internal::InternalDocument {
    let mut b = InternalDocumentBuilder::new("test");

    b.push_heading(1, "Main Heading", None, None);
    b.push_paragraph("This is a paragraph with some text.", vec![], None, None);

    b.push_list(false);
    b.push_list_item("First item", false, vec![], None, None);
    b.push_list_item("Second item", false, vec![], None, None);
    b.push_list_item("Third item", false, vec![], None, None);
    b.end_list();

    b.push_code("fn main() {\n    println!(\"hello\");\n}", Some("rust"), None, None);

    b.push_table_from_cells(
        &[
            vec!["Name".to_string(), "Value".to_string()],
            vec!["alpha".to_string(), "1".to_string()],
            vec!["beta".to_string(), "2".to_string()],
        ],
        None,
        None,
    );

    b.build()
}

/// Derive an `ExtractionResult` from a document in the given format.
fn derive(doc: xberg::types::internal::InternalDocument, format: OutputFormat) -> xberg::types::ExtractionResult {
    derive_extraction_result(doc, false, format)
}

/// Return the "effective content" — `formatted_content` when present,
/// otherwise the plain-text `content`.
fn effective_content(result: &xberg::types::ExtractionResult) -> &str {
    result.formatted_content.as_deref().unwrap_or(&result.content)
}

// ============================================================================
// 1. Markdown output preserves structure
// ============================================================================

#[tokio::test]
async fn test_markdown_output_preserves_structure() {
    let doc = build_rich_document();
    let result = derive(doc, OutputFormat::Markdown);
    let md = effective_content(&result);

    // Heading
    assert!(
        md.contains("# Main Heading"),
        "Markdown should contain an ATX heading, got:\n{md}"
    );
    // Paragraph
    assert!(
        md.contains("This is a paragraph"),
        "Markdown should contain the paragraph text"
    );
    // List items
    assert!(md.contains("First item"), "Markdown should contain list items");
    // Code block
    assert!(md.contains("```"), "Markdown should contain a fenced code block");
    assert!(md.contains("fn main()"), "Markdown code block should contain the code");
    // Table (pipe-delimited)
    assert!(md.contains('|'), "Markdown should contain pipe-delimited table syntax");
    assert!(md.contains("Name"), "Markdown table should contain header cells");
}

// ============================================================================
// 2. Djot output format through pipeline
// ============================================================================

#[tokio::test]
async fn test_djot_output_preserves_structure() {
    let doc = build_rich_document();
    let result = derive(doc, OutputFormat::Djot);
    let djot = effective_content(&result);

    // Djot headings use `#` just like markdown
    assert!(
        djot.contains("# Main Heading"),
        "Djot should contain a heading, got:\n{djot}"
    );
    // Paragraph text
    assert!(
        djot.contains("This is a paragraph"),
        "Djot should contain the paragraph text"
    );
    // Code (djot uses ``` fences too)
    assert!(djot.contains("fn main()"), "Djot should contain the code content");
}

// ============================================================================
// 3. HTML output format through pipeline
// ============================================================================

#[tokio::test]
async fn test_html_output_preserves_structure() {
    let doc = build_rich_document();
    let result = derive(doc, OutputFormat::Html);
    let html = effective_content(&result);

    assert!(html.contains("<h1"), "HTML should contain an h1 tag, got:\n{html}");
    assert!(html.contains("Main Heading"), "HTML h1 should contain the heading text");
    assert!(html.contains("<p"), "HTML should contain paragraph tags");
    assert!(html.contains("<li"), "HTML should contain list item tags");
    assert!(
        html.contains("<code") || html.contains("<pre"),
        "HTML should contain code/pre tags"
    );
    assert!(
        html.contains("<table") || html.contains("<th") || html.contains("<td"),
        "HTML should contain table markup"
    );
}

// ============================================================================
// 4. Plain text output through pipeline
// ============================================================================

#[tokio::test]
async fn test_plain_text_output_has_no_formatting() {
    let doc = build_rich_document();
    let result = derive(doc, OutputFormat::Plain);

    // For plain text the formatted_content should be None
    assert!(
        result.formatted_content.is_none(),
        "Plain format should not set formatted_content"
    );

    let plain = &result.content;

    // Should contain the words
    assert!(plain.contains("Main Heading"), "Plain text should contain heading text");
    assert!(
        plain.contains("This is a paragraph"),
        "Plain text should contain paragraph text"
    );
    assert!(plain.contains("First item"), "Plain text should contain list items");
    assert!(plain.contains("fn main()"), "Plain text should contain code content");

    // Should NOT contain markdown/html formatting
    assert!(!plain.contains("<h1"), "Plain text should not contain HTML tags");
    assert!(
        !plain.lines().any(|l| l.starts_with("# ")),
        "Plain text should not contain markdown heading syntax"
    );
}

// ============================================================================
// 5. Format switching consistency
// ============================================================================

#[tokio::test]
async fn test_format_switching_consistency() {
    // Render the same logical document to every format and check that the
    // plain-text words are present in all of them.
    let formats = [
        OutputFormat::Plain,
        OutputFormat::Markdown,
        OutputFormat::Djot,
        OutputFormat::Html,
    ];

    let expected_words = [
        "Main Heading",
        "paragraph",
        "First item",
        "Second item",
        "Third item",
        "fn main()",
        "alpha",
        "beta",
    ];

    for format in &formats {
        let doc = build_rich_document();
        let result = derive(doc, format.clone());
        let text = effective_content(&result);

        for word in &expected_words {
            assert!(
                text.contains(word),
                "Format {format:?} should contain \"{word}\" but output was:\n{text}"
            );
        }
    }
}

// ============================================================================
// 6. Footnote rendering end-to-end
// ============================================================================

#[tokio::test]
async fn test_footnote_rendering_end_to_end() {
    let mut b = InternalDocumentBuilder::new("test");

    b.push_paragraph("Text with a footnote reference.", vec![], None, None);
    b.push_footnote_ref("1", "fn1", None);
    b.push_paragraph("More text after the reference.", vec![], None, None);
    b.push_footnote_definition("This is the footnote content.", "fn1", None);

    let doc = b.build();
    let result = derive(doc, OutputFormat::Markdown);
    let md = effective_content(&result);

    // The footnote reference marker should appear
    assert!(
        md.contains("[^") || md.contains("fn1") || md.contains("[1]"),
        "Markdown should contain a footnote reference marker, got:\n{md}"
    );
    // The footnote definition content should appear
    assert!(
        md.contains("This is the footnote content"),
        "Markdown should contain the footnote definition text, got:\n{md}"
    );
}

// ============================================================================
// 7. Annotation rendering end-to-end
// ============================================================================

#[tokio::test]
async fn test_annotation_rendering_end_to_end() {
    let mut b = InternalDocumentBuilder::new("test");

    // "Hello bold world" with "bold" annotated as Bold (bytes 6..10)
    let bold_text = "Hello bold world";
    b.push_paragraph(
        bold_text,
        vec![TextAnnotation {
            start: 6,
            end: 10,
            kind: AnnotationKind::Bold,
        }],
        None,
        None,
    );

    // "Some italic text" with "italic" annotated as Italic (bytes 5..11)
    let italic_text = "Some italic text";
    b.push_paragraph(
        italic_text,
        vec![TextAnnotation {
            start: 5,
            end: 11,
            kind: AnnotationKind::Italic,
        }],
        None,
        None,
    );

    // "Click here for info" with "here" as a Link (bytes 6..10)
    let link_text = "Click here for info";
    b.push_paragraph(
        link_text,
        vec![TextAnnotation {
            start: 6,
            end: 10,
            kind: AnnotationKind::Link {
                url: "https://example.com".to_string(),
                title: None,
            },
        }],
        None,
        None,
    );

    let doc = b.build();
    let result = derive(doc, OutputFormat::Markdown);
    let md = effective_content(&result);

    // Bold: **bold**
    assert!(
        md.contains("**bold**"),
        "Markdown should render bold annotation as **bold**, got:\n{md}"
    );
    // Italic: *italic*
    assert!(
        md.contains("*italic*"),
        "Markdown should render italic annotation as *italic*, got:\n{md}"
    );
    // Link: [here](https://example.com)
    assert!(
        md.contains("[here](https://example.com)"),
        "Markdown should render link annotation, got:\n{md}"
    );
}

// ============================================================================
// 8. Empty document handling
// ============================================================================

#[tokio::test]
async fn test_empty_document_handling() {
    let formats = [
        OutputFormat::Plain,
        OutputFormat::Markdown,
        OutputFormat::Djot,
        OutputFormat::Html,
    ];

    for format in &formats {
        let b = InternalDocumentBuilder::new("test");
        let doc = b.build();
        // Should not panic
        let result = derive(doc, format.clone());

        // Content can be empty or whitespace-only — the key is no panic
        let text = effective_content(&result);
        assert!(
            text.len() < 100,
            "Empty document in {format:?} should produce minimal output, got {} chars",
            text.len()
        );
    }
}

// ============================================================================
// 9. Large document performance smoke test
// ============================================================================

#[tokio::test]
async fn test_large_document_renders_without_timeout() {
    let mut b = InternalDocumentBuilder::new("test");
    b.push_heading(1, "Large Document", None, None);

    for i in 0..1000 {
        b.push_paragraph(
            &format!("Paragraph number {i} with some filler text to make it realistic."),
            vec![],
            None,
            None,
        );
    }

    let _doc = b.build();

    // Render to each format — this is a smoke test; we just verify it completes.
    let formats = [
        OutputFormat::Plain,
        OutputFormat::Markdown,
        OutputFormat::Djot,
        OutputFormat::Html,
    ];

    for format in &formats {
        // Clone the doc elements into a new document for each format
        let mut b2 = InternalDocumentBuilder::new("test");
        b2.push_heading(1, "Large Document", None, None);
        for i in 0..1000 {
            b2.push_paragraph(
                &format!("Paragraph number {i} with some filler text to make it realistic."),
                vec![],
                None,
                None,
            );
        }
        let doc2 = b2.build();

        let result = derive(doc2, format.clone());
        let text = effective_content(&result);

        assert!(
            text.contains("Paragraph number 0"),
            "{format:?}: should contain first paragraph"
        );
        assert!(
            text.contains("Paragraph number 999"),
            "{format:?}: should contain last paragraph"
        );
        assert!(
            text.len() > 50_000,
            "{format:?}: 1000-paragraph document should produce substantial output, got {} bytes",
            text.len()
        );
    }
}
