//! XML extraction functions.
//!
//! Provides memory-efficient streaming XML parsing using `quick-xml`. Can handle
//! multi-GB XML files without loading them entirely into memory.
//!
//! # Features
//!
//! - **Streaming parser**: Processes XML files in constant memory
//! - **Element tracking**: Counts total elements and unique element names
//! - **Hierarchical text extraction**: Preserves document structure through indentation
//! - **Whitespace handling**: Optional whitespace preservation
//!
//! # Example
//!
//! ```rust
//! use kreuzberg::extraction::xml::parse_xml;
//!
//! # fn example() -> kreuzberg::Result<()> {
//! let xml = b"<root><item>Hello</item><item>World</item></root>";
//! let result = parse_xml(xml, false)?; // false = trim whitespace
//!
//! // Content preserves element hierarchy through indentation
//! assert!(result.content.contains("item\n  Hello"));
//! assert!(result.content.contains("item\n  World"));
//! assert_eq!(result.element_count, 3);
//! # Ok(())
//! # }
//! ```
use crate::error::{KreuzbergError, Result};
use crate::types::XmlExtractionResult;
use quick_xml::Reader;
use quick_xml::events::Event;
use std::borrow::Cow;
use std::collections::HashSet;

/// SVG text-bearing elements whose text content should be extracted.
const SVG_TEXT_ELEMENTS: &[&str] = &["text", "tspan", "title", "desc", "textPath", "altGlyph"];

/// Parse XML with optional SVG mode.
///
/// In SVG mode, only text from SVG text-bearing elements (`<text>`, `<tspan>`,
/// `<title>`, `<desc>`, `<textPath>`) is extracted, without element name prefixes.
/// Attribute values are also omitted in SVG mode.
pub fn parse_xml_svg(xml_bytes: &[u8], preserve_whitespace: bool) -> Result<XmlExtractionResult> {
    parse_xml_inner(xml_bytes, preserve_whitespace, true)
}

pub fn parse_xml(xml_bytes: &[u8], preserve_whitespace: bool) -> Result<XmlExtractionResult> {
    parse_xml_inner(xml_bytes, preserve_whitespace, false)
}

fn parse_xml_inner(xml_bytes: &[u8], preserve_whitespace: bool, svg_mode: bool) -> Result<XmlExtractionResult> {
    // Handle UTF-16 encoded XML by detecting BOM and transcoding to UTF-8
    let decoded_bytes;
    let effective_bytes = if xml_bytes.len() >= 2 {
        if xml_bytes[0] == 0xFF && xml_bytes[1] == 0xFE {
            // UTF-16 LE BOM
            decoded_bytes = decode_utf16_to_utf8(xml_bytes, false)?;
            &decoded_bytes
        } else if xml_bytes[0] == 0xFE && xml_bytes[1] == 0xFF {
            // UTF-16 BE BOM
            decoded_bytes = decode_utf16_to_utf8(xml_bytes, true)?;
            &decoded_bytes
        } else {
            xml_bytes
        }
    } else {
        xml_bytes
    };

    let mut reader = Reader::from_reader(effective_bytes);
    reader.config_mut().trim_text(!preserve_whitespace);
    reader.config_mut().check_end_names = false;

    let mut content = String::new();
    let mut element_count = 0usize;
    let mut unique_elements_set = HashSet::new();
    let mut buf = Vec::new();
    let mut element_stack: Vec<String> = Vec::new();
    let mut had_depth1_element = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name_bytes = (e.name().as_ref() as &[u8]).to_vec();
                let name: Cow<str> = String::from_utf8_lossy(&name_bytes);
                let name_owned = name.into_owned();
                element_count += 1;
                unique_elements_set.insert(name_owned.clone());

                // In SVG mode, skip attribute extraction entirely
                if !svg_mode {
                    let depth = element_stack.len();
                    let label = format_element_label(&name_owned, e.attributes());
                    write_element_line(&mut content, &label, depth, &mut had_depth1_element);
                }

                element_stack.push(name_owned);
            }
            Ok(Event::Empty(e)) => {
                let name_bytes = (e.name().as_ref() as &[u8]).to_vec();
                let name: Cow<str> = String::from_utf8_lossy(&name_bytes);
                let name_owned = name.into_owned();
                element_count += 1;
                unique_elements_set.insert(name_owned.clone());

                // In SVG mode, skip self-closing tag output entirely
                if !svg_mode {
                    let depth = element_stack.len();
                    let label = format_element_label(&name_owned, e.attributes());
                    write_element_line(&mut content, &label, depth, &mut had_depth1_element);
                }
            }
            Ok(Event::End(_e)) => {
                // Pop matching element from stack
                element_stack.pop();
            }
            Ok(Event::Text(e)) => {
                let text_cow: Cow<str> = String::from_utf8_lossy(e.as_ref());
                let trimmed = if preserve_whitespace {
                    text_cow.to_string()
                } else {
                    text_cow.trim().to_string()
                };

                if !trimmed.is_empty() {
                    // In SVG mode, only extract text from SVG text-bearing elements
                    if svg_mode {
                        let in_text_element = element_stack
                            .iter()
                            .any(|name| SVG_TEXT_ELEMENTS.contains(&name.as_str()));
                        if in_text_element {
                            if !content.is_empty() && !content.ends_with('\n') && !content.ends_with(' ') {
                                content.push(' ');
                            }
                            content.push_str(&trimmed);
                        }
                    } else {
                        write_text_line(&mut content, &trimmed, element_stack.len());
                    }
                }
            }
            Ok(Event::CData(e)) => {
                // In SVG mode, only extract CData from SVG text-bearing elements
                if svg_mode {
                    let in_text_element = element_stack
                        .iter()
                        .any(|name| SVG_TEXT_ELEMENTS.contains(&name.as_str()));
                    if !in_text_element {
                        continue;
                    }
                }

                let text_cow: Cow<str> = String::from_utf8_lossy(&e);
                write_text_line(&mut content, &text_cow, element_stack.len());
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(KreuzbergError::parsing(format!(
                    "XML parsing error at position {}: {}",
                    reader.buffer_position(),
                    e
                )));
            }
            _ => {}
        }
        buf.clear();
    }

    let content = content.trim().to_string();
    let mut unique_elements: Vec<String> = unique_elements_set.into_iter().collect();
    unique_elements.sort();

    Ok(XmlExtractionResult {
        content,
        element_count,
        unique_elements,
    })
}

/// Decode UTF-16 bytes (with BOM) to UTF-8 bytes.
fn decode_utf16_to_utf8(data: &[u8], big_endian: bool) -> Result<Vec<u8>> {
    // Skip BOM (first 2 bytes) and truncate to even length
    let data = &data[2..];
    let even_len = data.len() & !1;
    let data = &data[..even_len];

    let u16_iter = data.chunks_exact(2).map(|chunk| {
        if big_endian {
            u16::from_be_bytes([chunk[0], chunk[1]])
        } else {
            u16::from_le_bytes([chunk[0], chunk[1]])
        }
    });

    let text: String = char::decode_utf16(u16_iter)
        .map(|r| r.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect();

    Ok(text.into_bytes())
}

/// Build an element label with non-namespace attributes inline.
fn format_element_label(name: &str, attrs: quick_xml::events::attributes::Attributes) -> String {
    let attr_parts: Vec<String> = attrs
        .flatten()
        .filter_map(|attr| {
            let key: Cow<str> = String::from_utf8_lossy(attr.key.as_ref());
            if key.starts_with("xmlns") {
                return None;
            }
            let val: Cow<str> = String::from_utf8_lossy(&attr.value);
            let trimmed = val.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(format!("{}: {}", key, trimmed))
            }
        })
        .collect();
    if attr_parts.is_empty() {
        name.to_string()
    } else {
        format!("{} ({})", name, attr_parts.join(", "))
    }
}

/// Write an indented element label, with blank lines between depth-1 siblings.
fn write_element_line(content: &mut String, label: &str, depth: usize, had_depth1: &mut bool) {
    match depth {
        0 => {
            content.push_str(label);
            content.push('\n');
        }
        1 => {
            if *had_depth1 {
                content.push('\n');
            }
            content.push_str(label);
            content.push('\n');
            *had_depth1 = true;
        }
        _ => {
            for _ in 0..depth - 1 {
                content.push_str("  ");
            }
            content.push_str(label);
            content.push('\n');
        }
    }
}

/// Write indented text content under the current element.
fn write_text_line(content: &mut String, text: &str, stack_len: usize) {
    let indent = if stack_len == 0 {
        0
    } else {
        stack_len.saturating_sub(1).max(1)
    };
    for _ in 0..indent {
        content.push_str("  ");
    }
    content.push_str(text);
    content.push('\n');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_xml() {
        let xml = b"<root><item>Hello</item><item>World</item></root>";
        let result = parse_xml(xml, false).unwrap();
        // Element names with text indented beneath
        assert!(result.content.contains("item\n  Hello"));
        assert!(result.content.contains("item\n  World"));
        assert_eq!(result.element_count, 3);
        assert!(result.unique_elements.contains(&"root".to_string()));
        assert!(result.unique_elements.contains(&"item".to_string()));
        assert_eq!(result.unique_elements.len(), 2);
    }

    #[test]
    fn test_xml_with_cdata() {
        let xml = b"<root><![CDATA[Special <characters> & data]]></root>";
        let result = parse_xml(xml, false).unwrap();
        assert!(result.content.contains("Special <characters> & data"));
        assert_eq!(result.element_count, 1);
    }

    #[test]
    fn test_malformed_xml_lenient() {
        let xml = b"<root><item>Unclosed<item2>Content</root>";
        let result = parse_xml(xml, false).unwrap();
        assert!(!result.content.is_empty());
        assert!(result.content.contains("Content"));
    }

    #[test]
    fn test_empty_xml() {
        let xml = b"<root></root>";
        let result = parse_xml(xml, false).unwrap();
        assert_eq!(result.content, "root");
        assert_eq!(result.element_count, 1);
        assert_eq!(result.unique_elements.len(), 1);
    }

    #[test]
    fn test_whitespace_handling() {
        let xml = b"<root>  <item>  Text  </item>  </root>";
        let result = parse_xml(xml, false).unwrap();
        assert!(result.content.contains("item\n  Text"));
    }

    #[test]
    fn test_preserve_whitespace() {
        let xml = b"<root>  Text with   spaces  </root>";
        let result_trimmed = parse_xml(xml, false).unwrap();
        let result_preserved = parse_xml(xml, true).unwrap();
        assert!(result_trimmed.content.contains("Text with   spaces"));
        assert!(result_preserved.content.len() >= result_trimmed.content.len());
    }

    #[test]
    fn test_element_counting() {
        let xml = b"<root><a/><b/><c/><b/><d/></root>";
        let result = parse_xml(xml, false).unwrap();
        assert_eq!(result.element_count, 6);
        assert_eq!(result.unique_elements.len(), 5);
        assert!(result.unique_elements.contains(&"b".to_string()));
    }

    #[test]
    fn test_xml_with_attributes() {
        let xml = br#"<root id="1"><item type="test">Content</item></root>"#;
        let result = parse_xml(xml, false).unwrap();
        assert!(result.content.contains("item (type: test)\n  Content"));
        assert_eq!(result.element_count, 2);
    }

    #[test]
    fn test_xml_with_namespaces() {
        let xml = b"<ns:root xmlns:ns=\"http://example.com\"><ns:item>Text</ns:item></ns:root>";
        let result = parse_xml(xml, false).unwrap();
        assert!(result.content.contains("Text"));
        assert!(result.element_count >= 2);
    }

    #[test]
    fn test_xml_with_comments() {
        let xml = b"<root><!-- Comment --><item>Text</item></root>";
        let result = parse_xml(xml, false).unwrap();
        assert!(result.content.contains("item\n  Text"));
        assert_eq!(result.element_count, 2);
    }

    #[test]
    fn test_xml_with_processing_instructions() {
        let xml = b"<?xml version=\"1.0\"?><root><item>Text</item></root>";
        let result = parse_xml(xml, false).unwrap();
        assert!(result.content.contains("item\n  Text"));
        assert_eq!(result.element_count, 2);
    }

    #[test]
    fn test_xml_with_mixed_content() {
        let xml = b"<root>Text before<item>nested</item>Text after</root>";
        let result = parse_xml(xml, false).unwrap();
        assert!(result.content.contains("Text before"));
        assert!(result.content.contains("item\n  nested"));
        assert!(result.content.contains("Text after"));
    }

    #[test]
    fn test_xml_empty_bytes() {
        let xml = b"";
        let result = parse_xml(xml, false).unwrap();
        assert_eq!(result.content, "");
        assert_eq!(result.element_count, 0);
        assert!(result.unique_elements.is_empty());
    }

    #[test]
    fn test_xml_only_whitespace() {
        let xml = b"   \n\t  ";
        let result = parse_xml(xml, false).unwrap();
        assert_eq!(result.content, "");
        assert_eq!(result.element_count, 0);
    }

    #[test]
    fn test_xml_with_nested_elements() {
        let xml = b"<root><parent><child><grandchild>Deep</grandchild></child></parent></root>";
        let result = parse_xml(xml, false).unwrap();
        assert!(result.content.contains("    grandchild\n      Deep"));
        assert_eq!(result.element_count, 4);
        assert_eq!(result.unique_elements.len(), 4);
    }

    #[test]
    fn test_xml_with_special_characters() {
        let xml = b"<root>&lt;&gt;&amp;&quot;&apos;</root>";
        let result = parse_xml(xml, false).unwrap();
        assert!(result.element_count >= 1);
    }

    #[test]
    fn test_xml_self_closing_tags() {
        let xml = b"<root><item1/><item2/><item3/></root>";
        let result = parse_xml(xml, false).unwrap();
        assert_eq!(result.element_count, 4);
        assert_eq!(result.unique_elements.len(), 4);
    }

    #[test]
    fn test_xml_multiple_text_nodes() {
        let xml = b"<root>First<a/>Second<b/>Third</root>";
        let result = parse_xml(xml, false).unwrap();
        assert!(result.content.contains("First"));
        assert!(result.content.contains("Second"));
        assert!(result.content.contains("Third"));
    }

    #[test]
    fn test_xml_with_newlines() {
        let xml = b"<root>\n  <item>\n    Text\n  </item>\n</root>";
        let result = parse_xml(xml, false).unwrap();
        assert!(result.content.contains("item\n  Text"));
    }

    #[test]
    fn test_xml_large_cdata() {
        let large_text = "A".repeat(10000);
        let xml = format!("<root><![CDATA[{}]]></root>", large_text);
        let result = parse_xml(xml.as_bytes(), false).unwrap();
        assert!(result.content.contains(&large_text));
    }

    #[test]
    fn test_xml_unique_elements_sorted() {
        let xml = b"<root><z/><a/><m/><b/></root>";
        let result = parse_xml(xml, false).unwrap();
        let expected = vec!["a", "b", "m", "root", "z"];
        assert_eq!(result.unique_elements, expected);
    }

    #[test]
    fn test_xml_result_structure() {
        let xml = b"<root><item>Test</item></root>";
        let result = parse_xml(xml, false).unwrap();

        assert!(!result.content.is_empty());
        assert!(result.element_count > 0);
        assert!(!result.unique_elements.is_empty());
    }

    #[test]
    fn test_xml_with_multiple_cdata_sections() {
        let xml = b"<root><![CDATA[First]]>Text<![CDATA[Second]]></root>";
        let result = parse_xml(xml, false).unwrap();
        assert!(result.content.contains("First"));
        assert!(result.content.contains("Text"));
        assert!(result.content.contains("Second"));
    }

    #[test]
    fn test_xml_preserve_whitespace_flag() {
        let xml = b"<root>  A  B  </root>";
        let without_preserve = parse_xml(xml, false).unwrap();
        let with_preserve = parse_xml(xml, true).unwrap();

        assert!(!without_preserve.content.starts_with(' '));

        assert!(with_preserve.content.len() >= without_preserve.content.len());
    }

    #[test]
    fn test_xml_element_count_accuracy() {
        let xml = b"<root><a><b><c/></b></a><d/></root>";
        let result = parse_xml(xml, false).unwrap();
        assert_eq!(result.element_count, 5);
    }

    #[test]
    fn test_xml_with_invalid_utf8() {
        let xml = b"<root><item>Valid text \xFF invalid</item></root>";
        let result = parse_xml(xml, false).unwrap();
        assert!(result.content.contains("Valid text"));
        assert_eq!(result.element_count, 2);
    }

    #[test]
    fn test_xml_cdata_with_invalid_utf8() {
        let xml = b"<root><![CDATA[Text \xFF more text]]></root>";
        let result = parse_xml(xml, false).unwrap();
        assert!(result.content.contains("Text"));
        assert!(result.content.contains("more text"));
        assert_eq!(result.element_count, 1);
    }

    #[test]
    fn test_xml_element_name_with_invalid_utf8() {
        let xml = b"<root><item\xFF>Content</item\xFF></root>";
        let result = parse_xml(xml, false);
        let _ = result;
    }

    #[test]
    fn test_utf16_le_xml() {
        // UTF-16 LE BOM + "<r>A</r>" encoded as UTF-16 LE
        let mut xml = vec![0xFF, 0xFE]; // BOM
        for c in "<r>A</r>".encode_utf16() {
            xml.extend_from_slice(&c.to_le_bytes());
        }
        let result = parse_xml(&xml, false).unwrap();
        assert!(result.content.contains("A"));
    }

    #[test]
    fn test_utf16_be_xml() {
        // UTF-16 BE BOM + "<r>B</r>" encoded as UTF-16 BE
        let mut xml = vec![0xFE, 0xFF]; // BOM
        for c in "<r>B</r>".encode_utf16() {
            xml.extend_from_slice(&c.to_be_bytes());
        }
        let result = parse_xml(&xml, false).unwrap();
        assert!(result.content.contains("B"));
    }

    #[test]
    fn test_utf16_odd_byte_count_truncates_gracefully() {
        // UTF-16 LE BOM + "<r>X</r>" + trailing odd byte
        let mut xml = vec![0xFF, 0xFE]; // BOM
        for c in "<r>X</r>".encode_utf16() {
            xml.extend_from_slice(&c.to_le_bytes());
        }
        xml.push(0x0A); // trailing odd byte
        let result = parse_xml(&xml, false).unwrap();
        assert!(result.content.contains("X"));
    }

    #[test]
    fn test_svg_script_cdata_excluded() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg">
            <script><![CDATA[ var x = 1; alert("hello"); ]]></script>
            <text>Hello</text>
            <title>My Title</title>
        </svg>"#;
        let result = parse_xml_svg(svg, false).unwrap();
        assert!(
            !result.content.contains("var x"),
            "script CDATA should not appear in SVG output"
        );
        assert!(
            !result.content.contains("alert"),
            "script CDATA should not appear in SVG output"
        );
        assert!(
            result.content.contains("Hello"),
            "text element content should be included"
        );
        assert!(
            result.content.contains("My Title"),
            "title element content should be included"
        );
    }

    #[test]
    fn test_svg_style_cdata_excluded() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg">
            <style type="text/css"><![CDATA[ .cls { fill: red; } ]]></style>
            <text>Visible</text>
        </svg>"#;
        let result = parse_xml_svg(svg, false).unwrap();
        assert!(
            !result.content.contains("fill"),
            "style CDATA should not appear in SVG output"
        );
        assert!(
            !result.content.contains(".cls"),
            "style CDATA should not appear in SVG output"
        );
        assert!(
            result.content.contains("Visible"),
            "text element content should be included"
        );
    }

    #[test]
    fn test_svg_text_elements_included() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg">
            <title>Chart Title</title>
            <desc>A description</desc>
            <text x="10" y="20">Label</text>
            <text><tspan>Span text</tspan></text>
            <rect width="100" height="50"/>
        </svg>"#;
        let result = parse_xml_svg(svg, false).unwrap();
        assert!(result.content.contains("Chart Title"), "title text should be included");
        assert!(result.content.contains("A description"), "desc text should be included");
        assert!(
            result.content.contains("Label"),
            "text element content should be included"
        );
        assert!(result.content.contains("Span text"), "tspan content should be included");
    }

    #[test]
    fn test_svg_cdata_in_text_element_included() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg">
            <text><![CDATA[CDATA in text]]></text>
        </svg>"#;
        let result = parse_xml_svg(svg, false).unwrap();
        assert!(
            result.content.contains("CDATA in text"),
            "CDATA inside text element should be included"
        );
    }

    #[test]
    fn test_utf16_factbook_file() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test_documents/vendored/unstructured/xml/factbook-utf-16.xml");
        if path.exists() {
            let xml = std::fs::read(&path).unwrap();
            let result = parse_xml(&xml, false).unwrap();
            assert!(
                !result.content.is_empty(),
                "factbook-utf-16.xml should produce non-empty content"
            );
            assert!(result.content.contains("United States"));
            assert!(result.content.contains("Canada"));
        }
    }
}
