//! OPML parsing and content extraction.
//!
//! This module handles XML parsing, metadata extraction from the `<head>` section,
//! and recursive processing of `<outline>` elements in the `<body>` section.

use crate::Result;
use std::collections::HashMap;

#[cfg(feature = "office")]
use roxmltree::Node;

/// Extract OPML content and metadata from raw bytes.
///
/// Parses the XML document structure, extracts metadata from the `<head>` section,
/// and processes the outline hierarchy in the `<body>` section.
///
/// # Returns
///
/// A tuple containing:
/// - Extracted content as a String (outline hierarchy with indentation)
/// - Metadata HashMap with key-value pairs from the head section
#[cfg(feature = "office")]
pub(crate) fn extract_content_and_metadata(
    content: &[u8],
) -> Result<(String, HashMap<String, serde_json::Value>)> {
    let doc = roxmltree::Document::parse(
        std::str::from_utf8(content)
            .map_err(|e| crate::KreuzbergError::Other(format!("Invalid UTF-8 in OPML: {}", e)))?,
    )
    .map_err(|e| crate::KreuzbergError::Other(format!("Failed to parse OPML: {}", e)))?;

    let mut extracted_content = String::new();
    let mut metadata = HashMap::new();

    if let Some(opml) = doc.root().children().find(|n| n.tag_name().name() == "opml") {
        if let Some(head) = opml.children().find(|n| n.tag_name().name() == "head") {
            extract_metadata_from_head(head, &mut metadata);
        }

        if let Some(body) = opml.children().find(|n| n.tag_name().name() == "body") {
            if let Some(title) = metadata.get("title").and_then(|v| v.as_str()) {
                extracted_content.push_str(title);
                extracted_content.push('\n');
                extracted_content.push('\n');
            }

            for outline in body.children().filter(|n| n.tag_name().name() == "outline") {
                process_outline(outline, 0, &mut extracted_content);
            }
        }
    }

    Ok((extracted_content.trim().to_string(), metadata))
}

/// Extract metadata from the OPML `<head>` section.
///
/// Extracts standard OPML metadata fields:
/// - title: The document title
/// - dateCreated: Creation date
/// - dateModified: Last modification date
/// - ownerName: Document owner's name
/// - ownerEmail: Document owner's email
#[cfg(feature = "office")]
fn extract_metadata_from_head(head: Node, metadata: &mut HashMap<String, serde_json::Value>) {
    for child in head.children().filter(|n| n.is_element()) {
        let tag = child.tag_name().name();
        let text = child.text().unwrap_or("").trim();

        if text.is_empty() {
            continue;
        }

        match tag {
            "title" => {
                metadata.insert("title".to_string(), serde_json::json!(text));
            }
            "dateCreated" => {
                metadata.insert("dateCreated".to_string(), serde_json::json!(text));
            }
            "dateModified" => {
                metadata.insert("dateModified".to_string(), serde_json::json!(text));
            }
            "ownerName" => {
                metadata.insert("ownerName".to_string(), serde_json::json!(text));
            }
            "ownerEmail" => {
                metadata.insert("ownerEmail".to_string(), serde_json::json!(text));
            }
            _ => {}
        }
    }
}

/// Process outline elements recursively.
///
/// Extracts text content from outline hierarchy while preserving nesting depth
/// through indentation. URL attributes are excluded from the main content.
///
/// # Arguments
///
/// * `node` - The outline node to process
/// * `depth` - Current nesting depth (for indentation)
/// * `output` - Output string buffer to append content to
#[cfg(feature = "office")]
pub(crate) fn process_outline(node: Node, depth: usize, output: &mut String) {
    let text = node.attribute("text").unwrap_or("").trim();

    if !text.is_empty() {
        let indent = "  ".repeat(depth);
        output.push_str(&indent);
        output.push_str(text);
        output.push('\n');
    }

    for child in node.children().filter(|n| n.tag_name().name() == "outline") {
        process_outline(child, depth + 1, output);
    }
}

#[cfg(all(test, feature = "office"))]
mod tests {
    use super::*;

    #[test]
    fn test_simple_outline_parsing() {
        let opml = br#"<?xml version="1.0"?>
<opml version="2.0">
  <head>
    <title>Test</title>
  </head>
  <body>
    <outline text="Item 1" />
    <outline text="Item 2" />
  </body>
</opml>"#;

        let (content, metadata) = extract_content_and_metadata(opml).expect("Should parse simple OPML");

        assert!(content.contains("Item 1"), "Should extract first item");
        assert!(content.contains("Item 2"), "Should extract second item");
        assert_eq!(
            metadata.get("title").and_then(|v| v.as_str()),
            Some("Test"),
            "Should extract title"
        );
    }

    #[test]
    fn test_nested_hierarchy() {
        let opml = br#"<?xml version="1.0"?>
<opml version="2.0">
  <head>
    <title>Hierarchy Test</title>
  </head>
  <body>
    <outline text="Category">
      <outline text="Subcategory">
        <outline text="Item" />
      </outline>
    </outline>
  </body>
</opml>"#;

        let (content, _) = extract_content_and_metadata(opml).expect("Should parse nested OPML");

        assert!(content.contains("Category"), "Should contain top level");
        assert!(content.contains("Subcategory"), "Should contain nested level");
        assert!(content.contains("Item"), "Should contain deep item");
        assert!(content.contains("  "), "Should have indentation for nested items");
    }

    #[test]
    fn test_rss_feeds() {
        let opml = br#"<?xml version="1.0"?>
<opml version="2.0">
  <head>
    <title>Feeds</title>
  </head>
  <body>
    <outline text="Tech">
      <outline text="Hacker News" type="rss" xmlUrl="https://news.ycombinator.com/rss" htmlUrl="https://news.ycombinator.com/" />
      <outline text="TechCrunch" type="rss" xmlUrl="https://techcrunch.com/feed/" />
    </outline>
  </body>
</opml>"#;

        let (content, _) = extract_content_and_metadata(opml).expect("Should parse RSS OPML");

        assert!(content.contains("Hacker News"), "Should extract feed title");
        assert!(
            !content.contains("https://"),
            "Should NOT extract feed URLs (text-only extraction)"
        );
        assert!(content.contains("TechCrunch"), "Should extract multiple feeds");
    }

    #[test]
    fn test_metadata_extraction() {
        let opml = br#"<?xml version="1.0"?>
<opml version="2.0">
  <head>
    <title>My Feeds</title>
    <dateCreated>Mon, 06 Nov 2023 00:00:00 GMT</dateCreated>
    <dateModified>Fri, 01 Dec 2023 12:00:00 GMT</dateModified>
    <ownerName>John Doe</ownerName>
    <ownerEmail>john@example.com</ownerEmail>
  </head>
  <body>
    <outline text="Item" />
  </body>
</opml>"#;

        let (_content, metadata) = extract_content_and_metadata(opml).expect("Should extract metadata");

        assert_eq!(metadata.get("title").and_then(|v| v.as_str()), Some("My Feeds"));
        assert_eq!(metadata.get("ownerName").and_then(|v| v.as_str()), Some("John Doe"));
        assert_eq!(
            metadata.get("ownerEmail").and_then(|v| v.as_str()),
            Some("john@example.com")
        );
        assert!(metadata.contains_key("dateCreated"));
        assert!(metadata.contains_key("dateModified"));
    }

    #[test]
    fn test_with_special_characters() {
        let opml = br#"<?xml version="1.0"?>
<opml version="2.0">
  <head>
    <title>Test &amp; Special</title>
  </head>
  <body>
    <outline text="Business &amp; Startups" />
    <outline text="Science &lt;Advanced&gt;" />
  </body>
</opml>"#;

        let (content, metadata) =
            extract_content_and_metadata(opml).expect("Should handle special characters");

        assert!(
            content.contains("Business") && content.contains("Startups"),
            "Should decode HTML entities"
        );
        let title = metadata.get("title").and_then(|v| v.as_str()).unwrap_or("");
        assert!(!title.is_empty(), "Should extract title");
    }

    #[test]
    fn test_empty_body() {
        let opml = br#"<?xml version="1.0"?>
<opml version="2.0">
  <head>
    <title>Empty</title>
  </head>
  <body>
  </body>
</opml>"#;

        let (_content, metadata) = extract_content_and_metadata(opml).expect("Should handle empty body");

        assert_eq!(metadata.get("title").and_then(|v| v.as_str()), Some("Empty"));
    }

    #[test]
    fn test_malformed_missing_closing_tag() {
        let opml = br#"<?xml version="1.0"?>
<opml version="2.0">
  <head>
    <title>Broken</title>
  </head>
  <body>
    <outline text="Unclosed"
  </body>
</opml>"#;

        let result = extract_content_and_metadata(opml);
        assert!(result.is_err(), "Should fail to parse OPML with missing closing tags");
    }

    #[test]
    fn test_malformed_invalid_nesting() {
        let opml = br#"<?xml version="1.0"?>
<opml version="2.0">
  <head>
    <title>Invalid Nesting</title>
  <body>
    <outline text="Item" />
  </body>
</opml>"#;

        let result = extract_content_and_metadata(opml);
        assert!(result.is_err(), "Should fail to parse OPML with invalid nesting");
    }

    #[test]
    fn test_empty_outline_elements() {
        let opml = br#"<?xml version="1.0"?>
<opml version="2.0">
  <head>
    <title>Empty Outlines</title>
  </head>
  <body>
    <outline text="" />
    <outline />
    <outline text="Valid Item">
      <outline text="" />
      <outline text="Another Valid" />
    </outline>
  </body>
</opml>"#;

        let (content, metadata) =
            extract_content_and_metadata(opml).expect("Should handle empty outline elements");

        assert!(content.contains("Valid Item"), "Should extract valid items");
        assert!(content.contains("Another Valid"), "Should extract nested valid items");
        let empty_count = content.matches("\n\n").count();
        assert!(empty_count < 3, "Should skip empty outline elements");

        assert_eq!(metadata.get("title").and_then(|v| v.as_str()), Some("Empty Outlines"));
    }

    #[test]
    fn test_deeply_nested_empty_nodes() {
        let opml = br#"<?xml version="1.0"?>
<opml version="2.0">
  <head>
    <title>Deep Nesting</title>
  </head>
  <body>
    <outline text="Level 1">
      <outline text="">
        <outline text="">
          <outline text="Deep Item">
            <outline text="" />
          </outline>
        </outline>
      </outline>
      <outline text="Level 1 Sibling" />
    </outline>
  </body>
</opml>"#;

        let (content, _) =
            extract_content_and_metadata(opml).expect("Should handle deeply nested structures");

        assert!(content.contains("Level 1"), "Should extract top-level item");
        assert!(content.contains("Deep Item"), "Should extract deeply nested item");
        assert!(content.contains("Level 1 Sibling"), "Should extract sibling items");
    }

    #[test]
    fn test_outline_with_missing_text_attribute() {
        let opml = br#"<?xml version="1.0"?>
<opml version="2.0">
  <head>
    <title>Missing Attributes</title>
  </head>
  <body>
    <outline type="folder" />
    <outline text="Valid Item" type="rss" />
    <outline type="rss" xmlUrl="https://example.com/feed" />
  </body>
</opml>"#;

        let (content, metadata) = extract_content_and_metadata(opml)
            .expect("Should handle outline with missing text attribute");

        assert!(content.contains("Valid Item"), "Should extract item with text");
        assert!(!content.contains("https://"), "Should not extract URLs");

        assert_eq!(
            metadata.get("title").and_then(|v| v.as_str()),
            Some("Missing Attributes")
        );
    }

    #[test]
    fn test_whitespace_only_text_attribute() {
        let opml = br#"<?xml version="1.0"?>
<opml version="2.0">
  <head>
    <title>Whitespace Test</title>
  </head>
  <body>
    <outline text="   " />
    <outline text="
" />
    <outline text="Real Content" />
  </body>
</opml>"#;

        let (content, _) =
            extract_content_and_metadata(opml).expect("Should handle whitespace-only text");

        assert!(
            content.contains("Real Content"),
            "Should extract non-whitespace content"
        );
        let trimmed = content.trim();
        assert!(trimmed.contains("Whitespace Test"), "Should have title");
        assert!(trimmed.contains("Real Content"), "Should have real content");
    }

    #[test]
    fn test_html_entity_in_nested_structure() {
        let opml = br#"<?xml version="1.0"?>
<opml version="2.0">
  <head>
    <title>Entities &amp; Nesting</title>
  </head>
  <body>
    <outline text="News &amp; Updates">
      <outline text="Tech &lt; Science" />
      <outline text="Health &gt; Wealth" />
    </outline>
  </body>
</opml>"#;

        let (content, metadata) =
            extract_content_and_metadata(opml).expect("Should handle HTML entities");

        assert!(
            content.contains("News") && content.contains("Updates"),
            "Should decode &amp; entity"
        );
        assert!(content.contains("Tech"), "Should handle &lt; entity");
        assert!(content.contains("Science"), "Should decode entity properly");

        let title = metadata.get("title").and_then(|v| v.as_str()).unwrap_or("");
        assert!(
            title.contains("&") && title.contains("Nesting"),
            "Title should have decoded entity"
        );
    }

    #[test]
    fn test_single_outline_no_children() {
        let opml = br#"<?xml version="1.0"?>
<opml version="2.0">
  <head>
    <title>Single</title>
  </head>
  <body>
    <outline text="Only Item" />
  </body>
</opml>"#;

        let (content, metadata) =
            extract_content_and_metadata(opml).expect("Should handle single outline");

        assert!(content.contains("Only Item"), "Should extract single item");
        assert_eq!(metadata.get("title").and_then(|v| v.as_str()), Some("Single"));
    }

    #[test]
    fn test_head_without_body() {
        let opml = br#"<?xml version="1.0"?>
<opml version="2.0">
  <head>
    <title>No Body</title>
  </head>
</opml>"#;

        let (content, metadata) =
            extract_content_and_metadata(opml).expect("Should handle OPML without body");

        assert_eq!(metadata.get("title").and_then(|v| v.as_str()), Some("No Body"));
        assert!(content.is_empty() || content.trim() == "No Body");
    }

    #[test]
    fn test_body_without_head() {
        let opml = br#"<?xml version="1.0"?>
<opml version="2.0">
  <body>
    <outline text="Item" />
  </body>
</opml>"#;

        let (content, metadata) =
            extract_content_and_metadata(opml).expect("Should handle OPML without head");

        assert!(content.contains("Item"), "Should extract body content");
        assert!(metadata.is_empty(), "Should have no metadata without head");
    }
}
