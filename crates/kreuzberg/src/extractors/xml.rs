//! XML extractor.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extraction::xml::{parse_xml, parse_xml_svg};
use crate::extractors::SyncExtractor;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::ExtractionResult;
use ahash::AHashMap;
use async_trait::async_trait;

/// Build a `DocumentStructure` from XML content by parsing element hierarchy.
///
/// Maps XML elements to groups (for parent elements with children) and
/// paragraphs (for text content), preserving the element tree structure.
/// Element attributes are stored as node attributes.
fn build_xml_document_structure(
    content: &[u8],
    mime_type: &str,
) -> crate::types::document_structure::DocumentStructure {
    use crate::types::builder::DocumentStructureBuilder;
    use quick_xml::Reader;
    use quick_xml::events::Event;
    use std::borrow::Cow;

    let mut builder = DocumentStructureBuilder::new().source_format("xml");
    let is_svg = mime_type == "image/svg+xml";

    let mut reader = Reader::from_reader(content);
    reader.config_mut().trim_text(true);
    reader.config_mut().check_end_names = false;

    let mut buf = Vec::new();
    let mut depth = 0u8;
    let mut element_stack: Vec<String> = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name_bytes = e.name().as_ref().to_vec();
                let name_owned = String::from_utf8_lossy(&name_bytes).into_owned();

                // Extract element attributes
                let mut attrs = AHashMap::new();
                for attr in e.attributes().flatten() {
                    let key: Cow<str> = String::from_utf8_lossy(attr.key.as_ref());
                    let val: Cow<str> = String::from_utf8_lossy(&attr.value);
                    let trimmed_val = val.trim();
                    if !trimmed_val.is_empty() {
                        attrs.insert(key.to_string(), trimmed_val.to_string());
                    }
                }

                // Use heading for top-level structural elements
                let level = (depth + 1).min(6);
                let node_idx = builder.push_heading(level, &name_owned, None, None);
                if !attrs.is_empty() {
                    builder.set_attributes(node_idx, attrs);
                }

                element_stack.push(name_owned);
                depth = depth.saturating_add(1);
            }
            Ok(Event::End(_)) => {
                element_stack.pop();
                depth = depth.saturating_sub(1);
            }
            Ok(Event::Text(e)) => {
                if is_svg {
                    // In SVG mode, only extract text from text-bearing elements
                    let in_text_elem = element_stack
                        .iter()
                        .any(|n| matches!(n.as_str(), "text" | "tspan" | "title" | "desc" | "textPath"));
                    if !in_text_elem {
                        buf.clear();
                        continue;
                    }
                }
                let text: Cow<str> = String::from_utf8_lossy(e.as_ref());
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    builder.push_paragraph(trimmed, vec![], None, None);
                }
            }
            Ok(Event::Empty(e)) => {
                let name_bytes = e.name().as_ref().to_vec();
                let name = String::from_utf8_lossy(&name_bytes).into_owned();

                let mut attrs = AHashMap::new();
                for attr in e.attributes().flatten() {
                    let key: Cow<str> = String::from_utf8_lossy(attr.key.as_ref());
                    let val: Cow<str> = String::from_utf8_lossy(&attr.value);
                    let trimmed_val = val.trim();
                    if !trimmed_val.is_empty() {
                        attrs.insert(key.to_string(), trimmed_val.to_string());
                    }
                }

                let node_idx = builder.push_paragraph(&name, vec![], None, None);
                if !attrs.is_empty() {
                    builder.set_attributes(node_idx, attrs);
                }
            }
            Ok(Event::CData(e)) => {
                let text: Cow<str> = String::from_utf8_lossy(&e);
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    builder.push_paragraph(trimmed, vec![], None, None);
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    builder.build()
}

/// XML extractor.
///
/// Extracts text content from XML files, preserving element structure information.
pub struct XmlExtractor;

impl XmlExtractor {
    /// Create a new XML extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Default for XmlExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for XmlExtractor {
    fn name(&self) -> &str {
        "xml-extractor"
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

    fn description(&self) -> &str {
        "Extracts text content from XML files with element metadata"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

impl SyncExtractor for XmlExtractor {
    fn extract_sync(&self, content: &[u8], mime_type: &str, config: &ExtractionConfig) -> Result<ExtractionResult> {
        let xml_result = if mime_type == "image/svg+xml" {
            parse_xml_svg(content, false)?
        } else {
            parse_xml(content, false)?
        };

        let document = if config.include_document_structure && !xml_result.content.trim().is_empty() {
            Some(build_xml_document_structure(content, mime_type))
        } else {
            None
        };

        Ok(ExtractionResult {
            content: xml_result.content,
            mime_type: mime_type.to_string().into(),
            metadata: crate::types::Metadata {
                format: Some(crate::types::FormatMetadata::Xml(crate::types::XmlMetadata {
                    element_count: xml_result.element_count,
                    unique_elements: xml_result.unique_elements,
                })),
                ..Default::default()
            },
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
            pages: None,
            djot_content: None,
            elements: None,
            ocr_elements: None,
            document,
            #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
            extracted_keywords: None,
            quality_score: None,
            processing_warnings: Vec::new(),
            annotations: None,
            children: None,
        })
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl DocumentExtractor for XmlExtractor {
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

    fn supported_mime_types(&self) -> &[&str] {
        &[
            "application/xml",
            "text/xml",
            "image/svg+xml",
            "application/x-endnote+xml",
        ]
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

    #[tokio::test]
    async fn test_xml_extractor() {
        let extractor = XmlExtractor::new();
        let content = b"<root><item>Hello</item><item>World</item></root>";
        let config = ExtractionConfig::default();

        let result = extractor
            .extract_bytes(content, "application/xml", &config)
            .await
            .unwrap();

        assert_eq!(result.mime_type, "application/xml");
        // Hierarchical output: element names on their own line, text indented below
        assert!(result.content.contains("Hello"));
        assert!(result.content.contains("World"));
        assert!(result.metadata.format.is_some());
        let xml_meta = match result.metadata.format.as_ref().unwrap() {
            crate::types::FormatMetadata::Xml(meta) => meta,
            _ => panic!("Expected Xml metadata"),
        };
        assert_eq!(xml_meta.element_count, 3);
        assert!(xml_meta.unique_elements.contains(&"root".to_string()));
        assert!(xml_meta.unique_elements.contains(&"item".to_string()));
    }

    #[test]
    fn test_xml_plugin_interface() {
        let extractor = XmlExtractor::new();
        assert_eq!(extractor.name(), "xml-extractor");
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert_eq!(
            extractor.supported_mime_types(),
            &[
                "application/xml",
                "text/xml",
                "image/svg+xml",
                "application/x-endnote+xml"
            ]
        );
        assert_eq!(extractor.priority(), 50);
    }
}
