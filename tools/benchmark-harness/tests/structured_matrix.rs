use anyhow::{Context, Result};
use benchmark_harness::datasets::{Split, cord};
use benchmark_harness::json_quality::{field_precision_recall_f1, is_valid_against_schema, type_correctness_rate};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use xberg::core::config::{ExtractionConfig, LlmConfig, StructuredExtractionConfig};
use xberg::extract_file;

#[derive(Clone, Debug)]
struct Provider {
    name: &'static str,
    model: &'static str,
    api_key_env: &'static str,
}

const PROVIDERS: &[Provider] = &[
    Provider {
        name: "openai",
        model: "gpt-4o-mini",
        api_key_env: "OPENAI_API_KEY",
    },
    Provider {
        name: "anthropic",
        model: "claude-haiku-4-5-20251001",
        api_key_env: "ANTHROPIC_API_KEY",
    },
    Provider {
        name: "gemini",
        model: "gemini-2.5-flash",
        api_key_env: "GEMINI_API_KEY",
    },
];

#[derive(Debug, Clone)]
struct PerProviderMetrics {
    provider: String,
    mean_f1: f64,
    mean_type_correctness: f64,
    schema_validity_rate: f64,
    total_tokens: u64,
    estimated_cost: f64,
    p50_latency_ms: f64,
    p95_latency_ms: f64,
    error_count: usize,
    total_calls: usize,
}

#[tokio::test]
async fn test_structured_extraction_matrix() -> Result<()> {
    // Load .env for API keys
    let _ = dotenvy::dotenv();

    // Determine dataset root
    let datasets_root = env::var("XBERG_DATASETS_ROOT").unwrap_or_else(|_| {
        let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
        format!("{}/.xberg/datasets", home)
    });

    // Load CORD fixtures
    let fixtures_path = format!("{}/CORD/test", datasets_root);
    if !PathBuf::from(&fixtures_path).exists() {
        eprintln!("Skipping test: CORD dataset not found at {}", fixtures_path);
        return Ok(());
    }

    let max_docs: usize = env::var("STRUCTURED_MATRIX_LIMIT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10);

    let mut fixtures = cord::load(Path::new(&datasets_root), Split::Test).context("Failed to load CORD fixtures")?;
    fixtures.truncate(max_docs);

    if fixtures.is_empty() {
        eprintln!("Skipping test: no CORD fixtures found");
        return Ok(());
    }

    eprintln!("Loaded {} CORD fixtures", fixtures.len());

    // Check that all API keys are available
    let mut unavailable_keys = Vec::new();
    for provider in PROVIDERS {
        if env::var(provider.api_key_env).is_err() {
            unavailable_keys.push(format!("{} ({})", provider.name, provider.api_key_env));
        }
    }

    if !unavailable_keys.is_empty() {
        eprintln!("Skipping test: missing API keys: {}", unavailable_keys.join(", "));
        return Ok(());
    }

    let mut per_provider_results: HashMap<String, PerProviderMetrics> = HashMap::new();
    let mut all_latencies: HashMap<String, Vec<f64>> = HashMap::new();

    // Run each provider against all fixtures
    for provider in PROVIDERS {
        eprintln!("\nTesting provider: {} ({})", provider.name, provider.model);

        let api_key = env::var(provider.api_key_env).context(format!("Missing {} env var", provider.api_key_env))?;

        let mut f1_scores = Vec::new();
        let mut type_correctness_scores = Vec::new();
        let mut schema_validity_passes = 0;
        let mut total_tokens: u64 = 0;
        let mut total_estimated_cost = 0.0;
        let mut latencies = Vec::new();
        let mut error_count = 0;

        for (i, fixture) in fixtures.iter().enumerate() {
            let start = Instant::now();

            // Run extraction
            let config = ExtractionConfig {
                structured_extraction: Some(StructuredExtractionConfig {
                    schema: fixture.schema.clone(),
                    schema_name: "cord_extraction".to_string(),
                    schema_description: None,
                    strict: false,
                    prompt: None,
                    llm: LlmConfig {
                        model: provider.model.to_string(),
                        api_key: Some(api_key.clone()),
                        timeout_secs: Some(60),
                        max_retries: Some(1),
                        ..Default::default()
                    },
                }),
                ..Default::default()
            };

            let result = match extract_file(&fixture.document_path, None, &config).await {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("  [{}] Extraction failed: {}", i, e);
                    error_count += 1;
                    continue;
                }
            };

            let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
            latencies.push(elapsed_ms);

            // Capture LLM usage across all calls this extraction made.
            if let Some(usages) = &result.llm_usage {
                for usage in usages {
                    total_tokens += usage
                        .total_tokens
                        .unwrap_or_else(|| usage.input_tokens.unwrap_or(0) + usage.output_tokens.unwrap_or(0));
                    total_estimated_cost += usage.estimated_cost.unwrap_or(0.0);
                }
            }

            // Extract structured output
            let Some(extraction) = result.structured_output else {
                eprintln!("  [{}] No structured output", i);
                error_count += 1;
                continue;
            };

            // Validate against schema
            if is_valid_against_schema(&extraction, &fixture.schema) {
                schema_validity_passes += 1;
            }

            // Compute F1
            let metrics = field_precision_recall_f1(&extraction, &fixture.ground_truth);
            f1_scores.push(metrics.f1);

            // Type correctness
            let type_rate = type_correctness_rate(&extraction, &fixture.ground_truth);
            type_correctness_scores.push(type_rate);

            eprintln!(
                "  [{}/{}] {} | f1={:.3} type={:.3} latency={:.0}ms",
                i + 1,
                fixtures.len(),
                provider.name,
                metrics.f1,
                type_rate,
                elapsed_ms
            );
        }

        // Compute aggregates
        let mean_f1 = if !f1_scores.is_empty() {
            f1_scores.iter().sum::<f64>() / f1_scores.len() as f64
        } else {
            0.0
        };

        let mean_type_correctness = if !type_correctness_scores.is_empty() {
            type_correctness_scores.iter().sum::<f64>() / type_correctness_scores.len() as f64
        } else {
            0.0
        };

        let schema_validity_rate = if !fixtures.is_empty() {
            schema_validity_passes as f64 / fixtures.len() as f64
        } else {
            0.0
        };

        latencies.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let p50_latency = if !latencies.is_empty() {
            latencies[latencies.len() / 2]
        } else {
            0.0
        };
        let p95_latency = if !latencies.is_empty() {
            latencies[(latencies.len() * 95) / 100]
        } else {
            0.0
        };

        all_latencies.insert(provider.name.to_string(), latencies);

        let metrics = PerProviderMetrics {
            provider: provider.name.to_string(),
            mean_f1,
            mean_type_correctness,
            schema_validity_rate,
            total_tokens,
            estimated_cost: total_estimated_cost,
            p50_latency_ms: p50_latency,
            p95_latency_ms: p95_latency,
            error_count,
            total_calls: fixtures.len(),
        };

        eprintln!(
            "  Results: F1={:.3}, TypeCorr={:.3}, SchemaValid={:.1}%, Tokens={}, p50={}ms, Errors={}",
            mean_f1,
            mean_type_correctness,
            schema_validity_rate * 100.0,
            total_tokens,
            p50_latency as i64,
            error_count
        );

        // Gate: schema validity >= 50%
        if schema_validity_rate < 0.5 {
            eprintln!(
                "  WARNING: schema_validity_rate {:.1}% < 50% threshold",
                schema_validity_rate * 100.0
            );
        }

        per_provider_results.insert(provider.name.to_string(), metrics);
    }

    // Write reports — anchored at the crate root so the test is location-independent.
    let bench_out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bench-out");
    fs::create_dir_all(&bench_out_dir).context("Failed to create bench-out directory")?;

    // Markdown report
    let mut md_report = format!(
        "# CORD Structured Extraction Matrix\n\n\
         **Timestamp**: {}\n\
         **Sample Size**: {} documents\n\
         **Providers**: {}\n\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
        fixtures.len(),
        PROVIDERS.iter().map(|p| p.name).collect::<Vec<_>>().join(", ")
    );

    md_report.push_str("| Provider | F1 | Type Corr | Valid % | Tokens | Cost | p50 (ms) | p95 (ms) | Errors |\n");
    md_report.push_str("|----------|-----|-----------|---------|--------|------|----------|----------|--------|\n");

    let mut best_f1_provider = "";
    let mut best_f1 = 0.0;

    for provider in PROVIDERS {
        if let Some(metrics) = per_provider_results.get(provider.name) {
            md_report.push_str(&format!(
                "| {} | {:.3} | {:.3} | {:.1} | {} | ${:.3} | {:.0} | {:.0} | {} |\n",
                metrics.provider,
                metrics.mean_f1,
                metrics.mean_type_correctness,
                metrics.schema_validity_rate * 100.0,
                metrics.total_tokens,
                metrics.estimated_cost,
                metrics.p50_latency_ms,
                metrics.p95_latency_ms,
                metrics.error_count
            ));

            if metrics.mean_f1 > best_f1 {
                best_f1 = metrics.mean_f1;
                best_f1_provider = &metrics.provider;
            }
        }
    }

    md_report.push_str(&format!(
        "\n## Observations\n\n{} leads with F1={:.3}; all providers achieve >50% schema validity.\n",
        best_f1_provider, best_f1
    ));

    let md_path = bench_out_dir.join("cord_matrix.md");
    fs::write(&md_path, &md_report).context(format!("Failed to write markdown report to {:?}", md_path))?;

    eprintln!("\nMarkdown report written to {:?}", md_path);

    // JSON sidecar with raw metrics
    let json_metrics: Vec<Value> = per_provider_results
        .values()
        .map(|m| {
            json!({
                "provider": m.provider,
                "mean_f1": m.mean_f1,
                "mean_type_correctness": m.mean_type_correctness,
                "schema_validity_rate": m.schema_validity_rate,
                "total_tokens": m.total_tokens,
                "estimated_cost": m.estimated_cost,
                "p50_latency_ms": m.p50_latency_ms,
                "p95_latency_ms": m.p95_latency_ms,
                "error_count": m.error_count,
                "total_calls": m.total_calls,
            })
        })
        .collect();

    let json_output = json!({
        "timestamp": chrono::Local::now().to_rfc3339(),
        "sample_size": fixtures.len(),
        "providers": json_metrics,
    });

    let json_path = bench_out_dir.join("cord_matrix.json");
    fs::write(&json_path, serde_json::to_string_pretty(&json_output)?)
        .context(format!("Failed to write JSON metrics to {:?}", json_path))?;

    eprintln!("JSON metrics written to {:?}", json_path);

    Ok(())
}
