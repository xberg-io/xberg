//! Benchmark harness for comparing document extraction frameworks
//!
//! This crate provides infrastructure for benchmarking Kreuzberg against other
//! document extraction frameworks, measuring performance (throughput, memory, latency)
//! and quality (F1 scores, text accuracy).

pub mod adapter;
pub mod adapters;
pub mod config;
pub mod consolidate;
pub mod error;
pub mod fixture;
pub mod html;
pub mod monitoring;
pub mod output;
pub mod profile_report;
pub mod profiling;
pub mod registry;
pub mod runner;
pub mod types;

pub use adapter::FrameworkAdapter;
pub use adapters::{NativeAdapter, NodeAdapter, PythonAdapter, RubyAdapter};
pub use config::{BenchmarkConfig, BenchmarkMode, ProfilingConfig};
pub use consolidate::{
    ConsolidatedResults, CrossFrameworkComparison, FrameworkAggregation, FrameworkQuality, QualityAnalysis,
    aggregate_by_framework, analyze_quality, compare_frameworks, consolidate_runs, load_run_results,
    write_consolidated_json,
};
pub use error::{Error, Result};
pub use fixture::{Fixture, FixtureManager};
pub use html::{generate_flamegraph_index, write_html};
pub use monitoring::{ResourceMonitor, ResourceSample, ResourceStats};
pub use output::{write_by_extension_analysis, write_json};
pub use profile_report::{Hotspot, MemorySnapshot, ProfileReport};
pub use registry::AdapterRegistry;
pub use runner::BenchmarkRunner;
pub use types::{BenchmarkResult, FrameworkCapabilities, PdfMetadata};
