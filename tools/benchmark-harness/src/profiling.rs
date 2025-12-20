//! CPU profiling module for benchmark analysis
//!
//! This module provides infrastructure for capturing CPU profiles during benchmark
//! execution using the pprof profiler. Profiles are captured at 1000 Hz frequency
//! and can be exported as SVG flamegraphs for performance analysis.
//!
//! # Feature Gate
//!
//! This module is only available when the `profiling` feature is enabled and the
//! target OS is not Windows (pprof has platform limitations).
//!
//! # Usage
//!
//! ```rust,no_run
//! # #[cfg(all(feature = "profiling", not(target_os = "windows")))]
//! # {
//! use benchmark_harness::profiling::ProfileGuard;
//! use std::path::Path;
//!
//! // Create a profiler guard
//! let mut guard = ProfileGuard::new(1000)?;
//!
//! // ... run code to profile ...
//!
//! // Finish profiling and generate flamegraph
//! let result = guard.finish()?;
//! result.generate_flamegraph(Path::new("profile.svg"))?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! # }
//! ```
//!
//! # Overhead
//!
//! Profiling at 1000 Hz typically adds 1-5% overhead to benchmark execution time.
//! The profiler blocks system libraries to reduce noise from standard library calls.

#[cfg(all(feature = "profiling", not(target_os = "windows")))]
use crate::Result;

#[cfg(all(feature = "profiling", not(target_os = "windows")))]
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
}

#[cfg(all(feature = "profiling", not(target_os = "windows")))]
impl ProfileGuard {
    /// Create a new CPU profiler with the specified frequency
    ///
    /// # Arguments
    ///
    /// * `frequency` - Sampling frequency in Hz (typically 1000)
    ///
    /// # Returns
    ///
    /// A new ProfileGuard or an error if profiling setup fails
    ///
    /// # Errors
    ///
    /// Returns [`Error::Profiling`](crate::Error::Profiling) if the profiler cannot be initialized.
    pub fn new(frequency: i32) -> Result<Self> {
        // Build profiler with blocklist for system libraries
        let guard = pprof::ProfilerGuardBuilder::default()
            .frequency(frequency)
            .blocklist(&["libc", "libpthread", "libgcc", "libm"])
            .build()
            .map_err(|e| crate::Error::Profiling(format!("Failed to initialize profiler: {}", e)))?;

        Ok(Self {
            guard: Some(guard),
            start_time: std::time::Instant::now(),
        })
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

        // Take ownership of guard and generate report
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
            sample_count: 0, // pprof doesn't expose sample count directly
            report,
        })
    }
}

#[cfg(all(feature = "profiling", not(target_os = "windows")))]
impl Drop for ProfileGuard {
    fn drop(&mut self) {
        // Guard is dropped automatically when the Option is dropped
        // pprof's ProfilerGuard handles cleanup in its own Drop implementation
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
        // Create parent directories if needed
        if let Some(parent) = output_path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)
                .map_err(|e| crate::Error::Profiling(format!("Failed to create output directory: {}", e)))?;
        }

        // Open file for writing
        let file = std::fs::File::create(output_path)
            .map_err(|e| crate::Error::Profiling(format!("Failed to create output file: {}", e)))?;

        // Generate and write flamegraph
        self.report
            .flamegraph(file)
            .map_err(|e| crate::Error::Profiling(format!("Failed to generate flamegraph: {}", e)))?;

        // Log output path to stderr
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
    pub struct ProfileGuard;

    impl ProfileGuard {
        /// Create a no-op profiler (always succeeds)
        pub fn new(_frequency: i32) -> Result<Self> {
            Ok(ProfileGuard)
        }

        /// Finish no-op profiling
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
        pub fn generate_flamegraph(&self, _output_path: &Path) -> Result<()> {
            eprintln!("Profiling is not available on this platform or feature is disabled");
            Ok(())
        }
    }
}

/// Re-export the appropriate implementation based on feature and platform
#[cfg(not(all(feature = "profiling", not(target_os = "windows"))))]
pub use noop::{ProfileGuard, ProfilingResult};

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
        #[ignore] // pprof has safety issues in test environments
        fn test_profile_guard_creation() -> crate::Result<()> {
            let _guard = ProfileGuard::new(1000)?;
            Ok(())
        }

        #[test]
        #[ignore] // pprof has safety issues in test environments
        fn test_generate_flamegraph() -> crate::Result<()> {
            let guard = ProfileGuard::new(1000)?;

            // Simulate some work
            let _sum: u64 = (0..1_000_000).sum();

            let result = guard.finish()?;

            // Create temporary file
            let temp_dir = TempDir::new()?;
            let output_path = temp_dir.path().join("profile.svg");

            result.generate_flamegraph(&output_path)?;

            // Verify file exists
            assert!(output_path.exists(), "Flamegraph file should exist");

            Ok(())
        }

        #[test]
        #[ignore] // pprof has safety issues in test environments
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
