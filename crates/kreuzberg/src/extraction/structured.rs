//! Structured data extraction (JSON, JSONL, YAML, TOML).
//!
//! Parses structured data formats and extracts text content while preserving
//! schema information and metadata.
//!
//! # Supported Formats
//!
//! - **JSON**: Using `serde_json` with schema extraction
//! - **JSONL/NDJSON**: Line-delimited JSON, parsed per-line via `serde_json`
//! - **YAML**: Using `serde_yaml`
//! - **TOML**: Using `toml`
//!
//! # Features
//!
//! - **Text extraction**: Identifies text fields by common keywords (title, description, etc.)
//! - **Schema extraction**: Optional JSON schema generation
//! - **Depth limiting**: Prevents stack overflow on deeply nested data
//! - **Flattening**: Converts nested structures to flat text representation
//!
//! # Example
//!
//! ```rust
//! use kreuzberg::extraction::structured::parse_json;
//!
//! # fn example() -> kreuzberg::Result<()> {
//! let json = br#"{"title": "Example", "description": "Test document"}"#;
//! let result = parse_json(json, None)?;
//!
//! assert!(result.content.contains("Example"));
//! assert!(result.content.contains("Test document"));
//! # Ok(())
//! # }
//! ```
use crate::error::{KreuzbergError, Result};
use crate::text::utf8_validation;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredDataResult {
    pub content: String,
    pub format: Cow<'static, str>,
    pub metadata: HashMap<String, String>,
    pub text_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonExtractionConfig {
    pub extract_schema: bool,
    pub max_depth: usize,
    pub array_item_limit: usize,
    pub include_type_info: bool,
    pub flatten_nested_objects: bool,
    pub custom_text_field_patterns: Vec<String>,
}

impl Default for JsonExtractionConfig {
    fn default() -> Self {
        Self {
            extract_schema: false,
            max_depth: 20,
            array_item_limit: 500,
            include_type_info: false,
            flatten_nested_objects: true,
            custom_text_field_patterns: Vec::new(),
        }
    }
}

const TEXT_FIELD_KEYWORDS: &[&str] = &[
    "title",
    "name",
    "subject",
    "description",
    "content",
    "body",
    "text",
    "message",
    "payload",
    "data",
    "properties",
    "metadata",
    "value",
    "result",
    "summary",
    "label",
    "comment",
    "note",
    "info",
    "spec",
    "status",
    "kind",
    "type",
    "key",
    "id",
    "url",
    "path",
    "author",
    "email",
    "address",
    "version",
    "tag",
    "category",
    "caption",
    "heading",
    "abstract",
    "readme",
    "changelog",
    "license",
];

pub fn parse_json(data: &[u8], config: Option<JsonExtractionConfig>) -> Result<StructuredDataResult> {
    let config = config.unwrap_or_default();

    let value: serde_json::Value =
        serde_json::from_slice(data).map_err(|e| KreuzbergError::parsing(format!("Failed to parse JSON: {}", e)))?;

    let mut metadata = HashMap::new();
    let mut text_fields = Vec::new();

    if config.extract_schema {
        let mut path_buf = String::new();
        if let Ok(schema_json) = serde_json::to_string(&extract_json_schema(&value, &mut path_buf, 0, &config)) {
            metadata.insert("json_schema".to_string(), schema_json);
        }
    }

    // Still extract text fields for metadata population
    let mut path_buf = String::new();
    let _ = extract_from_json_value(&value, &mut path_buf, &config, &mut metadata, &mut text_fields);
    // Output pretty-printed JSON to preserve structure (matches ground truth format)
    let content = serde_json::to_string_pretty(&value).unwrap_or_else(|_| String::from_utf8_lossy(data).to_string());

    Ok(StructuredDataResult {
        content,
        format: Cow::Borrowed("json"),
        metadata,
        text_fields,
    })
}

fn extract_json_schema(
    value: &serde_json::Value,
    path: &mut String,
    depth: usize,
    config: &JsonExtractionConfig,
) -> serde_json::Value {
    if depth >= config.max_depth {
        return serde_json::json!({"max_depth_reached": true});
    }

    match value {
        serde_json::Value::Null => serde_json::json!({"type": "null"}),
        serde_json::Value::Bool(_) => serde_json::json!({"type": "bool"}),
        serde_json::Value::Number(_) => serde_json::json!({"type": "number"}),
        serde_json::Value::String(_) => serde_json::json!({"type": "string"}),
        serde_json::Value::Array(arr) => {
            if arr.is_empty() {
                serde_json::json!({"type": "array", "length": 0})
            } else if arr.len() <= config.array_item_limit {
                // Push "[0]" suffix and recurse, then restore.
                let base_len = path.len();
                path.push_str("[0]");
                let items = extract_json_schema(&arr[0], path, depth + 1, config);
                path.truncate(base_len);
                serde_json::json!({
                    "type": "array",
                    "length": arr.len(),
                    "items": items
                })
            } else {
                serde_json::json!({
                    "type": "array",
                    "length": arr.len(),
                    "truncated": true,
                    "items": {"type": "truncated"}
                })
            }
        }
        serde_json::Value::Object(obj) => {
            let mut properties = serde_json::Map::new();
            for (key, val) in obj {
                // Extend the path buffer with ".key" (or just "key" at root).
                let base_len = path.len();
                if !path.is_empty() {
                    path.push('.');
                }
                path.push_str(key);
                let schema = extract_json_schema(val, path, depth + 1, config);
                path.truncate(base_len);
                properties.insert(key.clone(), schema);
            }
            serde_json::json!({"type": "object", "properties": properties})
        }
    }
}

fn extract_from_json_value(
    value: &serde_json::Value,
    path: &mut String,
    config: &JsonExtractionConfig,
    metadata: &mut HashMap<String, String>,
    text_fields: &mut Vec<String>,
) -> Vec<String> {
    match value {
        serde_json::Value::Object(obj) => {
            let mut text_parts = Vec::new();
            for (key, val) in obj {
                // Append ".key" (or just "key" at root) to the shared buffer.
                let base_len = path.len();
                if !path.is_empty() {
                    path.push('.');
                }
                path.push_str(key);
                text_parts.extend(extract_from_json_value(val, path, config, metadata, text_fields));
                path.truncate(base_len);
            }
            text_parts
        }
        serde_json::Value::Array(arr) => {
            let mut text_parts = Vec::new();
            for (i, item) in arr.iter().enumerate() {
                // Append "[i]" (or "item_i" at root) to the shared buffer.
                let base_len = path.len();
                if path.is_empty() {
                    path.push_str("item_");
                    path.push_str(&i.to_string());
                } else {
                    path.push('[');
                    path.push_str(&i.to_string());
                    path.push(']');
                }
                text_parts.extend(extract_from_json_value(item, path, config, metadata, text_fields));
                path.truncate(base_len);
            }
            text_parts
        }
        serde_json::Value::String(s) => {
            if !s.trim().is_empty() {
                let formatted = if config.include_type_info {
                    format!("{} (string): {}", path, s)
                } else {
                    format!("{}: {}", path, s)
                };

                if is_text_field(path, &config.custom_text_field_patterns) {
                    metadata.insert(path.clone(), s.clone());
                    text_fields.push(path.clone());
                }

                vec![formatted]
            } else {
                Vec::new()
            }
        }
        serde_json::Value::Number(n) => {
            let formatted = if config.include_type_info {
                format!("{} (number): {}", path, n)
            } else {
                format!("{}: {}", path, n)
            };
            vec![formatted]
        }
        serde_json::Value::Bool(b) => {
            let formatted = if config.include_type_info {
                format!("{} (bool): {}", path, b)
            } else {
                format!("{}: {}", path, b)
            };
            vec![formatted]
        }
        serde_json::Value::Null => Vec::new(),
    }
}

fn is_text_field(key: &str, custom_patterns: &[String]) -> bool {
    // Extract leaf field name (last dot-separated segment)
    let leaf = key.rsplit('.').next().unwrap_or(key);
    // Strip array index suffix like "[0]"
    let leaf = if let Some(bracket_pos) = leaf.find('[') {
        &leaf[..bracket_pos]
    } else {
        leaf
    };
    let leaf_lower = leaf.to_lowercase();

    for keyword in TEXT_FIELD_KEYWORDS {
        if leaf_lower == *keyword {
            return true;
        }
    }

    for pattern in custom_patterns {
        if leaf_lower == pattern.to_lowercase() {
            return true;
        }
    }

    false
}

/// Parse JSONL (newline-delimited JSON) into a structured data result.
///
/// Each non-empty line is parsed as an independent JSON value. Blank lines
/// and whitespace-only lines are skipped. The output is a pretty-printed
/// JSON array of all parsed objects.
///
/// # Errors
///
/// Returns an error if any line contains invalid JSON (with 1-based line number)
/// or if the input is not valid UTF-8.
///
/// # Example
///
/// ```rust
/// use kreuzberg::extraction::structured::parse_jsonl;
///
/// # fn example() -> kreuzberg::Result<()> {
/// let jsonl = b"{\"name\": \"Alice\"}\n{\"name\": \"Bob\"}";
/// let result = parse_jsonl(jsonl, None)?;
/// assert!(result.content.contains("Alice"));
/// assert!(result.content.contains("Bob"));
/// # Ok(())
/// # }
/// ```
pub fn parse_jsonl(data: &[u8], config: Option<JsonExtractionConfig>) -> Result<StructuredDataResult> {
    let text = utf8_validation::from_utf8(data)
        .map_err(|e| KreuzbergError::parsing(format!("Invalid UTF-8 in JSONL: {}", e)))?;

    let config = config.unwrap_or_default();
    let line_count_estimate = memchr::memchr_iter(b'\n', data).count().saturating_add(1);
    let mut all_objects = Vec::with_capacity(line_count_estimate);
    let mut metadata = HashMap::new();
    let mut text_fields = Vec::new();
    let mut path_buf = String::new();

    for (line_num, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(trimmed)
            .map_err(|e| KreuzbergError::parsing(format!("Failed to parse JSONL line {}: {}", line_num + 1, e)))?;

        path_buf.clear();
        extract_from_json_value(&value, &mut path_buf, &config, &mut metadata, &mut text_fields);
        all_objects.push(value);
    }

    // Infallible: serde_json::to_string_pretty cannot fail on a Value::Array
    // of already-parsed Value objects.
    let content = serde_json::to_string_pretty(&serde_json::Value::Array(all_objects))
        .expect("serializing Vec<serde_json::Value> to JSON cannot fail");

    Ok(StructuredDataResult {
        content,
        format: Cow::Borrowed("jsonl"),
        metadata,
        text_fields,
    })
}

pub fn parse_yaml(data: &[u8]) -> Result<StructuredDataResult> {
    let yaml_str = utf8_validation::from_utf8(data)
        .map_err(|e| KreuzbergError::parsing(format!("Invalid UTF-8 in YAML: {}", e)))?;

    let value: serde_json::Value = serde_yaml_ng::from_str(yaml_str)
        .map_err(|e| KreuzbergError::parsing(format!("Failed to parse YAML: {}", e)))?;

    let mut metadata = HashMap::new();
    let mut text_fields = Vec::new();

    // Still extract for metadata population
    let mut path_buf = String::new();
    let _ = extract_from_value(&value, &mut path_buf, &mut metadata, &mut text_fields);
    // Output original YAML content to preserve structure (matches ground truth format)
    let content = yaml_str.to_string();

    Ok(StructuredDataResult {
        content,
        format: Cow::Borrowed("yaml"),
        metadata,
        text_fields,
    })
}

fn extract_from_value(
    value: &serde_json::Value,
    path: &mut String,
    metadata: &mut HashMap<String, String>,
    text_fields: &mut Vec<String>,
) -> Vec<String> {
    match value {
        serde_json::Value::Null => Vec::new(),
        serde_json::Value::Bool(b) => vec![format!("{}: {}", path, b)],
        serde_json::Value::Number(n) => vec![format!("{}: {}", path, n)],
        serde_json::Value::String(s) => {
            if !s.trim().is_empty() {
                let formatted = format!("{}: {}", path, s);

                if is_text_field(path, &[]) {
                    metadata.insert(path.clone(), s.to_string());
                    text_fields.push(path.clone());
                }

                vec![formatted]
            } else {
                Vec::new()
            }
        }
        serde_json::Value::Array(arr) => {
            let mut text_parts = Vec::new();
            for (i, item) in arr.iter().enumerate() {
                let base_len = path.len();
                if path.is_empty() {
                    path.push_str("item_");
                    path.push_str(&i.to_string());
                } else {
                    path.push('[');
                    path.push_str(&i.to_string());
                    path.push(']');
                }
                text_parts.extend(extract_from_value(item, path, metadata, text_fields));
                path.truncate(base_len);
            }
            text_parts
        }
        serde_json::Value::Object(obj) => {
            let mut text_parts = Vec::new();
            for (key, val) in obj {
                let base_len = path.len();
                if !path.is_empty() {
                    path.push('.');
                }
                path.push_str(key);
                text_parts.extend(extract_from_value(val, path, metadata, text_fields));
                path.truncate(base_len);
            }
            text_parts
        }
    }
}

pub fn parse_toml(data: &[u8]) -> Result<StructuredDataResult> {
    let toml_str = utf8_validation::from_utf8(data)
        .map_err(|e| KreuzbergError::parsing(format!("Invalid UTF-8 in TOML: {}", e)))?;

    let value: toml::Value =
        toml::from_str(toml_str).map_err(|e| KreuzbergError::parsing(format!("Failed to parse TOML: {}", e)))?;

    let mut metadata = HashMap::new();
    let mut text_fields = Vec::new();

    // Still extract for metadata population
    let mut path_buf = String::new();
    let _ = extract_from_toml_value(&value, &mut path_buf, &mut metadata, &mut text_fields);
    // Output original TOML content to preserve structure (matches ground truth format)
    let content = toml_str.to_string();

    Ok(StructuredDataResult {
        content,
        format: Cow::Borrowed("toml"),
        metadata,
        text_fields,
    })
}

fn extract_from_toml_value(
    value: &toml::Value,
    path: &mut String,
    metadata: &mut HashMap<String, String>,
    text_fields: &mut Vec<String>,
) -> Vec<String> {
    match value {
        toml::Value::String(s) => {
            if !s.trim().is_empty() {
                let formatted = format!("{}: {}", path, s);

                if is_text_field(path, &[]) {
                    metadata.insert(path.clone(), s.clone());
                    text_fields.push(path.clone());
                }

                vec![formatted]
            } else {
                Vec::new()
            }
        }
        toml::Value::Integer(i) => vec![format!("{}: {}", path, i)],
        toml::Value::Float(f) => vec![format!("{}: {}", path, f)],
        toml::Value::Boolean(b) => vec![format!("{}: {}", path, b)],
        toml::Value::Datetime(d) => vec![format!("{}: {}", path, d)],
        toml::Value::Array(arr) => {
            let mut text_parts = Vec::new();
            for (i, item) in arr.iter().enumerate() {
                let base_len = path.len();
                if path.is_empty() {
                    path.push_str("item_");
                    path.push_str(&i.to_string());
                } else {
                    path.push('[');
                    path.push_str(&i.to_string());
                    path.push(']');
                }
                text_parts.extend(extract_from_toml_value(item, path, metadata, text_fields));
                path.truncate(base_len);
            }
            text_parts
        }
        toml::Value::Table(table) => {
            let mut text_parts = Vec::new();
            for (key, val) in table {
                let base_len = path.len();
                if !path.is_empty() {
                    path.push('.');
                }
                path.push_str(key);
                text_parts.extend(extract_from_toml_value(val, path, metadata, text_fields));
                path.truncate(base_len);
            }
            text_parts
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_simple() {
        let json = r#"{"name": "John", "age": 30}"#;
        let result = parse_json(json.as_bytes(), None).unwrap();
        assert_eq!(result.format, "json");
        assert!(result.content.contains("\"name\": \"John\""));
        assert!(result.content.contains("\"age\": 30"));
    }

    #[test]
    fn test_parse_json_nested() {
        let json = r#"{"user": {"name": "Alice", "email": "alice@example.com"}}"#;
        let result = parse_json(json.as_bytes(), None).unwrap();
        assert!(result.content.contains("\"name\": \"Alice\""));
        assert!(result.content.contains("\"email\": \"alice@example.com\""));
    }

    #[test]
    fn test_parse_json_array() {
        let json = r#"{"items": ["apple", "banana", "cherry"]}"#;
        let result = parse_json(json.as_bytes(), None).unwrap();
        assert!(result.content.contains("\"apple\""));
        assert!(result.content.contains("\"banana\""));
        assert!(result.content.contains("\"cherry\""));
    }

    #[test]
    fn test_parse_json_with_schema() {
        let json = r#"{"name": "Test", "count": 42}"#;
        let config = JsonExtractionConfig {
            extract_schema: true,
            ..Default::default()
        };
        let result = parse_json(json.as_bytes(), Some(config)).unwrap();
        assert!(result.metadata.contains_key("json_schema"));
    }

    #[test]
    fn test_parse_yaml_simple() {
        let yaml = "name: John\nage: 30";
        let result = parse_yaml(yaml.as_bytes()).unwrap();
        assert_eq!(result.format, "yaml");
        assert!(result.content.contains("name: John"));
        assert!(result.content.contains("age: 30"));
    }

    #[test]
    fn test_parse_yaml_nested() {
        let yaml = "user:\n  name: Alice\n  email: alice@example.com";
        let result = parse_yaml(yaml.as_bytes()).unwrap();
        assert!(result.content.contains("name: Alice"));
        assert!(result.content.contains("email: alice@example.com"));
    }

    #[test]
    fn test_parse_toml_simple() {
        let toml = "name = \"John\"\nage = 30";
        let result = parse_toml(toml.as_bytes()).unwrap();
        assert_eq!(result.format, "toml");
        assert!(result.content.contains("name = \"John\""));
        assert!(result.content.contains("age = 30"));
    }

    #[test]
    fn test_parse_toml_table() {
        let toml = "[user]\nname = \"Alice\"\nemail = \"alice@example.com\"";
        let result = parse_toml(toml.as_bytes()).unwrap();
        assert!(result.content.contains("name = \"Alice\""));
        assert!(result.content.contains("email = \"alice@example.com\""));
    }

    #[test]
    fn test_text_field_detection() {
        assert!(is_text_field("title", &[]));
        assert!(is_text_field("user.name", &[]));
        assert!(is_text_field("description", &[]));
        assert!(is_text_field("id", &[]));
        assert!(is_text_field("summary", &[]));
        assert!(is_text_field("metadata.label", &[]));
        assert!(!is_text_field("count", &[]));
        assert!(!is_text_field("offset", &[]));
        // Exact match means "width" no longer matches just because it contains "id" substring
        assert!(!is_text_field("width", &[]));
        assert!(!is_text_field("valid", &[]));
    }

    #[test]
    fn test_parse_jsonl_simple() {
        let jsonl = "{\"name\": \"Alice\"}\n{\"name\": \"Bob\"}";
        let result = parse_jsonl(jsonl.as_bytes(), None).unwrap();
        assert!(result.content.contains("\"name\": \"Alice\""));
        assert!(result.content.contains("\"name\": \"Bob\""));
    }

    #[test]
    fn test_parse_jsonl_format_field() {
        let jsonl = "{\"a\": 1}";
        let result = parse_jsonl(jsonl.as_bytes(), None).unwrap();
        assert_eq!(result.format, "jsonl");
    }

    #[test]
    fn test_parse_jsonl_empty_lines_skipped() {
        let jsonl = "{\"a\": 1}\n\n\n{\"b\": 2}\n";
        let result = parse_jsonl(jsonl.as_bytes(), None).unwrap();
        assert!(result.content.contains("\"a\": 1"));
        assert!(result.content.contains("\"b\": 2"));
    }

    #[test]
    fn test_parse_jsonl_single_line() {
        let jsonl = "{\"key\": \"value\"}";
        let result = parse_jsonl(jsonl.as_bytes(), None).unwrap();
        assert!(result.content.contains("\"key\": \"value\""));
    }

    #[test]
    fn test_parse_jsonl_invalid_line() {
        let jsonl = "{\"a\": 1}\nnot json\n{\"b\": 2}";
        let result = parse_jsonl(jsonl.as_bytes(), None);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("line 2"), "Error should reference line 2, got: {}", err);
    }

    #[test]
    fn test_parse_jsonl_empty_input() {
        let result = parse_jsonl(b"", None).unwrap();
        assert_eq!(result.content, "[]");
    }

    #[test]
    fn test_parse_jsonl_metadata_extraction() {
        let jsonl = "{\"name\": \"Alice\", \"title\": \"Engineer\"}";
        let result = parse_jsonl(jsonl.as_bytes(), None).unwrap();
        assert!(result.metadata.contains_key("name"));
        assert!(result.metadata.contains_key("title"));
        assert!(result.text_fields.contains(&"name".to_string()));
        assert!(result.text_fields.contains(&"title".to_string()));
    }

    #[test]
    fn test_parse_jsonl_bare_scalars() {
        let jsonl = "42\n\"hello\"\ntrue\nnull";
        let result = parse_jsonl(jsonl.as_bytes(), None).unwrap();
        assert!(result.content.contains("42"));
        assert!(result.content.contains("\"hello\""));
        assert!(result.content.contains("true"));
        assert!(result.content.contains("null"));
    }

    #[test]
    fn test_parse_jsonl_only_blank_lines() {
        let jsonl = "\n\n\n";
        let result = parse_jsonl(jsonl.as_bytes(), None).unwrap();
        assert_eq!(result.content, "[]");
    }

    #[test]
    fn test_parse_jsonl_windows_line_endings() {
        let jsonl = "{\"a\": 1}\r\n{\"b\": 2}\r\n";
        let result = parse_jsonl(jsonl.as_bytes(), None).unwrap();
        assert!(result.content.contains("\"a\": 1"));
        assert!(result.content.contains("\"b\": 2"));
    }

    #[test]
    fn test_parse_jsonl_metadata_last_writer_wins() {
        // When multiple lines have the same key, last value wins in metadata.
        // This matches JSON array behavior where duplicate paths overwrite.
        let jsonl = "{\"name\": \"Alice\"}\n{\"name\": \"Bob\"}";
        let result = parse_jsonl(jsonl.as_bytes(), None).unwrap();
        assert_eq!(result.metadata.get("name").unwrap(), "Bob");
    }

    #[test]
    fn test_parse_jsonl_unicode_content() {
        let jsonl = "{\"name\": \"café\", \"emoji\": \"😀\"}";
        let result = parse_jsonl(jsonl.as_bytes(), None).unwrap();
        assert!(result.content.contains("café"));
        assert!(result.content.contains("😀"));
    }

    #[test]
    fn test_parse_jsonl_deeply_nested() {
        let jsonl = "{\"a\":{\"b\":{\"c\":{\"d\":\"deep\"}}}}";
        let result = parse_jsonl(jsonl.as_bytes(), None).unwrap();
        assert!(result.content.contains("deep"));
    }

    #[test]
    fn test_parse_jsonl_whitespace_only_lines() {
        let jsonl = "{\"a\": 1}\n  \t  \n{\"b\": 2}";
        let result = parse_jsonl(jsonl.as_bytes(), None).unwrap();
        assert!(result.content.contains("\"a\": 1"));
        assert!(result.content.contains("\"b\": 2"));
    }

    #[test]
    fn test_parse_jsonl_array_values_per_line() {
        let jsonl = "[1,2,3]\n[4,5,6]";
        let result = parse_jsonl(jsonl.as_bytes(), None).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result.content).unwrap();
        let arr = parsed.as_array().expect("top-level should be an array");
        assert_eq!(arr.len(), 2, "should contain two inner arrays");
        assert_eq!(arr[0], serde_json::json!([1, 2, 3]));
        assert_eq!(arr[1], serde_json::json!([4, 5, 6]));
    }

    #[test]
    fn test_parse_jsonl_empty_objects() {
        let jsonl = "{}\n{}";
        let result = parse_jsonl(jsonl.as_bytes(), None).unwrap();
        assert_eq!(result.format, "jsonl");
        // Two empty objects produce a valid array
        let parsed: serde_json::Value = serde_json::from_str(&result.content).unwrap();
        assert_eq!(parsed.as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_parse_jsonl_invalid_utf8() {
        let data: &[u8] = &[0xFF, 0xFE, 0x7B, 0x7D]; // invalid UTF-8 + "{}"
        let result = parse_jsonl(data, None);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("UTF-8"), "Error should mention UTF-8, got: {}", err);
    }

    #[test]
    fn test_parse_jsonl_large_line_count() {
        let lines: Vec<String> = (0..1000).map(|i| format!("{{\"id\": {}}}", i)).collect();
        let jsonl = lines.join("\n");
        let result = parse_jsonl(jsonl.as_bytes(), None).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result.content).unwrap();
        assert_eq!(parsed.as_array().unwrap().len(), 1000);
    }

    #[test]
    fn test_parse_json_invalid() {
        let json = "not valid json {";
        let result = parse_json(json.as_bytes(), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_yaml_invalid() {
        let yaml = "invalid: [unclosed";
        let result = parse_yaml(yaml.as_bytes());
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_toml_invalid() {
        let toml = "invalid = [unclosed";
        let result = parse_toml(toml.as_bytes());
        assert!(result.is_err());
    }
}
