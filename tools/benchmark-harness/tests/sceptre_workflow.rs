use std::path::Path;

fn benchmark_workflow() -> String {
    let harness_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(harness_root.join("../../.github/workflows/benchmarks.yaml")).unwrap()
}

#[test]
fn workflow_prewarm_selects_explicit_sceptre_engines() {
    let workflow = benchmark_workflow();
    assert!(workflow.contains("Pre-warm Sceptre ORT models"));
    assert!(workflow.contains(
        "cargo build --locked --release -p xberg-cli --features all,ort-dynamic,heic-dynamic,xberg/sceptre-ocr-tract \
         --config 'target.aarch64-unknown-linux-gnu.heif.rustc-link-lib=[]'"
    ));
    assert!(workflow.contains(r#"--ocr-backend-options '{"model":{"backend":"ort"}}'"#));
    assert!(workflow.contains("Pre-warm Sceptre diagnostic models"));
    assert!(workflow.contains(r#"backend="tract""#));
}

#[test]
fn sceptre_workflow_is_bounded_to_structured_markdown_single_and_batch() {
    let workflow = benchmark_workflow();
    let diagnostic_job = workflow.split("  sceptre-structured-diagnostic:").nth(1).unwrap();
    let diagnostic_job = diagnostic_job.split("\n  bench-docling:").next().unwrap();

    assert!(
        diagnostic_job.contains("pipeline: [sceptre-ort, sceptre-ort-layout, sceptre-ort-autorotate, sceptre-tract]")
    );
    assert!(diagnostic_job.contains("mode: [single-file, batch]"));
    assert!(diagnostic_job.contains("OUTPUT_FORMAT: markdown"));
    assert!(diagnostic_job.contains("cohorts/ocr-images-structured.json"));
    assert!(!diagnostic_job.contains("output_format: [markdown, plaintext]"));
}
