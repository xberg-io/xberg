//! Resource monitoring for benchmark execution
//!
//! This module provides real-time monitoring of CPU and memory usage during
//! document extraction, with percentile calculations for performance analysis.
//! When the "memory-profiling" feature is enabled, provides additional allocation
//! hotspot analysis and heap snapshot tracking.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
use tokio::sync::Mutex;

/// Snapshot of memory state at a point in time
///
/// Captures both virtual memory metrics and optional heap allocation data.
/// Used for detailed memory growth analysis and leak detection.
#[derive(Debug, Clone)]
pub struct MemorySnapshot {
    /// Timestamp relative to monitoring start
    pub timestamp: Duration,
    /// Resident Set Size in bytes (actual physical memory)
    pub rss_bytes: u64,
    /// Virtual memory size in bytes
    pub vm_bytes: u64,
    /// Major page faults at this snapshot
    pub page_faults: u64,
    /// Heap allocated bytes (only available with memory-profiling feature)
    #[cfg(feature = "memory-profiling")]
    pub heap_allocated: Option<u64>,
}

impl MemorySnapshot {
    /// Create a new memory snapshot
    #[cfg(not(feature = "memory-profiling"))]
    fn new(timestamp: Duration, rss_bytes: u64, vm_bytes: u64, page_faults: u64) -> Self {
        Self {
            timestamp,
            rss_bytes,
            vm_bytes,
            page_faults,
        }
    }

    /// Create a new memory snapshot with optional heap data
    #[cfg(feature = "memory-profiling")]
    fn new(timestamp: Duration, rss_bytes: u64, vm_bytes: u64, page_faults: u64, heap_allocated: Option<u64>) -> Self {
        Self {
            timestamp,
            rss_bytes,
            vm_bytes,
            page_faults,
            heap_allocated,
        }
    }
}

/// Allocation site with count and size information
///
/// Only available when memory-profiling feature is enabled.
#[cfg(feature = "memory-profiling")]
#[derive(Debug, Clone)]
pub struct AllocationSite {
    /// Source location (file:line format)
    pub location: String,
    /// Total bytes allocated from this site
    pub bytes_allocated: u64,
    /// Number of allocations from this site
    pub allocation_count: u64,
}

/// Sample of resource usage at a point in time
#[derive(Debug, Clone, Copy)]
pub struct ResourceSample {
    /// Memory usage in bytes (RSS)
    pub memory_bytes: u64,
    /// Virtual memory size in bytes
    pub vm_size_bytes: u64,
    /// Major page faults count
    pub page_faults: u64,
    /// CPU usage percentage (0.0 - 100.0 * num_cpus)
    pub cpu_percent: f64,
    /// Timestamp when sample was taken (relative to monitoring start)
    pub timestamp_ms: u64,
}

/// Resource monitor that samples CPU and memory usage periodically
///
/// Tracks both low-level CPU/memory metrics and optional heap allocation data.
/// Use the "memory-profiling" feature for enhanced allocation analysis.
pub struct ResourceMonitor {
    samples: Arc<Mutex<Vec<ResourceSample>>>,
    snapshots: Arc<Mutex<Vec<MemorySnapshot>>>,
    running: Arc<AtomicBool>,
    pid: Pid,
}

impl ResourceMonitor {
    /// Create a new resource monitor for the current process
    ///
    /// Initializes monitoring structures without starting background sampling.
    /// Call `start()` to begin collecting metrics.
    pub fn new() -> Self {
        let pid = sysinfo::get_current_pid().expect("Failed to get current PID");
        Self {
            samples: Arc::new(Mutex::new(Vec::new())),
            snapshots: Arc::new(Mutex::new(Vec::new())),
            running: Arc::new(AtomicBool::new(false)),
            pid,
        }
    }

    /// Capture heap allocation statistics from jemalloc
    ///
    /// Only available when "memory-profiling" feature is enabled.
    /// Returns the number of bytes currently allocated on the heap.
    /// Returns None if jemalloc statistics are unavailable.
    #[cfg(feature = "memory-profiling")]
    fn capture_heap_stats() -> Option<u64> {
        use tikv_jemalloc_ctl::{epoch, stats};

        let _prev_epoch = epoch::mib().and_then(|e| e.advance()).ok()?;

        let allocated = stats::allocated::mib().and_then(|a| a.read()).ok()?;

        Some(allocated as u64)
    }

    /// Start monitoring resources in the background
    ///
    /// Spawns a background task that samples memory and CPU usage at the specified interval.
    /// When "memory-profiling" feature is enabled, also captures heap allocation data.
    ///
    /// # Arguments
    /// * `sample_interval` - How often to sample (e.g., Duration::from_millis(10))
    pub async fn start(&self, sample_interval: Duration) {
        if self.running.swap(true, Ordering::SeqCst) {
            return;
        }

        let samples = Arc::clone(&self.samples);
        let snapshots = Arc::clone(&self.snapshots);
        let running = Arc::clone(&self.running);
        let pid = self.pid;

        tokio::spawn(async move {
            let mut system = System::new();
            let start = std::time::Instant::now();

            let refresh_kind = ProcessRefreshKind::nothing().with_memory().with_cpu();

            while running.load(Ordering::SeqCst) {
                system.refresh_cpu_usage();

                system.refresh_processes_specifics(ProcessesToUpdate::Some(&[pid]), false, refresh_kind);

                if let Some(process) = system.process(pid) {
                    let elapsed = start.elapsed();

                    let cpu_count = num_cpus::get() as f64;
                    let normalized_cpu_percent = (process.cpu_usage() as f64) / cpu_count;

                    let sample = ResourceSample {
                        memory_bytes: process.memory(),
                        vm_size_bytes: process.virtual_memory(),
                        page_faults: 0,
                        cpu_percent: normalized_cpu_percent,
                        timestamp_ms: elapsed.as_millis() as u64,
                    };

                    #[cfg(feature = "memory-profiling")]
                    let heap_allocated = Self::capture_heap_stats();
                    #[cfg(not(feature = "memory-profiling"))]
                    let _heap_allocated: Option<u64> = None;

                    #[cfg(feature = "memory-profiling")]
                    let snapshot =
                        MemorySnapshot::new(elapsed, process.memory(), process.virtual_memory(), 0, heap_allocated);
                    #[cfg(not(feature = "memory-profiling"))]
                    let snapshot = MemorySnapshot::new(elapsed, process.memory(), process.virtual_memory(), 0);

                    samples.lock().await.push(sample);
                    snapshots.lock().await.push(snapshot);
                }

                tokio::time::sleep(sample_interval).await;
            }
        });
    }

    /// Stop monitoring and return collected samples
    pub async fn stop(&self) -> Vec<ResourceSample> {
        self.running.store(false, Ordering::SeqCst);

        tokio::time::sleep(Duration::from_millis(20)).await;

        let samples = self.samples.lock().await;
        samples.clone()
    }

    /// Retrieve all collected memory snapshots
    ///
    /// Returns snapshots captured during monitoring, including detailed
    /// memory state at each sampling point.
    pub async fn get_snapshots(&self) -> Vec<MemorySnapshot> {
        let snapshots = self.snapshots.lock().await;
        snapshots.clone()
    }

    /// Get the peak memory snapshot
    ///
    /// Returns the snapshot with the highest RSS memory usage.
    /// Returns None if no snapshots were collected.
    pub async fn peak_snapshot(&self) -> Option<MemorySnapshot> {
        let snapshots = self.snapshots.lock().await;
        snapshots.iter().max_by_key(|s| s.rss_bytes).cloned()
    }

    /// Analyze memory growth trajectory
    ///
    /// Returns a vector of (timestamp, rss_bytes) pairs representing
    /// the memory growth over time. Useful for identifying sustained
    /// growth vs temporary spikes.
    pub async fn growth_trajectory(&self) -> Vec<(Duration, u64)> {
        let snapshots = self.snapshots.lock().await;
        snapshots.iter().map(|s| (s.timestamp, s.rss_bytes)).collect()
    }

    /// Detect potential memory leaks
    ///
    /// A leak is detected if memory grows by >5% from start to end
    /// and the end memory is >20% of peak. This avoids false positives
    /// from temporary allocations.
    pub async fn detect_leaks(&self) -> bool {
        let snapshots = self.snapshots.lock().await;

        if snapshots.len() < 2 {
            return false;
        }

        let start_rss = snapshots[0].rss_bytes as f64;
        let end_rss = snapshots[snapshots.len() - 1].rss_bytes as f64;
        let peak_rss = snapshots.iter().map(|s| s.rss_bytes as f64).fold(0.0, f64::max);

        let growth_percent = ((end_rss - start_rss) / start_rss) * 100.0;
        let retained_percent = (end_rss / peak_rss) * 100.0;

        growth_percent > 5.0 && retained_percent > 20.0
    }

    /// Calculate percentile from samples
    ///
    /// # Arguments
    /// * `samples` - Sorted samples (will be sorted if not already)
    /// * `percentile` - Percentile to calculate (0.0 - 1.0)
    fn calculate_percentile(mut values: Vec<u64>, percentile: f64) -> u64 {
        if values.is_empty() {
            return 0;
        }

        values.sort_unstable();
        let index = ((values.len() as f64 - 1.0) * percentile) as usize;
        values[index]
    }

    /// Calculate resource statistics from samples and snapshots
    ///
    /// Computes comprehensive resource metrics including percentiles,
    /// growth rates, and memory leak detection.
    pub fn calculate_stats(samples: &[ResourceSample], snapshots: &[MemorySnapshot]) -> ResourceStats {
        if samples.is_empty() {
            return ResourceStats::default();
        }

        let memory_values: Vec<u64> = samples.iter().map(|s| s.memory_bytes).collect();
        let cpu_values: Vec<f64> = samples.iter().map(|s| s.cpu_percent).collect();
        let vm_values: Vec<u64> = samples.iter().map(|s| s.vm_size_bytes).collect();

        let peak_memory = *memory_values.iter().max().unwrap_or(&0);
        let peak_vm = *vm_values.iter().max().unwrap_or(&0);
        let avg_cpu = cpu_values.iter().sum::<f64>() / cpu_values.len() as f64;

        let memory_growth_rate_mb_s = if samples.len() >= 2 {
            let first_memory = memory_values[0];
            let last_memory = memory_values[memory_values.len() - 1];
            let duration_ms = samples[samples.len() - 1].timestamp_ms - samples[0].timestamp_ms;
            let duration_s = if duration_ms > 0 {
                duration_ms as f64 / 1000.0
            } else {
                1.0
            };

            let memory_delta_bytes = if last_memory > first_memory {
                (last_memory - first_memory) as f64
            } else {
                0.0
            };

            memory_delta_bytes / 1_048_576.0 / duration_s
        } else {
            0.0
        };

        let leak_detected = if snapshots.len() >= 2 {
            let start_rss = snapshots[0].rss_bytes as f64;
            let end_rss = snapshots[snapshots.len() - 1].rss_bytes as f64;
            let peak_rss = snapshots.iter().map(|s| s.rss_bytes as f64).fold(0.0, f64::max);

            if peak_rss > 0.0 {
                let growth_percent = ((end_rss - start_rss) / start_rss) * 100.0;
                let retained_percent = (end_rss / peak_rss) * 100.0;
                growth_percent > 5.0 && retained_percent > 20.0
            } else {
                false
            }
        } else {
            false
        };

        let total_page_faults = samples.last().map(|s| s.page_faults).unwrap_or(0);

        ResourceStats {
            peak_memory_bytes: peak_memory,
            peak_vm_bytes: peak_vm,
            total_page_faults,
            memory_growth_rate_mb_s,
            avg_cpu_percent: avg_cpu,
            p50_memory_bytes: Self::calculate_percentile(memory_values.clone(), 0.50),
            p95_memory_bytes: Self::calculate_percentile(memory_values.clone(), 0.95),
            p99_memory_bytes: Self::calculate_percentile(memory_values, 0.99),
            sample_count: samples.len(),
            snapshots: snapshots.to_vec(),
            #[cfg(feature = "memory-profiling")]
            allocation_hotspots: Vec::new(), // TODO: Extract from jemalloc profiles
            leak_detected,
        }
    }
}

impl Default for ResourceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Resource usage statistics
///
/// Aggregated metrics from benchmark execution including percentiles,
/// growth rates, and optional allocation hotspot analysis.
#[derive(Debug, Clone, Default)]
pub struct ResourceStats {
    /// Peak memory usage in bytes
    pub peak_memory_bytes: u64,
    /// Peak virtual memory size in bytes
    pub peak_vm_bytes: u64,
    /// Total major page faults
    pub total_page_faults: u64,
    /// Memory growth rate in MB/s
    pub memory_growth_rate_mb_s: f64,
    /// Average CPU usage percentage
    pub avg_cpu_percent: f64,
    /// 50th percentile (median) memory usage
    pub p50_memory_bytes: u64,
    /// 95th percentile memory usage
    pub p95_memory_bytes: u64,
    /// 99th percentile memory usage
    pub p99_memory_bytes: u64,
    /// Number of samples collected
    pub sample_count: usize,
    /// Complete memory snapshots for detailed analysis
    pub snapshots: Vec<MemorySnapshot>,
    /// Memory allocation hotspots (only with memory-profiling feature)
    #[cfg(feature = "memory-profiling")]
    pub allocation_hotspots: Vec<AllocationSite>,
    /// Whether memory leak was detected (RSA growing without release)
    pub leak_detected: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_percentile() {
        let values = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

        assert_eq!(ResourceMonitor::calculate_percentile(values.clone(), 0.0), 1);
        assert_eq!(ResourceMonitor::calculate_percentile(values.clone(), 0.5), 5);
        assert_eq!(ResourceMonitor::calculate_percentile(values.clone(), 0.95), 9);
        assert_eq!(ResourceMonitor::calculate_percentile(values, 1.0), 10);
    }

    #[test]
    fn test_calculate_percentile_single_value() {
        let values = vec![42];
        assert_eq!(ResourceMonitor::calculate_percentile(values, 0.5), 42);
    }

    #[test]
    fn test_calculate_percentile_empty() {
        let values = vec![];
        assert_eq!(ResourceMonitor::calculate_percentile(values, 0.5), 0);
    }

    #[tokio::test]
    async fn test_resource_monitor_basic() {
        let monitor = ResourceMonitor::new();

        monitor.start(Duration::from_millis(10)).await;
        tokio::time::sleep(Duration::from_millis(50)).await;
        let samples = monitor.stop().await;

        assert!(!samples.is_empty(), "Should have collected samples");
        assert!(samples.len() >= 3, "Should have at least 3 samples");
    }

    #[tokio::test]
    async fn test_resource_stats_calculation() {
        let samples = vec![
            ResourceSample {
                memory_bytes: 100,
                vm_size_bytes: 500,
                page_faults: 10,
                cpu_percent: 10.0,
                timestamp_ms: 0,
            },
            ResourceSample {
                memory_bytes: 200,
                vm_size_bytes: 600,
                page_faults: 20,
                cpu_percent: 20.0,
                timestamp_ms: 10,
            },
            ResourceSample {
                memory_bytes: 150,
                vm_size_bytes: 550,
                page_faults: 25,
                cpu_percent: 15.0,
                timestamp_ms: 20,
            },
        ];

        let snapshots = vec![
            MemorySnapshot::new(
                Duration::from_millis(0),
                100,
                500,
                10,
                #[cfg(feature = "memory-profiling")]
                None,
            ),
            MemorySnapshot::new(
                Duration::from_millis(10),
                200,
                600,
                20,
                #[cfg(feature = "memory-profiling")]
                None,
            ),
            MemorySnapshot::new(
                Duration::from_millis(20),
                150,
                550,
                25,
                #[cfg(feature = "memory-profiling")]
                None,
            ),
        ];

        let stats = ResourceMonitor::calculate_stats(&samples, &snapshots);

        assert_eq!(stats.peak_memory_bytes, 200);
        assert_eq!(stats.peak_vm_bytes, 600);
        assert_eq!(stats.total_page_faults, 25);
        assert_eq!(stats.p50_memory_bytes, 150);
        assert!((stats.avg_cpu_percent - 15.0).abs() < 0.1);
        assert_eq!(stats.sample_count, 3);
        assert!(stats.memory_growth_rate_mb_s >= 0.0);
        assert_eq!(stats.snapshots.len(), 3);
    }

    #[tokio::test]
    async fn test_resource_stats_empty() {
        let stats = ResourceMonitor::calculate_stats(&[], &[]);
        assert_eq!(stats.peak_memory_bytes, 0);
        assert_eq!(stats.sample_count, 0);
    }

    #[tokio::test]
    async fn test_leak_detection() {
        let snapshots = vec![
            MemorySnapshot::new(
                Duration::from_millis(0),
                1000,
                5000,
                0,
                #[cfg(feature = "memory-profiling")]
                None,
            ),
            MemorySnapshot::new(
                Duration::from_millis(10),
                2000,
                6000,
                0,
                #[cfg(feature = "memory-profiling")]
                None,
            ),
            MemorySnapshot::new(
                Duration::from_millis(20),
                1200,
                5500,
                0,
                #[cfg(feature = "memory-profiling")]
                None,
            ),
        ];

        let samples = vec![ResourceSample {
            memory_bytes: 1200,
            vm_size_bytes: 5500,
            page_faults: 0,
            cpu_percent: 0.0,
            timestamp_ms: 20,
        }];
        let stats = ResourceMonitor::calculate_stats(&samples, &snapshots);
        assert!(
            stats.leak_detected,
            "Should detect leak with >5% growth and >20% retention"
        );
    }

    #[tokio::test]
    async fn test_no_leak_detection_temporary_spike() {
        let snapshots = vec![
            MemorySnapshot::new(
                Duration::from_millis(0),
                1000,
                5000,
                0,
                #[cfg(feature = "memory-profiling")]
                None,
            ),
            MemorySnapshot::new(
                Duration::from_millis(10),
                5000,
                9000,
                0,
                #[cfg(feature = "memory-profiling")]
                None,
            ),
            MemorySnapshot::new(
                Duration::from_millis(20),
                1001,
                5001,
                0,
                #[cfg(feature = "memory-profiling")]
                None,
            ),
        ];

        let samples = vec![ResourceSample {
            memory_bytes: 1001,
            vm_size_bytes: 5001,
            page_faults: 0,
            cpu_percent: 0.0,
            timestamp_ms: 20,
        }];
        let stats = ResourceMonitor::calculate_stats(&samples, &snapshots);
        assert!(!stats.leak_detected, "Should not detect leak when memory is released");
    }

    #[tokio::test]
    async fn test_snapshot_collection() {
        let monitor = ResourceMonitor::new();

        monitor.start(Duration::from_millis(10)).await;
        tokio::time::sleep(Duration::from_millis(50)).await;

        let snapshots = monitor.get_snapshots().await;
        assert!(
            !snapshots.is_empty(),
            "Should have collected snapshots during monitoring"
        );

        let peak = monitor.peak_snapshot().await;
        assert!(peak.is_some(), "Should find peak snapshot");

        let trajectory = monitor.growth_trajectory().await;
        assert_eq!(
            trajectory.len(),
            snapshots.len(),
            "Trajectory should match snapshot count"
        );

        monitor.stop().await;
    }
}
