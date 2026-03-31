use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Top-level parity manifest deserialized from `parity-manifest.json`.
/// Describes types, fields, and feature profiles that language bindings must match.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParityManifest {
    pub version: u32,
    pub types: BTreeMap<String, TypeDef>,
    pub feature_profiles: BTreeMap<String, Vec<String>>,
}

/// A type definition from the Rust core that bindings must replicate.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum TypeDef {
    #[serde(rename = "struct")]
    Struct { fields: BTreeMap<String, FieldDef> },
    #[serde(rename = "tagged_enum")]
    TaggedEnum {
        tag_field: String,
        variants: BTreeMap<String, VariantDef>,
    },
    #[serde(rename = "enum")]
    SimpleEnum { values: Vec<String> },
}

/// A single field within a struct or variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDef {
    /// JSON schema type: "string", "number", "boolean", "array", "object"
    pub json_type: String,
    /// Whether the field is required (non-optional) in the Rust source.
    pub required: bool,
    /// If set, this field only exists when the named feature is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feature_gate: Option<String>,
}

/// A variant within a tagged enum.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantDef {
    #[serde(default)]
    pub fields: BTreeMap<String, FieldDef>,
}

/// Load and deserialize a parity manifest from disk.
pub fn load_manifest(path: &camino::Utf8Path) -> anyhow::Result<ParityManifest> {
    let content = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&content)?)
}

/// Filter fields for a given feature profile, removing fields whose `feature_gate`
/// is not satisfied by the profile's enabled features.
pub fn filter_fields_for_profile(
    fields: &BTreeMap<String, FieldDef>,
    enabled_features: &[String],
) -> BTreeMap<String, FieldDef> {
    fields
        .iter()
        .filter(|(_, field)| match &field.feature_gate {
            None => true,
            Some(gate) => enabled_features.iter().any(|f| f == gate),
        })
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

/// Convert a `snake_case` field name to `camelCase` (for JS/TS/Java bindings).
pub fn to_camel_case(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut capitalize_next = false;
    for ch in s.chars() {
        if ch == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.extend(ch.to_uppercase());
            capitalize_next = false;
        } else {
            result.push(ch);
        }
    }
    result
}

/// Convert a `PascalCase` type name to `snake_case` (for test function names).
pub fn to_snake_case(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 4);
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(ch.to_ascii_lowercase());
    }
    result
}

/// Convert a `snake_case` field name to `PascalCase` (for C#/Go bindings).
pub fn to_pascal_case(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut capitalize_next = true;
    for ch in s.chars() {
        if ch == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.extend(ch.to_uppercase());
            capitalize_next = false;
        } else {
            result.push(ch);
        }
    }
    result
}

/// Return the feature profile name that applies to a given language target.
pub fn profile_for_language(lang: &str) -> &'static str {
    match lang {
        "wasm-deno" | "wasm-workers" => "wasm",
        "c" => "ffi",
        _ => "full",
    }
}

/// Get all fields for a struct type, filtered by the language's feature profile.
///
/// Returns `None` if `type_name` is not present in the manifest or is not a struct.
pub fn fields_for_type_and_lang(
    manifest: &ParityManifest,
    type_name: &str,
    lang: &str,
) -> Option<BTreeMap<String, FieldDef>> {
    let type_def = manifest.types.get(type_name)?;
    let profile_name = profile_for_language(lang);
    let enabled = manifest.feature_profiles.get(profile_name)?;
    match type_def {
        TypeDef::Struct { fields } => Some(filter_fields_for_profile(fields, enabled)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("FooBar"), "foo_bar");
        assert_eq!(to_snake_case("Hello"), "hello");
        assert_eq!(to_snake_case("ExtractionResult"), "extraction_result");
        assert_eq!(to_snake_case(""), "");
    }

    #[test]
    fn test_to_camel_case() {
        assert_eq!(to_camel_case("foo_bar"), "fooBar");
        assert_eq!(to_camel_case("hello"), "hello");
        assert_eq!(to_camel_case("one_two_three"), "oneTwoThree");
        assert_eq!(to_camel_case(""), "");
    }

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("foo_bar"), "FooBar");
        assert_eq!(to_pascal_case("hello"), "Hello");
        assert_eq!(to_pascal_case("one_two_three"), "OneTwoThree");
        assert_eq!(to_pascal_case(""), "");
    }

    #[test]
    fn test_profile_for_language() {
        assert_eq!(profile_for_language("python"), "full");
        assert_eq!(profile_for_language("typescript"), "full");
        assert_eq!(profile_for_language("wasm-deno"), "wasm");
        assert_eq!(profile_for_language("wasm-workers"), "wasm");
        assert_eq!(profile_for_language("c"), "ffi");
    }

    #[test]
    fn test_filter_fields_for_profile() {
        let mut fields = BTreeMap::new();
        fields.insert(
            "always".to_string(),
            FieldDef {
                json_type: "string".to_string(),
                required: true,
                feature_gate: None,
            },
        );
        fields.insert(
            "gated".to_string(),
            FieldDef {
                json_type: "number".to_string(),
                required: false,
                feature_gate: Some("ocr".to_string()),
            },
        );

        let enabled = vec!["ocr".to_string()];
        let filtered = filter_fields_for_profile(&fields, &enabled);
        assert_eq!(filtered.len(), 2);

        let empty: Vec<String> = vec![];
        let filtered = filter_fields_for_profile(&fields, &empty);
        assert_eq!(filtered.len(), 1);
        assert!(filtered.contains_key("always"));
    }

    #[test]
    fn test_deserialize_manifest() {
        let json = r#"{
            "version": 1,
            "types": {
                "ExtractionConfig": {
                    "kind": "struct",
                    "fields": {
                        "mime_type": {
                            "json_type": "string",
                            "required": true
                        }
                    }
                },
                "OutputFormat": {
                    "kind": "enum",
                    "values": ["text", "markdown", "html"]
                }
            },
            "feature_profiles": {
                "full": ["ocr", "table"],
                "wasm": ["table"],
                "ffi": ["ocr"]
            }
        }"#;

        let manifest: ParityManifest = serde_json::from_str(json).expect("deserialize");
        assert_eq!(manifest.version, 1);
        assert_eq!(manifest.types.len(), 2);
        assert_eq!(manifest.feature_profiles.len(), 3);
    }

    #[test]
    fn test_fields_for_type_and_lang() {
        let json = r#"{
            "version": 1,
            "types": {
                "Config": {
                    "kind": "struct",
                    "fields": {
                        "name": { "json_type": "string", "required": true },
                        "ocr_lang": { "json_type": "string", "required": false, "feature_gate": "ocr" }
                    }
                }
            },
            "feature_profiles": {
                "full": ["ocr", "table"],
                "wasm": ["table"],
                "ffi": ["ocr"]
            }
        }"#;

        let manifest: ParityManifest = serde_json::from_str(json).expect("deserialize");

        // Python uses "full" profile which includes "ocr" — both fields present
        let fields = fields_for_type_and_lang(&manifest, "Config", "python").unwrap();
        assert_eq!(fields.len(), 2);

        // wasm-deno uses "wasm" profile which lacks "ocr" — only ungated field
        let fields = fields_for_type_and_lang(&manifest, "Config", "wasm-deno").unwrap();
        assert_eq!(fields.len(), 1);
        assert!(fields.contains_key("name"));

        // Non-existent type returns None
        assert!(fields_for_type_and_lang(&manifest, "Missing", "python").is_none());
    }
}
