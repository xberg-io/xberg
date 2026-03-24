//! Tests verifying XML extraction produces embedding-friendly hierarchical output.
//!
//! Indented output preserves document structure so that related elements
//! (e.g. a plant's name and zone) stay grouped together for vector search.

use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::core::extractor::extract_bytes;

/// Sibling elements should be grouped under their parent with indentation.
#[tokio::test]
async fn test_xml_preserves_hierarchy() {
    let config = ExtractionConfig::default();
    let xml = br#"<?xml version="1.0"?><CATALOG><PLANT><COMMON>Bloodroot</COMMON><ZONE>4</ZONE></PLANT></CATALOG>"#;

    let result = extract_bytes(xml, "application/xml", &config).await.unwrap();

    // PLANT children should be indented under PLANT
    assert!(result.content.contains("PLANT"));
    assert!(result.content.contains("  COMMON\n    Bloodroot"));
    assert!(result.content.contains("  ZONE\n    4"));
}

/// Deeper nesting should produce deeper indentation.
#[tokio::test]
async fn test_xml_indentation_shows_nesting() {
    let config = ExtractionConfig::default();
    let xml = b"<root><parent><child><grandchild>Deep</grandchild></child></parent></root>";

    let result = extract_bytes(xml, "application/xml", &config).await.unwrap();

    assert!(result.content.contains("    grandchild\n      Deep"));
}

/// Attributes should appear inline with the element label.
#[tokio::test]
async fn test_xml_attributes_inline() {
    let config = ExtractionConfig::default();
    let xml = br#"<root><item type="book" id="42">Title</item></root>"#;

    let result = extract_bytes(xml, "application/xml", &config).await.unwrap();

    assert!(result.content.contains("item (type: book, id: 42)"));
    assert!(result.content.contains("Title"));
}

/// Top-level sibling groups should be separated by blank lines.
#[tokio::test]
async fn test_xml_sibling_separation() {
    let config = ExtractionConfig::default();
    let xml = b"<CATALOG><PLANT><COMMON>A</COMMON></PLANT><PLANT><COMMON>B</COMMON></PLANT></CATALOG>";

    let result = extract_bytes(xml, "application/xml", &config).await.unwrap();

    // Blank line between PLANT siblings
    assert!(result.content.contains("\n\nPLANT"));
}

/// Namespace attributes (xmlns:*) should be filtered from output.
#[tokio::test]
async fn test_xml_namespace_filtering() {
    let config = ExtractionConfig::default();
    let xml = br#"<root xmlns:ns="http://example.com" id="1"><item>Text</item></root>"#;

    let result = extract_bytes(xml, "application/xml", &config).await.unwrap();

    assert!(!result.content.contains("xmlns"), "Namespace attrs should be filtered");
    assert!(
        result.content.contains("root (id: 1)"),
        "Non-namespace attrs should be preserved"
    );
    assert!(result.content.contains("Text"));
}

/// Mixed content (text between elements) should be preserved with indentation.
#[tokio::test]
async fn test_xml_mixed_content() {
    let config = ExtractionConfig::default();
    let xml = b"<root>Text before<item>nested</item>Text after</root>";

    let result = extract_bytes(xml, "application/xml", &config).await.unwrap();

    assert!(result.content.contains("Text before"));
    assert!(result.content.contains("nested"));
    assert!(result.content.contains("Text after"));
}

/// Self-closing tags should appear in the output.
#[tokio::test]
async fn test_xml_self_closing_tags() {
    let config = ExtractionConfig::default();
    let xml = br#"<root><item type="empty"/></root>"#;

    let result = extract_bytes(xml, "application/xml", &config).await.unwrap();

    assert!(result.content.contains("item (type: empty)"));
}

/// Empty attribute values should be filtered from the label.
#[tokio::test]
async fn test_xml_empty_attribute_filtered() {
    let config = ExtractionConfig::default();
    let xml = br#"<root><item id="" type="book">Text</item></root>"#;

    let result = extract_bytes(xml, "application/xml", &config).await.unwrap();

    assert!(result.content.contains("item (type: book)"));
    assert!(!result.content.contains("id:"), "Empty attribute should be filtered");
}

/// Text directly inside the root element should still be indented.
#[tokio::test]
async fn test_xml_root_level_text() {
    let config = ExtractionConfig::default();
    let xml = b"<root>Some text</root>";

    let result = extract_bytes(xml, "application/xml", &config).await.unwrap();

    assert!(result.content.contains("root"));
    assert!(result.content.contains("Some text"));
}

/// Real XML file should produce grouped plant entries.
#[tokio::test]
async fn test_xml_real_file_plant_catalog() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/xml/plant_catalog.xml");
    if !path.exists() {
        return;
    }
    let content = std::fs::read(&path).unwrap();
    let config = ExtractionConfig::default();

    let result = extract_bytes(&content, "application/xml", &config).await.unwrap();

    // Each plant's fields should be grouped together
    assert!(result.content.contains("PLANT\n  COMMON\n    Bloodroot"));
    assert!(result.content.contains("PLANT\n  COMMON\n    Columbine"));
}
