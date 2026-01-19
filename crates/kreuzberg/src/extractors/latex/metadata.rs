//! Metadata extraction for LaTeX documents.
//!
//! This module handles extraction of document metadata like title, author, and date
//! from LaTeX preamble commands.

use crate::types::Metadata;
use super::utilities::extract_braced;

/// Extracts metadata from a LaTeX line.
///
/// Looks for \title{}, \author{}, and \date{} commands and populates
/// the provided Metadata structure.
pub fn extract_metadata_from_line(line: &str, metadata: &mut Metadata) {
    if line.starts_with("\\title{") {
        if let Some(title) = extract_braced(line, "title") {
            metadata.additional.insert("title".to_string(), title.into());
        }
    } else if line.starts_with("\\author{") {
        if let Some(author) = extract_braced(line, "author") {
            metadata.additional.insert("author".to_string(), author.into());
        }
    } else if line.starts_with("\\date{") {
        if let Some(date) = extract_braced(line, "date") {
            metadata.additional.insert("date".to_string(), date.into());
        }
    }
}
