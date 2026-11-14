//! Xberg adapter for Wave 2 benchmark harness.
//!
//! Provides subprocess-based extraction via xberg with support for:
//! - Native, layout, PaddleOCR, Sceptre, and candle OCR pipelines
//! - Single-file and batch extraction modes
//! - JSON envelope parsing (ExtractEnvelope and BatchEnvelope)
//! - PDF backend selection (`XbergPdfBackend`): `Native` (default) or `Pdfium` (opt-in only,
//!   named explicitly via `--frameworks`; the pdfium shared library is not bundled in benchmark
//!   images yet)

use crate::{
    adapter::{ExecutableBuildIdentity, declared_ocr_language_policy},
    adapters::{native::NativeAdapter, subprocess::SubprocessAdapter},
    error::Result,
    types::{BatchCapability, BatchEntryPoint, BatchTimingScope, OutputFormat, XbergPdfBackend, XbergPipeline},
};
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};
use which::which;

const XBERG_CLI_BINARY_ENV_VAR: &str = "XBERG_CLI_BINARY";
const SCEPTRE_ORT_OPTIONS_JSON: &str = r#"{"model":{"backend":"ort"}}"#;
const SCEPTRE_TRACT_OPTIONS_JSON: &str = r#"{"model":{"backend":"tract"}}"#;

/// Environment variable that requests per-stage cold-start timing from the xberg CLI (must
/// match `crates/xberg-cli/src/commands/extract.rs::STAGE_TIMING_ENV_VAR`).
///
/// Passed unconditionally to every xberg subprocess invocation this adapter spawns. This is
/// cheap for the CLI to check (a single `std::env::var` read gated behind an `if`) and makes
/// `xberg extract --format json` include a `stage_timings` object in its stdout.
///
/// Known gap: nothing consumes that object yet. `SubprocessAdapter::parse_output`
/// (`adapters/subprocess.rs`) does not extract it, and no benchmark result type carries it, so
/// [`crate::types::StageTimings`] is currently only exercised by its own round-trip tests. The
/// data is emitted and discarded; cold-start attribution needs the parse, a result field, and
/// reporting wired together before it is available.
const STAGE_TIMING_ENV_VAR: &str = "XBERG_EMIT_STAGE_TIMING";
const NATIVE_BENCHMARK_CONFIG_JSON: &str = r#"{"extraction_timeout_secs":1740,"use_cache":false}"#;
const TESSERACT_BENCHMARK_CONFIG_JSON: &str = r#"{
    "extraction_timeout_secs":1740,
    "use_cache":false,
    "force_ocr":true,
    "ocr":{
        "enabled":true,
        "backend":"tesseract"
    }
}"#;

fn benchmark_config_json(tesseract_ocr_enabled: bool) -> &'static str {
    if tesseract_ocr_enabled {
        TESSERACT_BENCHMARK_CONFIG_JSON
    } else {
        NATIVE_BENCHMARK_CONFIG_JSON
    }
}

fn benchmark_base_args(batch: bool, content_format: &str, tesseract_ocr_enabled: bool) -> Vec<String> {
    let subcommand = if batch { "batch" } else { "extract" };
    vec![
        subcommand.to_string(),
        "--no-config-discovery".to_string(),
        "--format".to_string(),
        "json".to_string(),
        "--content-format".to_string(),
        content_format.to_string(),
        "--config-json".to_string(),
        benchmark_config_json(tesseract_ocr_enabled).to_string(),
    ]
}

fn push_layout_args(args: &mut Vec<String>) {
    args.extend([
        "--layout".to_string(),
        "true".to_string(),
        "--use-layout-for-markdown".to_string(),
    ]);
}

fn push_layout_v3_args(args: &mut Vec<String>) {
    push_layout_args(args);
    args.extend([
        "--layout-model".to_string(),
        xberg::LayoutDetectionModel::PpDocLayoutV3.to_string(),
    ]);
}

fn push_ocr_args(args: &mut Vec<String>, backend: &str) {
    args.extend([
        "--ocr".to_string(),
        "true".to_string(),
        "--ocr-backend".to_string(),
        backend.to_string(),
        "--force-ocr".to_string(),
        "true".to_string(),
    ]);
}

fn push_sceptre_args(args: &mut Vec<String>, engine_options: &str) {
    push_ocr_args(args, "sceptre");
    args.extend(["--ocr-backend-options".to_string(), engine_options.to_string()]);
}

fn pipeline_requires_ocr(pipeline: XbergPipeline) -> bool {
    !matches!(
        pipeline,
        XbergPipeline::Baseline | XbergPipeline::Layout | XbergPipeline::LayoutV3
    )
}

/// Builds the framework name for a pipeline/format/batch/pdf-backend combination.
///
/// `XbergPdfBackend::Native` must keep producing exactly today's name (no suffix) so existing
/// committed results under `tools/benchmark-harness/results/` stay comparable. Only a non-default
/// backend earns a `-pdfium` suffix, inserted before the `-batch` suffix.
fn xberg_framework_name(
    pipeline: XbergPipeline,
    output_format: OutputFormat,
    batch: bool,
    pdf_backend: XbergPdfBackend,
) -> String {
    let format_slug = match output_format {
        OutputFormat::Markdown => "markdown",
        OutputFormat::Plaintext => "plaintext",
    };
    let pdf_backend_suffix = match pdf_backend {
        XbergPdfBackend::Native => "",
        XbergPdfBackend::Pdfium => "-pdfium",
    };
    if batch {
        format!(
            "xberg-{}-{}{}-batch",
            format_slug,
            pipeline.as_str(),
            pdf_backend_suffix
        )
    } else {
        format!("xberg-{}-{}{}", format_slug, pipeline.as_str(), pdf_backend_suffix)
    }
}

/// Push the flags that force OCR through one named backend.
fn push_forced_ocr_args(args: &mut Vec<String>, backend: &str) {
    args.push("--ocr".to_string());
    args.push("true".to_string());
    args.push("--ocr-backend".to_string());
    args.push(backend.to_string());
    args.push("--force-ocr".to_string());
    args.push("true".to_string());
}

/// Append the CLI flags that distinguish one pipeline from the baseline.
fn push_pipeline_args(args: &mut Vec<String>, pipeline: XbergPipeline) {
    match pipeline {
        XbergPipeline::Baseline => {}
        XbergPipeline::Layout => push_layout_args(args),
        XbergPipeline::LayoutV3 => push_layout_v3_args(args),
        XbergPipeline::PaddleOcr | XbergPipeline::BaselinePaddle => push_forced_ocr_args(args, "paddle-ocr"),
        XbergPipeline::LayoutPaddle => {
            push_layout_args(args);
            push_forced_ocr_args(args, "paddle-ocr");
        }
        XbergPipeline::SceptreOrt => push_sceptre_args(args, SCEPTRE_ORT_OPTIONS_JSON),
        XbergPipeline::SceptreOrtLayout => {
            push_layout_args(args);
            push_sceptre_args(args, SCEPTRE_ORT_OPTIONS_JSON);
        }
        XbergPipeline::SceptreOrtAutoRotate => {
            push_sceptre_args(args, SCEPTRE_ORT_OPTIONS_JSON);
            args.extend(["--ocr-auto-rotate".to_string(), "true".to_string()]);
        }
        XbergPipeline::SceptreTract => push_sceptre_args(args, SCEPTRE_TRACT_OPTIONS_JSON),
        XbergPipeline::CandleTrocr => push_forced_ocr_args(args, "candle-trocr"),
        XbergPipeline::CandlePaddleocrVl | XbergPipeline::CandlePaddleocrVl15 => {
            push_forced_ocr_args(args, "candle-paddleocr-vl");
        }
        XbergPipeline::CandleGlmOcr => push_forced_ocr_args(args, "candle-glm-ocr"),
        XbergPipeline::CandleDeepseekOcr => push_forced_ocr_args(args, "candle-deepseek-ocr"),
    }
}

/// Creates a Xberg adapter for the given pipeline and configuration.
///
/// # Arguments
/// * `pipeline` - The pipeline variant (baseline, layout, paddle-ocr)
/// * `output_format` - Output format for extraction (markdown or plaintext)
/// * `batch` - Whether to use batch extraction mode
/// * `pdf_backend` - PDF engine to select via `--pdf-backend`. `Native` reproduces today's
///   behavior and framework name exactly; `Pdfium` appends a `-pdfium` suffix to the framework
///   name so it never collides with (or silently replaces) an existing `Native` cohort.
///
/// # Returns
/// * `Ok(SubprocessAdapter)` - Configured adapter ready for extraction
/// * `Err(Error)` - If xberg cannot be located
pub fn create_xberg_adapter(
    pipeline: XbergPipeline,
    output_format: OutputFormat,
    batch: bool,
    ocr_enabled: bool,
    pdf_backend: XbergPdfBackend,
) -> Result<SubprocessAdapter> {
    if !ocr_enabled && pipeline_requires_ocr(pipeline) {
        return Err(crate::Error::Config(format!(
            "xberg pipeline '{}' requires OCR, but OCR is disabled",
            pipeline.as_str()
        )));
    }

    let cli_path = locate_xberg_cli()?;
    let build_id = probe_xberg_build_id(&cli_path)?;
    let build_identity = ExecutableBuildIdentity {
        build_id,
        path: cli_path.clone(),
    };

    let content_format = match output_format {
        OutputFormat::Markdown => "markdown",
        OutputFormat::Plaintext => "plain",
    };

    let tesseract_ocr_enabled = ocr_enabled
        && matches!(
            pipeline,
            XbergPipeline::Baseline | XbergPipeline::Layout | XbergPipeline::LayoutV3
        );
    let mut args = benchmark_base_args(batch, content_format, tesseract_ocr_enabled);

    push_pipeline_args(&mut args, pipeline);

    args.push("--pdf-backend".to_string());
    args.push(pdf_backend.as_str().to_string());

    let framework_name = xberg_framework_name(pipeline, output_format, batch, pdf_backend);
    // Derive the benchmarkable format list from xberg's own MIME registry so it never drifts from
    // what xberg can actually extract (a stale hardcoded list silently filtered out fixtures for
    // formats like fb2/eml/msg/rst/org/latex/typst/docbook/tsv, which xberg fully supports).
    let supported_formats = xberg::list_supported_formats()
        .into_iter()
        .map(|format| format.extension)
        .collect();

    let env = vec![(STAGE_TIMING_ENV_VAR.to_string(), "1".to_string())];

    let single_file_args = batch.then(|| {
        let mut single_args = args.clone();
        single_args[0] = "extract".to_string();
        single_args
    });

    let mut adapter = if batch {
        SubprocessAdapter::with_batch_capability(
            &framework_name,
            cli_path,
            args,
            env,
            supported_formats,
            BatchCapability {
                entry_point: BatchEntryPoint::XbergCliExtractBatch,
                timing_scope: BatchTimingScope::ColdEndToEndSubprocess,
                per_item_timing: true,
            },
        )
    } else {
        SubprocessAdapter::new(&framework_name, cli_path, args, env, supported_formats)
    }
    .with_supported_output_formats(vec![output_format])
    .with_ocr_language_policy(declared_ocr_language_policy(&framework_name))
    .with_executable_build_identity(build_identity);
    if let Some(single_args) = single_file_args {
        adapter = adapter.with_single_file_args(single_args);
    }

    Ok(adapter)
}

/// Creates an in-process Xberg adapter for steady-state latency measurements. ~keep
///
/// Only baseline and the two layout pipelines are exposed initially. Single-file mode retains a paired subprocess
/// adapter solely for its cold CLI probe. Batch mode has no truthful equivalent probe and retains
/// only one Rust [`xberg::Engine`] across discarded warmups and measured invocations. ~keep
pub fn create_xberg_steady_adapter(
    pipeline: XbergPipeline,
    output_format: OutputFormat,
    batch: bool,
    ocr_enabled: bool,
    pdf_backend: XbergPdfBackend,
    max_threads: Option<usize>,
) -> Result<NativeAdapter> {
    if !matches!(
        pipeline,
        XbergPipeline::Baseline | XbergPipeline::Layout | XbergPipeline::LayoutV3
    ) {
        return Err(crate::Error::Config(format!(
            "steady-state Xberg adapter does not yet support pipeline '{}'",
            pipeline.as_str()
        )));
    }

    let config = steady_extraction_config(pipeline, output_format, ocr_enabled, pdf_backend, max_threads)?;
    let name = format!(
        "{}-steady-state{}",
        xberg_framework_name(pipeline, output_format, false, pdf_backend),
        if batch { "-batch" } else { "" }
    );
    if batch {
        return Ok(NativeAdapter::with_identity(name, config));
    }
    let cold_adapter = match max_threads {
        Some(max_threads) => create_xberg_adapter(pipeline, output_format, false, ocr_enabled, pdf_backend)?
            .with_xberg_max_threads(max_threads),
        None => create_xberg_adapter(pipeline, output_format, false, ocr_enabled, pdf_backend)?,
    };
    Ok(NativeAdapter::with_identity_and_cold_adapter(
        name,
        config,
        cold_adapter,
    ))
}

fn steady_extraction_config(
    pipeline: XbergPipeline,
    output_format: OutputFormat,
    ocr_enabled: bool,
    pdf_backend: XbergPdfBackend,
    max_threads: Option<usize>,
) -> Result<xberg::ExtractionConfig> {
    let mut config: xberg::ExtractionConfig = serde_json::from_str(benchmark_config_json(ocr_enabled))?;
    if let Some(ocr) = config.ocr.as_mut()
        && ocr.backend == "tesseract"
    {
        let tesseract = ocr.tesseract_config.get_or_insert_with(Default::default);
        tesseract.language.clone_from(&ocr.language);
        tesseract.use_cache = false;
    }
    config.output_format = match output_format {
        OutputFormat::Markdown => xberg::OutputFormat::Markdown,
        OutputFormat::Plaintext => xberg::OutputFormat::Plain,
    };
    config.pdf_options.get_or_insert_with(Default::default).backend = match pdf_backend {
        XbergPdfBackend::Native => xberg::PdfBackend::Native,
        XbergPdfBackend::Pdfium => xberg::PdfBackend::Pdfium,
    };
    if let Some(max_threads) = max_threads {
        config.concurrency.get_or_insert_with(Default::default).max_threads = Some(max_threads);
    }
    match pipeline {
        XbergPipeline::Layout => {
            config.layout = Some(xberg::LayoutDetectionConfig::default());
            config.use_layout_for_markdown = true;
        }
        XbergPipeline::LayoutV3 => {
            config.layout = Some(xberg::LayoutDetectionConfig {
                model: xberg::LayoutDetectionModel::PpDocLayoutV3,
                ..Default::default()
            });
            config.use_layout_for_markdown = true;
        }
        _ => {}
    }

    Ok(config)
}

fn probe_xberg_build_id(path: &Path) -> Result<String> {
    let output = std::process::Command::new(path)
        .args(["version", "--format", "json"])
        .output()
        .map_err(|error| {
            crate::Error::Benchmark(format!(
                "failed to query Xberg build identity from `{}`: {error}",
                path.display()
            ))
        })?;
    if !output.status.success() {
        return Err(crate::Error::Benchmark(format!(
            "Xberg build identity query failed for `{}` with status {}",
            path.display(),
            output.status
        )));
    }
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).map_err(|error| {
        crate::Error::Benchmark(format!(
            "Xberg build identity query returned invalid JSON for `{}`: {error}",
            path.display()
        ))
    })?;
    value
        .get("build_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|build_id| !build_id.is_empty())
        .map(str::to_string)
        .ok_or_else(|| {
            crate::Error::Benchmark(format!(
                "Xberg build identity query returned no build_id for `{}`",
                path.display()
            ))
        })
}

/// Locates the xberg executable.
///
/// Searches in priority order:
/// 1. The executable file specified by `XBERG_CLI_BINARY`
/// 2. `target/release/xberg`
/// 3. `target/debug/xberg`
/// 4. `which xberg`
///
/// # Returns
/// * `Ok(PathBuf)` - Path to the executable
/// * `Err(Error)` - If the override is invalid or xberg cannot be found
fn locate_xberg_cli() -> Result<PathBuf> {
    let binary_override = std::env::var_os(XBERG_CLI_BINARY_ENV_VAR);
    locate_xberg_cli_with_override(binary_override.as_deref())
}

fn locate_xberg_cli_with_override(binary_override: Option<&OsStr>) -> Result<PathBuf> {
    if let Some(binary_override) = binary_override {
        let path = PathBuf::from(binary_override);
        validate_xberg_cli_override(&path)?;
        return Ok(path);
    }

    let release_path = PathBuf::from("target/release/xberg");
    if release_path.exists() {
        return Ok(release_path);
    }

    let debug_path = PathBuf::from("target/debug/xberg");
    if debug_path.exists() {
        return Ok(debug_path);
    }

    if let Ok(path) = which("xberg") {
        return Ok(path);
    }

    Err(crate::Error::Benchmark(
        "xberg binary not found. Build with: cargo build --release -p xberg-cli --features all".to_string(),
    ))
}

fn validate_xberg_cli_override(path: &Path) -> Result<()> {
    let metadata = path.metadata().map_err(|error| {
        crate::Error::Benchmark(format!(
            "{XBERG_CLI_BINARY_ENV_VAR} points to `{}` but its metadata could not be read: {error}",
            path.display()
        ))
    })?;

    if !metadata.is_file() {
        return Err(crate::Error::Benchmark(format!(
            "{XBERG_CLI_BINARY_ENV_VAR} must point to an executable file, but `{}` is not a file",
            path.display()
        )));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(crate::Error::Benchmark(format!(
                "{XBERG_CLI_BINARY_ENV_VAR} must point to an executable file, but `{}` has no execute permission",
                path.display()
            )));
        }
    }

    #[cfg(windows)]
    {
        let extension = path.extension().and_then(OsStr::to_str).map(str::to_ascii_lowercase);
        let is_executable = matches!(extension.as_deref(), Some("exe" | "com" | "cmd" | "bat"));
        if !is_executable {
            return Err(crate::Error::Benchmark(format!(
                "{XBERG_CLI_BINARY_ENV_VAR} must point to an executable file, but `{}` has no executable extension",
                path.display()
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::FrameworkAdapter;

    fn merged_benchmark_config(tesseract_ocr_enabled: bool) -> xberg::ExtractionConfig {
        xberg::core::config::merge::merge_config_json(
            &xberg::ExtractionConfig::default(),
            benchmark_config_json(tesseract_ocr_enabled),
        )
        .unwrap()
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
            let args = benchmark_base_args(batch, "markdown", false);
            assert!(args.iter().any(|arg| arg == "--no-config-discovery"));
        }
    }

    #[test]
    fn tesseract_benchmark_args_do_not_override_nested_config() {
        let args = benchmark_base_args(false, "markdown", true);

        assert!(!args.iter().any(|arg| arg == "--ocr"));
        assert!(!args.iter().any(|arg| arg == "--ocr-backend"));
        let config_index = args.iter().position(|arg| arg == "--config-json").unwrap();
        assert_eq!(args[config_index + 1], TESSERACT_BENCHMARK_CONFIG_JSON);
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
            steady_extraction_config(pipeline, OutputFormat::Markdown, true, XbergPdfBackend::Native, None)
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
        )
        .unwrap();
        assert!(layout_v3.use_layout_for_markdown);
        assert_eq!(
            layout_v3.ocr.as_ref().map(|ocr| ocr.backend.as_str()),
            Some("tesseract")
        );
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
        )
        .unwrap();
        let cold: xberg::ExtractionConfig = serde_json::from_str(benchmark_config_json(true)).unwrap();

        assert!(steady.force_ocr);
        assert!(cold.force_ocr);
        assert_eq!(steady.ocr.as_ref().map(|ocr| ocr.backend.as_str()), Some("tesseract"));
        assert_eq!(cold.ocr.as_ref().map(|ocr| ocr.backend.as_str()), Some("tesseract"));
        let steady_tesseract = steady
            .ocr
            .as_ref()
            .and_then(|ocr| ocr.tesseract_config.as_ref())
            .expect("warm in-process OCR must carry explicit nested cache control");
        assert_eq!(steady_tesseract.language, vec!["eng".to_string()]);
        assert!(!steady_tesseract.use_cache);
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
}
