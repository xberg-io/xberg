//! Validates that `render_json()` output conforms to the JSON Schema
//! defined in `docs/schemas/document-content.schema.json`.

use bytes::Bytes;
use kreuzberg::rendering::render_json;
use kreuzberg::types::extraction::ExtractedImage;
use kreuzberg::types::internal_builder::InternalDocumentBuilder;
use std::borrow::Cow;

/// Load the JSON Schema once and return the parsed value.
fn load_schema() -> serde_json::Value {
    let raw = include_str!("../../../docs/schemas/document-content.schema.json");
    serde_json::from_str(raw).expect("schema is valid JSON")
}

/// Validate a JSON string against the document-content schema.
/// Panics with a descriptive message on validation failure.
fn validate_json(json_str: &str) {
    let schema = load_schema();
    let instance: serde_json::Value = serde_json::from_str(json_str).expect("output is valid JSON");
    let validator = jsonschema::validator_for(&schema).expect("schema compiles");
    let mut errors = Vec::new();
    for error in validator.iter_errors(&instance) {
        errors.push(format!("  - {error}"));
    }
    if !errors.is_empty() {
        panic!(
            "JSON Schema validation failed for document:\n{}\nErrors:\n{}",
            serde_json::to_string_pretty(&instance).unwrap_or_default(),
            errors.join("\n")
        );
    }
}

fn make_image(description: Option<String>, source_path: Option<String>, data: Bytes) -> ExtractedImage {
    ExtractedImage {
        data,
        format: Cow::Borrowed("png"),
        image_index: 0,
        page_number: None,
        width: None,
        height: None,
        colorspace: None,
        bits_per_component: None,
        is_mask: false,
        description,
        ocr_result: None,
        bounding_box: None,
        source_path,
    }
}

#[test]
fn test_empty_document_validates() {
    let b = InternalDocumentBuilder::new("test");
    let doc = b.build();
    let json_str = render_json(&doc);
    validate_json(&json_str);
}

#[test]
fn test_headings_and_paragraphs_validate() {
    let mut b = InternalDocumentBuilder::new("test");
    b.push_title("My Document", None, None);
    b.push_heading(1, "Introduction", None, None);
    b.push_paragraph("This is the intro.", vec![], None, None);
    b.push_heading(2, "Details", None, None);
    b.push_paragraph("Some details here.", vec![], None, None);
    b.push_heading(1, "Conclusion", None, None);
    b.push_paragraph("Final thoughts.", vec![], None, None);
    let doc = b.build();
    let json_str = render_json(&doc);
    validate_json(&json_str);
}

#[test]
fn test_tables_code_blocks_and_lists_validate() {
    let mut b = InternalDocumentBuilder::new("test");

    // Table
    let cells = vec![
        vec!["Name".to_string(), "Age".to_string()],
        vec!["Alice".to_string(), "30".to_string()],
        vec!["Bob".to_string(), "25".to_string()],
    ];
    b.push_table_from_cells(&cells, None, None);

    // Code block with language
    b.push_code("fn main() {}", Some("rust"), None, None);

    // Code block without language
    b.push_code("echo hello", None, None, None);

    // Unordered list
    b.push_list(false);
    b.push_list_item("Apples", false, vec![], None, None);
    b.push_list_item("Bananas", false, vec![], None, None);
    b.end_list();

    // Ordered list
    b.push_list(true);
    b.push_list_item("First", true, vec![], None, None);
    b.push_list_item("Second", true, vec![], None, None);
    b.end_list();

    let doc = b.build();
    let json_str = render_json(&doc);
    validate_json(&json_str);
}

#[test]
fn test_images_formulas_blockquotes_validate() {
    let mut b = InternalDocumentBuilder::new("test");

    // Image with description and data
    let image = make_image(
        Some("A diagram".to_string()),
        None,
        Bytes::from_static(b"fake-png-data"),
    );
    b.push_image(Some("A diagram"), image, None, None);

    // Image with no description and a source path
    let image2 = make_image(None, Some("/images/photo.jpg".to_string()), Bytes::new());
    b.push_image(None, image2, None, None);

    // Formula
    b.push_formula("E = mc^2", None, None);
    b.push_formula("\\int_0^1 x^2 dx", None, None);

    // Blockquote
    b.push_quote_start();
    b.push_paragraph("To be or not to be.", vec![], None, None);
    b.push_quote_end();

    let doc = b.build();
    let json_str = render_json(&doc);
    validate_json(&json_str);
}

#[test]
fn test_complex_nested_document_validates() {
    let mut b = InternalDocumentBuilder::new("test");
    b.push_title("Complex Document", None, None);

    // Section with mixed content
    b.push_heading(1, "Overview", None, None);
    b.push_paragraph("This document tests all node types.", vec![], None, None);

    // Nested section
    b.push_heading(2, "Data", None, None);
    let cells = vec![
        vec!["Key".to_string(), "Value".to_string()],
        vec!["x".to_string(), "1".to_string()],
    ];
    b.push_table_from_cells(&cells, None, None);
    b.push_code("let x = 1;", Some("rust"), None, None);

    // Another top-level section
    b.push_heading(1, "Appendix", None, None);
    b.push_formula("a^2 + b^2 = c^2", None, None);

    b.push_list(false);
    b.push_list_item("Note 1", false, vec![], None, None);
    b.push_list_item("Note 2", false, vec![], None, None);
    b.end_list();

    b.push_quote_start();
    b.push_paragraph("Important quote.", vec![], None, None);
    b.push_quote_end();

    let doc = b.build();
    let json_str = render_json(&doc);
    validate_json(&json_str);
}
