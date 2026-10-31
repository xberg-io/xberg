//! System-load capture for benchmark measurements.
//!
//! Every [`crate::types::BenchmarkResult`] records the system load at the moment
//! it was measured. Local benchmarks share the machine with editors, browsers,
//! OS scanners, and this harness itself, so a throughput or cold-start number is
//! only comparable to another taken under similar load. Recording the load per
//! result lets the analysis layer flag high-contention outliers and verify that
//! two frameworks were measured under a comparable load profile — the
//! apples-to-apples check.
//!
//! Note: the load average includes the workload being measured, so during a
//! heavy multi-threaded extraction the numbers are expected to be high. The
//! useful signals are therefore *relative* (did framework A and framework B run
//! under similar load?) and *outlier* (was this one document measured while the
//! machine was unusually busy?), not the absolute value on its own.

use serde::{Deserialize, Serialize};

/// 1-minute load-per-logical-core above which timing measurements are treated as
/// contended and not directly comparable to an idle-machine baseline.
pub const CONTENDED_LOAD_PER_CORE: f64 = 0.7;

/// A snapshot of system load captured at benchmark measurement time.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SystemLoad {
    /// 1-minute load average (running + runnable tasks, OS-reported).
    pub load_avg_1m: f64,
    /// 5-minute load average.
    pub load_avg_5m: f64,
    /// 15-minute load average.
    pub load_avg_15m: f64,
    /// Logical (hyperthreaded) core count.
    pub logical_cores: usize,
    /// Physical core count.
    pub physical_cores: usize,
}

impl SystemLoad {
    /// Capture the current system load.
    ///
    /// Returns an error when the operating system cannot supply a load average.
    pub fn capture() -> crate::Result<Self> {
        Self::from_average(sysinfo::System::load_average())
    }

    fn from_average(average: Result<sysinfo::LoadAvg, sysinfo::Error>) -> crate::Result<Self> {
        let avg = average.map_err(|error| crate::Error::Benchmark(format!("capture system load: {error}")))?;
        Ok(Self {
            load_avg_1m: avg.one,
            load_avg_5m: avg.five,
            load_avg_15m: avg.fifteen,
            logical_cores: num_cpus::get(),
            physical_cores: num_cpus::get_physical(),
        })
    }

    /// 1-minute load normalized by logical core count.
    ///
    /// This is the headline "how busy was the machine" figure: `1.0` means the
    /// run had, on average, one runnable task per logical core over the last
    /// minute. Returns `0.0` if the core count is unknown.
    pub fn load_per_core(&self) -> f64 {
        if self.logical_cores == 0 {
            0.0
        } else {
            self.load_avg_1m / self.logical_cores as f64
        }
    }

    /// True when background load is high enough that timing measured now is not
    /// comparable to an idle-machine baseline.
    pub fn is_contended(&self) -> bool {
        self.load_per_core() > CONTENDED_LOAD_PER_CORE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_report_zero_load_per_core_when_cores_unknown() {
        let load = SystemLoad {
            load_avg_1m: 8.0,
            load_avg_5m: 8.0,
            load_avg_15m: 8.0,
            logical_cores: 0,
            physical_cores: 0,
        };
        assert_eq!(load.load_per_core(), 0.0);
        assert!(!load.is_contended());
    }

    #[test]
    fn should_normalize_load_by_logical_cores() {
        let load = SystemLoad {
            load_avg_1m: 7.0,
            load_avg_5m: 6.0,
            load_avg_15m: 5.0,
            logical_cores: 14,
            physical_cores: 10,
        };
        assert!((load.load_per_core() - 0.5).abs() < 1e-9);
        assert!(!load.is_contended());
    }

    #[test]
    fn should_flag_contended_when_load_per_core_exceeds_threshold() {
        let load = SystemLoad {
            load_avg_1m: 12.0,
            load_avg_5m: 11.0,
            load_avg_15m: 10.0,
            logical_cores: 14,
            physical_cores: 10,
        };
        assert!(load.load_per_core() > CONTENDED_LOAD_PER_CORE);
        assert!(load.is_contended());
    }

    #[test]
    fn should_capture_populated_core_counts() {
        let load = SystemLoad::capture().expect("host system load must be available");
        assert!(load.logical_cores >= 1);
        assert!(load.physical_cores >= 1);
    }

    #[test]
    fn should_report_unavailable_load_instead_of_recording_an_idle_host() {
        let error = SystemLoad::from_average(Err(sysinfo::Error::Unsupported)).unwrap_err();
        assert_eq!(error.to_string(), "Benchmark error: capture system load: unsupported");
    }
}
