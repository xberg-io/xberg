//! EPUB ZIP archive and XML parsing utilities.
//!
//! Provides low-level parsing functionality for EPUB container structure,
//! including ZIP archive operations and container.xml parsing.

use crate::Result;
use roxmltree;
use std::io::Cursor;
use zip::ZipArchive;

/// Parse container.xml to find the OPF file path
pub(super) fn parse_container_xml(xml: &str) -> Result<String> {
    match roxmltree::Document::parse(xml) {
        Ok(doc) => {
            for node in doc.descendants() {
                if node.tag_name().name() == "rootfile"
                    && let Some(full_path) = node.attribute("full-path")
                {
                    return Ok(full_path.to_string());
                }
            }
            Err(crate::KreuzbergError::Parsing {
                message: "No rootfile found in container.xml".to_string(),
                source: None,
            })
        }
        Err(e) => Err(crate::KreuzbergError::Parsing {
            message: format!("Failed to parse container.xml: {}", e),
            source: None,
        }),
    }
}

/// Read a file from the ZIP archive
pub(super) fn read_file_from_zip(archive: &mut ZipArchive<Cursor<Vec<u8>>>, path: &str) -> Result<String> {
    match archive.by_name(path) {
        Ok(mut file) => {
            let mut content = String::new();
            match std::io::Read::read_to_string(&mut file, &mut content) {
                Ok(_) => Ok(content),
                Err(e) => Err(crate::KreuzbergError::Parsing {
                    message: format!("Failed to read file from EPUB: {}", e),
                    source: None,
                }),
            }
        }
        Err(e) => Err(crate::KreuzbergError::Parsing {
            message: format!("File not found in EPUB: {} ({})", path, e),
            source: None,
        }),
    }
}

/// Resolve a relative path within the manifest directory
pub(super) fn resolve_path(base_dir: &str, relative_path: &str) -> String {
    if relative_path.starts_with('/') {
        relative_path.trim_start_matches('/').to_string()
    } else if base_dir.is_empty() || base_dir == "." {
        relative_path.to_string()
    } else {
        format!("{}/{}", base_dir.trim_end_matches('/'), relative_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_path_with_base_dir() {
        let result = resolve_path("OEBPS", "chapter.xhtml");
        assert_eq!(result, "OEBPS/chapter.xhtml");
    }

    #[test]
    fn test_resolve_path_absolute() {
        let result = resolve_path("OEBPS", "/chapter.xhtml");
        assert_eq!(result, "chapter.xhtml");
    }

    #[test]
    fn test_resolve_path_empty_base() {
        let result = resolve_path("", "chapter.xhtml");
        assert_eq!(result, "chapter.xhtml");
    }
}
