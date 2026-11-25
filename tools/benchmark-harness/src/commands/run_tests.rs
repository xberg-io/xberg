use super::*;

#[test]
fn steady_adapter_is_not_registered_without_explicit_selection() {
    let config = BenchmarkConfig::default();
    let should_init = |_name: &str| true;
    let context = XbergRegistration {
        config: &config,
        batch_mode: false,
        has_explicit_frameworks: false,
        ocr: false,
        format: OutputFormat::Markdown,
        should_init: &should_init,
    };
    let mut registry = AdapterRegistry::new();

    assert!(!register_steady_xberg_adapter(
        &mut registry,
        &context,
        XbergPipeline::Layout,
        XbergPdfBackend::Native,
    ));
    assert!(registry.is_empty());
}

fn registered_names(format: OutputFormat) -> Vec<String> {
    XBERG_RUN_PIPELINES
        .iter()
        .filter(|pipeline| !plaintext_layout_is_a_no_op(format, **pipeline))
        .map(|pipeline| format!("xberg-{format}-{pipeline}"))
        .collect()
}

#[test]
fn no_plaintext_layout_variant_is_registered() {
    let plaintext = registered_names(OutputFormat::Plaintext);
    assert!(!plaintext.is_empty());
    assert!(
        plaintext.iter().all(|name| !name.contains("layout")),
        "plaintext layout variants are a no-op: {plaintext:?}"
    );
    assert!(plaintext.contains(&"xberg-plaintext-baseline".to_string()));
    assert!(plaintext.contains(&"xberg-plaintext-baseline-paddle".to_string()));
    assert!(plaintext.contains(&"xberg-plaintext-sceptre-ort".to_string()));
}

#[test]
fn markdown_layout_variants_stay_registered() {
    let markdown = registered_names(OutputFormat::Markdown);
    assert_eq!(markdown.len(), XBERG_RUN_PIPELINES.len());
    for name in [
        "xberg-markdown-layout",
        "xberg-markdown-layout-paddle",
        "xberg-markdown-sceptre-ort-layout",
    ] {
        assert!(markdown.contains(&name.to_string()), "{name}");
    }
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
