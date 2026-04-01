//! PST (Outlook Personal Folders) extractor.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extractors::SyncExtractor;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
use crate::types::metadata::PstMetadata;
use crate::types::{FormatMetadata, Metadata};
use async_trait::async_trait;
use std::borrow::Cow;
#[cfg(feature = "tokio-runtime")]
use std::path::Path;

/// PST file extractor.
///
/// Supports: .pst (Microsoft Outlook Personal Folders)
pub struct PstExtractor;

impl Default for PstExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl PstExtractor {
    pub fn new() -> Self {
        Self
    }
}

impl Plugin for PstExtractor {
    fn name(&self) -> &str {
        "pst-extractor"
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
}

impl SyncExtractor for PstExtractor {
    fn extract_sync(&self, content: &[u8], mime_type: &str, _config: &ExtractionConfig) -> Result<InternalDocument> {
        let (messages, _processing_warnings) = crate::extraction::pst::extract_pst_messages(content)?;

        let mut doc = InternalDocument::new("pst");
        doc.mime_type = Cow::Owned(mime_type.to_string());

        for msg in &messages {
            let msg_text = crate::extraction::email::build_email_text_output(msg);
            if !msg_text.is_empty() {
                for paragraph in msg_text.split("\n\n") {
                    let trimmed = paragraph.trim();
                    if !trimmed.is_empty() {
                        doc.push_element(InternalElement::text(ElementKind::Paragraph, trimmed, 0));
                    }
                }
            }
        }

        // Use metadata from the first message if available (archive-level metadata)
        let (subject, created_at) = if let Some(first) = messages.first() {
            (first.subject.clone(), first.date.clone())
        } else {
            (None, None)
        };

        let pst_metadata = PstMetadata {
            message_count: messages.len(),
        };

        doc.metadata = Metadata {
            format: Some(FormatMetadata::Pst(pst_metadata)),
            subject,
            created_at,
            ..Default::default()
        };

        Ok(doc)
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl DocumentExtractor for PstExtractor {
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        let mut doc = self.extract_sync(content, mime_type, config)?;

        // Recursively extract attachments from all messages when depth allows.
        if config.max_archive_depth > 0
            && let Ok((messages, _)) = crate::extraction::pst::extract_pst_messages(content)
        {
            let all_attachments: Vec<_> = messages.iter().flat_map(|m| m.attachments.iter()).cloned().collect();
            let (children, warnings) =
                crate::extractors::email::extract_attachment_children(&all_attachments, config).await;
            if !children.is_empty() {
                doc.children = Some(children);
            }
            doc.processing_warnings.extend(warnings);
        }

        Ok(doc)
    }

    #[cfg(feature = "tokio-runtime")]
    #[cfg_attr(feature = "otel", tracing::instrument(
        skip(self, path, config),
        fields(
            extractor.name = self.name(),
        )
    ))]
    async fn extract_file(&self, path: &Path, mime_type: &str, config: &ExtractionConfig) -> Result<InternalDocument> {
        // Call extract_pst_from_path directly to avoid reading the whole file into memory
        // before writing it back out to a tempfile — PSTs can be multi-GB.
        let (messages, _processing_warnings) = crate::extraction::pst::extract_pst_from_path(path)?;

        let mut doc = InternalDocument::new("pst");
        doc.mime_type = std::borrow::Cow::Owned(mime_type.to_string());

        for msg in &messages {
            let msg_text = crate::extraction::email::build_email_text_output(msg);
            if !msg_text.is_empty() {
                for paragraph in msg_text.split("\n\n") {
                    let trimmed = paragraph.trim();
                    if !trimmed.is_empty() {
                        doc.push_element(InternalElement::text(ElementKind::Paragraph, trimmed, 0));
                    }
                }
            }
        }

        let (subject, created_at) = if let Some(first) = messages.first() {
            (first.subject.clone(), first.date.clone())
        } else {
            (None, None)
        };

        let pst_metadata = PstMetadata {
            message_count: messages.len(),
        };

        doc.metadata = crate::types::Metadata {
            format: Some(FormatMetadata::Pst(pst_metadata)),
            subject,
            created_at,
            ..Default::default()
        };

        // Recursively extract attachments from all messages when depth allows.
        if config.max_archive_depth > 0 {
            let all_attachments: Vec<_> = messages.iter().flat_map(|m| m.attachments.iter()).cloned().collect();
            let (children, warnings) =
                crate::extractors::email::extract_attachment_children(&all_attachments, config).await;
            if !children.is_empty() {
                doc.children = Some(children);
            }
            doc.processing_warnings.extend(warnings);
        }

        Ok(doc)
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/vnd.ms-outlook-pst"]
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

    #[test]
    fn test_pst_extractor_plugin_interface() {
        let extractor = PstExtractor::new();
        assert_eq!(extractor.name(), "pst-extractor");
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[test]
    fn test_pst_extractor_supported_mime_types() {
        let extractor = PstExtractor::new();
        let mime_types = extractor.supported_mime_types();
        assert_eq!(mime_types.len(), 1);
        assert!(mime_types.contains(&"application/vnd.ms-outlook-pst"));
    }

    #[test]
    fn test_pst_extractor_invalid_data() {
        let config = ExtractionConfig::default();
        let extractor = PstExtractor::new();
        let result = extractor.extract_sync(b"not a pst file", "application/vnd.ms-outlook-pst", &config);
        assert!(result.is_err());
    }
}
