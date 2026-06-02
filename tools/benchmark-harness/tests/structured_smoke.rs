//! Stage 0/2 smoke test: prove the structured-extraction benchmark plumbing
//! actually runs end-to-end.
//!
//! Composes the three Stage 0 deliverables (dataset fixture shape +
//! [`json_quality`] metrics) with the kreuzberg `extract_file` +
//! `StructuredExtractionConfig` pipeline against a real LLM.
//!
//! Required env var: `OPENAI_API_KEY` (loaded from the workspace `.env`).
//! Skips gracefully when the key is absent so the test stays runnable in
//! CI without secrets.
//!
//! Run with:
//!
//! ```text
//! cargo test -p benchmark-harness --test structured_smoke -- --nocapture --test-threads=1
//! ```

use std::path::PathBuf;

use benchmark_harness::datasets::{Split, StructuredFixture};
use benchmark_harness::json_quality;
use kreuzberg::core::config::{ExtractionConfig, LlmConfig, StructuredExtractionConfig};
use serde_json::{Value, json};

fn require_env(var: &str) -> Option<String> {
    let _ = dotenvy::dotenv();
    match std::env::var(var) {
        Ok(val) if !val.is_empty() => Some(val),
        _ => {
            eprintln!("SKIP: {var} not set, skipping live smoke");
            None
        }
    }
}

/// In-memory smoke fixtures.
///
/// Hand-rolled instead of dataset-loaded so the smoke proves the
/// `StructuredFixture` shape composes with the extraction + scoring stack
/// without taking a hard dependency on a downloaded CORD/SROIE/FUNSD copy.
/// The actual `datasets::cord::load` etc. functions have their own unit
/// coverage.
fn fake_memo_fixture() -> StructuredFixture {
    let schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "type": "object",
        "properties": {
            "title": { "type": "string" },
            "date": { "type": "string" },
            "summary": { "type": "string" }
        },
        "required": ["title", "date", "summary"],
        "additionalProperties": false
    });
    // Hand-rolled GT for the actual contents of fake_memo.pdf (a delivery
    // report dated 2023-05-05). Title is stable run-to-run; date/summary
    // wording varies a little so F1 here is informational, not a gate.
    let ground_truth = json!({
        "title": "Delivery Report",
        "date": "May 5, 2023",
        "summary": "Logistics delivery: water bottles, blankets, and laptops delivered using three trucks over 15 hours."
    });
    StructuredFixture {
        document_path: PathBuf::from("../../test_documents/pdf/fake_memo.pdf"),
        schema,
        ground_truth,
        dataset: "smoke-memo",
        split: Split::Test,
    }
}

async fn run_text_then_llm(fixture: &StructuredFixture, api_key: String) -> Value {
    let config = ExtractionConfig {
        structured_extraction: Some(StructuredExtractionConfig {
            schema: fixture.schema.clone(),
            schema_name: format!("{}_extraction", fixture.dataset),
            schema_description: Some("Extract structured fields from the document.".to_string()),
            strict: true,
            prompt: None,
            llm: LlmConfig {
                model: "openai/gpt-4o-mini".to_string(),
                api_key: Some(api_key),
                timeout_secs: Some(120),
                max_retries: Some(2),
                ..Default::default()
            },
        }),
        ..Default::default()
    };
    let result = kreuzberg::extract_file(&fixture.document_path, None, &config)
        .await
        .expect("kreuzberg extract_file failed");
    result
        .structured_output
        .expect("structured_output missing; structured_extraction did not run")
}

#[tokio::test]
async fn structured_extraction_smoke_text_then_llm() {
    let Some(api_key) = require_env("OPENAI_API_KEY") else {
        return;
    };

    let fixtures = vec![fake_memo_fixture()];

    let mut predictions: Vec<Value> = Vec::with_capacity(fixtures.len());
    let mut per_doc_f1: Vec<f64> = Vec::with_capacity(fixtures.len());
    let mut per_doc_type_correctness: Vec<f64> = Vec::with_capacity(fixtures.len());

    for fixture in &fixtures {
        let pred = run_text_then_llm(fixture, api_key.clone()).await;

        let f1 = json_quality::field_precision_recall_f1(&pred, &fixture.ground_truth);
        let type_score = json_quality::type_correctness_rate(&pred, &fixture.ground_truth);

        eprintln!(
            "fixture {}: precision={:.3} recall={:.3} f1={:.3} type_correctness={:.3}",
            fixture.document_path.display(),
            f1.precision,
            f1.recall,
            f1.f1,
            type_score
        );
        eprintln!("  predicted: {}", pred);
        eprintln!("  expected:  {}", fixture.ground_truth);

        per_doc_f1.push(f1.f1);
        per_doc_type_correctness.push(type_score);
        predictions.push(pred);
    }

    let mean_f1 = per_doc_f1.iter().sum::<f64>() / per_doc_f1.len() as f64;
    let mean_type = per_doc_type_correctness.iter().sum::<f64>() / per_doc_type_correctness.len() as f64;
    let schema_validity = json_quality::schema_validity_rate(&predictions, &fixtures[0].schema);

    eprintln!(
        "\nSMOKE SUMMARY (text-then-llm, openai/gpt-4o-mini):\n  \
         docs={}  schema_validity={:.3}  mean_field_f1={:.3}  mean_type_correctness={:.3}",
        fixtures.len(),
        schema_validity,
        mean_f1,
        mean_type
    );

    // Smoke gates: prove the pipeline composes. F1 is informational here
    // because the GT is hand-rolled paraphrase, not dataset-canonical; field
    // accuracy is what Stage 2 measures on real datasets (CORD / SROIE / etc.).
    assert_eq!(
        schema_validity, 1.0,
        "structured output must validate against the schema (strict mode is on)"
    );
    assert!(
        mean_type >= 0.5,
        "type correctness must be ≥0.5 for a 3-string schema, got {mean_type}"
    );
    assert!(
        mean_f1.is_finite() && mean_f1 >= 0.0,
        "field F1 must be a finite non-negative number, got {mean_f1}"
    );
}
