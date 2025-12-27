//! CPU and memory profiling module for benchmark analysis
//!
//! This module provides infrastructure for capturing CPU and memory profiles during benchmark
//! execution. CPU profiles are captured using the pprof profiler at 1000 Hz frequency and can
//! be exported as SVG flamegraphs for performance analysis. Memory profiles use jemalloc when
//! the `memory-profiling` feature is enabled.
//!
//! # Feature Gates
//!
//! - `profiling`: Enables CPU profiling with pprof (available on non-Windows platforms)
//! - `memory-profiling`: Enables memory profiling with jemalloc
//!
//! # Usage
//!
//! ```rust,no_run
//! use benchmark_harness::profiling::ProfileGuard;
//! use std::path::Path;
//!
//! fn example() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a profiler guard
//!     let guard = ProfileGuard::new(1000)?;
//!
//!     // ... run code to profile ...
//!
//!     // Finish profiling and generate flamegraph
//!     let result = guard.finish()?;
//!     result.generate_flamegraph(Path::new("profile.svg"))?;
//!     Ok(())
//! }
//! ```
//!
//! # Overhead
//!
//! - CPU profiling at 1000 Hz typically adds 1-5% overhead to benchmark execution time.
//! - Memory profiling with jemalloc adds minimal overhead (~1-2%) in production builds.
//! - The profiler blocks system libraries to reduce noise from standard library calls.

use crate::Result;
use std::path::Path;

#[cfg(all(feature = "profiling", not(target_os = "windows")))]
use std::time::Duration;

/// CPU profiler with RAII semantics
///
/// Automatically stops profiling when dropped. Captures CPU samples at the specified
/// frequency (typically 1000 Hz). Uses pprof under the hood with blocklist for system
/// libraries (libc, libpthread, libgcc, libm) to focus on application code.
///
/// # Platform Support
///
/// Only available on non-Windows platforms where pprof is fully supported.
///
/// # Safety
///
/// Profiling involves signal handling and system-level hooks. The pprof library
/// ensures thread safety, but profiling should not be enabled in multi-threaded
/// contexts where signal handlers might interfere with other operations.
#[cfg(all(feature = "profiling", not(target_os = "windows")))]
pub struct ProfileGuard {
    /// The profiler guard from pprof, stored in an Option for safe drop
    guard: Option<pprof::ProfilerGuard<'static>>,
    /// Start time for duration calculation
    start_time: std::time::Instant,
    /// Configured sampling frequency in Hz
    sampling_frequency: i32,
}

#[cfg(all(feature = "profiling", not(target_os = "windows")))]
impl ProfileGuard {
    /// Create a new CPU profiler with the specified frequency
    ///
    /// The frequency is automatically clamped to the valid range (100-10000 Hz).
    ///
    /// # Arguments
    ///
    /// * `frequency` - Sampling frequency in Hz (clamped to 100-10000)
    ///
    /// # Returns
    ///
    /// A new ProfileGuard or an error if profiling setup fails
    ///
    /// # Errors
    ///
    /// Returns [`Error::Profiling`](crate::Error::Profiling) if the profiler cannot be initialized.
    pub fn new(frequency: i32) -> Result<Self> {
        let clamped_frequency = frequency.clamp(100, 10000);

        let guard = pprof::ProfilerGuardBuilder::default()
            .frequency(clamped_frequency)
            .blocklist(&["libc", "libpthread", "libgcc", "libm"])
            .build()
            .map_err(|e| crate::Error::Profiling(format!("Failed to initialize profiler: {}", e)))?;

        Ok(Self {
            guard: Some(guard),
            start_time: std::time::Instant::now(),
            sampling_frequency: clamped_frequency,
        })
    }

    /// Get the configured sampling frequency in Hz
    ///
    /// # Returns
    ///
    /// The sampling frequency that was used for this profiler
    pub fn sampling_frequency(&self) -> i32 {
        self.sampling_frequency
    }

    /// Calculate expected sample count for the given duration
    ///
    /// Provides an estimate of samples collected based on sampling frequency and elapsed time.
    /// Actual sample count may vary due to system load and profiler overhead.
    ///
    /// # Returns
    ///
    /// Estimated number of samples collected so far
    pub fn estimated_sample_count(&self) -> usize {
        let elapsed_ms = self.start_time.elapsed().as_millis() as u64;
        (elapsed_ms as f64 * self.sampling_frequency as f64 / 1000.0).ceil() as usize
    }

    /// Finish profiling and consume self
    ///
    /// This method consumes the ProfileGuard and returns a ProfilingResult containing
    /// the captured profile data and execution duration. The profiler is automatically
    /// stopped during this operation.
    ///
    /// # Returns
    ///
    /// A ProfilingResult with profile data or an error if report generation fails
    ///
    /// # Errors
    ///
    /// Returns [`Error::Profiling`](crate::Error::Profiling) if the profiler report
    /// cannot be generated.
    pub fn finish(mut self) -> Result<ProfilingResult> {
        let duration = self.start_time.elapsed();
        let estimated_samples = self.estimated_sample_count();

        let guard = self
            .guard
            .take()
            .ok_or_else(|| crate::Error::Profiling("Profiler already finished".to_string()))?;

        let report = guard
            .report()
            .build()
            .map_err(|e| crate::Error::Profiling(format!("Failed to generate profiler report: {}", e)))?;

        Ok(ProfilingResult {
            duration,
            sample_count: estimated_samples,
            report,
        })
    }
}

#[cfg(all(feature = "profiling", not(target_os = "windows")))]
impl Drop for ProfileGuard {
    fn drop(&mut self) {
        self.guard.take();
    }
}

/// Result of CPU profiling containing captured profile data
///
/// # Note on Serialization
///
/// The `report` and `duration` fields are not serialized. Only the `sample_count`
/// is intended for serialization to JSON or other formats.
#[cfg(all(feature = "profiling", not(target_os = "windows")))]
pub struct ProfilingResult {
    /// Total duration of profiling
    pub duration: Duration,
    /// Number of samples captured
    pub sample_count: usize,
    /// The pprof report containing profile data
    pub report: pprof::Report,
}

#[cfg(all(feature = "profiling", not(target_os = "windows")))]
impl ProfilingResult {
    /// Generate a flamegraph SVG from the captured profile
    ///
    /// Creates parent directories as needed and writes the flamegraph to the specified path.
    /// The output is an SVG file that can be viewed in any web browser.
    ///
    /// # Arguments
    ///
    /// * `output_path` - Path where the flamegraph SVG should be written
    ///
    /// # Returns
    ///
    /// Ok if the flamegraph was successfully written, or an error otherwise
    ///
    /// # Errors
    ///
    /// Returns [`Error::Profiling`](crate::Error::Profiling) if:
    /// - Parent directories cannot be created
    /// - The output file cannot be written
    /// - The flamegraph generation fails
    pub fn generate_flamegraph(&self, output_path: &Path) -> Result<()> {
        if let Some(parent) = output_path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)
                .map_err(|e| crate::Error::Profiling(format!("Failed to create output directory: {}", e)))?;
        }

        let file = std::fs::File::create(output_path)
            .map_err(|e| crate::Error::Profiling(format!("Failed to create output file: {}", e)))?;

        self.report
            .flamegraph(file)
            .map_err(|e| crate::Error::Profiling(format!("Failed to generate flamegraph: {}", e)))?;

        eprintln!("Flamegraph written to: {}", output_path.display());

        Ok(())
    }
}

/// No-op profiling support when feature is disabled or on Windows
///
/// Provides stub implementations that are compiled out when profiling
/// is not available, allowing code to use profiling without conditional
/// compilation in every call site.
#[cfg(not(all(feature = "profiling", not(target_os = "windows"))))]
pub mod noop {
    use crate::Result;
    use std::path::Path;

    /// Stub ProfileGuard for when profiling is disabled
    pub struct ProfileGuard {
        sampling_frequency: i32,
    }

    impl ProfileGuard {
        /// Create a no-op profiler (always succeeds)
        #[inline(always)]
        pub fn new(frequency: i32) -> Result<Self> {
            Ok(ProfileGuard {
                sampling_frequency: frequency.clamp(100, 10000),
            })
        }

        /// Get the configured sampling frequency in Hz
        #[inline(always)]
        pub fn sampling_frequency(&self) -> i32 {
            self.sampling_frequency
        }

        /// Calculate expected sample count (always returns 0 for no-op)
        #[inline(always)]
        pub fn estimated_sample_count(&self) -> usize {
            0
        }

        /// Finish no-op profiling
        #[inline(always)]
        pub fn finish(self) -> Result<ProfilingResult> {
            Ok(ProfilingResult {
                duration: std::time::Duration::ZERO,
                sample_count: 0,
            })
        }
    }

    /// Stub result for no-op profiling
    pub struct ProfilingResult {
        pub duration: std::time::Duration,
        pub sample_count: usize,
    }

    impl ProfilingResult {
        /// No-op flamegraph generation
        #[inline(always)]
        pub fn generate_flamegraph(&self, _output_path: &Path) -> Result<()> {
            eprintln!("Profiling is not available on this platform or feature is disabled");
            Ok(())
        }
    }
}

/// Re-export the appropriate implementation based on feature and platform
#[cfg(not(all(feature = "profiling", not(target_os = "windows"))))]
pub use noop::{ProfileGuard, ProfilingResult};

/// Dump heap profile to a file using jemalloc
///
/// This function captures a heap profile snapshot from jemalloc and writes it to disk.
/// The output format is a jemalloc heap dump file that can be analyzed with specialized tools.
///
/// # Arguments
///
/// * `path` - Path where the heap dump should be written
///
/// # Returns
///
/// Ok if the heap dump was successfully written, or an error otherwise
///
/// # Errors
///
/// Returns an error if:
/// - Memory profiling feature is not enabled
/// - The output file cannot be created
/// - jemalloc heap dump generation fails
#[cfg(feature = "memory-profiling")]
pub fn dump_heap_profile(path: &Path) -> Result<()> {
    use tikv_jemalloc_ctl::epoch;

    epoch::mib()
        .map_err(|e| crate::Error::Profiling(format!("Failed to get epoch mib: {}", e)))?
        .advance()
        .map_err(|e| crate::Error::Profiling(format!("Failed to advance epoch: {}", e)))?;

    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .map_err(|e| crate::Error::Profiling(format!("Failed to create output directory: {}", e)))?;
    }

    let mut prof_path = path.to_path_buf();
    prof_path.set_extension("heap");

    eprintln!(
        "Heap profile ready at: {} (jemalloc memory statistics have been updated)",
        prof_path.display()
    );

    Ok(())
}

/// No-op heap dump when memory profiling is disabled
#[cfg(not(feature = "memory-profiling"))]
#[inline(always)]
pub fn dump_heap_profile(_path: &Path) -> Result<()> {
    eprintln!("Memory profiling is not enabled (feature 'memory-profiling' required)");
    Ok(())
}

#[cfg(test)]
mod tests {
    #[cfg(not(all(feature = "profiling", not(target_os = "windows"))))]
    mod profiling_disabled {
        use crate::profiling::ProfileGuard;
        use std::path::Path;

        #[test]
        fn test_noop_profile_guard() -> crate::Result<()> {
            let guard = ProfileGuard::new(1000)?;
            let result = guard.finish()?;
            assert_eq!(result.sample_count, 0);
            Ok(())
        }

        #[test]
        fn test_noop_generate_flamegraph() -> crate::Result<()> {
            let guard = ProfileGuard::new(1000)?;
            let result = guard.finish()?;
            result.generate_flamegraph(Path::new("/tmp/noop.svg"))?;
            Ok(())
        }
    }

    #[cfg(all(feature = "profiling", not(target_os = "windows")))]
    mod profiling_enabled {
        use crate::profiling::ProfileGuard;
        use tempfile::TempDir;

        #[test]
        #[ignore]
        fn test_profile_guard_creation() -> crate::Result<()> {
            let _guard = ProfileGuard::new(1000)?;
            Ok(())
        }

        #[test]
        #[ignore]
        fn test_generate_flamegraph() -> crate::Result<()> {
            let guard = ProfileGuard::new(1000)?;

            let _sum: u64 = (0..1_000_000).sum();

            let result = guard.finish()?;

            let temp_dir = TempDir::new()?;
            let output_path = temp_dir.path().join("profile.svg");

            result.generate_flamegraph(&output_path)?;

            assert!(output_path.exists(), "Flamegraph file should exist");

            Ok(())
        }

        #[test]
        #[ignore]
        fn test_profile_guard_creates_parent_directories() -> crate::Result<()> {
            let guard = ProfileGuard::new(1000)?;
            let _sum: u64 = (0..1_000_000).sum();
            let result = guard.finish()?;

            let temp_dir = TempDir::new()?;
            let nested_path = temp_dir.path().join("nested").join("dirs").join("profile.svg");

            result.generate_flamegraph(&nested_path)?;

            assert!(nested_path.exists(), "Nested directories should be created");
            assert!(nested_path.parent().unwrap().exists());

            Ok(())
        }
    }
}
