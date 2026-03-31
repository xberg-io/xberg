//! Benchmark harness for comparing document extraction frameworks.
//!
//! This crate provides infrastructure for benchmarking Kreuzberg against other
//! document extraction frameworks, measuring performance (throughput, memory, latency)
//! and quality (F1 scores, text accuracy).
//!
//! # Dual-use pattern
//!
//! The harness serves two distinct workflows through the CLI subcommands:
//!
//! - **CI benchmarking** (`run` / `consolidate`): automated multi-framework
//!   performance sweeps that produce JSON artifacts consumed by dashboards.
//!   `run` executes one framework at a time; `consolidate` merges per-framework
//!   result files into a single ranked report.
//!
//! - **Local quality assessment** (`compare` / `pipeline-benchmark`): interactive
//!   tools for developers tuning extraction quality. `compare` runs multiple
//!   Kreuzberg pipeline configurations side-by-side on the corpus, printing an
//!   SF1/TF1 table. `pipeline-benchmark` extends this with timing data.
//!
//! # Module organization
//!
//! | Module | Purpose |
//! |--------|---------|
//! | [`adapter`] / [`adapters`] | Framework adapter trait and concrete implementations (native, Node, Python, Ruby). |
//! | [`aggregate`] | Consolidation aggregation: groups results by framework/mode/file-type, computes percentiles. |
//! | [`comparison`] | Multi-pipeline quality comparison on the corpus with guardrail thresholds. |
//! | [`config`] | Configuration types for benchmark runs and profiling. |
//! | [`consolidate`] | Recursive loading of `results.json` files from disk. |
//! | [`corpus`] | Test corpus discovery and filtering. |
//! | [`fixture`] | Fixture loading and validation. |
//! | [`markdown_quality`] | Structural F1 scoring via fuzzy cross-type block matching. |
//! | [`quality`] | Token-level (bag-of-words) text and numeric F1 scoring. |
//! | [`runner`] | Benchmark execution orchestrator (warmup, iterations, resource monitoring). |
//! | [`stats`] | Percentile calculations (R-7 interpolation) and NaN sanitization. |
//! | [`types`] | Core data types (`BenchmarkResult`, `QualityMetrics`, etc.). |

pub mod adapter;
pub mod adapters;
pub mod aggregate;
pub mod comparison;
pub mod config;
pub mod consolidate;
pub mod corpus;
pub mod diagnostics;
pub mod embed_benchmark;
pub mod error;
pub mod fixture;
pub mod groups;
pub mod markdown_quality;
pub mod model_benchmark;
pub mod monitoring;
pub mod noise_detection;
pub mod output;
pub mod pipeline_benchmark;
pub mod pool_metrics;
pub mod profile_report;
pub mod profiling;
pub mod quality;
pub mod registry;
pub mod runner;
pub mod sizes;
pub mod stats;
pub mod survey;
pub mod types;
pub mod validate_gt;

pub use adapter::FrameworkAdapter;
pub use adapters::NativeAdapter;
pub use aggregate::{
    ComparisonData, ConsolidationMetadata, DeltaMetrics, DurationPercentiles, FileTypeAggregation,
    FrameworkModeAggregation, NewConsolidatedResults, Percentiles, PerformancePercentiles, QualityPercentiles,
    RankedFramework, aggregate_new_format,
};
pub use config::{BenchmarkConfig, BenchmarkMode, ProfilingConfig, load_framework_sizes};
pub use consolidate::load_run_results;
pub use error::{Error, Result};
pub use fixture::{Fixture, FixtureManager};
pub use monitoring::{ResourceMonitor, ResourceSample, ResourceStats};
pub use output::{write_by_extension_analysis, write_json};
pub use pool_metrics::{FilePoolMetrics, PoolMetricsReport};
pub use profile_report::{Hotspot, MemorySnapshot, ProfileReport};
pub use quality::{compute_quality, compute_quality_with_structure};
pub use registry::AdapterRegistry;
pub use runner::BenchmarkRunner;
pub use types::{BenchmarkResult, DiskSizeInfo, FrameworkCapabilities, PdfMetadata};

pub use sizes::{
    FrameworkSize, FrameworkSizes, load_framework_sizes as load_sizes_json, measure_framework_sizes,
    save_framework_sizes,
};
