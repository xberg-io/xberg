//! Whole paired-process resource evidence, including both models, loading and rendering.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use sysinfo::System;

use crate::monitoring::{ResourceMonitor, ResourceSample};
use crate::system_load::SystemLoad;
use crate::{Error, Result};

pub(super) const RESOURCE_SCOPE: &str = "paired_process_tree_including_both_models_load_and_render";
pub(super) const SAMPLE_INTERVAL_MS: u64 = 10;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceHostIdentity {
    pub os_family: String,
    pub os_name: Option<String>,
    pub os_version: Option<String>,
    pub kernel_version: Option<String>,
    pub architecture: String,
    pub cpu_brand: Option<String>,
    pub logical_cores: usize,
    pub physical_cores: usize,
}

impl ResourceHostIdentity {
    fn capture(load: &SystemLoad) -> Result<Self> {
        let mut system =
            System::new().map_err(|error| Error::Benchmark(format!("initialize resource host identity: {error}")))?;
        system.refresh_cpu_all();
        let cpu_brand = system
            .cpus()
            .first()
            .map(|cpu| cpu.brand().trim().to_string())
            .filter(|brand| !brand.is_empty());
        Ok(Self {
            os_family: std::env::consts::OS.to_string(),
            os_name: System::name().ok(),
            os_version: System::os_version().ok(),
            kernel_version: System::kernel_version().ok(),
            architecture: System::cpu_arch(),
            cpu_brand,
            logical_cores: load.logical_cores,
            physical_cores: load.physical_cores,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PairedProcessResources {
    pub scope: String,
    pub measured_process_id: u32,
    pub host: ResourceHostIdentity,
    pub load_before: SystemLoad,
    pub load_after: SystemLoad,
    pub timing_contended: bool,
    pub sampling_interval_ms: u64,
    pub sample_count: usize,
    pub sample_timestamps_ms: Vec<u64>,
    pub baseline_rss_bytes: u64,
    pub peak_rss_bytes: u64,
    pub process_cpu_core_seconds: f64,
}

impl PairedProcessResources {
    pub(super) fn validate(&self) -> Result<()> {
        if self.scope != RESOURCE_SCOPE || self.measured_process_id == 0 {
            return Err(Error::Benchmark(
                "invalid paired resource scope or process identity".to_string(),
            ));
        }
        validate_sample_timestamps(self.sample_count, &self.sample_timestamps_ms)?;
        if self.sampling_interval_ms != SAMPLE_INTERVAL_MS
            || self.baseline_rss_bytes == 0
            || self.peak_rss_bytes == 0
            || !self.process_cpu_core_seconds.is_finite()
            || self.process_cpu_core_seconds < 0.0
        {
            return Err(Error::Benchmark(
                "invalid paired process resource measurements".to_string(),
            ));
        }
        self.validate_host_and_load()?;
        if self.timing_contended != (self.load_before.is_contended() || self.load_after.is_contended()) {
            return Err(Error::Benchmark("paired resource contention flag mismatch".to_string()));
        }
        Ok(())
    }

    fn validate_host_and_load(&self) -> Result<()> {
        if self.host.os_family.trim().is_empty()
            || self.host.architecture.trim().is_empty()
            || self.host.logical_cores == 0
            || self.host.physical_cores == 0
        {
            return Err(Error::Benchmark("missing paired resource host identity".to_string()));
        }
        for load in [&self.load_before, &self.load_after] {
            if load.logical_cores != self.host.logical_cores
                || load.physical_cores != self.host.physical_cores
                || [load.load_avg_1m, load.load_avg_5m, load.load_avg_15m]
                    .iter()
                    .any(|value| !value.is_finite() || *value < 0.0)
            {
                return Err(Error::Benchmark(
                    "paired resource host/load identity mismatch".to_string(),
                ));
            }
        }
        Ok(())
    }
}

/// Always join the sampler before propagating a worker error or panic.
pub(super) async fn measure_operation<T, F>(
    monitor: &ResourceMonitor,
    operation: F,
) -> Result<(T, PairedProcessResources)>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T> + Send + 'static,
{
    let (host, load_before) = tokio::task::spawn_blocking(|| {
        let load = SystemLoad::capture()?;
        Ok::<_, Error>((ResourceHostIdentity::capture(&load)?, load))
    })
    .await
    .map_err(super::benchmark_error)??;
    monitor.start(Duration::from_millis(SAMPLE_INTERVAL_MS)).await;
    if monitor.get_snapshots().await.is_empty() {
        monitor.stop().await;
        return Err(Error::Benchmark("resource monitor collected zero samples".to_string()));
    }
    let outcome = tokio::task::spawn_blocking(operation).await;
    let final_sample = wait_for_post_operation_sample(monitor).await;
    // ~keep Stop and join sampling before propagating either result; errors must not leave a sampler running.
    let samples = monitor.stop().await;
    let output = outcome.map_err(super::benchmark_error)??;
    final_sample?;
    let resources = collect_resources(monitor, &samples, host, load_before).await?;
    Ok((output, resources))
}

async fn collect_resources(
    monitor: &ResourceMonitor,
    samples: &[ResourceSample],
    host: ResourceHostIdentity,
    load_before: SystemLoad,
) -> Result<PairedProcessResources> {
    let sample_timestamps_ms: Vec<u64> = samples.iter().map(|sample| sample.timestamp_ms).collect();
    validate_sample_timestamps(samples.len(), &sample_timestamps_ms)?;
    let snapshots = monitor.get_snapshots().await;
    let baseline = monitor.baseline_memory().await;
    let stats = ResourceMonitor::calculate_stats(samples, &snapshots, baseline);
    let load_after = tokio::task::spawn_blocking(SystemLoad::capture)
        .await
        .map_err(super::benchmark_error)??;
    let resources = PairedProcessResources {
        scope: RESOURCE_SCOPE.to_string(),
        measured_process_id: std::process::id(),
        host,
        load_before,
        load_after,
        timing_contended: load_before.is_contended() || load_after.is_contended(),
        sampling_interval_ms: SAMPLE_INTERVAL_MS,
        sample_count: samples.len(),
        sample_timestamps_ms,
        baseline_rss_bytes: stats.baseline_memory_bytes,
        peak_rss_bytes: stats.peak_memory_bytes,
        process_cpu_core_seconds: stats.cpu_seconds,
    };
    resources.validate()?;
    Ok(resources)
}

fn validate_sample_timestamps(sample_count: usize, timestamps: &[u64]) -> Result<()> {
    if sample_count < 2 || timestamps.len() != sample_count {
        return Err(Error::Benchmark(
            "resource monitor requires at least two timestamped samples".to_string(),
        ));
    }
    if timestamps.windows(2).any(|pair| pair[1] <= pair[0]) {
        return Err(Error::Benchmark(
            "resource sample timestamps must strictly increase over a positive span".to_string(),
        ));
    }
    Ok(())
}

async fn wait_for_post_operation_sample(monitor: &ResourceMonitor) -> Result<()> {
    let last_timestamp = monitor.get_snapshots().await.last().map(|sample| sample.timestamp);
    let wait = async {
        loop {
            tokio::time::sleep(Duration::from_millis(SAMPLE_INTERVAL_MS)).await;
            if monitor.get_snapshots().await.last().map(|sample| sample.timestamp) > last_timestamp {
                return;
            }
        }
    };
    tokio::time::timeout(Duration::from_secs(1), wait).await.map_err(|_| {
        Error::Benchmark("resource monitor produced no observed post-operation sample within one second".to_string())
    })
}

#[cfg(test)]
pub(super) fn valid_test_resources() -> PairedProcessResources {
    tests::valid_resources()
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) fn valid_resources() -> PairedProcessResources {
        let load = SystemLoad {
            load_avg_1m: 1.0,
            load_avg_5m: 1.0,
            load_avg_15m: 1.0,
            logical_cores: 4,
            physical_cores: 4,
        };
        PairedProcessResources {
            scope: RESOURCE_SCOPE.to_string(),
            measured_process_id: 123,
            host: ResourceHostIdentity {
                os_family: "test".to_string(),
                os_name: None,
                os_version: None,
                kernel_version: None,
                architecture: "test-cpu".to_string(),
                cpu_brand: None,
                logical_cores: 4,
                physical_cores: 4,
            },
            load_before: load,
            load_after: load,
            timing_contended: false,
            sampling_interval_ms: SAMPLE_INTERVAL_MS,
            sample_count: 2,
            sample_timestamps_ms: vec![0, 10],
            baseline_rss_bytes: 100,
            peak_rss_bytes: 200,
            process_cpu_core_seconds: 0.25,
        }
    }

    #[test]
    fn should_reject_zero_resource_samples_instead_of_an_unmeasured_pass() {
        let mut resources = valid_resources();
        resources.validate().unwrap();
        resources.sample_count = 0;
        assert_eq!(
            resources.validate().unwrap_err().to_string(),
            "Benchmark error: resource monitor requires at least two timestamped samples"
        );
    }

    #[test]
    fn should_reject_a_single_pre_operation_sample() {
        let mut resources = valid_resources();
        resources.sample_count = 1;
        resources.sample_timestamps_ms = vec![0];
        assert_eq!(
            resources.validate().unwrap_err().to_string(),
            "Benchmark error: resource monitor requires at least two timestamped samples"
        );
        assert!(validate_sample_timestamps(1, &[0]).is_err());
    }

    #[test]
    fn should_reject_zero_span_reversed_and_mismatched_sample_timestamps() {
        for timestamps in [vec![0, 0], vec![10, 0], vec![0, 10, 10]] {
            let mut resources = valid_resources();
            resources.sample_count = timestamps.len();
            resources.sample_timestamps_ms = timestamps.clone();
            assert_eq!(
                resources.validate().unwrap_err().to_string(),
                "Benchmark error: resource sample timestamps must strictly increase over a positive span"
            );
            assert!(validate_sample_timestamps(timestamps.len(), &timestamps).is_err());
        }
        let mut resources = valid_resources();
        resources.sample_timestamps_ms.push(20);
        assert_eq!(
            resources.validate().unwrap_err().to_string(),
            "Benchmark error: resource monitor requires at least two timestamped samples"
        );
        assert!(validate_sample_timestamps(2, &[0, 10, 20]).is_err());
    }

    #[tokio::test]
    async fn should_observe_a_post_operation_sample_for_tiny_work() {
        let monitor = ResourceMonitor::new();
        let (output, resources) = measure_operation(&monitor, || Ok(42)).await.unwrap();
        assert_eq!(output, 42);
        assert!(resources.sample_count >= 2);
        assert!(resources.sample_timestamps_ms.last() > resources.sample_timestamps_ms.first());
        resources.validate().unwrap();
    }

    #[test]
    fn should_record_contention_using_the_existing_system_load_policy() {
        let mut resources = valid_resources();
        resources.load_before.load_avg_1m = 4.0;
        assert_eq!(
            resources.validate().unwrap_err().to_string(),
            "Benchmark error: paired resource contention flag mismatch"
        );
        resources.timing_contended = true;
        resources.validate().unwrap();
    }

    #[test]
    fn should_reject_unscoped_or_invalid_memory_and_host_evidence() {
        let mut resources = valid_resources();
        resources.scope = "per-model-memory".to_string();
        assert_eq!(
            resources.validate().unwrap_err().to_string(),
            "Benchmark error: invalid paired resource scope or process identity"
        );
        let mut resources = valid_resources();
        resources.peak_rss_bytes = 0;
        assert_eq!(
            resources.validate().unwrap_err().to_string(),
            "Benchmark error: invalid paired process resource measurements"
        );
        let mut resources = valid_resources();
        resources.host.logical_cores = 0;
        assert_eq!(
            resources.validate().unwrap_err().to_string(),
            "Benchmark error: missing paired resource host identity"
        );
    }

    #[tokio::test]
    async fn should_refuse_unmeasured_work_before_starting_the_detector_operation() {
        let monitor = ResourceMonitor::new_for_pid(u32::MAX);
        let called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let operation_called = std::sync::Arc::clone(&called);
        let result = measure_operation(&monitor, move || {
            operation_called.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        })
        .await;
        assert_eq!(
            result.unwrap_err().to_string(),
            "Benchmark error: resource monitor collected zero samples"
        );
        assert!(
            !called.load(std::sync::atomic::Ordering::SeqCst),
            "unmeasured operation must not start"
        );
    }

    #[tokio::test]
    async fn should_join_resource_sampling_before_returning_an_operation_failure() {
        let monitor = ResourceMonitor::new();
        let result = measure_operation(&monitor, || {
            Err::<(), _>(Error::Benchmark("bounded failure".to_string()))
        })
        .await;
        assert_eq!(result.unwrap_err().to_string(), "Benchmark error: bounded failure");
        let samples_before = monitor.get_snapshots().await.len();
        assert!(samples_before > 0, "the monitor must actually sample this process");
        tokio::time::sleep(Duration::from_millis(SAMPLE_INTERVAL_MS * 3)).await;
        assert_eq!(
            monitor.get_snapshots().await.len(),
            samples_before,
            "sampling must stay stopped"
        );
    }
}
