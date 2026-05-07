//! XML extractor.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extraction::xml::{parse_xml, parse_xml_svg};
use crate::extractors::SyncExtractor;
use crate::extractors::security::SecurityBudget;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
use crate::types::metadata::Metadata;
use ahash::AHashMap;
use async_trait::async_trait;

/// Build an `InternalDocument` from XML content by parsing element hierarchy.
///
/// Maps XML elements to headings (for parent elements with children) and
/// paragraphs (for text content), preserving the element tree structure.
/// Element attributes are stored as element attributes.
///
/// `budget` enforces hostile-input limits (XML depth, iteration count, entity
/// length, cumulative content size). Any limit violation is converted into a
/// `KreuzbergError::Security` via the `?` operator at call-site.
fn build_internal_document(content: &[u8], mime_type: &str, budget: &mut SecurityBudget) -> Result<InternalDocument> {
    use quick_xml::Reader;
    use quick_xml::events::Event;
    use std::borrow::Cow;

    let mut doc = InternalDocument::new("xml");
    let is_svg = mime_type == "image/svg+xml";

    let mut reader = Reader::from_reader(content);
    reader.config_mut().trim_text(true);
    reader.config_mut().check_end_names = false;

    let mut buf = Vec::new();
    let mut depth: u16 = 0;
    let mut element_stack: Vec<String> = Vec::new();
    let mut index: u32 = 0;

    loop {
        budget.step()?;
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                budget.enter()?;
                let name_bytes = e.name().as_ref().to_vec();
                let name_owned = String::from_utf8_lossy(&name_bytes).into_owned();

                // Extract element attributes
                let mut attrs = AHashMap::new();
                for attr in e.attributes().flatten() {
                    let key: Cow<str> = String::from_utf8_lossy(attr.key.as_ref());
                    let val: Cow<str> = String::from_utf8_lossy(&attr.value);
                    budget.check_attr(&key, &val)?;
                    let trimmed_val = val.trim();
                    if !trimmed_val.is_empty() {
                        attrs.insert(key.to_string(), trimmed_val.to_string());
                    }
                }

                let level = ((depth as u8) + 1).min(6);
                let mut elem =
                    InternalElement::text(ElementKind::Heading { level }, &name_owned, depth).with_index(index);
                if !attrs.is_empty() {
                    elem = elem.with_attributes(attrs);
                }
                doc.push_element(elem);
                index += 1;

                element_stack.push(name_owned);
                depth = depth.saturating_add(1);
            }
            Ok(Event::End(_)) => {
                budget.leave();
                element_stack.pop();
                depth = depth.saturating_sub(1);
            }
            Ok(Event::Text(e)) => {
                if is_svg {
                    let in_text_elem = element_stack
                        .iter()
                        .any(|n| matches!(n.as_str(), "text" | "tspan" | "title" | "desc" | "textPath"));
                    if !in_text_elem {
                        buf.clear();
                        continue;
                    }
                }
                let text: std::borrow::Cow<str> = String::from_utf8_lossy(e.as_ref());
                budget.check_entity(&text)?;
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    budget.account_text(trimmed.len())?;
                    // Text content should be indented at the same level as its immediate parent,
                    // not one level deeper. Since we incremented depth after the opening tag,
                    // the parent heading is at (current depth - 1).
                    let text_depth = if depth > 0 { depth - 1 } else { 0 };
                    let elem = InternalElement::text(ElementKind::Paragraph, trimmed, text_depth).with_index(index);
                    doc.push_element(elem);
                    index += 1;
                }
            }
            Ok(Event::Empty(e)) => {
                let name_bytes = e.name().as_ref().to_vec();
                let name = String::from_utf8_lossy(&name_bytes).into_owned();

                let mut attrs = AHashMap::new();
                for attr in e.attributes().flatten() {
                    let key: std::borrow::Cow<str> = String::from_utf8_lossy(attr.key.as_ref());
                    let val: std::borrow::Cow<str> = String::from_utf8_lossy(&attr.value);
                    budget.check_attr(&key, &val)?;
                    let trimmed_val = val.trim();
                    if !trimmed_val.is_empty() {
                        attrs.insert(key.to_string(), trimmed_val.to_string());
                    }
                }

                let level = ((depth as u8) + 1).min(6);
                let mut elem = InternalElement::text(ElementKind::Heading { level }, &name, depth).with_index(index);
                if !attrs.is_empty() {
                    elem = elem.with_attributes(attrs);
                }
                doc.push_element(elem);
                index += 1;
            }
            Ok(Event::CData(e)) => {
                let text: std::borrow::Cow<str> = String::from_utf8_lossy(&e);
                budget.check_entity(&text)?;
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    budget.account_text(trimmed.len())?;
                    let elem = InternalElement::text(ElementKind::Paragraph, trimmed, depth).with_index(index);
                    doc.push_element(elem);
                    index += 1;
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(doc)
}

/// XML extractor.
///
/// Extracts text content from XML files, preserving element structure information.
pub struct XmlExtractor;

impl XmlExtractor {
    /// Create a new XML extractor.
    pub(crate) fn new() -> Self {
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
    fn extract_sync(&self, content: &[u8], mime_type: &str, config: &ExtractionConfig) -> Result<InternalDocument> {
        let default_limits;
        let limits: &crate::extractors::security::SecurityLimits = if let Some(ref l) = config.security_limits {
            l
        } else {
            default_limits = crate::extractors::security::SecurityLimits::default();
            &default_limits
        };
        let xml_result = if mime_type == "image/svg+xml" {
            parse_xml_svg(content, false, limits)?
        } else {
            parse_xml(content, false, limits)?
        };

        let mut budget = SecurityBudget::from_config(config);
        let mut doc = build_internal_document(content, mime_type, &mut budget)?;
        doc.mime_type = std::borrow::Cow::Owned(mime_type.to_string());

        doc.metadata = Metadata {
            format: Some(crate::types::FormatMetadata::Xml(crate::types::XmlMetadata {
                element_count: xml_result.element_count,
                unique_elements: xml_result.unique_elements,
            })),
            ..Default::default()
        };

        Ok(doc)
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl DocumentExtractor for XmlExtractor {
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
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
