//! MCP static resource definitions and handlers.
//!
//! Exposes xberg metadata (supported formats, model manifest, OCR languages,
//! embedding presets) as MCP resources at well-known URIs.

use rmcp::ErrorData;
use rmcp::model::{
    AnnotateAble as _, ListResourceTemplatesResult, ListResourcesResult, RawResource, ReadResourceResult,
    ResourceContents,
};

/// URI for the supported document formats resource.
pub const URI_FORMATS: &str = "xberg://formats";

/// URI for the model manifest resource.
pub const URI_MODELS: &str = "xberg://models";

/// URI for the OCR languages resource.
pub const URI_OCR_LANGUAGES: &str = "xberg://languages/ocr";

/// URI for the embedding presets resource (requires `embeddings` feature).
#[cfg(feature = "embeddings")]
pub const URI_EMBEDDING_PRESETS: &str = "xberg://presets/embeddings";

/// Return the list of static MCP resources.
pub fn list_resources() -> ListResourcesResult {
    #[allow(unused_mut)]
    let mut resources = vec![
        RawResource::new(URI_FORMATS, "Supported Formats")
            .with_description("All document formats and MIME types supported by Xberg")
            .with_mime_type("application/json")
            .no_annotation(),
        RawResource::new(URI_MODELS, "Model Manifest")
            .with_description("Model files, sizes, and SHA256 checksums")
            .with_mime_type("application/json")
            .no_annotation(),
        RawResource::new(URI_OCR_LANGUAGES, "OCR Languages")
            .with_description("Available OCR language codes")
            .with_mime_type("application/json")
            .no_annotation(),
    ];

    #[cfg(feature = "embeddings")]
    resources.push(
        RawResource::new(URI_EMBEDDING_PRESETS, "Embedding Presets")
            .with_description("Available embedding model presets")
            .with_mime_type("application/json")
            .no_annotation(),
    );

    ListResourcesResult {
        resources,
        next_cursor: None,
        meta: None,
    }
}

/// Return an empty resource template list (no URI templates are defined).
pub fn list_resource_templates() -> ListResourceTemplatesResult {
    ListResourceTemplatesResult {
        resource_templates: vec![],
        next_cursor: None,
        meta: None,
    }
}

/// Read the contents of a static resource by URI.
///
/// # Errors
///
/// Returns [`ErrorData::invalid_params`] when the URI is not recognised.
pub fn read_resource(uri: &str) -> Result<ReadResourceResult, ErrorData> {
    match uri {
        URI_FORMATS => {
            let formats = crate::core::mime::list_supported_formats();
            let json = serde_json::to_string_pretty(&formats).unwrap_or_default();
            Ok(ReadResourceResult::new(vec![
                ResourceContents::text(json, uri).with_mime_type("application/json"),
            ]))
        }

        URI_MODELS => {
            #[allow(unused_mut)]
            let mut entries: Vec<serde_json::Value> = Vec::new();

            #[cfg(feature = "paddle-ocr")]
            {
                let manifest = crate::paddle_ocr::ModelManager::manifest();
                for entry in manifest {
                    entries.push(serde_json::to_value(&entry).unwrap_or_default());
                }
            }

            #[cfg(feature = "layout-detection")]
            {
                let manifest = crate::layout::LayoutModelManager::manifest();
                for entry in manifest {
                    entries.push(serde_json::to_value(&entry).unwrap_or_default());
                }
            }

            #[cfg(feature = "ner-onnx")]
            {
                let manifest = crate::text::ner::manifest();
                for entry in manifest {
                    entries.push(serde_json::to_value(&entry).unwrap_or_default());
                }
            }

            let payload = serde_json::json!({
                "xberg_version": env!("CARGO_PKG_VERSION"),
                "models": entries,
            });
            let json = serde_json::to_string_pretty(&payload).unwrap_or_default();
            Ok(ReadResourceResult::new(vec![
                ResourceContents::text(json, uri).with_mime_type("application/json"),
            ]))
        }

        URI_OCR_LANGUAGES => {
            // Canonical Tesseract ISO 639 language code list.
            let langs = serde_json::json!({
                "languages": [
                    "afr","amh","ara","asm","aze","bel","ben","bod","bos","bul",
                    "cat","ceb","ces","chi_sim","chi_tra","chr","cos","cym","dan","deu",
                    "div","dzo","ell","eng","enm","epo","est","eus","fao","fas",
                    "fil","fin","fra","frm","gle","glg","grc","guj","hat","heb",
                    "hin","hrv","hun","hye","iku","ind","isl","ita","ita_old","jav",
                    "jpn","kan","kat","kaz","khm","kir","kor","kur","lao","lat",
                    "lav","lit","ltz","mal","mar","mkd","mlt","mon","mri","msa",
                    "mya","nep","nor","oci","ori","pan","pol","por","pus","ron",
                    "rus","san","sin","slk","slv","snd","spa","spa_old","sqi","srp",
                    "swa","swe","syr","tam","tat","tel","tgk","tgl","tha","tir",
                    "ton","tur","uig","ukr","urd","uzb","vie","yid","yor"
                ],
                "source": "tesseract"
            });
            let json = serde_json::to_string_pretty(&langs).unwrap_or_default();
            Ok(ReadResourceResult::new(vec![
                ResourceContents::text(json, uri).with_mime_type("application/json"),
            ]))
        }

        #[cfg(feature = "embeddings")]
        URI_EMBEDDING_PRESETS => {
            let presets = crate::embeddings::list_presets();
            let payload = serde_json::json!({ "presets": presets });
            let json = serde_json::to_string_pretty(&payload).unwrap_or_default();
            Ok(ReadResourceResult::new(vec![
                ResourceContents::text(json, uri).with_mime_type("application/json"),
            ]))
        }

        other => Err(ErrorData::invalid_params(format!("Resource not found: {other}"), None)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_resources_uris() {
        let result = list_resources();
        let uris: Vec<&str> = result.resources.iter().map(|r| r.uri.as_str()).collect();
        assert!(uris.contains(&URI_FORMATS));
        assert!(uris.contains(&URI_MODELS));
        assert!(uris.contains(&URI_OCR_LANGUAGES));
    }

    #[test]
    fn test_list_resource_templates_is_empty() {
        let result = list_resource_templates();
        assert!(result.resource_templates.is_empty());
    }

    #[test]
    fn test_read_resource_formats_is_valid_json() {
        let result = read_resource(URI_FORMATS).expect("formats resource should be readable");
        assert!(!result.contents.is_empty());
        if let ResourceContents::TextResourceContents { text, .. } = &result.contents[0] {
            let _: serde_json::Value = serde_json::from_str(text).expect("formats should be valid JSON");
        } else {
            panic!("Expected TextResourceContents");
        }
    }

    #[test]
    fn test_read_resource_models_is_valid_json() {
        let result = read_resource(URI_MODELS).expect("models resource should be readable");
        assert!(!result.contents.is_empty());
        if let ResourceContents::TextResourceContents { text, .. } = &result.contents[0] {
            let v: serde_json::Value = serde_json::from_str(text).expect("models should be valid JSON");
            assert!(v.get("xberg_version").is_some());
            assert!(v.get("models").is_some());
        } else {
            panic!("Expected TextResourceContents");
        }
    }

    #[test]
    fn test_read_resource_ocr_languages_is_valid_json() {
        let result = read_resource(URI_OCR_LANGUAGES).expect("languages resource should be readable");
        assert!(!result.contents.is_empty());
        if let ResourceContents::TextResourceContents { text, .. } = &result.contents[0] {
            let v: serde_json::Value = serde_json::from_str(text).expect("languages should be valid JSON");
            assert!(v.get("languages").is_some());
            let langs = v["languages"].as_array().expect("languages should be array");
            assert!(!langs.is_empty());
            // eng must be present
            let has_eng = langs.iter().any(|l| l.as_str() == Some("eng"));
            assert!(has_eng, "eng should be in OCR languages list");
        } else {
            panic!("Expected TextResourceContents");
        }
    }

    #[test]
    fn test_read_resource_unknown_uri_errors() {
        let result = read_resource("xberg://unknown");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("Resource not found"));
    }

    #[test]
    fn test_resources_have_mime_type() {
        let result = list_resources();
        for resource in &result.resources {
            assert!(
                resource.mime_type.is_some(),
                "Resource '{}' should have a mime_type",
                resource.uri
            );
        }
    }

    #[test]
    fn test_resources_have_descriptions() {
        let result = list_resources();
        for resource in &result.resources {
            assert!(
                resource.description.is_some(),
                "Resource '{}' should have a description",
                resource.uri
            );
        }
    }
}
