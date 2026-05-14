//! Metadata extraction and notes handling.
//!
//! This module provides functionality for extracting metadata from PPTX files
//! and extracting notes from slides.

use std::collections::HashMap;
use std::io::{Read, Seek};
use zip::ZipArchive;

use crate::error::Result;
use crate::text::utf8_validation;
use crate::types::metadata::PptxMetadata;
use roxmltree::Document;

#[cfg(feature = "office")]
use crate::extraction::office_metadata::{
    extract_core_properties, extract_custom_properties, extract_pptx_app_properties,
};
#[cfg(feature = "office")]
use serde_json::Value;

use super::container::PptxContainer;

use crate::extraction::ooxml_constants::DRAWINGML_NAMESPACE;

/// Extract comprehensive metadata from PPTX using office_metadata module.
///
/// Returns `(PptxMetadata, HashMap<String, String>)` where the second element
/// contains office metadata keys (title, author, created_by, etc.).
pub(super) fn extract_metadata<R: Read + Seek>(archive: &mut ZipArchive<R>) -> (PptxMetadata, HashMap<String, String>) {
    #[cfg(feature = "office")]
    {
        let mut metadata_map = HashMap::new();
        let mut slide_count = 0;
        let mut slide_names = Vec::new();

        if let Ok(core) = extract_core_properties(archive) {
            if let Some(title) = core.title {
                metadata_map.insert("title".to_string(), title);
            }
            if let Some(creator) = core.creator {
                metadata_map.insert("author".to_string(), creator.clone());
                metadata_map.insert("created_by".to_string(), creator);
            }
            if let Some(subject) = core.subject {
                metadata_map.insert("subject".to_string(), subject.clone());
                metadata_map.insert("summary".to_string(), subject);
            }
            if let Some(keywords) = core.keywords {
                metadata_map.insert("keywords".to_string(), keywords);
            }
            if let Some(description) = core.description {
                metadata_map.insert("description".to_string(), description);
            }
            if let Some(modified_by) = core.last_modified_by {
                metadata_map.insert("modified_by".to_string(), modified_by);
            }
            if let Some(created) = core.created {
                metadata_map.insert("created_at".to_string(), created);
            }
            if let Some(modified) = core.modified {
                metadata_map.insert("modified_at".to_string(), modified);
            }
            if let Some(revision) = core.revision {
                metadata_map.insert("revision".to_string(), revision);
            }
            if let Some(category) = core.category {
                metadata_map.insert("category".to_string(), category);
            }
        }

        if let Ok(app) = extract_pptx_app_properties(archive) {
            if let Some(slides) = app.slides {
                metadata_map.insert("slide_count".to_string(), slides.to_string());
                slide_count = slides.max(0) as usize;
            }
            if let Some(notes) = app.notes {
                metadata_map.insert("notes_count".to_string(), notes.to_string());
            }
            if let Some(hidden_slides) = app.hidden_slides {
                metadata_map.insert("hidden_slides".to_string(), hidden_slides.to_string());
            }
            if !app.slide_titles.is_empty() {
                slide_names = app.slide_titles.clone();
                metadata_map.insert("slide_titles".to_string(), app.slide_titles.join(", "));
            }
            if let Some(presentation_format) = app.presentation_format {
                metadata_map.insert("presentation_format".to_string(), presentation_format);
            }
            if let Some(company) = app.company {
                metadata_map.insert("organization".to_string(), company);
            }
            if let Some(application) = app.application {
                metadata_map.insert("application".to_string(), application);
            }
            if let Some(app_version) = app.app_version {
                metadata_map.insert("application_version".to_string(), app_version);
            }
        }

        if let Ok(custom) = extract_custom_properties(archive) {
            for (key, value) in custom {
                let value_str = match value {
                    Value::String(s) => s,
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    Value::Null => "null".to_string(),
                    Value::Array(_) | Value::Object(_) => value.to_string(),
                };
                metadata_map.insert(format!("custom_{}", key), value_str);
            }
        }

        (
            PptxMetadata {
                slide_count: slide_count as u32,
                slide_names,
                image_count: None,
                table_count: None,
            },
            metadata_map,
        )
    }

    #[cfg(not(feature = "office"))]
    {
        (
            PptxMetadata {
                slide_count: 0,
                slide_names: Vec::new(),
            },
            HashMap::new(),
        )
    }
}

pub(super) fn extract_all_notes<R: Read + Seek>(container: &mut PptxContainer<R>) -> Result<HashMap<u32, String>> {
    let mut notes = HashMap::new();

    let slide_paths: Vec<String> = container.slide_paths().to_vec();

    for (i, slide_path) in slide_paths.iter().enumerate() {
        let notes_path = slide_path.replace("slides/slide", "notesSlides/notesSlide");
        if let Ok(notes_xml) = container.read_file(&notes_path)
            && let Ok(note_text) = extract_notes_text(&notes_xml)
        {
            notes.insert((i + 1) as u32, note_text);
        }
    }

    Ok(notes)
}

fn extract_notes_text(notes_xml: &[u8]) -> Result<String> {
    let xml_str = utf8_validation::from_utf8(notes_xml)
        .map_err(|e| crate::error::KreuzbergError::parsing(format!("Invalid UTF-8 in notes XML: {}", e)))?;

    let doc = Document::parse(xml_str)
        .map_err(|e| crate::error::KreuzbergError::parsing(format!("Failed to parse notes XML: {}", e)))?;

    let mut text_parts = Vec::with_capacity(16);
    for node in doc.descendants() {
        if node.has_tag_name((DRAWINGML_NAMESPACE, "t"))
            && let Some(text) = node.text()
        {
            text_parts.push(text);
        }
    }

    Ok(text_parts.join(" "))
}
