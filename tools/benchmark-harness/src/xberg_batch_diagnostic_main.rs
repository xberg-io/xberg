use std::path::PathBuf;

use benchmark_harness::Result;
use benchmark_harness::batch_diagnostic::{
    BatchDiagnosticConfig, BatchDiagnosticLane, run_batch_diagnostic, run_batch_lane_diagnostic,
};
use clap::{Parser, ValueEnum};

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Lane {
    Compare,
    Sequential,
    Batch,
}

#[derive(Debug, Parser)]
#[command(about = "Fast warmed Xberg batch-vs-sequential diagnostic")]
struct Args {
    /// Input documents. Inputs are cycled until batch-size is reached.
    #[arg(short, long, required = true)]
    input: Vec<PathBuf>,

    #[arg(long, default_value_t = 8)]
    batch_size: usize,

    #[arg(long, default_value_t = 1)]
    warmup: usize,

    #[arg(long, default_value_t = 3)]
    iterations: usize,

    /// Inline JSON ExtractionConfig. Diagnostic cache and explicit thread flags take precedence.
    #[arg(long, value_name = "JSON")]
    config_json: Option<String>,

    #[arg(long)]
    max_threads: Option<usize>,

    #[arg(long)]
    max_concurrent: Option<usize>,

    /// Emit machine-readable JSON instead of the compact terminal summary.
    #[arg(long)]
    json: bool,

    /// Run both lanes for correctness, or isolate one lane for process-level profiling.
    #[arg(long, value_enum, default_value_t = Lane::Compare)]
    lane: Lane,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let config = BatchDiagnosticConfig {
        inputs: args.input,
        batch_size: args.batch_size,
        warmup_iterations: args.warmup,
        iterations: args.iterations,
        extraction_config_json: args.config_json,
        max_threads: args.max_threads,
        max_concurrent_extractions: args.max_concurrent,
    };

    match args.lane {
        Lane::Compare => {
            let report = run_batch_diagnostic(&config).await?;
            if args.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!(
                    "batch={} iterations={} sequential={:.2}ms ({:.1} docs/s) batch={:.2}ms ({:.1} docs/s) speedup={:.2}x outputs_match={}",
                    report.batch_size,
                    report.iterations,
                    report.sequential_median_ms,
                    report.sequential_documents_per_second,
                    report.batch_median_ms,
                    report.batch_documents_per_second,
                    report.speedup,
                    report.outputs_match,
                );
            }
        }
        Lane::Sequential | Lane::Batch => {
            let lane = match args.lane {
                Lane::Sequential => BatchDiagnosticLane::Sequential,
                Lane::Batch => BatchDiagnosticLane::Batch,
                Lane::Compare => unreachable!("compare lane handled above"),
            };
            let report = run_batch_lane_diagnostic(&config, lane).await?;
            if args.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!(
                    "lane={} batch={} iterations={} median={:.2}ms ({:.1} docs/s)",
                    report.lane, report.batch_size, report.iterations, report.median_ms, report.documents_per_second,
                );
            }
        }
    }
    Ok(())
}
