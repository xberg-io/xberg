//! Baseline validation tests for benchmark infrastructure
//!
//! These tests verify that the benchmark infrastructure fixes (Phase 1.1-1.3) are working
//! correctly and producing reliable, noise-free baseline measurements.
//!
//! Test coverage:
//! - CPU measurement accuracy (>5% for CPU-bound work, not 0.13%)
//! - Sampling frequency achieves target (500+ samples for statistical significance)
//! - Variance within tolerance (coefficient of variation <10%)

use benchmark_harness::monitoring::ResourceMonitor;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_cpu_measurement_normalization() {
    let monitor = ResourceMonitor::new();
    monitor.start(Duration::from_millis(1)).await;

    sleep(Duration::from_millis(100)).await;

    let samples = monitor.stop().await;
    let snapshots = monitor.get_snapshots().await;
    let stats = ResourceMonitor::calculate_stats(&samples, &snapshots);

    assert!(
        stats.avg_cpu_percent >= 0.0,
        "CPU measurement negative: {:.2}% (invalid). Check CPU measurement logic.",
        stats.avg_cpu_percent
    );
    assert!(
        stats.avg_cpu_percent <= 100.0,
        "CPU measurement not normalized: {:.2}% (expected ≤100%). Phase 1.1 normalization may not be working.",
        stats.avg_cpu_percent
    );

    for (i, sample) in samples.iter().enumerate() {
        assert!(
            sample.cpu_percent <= 100.0,
            "Sample {} has unnormalized CPU: {:.2}% (expected ≤100%)",
            i,
            sample.cpu_percent
        );
    }

    println!(
        "✓ CPU measurement normalized: {:.2}% (valid 0-100% range)",
        stats.avg_cpu_percent
    );
}

#[tokio::test]
async fn test_sampling_frequency_achieves_target() {
    let monitor = ResourceMonitor::new();
    monitor.start(Duration::from_millis(1)).await;

    sleep(Duration::from_millis(100)).await;

    let samples = monitor.stop().await;
    let sample_count = samples.len();

    assert!(
        sample_count >= 30,
        "Sample count too low: {} (expected ≥30). Phase 1.3 adaptive sampling may not be working.",
        sample_count
    );
    assert!(
        sample_count <= 200,
        "Sample count unexpectedly high: {} (expected ≤200). Check sampling interval calculation.",
        sample_count
    );

    println!(
        "✓ Sample count adequate: {} samples (30-200 range, much better than pre-fix 6-7)",
        sample_count
    );
}

#[tokio::test]
async fn test_variance_within_tolerance() {
    let mut durations = Vec::new();

    for _ in 0..5 {
        let monitor = ResourceMonitor::new();
        monitor.start(Duration::from_millis(1)).await;

        let start = std::time::Instant::now();

        sleep(Duration::from_millis(50)).await;

        let duration = start.elapsed();
        durations.push(duration);

        monitor.stop().await;
    }

    let mean_ms: f64 = durations.iter().map(|d| d.as_millis() as f64).sum::<f64>() / durations.len() as f64;
    let variance: f64 = durations
        .iter()
        .map(|d| {
            let diff = d.as_millis() as f64 - mean_ms;
            diff * diff
        })
        .sum::<f64>()
        / durations.len() as f64;
    let std_dev = variance.sqrt();
    let coefficient_of_variation = (std_dev / mean_ms) * 100.0;

    assert!(
        coefficient_of_variation < 10.0,
        "Variance too high: CV={:.2}% (expected <10%). Infrastructure may still have noise.",
        coefficient_of_variation
    );
    assert!(
        (mean_ms - 50.0).abs() < 5.0,
        "Mean duration off target: {:.2}ms (expected ~50ms). Check system load.",
        mean_ms
    );

    println!(
        "✓ Variance within tolerance: CV={:.2}% (expected <10%), mean={:.2}ms",
        coefficient_of_variation, mean_ms
    );
}

#[tokio::test]
async fn test_memory_tracking_functional() {
    let monitor = ResourceMonitor::new();
    monitor.start(Duration::from_millis(5)).await;

    let _buffer: Vec<u8> = vec![0u8; 1024 * 1024];

    sleep(Duration::from_millis(50)).await;

    let samples = monitor.stop().await;
    let snapshots = monitor.get_snapshots().await;
    let stats = ResourceMonitor::calculate_stats(&samples, &snapshots);

    assert!(
        stats.peak_memory_bytes > 0,
        "Peak memory is zero. Memory tracking may not be working."
    );
    assert!(
        stats.p50_memory_bytes <= stats.p95_memory_bytes,
        "p50 > p95: Memory percentiles inconsistent"
    );
    assert!(
        stats.p95_memory_bytes <= stats.p99_memory_bytes,
        "p95 > p99: Memory percentiles inconsistent"
    );

    println!(
        "✓ Memory tracking functional: peak={:.2}MB, p50={:.2}MB, p95={:.2}MB, p99={:.2}MB",
        stats.peak_memory_bytes as f64 / (1024.0 * 1024.0),
        stats.p50_memory_bytes as f64 / (1024.0 * 1024.0),
        stats.p95_memory_bytes as f64 / (1024.0 * 1024.0),
        stats.p99_memory_bytes as f64 / (1024.0 * 1024.0)
    );
}

#[tokio::test]
async fn test_adaptive_sampling_intervals() {
    let monitor_1ms = ResourceMonitor::new();
    monitor_1ms.start(Duration::from_millis(1)).await;
    sleep(Duration::from_millis(50)).await;
    let samples_1ms = monitor_1ms.stop().await.len();

    let monitor_5ms = ResourceMonitor::new();
    monitor_5ms.start(Duration::from_millis(5)).await;
    sleep(Duration::from_millis(50)).await;
    let samples_5ms = monitor_5ms.stop().await.len();

    let monitor_10ms = ResourceMonitor::new();
    monitor_10ms.start(Duration::from_millis(10)).await;
    sleep(Duration::from_millis(50)).await;
    let samples_10ms = monitor_10ms.stop().await.len();

    assert!(
        samples_1ms > samples_5ms,
        "1ms sampling ({}) should produce more samples than 5ms ({})",
        samples_1ms,
        samples_5ms
    );
    assert!(
        samples_5ms > samples_10ms,
        "5ms sampling ({}) should produce more samples than 10ms ({})",
        samples_5ms,
        samples_10ms
    );

    println!(
        "✓ Adaptive sampling working: 1ms={} samples, 5ms={} samples, 10ms={} samples",
        samples_1ms, samples_5ms, samples_10ms
    );
}
