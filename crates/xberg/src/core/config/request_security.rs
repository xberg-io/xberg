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

/// A setting outside an LLM stage that decides whether the stage runs. A caller who resends the
/// operator's stage unchanged must leave these as the operator has them too, or it could wake a
/// stage the operator configured but left dormant.
#[derive(Clone, Copy)]
enum StageSelector {
    /// The value at this JSON pointer must equal the operator's.
    Equal(&'static str),
    /// Whether this JSON pointer holds a value must match the operator's.
    Present(&'static str),
}

impl StageSelector {
    fn matches(self, operator: &Value, run: &Value) -> bool {
        fn value<'a>(config: &'a Value, pointer: &str) -> Option<&'a Value> {
            config.pointer(pointer).filter(|value| !value.is_null())
        }
        match self {
            Self::Equal(pointer) => value(operator, pointer) == value(run, pointer),
            Self::Present(pointer) => value(operator, pointer).is_some() == value(run, pointer).is_some(),
        }
    }
}

/// The post-processor stages (NER, summarization, translation, page and chunk classification,
/// captioning) run only when `postprocessor` leaves their processor enabled.
const POSTPROCESSOR: StageSelector = StageSelector::Equal("/postprocessor");

/// What decides whether the stage at each [`LLM_LOCATIONS`] pointer runs, traced from its dispatch:
/// - `ocr.vlm_config`: the `vlm` backend or a `vlm_fallback` policy uses it, and only while no
///   explicit `ocr.pipeline` replaces them (`OcrConfig::effective_pipeline`); `disable_ocr` and its
///   `ocr.enabled` shorthand turn OCR off (`ExtractionConfig::effective_disable_ocr`).
/// - `ner.llm`: only the `llm` backend uses it (`NerProcessor`).
/// - `summarization.llm`: the `extractive` strategy never calls it (`SummarizationProcessor`).
/// - `chunk_classification.llm`: classifies chunks, which exist only with `chunking`.
/// - `captioning.llm`: captions extracted images, which exist only with `images.extract_images`.
/// - `structured_extraction.llm`, `chunking.embedding.model.llm`, `translation.llm` and
///   `page_classification.llm` run whenever their section is set.
const LLM_STAGE_SELECTORS: &[(&str, &[StageSelector])] = &[
    ("/structured_extraction/llm", &[]),
    (
        "/ocr/vlm_config",
        &[
            StageSelector::Equal("/ocr/backend"),
            StageSelector::Equal("/ocr/vlm_fallback"),
            StageSelector::Present("/ocr/pipeline"),
            StageSelector::Equal("/ocr/enabled"),
            StageSelector::Equal("/disable_ocr"),
        ],
    ),
    ("/chunking/embedding/model/llm", &[]),
    ("/ner/llm", &[StageSelector::Equal("/ner/backend"), POSTPROCESSOR]),
    (
        "/summarization/llm",
        &[StageSelector::Equal("/summarization/strategy"), POSTPROCESSOR],
    ),
    ("/translation/llm", &[POSTPROCESSOR]),
    ("/page_classification/llm", &[POSTPROCESSOR]),
    (
        "/chunk_classification/llm",
        &[StageSelector::Present("/chunking"), POSTPROCESSOR],
    ),
    (
        "/captioning/llm",
        &[StageSelector::Equal("/images/extract_images"), POSTPROCESSOR],
    ),
];

/// An `ocr.pipeline.stages[i].vlm_config` runs when its stage's backend is `vlm` and the pipeline's
/// order and quality thresholds reach it, so the whole pipeline must be the operator's.
const OCR_PIPELINE_STAGE_SELECTORS: &[StageSelector] = &[
    StageSelector::Equal("/ocr/pipeline"),
    StageSelector::Equal("/ocr/enabled"),
    StageSelector::Equal("/disable_ocr"),
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
            check_caller_tessdata_model(operator, caller)?;
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
    config.tessdata_model = operator.tessdata_model;
    adopt_operator_tessdata_model(config.ocr.as_mut(), operator.ocr.as_ref());
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
    adopt_operator_tessdata_model(config.ocr.as_mut(), effective.ocr.as_ref());
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

/// Reject a caller `tessdata_model` or `ocr.tessdata_model` that differs from the set the
/// operator's config runs: the set decides what the server downloads and caches. Restating the
/// operator's set is allowed, so a client that serializes a whole config is not rejected. A value
/// that is not a set name is left to config deserialization, which reports it as invalid.
pub(crate) fn check_caller_tessdata_model(operator: &super::ExtractionConfig, caller: &Value) -> Result<(), String> {
    let operator_model = operator.effective_tessdata_model();
    for (pointer, path) in [
        ("/tessdata_model", "tessdata_model"),
        ("/ocr/tessdata_model", "ocr.tessdata_model"),
    ] {
        let Some(value) = caller.pointer(pointer).filter(|value| !value.is_null()) else {
            continue;
        };
        if let Ok(requested) = serde_json::from_value::<super::TessdataModel>(value.clone())
            && requested != operator_model
        {
            return Err(format!("Caller extraction config may not set {path}"));
        }
    }
    Ok(())
}

/// A caller `ocr` object replaces the operator's, so restore the operator's Tesseract model set:
/// it selects what the server downloads and caches. `None` inherits the top-level set.
fn adopt_operator_tessdata_model(ocr: Option<&mut super::OcrConfig>, operator: Option<&super::OcrConfig>) {
    if let Some(ocr) = ocr {
        ocr.tessdata_model = operator.and_then(|operator| operator.tessdata_model);
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

/// Reject a remote caller's per-input override that adds or changes an LLM stage, or a setting
/// that decides whether one runs, relative to the request's effective config, unless the operator
/// set [`ALLOW_CALLER_LLM_ENV_CREDENTIALS_ENV`]. `input` is the override as it will run.
pub(crate) fn reject_caller_input_llm_stages(
    effective: &super::ExtractionConfig,
    input: &super::FileExtractionConfig,
    allow_env_credentials: bool,
) -> Result<(), String> {
    let effective = config_json(effective)?;
    let run = with_input_overrides(&effective, &config_json(input)?);
    check_caller_llm_stages(&effective, &run, allow_env_credentials)
}

/// `effective` with each section the per-input override sets replaced by the override's, as
/// `ExtractionConfig::with_file_overrides` applies it.
fn with_input_overrides(effective: &Value, input: &Value) -> Value {
    let mut run = effective.clone();
    if let (Some(run), Some(input)) = (run.as_object_mut(), input.as_object()) {
        for (key, value) in input.iter().filter(|(_, value)| !value.is_null()) {
            run.insert(key.clone(), value.clone());
        }
    }
    run
}

fn config_json(config: &impl serde::Serialize) -> Result<Value, String> {
    serde_json::to_value(config).map_err(|e| format!("Failed to serialize extraction config: {e}"))
}

/// Every LLM stage of the configuration that will run (`run`) must be identical, field for field,
/// to the stage the trusted configuration (`operator`) has at the same place, and so must the
/// settings that decide whether it runs ([`LLM_STAGE_SELECTORS`]): then the caller changed nothing
/// and the stage runs only as the operator configured it. A new, changed or newly enabled stage
/// could spend the server's credentials on the caller's terms, so it needs the operator's opt-in.
fn check_caller_llm_stages(operator: &Value, run: &Value, allow_env_credentials: bool) -> Result<(), String> {
    if allow_env_credentials {
        return Ok(());
    }
    for (pointer, path, selectors) in caller_llm_stages(run) {
        let unchanged = run.pointer(&pointer) == operator.pointer(&pointer)
            && selectors.iter().all(|selector| selector.matches(operator, run));
        if !unchanged {
            return Err(format!(
                "Caller extraction config may not use the server's LLM credentials for {path}; \
                 the server operator must set {ALLOW_CALLER_LLM_ENV_CREDENTIALS_ENV}=1 to allow it"
            ));
        }
    }
    Ok(())
}

/// JSON pointer, dotted path and run selectors of every non-null LLM config in `config`.
fn caller_llm_stages(config: &Value) -> Vec<(String, String, &'static [StageSelector])> {
    let mut stages: Vec<(String, String, &'static [StageSelector])> = LLM_LOCATIONS
        .iter()
        .filter(|(pointer, _)| config.pointer(pointer).is_some_and(|llm| !llm.is_null()))
        .map(|(pointer, path)| ((*pointer).to_string(), (*path).to_string(), stage_selectors(pointer)))
        .collect();
    if let Some(pipeline) = config.pointer("/ocr/pipeline/stages").and_then(Value::as_array) {
        for (index, stage) in pipeline.iter().enumerate() {
            if stage.get("vlm_config").is_some_and(|llm| !llm.is_null()) {
                stages.push((
                    format!("/ocr/pipeline/stages/{index}/vlm_config"),
                    format!("ocr.pipeline.stages[{index}].vlm_config"),
                    OCR_PIPELINE_STAGE_SELECTORS,
                ));
            }
        }
    }
    stages
}

fn stage_selectors(pointer: &str) -> &'static [StageSelector] {
    LLM_STAGE_SELECTORS
        .iter()
        .find(|(location, _)| *location == pointer)
        .map_or(&[][..], |&(_, selectors)| selectors)
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
mod tests;
