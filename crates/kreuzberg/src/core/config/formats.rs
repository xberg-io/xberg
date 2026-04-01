//! Output format configuration and validation.
//!
//! This module defines the `OutputFormat` enum for controlling how extraction
//! results are formatted (plain text, markdown, HTML, etc.) and provides
//! serialization/deserialization support.

use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Output format for extraction results.
///
/// Controls the format of the `content` field in `ExtractionResult`.
/// When set to `Markdown`, `Djot`, or `Html`, the output will be formatted
/// accordingly. `Plain` returns the raw extracted text.
/// `Structured` returns JSON with full OCR element data including bounding
/// boxes and confidence scores.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    /// Plain text content only (default)
    #[default]
    Plain,
    /// Markdown format
    Markdown,
    /// Djot markup format
    Djot,
    /// HTML format
    Html,
    /// JSON tree format with heading-driven sections.
    Json,
    /// Structured JSON format with full OCR element metadata.
    Structured,
    /// Custom renderer registered via the RendererRegistry.
    /// The string is the renderer name (e.g., "docx", "latex").
    #[serde(untagged)]
    Custom(String),
}

impl OutputFormat {
    /// Get the renderer name for this format.
    /// Returns `None` for formats that don't use the renderer registry
    /// (Plain, Structured, Toon — these are handled differently).
    pub fn renderer_name(&self) -> Option<&str> {
        match self {
            OutputFormat::Plain | OutputFormat::Json | OutputFormat::Structured => None,
            OutputFormat::Markdown => Some("markdown"),
            OutputFormat::Djot => Some("djot"),
            OutputFormat::Html => Some("html"),
            OutputFormat::Custom(name) => Some(name.as_str()),
        }
    }
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Plain => write!(f, "plain"),
            OutputFormat::Markdown => write!(f, "markdown"),
            OutputFormat::Djot => write!(f, "djot"),
            OutputFormat::Html => write!(f, "html"),
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::Structured => write!(f, "structured"),
            OutputFormat::Custom(name) => write!(f, "{}", name),
        }
    }
}

impl FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "plain" | "text" => Ok(OutputFormat::Plain),
            "markdown" | "md" => Ok(OutputFormat::Markdown),
            "djot" => Ok(OutputFormat::Djot),
            "html" => Ok(OutputFormat::Html),
            "json" => Ok(OutputFormat::Json),
            "structured" | "structured-ocr" => Ok(OutputFormat::Structured),
            other => Ok(OutputFormat::Custom(other.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_from_str_plain() {
        assert_eq!("plain".parse::<OutputFormat>().unwrap(), OutputFormat::Plain);
        assert_eq!("PLAIN".parse::<OutputFormat>().unwrap(), OutputFormat::Plain);
        assert_eq!("text".parse::<OutputFormat>().unwrap(), OutputFormat::Plain);
        assert_eq!("TEXT".parse::<OutputFormat>().unwrap(), OutputFormat::Plain);
    }

    #[test]
    fn test_output_format_from_str_markdown() {
        assert_eq!("markdown".parse::<OutputFormat>().unwrap(), OutputFormat::Markdown);
        assert_eq!("MARKDOWN".parse::<OutputFormat>().unwrap(), OutputFormat::Markdown);
        assert_eq!("md".parse::<OutputFormat>().unwrap(), OutputFormat::Markdown);
        assert_eq!("MD".parse::<OutputFormat>().unwrap(), OutputFormat::Markdown);
    }

    #[test]
    fn test_output_format_from_str_djot() {
        assert_eq!("djot".parse::<OutputFormat>().unwrap(), OutputFormat::Djot);
        assert_eq!("DJOT".parse::<OutputFormat>().unwrap(), OutputFormat::Djot);
        assert_eq!("Djot".parse::<OutputFormat>().unwrap(), OutputFormat::Djot);
    }

    #[test]
    fn test_output_format_from_str_html() {
        assert_eq!("html".parse::<OutputFormat>().unwrap(), OutputFormat::Html);
        assert_eq!("HTML".parse::<OutputFormat>().unwrap(), OutputFormat::Html);
        assert_eq!("Html".parse::<OutputFormat>().unwrap(), OutputFormat::Html);
    }

    #[test]
    fn test_output_format_from_str_json() {
        assert_eq!("json".parse::<OutputFormat>().unwrap(), OutputFormat::Json);
        assert_eq!("JSON".parse::<OutputFormat>().unwrap(), OutputFormat::Json);
    }

    #[test]
    fn test_output_format_from_str_structured() {
        assert_eq!("structured".parse::<OutputFormat>().unwrap(), OutputFormat::Structured);
        assert_eq!("STRUCTURED".parse::<OutputFormat>().unwrap(), OutputFormat::Structured);
        assert_eq!(
            "structured-ocr".parse::<OutputFormat>().unwrap(),
            OutputFormat::Structured
        );
        assert_eq!(
            "STRUCTURED-OCR".parse::<OutputFormat>().unwrap(),
            OutputFormat::Structured
        );
    }

    #[test]
    fn test_output_format_from_str_custom() {
        let result = "docx".parse::<OutputFormat>().unwrap();
        assert_eq!(result, OutputFormat::Custom("docx".to_string()));
    }

    #[test]
    fn test_output_format_to_string() {
        assert_eq!(OutputFormat::Plain.to_string(), "plain");
        assert_eq!(OutputFormat::Markdown.to_string(), "markdown");
        assert_eq!(OutputFormat::Djot.to_string(), "djot");
        assert_eq!(OutputFormat::Html.to_string(), "html");
        assert_eq!(OutputFormat::Json.to_string(), "json");
        assert_eq!(OutputFormat::Structured.to_string(), "structured");
        assert_eq!(OutputFormat::Custom("docx".to_string()).to_string(), "docx");
    }

    #[test]
    fn test_output_format_default() {
        let format = OutputFormat::default();
        assert_eq!(format, OutputFormat::Plain);
    }

    #[test]
    fn test_output_format_serde_roundtrip() {
        for format in [
            OutputFormat::Plain,
            OutputFormat::Markdown,
            OutputFormat::Djot,
            OutputFormat::Html,
            OutputFormat::Json,
            OutputFormat::Structured,
        ] {
            let json = serde_json::to_string(&format).unwrap();
            let deserialized: OutputFormat = serde_json::from_str(&json).unwrap();
            assert_eq!(format, deserialized);
        }
    }

    #[test]
    fn test_output_format_serde_values() {
        assert_eq!(serde_json::to_string(&OutputFormat::Plain).unwrap(), "\"plain\"");
        assert_eq!(serde_json::to_string(&OutputFormat::Markdown).unwrap(), "\"markdown\"");
        assert_eq!(serde_json::to_string(&OutputFormat::Djot).unwrap(), "\"djot\"");
        assert_eq!(serde_json::to_string(&OutputFormat::Html).unwrap(), "\"html\"");
        assert_eq!(serde_json::to_string(&OutputFormat::Json).unwrap(), "\"json\"");
        assert_eq!(
            serde_json::to_string(&OutputFormat::Structured).unwrap(),
            "\"structured\""
        );
    }

    #[test]
    fn test_output_format_renderer_name() {
        assert_eq!(OutputFormat::Plain.renderer_name(), None);
        assert_eq!(OutputFormat::Markdown.renderer_name(), Some("markdown"));
        assert_eq!(OutputFormat::Html.renderer_name(), Some("html"));
        assert_eq!(OutputFormat::Djot.renderer_name(), Some("djot"));
        assert_eq!(OutputFormat::Json.renderer_name(), None);
        assert_eq!(OutputFormat::Structured.renderer_name(), None);
        assert_eq!(OutputFormat::Custom("docx".to_string()).renderer_name(), Some("docx"));
    }
}
