use super::xberg_registration::{XbergRegistration, plan_xberg_adapters};
use super::*;
use benchmark_harness::adapters::xberg::{AutoOcrBackend, auto_ocr_framework_name};

/// Framework names `run` would build for these selections: the registration plan, which is what
/// `register_xberg_pipelines` iterates, so the real filters decide the result.
fn planned_names(format: OutputFormat, batch_mode: bool, ocr: bool, requested: &[&str]) -> Vec<String> {
    let config = BenchmarkConfig::default();
    let should_init = |name: &str| requested.is_empty() || requested.contains(&name);
    let context = XbergRegistration {
        config: &config,
        batch_mode,
        has_explicit_frameworks: !requested.is_empty(),
        ocr,
        format,
        should_init: &should_init,
    };
    plan_xberg_adapters(&context, &[XbergPdfBackend::Native])
        .into_iter()
        .map(|planned| planned.framework_name(format, batch_mode))
        .collect()
}

#[test]
fn steady_adapter_is_not_planned_without_explicit_selection() {
    let names = planned_names(OutputFormat::Markdown, false, false, &[]);
    assert!(!names.is_empty());
    assert!(names.iter().all(|name| !name.contains("steady-state")), "{names:?}");

    assert_eq!(
        planned_names(
            OutputFormat::Markdown,
            false,
            false,
            &["xberg-markdown-layout-steady-state"]
        ),
        ["xberg-markdown-layout-steady-state"]
    );
}

#[test]
fn default_markdown_run_plans_exactly_the_proven_pipelines() {
    assert_eq!(
        planned_names(OutputFormat::Markdown, false, true, &[]),
        [
            "xberg-markdown-baseline",
            "xberg-markdown-layout",
            "xberg-markdown-baseline-paddle",
            "xberg-markdown-layout-paddle",
            "xberg-markdown-sceptre-ort",
            "xberg-markdown-sceptre-ort-layout",
            "xberg-markdown-sceptre-ort-autorotate",
        ]
    );
}

#[test]
fn no_plaintext_layout_variant_is_planned() {
    let plaintext = planned_names(OutputFormat::Plaintext, false, true, &[]);
    assert_eq!(
        plaintext,
        [
            "xberg-plaintext-baseline",
            "xberg-plaintext-baseline-paddle",
            "xberg-plaintext-sceptre-ort",
            "xberg-plaintext-sceptre-ort-autorotate",
        ]
    );
    assert!(
        planned_names(
            OutputFormat::Plaintext,
            false,
            true,
            &["xberg-plaintext-layout", "xberg-plaintext-layout-v3"]
        )
        .is_empty()
    );
}

#[test]
fn pipelines_that_need_ocr_are_not_planned_when_ocr_is_off() {
    assert_eq!(
        planned_names(OutputFormat::Markdown, false, false, &[]),
        ["xberg-markdown-baseline", "xberg-markdown-layout"]
    );
}

#[test]
fn auto_ocr_variants_are_named_per_format_and_mode() {
    let names = |format, batch| {
        AutoOcrBackend::ALL
            .iter()
            .map(|backend| auto_ocr_framework_name(*backend, format, batch))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        names(OutputFormat::Markdown, false),
        ["xberg-markdown-auto-ocr", "xberg-markdown-auto-ocr-paddle"]
    );
    assert_eq!(
        names(OutputFormat::Plaintext, true),
        [
            "xberg-plaintext-auto-ocr-batch",
            "xberg-plaintext-auto-ocr-paddle-batch"
        ]
    );
}

#[test]
fn tesseract_preflight_covers_auto_ocr_but_not_its_paddle_counterpart() {
    assert!(selected_frameworks_use_tesseract(&[
        "xberg-markdown-auto-ocr".to_string()
    ]));
    assert!(!selected_frameworks_use_tesseract(&[
        "xberg-markdown-auto-ocr-paddle".to_string()
    ]));
}
