//! Email message extractor.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extractors::SyncExtractor;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::internal::InternalDocument;
use crate::types::internal_builder::InternalDocumentBuilder;
use crate::types::metadata::Metadata;
use crate::types::{ArchiveEntry, EmailMetadata, ProcessingWarning};
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

impl EmailExtractor {
    /// Build an `InternalDocument` from extracted email content.
    ///
    /// Pushes email headers as a metadata block, then body content as paragraphs.
    fn build_internal_document(email_result: &crate::types::EmailExtractionResult) -> InternalDocument {
        let mut builder = InternalDocumentBuilder::new("email");

        // Push email headers as a metadata block
        let mut header_entries = Vec::new();
        if let Some(ref subject) = email_result.subject {
            header_entries.push(("Subject".to_string(), subject.clone()));
        }
        if let Some(ref from) = email_result.from_email {
            header_entries.push(("From".to_string(), from.clone()));
        }
        if !email_result.to_emails.is_empty() {
            header_entries.push(("To".to_string(), email_result.to_emails.join(", ")));
        }
        if !email_result.cc_emails.is_empty() {
            header_entries.push(("CC".to_string(), email_result.cc_emails.join(", ")));
        }
        if let Some(ref date) = email_result.date {
            header_entries.push(("Date".to_string(), date.clone()));
        }
        if !header_entries.is_empty() {
            builder.push_metadata_block(&header_entries, None);
        }

        // Push body content: if HTML body is available, walk the HTML
        // document structure for richer extraction; otherwise fall back to
        // plain text paragraph splitting.
        if let Some(ref html) = email_result.html_content {
            let html_doc = crate::extraction::html::structure::build_document_structure(html);
            for node in &html_doc.nodes {
                if node.parent.is_none() {
                    match &node.content {
                        crate::types::NodeContent::Paragraph { text } => {
                            let trimmed = text.trim();
                            if !trimmed.is_empty() {
                                builder.push_paragraph(trimmed, node.annotations.clone(), None, None);
                            }
                        }
                        crate::types::NodeContent::Heading { level, text } => {
                            builder.push_heading(*level, text.as_str(), None, None);
                        }
                        crate::types::NodeContent::List { ordered } => {
                            builder.push_list(*ordered);
                            for &child_idx in &node.children {
                                if let Some(child) = html_doc.nodes.get(child_idx.0 as usize)
                                    && let crate::types::NodeContent::ListItem { text } = &child.content
                                {
                                    builder.push_list_item(text.as_str(), *ordered, vec![], None, None);
                                }
                            }
                            builder.end_list();
                        }
                        crate::types::NodeContent::Code { text, language } => {
                            builder.push_code(text.as_str(), language.as_deref(), None, None);
                        }
                        _ => {
                            if let Some(text) = node.content.text() {
                                let trimmed = text.trim();
                                if !trimmed.is_empty() {
                                    builder.push_paragraph(trimmed, node.annotations.clone(), None, None);
                                }
                            }
                        }
                    }
                }
            }
        } else {
            for paragraph in email_result.cleaned_text.split("\n\n") {
                let trimmed = paragraph.trim();
                if !trimmed.is_empty() {
                    builder.push_paragraph(trimmed, vec![], None, None);
                }
            }
        }

        builder.build()
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
    fn extract_sync(&self, content: &[u8], mime_type: &str, config: &ExtractionConfig) -> Result<InternalDocument> {
        let fallback_codepage = config.email.as_ref().and_then(|e| e.msg_fallback_codepage);
        let email_result = crate::extraction::email::extract_email_content(content, mime_type, fallback_codepage)?;

        let attachment_names: Vec<String> = email_result
            .attachments
            .iter()
            .filter_map(|att| att.filename.clone().or_else(|| att.name.clone()))
            .collect();

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
            "email_from",
            "email_to",
            "email_cc",
            "email_bcc",
        ];
        let mut additional = AHashMap::new();
        for (key, value) in &email_result.metadata {
            if !EMAIL_STRUCT_KEYS.contains(&key.as_str()) {
                additional.insert(Cow::Owned(key.clone()), serde_json::json!(value));
            }
        }

        // Build internal document from email content
        let mut doc = Self::build_internal_document(&email_result);
        doc.mime_type = Cow::Owned(mime_type.to_string());

        // Move fields out of email_result now that all borrows above are complete.
        let subject = email_result.subject;
        let created_at = email_result.date;
        let from_name = email_result.metadata.get("from_name").cloned();
        let email_metadata = EmailMetadata {
            from_email: email_result.from_email,
            from_name: from_name.clone(),
            to_emails: email_result.to_emails,
            cc_emails: email_result.cc_emails,
            bcc_emails: email_result.bcc_emails,
            message_id: email_result.message_id,
            attachments: attachment_names,
        };

        // Map from_name to standard authors field
        let authors = from_name.filter(|n| !n.is_empty()).map(|n| vec![n]);

        doc.metadata = Metadata {
            format: Some(crate::types::FormatMetadata::Email(email_metadata)),
            subject,
            authors,
            created_at,
            additional,
            ..Default::default()
        };

        Ok(doc)
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl DocumentExtractor for EmailExtractor {
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        tracing::debug!(format = "email", size_bytes = content.len(), "extraction starting");
        let mut doc = self.extract_sync(content, mime_type, config)?;

        // Recursively extract attachment content and nested messages when archive depth allows.
        if config.max_archive_depth > 0 {
            let fallback_codepage = config.email.as_ref().and_then(|e| e.msg_fallback_codepage);
            if let Ok(email_result) =
                crate::extraction::email::extract_email_content(content, mime_type, fallback_codepage)
            {
                let (mut children, warnings) = extract_attachment_children(&email_result.attachments, config).await;

                // Also extract nested message/rfc822 parts (e.g. from multipart/digest)
                // as separate ArchiveEntry children for recursive processing.
                if mime_type == "message/rfc822" {
                    let (nested_children, nested_warnings) = extract_nested_message_children(content, config).await;
                    children.extend(nested_children);
                    doc.processing_warnings.extend(nested_warnings);
                }

                if !children.is_empty() {
                    doc.children = Some(children);
                }
                doc.processing_warnings.extend(warnings);
            }
        }

        tracing::debug!(
            element_count = doc.elements.len(),
            format = "email",
            "extraction complete"
        );
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

/// Recursively extract content from email attachments.
///
/// For each attachment with binary data, detects its MIME type and dispatches
/// to the appropriate extractor. Follows the same pattern as archive extractors.
/// Errors on individual attachments are captured as warnings, never failing
/// the whole extraction.
pub(crate) async fn extract_attachment_children(
    attachments: &[crate::types::EmailAttachment],
    config: &ExtractionConfig,
) -> (Vec<ArchiveEntry>, Vec<ProcessingWarning>) {
    let mut children = Vec::new();
    let mut warnings = Vec::new();

    for (idx, attachment) in attachments.iter().enumerate() {
        let bytes = match &attachment.data {
            Some(data) if !data.is_empty() => data,
            _ => continue,
        };

        let filename = attachment
            .filename
            .clone()
            .or_else(|| attachment.name.clone())
            .unwrap_or_else(|| format!("attachment_{}", idx));

        // Detect MIME type from bytes, falling back to extension-based detection,
        // then to the attachment's declared MIME type.
        let detected_mime = crate::core::mime::detect_mime_type_from_bytes(bytes)
            .ok()
            .or_else(|| {
                std::path::Path::new(&filename)
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .and_then(|ext| mime_guess::from_ext(ext).first())
                    .map(|m| m.to_string())
            })
            .or_else(|| attachment.mime_type.clone().filter(|m| m != "application/octet-stream"));

        let file_mime = match detected_mime {
            Some(m) if m != "application/octet-stream" => m,
            _ => continue,
        };

        let mut child_config = config.clone();
        child_config.max_archive_depth = config.max_archive_depth.saturating_sub(1);

        match crate::core::extractor::extract_bytes(bytes, &file_mime, &child_config).await {
            Ok(result) => {
                children.push(ArchiveEntry {
                    path: filename,
                    mime_type: file_mime,
                    result: Box::new(result),
                });
            }
            Err(e) => {
                warnings.push(ProcessingWarning {
                    source: Cow::Borrowed("email_attachment_extraction"),
                    message: Cow::Owned(format!("Failed to extract '{}': {}", filename, e)),
                });
            }
        }
    }

    (children, warnings)
}

/// Extract nested `message/rfc822` sub-messages as `ArchiveEntry` children.
///
/// Parses the top-level EML to find `PartType::Message` parts (e.g. in
/// `multipart/digest` emails) and recursively extracts each one via
/// `extract_bytes` with `message/rfc822` MIME type. This makes nested
/// messages available as separate children for recursive processing,
/// complementing the existing text-merging in `collect_nested_message_text`
/// / `collect_nested_message_html`.
async fn extract_nested_message_children(
    content: &[u8],
    config: &ExtractionConfig,
) -> (Vec<ArchiveEntry>, Vec<ProcessingWarning>) {
    use mail_parser::PartType;

    let mut children = Vec::new();
    let mut warnings = Vec::new();

    let message = match mail_parser::MessageParser::default().parse(content) {
        Some(msg) => msg,
        None => return (children, warnings),
    };

    let mut nested_idx: usize = 0;
    for part in &message.parts {
        if let PartType::Message(sub_msg) = &part.body {
            let raw_bytes = sub_msg.raw_message();
            if raw_bytes.is_empty() {
                continue;
            }

            let filename = format!("nested_message_{nested_idx}.eml");
            nested_idx += 1;

            let mut child_config = config.clone();
            child_config.max_archive_depth = config.max_archive_depth.saturating_sub(1);

            match crate::core::extractor::extract_bytes(raw_bytes, "message/rfc822", &child_config).await {
                Ok(result) => {
                    children.push(ArchiveEntry {
                        path: filename,
                        mime_type: "message/rfc822".to_string(),
                        result: Box::new(result),
                    });
                }
                Err(e) => {
                    warnings.push(ProcessingWarning {
                        source: Cow::Borrowed("nested_message_extraction"),
                        message: Cow::Owned(format!("Failed to extract '{}': {}", filename, e)),
                    });
                }
            }
        }
    }

    (children, warnings)
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
        let config = ExtractionConfig {
            email: Some(EmailConfig {
                msg_fallback_codepage: Some(1251),
            }),
            ..Default::default()
        };
        let extractor = EmailExtractor::new();
        // Empty data returns a validation error — config is still used without panic
        let result = extractor.extract_sync(b"", "application/vnd.ms-outlook", &config);
        assert!(result.is_err());
    }
}
