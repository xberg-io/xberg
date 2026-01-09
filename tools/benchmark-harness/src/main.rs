//! Benchmark harness CLI

#[cfg(feature = "memory-profiling")]
#[global_allocator]
static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

use benchmark_harness::{BenchmarkConfig, BenchmarkMode, FixtureManager, Result};
use clap::{Parser, Subcommand, ValueEnum};
use std::collections::HashSet;
use std::path::PathBuf;

/// CLI enum for benchmark mode
#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliMode {
    /// Single-file mode: Sequential execution for fair latency comparison
    SingleFile,
    /// Batch mode: Concurrent execution for throughput measurement
    Batch,
}

/// CLI enum for output format
#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    /// JSON format (default)
    Json,
}

impl From<CliMode> for BenchmarkMode {
    fn from(mode: CliMode) -> Self {
        match mode {
            CliMode::SingleFile => BenchmarkMode::SingleFile,
            CliMode::Batch => BenchmarkMode::Batch,
        }
    }
}

#[derive(Parser)]
#[command(name = "benchmark-harness")]
#[command(about = "Benchmark harness for document extraction frameworks", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all fixtures from a directory
    ListFixtures {
        /// Directory or file pattern to search for fixtures
        #[arg(short, long)]
        fixtures: PathBuf,
    },

    /// Validate fixtures without running benchmarks
    Validate {
        /// Directory or file pattern to search for fixtures
        #[arg(short, long)]
        fixtures: PathBuf,
    },

    /// Run benchmarks
    Run {
        /// Directory or file pattern to search for fixtures
        #[arg(short, long)]
        fixtures: PathBuf,

        /// Frameworks to benchmark (comma-separated)
        #[arg(short = 'F', long, value_delimiter = ',')]
        frameworks: Vec<String>,

        /// Output directory for results
        #[arg(short, long, default_value = "results")]
        output: PathBuf,

        /// Maximum concurrent extractions
        #[arg(short = 'c', long)]
        max_concurrent: Option<usize>,

        /// Timeout in seconds
        #[arg(short = 't', long)]
        timeout: Option<u64>,

        /// Benchmark mode: single-file (sequential) or batch (concurrent)
        #[arg(short = 'm', long, value_enum, default_value = "batch")]
        mode: CliMode,

        /// Number of warmup iterations (discarded from statistics)
        #[arg(short = 'w', long, default_value = "1")]
        warmup: usize,

        /// Number of benchmark iterations for statistical analysis
        #[arg(short = 'i', long, default_value = "3")]
        iterations: usize,

        /// Enable OCR for image extraction
        #[arg(long, default_value = "true")]
        ocr: bool,

        /// Enable quality assessment
        #[arg(long, default_value = "true")]
        measure_quality: bool,
    },

    /// Consolidate multiple benchmark runs
    Consolidate {
        /// Input directories containing benchmark results
        #[arg(short, long, value_delimiter = ',')]
        inputs: Vec<PathBuf>,

        /// Output directory for consolidated results
        #[arg(short, long)]
        output: PathBuf,

        /// Baseline framework for delta calculations (not used but provided for compatibility)
        #[arg(long, default_value = "kreuzberg-native")]
        baseline: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::ListFixtures { fixtures } => {
            let mut manager = FixtureManager::new();

            if fixtures.is_dir() {
                manager.load_fixtures_from_dir(&fixtures)?;
            } else {
                manager.load_fixture(&fixtures)?;
            }

            println!("Loaded {} fixture(s)", manager.len());
            for (path, fixture) in manager.fixtures() {
                println!(
                    "  {} - {} ({} bytes)",
                    path.display(),
                    fixture.document.display(),
                    fixture.file_size
                );
            }

            Ok(())
        }

        Commands::Validate { fixtures } => {
            let mut manager = FixtureManager::new();

            if fixtures.is_dir() {
                manager.load_fixtures_from_dir(&fixtures)?;
            } else {
                manager.load_fixture(&fixtures)?;
            }

            println!("✓ All {} fixture(s) are valid", manager.len());
            Ok(())
        }

        Commands::Run {
            fixtures,
            frameworks,
            output,
            max_concurrent,
            timeout,
            mode,
            warmup,
            iterations,
            ocr,
            measure_quality,
        } => {
            use benchmark_harness::{AdapterRegistry, BenchmarkRunner, NativeAdapter};
            use kreuzberg::{ExtractionConfig, OcrConfig};
            use std::sync::Arc;

            let config = BenchmarkConfig {
                output_dir: output.clone(),
                max_concurrent: max_concurrent.unwrap_or_else(num_cpus::get),
                timeout: std::time::Duration::from_secs(timeout.unwrap_or(1800)),
                benchmark_mode: mode.into(),
                warmup_iterations: warmup,
                benchmark_iterations: iterations,
                measure_quality,
                ..Default::default()
            };

            config.validate()?;

            let mut extraction_config = if ocr {
                ExtractionConfig {
                    ocr: Some(OcrConfig {
                        backend: "tesseract".to_string(),
                        language: "eng".to_string(),
                        tesseract_config: None,
                    }),
                    ..Default::default()
                }
            } else {
                ExtractionConfig::default()
            };
            extraction_config.max_concurrent_extractions = Some(config.max_concurrent);

            let mut registry = AdapterRegistry::new();

            // Helper to check if a framework should be initialized
            // When specific frameworks are requested, only initialize those
            // When no frameworks are specified (empty list), initialize all available
            let should_init = |name: &str| -> bool { frameworks.is_empty() || frameworks.iter().any(|f| f == name) };

            // Helper macro for registering adapters conditionally
            macro_rules! try_register {
                ($name:expr, $create_fn:expr, $count:expr) => {
                    if should_init($name) {
                        match $create_fn() {
                            Ok(adapter) => {
                                if let Err(err) = registry.register(Arc::new(adapter)) {
                                    eprintln!("[adapter] ✗ {} (registration failed: {})", $name, err);
                                } else {
                                    eprintln!("[adapter] ✓ {} (registered)", $name);
                                    $count += 1;
                                }
                            }
                            Err(err) => eprintln!("[adapter] ✗ {} (initialization failed: {})", $name, err),
                        }
                    }
                };
            }

            // Always register native adapter if requested or no specific frameworks specified
            let mut kreuzberg_count = 0;
            if should_init("kreuzberg-native") {
                registry.register(Arc::new(NativeAdapter::with_config(extraction_config)))?;
                eprintln!("[adapter] ✓ kreuzberg-native (registered)");
                kreuzberg_count += 1;
            }

            use benchmark_harness::adapters::{
                create_csharp_adapter, create_csharp_batch_adapter, create_elixir_adapter, create_elixir_batch_adapter,
                create_go_adapter, create_go_batch_adapter, create_java_adapter, create_java_batch_adapter,
                create_node_adapter, create_node_batch_adapter, create_php_adapter, create_php_batch_adapter,
                create_python_adapter, create_python_batch_adapter, create_ruby_adapter, create_ruby_batch_adapter,
                create_wasm_adapter, create_wasm_batch_adapter,
            };

            try_register!("kreuzberg-python", create_python_adapter, kreuzberg_count);
            try_register!("kreuzberg-python-batch", create_python_batch_adapter, kreuzberg_count);
            try_register!("kreuzberg-go", create_go_adapter, kreuzberg_count);
            try_register!("kreuzberg-go-batch", create_go_batch_adapter, kreuzberg_count);
            try_register!("kreuzberg-node", create_node_adapter, kreuzberg_count);
            try_register!("kreuzberg-node-batch", create_node_batch_adapter, kreuzberg_count);
            try_register!("kreuzberg-wasm", create_wasm_adapter, kreuzberg_count);
            try_register!("kreuzberg-wasm-batch", create_wasm_batch_adapter, kreuzberg_count);
            try_register!("kreuzberg-ruby", create_ruby_adapter, kreuzberg_count);
            try_register!("kreuzberg-ruby-batch", create_ruby_batch_adapter, kreuzberg_count);
            try_register!("kreuzberg-java", create_java_adapter, kreuzberg_count);
            try_register!("kreuzberg-java-batch", create_java_batch_adapter, kreuzberg_count);
            try_register!("kreuzberg-csharp", create_csharp_adapter, kreuzberg_count);
            try_register!("kreuzberg-csharp-batch", create_csharp_batch_adapter, kreuzberg_count);
            try_register!("kreuzberg-php", create_php_adapter, kreuzberg_count);
            try_register!("kreuzberg-php-batch", create_php_batch_adapter, kreuzberg_count);
            try_register!("kreuzberg-elixir", create_elixir_adapter, kreuzberg_count);
            try_register!("kreuzberg-elixir-batch", create_elixir_batch_adapter, kreuzberg_count);

            let total_requested = if frameworks.is_empty() { 19 } else { frameworks.len() };
            eprintln!(
                "[adapter] Kreuzberg bindings: {}/{} available",
                kreuzberg_count, total_requested
            );

            use benchmark_harness::adapters::{
                create_docling_adapter, create_docling_batch_adapter, create_markitdown_adapter, create_mineru_adapter,
                create_mineru_batch_adapter, create_pandoc_adapter, create_pdfplumber_adapter,
                create_pdfplumber_batch_adapter, create_pymupdf4llm_adapter, create_tika_adapter,
                create_tika_batch_adapter, create_unstructured_adapter,
            };

            let mut external_count = 0;

            try_register!("docling", create_docling_adapter, external_count);
            try_register!("docling-batch", create_docling_batch_adapter, external_count);
            try_register!("markitdown", create_markitdown_adapter, external_count);
            try_register!("pandoc", create_pandoc_adapter, external_count);
            try_register!("unstructured", create_unstructured_adapter, external_count);
            try_register!("tika", create_tika_adapter, external_count);
            try_register!("tika-batch", create_tika_batch_adapter, external_count);
            try_register!("pymupdf4llm", create_pymupdf4llm_adapter, external_count);
            try_register!("pdfplumber", create_pdfplumber_adapter, external_count);
            try_register!("pdfplumber-batch", create_pdfplumber_batch_adapter, external_count);
            try_register!("mineru", create_mineru_adapter, external_count);
            try_register!("mineru-batch", create_mineru_batch_adapter, external_count);

            eprintln!(
                "[adapter] Open source extraction frameworks: {}/12 available",
                external_count
            );
            eprintln!(
                "[adapter] Total adapters: {} available",
                kreuzberg_count + external_count
            );

            let mut runner = BenchmarkRunner::new(config, registry);
            runner.load_fixtures(&fixtures)?;

            println!("Loaded {} fixture(s)", runner.fixture_count());
            println!("Frameworks: {:?}", frameworks);
            println!("Configuration: {:?}", runner.config());

            if runner.fixture_count() == 0 {
                println!("No fixtures to benchmark");
                return Ok(());
            }

            println!("\nRunning benchmarks...");
            let results = runner.run(&frameworks).await?;

            println!("\nCompleted {} benchmark(s)", results.len());

            let mut success_count = 0;
            let mut failure_count = 0;

            for result in &results {
                if result.success {
                    success_count += 1;
                } else {
                    failure_count += 1;
                }
            }

            println!("\nSummary:");
            println!("  Successful: {}", success_count);
            println!("  Failed: {}", failure_count);
            println!("  Total: {}", results.len());

            use benchmark_harness::{write_by_extension_analysis, write_json};

            // Always output JSON format
            let output_file = output.join("results.json");
            write_json(&results, &output_file)?;
            println!("\nResults written to: {}", output_file.display());

            let by_ext_file = output.join("by-extension.json");
            write_by_extension_analysis(&results, &by_ext_file)?;
            println!("Per-extension analysis written to: {}", by_ext_file.display());

            Ok(())
        }
        Commands::Consolidate {
            inputs,
            output,
            baseline: _baseline,
        } => {
            use benchmark_harness::{consolidate_runs, load_run_results, write_consolidated_json};

            if inputs.is_empty() {
                return Err(benchmark_harness::Error::Benchmark(
                    "No input directories specified".to_string(),
                ));
            }

            println!("Loading benchmark results from {} directory(ies)...", inputs.len());
            let mut all_runs = Vec::new();

            for input in &inputs {
                if !input.is_dir() {
                    return Err(benchmark_harness::Error::Benchmark(format!(
                        "Input path is not a directory: {}",
                        input.display()
                    )));
                }
                println!("  Loading from: {}", input.display());
                let run_results = load_run_results(input)?;
                println!("    Loaded {} results", run_results.len());
                all_runs.push(run_results);
            }

            println!("\nConsolidating {} run(s)...", all_runs.len());
            let consolidated = consolidate_runs(all_runs)?;

            // Create new aggregation format
            println!("\nCreating new aggregation format...");
            // Flatten all runs into a single vec of BenchmarkResult
            let all_results: Vec<_> = inputs
                .iter()
                .flat_map(|input| load_run_results(input).unwrap_or_default())
                .collect();
            let aggregated = benchmark_harness::aggregate_new_format(&all_results);
            println!(
                "  Aggregated {} frameworks across {} file types",
                aggregated.by_framework_mode.len(),
                aggregated
                    .by_framework_mode
                    .values()
                    .flat_map(|fm| fm.by_file_type.keys())
                    .collect::<HashSet<_>>()
                    .len()
            );

            println!("\nConsolidation Summary:");
            println!("  Total files processed: {}", consolidated.total_files);
            println!("  Number of runs: {}", consolidated.run_count);
            println!("  Frameworks analyzed: {}", consolidated.framework_count);

            eprintln!("\nFramework Summary:");
            for (framework, agg) in &consolidated.by_framework {
                eprintln!("  {}:", framework);
                eprintln!("    Files processed: {}", agg.total_files);
                eprintln!("    Mean duration: {:.2} ms", agg.mean_duration_ms);
                eprintln!("    Std dev: {:.2} ms", agg.duration_std_dev_ms);
                eprintln!("    Success rate: {:.1}%", agg.success_rate * 100.0);
            }

            std::fs::create_dir_all(&output).map_err(benchmark_harness::Error::Io)?;

            // Always output JSON format
            let output_file = output.join("consolidated.json");
            write_consolidated_json(&consolidated, &output_file)?;
            println!("\nConsolidated results written to: {}", output_file.display());

            let aggregated_file = output.join("aggregated.json");
            let json = serde_json::to_string_pretty(&aggregated).map_err(|e| {
                benchmark_harness::Error::Benchmark(format!("Failed to serialize aggregated results: {}", e))
            })?;
            std::fs::write(&aggregated_file, json).map_err(benchmark_harness::Error::Io)?;
            println!("Aggregated metrics written to: {}", aggregated_file.display());
            println!("View results with the Vite visualizer at tools/benchmark-visualizer");

            Ok(())
        }
    }
}
