//! Email message extractor.

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

/// Email message extractor.
///
/// Supports: .eml, .msg
pub struct EmailExtractor;

impl Default for EmailExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl EmailExtractor {
    pub fn new() -> Self {
        Self
    }
}

impl Plugin for EmailExtractor {
    fn name(&self) -> &str {
        "email-extractor"
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

impl SyncExtractor for EmailExtractor {
    fn extract_sync(&self, content: &[u8], mime_type: &str, config: &ExtractionConfig) -> Result<ExtractionResult> {
        let fallback_codepage = config.email.as_ref().and_then(|e| e.msg_fallback_codepage);
        let email_result = crate::extraction::email::extract_email_content(content, mime_type, fallback_codepage)?;

        let text = crate::extraction::email::build_email_text_output(&email_result);

        let attachment_names: Vec<String> = email_result
            .attachments
            .iter()
            .filter_map(|att| att.filename.clone().or_else(|| att.name.clone()))
            .collect();

        let email_metadata = EmailMetadata {
            from_email: email_result.from_email.clone(),
            from_name: None,
            to_emails: email_result.to_emails.clone(),
            cc_emails: email_result.cc_emails.clone(),
            bcc_emails: email_result.bcc_emails.clone(),
            message_id: email_result.message_id.clone(),
            attachments: attachment_names,
        };

        // Filter out keys already represented in EmailMetadata to avoid
        // flattened field conflicts (e.g. "attachments" as string vs Vec).
        const EMAIL_STRUCT_KEYS: &[&str] = &[
            "from_email",
            "from_name",
            "to_emails",
            "cc_emails",
            "bcc_emails",
            "message_id",
            "attachments",
            "subject",
            "date",
        ];
        let mut additional = AHashMap::new();
        for (key, value) in &email_result.metadata {
            if !EMAIL_STRUCT_KEYS.contains(&key.as_str()) {
                additional.insert(Cow::Owned(key.clone()), serde_json::json!(value));
            }
        }

        Ok(ExtractionResult {
            content: text,
            mime_type: mime_type.to_string().into(),
            metadata: Metadata {
                format: Some(crate::types::FormatMetadata::Email(email_metadata)),
                subject: email_result.subject.clone(),
                created_at: email_result.date.clone(),
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
            processing_warnings: Vec::new(),
            annotations: None,
        })
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl DocumentExtractor for EmailExtractor {
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
        &["message/rfc822", "application/vnd.ms-outlook"]
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
    fn test_email_extractor_plugin_interface() {
        let extractor = EmailExtractor::new();
        assert_eq!(extractor.name(), "email-extractor");
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[test]
    fn test_email_extractor_supported_mime_types() {
        let extractor = EmailExtractor::new();
        let mime_types = extractor.supported_mime_types();
        assert_eq!(mime_types.len(), 2);
        assert!(mime_types.contains(&"message/rfc822"));
        assert!(mime_types.contains(&"application/vnd.ms-outlook"));
    }

    #[test]
    fn test_email_extractor_uses_config() {
        use crate::core::config::EmailConfig;

        // Extractor with email config set should not panic or error on invalid data
        let mut config = ExtractionConfig::default();
        config.email = Some(EmailConfig {
            msg_fallback_codepage: Some(1251),
        });
        let extractor = EmailExtractor::new();
        // Empty data returns a validation error — config is still used without panic
        let result = extractor.extract_sync(b"", "application/vnd.ms-outlook", &config);
        assert!(result.is_err());
    }
}
