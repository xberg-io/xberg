//! Hangul Word Processor XML (.hwpx) extractor.
//!
//! Extracts text, headings, tables, and images from HWPX documents using the `unhwp` crate.

use std::borrow::Cow;
use std::io::Cursor;

use async_trait::async_trait;
use bytes::Bytes;

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extractors::security::ZipBombValidator;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::ExtractedImage;
use crate::types::internal::InternalDocument;
use crate::types::internal_builder::InternalDocumentBuilder;

/// Extractor for Hangul Word Processor XML (.hwpx) files.
///
/// Supports HWPX (Open HWPML), the ZIP-based XML successor to the binary HWP 5.0 format.
pub struct HwpxExtractor;

impl HwpxExtractor {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl Default for HwpxExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for HwpxExtractor {
    fn name(&self) -> &str {
        "hwpx-extractor"
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
        "Hangul Word Processor XML (.hwpx) text extraction"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

fn mime_to_format(mime: &str) -> Cow<'static, str> {
    match mime {
        "image/png" => Cow::Borrowed("png"),
        "image/jpeg" | "image/jpg" => Cow::Borrowed("jpeg"),
        "image/gif" => Cow::Borrowed("gif"),
        "image/bmp" => Cow::Borrowed("bmp"),
        "image/webp" => Cow::Borrowed("webp"),
        _ => Cow::Borrowed("bin"),
    }
}

fn build_hwpx_internal_document(doc: unhwp::model::Document, mime_type: &str) -> InternalDocument {
    let mut builder = InternalDocumentBuilder::new("hwpx");
    builder.set_mime_type(Cow::Owned(mime_type.to_string()));

    let mut metadata = crate::types::metadata::Metadata::default();
    if let Some(title) = &doc.metadata.title {
        metadata.title = Some(title.clone());
    }
    if let Some(author) = &doc.metadata.author {
        metadata.authors = Some(vec![author.clone()]);
    }
    if let Some(subject) = &doc.metadata.subject {
        metadata.subject = Some(subject.clone());
    }
    if !doc.metadata.keywords.is_empty() {
        metadata.keywords = Some(doc.metadata.keywords.clone());
    }
    if let Some(created) = &doc.metadata.created {
        metadata.created_at = Some(created.clone());
    }
    if let Some(modified) = &doc.metadata.modified {
        metadata.modified_at = Some(modified.clone());
    }
    if let Some(creator_app) = &doc.metadata.creator_app {
        metadata.additional.insert(
            Cow::Borrowed("creator_app"),
            serde_json::Value::String(creator_app.clone()),
        );
    }
    if let Some(version) = &doc.metadata.format_version {
        metadata.document_version = Some(version.clone());
    }
    if !metadata.is_empty() {
        builder.set_metadata(metadata);
    }

    let mut image_index: usize = 0;

    for section in &doc.sections {
        for block in &section.content {
            match block {
                unhwp::model::Block::Paragraph(p) => {
                    if p.style.is_heading() && p.has_text_content() {
                        let text = p.plain_text();
                        let trimmed = text.trim();
                        if !trimmed.is_empty() {
                            builder.push_heading(p.style.heading_level, trimmed, None, None);
                        }
                    } else if p.has_text_content() {
                        let text = p.plain_text();
                        let trimmed = text.trim();
                        if !trimmed.is_empty() {
                            builder.push_paragraph(trimmed, vec![], None, None);
                        }
                    }

                    for inline in &p.content {
                        if let unhwp::model::InlineContent::Image(img_ref) = inline
                            && let Some(resource) = doc.resources.get(&img_ref.id)
                        {
                            let image = ExtractedImage {
                                data: Bytes::from(resource.data.clone()),
                                format: mime_to_format(resource.mime_type.as_deref().unwrap_or("")),
                                image_index: image_index as u32,
                                page_number: None,
                                width: img_ref.width,
                                height: img_ref.height,
                                colorspace: None,
                                bits_per_component: None,
                                is_mask: false,
                                description: img_ref.alt_text.clone(),
                                ocr_result: None,
                                bounding_box: None,
                                source_path: None,
                                image_kind: None,
                                kind_confidence: None,
                                cluster_id: None,
                            };
                            builder.push_image(img_ref.alt_text.as_deref(), image, None, None);
                            image_index += 1;
                        }
                    }
                }
                unhwp::model::Block::Table(t) => {
                    if !t.rows.is_empty() {
                        let cells: Vec<Vec<String>> = t
                            .rows
                            .iter()
                            .map(|row| row.cells.iter().map(|cell| cell.plain_text()).collect())
                            .collect();
                        builder.push_table_from_cells(&cells, None, None);
                    }
                }
            }
        }
    }

    builder.build()
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl DocumentExtractor for HwpxExtractor {
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        let limits = config.security_limits.clone().unwrap_or_default();

        if content.len() as u64 > limits.max_archive_size as u64 {
            return Err(crate::KreuzbergError::validation(format!(
                "HWPX file exceeds size limit ({} > {} bytes)",
                content.len(),
                limits.max_archive_size
            )));
        }

        let cursor = Cursor::new(content);
        let mut archive = zip::ZipArchive::new(cursor)
            .map_err(|e| crate::KreuzbergError::parsing(format!("invalid HWPX zip: {e}")))?;
        ZipBombValidator::new(limits)
            .validate(&mut archive)
            .map_err(|e| crate::KreuzbergError::validation(e.to_string()))?;

        let doc = unhwp::parse_bytes(content)
            .map_err(|e| crate::KreuzbergError::parsing(format!("Failed to parse HWPX: {e}")))?;
        Ok(build_hwpx_internal_document(doc, mime_type))
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/haansofthwpx"]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hwpx_extractor_plugin_interface() {
        let extractor = HwpxExtractor::new();
        assert_eq!(extractor.name(), "hwpx-extractor");
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert_eq!(extractor.priority(), 50);
        assert_eq!(extractor.supported_mime_types(), &["application/haansofthwpx"]);
    }

    #[test]
    fn test_hwpx_extractor_initialize_shutdown() {
        let extractor = HwpxExtractor::new();
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[tokio::test]
    async fn test_hwpx_extract_real_document() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../test_documents/hwpx/simple.hwpx");
        let content = std::fs::read(path).expect("test_documents/hwpx/simple.hwpx must exist");
        let extractor = HwpxExtractor::new();
        let result = extractor
            .extract_bytes(&content, "application/haansofthwpx", &ExtractionConfig::default())
            .await
            .expect("extraction of simple.hwpx must succeed");

        let text = result.content();
        assert!(
            text.contains("Hello from HWPX document"),
            "expected body text not found; got: {text}"
        );
    }

    #[tokio::test]
    async fn test_hwpx_extract_corrupted_returns_err() {
        let extractor = HwpxExtractor::new();
        let result = extractor
            .extract_bytes(b"not a zip", "application/haansofthwpx", &ExtractionConfig::default())
            .await;
        assert!(result.is_err(), "corrupted input must return Err, not panic");
    }

    fn make_zip_with_ratio(uncompressed_len: usize) -> Vec<u8> {
        use std::io::Write as _;
        let mut buf = std::io::Cursor::new(Vec::new());
        let mut zw = zip::ZipWriter::new(&mut buf);
        let opts = zip::write::FileOptions::<()>::default().compression_method(zip::CompressionMethod::Deflated);
        zw.start_file("content.hml", opts).unwrap();
        zw.write_all(&vec![0u8; uncompressed_len]).unwrap();
        zw.finish().unwrap();
        buf.into_inner()
    }

    fn make_zip_with_n_files(n: usize) -> Vec<u8> {
        use std::io::Write as _;
        let mut buf = std::io::Cursor::new(Vec::new());
        let mut zw = zip::ZipWriter::new(&mut buf);
        let opts = zip::write::FileOptions::<()>::default().compression_method(zip::CompressionMethod::Stored);
        for i in 0..n {
            zw.start_file(format!("f{i}.bin"), opts).unwrap();
            zw.write_all(b"x").unwrap();
        }
        zw.finish().unwrap();
        buf.into_inner()
    }

    #[tokio::test]
    async fn test_hwpx_rejects_zip_bomb_default_limits() {
        let zip_bytes = make_zip_with_ratio(256 * 1024);
        let extractor = HwpxExtractor::new();
        let result = extractor
            .extract_bytes(&zip_bytes, "application/haansofthwpx", &ExtractionConfig::default())
            .await;
        assert!(result.is_err(), "default limits must block a >100:1 zip bomb");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("ZIP bomb") || err.contains("ratio") || err.contains("validation"),
            "error should mention bomb/ratio/validation, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_hwpx_rejects_zip_bomb() {
        use crate::extractors::security::SecurityLimits;
        let zip_bytes = make_zip_with_ratio(8 * 1024);
        let config = ExtractionConfig {
            security_limits: Some(SecurityLimits {
                max_compression_ratio: 1,
                ..SecurityLimits::default()
            }),
            ..ExtractionConfig::default()
        };
        let extractor = HwpxExtractor::new();
        let result = extractor
            .extract_bytes(&zip_bytes, "application/haansofthwpx", &config)
            .await;
        assert!(result.is_err(), "zip bomb must be rejected");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("ZIP bomb") || err.contains("ratio") || err.contains("validation"),
            "error should mention bomb/ratio/validation, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_hwpx_rejects_oversized_file() {
        use crate::extractors::security::SecurityLimits;
        let limits = SecurityLimits {
            max_archive_size: 10,
            ..SecurityLimits::default()
        };
        let config = ExtractionConfig {
            security_limits: Some(limits),
            ..ExtractionConfig::default()
        };
        let oversized = vec![0u8; 11];
        let extractor = HwpxExtractor::new();
        let result = extractor
            .extract_bytes(&oversized, "application/haansofthwpx", &config)
            .await;
        assert!(result.is_err(), "oversized file must be rejected");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("size limit") || err.contains("validation"),
            "error should mention size limit, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_hwpx_rejects_too_many_files() {
        use crate::extractors::security::SecurityLimits;
        let zip_bytes = make_zip_with_n_files(3);
        let config = ExtractionConfig {
            security_limits: Some(SecurityLimits {
                max_files_in_archive: 2,
                ..SecurityLimits::default()
            }),
            ..ExtractionConfig::default()
        };
        let extractor = HwpxExtractor::new();
        let result = extractor
            .extract_bytes(&zip_bytes, "application/haansofthwpx", &config)
            .await;
        assert!(result.is_err(), "archive exceeding file-count limit must be rejected");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("files") || err.contains("count") || err.contains("validation"),
            "error should mention file count, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_hwpx_valid_zip_passes_security_check() {
        use crate::extractors::security::SecurityLimits;
        let zip_bytes = make_zip_with_ratio(1024);
        let config = ExtractionConfig {
            security_limits: Some(SecurityLimits {
                max_compression_ratio: 10_000,
                max_archive_size: 10 * 1024 * 1024,
                max_files_in_archive: 1_000,
                ..SecurityLimits::default()
            }),
            ..ExtractionConfig::default()
        };
        let extractor = HwpxExtractor::new();
        let result = extractor
            .extract_bytes(&zip_bytes, "application/haansofthwpx", &config)
            .await;
        // The ZIP is valid but not a real HWPX — we expect a parse error, not a security error.
        let is_parse_err = match &result {
            Err(e) => {
                let msg = e.to_string();
                !msg.contains("ZIP bomb")
                    && !msg.contains("ratio")
                    && !msg.contains("size limit")
                    && !msg.contains("too many files")
            }
            Ok(_) => true,
        };
        assert!(
            is_parse_err,
            "security validator must not reject a safe ZIP; got: {result:?}"
        );
    }
}
