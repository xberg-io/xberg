//! Resource monitoring for benchmark execution
//!
//! This module provides real-time monitoring of CPU and memory usage during
//! document extraction, with percentile calculations for performance analysis.
//! When the "memory-profiling" feature is enabled, heap allocation snapshots
//! are captured alongside RSS and virtual-memory samples.
//!
//! # Measurement Methodology
//!
//! Both memory and CPU measurements include the entire process tree (parent + all
//! child processes). This is critical for accurate measurement of extraction
//! frameworks that spawn subprocesses (e.g., pandoc, tika). Without this,
//! measurements would only capture the idle wrapper process, not the actual
//! extraction work happening in child processes.
//!
//! Changed in v4.0: Previously only measured parent process memory.
//! Changed in v4.3.7: CPU now also measures the entire process tree (previously
//! only measured parent process CPU, causing near-zero readings for subprocess-based
//! frameworks).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::time::Duration;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

/// Calculate adaptive sampling interval based on file size.
///
/// Small files (<100KB) use 1ms sampling for fine-grained measurement.
/// Medium files (100KB-10MB) use 5ms sampling.
/// Large files (>10MB) use 10ms sampling to reduce overhead.
pub fn adaptive_sampling_interval_ms(file_size: u64) -> u64 {
    if file_size < 100_000 {
        1
    } else if file_size < 10_000_000 {
        5
    } else {
        10
    }
}

/// Snapshot of memory state at a point in time.
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

/// Sample of resource usage at a point in time
#[derive(Debug, Clone, Copy)]
pub struct ResourceSample {
    /// Memory usage in bytes (RSS)
    pub memory_bytes: u64,
    /// Virtual memory size in bytes
    pub vm_size_bytes: u64,
    /// Major page faults count
    pub page_faults: u64,
    /// CPU usage percentage normalized across cores (0.0 - 100.0)
    /// Includes the entire process tree (parent + all child processes).
    pub cpu_percent: f64,
    /// Timestamp when sample was taken (relative to monitoring start)
    pub timestamp_ms: u64,
}

/// Collect all child process IDs for a given parent process
///
/// Recursively finds all descendants in the process tree by iterating through
/// all system processes and checking parent PIDs.
///
/// Threads are explicitly excluded. On Linux, sysinfo enumerates a process's
/// threads (tasks) as entries whose `parent()` is the owning process PID. Each
/// thread reports the *whole* process RSS (threads share one address space), so
/// summing them in `collect_process_tree_memory` multiplies real RSS by the
/// thread count. That is exactly what inflated the ORT-backed layout pipeline to
/// physically impossible peaks (p95 ~138 GB, p99 ~310 GB) while single-threaded
/// paths stayed correct. `thread_kind()` is `Some(..)` for threads and `None`
/// for real processes, so filtering on it counts each address space once.
fn get_child_processes(parent_pid: Pid, system: &System) -> Vec<Pid> {
    system
        .processes()
        .iter()
        .filter_map(|(pid, proc)| {
            if proc.parent() == Some(parent_pid) && proc.thread_kind().is_none() {
                Some(*pid)
            } else {
                None
            }
        })
        .collect()
}

/// Collect total memory usage from a process and all its descendants
///
/// Recursively traverses the process tree, summing RSS memory from the parent
/// and all child processes. This is essential for accurately measuring frameworks
/// that spawn subprocesses for extraction work.
///
/// # Arguments
/// * `pid` - The root process ID to measure
/// * `system` - System instance with refreshed process information
///
/// # Returns
/// Total RSS memory in bytes for the entire process tree
fn collect_process_tree_memory(pid: Pid, system: &System) -> u64 {
    let mut total = 0;

    if let Some(proc) = system.process(pid) {
        total += proc.memory();

        for child_pid in get_child_processes(pid, system) {
            total += collect_process_tree_memory(child_pid, system);
        }
    }

    total
}

/// Collect total virtual memory usage from a process and all its descendants
///
/// Similar to collect_process_tree_memory but for virtual memory size.
///
/// # Arguments
/// * `pid` - The root process ID to measure
/// * `system` - System instance with refreshed process information
///
/// # Returns
/// Total virtual memory in bytes for the entire process tree
fn collect_process_tree_vm(pid: Pid, system: &System) -> u64 {
    let mut total = 0;

    if let Some(proc) = system.process(pid) {
        total += proc.virtual_memory();

        for child_pid in get_child_processes(pid, system) {
            total += collect_process_tree_vm(child_pid, system);
        }
    }

    total
}

/// Approximate total CPU-time consumed by the process tree in core-seconds, via trapezoidal
/// integration of the per-sample CPU percentage over the sampled timeline.
///
/// `ResourceSample::cpu_percent` is normalized to a fraction of total system capacity (divided
/// by `logical_cores` in [`ResourceMonitor::collect_sample`]); this recovers the un-normalized
/// (raw, `0..=100*logical_cores`) percentage before integrating, since core-seconds measures
/// actual CPU-time consumed rather than a per-core-normalized rate.
///
/// `logical_cores` is taken as a parameter (rather than calling `num_cpus::get()` internally)
/// so the integration math is independently unit-testable regardless of the host's actual core
/// count. Precision is bounded by the sampling interval (1-10ms, adaptive on file size, see
/// [`adaptive_sampling_interval_ms`]): CPU bursts shorter than the gap between two samples are
/// smoothed by the trapezoidal average rather than captured exactly. Returns `0.0` when fewer
/// than two samples are available, since there is no timeline to integrate over.
fn integrate_cpu_core_seconds(samples: &[ResourceSample], logical_cores: f64) -> f64 {
    if samples.len() < 2 {
        return 0.0;
    }

    samples
        .windows(2)
        .map(|pair| {
            let (prev, curr) = (pair[0], pair[1]);
            let delta_secs = curr.timestamp_ms.saturating_sub(prev.timestamp_ms) as f64 / 1000.0;
            let prev_raw_percent = prev.cpu_percent * logical_cores;
            let curr_raw_percent = curr.cpu_percent * logical_cores;
            let avg_raw_percent = (prev_raw_percent + curr_raw_percent) / 2.0;
            (avg_raw_percent / 100.0) * delta_secs
        })
        .sum()
}

/// Collect total CPU usage from a process and all its descendants
///
/// Recursively traverses the process tree, summing CPU usage from the parent
/// and all child processes. This mirrors `collect_process_tree_memory` to ensure
/// CPU measurement is consistent with memory measurement.
///
/// Without this, subprocess-based frameworks (tika, pandoc, etc.) show near-zero
/// CPU because only the idle parent/wrapper process is measured, while the actual
/// extraction work happens in child processes.
///
/// # Arguments
/// * `pid` - The root process ID to measure
/// * `system` - System instance with refreshed process information
///
/// # Returns
/// Total CPU usage percentage for the entire process tree (0.0 - 100.0 * num_cores)
fn collect_process_tree_cpu(pid: Pid, system: &System) -> f64 {
    let mut total = 0.0;

    if let Some(proc) = system.process(pid) {
        total += proc.cpu_usage() as f64;

        for child_pid in get_child_processes(pid, system) {
            total += collect_process_tree_cpu(child_pid, system);
        }
    }

    total
}

/// Resource monitor that samples CPU and memory usage periodically
///
/// Tracks both low-level CPU/memory metrics and optional heap allocation data.
/// Use the "memory-profiling" feature for enhanced allocation analysis.
struct PreparedSampler {
    system: System,
    refresh_kind: ProcessRefreshKind,
}

#[derive(Default)]
struct SamplerState {
    prepared: Option<PreparedSampler>,
    task: Option<JoinHandle<()>>,
    stop_sender: Option<Sender<()>>,
}

pub struct ResourceMonitor {
    samples: Arc<Mutex<Vec<ResourceSample>>>,
    snapshots: Arc<Mutex<Vec<MemorySnapshot>>>,
    running: Arc<AtomicBool>,
    pid: Pid,
    /// Baseline RSS captured at start(), used to compute delta-based memory metrics.
    /// This removes the effect of pre-loaded models/runtimes from per-extraction measurements.
    baseline_memory_bytes: Arc<Mutex<u64>>,
    sampler: Mutex<SamplerState>,
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
            baseline_memory_bytes: Arc::new(Mutex::new(0)),
            sampler: Mutex::new(SamplerState::default()),
        }
    }

    /// Create a resource monitor targeting a specific process ID.
    ///
    /// Use this for persistent-mode subprocesses where the extraction server's PID
    /// is known. Monitoring a specific PID captures that process tree's actual memory
    /// rather than the harness process memory.
    pub fn new_for_pid(pid: u32) -> Self {
        Self {
            samples: Arc::new(Mutex::new(Vec::new())),
            snapshots: Arc::new(Mutex::new(Vec::new())),
            running: Arc::new(AtomicBool::new(false)),
            pid: Pid::from_u32(pid),
            baseline_memory_bytes: Arc::new(Mutex::new(0)),
            sampler: Mutex::new(SamplerState::default()),
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

    fn collect_sample(pid: Pid, system: &System, elapsed: Duration) -> Option<(ResourceSample, MemorySnapshot)> {
        system.process(pid)?;
        let tree_memory = collect_process_tree_memory(pid, system);
        let tree_vm = collect_process_tree_vm(pid, system);
        let tree_cpu = collect_process_tree_cpu(pid, system);
        let sample = ResourceSample {
            memory_bytes: tree_memory,
            vm_size_bytes: tree_vm,
            page_faults: 0,
            cpu_percent: tree_cpu / num_cpus::get() as f64,
            timestamp_ms: elapsed.as_millis() as u64,
        };

        #[cfg(feature = "memory-profiling")]
        let snapshot = MemorySnapshot::new(elapsed, tree_memory, tree_vm, 0, Self::capture_heap_stats());
        #[cfg(not(feature = "memory-profiling"))]
        let snapshot = MemorySnapshot::new(elapsed, tree_memory, tree_vm, 0);

        Some((sample, snapshot))
    }

    /// Prepare resource monitoring without recording a sample.
    ///
    /// Captures the target process tree's baseline RSS and retains the refreshed
    /// system state for [`Self::activate`]. This is useful when a subprocess is
    /// blocked behind a start barrier: the blocked shell remains baseline metadata
    /// and is never counted as target workload RSS.
    pub async fn prepare(&self) {
        let mut sampler = self.sampler.lock().await;
        if self.running.load(Ordering::SeqCst) || sampler.prepared.is_some() {
            return;
        }

        let pid = self.pid;
        let prepared = tokio::task::spawn_blocking(move || {
            let mut system = System::new().map_err(|error| format!("initialize process monitoring: {error}"))?;
            let refresh_kind = ProcessRefreshKind::nothing().with_memory().with_cpu();

            system.refresh_processes_specifics(ProcessesToUpdate::All, true, refresh_kind);
            let baseline_rss = collect_process_tree_memory(pid, &system);

            Ok::<_, String>((PreparedSampler { system, refresh_kind }, baseline_rss))
        })
        .await
        .map_err(|error| format!("prepare process monitoring: {error}"))
        .and_then(std::convert::identity);
        let (prepared, baseline_rss) = match prepared {
            Ok(prepared) => prepared,
            Err(error) => {
                tracing::warn!(%error, "Resource monitoring could not start");
                return;
            }
        };

        *self.baseline_memory_bytes.lock().await = baseline_rss;
        sampler.prepared = Some(prepared);
    }

    /// Activate periodic sampling after [`Self::prepare`].
    ///
    /// Resets the sampling clock and schedules the first target-window sample one
    /// interval after activation. Delaying that first sample prevents a released
    /// start-barrier shell from being mistaken for the target before it calls
    /// `exec`. If the target exits before the first refresh, no sample is recorded.
    pub async fn activate(&self, sample_interval: Duration) {
        self.activate_prepared(sample_interval, false).await;
    }

    async fn activate_prepared(&self, sample_interval: Duration, record_initial_sample: bool) {
        let mut sampler = self.sampler.lock().await;
        if self.running.load(Ordering::SeqCst) {
            return;
        }
        let Some(prepared) = sampler.prepared.take() else {
            return;
        };

        let PreparedSampler {
            mut system,
            refresh_kind,
        } = prepared;
        let pid = self.pid;
        let start = std::time::Instant::now();

        if record_initial_sample && let Some((sample, snapshot)) = Self::collect_sample(pid, &system, start.elapsed()) {
            self.samples.lock().await.push(sample);
            self.snapshots.lock().await.push(snapshot);
        }

        let samples = Arc::clone(&self.samples);
        let snapshots = Arc::clone(&self.snapshots);
        let running = Arc::clone(&self.running);
        let (stop_tx, stop_rx) = mpsc::channel();
        self.running.store(true, Ordering::SeqCst);
        let task = tokio::task::spawn_blocking(move || {
            let mut next_sample = std::time::Instant::now() + sample_interval;

            while running.load(Ordering::SeqCst) {
                let wait = next_sample.saturating_duration_since(std::time::Instant::now());
                match stop_rx.recv_timeout(wait) {
                    Ok(()) | Err(RecvTimeoutError::Disconnected) => break,
                    Err(RecvTimeoutError::Timeout) => {}
                }
                if !running.load(Ordering::SeqCst) {
                    break;
                }
                system.refresh_processes_specifics(ProcessesToUpdate::All, true, refresh_kind);
                if let Some((sample, snapshot)) = Self::collect_sample(pid, &system, start.elapsed()) {
                    samples.blocking_lock().push(sample);
                    snapshots.blocking_lock().push(snapshot);
                }
                next_sample += sample_interval;
            }
        });
        sampler.stop_sender = Some(stop_tx);
        sampler.task = Some(task);
    }

    /// Start monitoring resources in the background
    ///
    /// Spawns a background task that samples memory and CPU usage at the specified interval.
    /// When "memory-profiling" feature is enabled, also captures heap allocation data.
    ///
    /// # Arguments
    /// * `sample_interval` - How often to sample (e.g., Duration::from_millis(10))
    pub async fn start(&self, sample_interval: Duration) {
        self.prepare().await;
        self.activate_prepared(sample_interval, true).await;
    }

    /// Take a single synchronous memory and CPU measurement of the current process tree.
    ///
    /// Useful as a fallback when the background sampler collects zero samples
    /// (e.g., sub-millisecond extractions that complete before the first sample).
    /// Performs two refreshes with a 50ms gap to get a valid CPU delta.
    pub fn snapshot_current_memory(&self) -> crate::Result<ResourceSample> {
        let mut system = System::new()
            .map_err(|error| crate::Error::Benchmark(format!("initialize process memory snapshot: {error}")))?;
        let refresh_kind = ProcessRefreshKind::nothing().with_memory().with_cpu();

        system.refresh_processes_specifics(ProcessesToUpdate::All, false, refresh_kind);
        std::thread::sleep(std::time::Duration::from_millis(50));
        system.refresh_processes_specifics(ProcessesToUpdate::All, false, refresh_kind);

        let tree_memory = collect_process_tree_memory(self.pid, &system);
        let tree_vm = collect_process_tree_vm(self.pid, &system);
        let cpu_count = num_cpus::get() as f64;
        let tree_cpu = collect_process_tree_cpu(self.pid, &system);
        let normalized_cpu_percent = tree_cpu / cpu_count;

        Ok(ResourceSample {
            memory_bytes: tree_memory,
            vm_size_bytes: tree_vm,
            page_faults: 0,
            cpu_percent: normalized_cpu_percent,
            timestamp_ms: 0,
        })
    }

    /// Stop monitoring and return collected samples
    pub async fn stop(&self) -> Vec<ResourceSample> {
        let (stop_sender, task) = {
            let mut sampler = self.sampler.lock().await;
            self.running.store(false, Ordering::SeqCst);
            sampler.prepared = None;
            (sampler.stop_sender.take(), sampler.task.take())
        };
        if let Some(stop_sender) = stop_sender {
            let _ = stop_sender.send(());
        }

        if let Some(task) = task {
            let _ = task.await;
        }

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

    /// Get the baseline memory captured at start().
    pub async fn baseline_memory(&self) -> u64 {
        *self.baseline_memory_bytes.lock().await
    }

    /// Calculate resource statistics from samples and snapshots
    ///
    /// Absolute RSS is the primary peak metric. `baseline_bytes` and the
    /// corresponding peak delta are retained separately so callers can distinguish
    /// total process footprint from memory added during extraction.
    pub fn calculate_stats(
        samples: &[ResourceSample],
        snapshots: &[MemorySnapshot],
        baseline_bytes: u64,
    ) -> ResourceStats {
        if samples.is_empty() {
            return Self::stats_from_snapshots_only(snapshots, baseline_bytes);
        }

        let memory_values: Vec<u64> = samples.iter().map(|s| s.memory_bytes).collect();
        let memory_delta_values: Vec<u64> = memory_values
            .iter()
            .map(|memory| memory.saturating_sub(baseline_bytes))
            .collect();
        let cpu_values: Vec<f64> = samples.iter().map(|s| s.cpu_percent).collect();
        let vm_values: Vec<u64> = samples.iter().map(|s| s.vm_size_bytes).collect();

        let peak_memory = *memory_values.iter().max().unwrap_or(&0);
        let peak_vm = *vm_values.iter().max().unwrap_or(&0);
        let avg_cpu = cpu_values.iter().sum::<f64>() / cpu_values.len() as f64;
        let total_page_faults = samples.last().map(|s| s.page_faults).unwrap_or(0);

        ResourceStats {
            baseline_memory_bytes: baseline_bytes,
            peak_memory_bytes: peak_memory,
            peak_memory_delta_bytes: memory_delta_values.iter().copied().max().unwrap_or(0),
            peak_vm_bytes: peak_vm,
            total_page_faults,
            memory_growth_rate_mb_s: memory_growth_rate_mb_per_s(samples, &memory_values),
            avg_cpu_percent: avg_cpu,
            cpu_seconds: integrate_cpu_core_seconds(samples, num_cpus::get() as f64),
            p50_memory_bytes: Self::calculate_percentile(memory_values.clone(), 0.50),
            p95_memory_bytes: Self::calculate_percentile(memory_values.clone(), 0.95),
            p99_memory_bytes: Self::calculate_percentile(memory_values, 0.99),
            sample_count: samples.len(),
            snapshots: snapshots.to_vec(),
            leak_detected: memory_leak_detected(snapshots),
        }
    }

    /// Stats when no resource samples were collected but memory snapshots exist (or not),
    /// e.g. an extraction that finished faster than the sampling interval.
    fn stats_from_snapshots_only(snapshots: &[MemorySnapshot], baseline_bytes: u64) -> ResourceStats {
        if snapshots.is_empty() {
            return ResourceStats {
                baseline_memory_bytes: baseline_bytes,
                ..Default::default()
            };
        }
        let peak_rss = snapshots.iter().map(|s| s.rss_bytes).max().unwrap_or(0);
        let peak_vm = snapshots.iter().map(|s| s.vm_bytes).max().unwrap_or(0);
        ResourceStats {
            baseline_memory_bytes: baseline_bytes,
            peak_memory_bytes: peak_rss,
            peak_memory_delta_bytes: peak_rss.saturating_sub(baseline_bytes),
            peak_vm_bytes: peak_vm,
            p50_memory_bytes: peak_rss,
            p95_memory_bytes: peak_rss,
            p99_memory_bytes: peak_rss,
            sample_count: snapshots.len(),
            snapshots: snapshots.to_vec(),
            ..Default::default()
        }
    }
}

/// Memory growth rate in MB/s between the first and last sample, or `0.0` with fewer than
/// two samples. Never negative — a shrinking trace reports no growth rather than a negative
/// rate.
fn memory_growth_rate_mb_per_s(samples: &[ResourceSample], memory_values: &[u64]) -> f64 {
    if samples.len() < 2 {
        return 0.0;
    }
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
}

/// Heuristic leak detection: memory grew more than 5% from start to end of the run AND more
/// than 20% of the peak is still retained at the end. Requires at least two snapshots.
fn memory_leak_detected(snapshots: &[MemorySnapshot]) -> bool {
    if snapshots.len() < 2 {
        return false;
    }
    let start_rss = snapshots[0].rss_bytes as f64;
    let end_rss = snapshots[snapshots.len() - 1].rss_bytes as f64;
    let peak_rss = snapshots.iter().map(|s| s.rss_bytes as f64).fold(0.0, f64::max);

    if peak_rss <= 0.0 {
        return false;
    }
    let growth_percent = ((end_rss - start_rss) / start_rss) * 100.0;
    let retained_percent = (end_rss / peak_rss) * 100.0;
    growth_percent > 5.0 && retained_percent > 20.0
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
    /// RSS captured immediately after monitoring attached to the target.
    pub baseline_memory_bytes: u64,
    /// Absolute peak RSS in bytes.
    pub peak_memory_bytes: u64,
    /// Peak RSS above the captured baseline in bytes.
    pub peak_memory_delta_bytes: u64,
    /// Peak virtual memory size in bytes
    pub peak_vm_bytes: u64,
    /// Total major page faults
    pub total_page_faults: u64,
    /// Memory growth rate in MB/s
    pub memory_growth_rate_mb_s: f64,
    /// Average CPU usage percentage
    pub avg_cpu_percent: f64,
    /// Total process-tree CPU time consumed, in core-seconds (trapezoidal integration of the
    /// sampled timeline; see [`integrate_cpu_core_seconds`]).
    pub cpu_seconds: f64,
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
    /// Whether memory leak was detected (RSA growing without release)
    pub leak_detected: bool,
}

#[cfg(test)]
mod tests;
