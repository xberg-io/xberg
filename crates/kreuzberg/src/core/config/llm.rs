//! LLM configuration types for liter-llm integration.
//!
//! These types are always available (not feature-gated) since they are
//! pure configuration data with no runtime dependency on liter-llm.

use serde::{Deserialize, Serialize};

/// Configuration for an LLM provider/model via liter-llm.
///
/// Each feature (VLM OCR, VLM embeddings, structured extraction) carries
/// its own `LlmConfig`, allowing different providers per feature.
///
/// # Example
///
/// ```toml
/// [structured_extraction.llm]
/// model = "openai/gpt-4o"
/// api_key = "sk-..."  # or use KREUZBERG_LLM_API_KEY env var
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Provider/model string using liter-llm routing format.
    ///
    /// Examples: `"openai/gpt-4o"`, `"anthropic/claude-sonnet-4-20250514"`,
    /// `"groq/llama-3.1-70b-versatile"`.
    pub model: String,

    /// API key for the provider. When `None`, liter-llm falls back to
    /// the provider's standard environment variable (e.g., `OPENAI_API_KEY`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,

    /// Custom base URL override for the provider endpoint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,

    /// Request timeout in seconds (default: 60).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<u64>,

    /// Maximum retry attempts (default: 3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_retries: Option<u32>,

    /// Sampling temperature for generation tasks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,

    /// Maximum tokens to generate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u64>,
}

/// Configuration for LLM-based structured data extraction.
///
/// Sends extracted document content to a VLM with a JSON schema,
/// returning structured data that conforms to the schema.
///
/// # Example
///
/// ```toml
/// [structured_extraction]
/// schema_name = "invoice_data"
/// strict = true
///
/// [structured_extraction.schema]
/// type = "object"
/// properties.vendor = { type = "string" }
/// properties.total = { type = "number" }
/// required = ["vendor", "total"]
///
/// [structured_extraction.llm]
/// model = "openai/gpt-4o"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredExtractionConfig {
    /// JSON Schema defining the desired output structure.
    pub schema: serde_json::Value,

    /// Schema name passed to the LLM's structured output mode.
    #[serde(default = "default_schema_name")]
    pub schema_name: String,

    /// Optional schema description for the LLM.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_description: Option<String>,

    /// Enable strict mode — output must exactly match the schema.
    #[serde(default)]
    pub strict: bool,

    /// Custom Jinja2 extraction prompt template. When `None`, a default template is used.
    ///
    /// Available template variables:
    /// - `{{ content }}` — The extracted document text.
    /// - `{{ schema }}` — The JSON schema as a formatted string.
    /// - `{{ schema_name }}` — The schema name.
    /// - `{{ schema_description }}` — The schema description (may be empty).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,

    /// LLM configuration for the extraction.
    pub llm: LlmConfig,
}

fn default_schema_name() -> String {
    "extraction".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression test for https://github.com/kreuzberg-dev/kreuzberg/issues/716
    ///
    /// `LlmConfig` must implement `Default` so callers can use the struct-update
    /// syntax documented in the VLM OCR guide:
    ///
    /// ```rust
    /// use kreuzberg::core::config::LlmConfig;
    /// let cfg = LlmConfig {
    ///     model: "openai/gpt-4o-mini".to_string(),
    ///     ..Default::default()
    /// };
    /// ```
    #[test]
    fn test_llm_config_default_trait_is_satisfied() {
        let cfg = LlmConfig::default();
        assert!(cfg.model.is_empty(), "default model should be empty string");
        assert!(cfg.api_key.is_none());
        assert!(cfg.base_url.is_none());
        assert!(cfg.timeout_secs.is_none());
        assert!(cfg.max_retries.is_none());
        assert!(cfg.temperature.is_none());
        assert!(cfg.max_tokens.is_none());
    }

    /// Verify the struct-update pattern from the issue compiles and produces
    /// only the explicitly set field.
    #[test]
    fn test_llm_config_struct_update_syntax() {
        let cfg = LlmConfig {
            model: "openai/gpt-4o-mini".to_string(),
            ..Default::default()
        };
        assert_eq!(cfg.model, "openai/gpt-4o-mini");
        assert!(cfg.api_key.is_none());
        assert!(cfg.base_url.is_none());
        assert!(cfg.timeout_secs.is_none());
        assert!(cfg.max_retries.is_none());
        assert!(cfg.temperature.is_none());
        assert!(cfg.max_tokens.is_none());
    }
}
