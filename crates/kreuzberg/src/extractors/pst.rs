//! PST (Outlook Personal Folders) extractor.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extractors::SyncExtractor;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::{EmailMetadata, ExtractionResult, Metadata};
use ahash::AHashMap;
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
    fn extract_sync(&self, content: &[u8], mime_type: &str, _config: &ExtractionConfig) -> Result<ExtractionResult> {
        let (messages, processing_warnings) = crate::extraction::pst::extract_pst_messages(content)?;

        let mut all_text_parts = Vec::with_capacity(messages.len());
        for msg in &messages {
            let msg_text = crate::extraction::email::build_email_text_output(msg);
            if !msg_text.is_empty() {
                all_text_parts.push(msg_text);
            }
        }

        let content_text = all_text_parts.join("\n\n---\n\n");

        // Use metadata from the first message if available (archive-level metadata)
        let (subject, format_metadata, created_at) = if let Some(first) = messages.first() {
            let attachment_names: Vec<String> = first
                .attachments
                .iter()
                .filter_map(|a| a.filename.clone().or_else(|| a.name.clone()))
                .collect();

            let email_metadata = EmailMetadata {
                from_email: first.from_email.clone(),
                from_name: None,
                to_emails: first.to_emails.clone(),
                cc_emails: first.cc_emails.clone(),
                bcc_emails: first.bcc_emails.clone(),
                message_id: first.message_id.clone(),
                attachments: attachment_names,
            };

            (first.subject.clone(), Some(crate::types::FormatMetadata::Email(email_metadata)), first.date.clone())
        } else {
            (None, None, None)
        };

        let mut additional: AHashMap<Cow<'static, str>, serde_json::Value> = AHashMap::new();
        additional.insert(
            Cow::Borrowed("message_count"),
            serde_json::json!(messages.len()),
        );

        Ok(ExtractionResult {
            content: content_text,
            mime_type: mime_type.to_string().into(),
            metadata: Metadata {
                format: format_metadata,
                subject,
                created_at,
                additional,
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
            document: None,
            #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
            extracted_keywords: None,
            quality_score: None,
            processing_warnings,
            annotations: None,
            children: None,
        })
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl DocumentExtractor for PstExtractor {
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

    #[cfg(feature = "tokio-runtime")]
    #[cfg_attr(feature = "otel", tracing::instrument(
        skip(self, path, config),
        fields(
            extractor.name = self.name(),
        )
    ))]
    #[cfg(feature = "tokio-runtime")]
    async fn extract_file(&self, path: &Path, mime_type: &str, config: &ExtractionConfig) -> Result<ExtractionResult> {
        let bytes = tokio::fs::read(path).await?;
        self.extract_bytes(&bytes, mime_type, config).await
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
