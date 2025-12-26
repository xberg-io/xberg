//! Benchmark harness CLI

#[cfg(feature = "memory-profiling")]
#[global_allocator]
static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

use benchmark_harness::{BenchmarkConfig, BenchmarkMode, FixtureManager, Result};
use clap::{Parser, Subcommand, ValueEnum};
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
    /// HTML format with interactive visualizations
    Html,
    /// Both JSON and HTML formats
    Both,
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

    /// Generate an HTML index gallery for flamegraphs
    GenerateFlamegraphIndex {
        /// Directory containing flamegraph SVG files
        #[arg(long)]
        flamegraphs: PathBuf,

        /// Output path for the HTML gallery file
        #[arg(long)]
        output: PathBuf,
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

        /// Output format: json, html, or both
        #[arg(long, value_enum, default_value = "json")]
        format: OutputFormat,

        /// Benchmark execution date (e.g., "2025-12-13 14:30:00 UTC")
        /// Used for marking when the benchmark was run in the HTML output
        #[arg(long)]
        benchmark_date: Option<String>,
    },

    /// Consolidate multiple benchmark runs
    Consolidate {
        /// Input directories containing benchmark results
        #[arg(short, long, value_delimiter = ',')]
        inputs: Vec<PathBuf>,

        /// Output directory for consolidated results
        #[arg(short, long)]
        output: PathBuf,

        /// Output format: json, html, or both
        #[arg(long, value_enum, default_value = "both")]
        format: OutputFormat,

        /// Baseline framework for delta calculations (not used but provided for compatibility)
        #[arg(long, default_value = "kreuzberg-native")]
        baseline: String,
    },

    /// Visualize benchmark results from existing runs
    Visualize {
        /// Input directories containing benchmark results
        #[arg(short, long, value_delimiter = ',')]
        inputs: Vec<PathBuf>,

        /// Output directory for visualization assets
        #[arg(short, long)]
        output: PathBuf,

        /// Output format: json, html, or both
        #[arg(long, value_enum, default_value = "html")]
        format: OutputFormat,

        /// Benchmark execution date (e.g., "2025-12-13 14:30:00 UTC")
        #[arg(long)]
        benchmark_date: Option<String>,
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

        Commands::GenerateFlamegraphIndex { flamegraphs, output } => {
            use benchmark_harness::generate_flamegraph_index;
            generate_flamegraph_index(&flamegraphs, &output)?;
            println!("✓ Flamegraph index generated: {}", output.display());
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
            format,
            benchmark_date,
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

            let extraction_config = if ocr {
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

            let mut registry = AdapterRegistry::new();

            registry.register(Arc::new(NativeAdapter::with_config(extraction_config)))?;
            eprintln!("[adapter] ✓ kreuzberg-native (registered)");

            use benchmark_harness::adapters::{
                create_csharp_sync_adapter, create_go_batch_adapter, create_go_sync_adapter, create_java_sync_adapter,
                create_node_async_adapter, create_node_batch_adapter, create_python_async_adapter,
                create_python_batch_adapter, create_python_sync_adapter, create_ruby_batch_adapter,
                create_ruby_sync_adapter, create_wasm_async_adapter, create_wasm_batch_adapter,
            };

            let mut kreuzberg_count = 1;

            if let Ok(adapter) = create_python_sync_adapter() {
                if let Ok(()) = registry.register(Arc::new(adapter)) {
                    eprintln!("[adapter] ✓ kreuzberg-python-sync (registered)");
                    kreuzberg_count += 1;
                } else {
                    eprintln!("[adapter] ✗ kreuzberg-python-sync (registration failed)");
                }
            } else {
                eprintln!("[adapter] ✗ kreuzberg-python-sync (initialization failed)");
            }

            if let Ok(adapter) = create_python_async_adapter() {
                if let Ok(()) = registry.register(Arc::new(adapter)) {
                    eprintln!("[adapter] ✓ kreuzberg-python-async (registered)");
                    kreuzberg_count += 1;
                } else {
                    eprintln!("[adapter] ✗ kreuzberg-python-async (registration failed)");
                }
            } else {
                eprintln!("[adapter] ✗ kreuzberg-python-async (initialization failed)");
            }

            if let Ok(adapter) = create_python_batch_adapter() {
                if let Ok(()) = registry.register(Arc::new(adapter)) {
                    eprintln!("[adapter] ✓ kreuzberg-python-batch (registered)");
                    kreuzberg_count += 1;
                } else {
                    eprintln!("[adapter] ✗ kreuzberg-python-batch (registration failed)");
                }
            } else {
                eprintln!("[adapter] ✗ kreuzberg-python-batch (initialization failed)");
            }

            match create_go_sync_adapter() {
                Ok(adapter) => {
                    if let Err(err) = registry.register(Arc::new(adapter)) {
                        eprintln!("[adapter] ✗ kreuzberg-go-sync (registration failed: {err})");
                    } else {
                        eprintln!("[adapter] ✓ kreuzberg-go-sync (registered)");
                        kreuzberg_count += 1;
                    }
                }
                Err(err) => eprintln!("[adapter] ✗ kreuzberg-go-sync (initialization failed: {err})"),
            }

            match create_go_batch_adapter() {
                Ok(adapter) => {
                    if let Err(err) = registry.register(Arc::new(adapter)) {
                        eprintln!("[adapter] ✗ kreuzberg-go-batch (registration failed: {err})");
                    } else {
                        eprintln!("[adapter] ✓ kreuzberg-go-batch (registered)");
                        kreuzberg_count += 1;
                    }
                }
                Err(err) => eprintln!("[adapter] ✗ kreuzberg-go-batch (initialization failed: {err})"),
            }

            match create_node_async_adapter() {
                Ok(adapter) => {
                    if let Err(err) = registry.register(Arc::new(adapter)) {
                        eprintln!("[adapter] ✗ kreuzberg-node-async (registration failed: {err})");
                    } else {
                        eprintln!("[adapter] ✓ kreuzberg-node-async (registered)");
                        kreuzberg_count += 1;
                    }
                }
                Err(err) => eprintln!("[adapter] ✗ kreuzberg-node-async (initialization failed: {err})"),
            }

            match create_node_batch_adapter() {
                Ok(adapter) => {
                    if let Err(err) = registry.register(Arc::new(adapter)) {
                        eprintln!("[adapter] ✗ kreuzberg-node-batch (registration failed: {err})");
                    } else {
                        eprintln!("[adapter] ✓ kreuzberg-node-batch (registered)");
                        kreuzberg_count += 1;
                    }
                }
                Err(err) => eprintln!("[adapter] ✗ kreuzberg-node-batch (initialization failed: {err})"),
            }

            match create_wasm_async_adapter() {
                Ok(adapter) => {
                    if let Err(err) = registry.register(Arc::new(adapter)) {
                        eprintln!("[adapter] ✗ kreuzberg-wasm-async (registration failed: {err})");
                    } else {
                        eprintln!("[adapter] ✓ kreuzberg-wasm-async (registered)");
                        kreuzberg_count += 1;
                    }
                }
                Err(err) => eprintln!("[adapter] ✗ kreuzberg-wasm-async (initialization failed: {err})"),
            }

            match create_wasm_batch_adapter() {
                Ok(adapter) => {
                    if let Err(err) = registry.register(Arc::new(adapter)) {
                        eprintln!("[adapter] ✗ kreuzberg-wasm-batch (registration failed: {err})");
                    } else {
                        eprintln!("[adapter] ✓ kreuzberg-wasm-batch (registered)");
                        kreuzberg_count += 1;
                    }
                }
                Err(err) => eprintln!("[adapter] ✗ kreuzberg-wasm-batch (initialization failed: {err})"),
            }

            match create_ruby_sync_adapter() {
                Ok(adapter) => {
                    if let Err(err) = registry.register(Arc::new(adapter)) {
                        eprintln!("[adapter] ✗ kreuzberg-ruby-sync (registration failed: {err})");
                    } else {
                        eprintln!("[adapter] ✓ kreuzberg-ruby-sync (registered)");
                        kreuzberg_count += 1;
                    }
                }
                Err(err) => eprintln!("[adapter] ✗ kreuzberg-ruby-sync (initialization failed: {err})"),
            }

            match create_ruby_batch_adapter() {
                Ok(adapter) => {
                    if let Err(err) = registry.register(Arc::new(adapter)) {
                        eprintln!("[adapter] ✗ kreuzberg-ruby-batch (registration failed: {err})");
                    } else {
                        eprintln!("[adapter] ✓ kreuzberg-ruby-batch (registered)");
                        kreuzberg_count += 1;
                    }
                }
                Err(err) => eprintln!("[adapter] ✗ kreuzberg-ruby-batch (initialization failed: {err})"),
            }

            match create_java_sync_adapter() {
                Ok(adapter) => {
                    if let Err(err) = registry.register(Arc::new(adapter)) {
                        eprintln!("[adapter] ✗ kreuzberg-java-sync (registration failed: {err})");
                    } else {
                        eprintln!("[adapter] ✓ kreuzberg-java-sync (registered)");
                        kreuzberg_count += 1;
                    }
                }
                Err(err) => eprintln!("[adapter] ✗ kreuzberg-java-sync (initialization failed: {err})"),
            }

            match create_csharp_sync_adapter() {
                Ok(adapter) => {
                    if let Err(err) = registry.register(Arc::new(adapter)) {
                        eprintln!("[adapter] ✗ kreuzberg-csharp-sync (registration failed: {err})");
                    } else {
                        eprintln!("[adapter] ✓ kreuzberg-csharp-sync (registered)");
                        kreuzberg_count += 1;
                    }
                }
                Err(err) => eprintln!("[adapter] ✗ kreuzberg-csharp-sync (initialization failed: {err})"),
            }

            eprintln!("[adapter] Kreuzberg bindings: {}/13 available", kreuzberg_count);

            use benchmark_harness::adapters::external::{
                create_docling_adapter, create_docling_batch_adapter, create_markitdown_adapter, create_mineru_adapter,
                create_mineru_batch_adapter, create_pandoc_adapter, create_pdfplumber_adapter,
                create_pdfplumber_batch_adapter, create_pymupdf4llm_adapter, create_tika_batch_adapter,
                create_tika_sync_adapter, create_unstructured_adapter,
            };

            let mut external_count = 0;

            if let Ok(adapter) = create_docling_adapter() {
                if let Ok(()) = registry.register(Arc::new(adapter)) {
                    eprintln!("[adapter] ✓ docling (registered)");
                    external_count += 1;
                } else {
                    eprintln!("[adapter] ✗ docling (registration failed)");
                }
            } else {
                eprintln!("[adapter] ✗ docling (initialization failed)");
            }

            if let Ok(adapter) = create_docling_batch_adapter() {
                if let Ok(()) = registry.register(Arc::new(adapter)) {
                    eprintln!("[adapter] ✓ docling-batch (registered)");
                    external_count += 1;
                } else {
                    eprintln!("[adapter] ✗ docling-batch (registration failed)");
                }
            } else {
                eprintln!("[adapter] ✗ docling-batch (initialization failed)");
            }

            if let Ok(adapter) = create_markitdown_adapter() {
                if let Ok(()) = registry.register(Arc::new(adapter)) {
                    eprintln!("[adapter] ✓ markitdown (registered)");
                    external_count += 1;
                } else {
                    eprintln!("[adapter] ✗ markitdown (registration failed)");
                }
            } else {
                eprintln!("[adapter] ✗ markitdown (initialization failed)");
            }

            if let Ok(adapter) = create_pandoc_adapter() {
                if let Ok(()) = registry.register(Arc::new(adapter)) {
                    eprintln!("[adapter] ✓ pandoc (registered)");
                    external_count += 1;
                } else {
                    eprintln!("[adapter] ✗ pandoc (registration failed)");
                }
            } else {
                eprintln!("[adapter] ✗ pandoc (initialization failed)");
            }

            if let Ok(adapter) = create_unstructured_adapter() {
                if let Ok(()) = registry.register(Arc::new(adapter)) {
                    eprintln!("[adapter] ✓ unstructured (registered)");
                    external_count += 1;
                } else {
                    eprintln!("[adapter] ✗ unstructured (registration failed)");
                }
            } else {
                eprintln!("[adapter] ✗ unstructured (initialization failed)");
            }

            if let Ok(adapter) = create_tika_sync_adapter() {
                if let Ok(()) = registry.register(Arc::new(adapter)) {
                    eprintln!("[adapter] ✓ tika-sync (registered)");
                    external_count += 1;
                } else {
                    eprintln!("[adapter] ✗ tika-sync (registration failed)");
                }
            } else {
                eprintln!("[adapter] ✗ tika-sync (initialization failed)");
            }

            if let Ok(adapter) = create_tika_batch_adapter() {
                if let Ok(()) = registry.register(Arc::new(adapter)) {
                    eprintln!("[adapter] ✓ tika-batch (registered)");
                    external_count += 1;
                } else {
                    eprintln!("[adapter] ✗ tika-batch (registration failed)");
                }
            } else {
                eprintln!("[adapter] ✗ tika-batch (initialization failed)");
            }

            if let Ok(adapter) = create_pymupdf4llm_adapter() {
                if let Ok(()) = registry.register(Arc::new(adapter)) {
                    eprintln!("[adapter] ✓ pymupdf4llm (registered)");
                    external_count += 1;
                } else {
                    eprintln!("[adapter] ✗ pymupdf4llm (registration failed)");
                }
            } else {
                eprintln!("[adapter] ✗ pymupdf4llm (initialization failed)");
            }

            if let Ok(adapter) = create_pdfplumber_adapter() {
                if let Ok(()) = registry.register(Arc::new(adapter)) {
                    eprintln!("[adapter] ✓ pdfplumber (registered)");
                    external_count += 1;
                } else {
                    eprintln!("[adapter] ✗ pdfplumber (registration failed)");
                }
            } else {
                eprintln!("[adapter] ✗ pdfplumber (initialization failed)");
            }

            if let Ok(adapter) = create_pdfplumber_batch_adapter() {
                if let Ok(()) = registry.register(Arc::new(adapter)) {
                    eprintln!("[adapter] ✓ pdfplumber-batch (registered)");
                    external_count += 1;
                } else {
                    eprintln!("[adapter] ✗ pdfplumber-batch (registration failed)");
                }
            } else {
                eprintln!("[adapter] ✗ pdfplumber-batch (initialization failed)");
            }

            if let Ok(adapter) = create_mineru_adapter() {
                if let Ok(()) = registry.register(Arc::new(adapter)) {
                    eprintln!("[adapter] ✓ mineru (registered)");
                    external_count += 1;
                } else {
                    eprintln!("[adapter] ✗ mineru (registration failed)");
                }
            } else {
                eprintln!("[adapter] ✗ mineru (initialization failed)");
            }

            if let Ok(adapter) = create_mineru_batch_adapter() {
                if let Ok(()) = registry.register(Arc::new(adapter)) {
                    eprintln!("[adapter] ✓ mineru-batch (registered)");
                    external_count += 1;
                } else {
                    eprintln!("[adapter] ✗ mineru-batch (registration failed)");
                }
            } else {
                eprintln!("[adapter] ✗ mineru-batch (initialization failed)");
            }

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

            use benchmark_harness::{write_by_extension_analysis, write_html, write_json};

            match format {
                OutputFormat::Json => {
                    let output_file = output.join("results.json");
                    write_json(&results, &output_file)?;
                    println!("\nResults written to: {}", output_file.display());

                    let by_ext_file = output.join("by-extension.json");
                    write_by_extension_analysis(&results, &by_ext_file)?;
                    println!("Per-extension analysis written to: {}", by_ext_file.display());
                }
                OutputFormat::Html => {
                    let html_file = output.join("index.html");
                    write_html(&results, &html_file, benchmark_date.as_deref())?;
                    println!("\nHTML report written to: {}", html_file.display());
                }
                OutputFormat::Both => {
                    let output_file = output.join("results.json");
                    write_json(&results, &output_file)?;
                    println!("\nResults written to: {}", output_file.display());

                    let by_ext_file = output.join("by-extension.json");
                    write_by_extension_analysis(&results, &by_ext_file)?;
                    println!("Per-extension analysis written to: {}", by_ext_file.display());

                    let html_file = output.join("index.html");
                    write_html(&results, &html_file, benchmark_date.as_deref())?;
                    println!("HTML report written to: {}", html_file.display());
                }
            }

            Ok(())
        }
        Commands::Consolidate {
            inputs,
            output,
            format,
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

            // Ensure output directory exists
            std::fs::create_dir_all(&output).map_err(benchmark_harness::Error::Io)?;

            match format {
                OutputFormat::Json => {
                    let output_file = output.join("consolidated.json");
                    write_consolidated_json(&consolidated, &output_file)?;
                    println!("\nConsolidated results written to: {}", output_file.display());
                }
                OutputFormat::Html => {
                    let html_file = output.join("consolidated.html");
                    write_simple_html(&consolidated, &html_file)?;
                    println!("\nConsolidated HTML report written to: {}", html_file.display());
                }
                OutputFormat::Both => {
                    let output_file = output.join("consolidated.json");
                    write_consolidated_json(&consolidated, &output_file)?;
                    println!("\nConsolidated results written to: {}", output_file.display());

                    let html_file = output.join("consolidated.html");
                    write_simple_html(&consolidated, &html_file)?;
                    println!("Consolidated HTML report written to: {}", html_file.display());
                }
            }

            Ok(())
        }

        Commands::Visualize {
            inputs,
            output,
            format,
            benchmark_date,
        } => {
            use benchmark_harness::{load_run_results, write_by_extension_analysis, write_html, write_json};

            if inputs.is_empty() {
                return Err(benchmark_harness::Error::Benchmark(
                    "No input directories specified".to_string(),
                ));
            }

            let mut results = Vec::new();
            for input in &inputs {
                if !input.is_dir() {
                    return Err(benchmark_harness::Error::Benchmark(format!(
                        "Input path is not a directory: {}",
                        input.display()
                    )));
                }
                let mut run_results = load_run_results(input)?;
                results.append(&mut run_results);
            }

            if results.is_empty() {
                return Err(benchmark_harness::Error::Benchmark(
                    "No benchmark results found to visualize".to_string(),
                ));
            }

            std::fs::create_dir_all(&output).map_err(benchmark_harness::Error::Io)?;

            match format {
                OutputFormat::Json => {
                    let output_file = output.join("results.json");
                    write_json(&results, &output_file)?;
                    println!("\nResults written to: {}", output_file.display());

                    let by_ext_file = output.join("by-extension.json");
                    write_by_extension_analysis(&results, &by_ext_file)?;
                    println!("Per-extension analysis written to: {}", by_ext_file.display());
                }
                OutputFormat::Html => {
                    let html_file = output.join("index.html");
                    write_html(&results, &html_file, benchmark_date.as_deref())?;
                    println!("\nHTML report written to: {}", html_file.display());
                }
                OutputFormat::Both => {
                    let output_file = output.join("results.json");
                    write_json(&results, &output_file)?;
                    println!("\nResults written to: {}", output_file.display());

                    let by_ext_file = output.join("by-extension.json");
                    write_by_extension_analysis(&results, &by_ext_file)?;
                    println!("Per-extension analysis written to: {}", by_ext_file.display());

                    let html_file = output.join("index.html");
                    write_html(&results, &html_file, benchmark_date.as_deref())?;
                    println!("HTML report written to: {}", html_file.display());
                }
            }

            Ok(())
        }
    }
}

/// Write simple consolidated HTML report
fn write_simple_html(consolidated: &benchmark_harness::ConsolidatedResults, path: &PathBuf) -> Result<()> {
    let mut html = String::from(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Consolidated Benchmark Report</title>
    <style>
        body { font-family: system-ui, -apple-system, sans-serif; margin: 20px; line-height: 1.6; }
        h1 { color: #333; border-bottom: 2px solid #0066cc; padding-bottom: 10px; }
        h2 { color: #0066cc; margin-top: 30px; }
        table { border-collapse: collapse; width: 100%; margin: 15px 0; }
        th, td { border: 1px solid #ddd; padding: 12px; text-align: left; }
        th { background-color: #0066cc; color: white; }
        tr:nth-child(even) { background-color: #f9f9f9; }
        .summary { background-color: #e7f3ff; padding: 15px; border-radius: 5px; margin: 15px 0; }
    </style>
</head>
<body>
    <h1>Consolidated Benchmark Report</h1>
"#,
    );

    html.push_str(&format!(
        r#"    <div class="summary">
        <h3>Summary</h3>
        <p>Total files processed: <strong>{}</strong></p>
        <p>Number of runs: <strong>{}</strong></p>
        <p>Frameworks analyzed: <strong>{}</strong></p>
    </div>
"#,
        consolidated.total_files, consolidated.run_count, consolidated.framework_count
    ));

    html.push_str("<h2>Framework Performance</h2>\n");
    html.push_str("<table>\n");
    html.push_str("    <tr><th>Framework</th><th>Files</th><th>Avg Duration (ms)</th><th>Std Dev (ms)</th><th>Success Rate</th><th>Avg Throughput (MB/s)</th></tr>\n");

    for (framework, agg) in &consolidated.by_framework {
        html.push_str(&format!(
            r#"    <tr>
        <td>{}</td>
        <td>{}</td>
        <td>{:.2}</td>
        <td>{:.2}</td>
        <td>{:.1}%</td>
        <td>{:.2}</td>
    </tr>
"#,
            framework,
            agg.total_files,
            agg.mean_duration_ms,
            agg.duration_std_dev_ms,
            agg.success_rate * 100.0,
            agg.mean_throughput_bps / 1_000_000.0
        ));
    }

    html.push_str("</table>\n");
    html.push_str("</body>\n</html>");

    std::fs::write(path, html).map_err(benchmark_harness::Error::Io)?;
    Ok(())
}
