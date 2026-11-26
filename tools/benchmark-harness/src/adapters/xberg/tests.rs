use super::auto_ocr::auto_ocr_args;
use super::*;
use crate::adapter::FrameworkAdapter;

/// The harness's default `--timeout`, so these tests exercise the production default.
fn default_run_timeout() -> Duration {
    crate::config::BenchmarkConfig::default().timeout
}

fn default_extraction_timeout_secs() -> u64 {
    extraction_timeout_secs(default_run_timeout())
}

fn merged_benchmark_config(tesseract_ocr_enabled: bool) -> xberg::ExtractionConfig {
    xberg::core::config::merge::merge_config_json(
        &xberg::ExtractionConfig::default(),
        &benchmark_config_json(tesseract_ocr_enabled, default_extraction_timeout_secs()),
    )
    .unwrap()
}

fn config_json_arg(args: &[String]) -> &str {
    &args[args.iter().position(|arg| arg == "--config-json").unwrap() + 1]
}

fn emitted_extraction_timeout_secs(args: &[String]) -> Option<u64> {
    let config: serde_json::Value = serde_json::from_str(config_json_arg(args)).unwrap();
    config
        .get("extraction_timeout_secs")
        .and_then(serde_json::Value::as_u64)
}

#[test]
fn default_run_timeout_keeps_the_historical_xberg_extraction_limit() {
    assert_eq!(default_run_timeout(), std::time::Duration::from_secs(1800));
    for tesseract_ocr_enabled in [false, true] {
        let args = benchmark_base_args(
            false,
            "markdown",
            tesseract_ocr_enabled,
            default_extraction_timeout_secs(),
        );
        assert_eq!(emitted_extraction_timeout_secs(&args), Some(1740));
    }
}

#[test]
fn xberg_extraction_limit_follows_the_run_timeout() {
    let run_timeout = std::time::Duration::from_secs(7200);
    for tesseract_ocr_enabled in [false, true] {
        let args = benchmark_base_args(
            false,
            "markdown",
            tesseract_ocr_enabled,
            extraction_timeout_secs(run_timeout),
        );
        assert_eq!(emitted_extraction_timeout_secs(&args), Some(7140));
    }
    for backend in AutoOcrBackend::ALL {
        let args = auto_ocr_args(
            backend,
            OutputFormat::Markdown,
            false,
            extraction_timeout_secs(run_timeout),
        );
        assert_eq!(emitted_extraction_timeout_secs(&args), Some(7140), "{backend:?}");
    }
    let steady = steady_extraction_config(
        XbergPipeline::Baseline,
        OutputFormat::Markdown,
        false,
        XbergPdfBackend::Native,
        None,
        run_timeout,
    )
    .unwrap();
    assert_eq!(steady.extraction_timeout_secs, Some(7140));
}

#[test]
fn a_run_timeout_below_the_margin_never_yields_a_zero_xberg_limit() {
    assert_eq!(extraction_timeout_secs(std::time::Duration::from_secs(30)), 1);
}

fn create_executable_file(directory: &Path) -> PathBuf {
    let path = directory.join(if cfg!(windows) { "xberg.exe" } else { "xberg" });
    std::fs::write(&path, b"test executable").unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = path.metadata().unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&path, permissions).unwrap();
    }

    path
}

#[cfg(unix)]
#[test]
fn stale_executable_is_rejected_for_clean_checkout_build_id_mismatch() {
    use std::os::unix::fs::PermissionsExt;

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("xberg");
    std::fs::write(
        &path,
        b"#!/bin/sh\nprintf '%s\\n' '{\"name\":\"xberg-cli\",\"version\":\"1.3.7\",\"build_id\":\"2222222222222222222222222222222222222222\"}'\n",
    )
    .unwrap();
    let mut permissions = path.metadata().unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&path, permissions).unwrap();

    let actual = probe_xberg_build_id(&path).unwrap();
    assert_eq!(actual, "2222222222222222222222222222222222222222");

    let repository = crate::provenance::RepositoryProvenance {
        commit: Some("1111111111111111111111111111111111111111".to_string()),
        dirty: Some(false),
    };
    let error =
        crate::provenance::validate_xberg_build_identity(&repository, "xberg-markdown-baseline", &actual, &path)
            .unwrap_err();
    let message = error.to_string();

    assert!(message.contains("build ID mismatch"));
    assert!(message.contains("1111111111111111111111111111111111111111"));
    assert!(message.contains("2222222222222222222222222222222222222222"));
    assert!(message.contains(path.to_string_lossy().as_ref()));
}

#[test]
fn test_pipeline_baseline_str() {
    assert_eq!(XbergPipeline::Baseline.as_str(), "baseline");
}

#[test]
fn test_pipeline_layout_str() {
    assert_eq!(XbergPipeline::Layout.as_str(), "layout");
}

#[test]
fn test_pipeline_paddle_ocr_str() {
    assert_eq!(XbergPipeline::PaddleOcr.as_str(), "paddle-ocr");
}

#[test]
fn sceptre_args_select_explicit_inference_engine() {
    for (pipeline, expected_options) in [
        (XbergPipeline::SceptreOrt, SCEPTRE_ORT_OPTIONS_JSON),
        (XbergPipeline::SceptreTract, SCEPTRE_TRACT_OPTIONS_JSON),
    ] {
        let mut args = Vec::new();
        match pipeline {
            XbergPipeline::SceptreOrt => push_sceptre_args(&mut args, SCEPTRE_ORT_OPTIONS_JSON),
            XbergPipeline::SceptreTract => push_sceptre_args(&mut args, SCEPTRE_TRACT_OPTIONS_JSON),
            _ => unreachable!(),
        }
        assert_eq!(
            args.windows(2).find(|pair| pair[0] == "--ocr-backend").unwrap()[1],
            "sceptre"
        );
        assert_eq!(
            args.windows(2).find(|pair| pair[0] == "--ocr-backend-options").unwrap()[1],
            expected_options
        );
    }
}

#[test]
fn sceptre_ort_variants_enable_only_the_requested_structure_option() {
    let mut layout = Vec::new();
    push_layout_args(&mut layout);
    push_sceptre_args(&mut layout, SCEPTRE_ORT_OPTIONS_JSON);
    assert!(layout.iter().any(|arg| arg == "--layout"));
    assert!(!layout.iter().any(|arg| arg == "--ocr-auto-rotate"));

    let mut autorotate = Vec::new();
    push_sceptre_args(&mut autorotate, SCEPTRE_ORT_OPTIONS_JSON);
    autorotate.extend(["--ocr-auto-rotate".to_string(), "true".to_string()]);
    assert!(!autorotate.iter().any(|arg| arg == "--layout"));
    assert!(autorotate.iter().any(|arg| arg == "--ocr-auto-rotate"));
}

#[test]
fn test_output_format_markdown() {
    assert_eq!(OutputFormat::Markdown.to_string(), "markdown");
}

#[test]
fn test_output_format_plaintext() {
    assert_eq!(OutputFormat::Plaintext.to_string(), "plaintext");
}

#[test]
fn benchmark_config_disables_extraction_cache() {
    for tesseract_ocr_enabled in [false, true] {
        assert!(!merged_benchmark_config(tesseract_ocr_enabled).use_cache);
    }
}

#[test]
fn tesseract_benchmark_config_forces_ocr_without_pinning_psm() {
    let config = merged_benchmark_config(true);
    assert!(config.force_ocr);
    let ocr = config.ocr.unwrap();

    assert!(ocr.enabled);
    assert_eq!(ocr.backend, "tesseract");
    // As of xberg#1573, `apply_default_tesseract_psm` keys off the `psm` FIELD being
    // unset, not off `tesseract_config` presence, so a materialized struct with `psm:
    // None` would no longer pin PSM 3 either. This assertion is accordingly no longer
    // load-bearing for the auto-PSM outcome — it stays as a narrower, still-true
    // invariant: this benchmark config path does not materialize `tesseract_config` at
    // all. ~keep
    assert!(
        ocr.tesseract_config.is_none(),
        "benchmark config must not materialize tesseract_config"
    );
}

#[test]
fn native_benchmark_config_does_not_enable_ocr() {
    assert!(merged_benchmark_config(false).ocr.is_none());
}

#[test]
fn benchmark_descriptors_are_eligible_when_xberg_supports_their_extensions() {
    let supported_formats: std::collections::HashSet<String> = xberg::list_supported_formats()
        .into_iter()
        .map(|format| format.extension)
        .collect();

    for descriptor_file_type in ["asciidoc", "nxml", "vtt"] {
        assert!(
            supported_formats.contains(descriptor_file_type),
            "Xberg benchmark descriptor `{descriptor_file_type}` was filtered out"
        );
    }
}

#[test]
fn benchmark_invocations_disable_user_config_discovery() {
    for batch in [false, true] {
        let args = benchmark_base_args(batch, "markdown", false, default_extraction_timeout_secs());
        assert!(args.iter().any(|arg| arg == "--no-config-discovery"));
    }
}

#[test]
fn tesseract_benchmark_args_do_not_override_nested_config() {
    let args = benchmark_base_args(false, "markdown", true, default_extraction_timeout_secs());

    assert!(!args.iter().any(|arg| arg == "--ocr"));
    assert!(!args.iter().any(|arg| arg == "--ocr-backend"));
    assert_eq!(
        config_json_arg(&args),
        tesseract_benchmark_config_json(default_extraction_timeout_secs())
    );
}

/// The native leg must keep producing exactly today's framework name, or existing committed
/// results under `tools/benchmark-harness/results/` stop being comparable to new runs.
#[test]
fn native_pdf_backend_keeps_todays_framework_name() {
    for (batch, expected) in [
        (false, "xberg-markdown-baseline"),
        (true, "xberg-markdown-baseline-batch"),
    ] {
        assert_eq!(
            xberg_framework_name(
                XbergPipeline::Baseline,
                OutputFormat::Markdown,
                batch,
                XbergPdfBackend::Native
            ),
            expected
        );
    }
}

#[test]
fn pdfium_pdf_backend_gets_a_distinct_suffixed_framework_name() {
    for (batch, expected) in [
        (false, "xberg-markdown-baseline-pdfium"),
        (true, "xberg-markdown-baseline-pdfium-batch"),
    ] {
        assert_eq!(
            xberg_framework_name(
                XbergPipeline::Baseline,
                OutputFormat::Markdown,
                batch,
                XbergPdfBackend::Pdfium
            ),
            expected
        );
    }
    assert_eq!(XbergPdfBackend::Native.as_str(), "native");
    assert_eq!(XbergPdfBackend::Pdfium.as_str(), "pdfium");
}

#[test]
fn ocr_only_pipeline_is_rejected_when_ocr_is_disabled() {
    let error = match create_xberg_adapter(
        XbergPipeline::PaddleOcr,
        OutputFormat::Markdown,
        false,
        false,
        XbergPdfBackend::Native,
        default_run_timeout(),
    ) {
        Ok(_) => panic!("OCR-only adapter should not be created when OCR is disabled"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("requires OCR"));
}

#[test]
fn steady_baseline_and_layout_configs_match_cli_semantics() {
    let baseline = steady_extraction_config(
        XbergPipeline::Baseline,
        OutputFormat::Plaintext,
        false,
        XbergPdfBackend::Native,
        None,
        default_run_timeout(),
    )
    .unwrap();
    assert!(!baseline.use_cache);
    assert_eq!(baseline.output_format, xberg::OutputFormat::Plain);
    assert!(baseline.layout.is_none());
    assert!(!baseline.use_layout_for_markdown);

    let layout = steady_extraction_config(
        XbergPipeline::Layout,
        OutputFormat::Markdown,
        true,
        XbergPdfBackend::Native,
        Some(3),
        default_run_timeout(),
    )
    .unwrap();
    assert_eq!(layout.output_format, xberg::OutputFormat::Markdown);
    assert!(layout.layout.is_some());
    assert!(layout.use_layout_for_markdown);
    assert!(layout.force_ocr);
    assert_eq!(layout.concurrency.as_ref().and_then(|value| value.max_threads), Some(3));
    assert_eq!(layout.ocr.as_ref().map(|ocr| ocr.backend.as_str()), Some("tesseract"));
}

#[test]
fn layout_pipelines_differ_only_in_the_selected_model() {
    let mut layout = Vec::new();
    push_pipeline_args(&mut layout, XbergPipeline::Layout);
    let mut layout_v3 = Vec::new();
    push_pipeline_args(&mut layout_v3, XbergPipeline::LayoutV3);

    assert!(!layout.iter().any(|arg| arg == "--layout-model"));
    assert_eq!(layout_v3[..layout.len()], layout[..]);
    assert_eq!(layout_v3[layout.len()..], ["--layout-model", "pp_doc_layout_v3"]);
}

#[test]
fn steady_layout_configs_select_their_detector() {
    let model = |pipeline| {
        steady_extraction_config(
            pipeline,
            OutputFormat::Markdown,
            true,
            XbergPdfBackend::Native,
            None,
            default_run_timeout(),
        )
        .unwrap()
        .layout
        .map(|layout| layout.model)
    };
    assert_eq!(model(XbergPipeline::Layout), Some(xberg::LayoutDetectionModel::RtDetr));
    assert_eq!(
        model(XbergPipeline::LayoutV3),
        Some(xberg::LayoutDetectionModel::PpDocLayoutV3)
    );

    let layout_v3 = steady_extraction_config(
        XbergPipeline::LayoutV3,
        OutputFormat::Markdown,
        true,
        XbergPdfBackend::Native,
        None,
        default_run_timeout(),
    )
    .unwrap();
    assert!(layout_v3.use_layout_for_markdown);
    assert_eq!(
        layout_v3.ocr.as_ref().map(|ocr| ocr.backend.as_str()),
        Some("tesseract")
    );
}

/// Negative control: every layout-bearing adapter must refuse a document whose layout pass
/// failed, so a fallback read is never scored as layout output; baseline must not. ~keep
#[test]
fn only_layout_adapters_reject_a_failed_layout_pass() {
    let adapter = |pipeline| {
        create_xberg_steady_adapter(
            pipeline,
            OutputFormat::Markdown,
            true,
            true,
            XbergPdfBackend::Native,
            None,
            default_run_timeout(),
        )
        .unwrap()
    };
    assert!(adapter(XbergPipeline::LayoutV3).rejects_layout_failures());
    assert!(adapter(XbergPipeline::Layout).rejects_layout_failures());
    assert!(!adapter(XbergPipeline::Baseline).rejects_layout_failures());
}

#[test]
fn layout_rejection_policy_covers_exactly_the_layout_bearing_pipelines() {
    let layout_bearing = [
        XbergPipeline::Layout,
        XbergPipeline::LayoutV3,
        XbergPipeline::LayoutPaddle,
        XbergPipeline::SceptreOrtLayout,
    ];
    let others = [
        XbergPipeline::Baseline,
        XbergPipeline::PaddleOcr,
        XbergPipeline::BaselinePaddle,
        XbergPipeline::SceptreOrt,
        XbergPipeline::SceptreOrtAutoRotate,
        XbergPipeline::SceptreTract,
    ];
    for pipeline in layout_bearing {
        assert!(pipeline_runs_layout(pipeline), "{pipeline} runs layout");
        let mut args = Vec::new();
        push_pipeline_args(&mut args, pipeline);
        assert!(args.iter().any(|arg| arg == "--layout"), "{pipeline} passes --layout");
    }
    for pipeline in others {
        assert!(!pipeline_runs_layout(pipeline), "{pipeline} does not run layout");
        let mut args = Vec::new();
        push_pipeline_args(&mut args, pipeline);
        assert!(
            !args.iter().any(|arg| arg == "--layout"),
            "{pipeline} must not pass --layout"
        );
    }
}

#[test]
fn auto_ocr_variants_configure_ocr_without_forcing_it() {
    for (backend, name) in [
        (AutoOcrBackend::Tesseract, "xberg-markdown-auto-ocr"),
        (AutoOcrBackend::Paddle, "xberg-markdown-auto-ocr-paddle"),
    ] {
        assert_eq!(auto_ocr_framework_name(backend, OutputFormat::Markdown, false), name);
        assert_eq!(
            auto_ocr_framework_name(backend, OutputFormat::Markdown, true),
            format!("{name}-batch")
        );
        let args = auto_ocr_args(
            backend,
            OutputFormat::Markdown,
            false,
            default_extraction_timeout_secs(),
        );
        assert!(!args.iter().any(|arg| arg == "--force-ocr"), "{name}: {args:?}");
        let config_json = &args[args.iter().position(|arg| arg == "--config-json").unwrap() + 1];
        let config: xberg::ExtractionConfig = serde_json::from_str(config_json).unwrap();
        assert!(!config.force_ocr, "{name}");
        match backend {
            AutoOcrBackend::Tesseract => {
                let ocr = config.ocr.expect("tesseract auto-ocr enables OCR in the config");
                assert!(ocr.enabled);
                assert_eq!(ocr.backend, "tesseract");
            }
            AutoOcrBackend::Paddle => {
                assert!(args.windows(2).any(|pair| pair == ["--ocr", "true"]));
                assert!(args.windows(2).any(|pair| pair == ["--ocr-backend", "paddle-ocr"]));
            }
        }
    }
}

#[test]
fn forced_ocr_variants_still_force_ocr() {
    let forces_ocr = |pipeline| {
        let mut args = Vec::new();
        push_pipeline_args(&mut args, pipeline);
        args.windows(2).any(|pair| pair == ["--force-ocr", "true"])
    };
    for pipeline in [
        XbergPipeline::BaselinePaddle,
        XbergPipeline::LayoutPaddle,
        XbergPipeline::SceptreOrt,
        XbergPipeline::SceptreOrtLayout,
    ] {
        assert!(forces_ocr(pipeline), "{pipeline} must keep forcing OCR");
    }
    assert!(
        merged_benchmark_config(true).force_ocr,
        "the tesseract baseline forces OCR"
    );
}

#[test]
fn auto_ocr_adapter_is_rejected_when_ocr_is_disabled() {
    let error = match create_xberg_auto_ocr_adapter(
        AutoOcrBackend::Tesseract,
        OutputFormat::Markdown,
        false,
        false,
        default_run_timeout(),
    ) {
        Ok(_) => panic!("auto-ocr adapter should not be created when OCR is disabled"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("requires OCR"));
}

#[test]
fn steady_adapter_rejects_unimplemented_pipeline() {
    let error = match create_xberg_steady_adapter(
        XbergPipeline::SceptreOrt,
        OutputFormat::Markdown,
        false,
        true,
        XbergPdfBackend::Native,
        None,
        default_run_timeout(),
    ) {
        Ok(_) => panic!("unsupported steady pipeline must fail closed"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("does not yet support"));
}

#[test]
fn warm_batch_adapter_has_no_misleading_single_file_cold_probe() {
    let adapter = create_xberg_steady_adapter(
        XbergPipeline::Baseline,
        OutputFormat::Markdown,
        true,
        false,
        XbergPdfBackend::Native,
        None,
        default_run_timeout(),
    )
    .unwrap();

    assert_eq!(adapter.name(), "xberg-markdown-baseline-steady-state-batch");
    assert!(!adapter.has_separate_cold_probe());
}

#[test]
fn steady_and_cold_probe_share_forced_tesseract_config() {
    let steady = steady_extraction_config(
        XbergPipeline::Layout,
        OutputFormat::Markdown,
        true,
        XbergPdfBackend::Native,
        Some(4),
        default_run_timeout(),
    )
    .unwrap();
    let cold: xberg::ExtractionConfig =
        serde_json::from_str(&benchmark_config_json(true, default_extraction_timeout_secs())).unwrap();

    assert!(steady.force_ocr);
    assert!(cold.force_ocr);
    assert_eq!(steady.ocr.as_ref().map(|ocr| ocr.backend.as_str()), Some("tesseract"));
    assert_eq!(cold.ocr.as_ref().map(|ocr| ocr.backend.as_str()), Some("tesseract"));
    let steady_ocr = steady.ocr.as_ref().expect("warm in-process OCR config");
    assert!(
        steady_ocr.tesseract_config.is_none() && steady_ocr.backend_options.is_none(),
        "either field would switch xberg off its default OCR route"
    );
    assert_eq!(steady_ocr.language, vec!["eng".to_string()]);
    assert!(
        !steady.use_cache && !cold.use_cache,
        "request-level use_cache=false keeps OCR cold"
    );
    assert_eq!(steady.concurrency.as_ref().and_then(|value| value.max_threads), Some(4));
}

#[test]
fn explicit_binary_override_takes_precedence() {
    let directory = tempfile::tempdir().unwrap();
    let path = create_executable_file(directory.path());

    let located = locate_xberg_cli_with_override(Some(path.as_os_str())).unwrap();

    assert_eq!(located, path);
}

#[test]
fn explicit_binary_override_rejects_missing_path_with_context() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("missing-xberg");

    let error = locate_xberg_cli_with_override(Some(path.as_os_str())).unwrap_err();
    let message = error.to_string();

    assert!(message.contains(XBERG_CLI_BINARY_ENV_VAR));
    assert!(message.contains(path.to_string_lossy().as_ref()));
    assert!(message.contains("metadata could not be read"));
}

#[test]
fn explicit_binary_override_rejects_directory() {
    let directory = tempfile::tempdir().unwrap();

    let error = locate_xberg_cli_with_override(Some(directory.path().as_os_str())).unwrap_err();
    let message = error.to_string();

    assert!(message.contains(XBERG_CLI_BINARY_ENV_VAR));
    assert!(message.contains(directory.path().to_string_lossy().as_ref()));
    assert!(message.contains("is not a file"));
}

#[cfg(unix)]
#[test]
fn explicit_binary_override_rejects_file_without_execute_permission() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("xberg");
    std::fs::write(&path, b"not executable").unwrap();

    let error = locate_xberg_cli_with_override(Some(path.as_os_str())).unwrap_err();
    let message = error.to_string();

    assert!(message.contains(XBERG_CLI_BINARY_ENV_VAR));
    assert!(message.contains(path.to_string_lossy().as_ref()));
    assert!(message.contains("has no execute permission"));
}

#[cfg(windows)]
#[test]
fn explicit_binary_override_rejects_file_without_executable_extension() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("xberg.txt");
    std::fs::write(&path, b"not executable").unwrap();

    let error = locate_xberg_cli_with_override(Some(path.as_os_str())).unwrap_err();
    let message = error.to_string();

    assert!(message.contains(XBERG_CLI_BINARY_ENV_VAR));
    assert!(message.contains(path.to_string_lossy().as_ref()));
    assert!(message.contains("has no executable extension"));
}
