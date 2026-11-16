//! Trust-boundary validation for caller-owned REST and MCP extraction config.

use serde_json::Value;

const FORBIDDEN_LLM_FIELDS: &[&str] = &[
    "api_key",
    "base_url",
    "load_env",
    "headers",
    "providers",
    "bedrock",
    "credential_provider",
];

const LLM_LOCATIONS: &[(&str, &str)] = &[
    ("/structured_extraction/llm", "structured_extraction.llm"),
    ("/ocr/vlm_config", "ocr.vlm_config"),
    ("/chunking/embedding/model/llm", "chunking.embedding.model.llm"),
    ("/ner/llm", "ner.llm"),
    ("/summarization/llm", "summarization.llm"),
    ("/translation/llm", "translation.llm"),
    ("/page_classification/llm", "page_classification.llm"),
    ("/chunk_classification/llm", "chunk_classification.llm"),
    ("/captioning/llm", "captioning.llm"),
];

/// Server environment variable that lets a caller's LLM stage authenticate with the server's
/// environment credentials (provider API-key variables, the AWS credential chain, Vertex ADC).
pub(crate) const ALLOW_CALLER_LLM_ENV_CREDENTIALS_ENV: &str = "XBERG_ALLOW_CALLER_LLM_ENV_CREDENTIALS";

/// Server filesystem paths, model sources and process-global settings, as JSON pointers
/// into an `ExtractionConfig` or `FileExtractionConfig`.
const OPERATOR_OWNED_FIELDS: &[&str] = &[
    "/html_output/css_file",
    "/tree_sitter/cache_dir",
    "/ocr/tessdata_path",
    "/chunking/embedding/cache_dir",
    "/chunking/embedding/model/model_id",
    "/chunking/sparse_embedding/cache_dir",
    "/chunking/sparse_embedding/model/model_id",
    "/chunking/late_interaction/cache_dir",
    "/chunking/late_interaction/model/model_id",
    "/transcription/model_cache_dir",
    "/redaction/findings_path",
    "/concurrency/max_threads",
    "/concurrency/max_concurrent_ocr",
];

/// Operator-owned fields of one OCR engine selection: the `ocr` section and each
/// `ocr.pipeline.stages[i]`. `backend_options` is parsed by the backend, which accepts the
/// dashed aliases too (candle OCR) and a nested `model` section (Sceptre).
const OPERATOR_OWNED_OCR_ENGINE_FIELDS: &[&str] = &[
    "/tesseract_config/tessdata_path",
    "/paddle_ocr_settings/cache_dir",
    "/backend_options/cache_dir",
    "/backend_options/cache-dir",
    "/backend_options/model_path",
    "/backend_options/model-path",
    "/backend_options/model_id",
    "/backend_options/model-id",
    "/backend_options/model/detector_path",
    "/backend_options/model/recognizer_path",
    "/backend_options/model/cache_dir",
    "/backend_options/model/registry_owner",
];

/// Reject transport, credential, filesystem and process-global settings supplied by an
/// untrusted request.
pub(crate) fn validate_caller_extraction_config(config: &Value) -> Result<(), String> {
    #[cfg(feature = "url-config-types")]
    if let Some(crawl) = config.pointer("/url/crawl") {
        crawlberg::reject_untrusted_fields(crawl)
            .map_err(|error| format!("Caller extraction config contains forbidden url.crawl setting: {error}"))?;
    }

    for (pointer, path) in LLM_LOCATIONS {
        if let Some(llm) = config.pointer(pointer) {
            validate_llm_fields(llm, path)?;
        }
    }

    reject_operator_owned_fields(config, "", OPERATOR_OWNED_FIELDS)?;
    if let Some(ocr) = config.get("ocr") {
        validate_ocr_engine_fields(ocr, "ocr.")?;
    }

    if let Some(stages) = config.pointer("/ocr/pipeline/stages").and_then(Value::as_array) {
        for (index, stage) in stages.iter().enumerate() {
            if let Some(llm) = stage.get("vlm_config") {
                validate_llm_fields(llm, &format!("ocr.pipeline.stages[{index}].vlm_config"))?;
            }
            validate_ocr_engine_fields(stage, &format!("ocr.pipeline.stages[{index}]."))?;
        }
    }

    Ok(())
}

fn validate_ocr_engine_fields(engine: &Value, prefix: &str) -> Result<(), String> {
    reject_operator_owned_fields(engine, prefix, OPERATOR_OWNED_OCR_ENGINE_FIELDS)?;
    if let Some(legacy) = engine.get("paddle_ocr_config") {
        let canonical = crate::paddle_ocr::PaddleOcrConfig::canonicalize_legacy_fields(legacy);
        reject_operator_owned_fields(&canonical, &format!("{prefix}paddle_ocr_config."), &["/cache_dir"])?;
    }
    Ok(())
}

fn reject_operator_owned_fields(value: &Value, prefix: &str, pointers: &[&str]) -> Result<(), String> {
    for pointer in pointers {
        if value.pointer(pointer).is_some_and(|field| !field.is_null()) {
            let path = pointer.trim_start_matches('/').replace('/', ".");
            return Err(format!("Caller extraction config may not set {prefix}{path}"));
        }
    }
    Ok(())
}

/// Build the effective config for an untrusted request: validate the caller JSON, merge it
/// over the operator config and re-apply the operator's policy to the result.
pub(crate) fn resolve_caller_extraction_config(
    operator: &super::ExtractionConfig,
    caller: Option<&Value>,
) -> Result<super::ExtractionConfig, String> {
    if let Some(caller) = caller {
        validate_caller_extraction_config(caller)?;
    }
    let caller_json = caller
        .map(|caller| {
            let mut caller = caller.clone();
            merge_html_override(operator, &mut caller)?;
            Ok::<_, String>(caller.to_string())
        })
        .transpose()?;
    let mut config = super::merge::build_config_from_json(operator, caller_json.as_deref())?;
    adopt_operator_policy(&mut config, operator);
    Ok(config)
}

/// [`resolve_caller_extraction_config`] for a caller that reaches the server over the network
/// (the REST API and MCP over HTTP), whose LLM stages also need the operator's opt-in; see
/// [`check_caller_llm_stages`]. MCP over stdio runs as the operator and skips it.
/// `allow_env_credentials` is [`caller_llm_env_credentials_allowed`] on a server.
pub(crate) fn resolve_remote_caller_extraction_config(
    operator: &super::ExtractionConfig,
    caller: Option<&Value>,
    allow_env_credentials: bool,
) -> Result<super::ExtractionConfig, String> {
    let effective = resolve_caller_extraction_config(operator, caller)?;
    if caller.is_some() {
        check_caller_llm_stages(
            &config_json(operator)?,
            &config_json(&effective)?,
            allow_env_credentials,
        )?;
    }
    Ok(effective)
}

pub(crate) fn merge_html_override(operator: &super::ExtractionConfig, caller: &mut Value) -> Result<(), String> {
    #[cfg(feature = "html")]
    if let (Some(base), Some(overrides)) = (
        operator.html_output.as_ref(),
        caller.get("html_output").and_then(Value::as_object),
    ) {
        let mut html = serde_json::to_value(base).map_err(|e| format!("Failed to serialize HTML config: {e}"))?;
        if let Some(fields) = html.as_object_mut() {
            fields.extend(overrides.clone());
        }
        caller["html_output"] = html;
    }
    #[cfg(not(feature = "html"))]
    let _ = (operator, caller);
    Ok(())
}

/// Re-apply operator policy to a request config already merged over `operator`: crawl
/// egress comes from the operator, and resource limits can only tighten.
pub(crate) fn adopt_operator_policy(config: &mut super::ExtractionConfig, operator: &super::ExtractionConfig) {
    adopt_operator_crawl_egress(config, operator);
    config.concurrency = operator.concurrency.clone();
    #[cfg(feature = "html")]
    preserve_operator_css_file(config.html_output.as_mut(), operator.html_output.as_ref());
    tighten_security_limits(&mut config.security_limits, operator.security_limits.as_ref());
    config.max_archive_depth = config.max_archive_depth.min(operator.max_archive_depth);
    config.extraction_timeout_secs = stricter_cap(config.extraction_timeout_secs, operator.extraction_timeout_secs);
    config.max_embedded_file_bytes = stricter_cap(config.max_embedded_file_bytes, operator.max_embedded_file_bytes);
    tighten_url_limits(&mut config.url, &operator.url);
    #[cfg(feature = "transcription-types")]
    if let Some(transcription) = config.transcription.as_mut() {
        tighten_transcription_limits(transcription, operator.transcription.as_ref());
    }
}

/// Apply the policy of the request's effective config to a per-input override, which
/// `ExtractionConfig::with_file_overrides` would otherwise let replace it.
pub(crate) fn adopt_file_operator_policy(
    config: &mut super::FileExtractionConfig,
    effective: &super::ExtractionConfig,
) {
    adopt_file_operator_crawl_egress(config, effective);
    #[cfg(feature = "html")]
    preserve_operator_css_file(config.html_output.as_mut(), effective.html_output.as_ref());
    if let Some(timeout) = config.timeout_secs {
        config.timeout_secs = stricter_cap(Some(timeout), effective.extraction_timeout_secs);
    }
    if let Some(url) = config.url.as_mut() {
        tighten_url_limits(url, &effective.url);
    }
    #[cfg(feature = "transcription-types")]
    if let Some(transcription) = config.transcription.as_mut() {
        tighten_transcription_limits(transcription, effective.transcription.as_ref());
    }
}

#[cfg(feature = "html")]
fn preserve_operator_css_file(
    html: Option<&mut super::html_output::HtmlOutputConfig>,
    operator: Option<&super::html_output::HtmlOutputConfig>,
) {
    if let Some(html) = html {
        html.css_file = operator.and_then(|html| html.css_file.clone());
    }
}

/// The stricter of two optional caps, where `None` means unlimited.
fn stricter_cap<T: Ord>(caller: Option<T>, operator: Option<T>) -> Option<T> {
    match (caller, operator) {
        (Some(caller), Some(operator)) => Some(caller.min(operator)),
        (caller, operator) => caller.or(operator),
    }
}

fn tighten_security_limits(
    limits: &mut Option<crate::extractors::security::SecurityLimits>,
    operator: Option<&crate::extractors::security::SecurityLimits>,
) {
    let Some(caller) = limits.as_mut() else {
        *limits = operator.cloned();
        return;
    };
    // `None` on the operator side means the defaults are in force, so they are the bound.
    let crate::extractors::security::SecurityLimits {
        max_archive_size,
        max_compression_ratio,
        max_files_in_archive,
        max_nesting_depth,
        max_entity_length,
        max_content_size,
        max_iterations,
        max_xml_depth,
        max_table_cells,
        max_pages,
    } = operator.cloned().unwrap_or_default();
    caller.max_archive_size = caller.max_archive_size.min(max_archive_size);
    caller.max_compression_ratio = caller.max_compression_ratio.min(max_compression_ratio);
    caller.max_files_in_archive = caller.max_files_in_archive.min(max_files_in_archive);
    caller.max_nesting_depth = caller.max_nesting_depth.min(max_nesting_depth);
    caller.max_entity_length = caller.max_entity_length.min(max_entity_length);
    caller.max_content_size = caller.max_content_size.min(max_content_size);
    caller.max_iterations = caller.max_iterations.min(max_iterations);
    caller.max_xml_depth = caller.max_xml_depth.min(max_xml_depth);
    caller.max_table_cells = caller.max_table_cells.min(max_table_cells);
    caller.max_pages = stricter_cap(caller.max_pages, max_pages);
}

fn tighten_url_limits(url: &mut super::UrlExtractionConfig, operator: &super::UrlExtractionConfig) {
    url.max_document_urls_per_result =
        stricter_cap(url.max_document_urls_per_result, operator.max_document_urls_per_result);
    url.max_total_urls = stricter_cap(url.max_total_urls, operator.max_total_urls);
    url.allow_local_file_inputs &= operator.allow_local_file_inputs;
    url.allow_file_uris &= operator.allow_file_uris;
}

#[cfg(feature = "transcription-types")]
fn tighten_transcription_limits(
    transcription: &mut super::TranscriptionConfig,
    operator: Option<&super::TranscriptionConfig>,
) {
    let operator = operator.cloned().unwrap_or_default();
    transcription.max_duration_ms = stricter_cap(transcription.max_duration_ms, operator.max_duration_ms);
    transcription.max_bytes = stricter_cap(transcription.max_bytes, operator.max_bytes);
    transcription.timeout_ms = stricter_cap(transcription.timeout_ms, operator.timeout_ms);
    transcription.allow_network &= operator.allow_network;
    transcription.verify_hash |= operator.verify_hash;
}

/// Whether the operator set [`ALLOW_CALLER_LLM_ENV_CREDENTIALS_ENV`]. Read by the servers, which
/// pass it on, so tests can set the flag without touching the environment.
#[cfg_attr(not(any(feature = "api", feature = "mcp-http")), allow(dead_code))]
pub(crate) fn caller_llm_env_credentials_allowed() -> bool {
    std::env::var(ALLOW_CALLER_LLM_ENV_CREDENTIALS_ENV)
        .is_ok_and(|value| matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
}

/// Reject an LLM stage of a remote caller's per-input override that differs from the request's
/// effective stage at the same place, unless the operator set
/// [`ALLOW_CALLER_LLM_ENV_CREDENTIALS_ENV`]. `input` is the override as it will run.
pub(crate) fn reject_caller_input_llm_stages(
    effective: &super::ExtractionConfig,
    input: &super::FileExtractionConfig,
    allow_env_credentials: bool,
) -> Result<(), String> {
    check_caller_llm_stages(&config_json(effective)?, &config_json(input)?, allow_env_credentials)
}

fn config_json(config: &impl serde::Serialize) -> Result<Value, String> {
    serde_json::to_value(config).map_err(|e| format!("Failed to serialize extraction config: {e}"))
}

/// Every LLM stage of the configuration that will run (`effective`) must be identical, field for
/// field, to the stage the trusted configuration (`operator`) has at the same place: then the
/// caller changed nothing and the stage runs as the operator configured it. A new or changed stage
/// could spend the server's credentials on the caller's terms, so it needs the operator's opt-in.
fn check_caller_llm_stages(operator: &Value, effective: &Value, allow_env_credentials: bool) -> Result<(), String> {
    if allow_env_credentials {
        return Ok(());
    }
    for (pointer, path) in caller_llm_stages(effective) {
        if effective.pointer(&pointer) != operator.pointer(&pointer) {
            return Err(format!(
                "Caller extraction config may not use the server's LLM credentials for {path}; \
                 the server operator must set {ALLOW_CALLER_LLM_ENV_CREDENTIALS_ENV}=1 to allow it"
            ));
        }
    }
    Ok(())
}

/// JSON pointer and dotted path of every non-null LLM config in `config`.
fn caller_llm_stages(config: &Value) -> Vec<(String, String)> {
    let mut stages: Vec<(String, String)> = LLM_LOCATIONS
        .iter()
        .filter(|(pointer, _)| config.pointer(pointer).is_some_and(|llm| !llm.is_null()))
        .map(|(pointer, path)| ((*pointer).to_string(), (*path).to_string()))
        .collect();
    if let Some(pipeline) = config.pointer("/ocr/pipeline/stages").and_then(Value::as_array) {
        for (index, stage) in pipeline.iter().enumerate() {
            if stage.get("vlm_config").is_some_and(|llm| !llm.is_null()) {
                stages.push((
                    format!("/ocr/pipeline/stages/{index}/vlm_config"),
                    format!("ocr.pipeline.stages[{index}].vlm_config"),
                ));
            }
        }
    }
    stages
}

fn validate_llm_fields(llm: &Value, path: &str) -> Result<(), String> {
    for field in FORBIDDEN_LLM_FIELDS {
        if llm.get(field).is_some_and(|value| !value.is_null()) {
            return Err(format!("Caller extraction config may not set {path}.{field}"));
        }
    }
    Ok(())
}

pub(crate) fn adopt_operator_crawl_egress(config: &mut super::ExtractionConfig, operator: &super::ExtractionConfig) {
    #[cfg(feature = "url-config-types")]
    config.url.crawl.adopt_operator_egress(&operator.url.crawl);
    #[cfg(not(feature = "url-config-types"))]
    let _ = (config, operator);
}

pub(crate) fn adopt_file_operator_crawl_egress(
    config: &mut super::FileExtractionConfig,
    operator: &super::ExtractionConfig,
) {
    #[cfg(feature = "url-config-types")]
    if let Some(url) = config.url.as_mut() {
        url.crawl.adopt_operator_egress(&operator.url.crawl);
    }
    #[cfg(not(feature = "url-config-types"))]
    let _ = (config, operator);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_keep_operator_concurrency_when_request_uses_defaults() {
        let operator = super::super::ExtractionConfig {
            concurrency: Some(super::super::ConcurrencyConfig {
                max_threads: Some(2),
                max_concurrent_ocr: Some(1),
            }),
            ..Default::default()
        };

        let caller = serde_json::json!({"concurrency": {}});
        let resolved = resolve_caller_extraction_config(&operator, Some(&caller)).unwrap();
        let concurrency = resolved.concurrency.unwrap();
        assert_eq!(concurrency.max_threads, Some(2));
        assert_eq!(concurrency.max_concurrent_ocr, Some(1));
    }

    #[test]
    #[cfg(feature = "html")]
    fn should_preserve_omitted_operator_html_settings() {
        use super::super::html_output::{HtmlOutputConfig, HtmlTheme};

        let operator = super::super::ExtractionConfig {
            html_output: Some(HtmlOutputConfig {
                css_file: Some("operator.css".into()),
                theme: HtmlTheme::Dark,
                class_prefix: "operator-".into(),
                embed_css: false,
                ..Default::default()
            }),
            ..Default::default()
        };
        let caller = serde_json::json!({"html_output": {"css": "body { color: red; }", "embed_css": true}});
        let resolved = resolve_caller_extraction_config(&operator, Some(&caller)).unwrap();
        let html = resolved.html_output.as_ref().unwrap();
        assert_eq!(html.css_file.as_deref(), Some(std::path::Path::new("operator.css")));
        assert_eq!(html.theme, HtmlTheme::Dark);
        assert_eq!(html.class_prefix, "operator-");
        assert_eq!(html.css.as_deref(), Some("body { color: red; }"));
        assert!(html.embed_css);

        let mut file = super::super::FileExtractionConfig {
            html_output: Some(HtmlOutputConfig::default()),
            ..Default::default()
        };
        adopt_file_operator_policy(&mut file, &resolved);
        assert_eq!(file.html_output.unwrap().css_file, html.css_file);
    }

    const LLM_PATHS: &[&[&str]] = &[
        &["structured_extraction", "llm"],
        &["ocr", "vlm_config"],
        &["chunking", "embedding", "model", "llm"],
        &["ner", "llm"],
        &["summarization", "llm"],
        &["translation", "llm"],
        &["page_classification", "llm"],
        &["chunk_classification", "llm"],
        &["captioning", "llm"],
    ];

    const FORBIDDEN_FIELDS: &[&str] = &[
        "api_key",
        "base_url",
        "load_env",
        "headers",
        "providers",
        "bedrock",
        "credential_provider",
    ];

    #[cfg(feature = "url-config-types")]
    const FORBIDDEN_CRAWL_FIELDS: &[&str] = &[
        "/danger_accept_invalid_certs",
        "/ssrf",
        "/ssrf_deny_private_explicit",
        "/max_redirects",
        "/proxy",
        "/browser/proxy",
        "/browser/endpoint",
        "/browser/chrome_path",
        "/browser/chrome_args",
        "/browser/private_pipe",
        "/browser/eval_script",
        "/browser/session_affinity",
        "/browser_profile",
        "/save_browser_profile",
        "/document_output_dir",
        "/warc_output",
    ];

    fn nested_config(path: &[&str], field: &str, value: Value) -> Value {
        let mut current = serde_json::json!({field: value});
        for segment in path.iter().rev() {
            current = serde_json::json!({(*segment): current});
        }
        current
    }

    fn value_with_pointer(pointer: &str, leaf: Value) -> Value {
        pointer
            .split('/')
            .filter(|segment| !segment.is_empty())
            .rev()
            .fold(leaf, |value, segment| serde_json::json!({segment: value}))
    }

    #[test]
    fn should_reject_each_transport_field_at_every_llm_path() {
        for path in LLM_PATHS {
            for field in FORBIDDEN_FIELDS {
                let config = nested_config(path, field, serde_json::json!("secret"));
                let error = validate_caller_extraction_config(&config).expect_err("field must be rejected");
                assert_eq!(
                    error,
                    format!("Caller extraction config may not set {}.{field}", path.join("."))
                );
            }
        }
    }

    #[test]
    fn should_reject_transport_fields_in_each_ocr_pipeline_stage() {
        let config = serde_json::json!({
            "ocr": {
                "pipeline": {
                    "stages": [
                        {"vlm_config": {"model": "safe"}},
                        {"vlm_config": {"base_url": "http://internal"}}
                    ]
                }
            }
        });

        let error = validate_caller_extraction_config(&config).expect_err("base_url must be rejected");
        assert_eq!(
            error,
            "Caller extraction config may not set ocr.pipeline.stages[1].vlm_config.base_url"
        );
    }

    #[test]
    fn should_allow_null_transport_fields_and_safe_generation_fields() {
        let config = serde_json::json!({
            "structured_extraction": {
                "llm": {
                    "api_key": null,
                    "base_url": null,
                    "load_env": null,
                    "headers": null,
                    "providers": null,
                    "bedrock": null,
                    "credential_provider": null,
                    "model": "openai/gpt-4o-mini",
                    "temperature": 0.2,
                    "max_tokens": 100
                }
            }
        });

        assert_eq!(validate_caller_extraction_config(&config), Ok(()));
    }

    #[test]
    fn should_ignore_forbidden_lookalikes_in_opaque_values() {
        let config = serde_json::json!({
            "ocr": {
                "backend_options": {"api_key": "backend-owned"},
                "pipeline": {
                    "stages": [{"backend": "custom", "backend_options": {"providers": []}}]
                }
            },
            "structured_extraction": {
                "llm": {
                    "model": "openai/gpt-4o-mini",
                    "extra_body": {"base_url": "provider-specific", "headers": {"x": "y"}}
                },
                "schema": {
                    "type": "object",
                    "properties": {"credential_provider": {"type": "string"}}
                }
            }
        });

        assert_eq!(validate_caller_extraction_config(&config), Ok(()));
    }

    fn operator_with_vlm(llm: super::super::LlmConfig) -> super::super::ExtractionConfig {
        super::super::ExtractionConfig {
            ocr: Some(super::super::OcrConfig {
                vlm_config: Some(llm),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn operator_vlm() -> super::super::LlmConfig {
        super::super::LlmConfig {
            model: "openai/gpt-4o-mini".to_string(),
            temperature: Some(0.2),
            ..Default::default()
        }
    }

    fn caller_vlm(temperature: f64) -> Value {
        serde_json::json!({"ocr": {"vlm_config": {"model": "openai/gpt-4o-mini", "temperature": temperature}}})
    }

    fn env_credentials_error(path: &str) -> String {
        format!(
            "Caller extraction config may not use the server's LLM credentials for {path}; \
             the server operator must set XBERG_ALLOW_CALLER_LLM_ENV_CREDENTIALS=1 to allow it"
        )
    }

    /// The operator and effective configs as JSON, after the caller JSON is merged as it runs.
    fn operator_and_effective(operator: &super::super::ExtractionConfig, caller: &Value) -> (Value, Value) {
        let effective = resolve_caller_extraction_config(operator, Some(caller)).expect("caller config merges");
        (config_json(operator).unwrap(), config_json(&effective).unwrap())
    }

    #[test]
    fn should_reject_new_llm_stage_at_every_path_without_operator_opt_in() {
        let operator = config_json(&super::super::ExtractionConfig::default()).unwrap();
        for path in LLM_PATHS {
            let effective = nested_config(path, "model", serde_json::json!("openai/gpt-4o-mini"));

            assert_eq!(
                check_caller_llm_stages(&operator, &effective, false),
                Err(env_credentials_error(&path.join(".")))
            );
            assert_eq!(check_caller_llm_stages(&operator, &effective, true), Ok(()));
        }
    }

    #[test]
    fn should_reject_new_vlm_in_ocr_pipeline_stage_without_operator_opt_in() {
        let effective = serde_json::json!({
            "ocr": {"pipeline": {"stages": [{"backend": "tesseract"}, {"vlm_config": {"model": "openai/gpt-4o"}}]}}
        });

        assert_eq!(
            check_caller_llm_stages(&serde_json::json!({}), &effective, false),
            Err(env_credentials_error("ocr.pipeline.stages[1].vlm_config"))
        );
    }

    #[test]
    fn should_accept_operator_llm_stage_resent_unchanged() {
        let operator = operator_with_vlm(operator_vlm());
        let (operator_json, effective) = operator_and_effective(&operator, &caller_vlm(0.2));

        assert_eq!(check_caller_llm_stages(&operator_json, &effective, false), Ok(()));
        assert_eq!(
            resolve_remote_caller_extraction_config(&operator, Some(&caller_vlm(0.2)), false)
                .map(|config| config.ocr.and_then(|ocr| ocr.vlm_config)),
            Ok(Some(operator_vlm()))
        );
    }

    #[test]
    fn should_reject_tweaked_operator_stage_without_opt_in_and_accept_it_with_opt_in() {
        let operator = operator_with_vlm(operator_vlm());
        let (operator_json, effective) = operator_and_effective(&operator, &caller_vlm(0.7));

        assert_eq!(
            check_caller_llm_stages(&operator_json, &effective, false),
            Err(env_credentials_error("ocr.vlm_config"))
        );
        assert_eq!(check_caller_llm_stages(&operator_json, &effective, true), Ok(()));
    }

    #[test]
    fn should_reject_operator_stage_resent_without_its_credentials() {
        let operator = operator_with_vlm(super::super::LlmConfig {
            api_key: Some("operator-key".to_string()),
            ..operator_vlm()
        });
        let (operator_json, effective) = operator_and_effective(&operator, &caller_vlm(0.2));

        let error = check_caller_llm_stages(&operator_json, &effective, false)
            .expect_err("the caller's copy drops the operator's key, so it would run on the environment");
        assert_eq!(error, env_credentials_error("ocr.vlm_config"));
        assert!(!error.contains("operator-key"));
    }

    #[test]
    fn should_gate_caller_llm_stage_only_when_resolving_a_remote_request_config() {
        let operator = super::super::ExtractionConfig::default();
        let caller = serde_json::json!({"ocr": {"vlm_config": {"model": "openai/gpt-4o"}}});

        assert_eq!(
            resolve_remote_caller_extraction_config(&operator, Some(&caller), false).map(drop),
            Err(env_credentials_error("ocr.vlm_config"))
        );
        let local = resolve_caller_extraction_config(&operator, Some(&caller)).expect("local caller is the operator");
        assert_eq!(
            local.ocr.and_then(|ocr| ocr.vlm_config).map(|llm| llm.model),
            Some("openai/gpt-4o".to_string())
        );
    }

    #[test]
    fn should_compare_per_input_llm_stage_with_the_request_stage() {
        let effective = operator_with_vlm(operator_vlm());
        let input = |llm| super::super::FileExtractionConfig {
            ocr: Some(super::super::OcrConfig {
                vlm_config: Some(llm),
                ..Default::default()
            }),
            ..Default::default()
        };

        assert_eq!(
            reject_caller_input_llm_stages(&effective, &input(operator_vlm()), false),
            Ok(())
        );
        let tweaked = super::super::LlmConfig {
            temperature: Some(0.7),
            ..operator_vlm()
        };
        assert_eq!(
            reject_caller_input_llm_stages(&effective, &input(tweaked), false),
            Err(env_credentials_error("ocr.vlm_config"))
        );
        assert_eq!(
            reject_caller_input_llm_stages(
                &super::super::ExtractionConfig::default(),
                &input(operator_vlm()),
                false
            ),
            Err(env_credentials_error("ocr.vlm_config"))
        );
    }

    #[test]
    fn should_reject_caller_css_file() {
        let config = serde_json::json!({"html_output": {"css_file": "/proc/self/environ"}});

        assert_eq!(
            validate_caller_extraction_config(&config),
            Err("Caller extraction config may not set html_output.css_file".to_string())
        );
    }

    #[test]
    fn should_reject_caller_tree_sitter_cache_dir() {
        let config = serde_json::json!({"tree_sitter": {"cache_dir": "/tmp/attacker-grammars"}});

        assert_eq!(
            validate_caller_extraction_config(&config),
            Err("Caller extraction config may not set tree_sitter.cache_dir".to_string())
        );
    }

    #[test]
    fn should_reject_caller_tessdata_path() {
        let config = serde_json::json!({"ocr": {"tessdata_path": "/tmp/attacker-tessdata"}});

        assert_eq!(
            validate_caller_extraction_config(&config),
            Err("Caller extraction config may not set ocr.tessdata_path".to_string())
        );
    }

    #[test]
    fn should_reject_each_operator_owned_field_without_leaking_its_value() {
        for pointer in OPERATOR_OWNED_FIELDS {
            let secret = format!("/secret/for{pointer}");
            let config = value_with_pointer(pointer, serde_json::json!(secret));

            let error = validate_caller_extraction_config(&config).expect_err("operator-owned field must be rejected");

            let path = pointer.trim_start_matches('/').replace('/', ".");
            assert_eq!(error, format!("Caller extraction config may not set {path}"));
            assert!(!error.contains(&secret), "rejection must not expose the field value");
        }
    }

    #[test]
    fn should_reject_operator_owned_ocr_engine_fields_in_ocr_and_each_pipeline_stage() {
        for pointer in OPERATOR_OWNED_OCR_ENGINE_FIELDS
            .iter()
            .copied()
            .chain(["/paddle_ocr_config/cache_dir"])
        {
            let path = pointer.trim_start_matches('/').replace('/', ".");
            let engine = value_with_pointer(pointer, serde_json::json!("/srv/models"));

            let config = serde_json::json!({"ocr": engine.clone()});
            assert_eq!(
                validate_caller_extraction_config(&config),
                Err(format!("Caller extraction config may not set ocr.{path}"))
            );

            let config = serde_json::json!({"ocr": {"pipeline": {"stages": [{"backend": "tesseract"}, engine]}}});
            assert_eq!(
                validate_caller_extraction_config(&config),
                Err(format!(
                    "Caller extraction config may not set ocr.pipeline.stages[1].{path}"
                ))
            );
        }
    }

    #[test]
    fn should_reject_legacy_paddle_cache_dir_alias_at_both_ocr_levels() {
        let engine = serde_json::json!({"backend": "paddleocr", "paddle_ocr_config": {"cacheDir": "model-cache"}});
        let top_level = serde_json::json!({"ocr": engine.clone()});
        let pipeline = serde_json::json!({"ocr": {"pipeline": {"stages": [engine]}}});

        assert_eq!(
            validate_caller_extraction_config(&top_level),
            Err("Caller extraction config may not set ocr.paddle_ocr_config.cache_dir".to_string())
        );
        assert_eq!(
            validate_caller_extraction_config(&pipeline),
            Err("Caller extraction config may not set ocr.pipeline.stages[0].paddle_ocr_config.cache_dir".to_string())
        );
    }

    #[test]
    fn should_allow_null_operator_owned_fields_and_preset_models() {
        for pointer in OPERATOR_OWNED_FIELDS {
            assert_eq!(
                validate_caller_extraction_config(&value_with_pointer(pointer, Value::Null)),
                Ok(())
            );
        }
        for pointer in OPERATOR_OWNED_OCR_ENGINE_FIELDS
            .iter()
            .copied()
            .chain(["/paddle_ocr_config/cache_dir"])
        {
            let config = serde_json::json!({"ocr": value_with_pointer(pointer, Value::Null)});
            assert_eq!(validate_caller_extraction_config(&config), Ok(()));
        }

        let config = serde_json::json!({
            "chunking": {"embedding": {"model": {"type": "preset", "name": "balanced"}}},
            "ocr": {"backend": "tesseract", "language": ["eng"], "backend_options": {"mode": "accurate"}},
            "tree_sitter": {"enabled": true},
            "html_output": {"css": "body { color: red; }"}
        });
        assert_eq!(validate_caller_extraction_config(&config), Ok(()));
    }

    #[test]
    #[cfg(feature = "url-config-types")]
    fn should_reject_each_operator_owned_crawl_field_without_leaking_its_value() {
        let mut dependency_fields = crawlberg::UNTRUSTED_CALLER_FORBIDDEN_FIELDS.to_vec();
        let mut expected_fields = FORBIDDEN_CRAWL_FIELDS.to_vec();
        dependency_fields.sort_unstable();
        expected_fields.sort_unstable();
        assert_eq!(dependency_fields, expected_fields);

        for pointer in FORBIDDEN_CRAWL_FIELDS {
            let secret = format!("secret-for-{pointer}");
            let config = serde_json::json!({
                "url": {"crawl": value_with_pointer(pointer, serde_json::json!(secret))}
            });

            let error =
                validate_caller_extraction_config(&config).expect_err("operator-owned crawl field must be rejected");

            assert_eq!(
                error,
                format!(
                    "Caller extraction config contains forbidden url.crawl setting: \
                     invalid_config: untrusted caller may not set {pointer}"
                )
            );
            assert!(!error.contains(&secret), "rejection must not expose the field value");
        }
    }

    #[test]
    #[cfg(feature = "url-config-types")]
    fn should_allow_null_operator_owned_crawl_fields_and_caller_owned_fields() {
        for pointer in FORBIDDEN_CRAWL_FIELDS {
            let config = serde_json::json!({
                "url": {"crawl": value_with_pointer(pointer, Value::Null)}
            });
            assert_eq!(validate_caller_extraction_config(&config), Ok(()));
        }

        let config = serde_json::json!({
            "url": {
                "crawl": {
                    "max_depth": 2,
                    "auth": {"type": "bearer", "token": "caller-target-token"},
                    "custom_headers": {"Authorization": "caller-target-token"}
                }
            }
        });
        assert_eq!(validate_caller_extraction_config(&config), Ok(()));
    }

    #[test]
    #[cfg(feature = "url-config-types")]
    fn should_reject_caller_tls_verification_overrides_for_both_boolean_values() {
        for enabled in [false, true] {
            let config = serde_json::json!({
                "url": {"crawl": {"danger_accept_invalid_certs": enabled}}
            });
            let error = validate_caller_extraction_config(&config).expect_err("TLS policy is operator-owned");
            assert_eq!(
                error,
                "Caller extraction config contains forbidden url.crawl setting: \
                 invalid_config: untrusted caller may not set /danger_accept_invalid_certs"
            );
        }
    }

    #[test]
    #[cfg(feature = "url-config-types")]
    fn should_preserve_operator_tls_verification_policy_for_both_config_surfaces() {
        for enabled in [false, true] {
            let mut operator = crate::ExtractionConfig::default();
            operator.url.crawl.danger_accept_invalid_certs = enabled;
            let mut config = crate::ExtractionConfig::default();
            config.url.crawl.danger_accept_invalid_certs = !enabled;
            adopt_operator_crawl_egress(&mut config, &operator);
            assert_eq!(config.url.crawl.danger_accept_invalid_certs, enabled);

            let mut file = crate::FileExtractionConfig {
                url: Some(config.url.clone()),
                ..Default::default()
            };
            file.url
                .as_mut()
                .expect("URL configuration exists")
                .crawl
                .danger_accept_invalid_certs = !enabled;
            adopt_file_operator_crawl_egress(&mut file, &operator);
            assert_eq!(
                file.url
                    .expect("URL configuration stays present")
                    .crawl
                    .danger_accept_invalid_certs,
                enabled
            );
        }
    }
}
