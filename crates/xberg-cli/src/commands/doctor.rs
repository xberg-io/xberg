//! Doctor command - probe configured backends and report what will actually
//! execute on this host.

use anyhow::{Context, Result};
use std::path::PathBuf;
use xberg::{DoctorCheck, DoctorReport, ProbeStatus, doctor};

use super::config::load_config;
use crate::{WireFormat, style};

/// Experimental local ONNX candidate selection for `xberg doctor`.
///
/// Every field is feature-gated so a build without the experimental adapters exposes no flags and
/// appends no candidate checks. Nothing here downloads; reports only resolve the local cache.
#[derive(Debug, Default)]
pub struct OnnxCandidateOptions {
    #[cfg(any(feature = "captioning-onnx", feature = "classification-onnx"))]
    pub cache_dir: Option<PathBuf>,
    #[cfg(feature = "captioning-onnx")]
    pub captioning_model: Option<String>,
    #[cfg(feature = "captioning-onnx")]
    pub captioning_precision: Option<String>,
    #[cfg(feature = "captioning-onnx")]
    pub captioning_backend: Option<String>,
    #[cfg(feature = "classification-onnx")]
    pub classification_model: Option<String>,
    #[cfg(feature = "classification-onnx")]
    pub classification_precision: Option<String>,
    #[cfg(feature = "classification-onnx")]
    pub classification_backend: Option<String>,
}

/// Fold the outcome of an already-run cache cleanup pass into `report` as a regular
/// check, so the text/JSON/TOON renderers only ever need one output shape. `cleaned` is
/// `None` when cleanup was not requested at all (nothing to report); `Some(None)` when it
/// was requested but not attempted (cache dir override, ownership unverifiable).
fn append_cache_clean_check(report: &mut DoctorReport, cleaned: Option<Option<xberg::doctor::CleanOutcome>>) {
    let Some(cleaned) = cleaned else {
        return;
    };
    report.checks.push(match cleaned {
        Some(outcome) if outcome.failed == 0 => DoctorCheck::pass(
            "cache.clean",
            format!("removed {} stray cache file(s)", outcome.removed),
        ),
        Some(outcome) => DoctorCheck::fail(
            "cache.clean",
            format!(
                "removed {} stray cache file(s), failed to remove {}",
                outcome.removed, outcome.failed
            ),
        ),
        None => DoctorCheck::skip(
            "cache.clean",
            "not attempted: XBERG_CACHE_DIR override in effect and ownership cannot be verified",
        ),
    });
}

/// Render the doctor report as human-readable text.
#[expect(
    clippy::print_stdout,
    reason = "the doctor report is the command's stdout result output"
)]
fn print_text_report(report: &DoctorReport) {
    println!("{}", style::header("Doctor"));
    println!("{}", style::dim("======"));
    for check in &report.checks {
        let marker = match check.status {
            ProbeStatus::Pass => style::success("pass"),
            ProbeStatus::Warn => style::warning("warn"),
            ProbeStatus::Fail => style::error("FAIL"),
            ProbeStatus::Skip => style::dim("skip"),
        };
        println!("{marker} {} — {}", check.name, check.message);
    }

    let count = |status: ProbeStatus| report.checks.iter().filter(|c| c.status == status).count();
    let mut parts = vec![format!("{} passed", count(ProbeStatus::Pass))];
    for (status, noun) in [
        (ProbeStatus::Warn, "warning(s)"),
        (ProbeStatus::Skip, "skipped"),
        (ProbeStatus::Fail, "failed"),
    ] {
        let n = count(status);
        if n > 0 {
            parts.push(format!("{n} {noun}"));
        }
    }
    let n = report.checks.len();
    let noun = if n == 1 { "check" } else { "checks" };
    println!();
    println!("{n} {noun}: {}", parts.join(", "));
}

/// Render the doctor report in the requested wire format.
#[expect(
    clippy::print_stdout,
    reason = "the doctor report is the command's stdout result output"
)]
fn print_report(report: &DoctorReport, format: WireFormat) -> Result<()> {
    match format {
        WireFormat::Text => print_text_report(report),
        WireFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(report).context("Failed to serialize doctor report to JSON")?
            );
        }
        WireFormat::Toon => {
            println!(
                "{}",
                serde_toon::to_string(report).context("Failed to serialize doctor report to TOON")?
            );
        }
    }
    Ok(())
}

/// Execute the doctor command. Exits nonzero when any check fails, so the
/// command is usable as a setup gate in scripts and CI.
pub fn doctor_command(
    config_path: Option<PathBuf>,
    no_config_discovery: bool,
    format: WireFormat,
    clean: bool,
    #[cfg_attr(
        not(any(feature = "captioning-onnx", feature = "classification-onnx")),
        allow(unused_variables)
    )]
    candidates: &OnnxCandidateOptions,
) -> Result<()> {
    let config = doctor_config(config_path, no_config_discovery)?;

    // Clean first so the report reflects the post-clean cache state; the
    // cleanup result is a regular check, keeping one output shape per format.
    let cleaned = clean.then(xberg::doctor::clean_obsolete);
    let mut report = doctor(&config);
    append_cache_clean_check(&mut report, cleaned);
    #[cfg(any(feature = "captioning-onnx", feature = "classification-onnx"))]
    append_candidate_checks(&mut report, candidates)?;

    print_report(&report, format)?;

    if !report.is_ok() {
        std::process::exit(1);
    }
    Ok(())
}

/// The config doctor reports on: the loaded config with the environment overrides `serve` and
/// `mcp` apply, so doctor sees what those servers run.
fn doctor_config(config_path: Option<PathBuf>, no_config_discovery: bool) -> Result<xberg::ExtractionConfig> {
    let mut config = load_config(config_path, !no_config_discovery)?;
    config.apply_env_overrides()?;
    Ok(config)
}

/// Append the experimental candidate artifact catalog to the report.
///
/// Without an explicit `--captioning-model`/`--classification-model` every staged candidate is
/// listed, so the offline doctor output always names each pinned artifact. Naming one model
/// narrows the catalog to it. Nothing here downloads.
#[cfg(any(feature = "captioning-onnx", feature = "classification-onnx"))]
fn append_candidate_checks(report: &mut DoctorReport, options: &OnnxCandidateOptions) -> Result<()> {
    use super::onnx_candidates;

    let cache = options.cache_dir.as_deref();
    #[allow(unused_mut)]
    let mut any_selected = false;
    #[cfg(feature = "captioning-onnx")]
    {
        any_selected |= options.captioning_model.is_some();
    }
    #[cfg(feature = "classification-onnx")]
    {
        any_selected |= options.classification_model.is_some();
    }
    let mut summaries = Vec::new();

    #[cfg(feature = "captioning-onnx")]
    {
        use xberg::local_captioning::LocalCaptioningModel;
        onnx_candidates::parse_backend(options.captioning_backend.as_deref(), "captioning")?;
        let precision = onnx_candidates::parse_captioning_precision(options.captioning_precision.as_deref())?;
        match options.captioning_model.as_deref() {
            Some(id) => summaries.push(onnx_candidates::captioning_report(id, precision, cache)?),
            None if !any_selected => {
                for id in LocalCaptioningModel::staged_candidates() {
                    summaries.push(onnx_candidates::captioning_report(id, precision, cache)?);
                }
            }
            None => {}
        }
    }

    #[cfg(feature = "classification-onnx")]
    {
        use xberg::local_classification::LocalNliModel;
        onnx_candidates::parse_backend(options.classification_backend.as_deref(), "classification")?;
        let precision = onnx_candidates::parse_classification_precision(options.classification_precision.as_deref())?;
        match options.classification_model.as_deref() {
            Some(id) => summaries.push(onnx_candidates::classification_report(id, precision, cache)?),
            None if !any_selected => {
                for id in LocalNliModel::staged_candidates() {
                    summaries.push(onnx_candidates::classification_report(id, precision, cache)?);
                }
            }
            None => {}
        }
    }

    onnx_candidates::append_doctor_checks(report, &summaries, any_selected);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::env_test;

    const CHILD: &str = "commands::doctor::tests::doctor_config_applies_env_overrides_child";

    #[test]
    fn doctor_config_applies_env_overrides() {
        let prefix = tempfile::tempdir().unwrap();
        std::fs::write(prefix.path().join("eng.traineddata"), b"fast").unwrap();
        env_test::run_child_test(CHILD, env_test::CASE, "best", |command| {
            command
                .env("XBERG_OCR_TESSDATA_MODEL", "best")
                .env("TESSDATA_PREFIX", prefix.path());
        });
    }

    #[test]
    #[ignore = "run with an isolated environment by its parent test"]
    fn doctor_config_applies_env_overrides_child() {
        if env_test::child_test_root(env_test::CASE).is_none() {
            return;
        }
        let config = doctor_config(None, true).expect("doctor config loads");

        assert_eq!(
            xberg::core::config::ocr::implicit_tessdata_model(),
            xberg::TessdataModel::Best,
            "XBERG_OCR_TESSDATA_MODEL must reach doctor"
        );
        let report = doctor(&config);
        if let Some(check) = report.checks.iter().find(|check| check.name == "ocr.tesseract")
            && check.status != ProbeStatus::Skip
        {
            assert_eq!(check.status, ProbeStatus::Warn, "{}", check.message);
        }
    }
}
