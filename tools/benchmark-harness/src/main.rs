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
        #[arg(long, default_value = "false")]
        ocr: bool,

        /// Enable quality assessment
        #[arg(long, default_value = "false")]
        measure_quality: bool,

        /// Run only a subset of fixtures (format: INDEX/TOTAL, e.g. 1/3 for first of 3 shards)
        #[arg(long)]
        shard: Option<String>,
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
        #[arg(long, default_value = "kreuzberg-rust")]
        baseline: String,
    },

    /// Generate fixture JSON files from vendored test documents
    GenerateFixtures {
        /// Directory containing vendored test documents
        #[arg(long)]
        vendored_dir: PathBuf,

        /// Output directory for generated fixture JSON files
        #[arg(long)]
        output_dir: PathBuf,

        /// Directory containing ground truth files (optional)
        #[arg(long)]
        ground_truth_dir: Option<PathBuf>,

        /// Overwrite existing fixtures
        #[arg(long, default_value = "false")]
        overwrite: bool,
    },

    /// Measure framework installation sizes
    MeasureFrameworkSizes {
        /// Output JSON file for framework sizes
        #[arg(long)]
        output: PathBuf,
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
            shard,
        } => {
            use benchmark_harness::{AdapterRegistry, BenchmarkRunner, NativeAdapter};
            use kreuzberg::{ExtractionConfig, OcrConfig};
            use std::sync::Arc;

            // Validate framework names: alphanumeric, hyphens, underscores only
            for framework in &frameworks {
                if !framework.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
                    return Err(benchmark_harness::Error::Benchmark(format!(
                        "Invalid framework name '{}': must contain only alphanumeric characters, hyphens, or underscores",
                        framework
                    )));
                }
            }

            let config = BenchmarkConfig {
                output_dir: output.clone(),
                max_concurrent: max_concurrent.unwrap_or_else(num_cpus::get),
                timeout: std::time::Duration::from_secs(timeout.unwrap_or(1800)),
                benchmark_mode: mode.into(),
                warmup_iterations: warmup,
                benchmark_iterations: iterations,
                measure_quality,
                ocr_enabled: ocr,
                ..Default::default()
            };

            config.validate()?;

            let mut extraction_config = if ocr {
                ExtractionConfig {
                    ocr: Some(OcrConfig {
                        backend: "tesseract".to_string(),
                        language: "eng".to_string(),
                        tesseract_config: None,
                        output_format: None,
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
            if should_init("kreuzberg-rust") {
                registry.register(Arc::new(NativeAdapter::with_config(extraction_config)))?;
                eprintln!("[adapter] ✓ kreuzberg-rust (registered)");
                kreuzberg_count += 1;
            }

            use benchmark_harness::adapters::{
                create_csharp_adapter, create_elixir_adapter, create_go_adapter, create_java_adapter,
                create_node_adapter, create_php_adapter, create_python_adapter, create_ruby_adapter,
                create_wasm_adapter,
            };

            try_register!("kreuzberg-python", || create_python_adapter(ocr), kreuzberg_count);
            try_register!("kreuzberg-go", || create_go_adapter(ocr), kreuzberg_count);
            try_register!("kreuzberg-node", || create_node_adapter(ocr), kreuzberg_count);
            try_register!("kreuzberg-wasm", || create_wasm_adapter(ocr), kreuzberg_count);
            try_register!("kreuzberg-ruby", || create_ruby_adapter(ocr), kreuzberg_count);
            try_register!("kreuzberg-java", || create_java_adapter(ocr), kreuzberg_count);
            try_register!("kreuzberg-csharp", || create_csharp_adapter(ocr), kreuzberg_count);
            try_register!("kreuzberg-php", || create_php_adapter(ocr), kreuzberg_count);
            try_register!("kreuzberg-elixir", || create_elixir_adapter(ocr), kreuzberg_count);

            let total_requested = if frameworks.is_empty() { 10 } else { frameworks.len() };
            eprintln!(
                "[adapter] Kreuzberg bindings: {}/{} available",
                kreuzberg_count, total_requested
            );

            // Track which requested frameworks failed to initialize
            let mut failed_frameworks = Vec::new();
            for name in &frameworks {
                if !registry.contains(name) {
                    failed_frameworks.push(name.clone());
                }
            }
            if !failed_frameworks.is_empty() {
                eprintln!(
                    "[adapter] WARNING: {} requested kreuzberg framework(s) failed to initialize: {}",
                    failed_frameworks.len(),
                    failed_frameworks.join(", ")
                );
            }

            use benchmark_harness::adapters::{
                create_docling_adapter, create_markitdown_adapter, create_mineru_adapter, create_pandoc_adapter,
                create_pdfplumber_adapter, create_pymupdf4llm_adapter, create_tika_adapter,
                create_unstructured_adapter,
            };

            let mut external_count = 0;

            try_register!("docling", || create_docling_adapter(ocr), external_count);
            try_register!("markitdown", || create_markitdown_adapter(ocr), external_count);
            try_register!("pandoc", create_pandoc_adapter, external_count);
            try_register!("unstructured", || create_unstructured_adapter(ocr), external_count);
            try_register!("tika", || create_tika_adapter(ocr), external_count);
            try_register!("pymupdf4llm", || create_pymupdf4llm_adapter(ocr), external_count);
            try_register!("pdfplumber", || create_pdfplumber_adapter(ocr), external_count);
            try_register!("mineru", || create_mineru_adapter(ocr), external_count);

            eprintln!(
                "[adapter] Open source extraction frameworks: {}/8 available",
                external_count
            );
            eprintln!(
                "[adapter] Total adapters: {} available",
                kreuzberg_count + external_count
            );

            let mut runner = BenchmarkRunner::new(config, registry);
            runner.load_fixtures(&fixtures)?;

            // Apply sharding if requested
            if let Some(ref shard_spec) = shard {
                let parts: Vec<&str> = shard_spec.split('/').collect();
                if parts.len() != 2 {
                    return Err(benchmark_harness::Error::Config(format!(
                        "Invalid shard format '{}': expected INDEX/TOTAL (e.g. 1/3)",
                        shard_spec
                    )));
                }
                let index: usize = parts[0].parse().map_err(|_| {
                    benchmark_harness::Error::Config(format!("Invalid shard index '{}': must be a number", parts[0]))
                })?;
                let total: usize = parts[1].parse().map_err(|_| {
                    benchmark_harness::Error::Config(format!("Invalid shard total '{}': must be a number", parts[1]))
                })?;
                if index < 1 || index > total || total < 1 {
                    return Err(benchmark_harness::Error::Config(format!(
                        "Invalid shard {}/{}: index must be 1..=total",
                        index, total
                    )));
                }
                let total_before = runner.fixture_count();
                runner.apply_shard(index, total);
                println!(
                    "Shard {}/{}: {} of {} fixtures",
                    index,
                    total,
                    runner.fixture_count(),
                    total_before
                );
            }

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

            // Fail if any requested frameworks failed to initialize
            if !failed_frameworks.is_empty() {
                return Err(benchmark_harness::Error::Benchmark(format!(
                    "Requested framework(s) failed to initialize: {}",
                    failed_frameworks.join(", ")
                )));
            }

            // Fail if no extractions succeeded (binding compile/link/runtime failure)
            if !results.is_empty() && success_count == 0 {
                return Err(benchmark_harness::Error::Benchmark(format!(
                    "All {} extraction(s) failed. The framework likely failed to compile, link, or start.",
                    results.len()
                )));
            }

            Ok(())
        }
        Commands::Consolidate {
            inputs,
            output,
            baseline: _baseline,
        } => {
            use benchmark_harness::load_run_results;

            if inputs.is_empty() {
                return Err(benchmark_harness::Error::Benchmark(
                    "No input directories specified".to_string(),
                ));
            }

            println!("Loading benchmark results from {} directory(ies)...", inputs.len());

            let mut all_results = Vec::new();
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
                all_results.extend(run_results);
            }

            println!("\nAggregating {} results...", all_results.len());
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

            eprintln!("\nFramework Summary:");
            for (key, agg) in &aggregated.by_framework_mode {
                eprintln!("  {} ({}):", agg.framework, agg.mode);
                eprintln!("    File types: {}", agg.by_file_type.len());
                if let Some(cs) = &agg.cold_start {
                    eprintln!("    Cold start p50: {:.2} ms", cs.p50_ms);
                }
                let _ = key; // used as map key
            }

            std::fs::create_dir_all(&output).map_err(benchmark_harness::Error::Io)?;

            // Single unified output file
            let output_file = output.join("aggregated.json");
            let json = serde_json::to_string_pretty(&aggregated)
                .map_err(|e| benchmark_harness::Error::Benchmark(format!("Failed to serialize results: {}", e)))?;
            std::fs::write(&output_file, json).map_err(benchmark_harness::Error::Io)?;
            println!("\nResults written to: {}", output_file.display());

            Ok(())
        }

        Commands::GenerateFixtures {
            vendored_dir,
            output_dir,
            ground_truth_dir,
            overwrite,
        } => {
            use benchmark_harness::{GenerateConfig, generate_fixtures};

            println!("Generating fixtures from vendored documents...");
            println!("  Vendored directory: {}", vendored_dir.display());
            println!("  Output directory: {}", output_dir.display());
            if let Some(gt_dir) = &ground_truth_dir {
                println!("  Ground truth directory: {}", gt_dir.display());
            }
            println!("  Overwrite existing: {}", overwrite);

            let config = GenerateConfig {
                vendored_dir,
                output_dir,
                ground_truth_dir,
                overwrite,
            };

            let stats = generate_fixtures(&config)?;

            println!("\nGeneration complete:");
            println!("  Created: {} fixtures", stats.created);
            println!("  Skipped: {} fixtures (already exist)", stats.skipped);
            println!("  Errors: {}", stats.errors);

            if !stats.by_type.is_empty() {
                println!("\nBy file type:");
                let mut types: Vec<_> = stats.by_type.iter().collect();
                types.sort_by_key(|(k, _)| *k);
                for (file_type, count) in types {
                    println!("  {}: {}", file_type, count);
                }
            }

            Ok(())
        }

        Commands::MeasureFrameworkSizes { output } => {
            use benchmark_harness::{measure_framework_sizes, save_framework_sizes};

            println!("Measuring framework installation sizes...");

            let sizes = measure_framework_sizes()?;

            println!("\nFramework sizes:");
            let mut items: Vec<_> = sizes.iter().collect();
            items.sort_by_key(|(k, _)| *k);

            for (name, info) in &items {
                let size_str = if info.size_bytes > 0 {
                    format_size(info.size_bytes)
                } else {
                    "unknown".to_string()
                };
                let status = if info.estimated { " (estimated)" } else { "" };
                println!("  {}: {}{} - {}", name, size_str, status, info.description);
            }

            // Create parent directory if needed
            if let Some(parent) = output.parent() {
                std::fs::create_dir_all(parent).map_err(benchmark_harness::Error::Io)?;
            }

            save_framework_sizes(&sizes, &output)?;
            println!("\nSizes written to: {}", output.display());

            Ok(())
        }
    }
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}
