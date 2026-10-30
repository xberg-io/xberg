//! Benchmark harness CLI

// Internal dev tool: stdout IS this binary's report output, so raw printing is intentional. ~keep
#![allow(clippy::print_stdout, clippy::print_stderr)]

#[cfg(feature = "memory-profiling")]
#[global_allocator]
static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

mod cli;
mod commands;
#[cfg(test)]
mod main_tests;

use benchmark_harness::Result;
use clap::Parser;
use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    let rust_log = std::env::var("RUST_LOG").ok();
    let benchmark_debug = std::env::var_os("BENCHMARK_DEBUG").is_some();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_filter(rust_log.as_deref(), benchmark_debug))
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::ListFixtures { fixtures } => commands::fixtures::list_fixtures(&fixtures),

        Commands::Validate { fixtures } => commands::fixtures::validate(&fixtures),

        Commands::Run(args) => commands::run::execute(args).await,

        Commands::Consolidate {
            inputs,
            output,
            baseline: _baseline,
        } => commands::consolidate::execute(inputs, output),

        Commands::MeasureFrameworkSizes { output } => commands::measure_sizes::execute(output),

        Commands::GapReport {
            results,
            output,
            baseline,
            layout,
            competitors,
        } => commands::gap_report::execute(results, output, baseline, layout, competitors),

        Commands::Compare(args) => commands::compare::execute(args).await,

        Commands::PipelineBenchmark(args) => commands::pipeline_benchmark::execute(args).await,

        Commands::Survey { fixtures, types } => commands::survey::execute(fixtures, types).await,

        Commands::ModelBenchmark {
            fixtures,
            model_a,
            model_b,
            cohort,
            max_pages,
            iterations,
            dpi,
            confidence_threshold,
            expected_pages,
            geometry_annotations,
            output,
        } => {
            commands::model_benchmark::execute(
                benchmark_harness::model_benchmark::ModelBenchmarkConfig {
                    fixtures_dir: fixtures,
                    model_a,
                    model_b,
                    cohort,
                    max_pages,
                    iterations,
                    dpi,
                    confidence_threshold,
                    expected_pages,
                    geometry_annotations,
                },
                output,
            )
            .await
        }

        Commands::ModelBenchmarkValidate { artifact } => commands::model_benchmark::validate_artifact(artifact).await,

        Commands::SplitBenchmark {
            fixtures,
            sweep,
            guardrails_out,
            profile_out,
        } => commands::split_benchmark::execute(fixtures, sweep, guardrails_out, profile_out).await,

        Commands::EmbedBenchmark => commands::embed_benchmark::execute(),

        Commands::ValidateGt { fixtures, fix, strict } => commands::validate_gt::execute(fixtures, fix, strict).await,

        Commands::ValidateArtifacts(args) => commands::validate_artifacts::execute(args),

        Commands::CohortContract { cohort } => commands::cohort_contract::execute(cohort.into()),
    }
}

fn tracing_filter(rust_log: Option<&str>, benchmark_debug: bool) -> tracing_subscriber::EnvFilter {
    const DEFAULT_FILTER: &str = "benchmark_harness=info";
    const DEBUG_FILTER: &str = "benchmark_harness=debug";

    rust_log
        .and_then(|filter| tracing_subscriber::EnvFilter::try_new(filter).ok())
        .unwrap_or_else(|| {
            tracing_subscriber::EnvFilter::new(if benchmark_debug { DEBUG_FILTER } else { DEFAULT_FILTER })
        })
}
