//! Structured data extraction using LLM-based schema-guided generation.
//!
//! Uses liter-llm's JSON schema response format to extract structured data
//! from document content. The LLM is constrained to produce output conforming
//! to the caller's JSON schema.

use crate::core::config::llm::StructuredExtractionConfig;
use serde_json::Value;

/// Strip JSON Schema fields that some providers don't support.
///
/// `additionalProperties` is rejected by Gemini and Anthropic but required
/// by OpenAI strict mode. We strip it only for providers known to reject it.
fn sanitize_schema_for_provider(schema: &Value, model: &str) -> Value {
    // OpenAI requires additionalProperties for strict mode — don't strip
    let needs_strip = !model.starts_with("openai/");

    if needs_strip {
        strip_additional_properties(schema)
    } else {
        schema.clone()
    }
}

fn strip_additional_properties(schema: &Value) -> Value {
    match schema {
        Value::Object(map) => {
            let mut cleaned = serde_json::Map::new();
            for (key, value) in map {
                if key == "additionalProperties" {
                    continue;
                }
                cleaned.insert(key.clone(), strip_additional_properties(value));
            }
            Value::Object(cleaned)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(strip_additional_properties).collect()),
        other => other.clone(),
    }
}

/// Extract structured data from document content using an LLM with JSON schema.
///
/// Sends the document content to the configured LLM with a JSON schema constraint,
/// returning structured data that conforms to the schema.
///
/// # Arguments
///
/// * `content` - The extracted document text to send to the LLM.
/// * `config` - Structured extraction configuration including schema and LLM settings.
///
/// # Returns
///
/// A `serde_json::Value` conforming to the provided JSON schema.
///
/// # Errors
///
/// Returns an error if:
/// - The LLM client cannot be created (invalid provider/credentials).
/// - The LLM request fails (network, rate-limit, etc.).
/// - The LLM response cannot be parsed as valid JSON.
pub async fn extract_structured(
    content: &str,
    config: &StructuredExtractionConfig,
) -> crate::Result<(serde_json::Value, Option<crate::types::LlmUsage>)> {
    use liter_llm::LlmClient;

    let client = super::client::create_client(&config.llm)?;

    // Build prompt from custom Jinja2 template or default
    let template = config
        .prompt
        .as_deref()
        .unwrap_or(super::prompts::STRUCTURED_EXTRACTION_TEMPLATE);

    let schema_json = serde_json::to_string_pretty(&config.schema)
        .map_err(|e| crate::KreuzbergError::validation(format!("Failed to serialize schema for prompt: {e}")))?;

    let ctx = minijinja::context! {
        content => content,
        schema => schema_json,
        schema_name => &config.schema_name,
        schema_description => config.schema_description.as_deref().unwrap_or(""),
    };

    let prompt = super::prompts::render_template(template, &ctx)?;

    // Sanitize the schema for cross-provider compatibility.
    // Some providers (Gemini, Anthropic) reject fields like `additionalProperties`
    // that others (OpenAI) require for strict mode. Strip unsupported fields so
    // the same schema works across all providers.
    let sanitized_schema = sanitize_schema_for_provider(&config.schema, &config.llm.model);

    // Build chat request with JSON schema response format.
    // Use field assignment because `stream` is pub(crate) in liter-llm; struct-init
    // syntax with `..Default::default()` won't compile across the crate boundary.
    #[allow(clippy::field_reassign_with_default)]
    let request = {
        let mut req = liter_llm::ChatCompletionRequest::default();
        req.model = config.llm.model.clone();
        req.messages = vec![liter_llm::Message::User(liter_llm::UserMessage {
            content: liter_llm::UserContent::Text(prompt),
            name: None,
        })];
        req.temperature = config.llm.temperature;
        req.max_tokens = config.llm.max_tokens;
        req.response_format = Some(liter_llm::ResponseFormat::JsonSchema {
            json_schema: liter_llm::JsonSchemaFormat {
                name: config.schema_name.clone(),
                description: config.schema_description.clone(),
                schema: sanitized_schema,
                strict: Some(config.strict),
            },
        });
        req
    };

    let response = client
        .chat(request)
        .await
        .map_err(|e| crate::KreuzbergError::parsing(format!("LLM structured extraction request failed: {e}")))?;

    let usage = super::usage::extract_usage_from_chat(&response, "structured_extraction");

    // Extract text content from the first choice.
    let text = response
        .choices
        .first()
        .and_then(|c| c.message.content.as_deref())
        .ok_or_else(|| {
            crate::KreuzbergError::parsing(format!(
                "LLM structured extraction returned no content (model={}, {} choices)",
                config.llm.model,
                response.choices.len()
            ))
        })?;

    // Parse the response as JSON. Some providers may wrap JSON in markdown
    // code fences — strip them if present.
    let cleaned = text
        .trim()
        .strip_prefix("```json")
        .or_else(|| text.trim().strip_prefix("```"))
        .and_then(|s| s.strip_suffix("```"))
        .map(|s| s.trim())
        .unwrap_or(text.trim());

    let value = serde_json::from_str(cleaned).map_err(|e| {
        crate::KreuzbergError::parsing(format!(
            "LLM structured extraction returned invalid JSON (model={}): {e}\nRaw response: {}",
            config.llm.model,
            &text[..text.floor_char_boundary(text.len().min(200))]
        ))
    })?;

    Ok((value, usage))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_sanitize_schema_strips_for_non_openai() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "nested": {
                    "type": "object",
                    "properties": { "x": { "type": "integer" } },
                    "additionalProperties": false
                }
            },
            "required": ["name"],
            "additionalProperties": false
        });

        let sanitized = sanitize_schema_for_provider(&schema, "gemini/gemini-2.5-flash");
        assert!(sanitized.get("additionalProperties").is_none());
        assert!(sanitized["properties"]["nested"].get("additionalProperties").is_none());
        assert_eq!(sanitized["type"], "object");
        assert_eq!(sanitized["required"], json!(["name"]));
    }

    #[test]
    fn test_sanitize_schema_preserves_for_openai() {
        let schema = json!({
            "type": "object",
            "properties": { "a": { "type": "string" } },
            "required": ["a"],
            "additionalProperties": false
        });

        let sanitized = sanitize_schema_for_provider(&schema, "openai/gpt-4o");
        assert_eq!(sanitized, schema);
    }
}
