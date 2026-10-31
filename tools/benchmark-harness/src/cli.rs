//! Command-line surface: the `clap` argument grammar for `benchmark-harness`.
//!
//! Kept separate from `main.rs` so the argument grammar (this file) is reviewable independently
//! of the command dispatch and implementations, which live under `commands/`.

use benchmark_harness::BenchmarkMode;
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// CLI enum for benchmark mode
#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum CliMode {
    /// Single-file mode: Sequential execution for fair latency comparison
    SingleFile,
    /// Batch mode: Verified native framework batch APIs for throughput measurement
    Batch,
}

/// CLI enum for output file format
#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum OutputFileFormat {
    /// JSON format (default)
    Json,
}

/// CLI enum for the pinned benchmark artifact release cohort
#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum CliCohort {
    /// Native (non-OCR) fast PDF cohort
    Native,
    /// OCR fast PDF cohort
    Ocr,
    /// Office family (docx, doc, pptx, ppt, xlsx, odt, rtf)
    Office,
    /// Markup family (html, md, latex, typst, rst, org, docbook)
    Markup,
    /// E-book family (epub, fb2)
    Ebook,
    /// E-mail family (eml, msg)
    Email,
    /// Structured-data family (csv, tsv, json, yaml)
    Data,
    /// OCR document-image family (png, jpeg, tiff)
    Images,
}

impl From<CliCohort> for benchmark_harness::bench_matrix::Cohort {
    fn from(cohort: CliCohort) -> Self {
        use benchmark_harness::bench_matrix::Cohort;
        match cohort {
            CliCohort::Native => Cohort::Native,
            CliCohort::Ocr => Cohort::Ocr,
            CliCohort::Office => Cohort::Office,
            CliCohort::Markup => Cohort::Markup,
            CliCohort::Ebook => Cohort::Ebook,
            CliCohort::Email => Cohort::Email,
            CliCohort::Data => Cohort::Data,
            CliCohort::Images => Cohort::Images,
        }
    }
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
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
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
    Run(crate::commands::run::RunArgs),

    /// Consolidate multiple benchmark runs
    Consolidate {
        /// Input directories containing benchmark results
        #[arg(short, long, value_delimiter = ',')]
        inputs: Vec<PathBuf>,

        /// Output directory for consolidated results
        #[arg(short, long)]
        output: PathBuf,

        /// Baseline framework for delta calculations (not used but provided for compatibility)
        #[arg(long, default_value = "xberg-rust")]
        baseline: String,
    },

    /// Measure framework installation sizes
    MeasureFrameworkSizes {
        /// Output JSON file for framework sizes
        #[arg(long)]
        output: PathBuf,
    },

    /// Build a per-document gap report from a `run` result set.
    ///
    /// Pivots `results.json` by document and ranks the documents where
    /// competitors beat our heuristics path, split by text (TF1) vs structure
    /// (SF1). Writes `per_document.json` + `gaps.md`.
    GapReport {
        /// Directory containing `results.json` (as produced by `run`)
        #[arg(short, long, default_value = "results")]
        results: PathBuf,

        /// Output directory for `per_document.json` + `gaps.md` (defaults to the results dir)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Our model-free heuristics-path framework row
        #[arg(long, default_value = "xberg-markdown-baseline")]
        baseline: String,

        /// Our routed ML-layout framework row
        #[arg(long, default_value = "xberg-markdown-layout")]
        layout: String,

        /// Competitor frameworks to rank against (comma-separated)
        #[arg(long, value_delimiter = ',', default_value = "liteparse,docling")]
        competitors: Vec<String>,
    },

    /// Compare extraction pipelines on document corpus with quality scoring
    Compare(crate::commands::compare::CompareArgs),

    /// Run 6-path pipeline benchmark across the document corpus
    PipelineBenchmark(crate::commands::pipeline_benchmark::PipelineBenchmarkArgs),

    /// Corpus-wide extraction survey with stats
    Survey {
        /// Directory containing fixture JSON files
        #[arg(short, long)]
        fixtures: PathBuf,

        /// File types to include (comma-separated, e.g. pdf,docx)
        #[arg(long, value_delimiter = ',')]
        types: Option<Vec<String>>,
    },

    /// Layout model A/B comparison benchmark
    ModelBenchmark {
        /// Directory containing fixture JSON files
        #[arg(short, long)]
        fixtures: PathBuf,

        /// First layout detector (rtdetr or pp_doclayout_v3)
        #[arg(long, default_value = "rtdetr")]
        model_a: String,

        /// Second layout detector (rtdetr or pp_doclayout_v3)
        #[arg(long, default_value = "pp_doclayout_v3")]
        model_b: String,

        /// Exact ordered fixture cohort; missing members fail the run
        #[arg(long, default_value = "tools/benchmark-harness/cohorts/layout-pdf-fast.json")]
        cohort: PathBuf,

        /// Maximum physical pages per PDF, starting at page one
        #[arg(long, default_value_t = 3)]
        max_pages: usize,

        /// Paired repeats per rendered page
        #[arg(long, default_value_t = 1)]
        iterations: usize,

        /// Shared native PDF render DPI
        #[arg(long, default_value_t = 150)]
        dpi: i32,

        /// Explicit common confidence threshold; generic layout heuristics stay disabled
        #[arg(long, default_value_t = 0.5)]
        confidence_threshold: f32,

        /// Independently expected total pages after applying the per-document limit
        #[arg(long)]
        expected_pages: Option<usize>,

        /// Hash-bound COCO annotations and explicit label projections; opt-in bbox metrics only
        #[arg(long)]
        geometry_annotations: Option<PathBuf>,

        /// Complete detector artifact with optional bbox metrics; not qualification
        #[arg(long, default_value = "benchmark-results/layout-models.json")]
        output: PathBuf,
    },

    /// Validate a saved detector artifact and recompute attached bbox metrics
    ModelBenchmarkValidate {
        #[arg(long)]
        artifact: PathBuf,
    },

    /// Multi-document PDF split-boundary benchmark (Auto boundary accuracy,
    /// reconstruction fidelity, single-parse timing)
    SplitBenchmark {
        /// Directory scanned recursively for `*.split.json` manifests
        #[arg(short, long, default_value = "tools/benchmark-harness/fixtures/split")]
        fixtures: PathBuf,

        /// Also run the MultidocThresholds sweep grid
        #[arg(long)]
        sweep: bool,

        /// Write a split-boundary-guardrails.json to this path (from default-threshold results)
        #[arg(long)]
        guardrails_out: Option<PathBuf>,

        /// Generate a CPU flamegraph SVG for the single-parse path at this path
        #[arg(long)]
        profile_out: Option<PathBuf>,
    },

    /// Embedding throughput and batch-size benchmark across all presets
    EmbedBenchmark,

    /// Validate ground truth files and optionally fix HTML artifacts
    ValidateGt {
        /// Directory containing fixture JSON files
        #[arg(short, long)]
        fixtures: PathBuf,

        /// Auto-fix HTML tags in markdown ground truth files
        #[arg(long)]
        fix: bool,

        /// Fail (non-zero exit) if any fixture cannot load its ground truth — e.g. the
        /// reference-corpus cache was not restored. Used as a fast CI pre-check.
        #[arg(long)]
        strict: bool,
    },

    /// Validate the exact benchmark artifact/aggregate contract for one fixed cohort
    ///
    /// Two modes: with `--aggregated-file`, validates one consolidated `aggregated.json`;
    /// otherwise validates a directory of raw per-framework `run/{provenance,results}.json`
    /// artifacts, which requires `--artifacts-dir`, `--cohort-manifest`, `--fixtures-root`,
    /// `--source-sha`, and `--run-id`.
    ValidateArtifacts(crate::commands::validate_artifacts::ValidateArtifactsCliArgs),

    /// Print one cohort's pinned release contract, including its exact matrix cells, as a single
    /// JSON line. The release workflow consumes this so cohort metadata and workflow coverage have
    /// a single source of truth in bench_matrix.
    CohortContract {
        /// Which cohort's contract to print
        #[arg(long)]
        cohort: CliCohort,
    },
}
