//! Metadata extraction from EPUB OPF files.
//!
//! Handles parsing of OPF (Open Packaging Format) files and extraction of
//! Dublin Core metadata following EPUB2 and EPUB3 standards.

use crate::Result;
use roxmltree;
use std::collections::BTreeMap;

/// Metadata extracted from OPF (Open Packaging Format) file
#[derive(Debug, Default, Clone)]
pub(super) struct OepbMetadata {
    pub(super) title: Option<String>,
    pub(super) creator: Option<String>,
    pub(super) date: Option<String>,
    pub(super) language: Option<String>,
    pub(super) identifier: Option<String>,
    pub(super) publisher: Option<String>,
    pub(super) subject: Option<String>,
    pub(super) description: Option<String>,
    pub(super) rights: Option<String>,
}

/// Extract metadata from EPUB OPF file
pub(super) fn extract_metadata(opf_xml: &str) -> Result<(OepbMetadata, BTreeMap<String, serde_json::Value>)> {
    let mut additional_metadata = BTreeMap::new();

    let (epub_metadata, _) = parse_opf(opf_xml)?;

    if let Some(identifier) = epub_metadata.identifier.clone() {
        additional_metadata.insert("identifier".to_string(), serde_json::json!(identifier));
    }

    if let Some(publisher) = epub_metadata.publisher.clone() {
        additional_metadata.insert("publisher".to_string(), serde_json::json!(publisher));
    }

    if let Some(subject) = epub_metadata.subject.clone() {
        additional_metadata.insert("subject".to_string(), serde_json::json!(subject));
    }

    if let Some(description) = epub_metadata.description.clone() {
        additional_metadata.insert("description".to_string(), serde_json::json!(description));
    }

    if let Some(rights) = epub_metadata.rights.clone() {
        additional_metadata.insert("rights".to_string(), serde_json::json!(rights));
    }

    Ok((epub_metadata, additional_metadata))
}

/// Parse OPF file and extract metadata and spine order
pub(super) fn parse_opf(xml: &str) -> Result<(OepbMetadata, Vec<String>)> {
    match roxmltree::Document::parse(xml) {
        Ok(doc) => {
            let root = doc.root();

            let mut metadata = OepbMetadata::default();
            let mut manifest: BTreeMap<String, String> = BTreeMap::new();
            let mut spine_order: Vec<String> = Vec::new();

            for node in root.descendants() {
                match node.tag_name().name() {
                    "title" => {
                        if let Some(text) = node.text() {
                            metadata.title = Some(text.trim().to_string());
                        }
                    }
                    "creator" => {
                        if let Some(text) = node.text() {
                            metadata.creator = Some(text.trim().to_string());
                        }
                    }
                    "date" => {
                        if let Some(text) = node.text() {
                            metadata.date = Some(text.trim().to_string());
                        }
                    }
                    "language" => {
                        if let Some(text) = node.text() {
                            metadata.language = Some(text.trim().to_string());
                        }
                    }
                    "identifier" => {
                        if let Some(text) = node.text() {
                            metadata.identifier = Some(text.trim().to_string());
                        }
                    }
                    "publisher" => {
                        if let Some(text) = node.text() {
                            metadata.publisher = Some(text.trim().to_string());
                        }
                    }
                    "subject" => {
                        if let Some(text) = node.text() {
                            metadata.subject = Some(text.trim().to_string());
                        }
                    }
                    "description" => {
                        if let Some(text) = node.text() {
                            metadata.description = Some(text.trim().to_string());
                        }
                    }
                    "rights" => {
                        if let Some(text) = node.text() {
                            metadata.rights = Some(text.trim().to_string());
                        }
                    }
                    "item" => {
                        if let Some(id) = node.attribute("id")
                            && let Some(href) = node.attribute("href")
                        {
                            manifest.insert(id.to_string(), href.to_string());
                        }
                    }
                    _ => {}
                }
            }

            for node in root.descendants() {
                if node.tag_name().name() == "itemref"
                    && let Some(idref) = node.attribute("idref")
                    && let Some(href) = manifest.get(idref)
                {
                    spine_order.push(href.clone());
                }
            }

            Ok((metadata, spine_order))
        }
        Err(e) => Err(crate::KreuzbergError::Parsing {
            message: format!("Failed to parse OPF file: {}", e),
            source: None,
        }),
    }
}
