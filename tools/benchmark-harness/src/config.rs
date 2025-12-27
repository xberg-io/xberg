//! Benchmark configuration

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Benchmark execution mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BenchmarkMode {
    /// Single-file mode: Sequential execution (max_concurrent=1) for fair latency comparison
    SingleFile,
    /// Batch mode: Concurrent execution to measure throughput
    Batch,
}

/// CPU/memory profiling configuration for benchmark analysis
///
/// Controls adaptive sampling frequency, task duration amplification, and sample collection
/// thresholds to ensure high-quality profiles with 500-5000 samples per run.
///
/// # Sampling Frequency
///
/// The sampling frequency (100-10000 Hz) is automatically adjusted based on task duration:
/// - Quick tasks (<100ms): Higher frequency (up to 10000 Hz)
/// - Medium tasks (100-1000ms): Standard frequency (1000 Hz)
/// - Long tasks (>1000ms): Lower frequency (100-1000 Hz)
///
/// # Task Duration Amplification
///
/// When profiling is enabled, tasks can be amplified (repeated multiple times) to increase
/// profiling duration and reduce variance in sample collection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilingConfig {
    /// Enable/disable CPU profiling
    pub enabled: bool,

    /// CPU sampling frequency in Hz (100-10000)
    /// Adjusted adaptively based on estimated task duration
    pub sampling_frequency: i32,

    /// Minimum task duration in milliseconds for adaptive frequency calculation
    /// Tasks shorter than this use higher sampling frequencies
    pub task_duration_ms: u64,

    /// Number of documents per profiling batch
    /// Larger batches provide more samples but increase memory usage
    pub batch_size: usize,

    /// Memory sample collection interval in milliseconds (0 = disabled)
    pub memory_sampling_interval_ms: u64,

    /// Enable flamegraph generation after profiling completes
    pub flamegraph_enabled: bool,

    /// Minimum number of samples required for a valid profile
    /// Profiles with fewer samples may have high variance
    pub sample_count_threshold: usize,
}

impl Default for ProfilingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            sampling_frequency: 1000,
            task_duration_ms: 500,
            batch_size: 10,
            memory_sampling_interval_ms: 10,
            flamegraph_enabled: true,
            sample_count_threshold: 500,
        }
    }
}

impl ProfilingConfig {
    /// Validate the profiling configuration
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Config`] if any configuration value is invalid
    pub fn validate(&self) -> crate::Result<()> {
        if self.sampling_frequency < 100 || self.sampling_frequency > 10000 {
            return Err(crate::Error::Config(format!(
                "sampling_frequency must be 100-10000 Hz, got {}",
                self.sampling_frequency
            )));
        }

        if self.batch_size == 0 {
            return Err(crate::Error::Config("batch_size must be > 0".to_string()));
        }

        if self.sample_count_threshold == 0 {
            return Err(crate::Error::Config("sample_count_threshold must be > 0".to_string()));
        }

        Ok(())
    }

    /// Calculate optimal sampling frequency based on estimated task duration
    ///
    /// Uses realistic sysinfo limits (100-500 Hz) to achieve target sample count.
    /// sysinfo cannot reliably achieve >500 Hz on most systems due to:
    /// - Process scheduling granularity
    /// - System call overhead
    /// - File descriptor refresh costs
    ///
    /// Target: 500 samples minimum for statistical significance
    ///
    /// # Arguments
    ///
    /// * `estimated_duration_ms` - Estimated task duration in milliseconds
    ///
    /// # Returns
    ///
    /// Optimal sampling frequency in Hz (clamped to 100-500 range)
    pub fn calculate_optimal_frequency(estimated_duration_ms: u64) -> i32 {
        const TARGET_SAMPLE_COUNT: u64 = 500;
        const REALISTIC_MAX_HZ: i32 = 500;

        if estimated_duration_ms == 0 {
            return REALISTIC_MAX_HZ;
        }

        let required_hz = (TARGET_SAMPLE_COUNT * 1000) / estimated_duration_ms.max(1);
        (required_hz as i32).clamp(100, REALISTIC_MAX_HZ)
    }

    /// Calculate sampling interval in milliseconds from frequency in Hz
    ///
    /// Converts sampling frequency to the actual interval between samples.
    ///
    /// # Arguments
    ///
    /// * `sampling_frequency_hz` - Sampling frequency in Hz
    ///
    /// # Returns
    ///
    /// Sampling interval in milliseconds (minimum 1ms)
    pub fn calculate_sample_interval_ms(sampling_frequency_hz: i32) -> u64 {
        (1000 / sampling_frequency_hz as u64).max(1)
    }
}

/// Configuration for benchmark runs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkConfig {
    /// Maximum file size to process (bytes)
    pub max_file_size: Option<u64>,

    /// File types to include (e.g., ["pdf", "docx"])
    pub file_types: Option<Vec<String>>,

    /// Timeout for each extraction
    pub timeout: Duration,

    /// Maximum number of concurrent extractions
    pub max_concurrent: usize,

    /// Output directory for results
    pub output_dir: PathBuf,

    /// Whether to include quality assessment
    pub measure_quality: bool,

    /// Sample interval for resource monitoring (milliseconds)
    pub sample_interval_ms: u64,

    /// Benchmark execution mode (single-file or batch)
    pub benchmark_mode: BenchmarkMode,

    /// Number of warmup iterations (discarded from statistics)
    pub warmup_iterations: usize,

    /// Number of benchmark iterations for statistical analysis
    pub benchmark_iterations: usize,

    /// Profiling configuration for CPU/memory analysis
    pub profiling: ProfilingConfig,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            max_file_size: None,
            file_types: None,
            timeout: Duration::from_secs(1800),
            max_concurrent: num_cpus::get(),
            output_dir: PathBuf::from("results"),
            measure_quality: false,
            sample_interval_ms: 10,
            benchmark_mode: BenchmarkMode::Batch,
            warmup_iterations: 1,
            benchmark_iterations: 3,
            profiling: ProfilingConfig::default(),
        }
    }
}

impl BenchmarkConfig {
    /// Validate the configuration
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Config`] if any configuration value is invalid
    pub fn validate(&self) -> crate::Result<()> {
        if self.timeout.as_secs() == 0 {
            return Err(crate::Error::Config("Timeout must be > 0".to_string()));
        }

        if self.max_concurrent == 0 {
            return Err(crate::Error::Config("max_concurrent must be > 0".to_string()));
        }

        if self.sample_interval_ms == 0 {
            return Err(crate::Error::Config("sample_interval_ms must be > 0".to_string()));
        }

        if self.benchmark_iterations == 0 {
            return Err(crate::Error::Config("benchmark_iterations must be > 0".to_string()));
        }

        if self.benchmark_mode == BenchmarkMode::SingleFile && self.max_concurrent != 1 {
            return Err(crate::Error::Config(
                "single-file mode requires max_concurrent=1".to_string(),
            ));
        }

        self.profiling.validate()?;

        Ok(())
    }
}
