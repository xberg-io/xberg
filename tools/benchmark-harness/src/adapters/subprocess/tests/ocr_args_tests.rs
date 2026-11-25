//! Tests for OCR-argument construction: Tesseract file-config overrides, per-file batch
//! configs, and the CLI/xberg force-OCR argument upgrades.

use std::path::Path;

use super::super::SubprocessAdapter;
use super::super::ocr_args::{
    apply_tesseract_ocr_override_to_args, build_batch_file_configs, effective_ocr_config_from_args,
    xberg_ocr_language_args,
};

#[test]
fn forced_ocr_upgrades_external_no_ocr_flag() {
    let adapter = SubprocessAdapter::new(
        "docling",
        "echo",
        vec!["--no-ocr".to_string(), "sync".to_string()],
        vec![],
        vec!["pdf".to_string()],
    );

    assert_eq!(adapter.request_args(false)[0], "--no-ocr");
    assert_eq!(adapter.request_args(true)[0], "--ocr");
}

#[test]
fn tesseract_file_config_preserves_effective_backend_and_cache_settings() {
    let args = vec![
        "--config-json".to_string(),
        r#"{"ocr":{"enabled":true,"backend":"tesseract","tesseract_config":{"use_cache":false}}}"#.to_string(),
    ];
    let base_ocr = effective_ocr_config_from_args(&args).expect("Tesseract OCR config");
    let cwd = tempfile::tempdir().unwrap();
    let input = Path::new("sample.pdf");
    let configs = build_batch_file_configs(
        &[input],
        &[Some(" deu + eng ".to_string())],
        cwd.path(),
        Some(&base_ocr),
    );
    let config = configs
        .get(&cwd.path().join(input).to_string_lossy().into_owned())
        .expect("file config");

    assert_eq!(
        config.pointer("/ocr/backend").and_then(serde_json::Value::as_str),
        Some("tesseract")
    );
    assert_eq!(
        config
            .pointer("/ocr/tesseract_config/use_cache")
            .and_then(serde_json::Value::as_bool),
        Some(false)
    );
    assert_eq!(
        config.pointer("/ocr/language"),
        Some(&serde_json::json!(["deu", "eng"]))
    );
    assert_eq!(
        config.pointer("/ocr/tesseract_config/language"),
        Some(&serde_json::json!(["deu", "eng"]))
    );
}

#[test]
fn tesseract_file_config_updates_pipeline_stage_languages() {
    let config = serde_json::json!({
        "ocr": {
            "enabled": true,
            "backend": "tesseract",
            "pipeline": {
                "stages": [
                    {
                        "backend": "tesseract",
                        "language": ["eng"],
                        "tesseract_config": {"language": ["eng"], "use_cache": false}
                    },
                    {"backend": "paddle-ocr", "language": ["en"]}
                ]
            }
        }
    });
    let args = vec!["--config-json".to_string(), config.to_string()];
    let base_ocr = effective_ocr_config_from_args(&args).expect("Tesseract OCR config");
    let cwd = tempfile::tempdir().unwrap();
    let input = Path::new("sample.pdf");
    let configs = build_batch_file_configs(&[input], &[Some("deu".to_string())], cwd.path(), Some(&base_ocr));
    let config = configs
        .get(&cwd.path().join(input).to_string_lossy().into_owned())
        .expect("file config");

    assert_eq!(
        config.pointer("/ocr/pipeline/stages/0/language"),
        Some(&serde_json::json!(["deu"]))
    );
    assert_eq!(
        config.pointer("/ocr/pipeline/stages/0/tesseract_config/language"),
        Some(&serde_json::json!(["deu"]))
    );
    assert_eq!(
        config.pointer("/ocr/pipeline/stages/1"),
        Some(&serde_json::json!({
            "backend": "paddle-ocr",
            "language": ["en"]
        }))
    );
}

#[test]
fn tesseract_file_config_preserves_implicit_psm_without_fixture_language() {
    let base_ocr = serde_json::json!({"enabled": true, "backend": "tesseract"});
    let cwd = tempfile::tempdir().unwrap();
    let input = Path::new("sample.pdf");
    let configs = build_batch_file_configs(&[input], &[None], cwd.path(), Some(&base_ocr));
    let config = configs
        .get(&cwd.path().join(input).to_string_lossy().into_owned())
        .expect("every Tesseract fixture must get a per-file override, even without a fixture language");

    assert_eq!(config.pointer("/ocr/language"), Some(&serde_json::json!(["eng"])));
    assert_eq!(
        config
            .pointer("/ocr/backend_options/use_cache")
            .and_then(serde_json::Value::as_bool),
        Some(false)
    );
    assert_eq!(config.pointer("/ocr/tesseract_config"), None);
}

#[test]
fn tesseract_file_config_preserves_implicit_vertical_psm_selection() {
    let base_ocr = serde_json::json!({"enabled": true, "backend": "tesseract"});
    let cwd = tempfile::tempdir().unwrap();
    let input = Path::new("sample.jpeg");
    let configs = build_batch_file_configs(&[input], &[Some("jpn_vert".to_string())], cwd.path(), Some(&base_ocr));
    let config = configs
        .get(&cwd.path().join(input).to_string_lossy().into_owned())
        .expect("file config");

    assert_eq!(config.pointer("/ocr/language"), Some(&serde_json::json!(["jpn_vert"])));
    assert_eq!(
        config
            .pointer("/ocr/backend_options/use_cache")
            .and_then(serde_json::Value::as_bool),
        Some(false)
    );
    assert_eq!(config.pointer("/ocr/tesseract_config"), None);
}

#[test]
fn non_tesseract_file_config_without_fixture_language_produces_no_override() {
    let base_ocr = serde_json::json!({"enabled": true, "backend": "paddle-ocr"});
    let cwd = tempfile::tempdir().unwrap();
    let configs = build_batch_file_configs(&[Path::new("sample.pdf")], &[None], cwd.path(), Some(&base_ocr));

    assert!(
        configs.is_empty(),
        "a non-Tesseract fixture without an explicit language needs no per-file override"
    );
}

#[test]
fn single_file_tesseract_override_preserves_implicit_psm_without_fixture_language() {
    let args = vec![
        "--config-json".to_string(),
        r#"{"ocr":{"enabled":true,"backend":"tesseract"}}"#.to_string(),
    ];

    let rewritten = apply_tesseract_ocr_override_to_args(&args, None).expect("tesseract config to rewrite");
    let config: serde_json::Value = serde_json::from_str(&rewritten[1]).unwrap();

    assert_eq!(config.pointer("/ocr/language"), Some(&serde_json::json!(["eng"])));
    assert_eq!(
        config
            .pointer("/ocr/backend_options/use_cache")
            .and_then(serde_json::Value::as_bool),
        Some(false)
    );
    assert_eq!(config.pointer("/ocr/tesseract_config"), None);
}

#[test]
fn single_file_tesseract_override_preserves_implicit_vertical_psm_selection() {
    let args = vec![
        "--config-json".to_string(),
        r#"{"ocr":{"enabled":true,"backend":"tesseract"}}"#.to_string(),
    ];

    let rewritten = apply_tesseract_ocr_override_to_args(&args, Some("jpn_vert")).expect("tesseract config to rewrite");
    let config: serde_json::Value = serde_json::from_str(&rewritten[1]).unwrap();

    assert_eq!(config.pointer("/ocr/language"), Some(&serde_json::json!(["jpn_vert"])));
    assert_eq!(
        config
            .pointer("/ocr/backend_options/use_cache")
            .and_then(serde_json::Value::as_bool),
        Some(false)
    );
    assert_eq!(config.pointer("/ocr/tesseract_config"), None);
}

#[test]
fn single_file_tesseract_override_preserves_explicit_psm() {
    let args = vec![
            "--config-json".to_string(),
            r#"{"ocr":{"enabled":true,"backend":"tesseract","backend_options":{"use_cache":true},"tesseract_config":{"psm":6}}}"#.to_string(),
        ];

    let rewritten = apply_tesseract_ocr_override_to_args(&args, Some("jpn_vert")).expect("tesseract config to rewrite");
    let config: serde_json::Value = serde_json::from_str(&rewritten[1]).unwrap();

    assert_eq!(
        config
            .pointer("/ocr/tesseract_config/psm")
            .and_then(serde_json::Value::as_i64),
        Some(6),
        "an explicit PSM must survive a language update untouched"
    );
    assert_eq!(
        config
            .pointer("/ocr/tesseract_config/use_cache")
            .and_then(serde_json::Value::as_bool),
        Some(false)
    );
    assert_eq!(
        config
            .pointer("/ocr/backend_options/use_cache")
            .and_then(serde_json::Value::as_bool),
        Some(false)
    );
}

#[test]
fn single_file_tesseract_override_is_none_for_non_tesseract_backend() {
    let args = vec![
        "--config-json".to_string(),
        r#"{"ocr":{"enabled":true,"backend":"paddle-ocr"}}"#.to_string(),
    ];

    assert_eq!(apply_tesseract_ocr_override_to_args(&args, Some("deu")), None);
}

#[test]
fn single_file_tesseract_override_synthesizes_config_json_for_cli_only_ocr() {
    // BLOCKER 1 regression: OCR enabled purely via `--ocr true` (e.g. `request_args_from`'s
    // force-OCR upgrade path for an adapter whose base `--config-json` carries no `ocr` key
    // at all — see `forced_ocr_adds_cli_ocr_for_native_xberg_config`) must still get its
    // result cache disabled without materializing an explicit PSM, not just its language
    // forwarded via `--ocr-language`.
    let args = vec!["--ocr".to_string(), "true".to_string()];

    let rewritten =
        apply_tesseract_ocr_override_to_args(&args, Some("deu")).expect("CLI-only tesseract OCR must be rewritten");

    assert!(rewritten.iter().any(|arg| arg == "--config-json"));
    let config_index = rewritten.iter().position(|arg| arg == "--config-json").unwrap();
    let config: serde_json::Value = serde_json::from_str(&rewritten[config_index + 1]).unwrap();

    assert_eq!(config.pointer("/ocr/backend"), Some(&serde_json::json!("tesseract")));
    assert_eq!(config.pointer("/ocr/language"), Some(&serde_json::json!(["deu"])));
    assert_eq!(
        config
            .pointer("/ocr/backend_options/use_cache")
            .and_then(serde_json::Value::as_bool),
        Some(false)
    );
    assert_eq!(config.pointer("/ocr/tesseract_config"), None);
}

#[test]
fn single_file_tesseract_override_rewrites_config_json_with_no_preexisting_ocr_key() {
    // BLOCKER 1 regression: the base `--config-json` (e.g. `NATIVE_BENCHMARK_CONFIG_JSON`)
    // has no `ocr` key at all, but `--ocr true` was force-upgraded in; the existing
    // `use_cache` field must survive the rewrite alongside the new materialized `ocr` key.
    let args = vec![
        "--config-json".to_string(),
        r#"{"extraction_timeout_secs":1740,"use_cache":false}"#.to_string(),
        "--ocr".to_string(),
        "true".to_string(),
        "--force-ocr".to_string(),
        "true".to_string(),
    ];

    let rewritten = apply_tesseract_ocr_override_to_args(&args, None).expect("CLI-only tesseract OCR to rewrite");
    let config_index = rewritten.iter().position(|arg| arg == "--config-json").unwrap();
    let config: serde_json::Value = serde_json::from_str(&rewritten[config_index + 1]).unwrap();

    assert_eq!(config.pointer("/use_cache"), Some(&serde_json::json!(false)));
    assert_eq!(config.pointer("/ocr/backend"), Some(&serde_json::json!("tesseract")));
    assert_eq!(config.pointer("/ocr/language"), Some(&serde_json::json!(["eng"])));
    assert_eq!(
        config
            .pointer("/ocr/backend_options/use_cache")
            .and_then(serde_json::Value::as_bool),
        Some(false)
    );
    assert_eq!(config.pointer("/ocr/tesseract_config"), None);
}

#[test]
fn forced_ocr_without_nested_config_keeps_tesseract_config_implicit() {
    let args = vec![
        "--config-json".to_string(),
        r#"{"use_cache":false}"#.to_string(),
        "--ocr".to_string(),
        "--force-ocr".to_string(),
        "true".to_string(),
    ];
    let ocr = effective_ocr_config_from_args(&args).expect("forced Tesseract OCR config");

    assert_eq!(ocr.pointer("/enabled"), Some(&serde_json::json!(true)));
    assert_eq!(ocr.pointer("/backend"), Some(&serde_json::json!("tesseract")));
    // `tesseract_config` must stay absent here: this raw config feeds `build_batch_file_configs`
    // (via `ocr_uses_tesseract` + `materialize_tesseract_ocr`), where cache control is added
    // through backend options without defeating xberg's own auto-PSM selection.
    assert_eq!(ocr.pointer("/tesseract_config"), None);
}

#[test]
fn paddle_file_config_receives_fixture_language_without_tesseract_config() {
    let args = vec![
        "--config-json".to_string(),
        r#"{"ocr":{"enabled":true,"backend":"tesseract"}}"#.to_string(),
        "--ocr-backend".to_string(),
        "paddle-ocr".to_string(),
    ];

    let base_ocr = effective_ocr_config_from_args(&args).expect("Paddle OCR config");
    let configs = build_batch_file_configs(
        &[Path::new("sample.pdf")],
        &[Some(" deu + eng ".to_string())],
        Path::new("/tmp"),
        Some(&base_ocr),
    );
    let config = configs.get("/tmp/sample.pdf").expect("file config");

    assert_eq!(config.pointer("/ocr/backend"), Some(&serde_json::json!("paddle-ocr")));
    assert_eq!(
        config.pointer("/ocr/language"),
        Some(&serde_json::json!(["deu", "eng"]))
    );
    assert_eq!(config.pointer("/ocr/tesseract_config"), None);
}

#[test]
fn paddle_single_file_receives_fixture_language() {
    let args = vec![
        "--ocr".to_string(),
        "true".to_string(),
        "--ocr-backend".to_string(),
        "paddle-ocr".to_string(),
    ];

    assert_eq!(
        xberg_ocr_language_args(&args, Some(" deu + eng ")),
        Some(["--ocr-language".to_string(), "deu+eng".to_string()])
    );
}

#[test]
fn tesseract_single_file_language_forwarding_is_unchanged() {
    let args = vec![
        "--ocr".to_string(),
        "true".to_string(),
        "--ocr-backend".to_string(),
        "tesseract".to_string(),
    ];

    assert_eq!(
        xberg_ocr_language_args(&args, Some("jpn_vert")),
        Some(["--ocr-language".to_string(), "jpn_vert".to_string()])
    );
    // The raw CLI-only synthesis leaves `tesseract_config` absent. The later cache override
    // writes through `backend_options`, preserving the same implicit-PSM behavior.
    let ocr = effective_ocr_config_from_args(&args).expect("Tesseract OCR config");
    assert_eq!(ocr.pointer("/tesseract_config"), None);
}

#[test]
fn forced_ocr_upgrades_xberg_boolean_args() {
    let adapter = SubprocessAdapter::new(
        "xberg-markdown-baseline",
        "echo",
        vec!["--ocr".to_string(), "false".to_string()],
        vec![],
        vec!["pdf".to_string()],
    );

    let args = adapter.request_args(true);
    assert_eq!(&args[..2], ["--ocr", "true"]);
    assert!(args.windows(2).any(|pair| pair == ["--force-ocr", "true"]));
}

#[test]
fn forced_ocr_preserves_enabled_xberg_json_config_without_cli_ocr_override() {
    let config =
        r#"{"use_cache":false,"ocr":{"enabled":true,"backend":"tesseract","tesseract_config":{"use_cache":false}}}"#;
    let adapter = SubprocessAdapter::new(
        "xberg-markdown-baseline",
        "echo",
        vec!["--config-json".to_string(), config.to_string()],
        vec![],
        vec!["pdf".to_string()],
    );

    let args = adapter.request_args(true);
    assert_eq!(args[1], config);
    assert!(!args.iter().any(|arg| arg == "--ocr"));
    assert!(args.windows(2).any(|pair| pair == ["--force-ocr", "true"]));
}

#[test]
fn forced_ocr_adds_force_flag_to_xberg_json_ocr_config() {
    let adapter = SubprocessAdapter::new(
        "xberg-markdown-baseline",
        "echo",
        vec!["--config-json".to_string(), r#"{"ocr":{"enabled":true}}"#.to_string()],
        vec![],
        vec!["pdf".to_string()],
    );

    let args = adapter.request_args(true);
    assert!(args.windows(2).any(|pair| pair == ["--force-ocr", "true"]));
}

#[test]
fn forced_ocr_upgrades_explicit_xberg_cli_override_despite_json_config() {
    let adapter = SubprocessAdapter::new(
        "xberg-markdown-baseline",
        "echo",
        vec![
            "--config-json".to_string(),
            r#"{"ocr":{"enabled":true}}"#.to_string(),
            "--ocr".to_string(),
            "false".to_string(),
        ],
        vec![],
        vec!["pdf".to_string()],
    );

    let args = adapter.request_args(true);
    let ocr_index = args.iter().position(|arg| arg == "--ocr").unwrap();
    assert_eq!(args[ocr_index + 1], "true");
    assert!(args.windows(2).any(|pair| pair == ["--force-ocr", "true"]));
}

#[test]
fn forced_ocr_adds_cli_ocr_for_native_xberg_config() {
    let adapter = SubprocessAdapter::new(
        "xberg-markdown-baseline",
        "echo",
        vec!["--config-json".to_string(), r#"{"use_cache":false}"#.to_string()],
        vec![],
        vec!["pdf".to_string()],
    );

    let args = adapter.request_args(true);
    assert!(args.iter().any(|arg| arg == "--ocr"));
    assert!(args.windows(2).any(|pair| pair == ["--force-ocr", "true"]));
}

#[test]
fn forced_ocr_uses_existing_behavior_for_malformed_xberg_config_json() {
    let adapter = SubprocessAdapter::new(
        "xberg-markdown-baseline",
        "echo",
        vec!["--config-json".to_string(), "{malformed".to_string()],
        vec![],
        vec!["pdf".to_string()],
    );

    let args = adapter.request_args(true);
    assert!(args.iter().any(|arg| arg == "--ocr"));
    assert!(args.windows(2).any(|pair| pair == ["--force-ocr", "true"]));
}

mod auto_ocr_force_ocr {
    use super::super::support::test_build_identity;
    use crate::adapters::subprocess::SubprocessAdapter;
    use crate::adapters::xberg::{AutoOcrBackend, adapter_with_identity, auto_ocr_adapter_with_identity};
    use crate::types::{OcrStatus, OutputFormat, XbergPdfBackend, XbergPipeline};

    fn auto_adapter(backend: AutoOcrBackend, batch: bool) -> SubprocessAdapter {
        auto_ocr_adapter_with_identity(backend, OutputFormat::Markdown, batch, test_build_identity()).unwrap()
    }

    fn forces_ocr(args: &[String]) -> bool {
        args.iter().any(|arg| arg == "--force-ocr")
    }

    /// A fixture that requires OCR must not turn an auto-OCR variant into a forced one, in the
    /// single-file, batch and single-file-fallback request paths. ~keep
    #[test]
    fn auto_ocr_adapters_never_force_ocr_for_ocr_fixtures() {
        for backend in AutoOcrBackend::ALL {
            for batch in [false, true] {
                let adapter = auto_adapter(backend, batch);
                let name = adapter.name.clone();
                assert!(!forces_ocr(&adapter.request_args(true)), "{name} request_args");
                assert!(
                    !forces_ocr(&adapter.single_file_request_args(true)),
                    "{name} single_file_request_args"
                );
                assert_eq!(
                    adapter.request_args(true),
                    adapter.request_args(false),
                    "{name}: the fixture's OCR requirement must not change the request"
                );
            }
        }
    }

    #[test]
    fn auto_ocr_adapter_does_not_claim_ocr_ran_when_the_cli_reports_nothing() {
        let adapter = auto_adapter(AutoOcrBackend::Tesseract, false);
        assert_eq!(adapter.resolve_ocr_status(None, true), OcrStatus::Unknown);
        assert_eq!(
            adapter.resolve_ocr_status(Some(&serde_json::json!(false)), true),
            OcrStatus::NotUsed
        );
    }

    #[test]
    fn forced_baseline_adapters_still_force_ocr_for_ocr_fixtures() {
        for batch in [false, true] {
            for pipeline in [XbergPipeline::Baseline, XbergPipeline::BaselinePaddle] {
                let adapter = adapter_with_identity(
                    pipeline,
                    OutputFormat::Markdown,
                    batch,
                    true,
                    XbergPdfBackend::Native,
                    test_build_identity(),
                )
                .unwrap();
                let name = adapter.name.clone();
                assert!(forces_ocr(&adapter.request_args(true)), "{name} request_args");
                assert!(
                    forces_ocr(&adapter.single_file_request_args(true)),
                    "{name} single_file_request_args"
                );
            }
        }
    }
}
