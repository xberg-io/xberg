use super::xberg_registration::{XbergRegistration, plan_xberg_adapters, reject_plaintext_layout_requests};
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
fn auto_ocr_variants_are_opt_in_and_need_ocr() {
    let auto_ocr =
        |names: Vec<String>| -> Vec<String> { names.into_iter().filter(|name| name.contains("-auto-ocr")).collect() };
    assert!(
        auto_ocr(planned_names(OutputFormat::Markdown, false, true, &[])).is_empty(),
        "a default run must not register auto-OCR"
    );
    assert_eq!(
        auto_ocr(planned_names(
            OutputFormat::Markdown,
            false,
            true,
            &["xberg-markdown-auto-ocr", "xberg-markdown-auto-ocr-paddle"]
        )),
        ["xberg-markdown-auto-ocr", "xberg-markdown-auto-ocr-paddle"]
    );
    assert_eq!(
        planned_names(OutputFormat::Plaintext, true, true, &["xberg-plaintext-auto-ocr-batch"]),
        ["xberg-plaintext-auto-ocr-batch"]
    );
    assert!(
        planned_names(OutputFormat::Markdown, false, false, &["xberg-markdown-auto-ocr"]).is_empty(),
        "auto-OCR needs OCR enabled"
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

#[test]
fn explicit_plaintext_layout_requests_are_rejected_with_the_reason() {
    for name in [
        "xberg-plaintext-layout",
        "xberg-plaintext-layout-batch",
        "xberg-plaintext-layout-v3",
        "xberg-plaintext-layout-paddle",
        "xberg-plaintext-sceptre-ort-layout",
        "xberg-plaintext-layout-steady-state",
    ] {
        let error = reject_plaintext_layout_requests(&[name.to_string()]).expect_err(name);
        let message = error.to_string();
        assert!(message.contains("discards layout structure"), "{name}: {message}");
        assert!(message.contains(name), "{name}: {message}");
    }
}

#[test]
fn other_requests_pass_the_plaintext_layout_check() {
    let accepted: Vec<String> = [
        "xberg-plaintext-baseline",
        "xberg-plaintext-baseline-paddle",
        "xberg-plaintext-sceptre-ort",
        "xberg-markdown-layout",
        "xberg-markdown-sceptre-ort-layout-batch",
        "docling",
    ]
    .map(String::from)
    .to_vec();
    assert!(reject_plaintext_layout_requests(&accepted).is_ok());
}

#[test]
fn build_config_carries_the_selected_tessdata_model() {
    use clap::Parser as _;

    let parse = |extra: &[&str]| {
        let cli = crate::cli::Cli::try_parse_from(
            ["benchmark-harness", "run", "--fixtures", "fixtures"]
                .into_iter()
                .chain(extra.iter().copied()),
        )
        .unwrap();
        let crate::cli::Commands::Run(args) = cli.command else {
            panic!("expected run command");
        };
        build_config(&args)
    };

    assert_eq!(parse(&[]).tessdata_model, xberg::TessdataModel::Fast);
    assert_eq!(
        parse(&["--tessdata-model", "best"]).tessdata_model,
        xberg::TessdataModel::Best
    );
}
