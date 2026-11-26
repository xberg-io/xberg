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
fn should_list_run_selectors_for_every_llm_location() {
    let locations: Vec<&str> = LLM_LOCATIONS.iter().map(|(pointer, _)| *pointer).collect();
    let selectors: Vec<&str> = LLM_STAGE_SELECTORS.iter().map(|(pointer, _)| *pointer).collect();
    assert_eq!(locations, selectors);
}

#[test]
fn should_reject_operator_stage_woken_by_a_changed_selector() {
    let llm = serde_json::json!({"model": "openai/gpt-4o-mini"});
    let cases = [
        (
            serde_json::json!({"ocr": {"backend": "tesseract", "vlm_config": llm}}),
            serde_json::json!({"ocr": {"backend": "vlm", "vlm_config": llm}}),
            "ocr.vlm_config",
        ),
        (
            serde_json::json!({"ocr": {"vlm_config": llm, "vlm_fallback": {"mode": "disabled"}}}),
            serde_json::json!({"ocr": {"vlm_config": llm, "vlm_fallback": {"mode": "always"}}}),
            "ocr.vlm_config",
        ),
        (
            serde_json::json!({"ocr": {"backend": "vlm", "vlm_config": llm, "pipeline": {"stages": []}}}),
            serde_json::json!({"ocr": {"backend": "vlm", "vlm_config": llm}}),
            "ocr.vlm_config",
        ),
        (
            serde_json::json!({"disable_ocr": true, "ocr": {"backend": "vlm", "vlm_config": llm}}),
            serde_json::json!({"disable_ocr": false, "ocr": {"backend": "vlm", "vlm_config": llm}}),
            "ocr.vlm_config",
        ),
        (
            serde_json::json!({"ocr": {"enabled": false, "backend": "vlm", "vlm_config": llm}}),
            serde_json::json!({"ocr": {"enabled": true, "backend": "vlm", "vlm_config": llm}}),
            "ocr.vlm_config",
        ),
        (
            serde_json::json!({"ocr": {"pipeline": {"stages": [{"backend": "tesseract", "vlm_config": llm}]}}}),
            serde_json::json!({"ocr": {"pipeline": {"stages": [{"backend": "vlm", "vlm_config": llm}]}}}),
            "ocr.pipeline.stages[0].vlm_config",
        ),
        (
            serde_json::json!({"ner": {"backend": "onnx", "llm": llm}}),
            serde_json::json!({"ner": {"backend": "llm", "llm": llm}}),
            "ner.llm",
        ),
        (
            serde_json::json!({"summarization": {"strategy": "extractive", "llm": llm}}),
            serde_json::json!({"summarization": {"strategy": "abstractive", "llm": llm}}),
            "summarization.llm",
        ),
        (
            serde_json::json!({"chunk_classification": {"llm": llm}}),
            serde_json::json!({"chunk_classification": {"llm": llm}, "chunking": {}}),
            "chunk_classification.llm",
        ),
        (
            serde_json::json!({"captioning": {"llm": llm}}),
            serde_json::json!({"captioning": {"llm": llm}, "images": {"extract_images": true}}),
            "captioning.llm",
        ),
        (
            serde_json::json!({"translation": {"llm": llm}, "postprocessor": {"enabled": false}}),
            serde_json::json!({"translation": {"llm": llm}, "postprocessor": {"enabled": true}}),
            "translation.llm",
        ),
    ];

    for (operator, run, path) in cases {
        assert_eq!(check_caller_llm_stages(&operator, &operator, false), Ok(()), "{path}");
        assert_eq!(
            check_caller_llm_stages(&operator, &run, false),
            Err(env_credentials_error(path)),
            "{run}"
        );
        assert_eq!(check_caller_llm_stages(&operator, &run, true), Ok(()), "{path}");
    }
}

#[test]
fn should_reject_request_config_that_wakes_a_dormant_operator_vlm_stage() {
    let operator = operator_with_vlm(operator_vlm());
    let caller = serde_json::json!({"ocr": {"backend": "vlm", "vlm_config": {"model": "openai/gpt-4o-mini", "temperature": 0.2}}});

    assert_eq!(
        resolve_remote_caller_extraction_config(&operator, Some(&caller), false).map(drop),
        Err(env_credentials_error("ocr.vlm_config"))
    );
}

#[test]
fn should_reject_per_input_override_that_wakes_a_dormant_operator_stage() {
    let effective = super::super::ExtractionConfig {
        disable_ocr: true,
        ocr: Some(super::super::OcrConfig {
            backend: "vlm".to_string(),
            vlm_config: Some(operator_vlm()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let input = |disable_ocr| super::super::FileExtractionConfig {
        disable_ocr: Some(disable_ocr),
        ..Default::default()
    };

    assert_eq!(
        reject_caller_input_llm_stages(&effective, &input(false), false),
        Err(env_credentials_error("ocr.vlm_config"))
    );
    assert_eq!(reject_caller_input_llm_stages(&effective, &input(true), false), Ok(()));
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
fn should_reject_only_a_caller_tessdata_model_that_differs_from_the_operators() {
    use super::super::{OcrConfig, TessdataModel};

    let fast_operator = super::super::ExtractionConfig::default();
    let best_operator = super::super::ExtractionConfig {
        tessdata_model: Some(TessdataModel::Best),
        ..Default::default()
    };
    let in_ocr = |model: &str| serde_json::json!({"ocr": {"tessdata_model": model}});
    let top_level = |model: &str| serde_json::json!({"tessdata_model": model});
    let rejected_ocr = Err("Caller extraction config may not set ocr.tessdata_model".to_string());
    let rejected_top = Err("Caller extraction config may not set tessdata_model".to_string());

    assert_eq!(
        check_caller_tessdata_model(&fast_operator, &in_ocr("best")),
        rejected_ocr
    );
    assert_eq!(
        check_caller_tessdata_model(&best_operator, &in_ocr("fast")),
        rejected_ocr
    );
    assert_eq!(
        check_caller_tessdata_model(&fast_operator, &top_level("best")),
        rejected_top
    );
    assert_eq!(
        check_caller_tessdata_model(&best_operator, &top_level("fast")),
        rejected_top
    );
    assert_eq!(check_caller_tessdata_model(&fast_operator, &in_ocr("fast")), Ok(()));
    assert_eq!(check_caller_tessdata_model(&best_operator, &in_ocr("best")), Ok(()));
    assert_eq!(check_caller_tessdata_model(&best_operator, &top_level("best")), Ok(()));
    assert!(resolve_caller_extraction_config(&fast_operator, Some(&in_ocr("best"))).is_err());

    assert_eq!(
        check_caller_tessdata_model(&best_operator, &in_ocr("Best")),
        Ok(()),
        "an invalid value is left to deserialization"
    );
    let invalid = resolve_caller_extraction_config(&best_operator, Some(&in_ocr("Best")))
        .expect_err("an invalid set must be rejected");
    assert!(!invalid.contains("may not set"), "{invalid}");

    let serialized = serde_json::json!({"ocr": serde_json::to_value(OcrConfig::default()).unwrap()});
    assert!(
        resolve_caller_extraction_config(&fast_operator, Some(&serialized)).is_ok(),
        "a caller that serializes a default OCR config must not be rejected"
    );
    let resolved = resolve_caller_extraction_config(&best_operator, Some(&serialized)).unwrap();
    assert_eq!(
        resolved.effective_tessdata_model(),
        TessdataModel::Best,
        "an omitted set keeps the operator's"
    );
}

#[test]
fn should_keep_the_operator_section_set_when_a_caller_drops_the_ocr_section() {
    use super::super::{OcrConfig, TessdataModel};

    let operator = super::super::ExtractionConfig {
        ocr: Some(OcrConfig {
            tessdata_model: Some(TessdataModel::Best),
            ..Default::default()
        }),
        ..Default::default()
    };
    let caller = serde_json::json!({"ocr": null});

    for resolved in [
        resolve_caller_extraction_config(&operator, Some(&caller)).unwrap(),
        resolve_remote_caller_extraction_config(&operator, Some(&caller), false).unwrap(),
    ] {
        assert!(resolved.ocr.is_none(), "the caller may drop the section itself");
        assert_eq!(resolved.effective_tessdata_model(), TessdataModel::Best);
    }
}

#[test]
fn should_accept_a_caller_echoing_a_mixed_operator_config() {
    use super::super::{OcrConfig, TessdataModel};

    let operator = super::super::ExtractionConfig {
        tessdata_model: Some(TessdataModel::Best),
        ocr: Some(OcrConfig {
            tessdata_model: Some(TessdataModel::Fast),
            ..Default::default()
        }),
        ..Default::default()
    };
    let echo = serde_json::json!({"tessdata_model": "best", "ocr": {"tessdata_model": "fast"}});
    let swapped = serde_json::json!({"tessdata_model": "fast", "ocr": {"tessdata_model": "best"}});

    assert_eq!(check_caller_tessdata_model(&operator, &echo), Ok(()));
    assert!(check_caller_tessdata_model(&operator, &swapped).is_err());
    assert_eq!(
        resolve_caller_extraction_config(&operator, Some(&echo))
            .unwrap()
            .effective_tessdata_model(),
        TessdataModel::Fast
    );
}

#[test]
fn should_explain_a_per_input_top_level_tessdata_model() {
    use super::super::TessdataModel;

    let request = super::super::ExtractionConfig {
        tessdata_model: Some(TessdataModel::Best),
        ..Default::default()
    };
    let mut differing = serde_json::json!({"tessdata_model": "fast", "force_ocr": true});
    let mut restated = serde_json::json!({"tessdata_model": "best", "force_ocr": true});

    let error = strip_per_input_tessdata_model(&request, &mut differing).unwrap_err();
    assert!(
        error.starts_with("Per-input config may not set tessdata_model"),
        "{error}"
    );
    strip_per_input_tessdata_model(&request, &mut restated).unwrap();
    assert_eq!(restated, serde_json::json!({"force_ocr": true}));
    assert!(serde_json::from_value::<super::super::FileExtractionConfig>(restated).is_ok());
}

#[test]
fn should_keep_operator_tessdata_model_when_caller_sends_an_ocr_object() {
    use super::super::{OcrConfig, TessdataModel};

    let operator = super::super::ExtractionConfig {
        ocr: Some(OcrConfig {
            tessdata_model: Some(TessdataModel::Best),
            ..Default::default()
        }),
        ..Default::default()
    };
    let caller = serde_json::json!({"ocr": {"language": ["deu"]}});

    let resolved = resolve_caller_extraction_config(&operator, Some(&caller)).unwrap();
    let ocr = resolved.ocr.as_ref().unwrap();
    assert_eq!(ocr.tessdata_model, Some(TessdataModel::Best));
    assert_eq!(ocr.language, ["deu"]);

    let mut file = super::super::FileExtractionConfig {
        ocr: Some(OcrConfig::default()),
        ..Default::default()
    };
    adopt_file_operator_policy(&mut file, &resolved);
    assert_eq!(file.ocr.unwrap().tessdata_model, Some(TessdataModel::Best));

    let top_level_operator = super::super::ExtractionConfig {
        tessdata_model: Some(TessdataModel::Best),
        ..Default::default()
    };
    let mut config = super::super::ExtractionConfig {
        ocr: Some(OcrConfig::default()),
        ..Default::default()
    };
    adopt_operator_policy(&mut config, &top_level_operator);
    assert_eq!(config.tessdata_model, Some(TessdataModel::Best));
    assert_eq!(config.effective_tessdata_model(), TessdataModel::Best);

    let mut config = super::super::ExtractionConfig {
        tessdata_model: Some(TessdataModel::Best),
        ocr: Some(OcrConfig {
            tessdata_model: Some(TessdataModel::Best),
            ..Default::default()
        }),
        ..Default::default()
    };
    adopt_operator_policy(&mut config, &super::super::ExtractionConfig::default());
    assert_eq!(config.effective_tessdata_model(), TessdataModel::Fast);
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
