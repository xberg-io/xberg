//! Markdown output lint quality tests.
//!
//! These tests extract representative documents to Markdown and validate the
//! output with `rumdl` (a Markdown linter). If `rumdl` is not installed the
//! tests skip gracefully.
//!
//! Usage:
//!   cargo test -p kreuzberg --test markdown_lint_quality -- --nocapture

mod helpers;

use kreuzberg::core::config::OutputFormat;
use kreuzberg::extraction::derive::derive_extraction_result;
use kreuzberg::types::internal_builder::InternalDocumentBuilder;

/// Check whether `rumdl` is available on PATH.
fn rumdl_available() -> bool {
    std::process::Command::new("rumdl").arg("--version").output().is_ok()
}

/// Run `rumdl` on the given Markdown content. Returns `Ok(())` when the lint
/// passes, `Err(message)` with combined stdout/stderr when it fails.
fn run_rumdl(md_content: &str) -> Result<(), String> {
    let tmp = tempfile::Builder::new()
        .suffix(".md")
        .tempfile()
        .expect("failed to create temp file");

    std::fs::write(tmp.path(), md_content).expect("failed to write temp file");

    let output = std::process::Command::new("rumdl")
        .args(["check", "--no-config"])
        .arg(tmp.path())
        .output()
        .map_err(|e| format!("failed to run rumdl: {e}"))?;

    if output.status.success() {
        Ok(())
    } else {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("rumdl failed:\n{stdout}\n{stderr}"))
    }
}

/// Render an `InternalDocument` to Markdown via the derive pipeline.
fn render_markdown(doc: kreuzberg::types::internal::InternalDocument) -> String {
    let result = derive_extraction_result(doc, false, OutputFormat::Markdown);
    result.formatted_content.unwrap_or(result.content)
}

// ---------------------------------------------------------------------------
// Document builders
// ---------------------------------------------------------------------------

/// A rich document with headings, paragraph, list, code block, and table.
fn build_rich_document() -> kreuzberg::types::internal::InternalDocument {
    let mut b = InternalDocumentBuilder::new("test-rich");

    b.push_heading(1, "Main Heading", None, None);
    b.push_paragraph("This is a paragraph with some descriptive text.", vec![], None, None);

    b.push_heading(2, "Details", None, None);
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

/// A document with multiple heading levels.
fn build_heading_hierarchy() -> kreuzberg::types::internal::InternalDocument {
    let mut b = InternalDocumentBuilder::new("test-headings");

    b.push_heading(1, "Title", None, None);
    b.push_paragraph("Introduction paragraph.", vec![], None, None);

    b.push_heading(2, "Section One", None, None);
    b.push_paragraph("Content of section one.", vec![], None, None);

    b.push_heading(3, "Subsection", None, None);
    b.push_paragraph("Subsection content.", vec![], None, None);

    b.push_heading(2, "Section Two", None, None);
    b.push_paragraph("Content of section two.", vec![], None, None);

    b.build()
}

/// A document with nested lists.
fn build_list_document() -> kreuzberg::types::internal::InternalDocument {
    let mut b = InternalDocumentBuilder::new("test-lists");

    b.push_heading(1, "Lists", None, None);

    b.push_list(false);
    b.push_list_item("Unordered item one", false, vec![], None, None);
    b.push_list_item("Unordered item two", false, vec![], None, None);
    b.end_list();

    b.push_list(true);
    b.push_list_item("Ordered item one", false, vec![], None, None);
    b.push_list_item("Ordered item two", false, vec![], None, None);
    b.end_list();

    b.build()
}

/// A minimal document with a heading and a single paragraph.
fn build_minimal_document() -> kreuzberg::types::internal::InternalDocument {
    let mut b = InternalDocumentBuilder::new("test-minimal");
    b.push_heading(1, "Note", None, None);
    b.push_paragraph("A single paragraph of text.", vec![], None, None);
    b.build()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn test_rich_document_markdown_passes_rumdl() {
    if !rumdl_available() {
        eprintln!("rumdl not found on PATH, skipping markdown lint test");
        return;
    }

    let md = render_markdown(build_rich_document());
    if let Err(msg) = run_rumdl(&md) {
        panic!("Rich document Markdown failed rumdl lint:\n{msg}\n\nGenerated markdown:\n{md}");
    }
}

#[test]
fn test_heading_hierarchy_markdown_passes_rumdl() {
    if !rumdl_available() {
        eprintln!("rumdl not found on PATH, skipping markdown lint test");
        return;
    }

    let md = render_markdown(build_heading_hierarchy());
    if let Err(msg) = run_rumdl(&md) {
        panic!("Heading hierarchy Markdown failed rumdl lint:\n{msg}\n\nGenerated markdown:\n{md}");
    }
}

#[test]
fn test_list_document_markdown_passes_rumdl() {
    if !rumdl_available() {
        eprintln!("rumdl not found on PATH, skipping markdown lint test");
        return;
    }

    let md = render_markdown(build_list_document());
    if let Err(msg) = run_rumdl(&md) {
        panic!("List document Markdown failed rumdl lint:\n{msg}\n\nGenerated markdown:\n{md}");
    }
}

#[test]
fn test_minimal_document_markdown_passes_rumdl() {
    if !rumdl_available() {
        eprintln!("rumdl not found on PATH, skipping markdown lint test");
        return;
    }

    let md = render_markdown(build_minimal_document());
    if let Err(msg) = run_rumdl(&md) {
        panic!("Minimal document Markdown failed rumdl lint:\n{msg}\n\nGenerated markdown:\n{md}");
    }
}

/// Test markdown output from actual file extraction when test documents and
/// the `office` feature are available.
#[cfg(feature = "office")]
#[test]
fn test_file_extraction_markdown_passes_rumdl() {
    use helpers::{get_test_file_path, test_documents_available};
    use kreuzberg::core::config::ExtractionConfig;
    use kreuzberg::extract_file_sync;

    if !rumdl_available() {
        eprintln!("rumdl not found on PATH, skipping markdown lint test");
        return;
    }
    if !test_documents_available() {
        eprintln!("test_documents not available, skipping file extraction lint test");
        return;
    }

    let test_files: &[&str] = &["latex/basic_sections.tex", "typst/simple.typ"];

    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        ..Default::default()
    };

    for &rel_path in test_files {
        let path = get_test_file_path(rel_path);
        if !path.exists() {
            eprintln!("Skipping {rel_path}: file not found");
            continue;
        }

        let result = extract_file_sync(&path, None, &config).expect("extraction should succeed");
        let md = result.formatted_content.as_deref().unwrap_or(&result.content);

        if let Err(msg) = run_rumdl(md) {
            panic!("File {rel_path} Markdown output failed rumdl lint:\n{msg}\n\nGenerated markdown:\n{md}");
        }
    }
}
