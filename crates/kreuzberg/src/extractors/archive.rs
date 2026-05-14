//! Archive extractors for ZIP, TAR, 7z, and GZIP formats.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extraction::archive::{
    ArchiveMetadata as ExtractedMetadata, extract_7z_file_bytes, extract_7z_metadata, extract_7z_text_content,
    extract_gzip, extract_gzip_with_bytes, extract_tar_file_bytes, extract_tar_metadata, extract_tar_text_content,
    extract_zip_file_bytes, extract_zip_metadata, extract_zip_text_content,
};
use crate::extractors::SyncExtractor;
use crate::extractors::security::ZipBombValidator;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
use crate::types::{ArchiveMetadata, Metadata, ProcessingWarning};
use ahash::AHashMap;
use async_trait::async_trait;
use std::borrow::Cow;
use std::io::Cursor;

/// Build an `InternalDocument` from archive metadata and text contents.
///
/// Shared inner function — takes pre-computed children and warnings.
fn build_archive_doc_inner(
    extraction_metadata: ExtractedMetadata,
    text_contents: AHashMap<String, String>,
    format_name: &'static str,
    mime_type: &str,
    children: Vec<crate::types::ArchiveEntry>,
    processing_warnings: Vec<ProcessingWarning>,
) -> InternalDocument {
    let file_names: Vec<String> = extraction_metadata
        .file_list
        .iter()
        .map(|entry| entry.path.clone())
        .collect();

    let archive_metadata = ArchiveMetadata {
        format: Cow::Borrowed(format_name),
        file_count: extraction_metadata.file_count as u32,
        file_list: file_names,
        total_size: extraction_metadata.total_size,
        compressed_size: None,
    };

    let mut additional = AHashMap::new();
    let file_details: Vec<serde_json::Value> = extraction_metadata
        .file_list
        .iter()
        .map(|entry| {
            serde_json::json!({
                "path": entry.path,
                "size": entry.size,
                "is_dir": entry.is_dir,
            })
        })
        .collect();
    additional.insert(Cow::Borrowed("files"), serde_json::json!(file_details));

    let metadata = Metadata {
        format: Some(crate::types::FormatMetadata::Archive(archive_metadata)),
        additional,
        ..Default::default()
    };

    // Build InternalDocument with archive content as elements
    let mut doc = InternalDocument::new(format_name.to_lowercase());
    doc.mime_type = mime_type.to_string();
    doc.metadata = metadata;

    // Add archive summary as a paragraph element
    let mut idx = 0u32;
    let summary = format!(
        "{} Archive ({} files, {} bytes)",
        format_name, extraction_metadata.file_count, extraction_metadata.total_size
    );
    doc.push_element(InternalElement::text(ElementKind::Paragraph, &summary, 0).with_index(idx));
    idx += 1;

    // Add file listing
    let mut file_list = String::from("Files:\n");
    for entry in &extraction_metadata.file_list {
        file_list.push_str(&format!("- {} ({} bytes)\n", entry.path, entry.size));
    }
    doc.push_element(InternalElement::text(ElementKind::Paragraph, &file_list, 0).with_index(idx));
    idx += 1;

    // Add text file contents
    for (path, content) in &text_contents {
        let text = format!("=== {} ===\n{}", path, content);
        doc.push_element(InternalElement::text(ElementKind::Paragraph, &text, 0).with_index(idx));
        idx += 1;
    }

    // Store children (recursively extracted archive entries)
    doc.children = if children.is_empty() { None } else { Some(children) };
    doc.processing_warnings = processing_warnings;

    doc
}

/// Sync version — no recursive child extraction.
fn build_archive_doc_sync(
    extraction_metadata: ExtractedMetadata,
    text_contents: AHashMap<String, String>,
    format_name: &'static str,
    mime_type: &str,
) -> InternalDocument {
    build_archive_doc_inner(
        extraction_metadata,
        text_contents,
        format_name,
        mime_type,
        Vec::new(),
        Vec::new(),
    )
}

/// Async version with recursive extraction of archive children.
///
/// When `config.max_archive_depth > current_depth`, extracts each file in `file_bytes`
/// by detecting its MIME type and dispatching to the appropriate extractor.
async fn build_archive_doc(
    extraction_metadata: ExtractedMetadata,
    text_contents: AHashMap<String, String>,
    file_bytes: AHashMap<String, Vec<u8>>,
    format_name: &'static str,
    mime_type: &str,
    config: &ExtractionConfig,
    current_depth: usize,
) -> InternalDocument {
    let mut children = Vec::new();
    let mut processing_warnings = Vec::new();

    if config.max_archive_depth > current_depth && !file_bytes.is_empty() {
        for (path, bytes) in &file_bytes {
            let detected_mime = crate::core::mime::detect_mime_type_from_bytes(bytes).ok().or_else(|| {
                std::path::Path::new(path)
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .and_then(|ext| mime_guess::from_ext(ext).first())
                    .map(|m| m.to_string())
            });

            let file_mime = match detected_mime {
                Some(m) if m != "application/octet-stream" => m,
                _ => continue,
            };

            let mut child_config = config.clone();
            child_config.max_archive_depth = config.max_archive_depth.saturating_sub(current_depth + 1);

            match crate::core::extractor::extract_bytes(bytes, &file_mime, &child_config).await {
                Ok(result) => {
                    children.push(crate::types::ArchiveEntry {
                        path: path.clone(),
                        mime_type: file_mime,
                        result: Box::new(result),
                    });
                }
                Err(e) => {
                    processing_warnings.push(ProcessingWarning {
                        source: Cow::Borrowed("archive_recursive_extraction"),
                        message: Cow::Owned(format!("Failed to extract '{}': {}", path, e)),
                    });
                }
            }
        }
    }

    build_archive_doc_inner(
        extraction_metadata,
        text_contents,
        format_name,
        mime_type,
        children,
        processing_warnings,
    )
}

/// ZIP archive extractor.
///
/// Extracts file lists and text content from ZIP archives.
pub struct ZipExtractor;

impl ZipExtractor {
    /// Create a new ZIP extractor.
    pub(crate) fn new() -> Self {
        Self
    }
}

impl Default for ZipExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for ZipExtractor {
    fn name(&self) -> &str {
        "zip-extractor"
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
        "Extracts file lists and text content from ZIP archives"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl DocumentExtractor for ZipExtractor {
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        let limits = config.security_limits.clone().unwrap_or_default();

        // Validate ZIP archive for bomb attacks before extraction
        let cursor = Cursor::new(content);
        let mut archive = zip::ZipArchive::new(cursor)
            .map_err(|e| crate::error::KreuzbergError::parsing(format!("Failed to read ZIP archive: {}", e)))?;
        let validator = ZipBombValidator::new(limits.clone());
        validator
            .validate(&mut archive)
            .map_err(|e| crate::error::KreuzbergError::validation(e.to_string()))?;

        let extraction_metadata = extract_zip_metadata(content, &limits)?;
        let text_contents = extract_zip_text_content(content, &limits)?;
        let file_bytes = extract_zip_file_bytes(content, &limits)?;
        Ok(build_archive_doc(
            extraction_metadata,
            text_contents,
            file_bytes,
            "ZIP",
            mime_type,
            config,
            0,
        )
        .await)
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/zip", "application/x-zip-compressed"]
    }

    fn priority(&self) -> i32 {
        50
    }

    fn as_sync_extractor(&self) -> Option<&dyn SyncExtractor> {
        Some(self)
    }
}

impl SyncExtractor for ZipExtractor {
    fn extract_sync(&self, content: &[u8], mime_type: &str, config: &ExtractionConfig) -> Result<InternalDocument> {
        let limits = config.security_limits.clone().unwrap_or_default();
        let cursor = Cursor::new(content);
        let mut archive = zip::ZipArchive::new(cursor)
            .map_err(|e| crate::error::KreuzbergError::parsing(format!("Failed to read ZIP archive: {}", e)))?;
        let validator = ZipBombValidator::new(limits.clone());
        validator
            .validate(&mut archive)
            .map_err(|e| crate::error::KreuzbergError::validation(e.to_string()))?;

        let extraction_metadata = extract_zip_metadata(content, &limits)?;
        let text_contents = extract_zip_text_content(content, &limits)?;
        Ok(build_archive_doc_sync(
            extraction_metadata,
            text_contents,
            "ZIP",
            mime_type,
        ))
    }
}

/// TAR archive extractor.
///
/// Extracts file lists and text content from TAR archives.
pub struct TarExtractor;

impl TarExtractor {
    /// Create a new TAR extractor.
    pub(crate) fn new() -> Self {
        Self
    }
}

impl Default for TarExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for TarExtractor {
    fn name(&self) -> &str {
        "tar-extractor"
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
        "Extracts file lists and text content from TAR archives"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl DocumentExtractor for TarExtractor {
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        let limits = config.security_limits.clone().unwrap_or_default();
        let extraction_metadata = extract_tar_metadata(content, &limits)?;
        let text_contents = extract_tar_text_content(content, &limits)?;
        let file_bytes = extract_tar_file_bytes(content, &limits)?;
        Ok(build_archive_doc(
            extraction_metadata,
            text_contents,
            file_bytes,
            "TAR",
            mime_type,
            config,
            0,
        )
        .await)
    }

    fn supported_mime_types(&self) -> &[&str] {
        &[
            "application/x-tar",
            "application/tar",
            "application/x-gtar",
            "application/x-ustar",
        ]
    }

    fn priority(&self) -> i32 {
        50
    }

    fn as_sync_extractor(&self) -> Option<&dyn SyncExtractor> {
        Some(self)
    }
}

impl SyncExtractor for TarExtractor {
    fn extract_sync(&self, content: &[u8], mime_type: &str, _config: &ExtractionConfig) -> Result<InternalDocument> {
        let limits = _config.security_limits.clone().unwrap_or_default();
        let extraction_metadata = extract_tar_metadata(content, &limits)?;
        let text_contents = extract_tar_text_content(content, &limits)?;
        Ok(build_archive_doc_sync(
            extraction_metadata,
            text_contents,
            "TAR",
            mime_type,
        ))
    }
}

/// 7z archive extractor.
///
/// Extracts file lists and text content from 7z archives.
pub struct SevenZExtractor;

impl SevenZExtractor {
    /// Create a new 7z extractor.
    pub(crate) fn new() -> Self {
        Self
    }
}

impl Default for SevenZExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for SevenZExtractor {
    fn name(&self) -> &str {
        "7z-extractor"
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
        "Extracts file lists and text content from 7z archives"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl DocumentExtractor for SevenZExtractor {
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        let limits = config.security_limits.clone().unwrap_or_default();
        let extraction_metadata = extract_7z_metadata(content, &limits)?;
        let text_contents = extract_7z_text_content(content, &limits)?;
        let file_bytes = extract_7z_file_bytes(content, &limits)?;
        Ok(build_archive_doc(
            extraction_metadata,
            text_contents,
            file_bytes,
            "7Z",
            mime_type,
            config,
            0,
        )
        .await)
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/x-7z-compressed"]
    }

    fn priority(&self) -> i32 {
        50
    }

    fn as_sync_extractor(&self) -> Option<&dyn SyncExtractor> {
        Some(self)
    }
}

impl SyncExtractor for SevenZExtractor {
    fn extract_sync(&self, content: &[u8], mime_type: &str, _config: &ExtractionConfig) -> Result<InternalDocument> {
        let limits = _config.security_limits.clone().unwrap_or_default();
        let extraction_metadata = extract_7z_metadata(content, &limits)?;
        let text_contents = extract_7z_text_content(content, &limits)?;
        Ok(build_archive_doc_sync(
            extraction_metadata,
            text_contents,
            "7Z",
            mime_type,
        ))
    }
}

/// Gzip archive extractor.
///
/// Decompresses gzip files and extracts text content from the compressed data.
pub struct GzipExtractor;

impl GzipExtractor {
    /// Create a new gzip extractor.
    pub(crate) fn new() -> Self {
        Self
    }
}

impl Default for GzipExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for GzipExtractor {
    fn name(&self) -> &str {
        "gzip-extractor"
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
        "Decompresses and extracts text content from gzip-compressed files"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl DocumentExtractor for GzipExtractor {
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        let limits = config.security_limits.clone().unwrap_or_default();
        let (extraction_metadata, text_contents, file_bytes) = extract_gzip_with_bytes(content, &limits)?;
        Ok(build_archive_doc(
            extraction_metadata,
            text_contents,
            file_bytes,
            "GZIP",
            mime_type,
            config,
            0,
        )
        .await)
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/gzip", "application/x-gzip"]
    }

    fn priority(&self) -> i32 {
        50
    }

    fn as_sync_extractor(&self) -> Option<&dyn SyncExtractor> {
        Some(self)
    }
}

impl SyncExtractor for GzipExtractor {
    fn extract_sync(&self, content: &[u8], mime_type: &str, _config: &ExtractionConfig) -> Result<InternalDocument> {
        let limits = _config.security_limits.clone().unwrap_or_default();
        let (extraction_metadata, text_contents) = extract_gzip(content, &limits)?;
        Ok(build_archive_doc_sync(
            extraction_metadata,
            text_contents,
            "GZIP",
            mime_type,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};
    use tar::Builder as TarBuilder;
    use zip::write::{FileOptions, ZipWriter};

    #[tokio::test]
    async fn test_zip_extractor() {
        let extractor = ZipExtractor::new();

        let mut cursor = Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut cursor);
            let options = FileOptions::<'_, ()>::default();

            zip.start_file("test.txt", options).unwrap();
            zip.write_all(b"Hello, World!").unwrap();

            zip.finish().unwrap();
        }

        let bytes = cursor.into_inner();
        let config = ExtractionConfig::default();

        let result = extractor
            .extract_bytes(&bytes, "application/zip", &config)
            .await
            .unwrap();
        let result =
            crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);

        assert_eq!(result.mime_type, "application/zip");
        assert!(result.content.contains("ZIP Archive"));
        assert!(result.content.contains("test.txt"));
        assert!(result.content.contains("Hello, World!"));
        assert!(result.metadata.format.is_some());
        let archive_meta = match result.metadata.format.as_ref().unwrap() {
            crate::types::FormatMetadata::Archive(meta) => meta,
            _ => panic!("Expected Archive metadata"),
        };
        assert_eq!(archive_meta.format, "ZIP");
        assert_eq!(archive_meta.file_count, 1);
    }

    #[tokio::test]
    async fn test_tar_extractor() {
        let extractor = TarExtractor::new();

        let mut cursor = Cursor::new(Vec::new());
        {
            let mut tar = TarBuilder::new(&mut cursor);

            let data = b"Hello, World!";
            let mut header = tar::Header::new_gnu();
            header.set_path("test.txt").unwrap();
            header.set_size(data.len() as u64);
            header.set_cksum();
            tar.append(&header, &data[..]).unwrap();

            tar.finish().unwrap();
        }

        let bytes = cursor.into_inner();
        let config = ExtractionConfig::default();

        let result = extractor
            .extract_bytes(&bytes, "application/x-tar", &config)
            .await
            .unwrap();
        let result =
            crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);

        assert_eq!(result.mime_type, "application/x-tar");
        assert!(result.content.contains("TAR Archive"));
        assert!(result.content.contains("test.txt"));
        assert!(result.content.contains("Hello, World!"));
        assert!(result.metadata.format.is_some());
        let archive_meta = match result.metadata.format.as_ref().unwrap() {
            crate::types::FormatMetadata::Archive(meta) => meta,
            _ => panic!("Expected Archive metadata"),
        };
        assert_eq!(archive_meta.format, "TAR");
        assert_eq!(archive_meta.file_count, 1);
    }

    #[tokio::test]
    async fn test_zip_extractor_invalid() {
        let extractor = ZipExtractor::new();
        let invalid_bytes = vec![0, 1, 2, 3, 4, 5];
        let config = ExtractionConfig::default();

        let result = extractor
            .extract_bytes(&invalid_bytes, "application/zip", &config)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_tar_extractor_invalid() {
        let extractor = TarExtractor::new();
        let invalid_bytes = vec![0, 1, 2, 3, 4, 5];
        let config = ExtractionConfig::default();

        let result = extractor
            .extract_bytes(&invalid_bytes, "application/x-tar", &config)
            .await;
        assert!(result.is_err());
    }

    #[test]
    fn test_zip_plugin_interface() {
        let extractor = ZipExtractor::new();
        assert_eq!(extractor.name(), "zip-extractor");
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert!(extractor.supported_mime_types().contains(&"application/zip"));
        assert_eq!(extractor.priority(), 50);
    }

    #[test]
    fn test_tar_plugin_interface() {
        let extractor = TarExtractor::new();
        assert_eq!(extractor.name(), "tar-extractor");
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert!(extractor.supported_mime_types().contains(&"application/x-tar"));
        assert!(extractor.supported_mime_types().contains(&"application/tar"));
        assert_eq!(extractor.priority(), 50);
    }

    #[test]
    fn test_gzip_plugin_interface() {
        let extractor = GzipExtractor::new();
        assert_eq!(extractor.name(), "gzip-extractor");
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert!(extractor.supported_mime_types().contains(&"application/gzip"));
        assert!(extractor.supported_mime_types().contains(&"application/x-gzip"));
        assert_eq!(extractor.priority(), 50);
    }

    #[tokio::test]
    async fn test_gzip_extractor_valid_data() {
        use flate2::Compression;
        use flate2::write::GzEncoder;
        use std::io::Write;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(b"Hello from gzip extraction!").unwrap();
        let compressed = encoder.finish().unwrap();

        let extractor = GzipExtractor::new();
        let config = ExtractionConfig::default();
        let result = extractor.extract_bytes(&compressed, "application/gzip", &config).await;
        assert!(result.is_ok());
        let extraction = result.unwrap();
        let extraction = crate::extraction::derive::derive_extraction_result(
            extraction,
            true,
            crate::core::config::OutputFormat::Plain,
        );
        assert!(extraction.content.contains("Hello from gzip extraction!"));
    }

    #[tokio::test]
    async fn test_gzip_extractor_invalid_data() {
        let extractor = GzipExtractor::new();
        let config = ExtractionConfig::default();
        let result = extractor
            .extract_bytes(&[0, 1, 2, 3], "application/gzip", &config)
            .await;
        assert!(result.is_err());
    }
}
