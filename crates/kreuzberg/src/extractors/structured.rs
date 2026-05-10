//! Structured data extractor (JSON, JSONL, YAML, TOML).

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extractors::security::SecurityBudget;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::internal::InternalDocument;
use crate::types::internal_builder::InternalDocumentBuilder;
use crate::types::metadata::Metadata;
use ahash::AHashMap;
use async_trait::async_trait;
use std::borrow::Cow;
#[cfg(feature = "tokio-runtime")]
use std::path::Path;

/// Build an `InternalDocument` from a structured data result.
///
/// For JSON objects: top-level keys become headings, nested objects become
/// sub-headings, arrays become lists. Falls back to a code block for other formats.
///
/// `budget` enforces hostile-input limits (nesting depth, iteration count, entity
/// length, cumulative content size). Any limit violation is converted into a
/// `KreuzbergError::Security` via the `?` operator.
fn build_internal_document(
    result: &crate::extraction::structured::StructuredDataResult,
    mime_type: &str,
    budget: &mut SecurityBudget,
) -> Result<InternalDocument> {
    let source_format = match mime_type {
        "application/json" | "text/json" | "application/csl+json" => "json",
        "application/x-ndjson" | "application/jsonl" | "application/x-jsonlines" => "jsonl",
        "application/yaml" | "application/x-yaml" | "text/yaml" | "text/x-yaml" => "yaml",
        "application/toml" | "text/toml" => "toml",
        _ => "structured",
    };

    let language = match source_format {
        "json" | "jsonl" => Some("json"),
        "yaml" => Some("yaml"),
        "toml" => Some("toml"),
        _ => None,
    };

    let mut builder = InternalDocumentBuilder::new(source_format);

    // Try to build structured document for JSON objects
    if source_format == "json"
        && let Ok(value) = serde_json::from_str::<serde_json::Value>(&result.content)
        && value.is_object()
    {
        build_json_internal_structure(&value, &mut builder, 1, budget)?;
        return Ok(builder.build());
    }

    // Fallback: code block — account for the full content size
    budget.account_text(result.content.len())?;
    builder.push_code(&result.content, language, None, None);
    Ok(builder.build())
}

/// Recursively build internal document structure from a JSON value.
///
/// `budget` enforces nesting depth, iteration count, entity length, and
/// cumulative text size limits against hostile input.
fn build_json_internal_structure(
    value: &serde_json::Value,
    builder: &mut InternalDocumentBuilder,
    depth: u8,
    budget: &mut SecurityBudget,
) -> Result<()> {
    let level = depth.min(6);
    match value {
        serde_json::Value::Object(map) => {
            budget.enter()?;
            for (key, val) in map {
                budget.step()?;
                budget.check_entity(key)?;
                match val {
                    serde_json::Value::Object(_) => {
                        builder.push_heading(level, key, None, None);
                        build_json_internal_structure(val, builder, depth + 1, budget)?;
                    }
                    serde_json::Value::Array(arr) => {
                        builder.push_heading(level, key, None, None);
                        builder.push_list(false);
                        for item in arr {
                            budget.step()?;
                            let text = match item {
                                serde_json::Value::String(s) => {
                                    budget.check_entity(s)?;
                                    s.clone()
                                }
                                other => other.to_string(),
                            };
                            budget.account_text(text.len())?;
                            builder.push_list_item(&text, false, vec![], None, None);
                        }
                        builder.end_list();
                    }
                    serde_json::Value::String(s) => {
                        budget.check_entity(s)?;
                        let rendered = format!("{}: {}", key, s);
                        budget.account_text(rendered.len())?;
                        builder.push_paragraph(&rendered, vec![], None, None);
                    }
                    other => {
                        let rendered = format!("{}: {}", key, other);
                        budget.account_text(rendered.len())?;
                        builder.push_paragraph(&rendered, vec![], None, None);
                    }
                }
            }
            budget.leave();
        }
        serde_json::Value::Array(arr) => {
            budget.enter()?;
            builder.push_list(false);
            for item in arr {
                budget.step()?;
                let text = match item {
                    serde_json::Value::String(s) => {
                        budget.check_entity(s)?;
                        s.clone()
                    }
                    other => other.to_string(),
                };
                budget.account_text(text.len())?;
                builder.push_list_item(&text, false, vec![], None, None);
            }
            builder.end_list();
            budget.leave();
        }
        serde_json::Value::String(s) => {
            budget.check_entity(s)?;
            budget.account_text(s.len())?;
            builder.push_paragraph(s, vec![], None, None);
        }
        other => {
            let rendered = other.to_string();
            budget.account_text(rendered.len())?;
            builder.push_paragraph(&rendered, vec![], None, None);
        }
    }
    Ok(())
}

/// Structured data extractor supporting JSON, JSONL/NDJSON, YAML, and TOML.
pub struct StructuredExtractor;

impl Default for StructuredExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl StructuredExtractor {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl Plugin for StructuredExtractor {
    fn name(&self) -> &str {
        "structured-extractor"
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

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl DocumentExtractor for StructuredExtractor {
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
    ) -> Result<InternalDocument> {
        let structured_result = match mime_type {
            "application/json" | "text/json" | "application/csl+json" => {
                crate::extraction::structured::parse_json(content, None)?
            }
            "application/x-ndjson" | "application/jsonl" | "application/x-jsonlines" => {
                crate::extraction::structured::parse_jsonl(content, None)?
            }
            "application/yaml" | "application/x-yaml" | "text/yaml" | "text/x-yaml" => {
                crate::extraction::structured::parse_yaml(content)?
            }
            "application/toml" | "text/toml" => crate::extraction::structured::parse_toml(content)?,
            _ => return Err(crate::KreuzbergError::UnsupportedFormat(mime_type.to_string())),
        };

        let mut additional = AHashMap::new();
        additional.insert(
            Cow::Borrowed("field_count"),
            serde_json::json!(structured_result.text_fields.len()),
        );
        additional.insert(
            Cow::Borrowed("data_format"),
            serde_json::json!(structured_result.format),
        );

        for (key, value) in &structured_result.metadata {
            additional.insert(Cow::Owned(key.clone()), serde_json::json!(value));
        }

        let mut budget = SecurityBudget::from_config(config);
        let mut doc = build_internal_document(&structured_result, mime_type, &mut budget)?;
        doc.mime_type = mime_type.to_string();

        doc.metadata = Metadata {
            additional,
            ..Default::default()
        };

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
        &[
            "application/json",
            "text/json",
            "application/csl+json",
            "application/x-ndjson",
            "application/jsonl",
            "application/x-jsonlines",
            "application/yaml",
            "application/x-yaml",
            "text/yaml",
            "text/x-yaml",
            "application/toml",
            "text/toml",
        ]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_structured_extractor_plugin_interface() {
        let extractor = StructuredExtractor::new();
        assert_eq!(extractor.name(), "structured-extractor");
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[test]
    fn test_structured_extractor_supported_mime_types() {
        let extractor = StructuredExtractor::new();
        let mime_types = extractor.supported_mime_types();
        assert_eq!(mime_types.len(), 12);
        assert!(mime_types.contains(&"application/json"));
        assert!(mime_types.contains(&"application/x-ndjson"));
        assert!(mime_types.contains(&"application/jsonl"));
        assert!(mime_types.contains(&"application/x-jsonlines"));
        assert!(mime_types.contains(&"application/x-yaml"));
        assert!(mime_types.contains(&"application/toml"));
        assert!(mime_types.contains(&"application/csl+json"));
    }
}
