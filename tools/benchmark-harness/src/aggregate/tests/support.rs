//! Shared test fixture builders used across the split `aggregate` test modules.

use super::*;
use crate::types::{ErrorKind, FrameworkCapabilities, OcrStatus, PdfMetadata, PerformanceMetrics};
use std::path::PathBuf;
use std::time::Duration;

pub(super) fn create_test_result(
    framework: &str,
    file_ext: &str,
    ocr_status: OcrStatus,
    duration_ms: u64,
    throughput_bps: f64,
    memory_bytes: u64,
) -> BenchmarkResult {
    BenchmarkResult {
        framework: framework.to_string(),
        file_path: PathBuf::from(format!("test.{}", file_ext)),
        file_size: 1024,
        success: true,
        error_message: None,
        error_kind: ErrorKind::None,
        duration: Duration::from_millis(duration_ms),
        extraction_duration: None,
        subprocess_overhead: None,
        metrics: PerformanceMetrics {
            baseline_memory_bytes: 0,
            peak_memory_bytes: memory_bytes,
            peak_memory_delta_bytes: memory_bytes,
            avg_cpu_percent: 50.0,
            cpu_seconds: 50.0,
            throughput_bytes_per_sec: throughput_bps,
            p50_memory_bytes: memory_bytes,
            p95_memory_bytes: memory_bytes,
            p99_memory_bytes: memory_bytes,
            ..Default::default()
        },
        quality: None,
        iterations: vec![],
        statistics: None,
        cold_start_duration: Some(Duration::from_millis(500)),
        file_extension: file_ext.to_string(),
        framework_capabilities: FrameworkCapabilities {
            timing_regime: crate::types::TimingRegime::ColdProcess,
            resource_measurement_scope: crate::types::ResourceMeasurementScope::IsolatedProcess,
            ..Default::default()
        },
        pdf_metadata: None,
        ocr_status,
        output_format: OutputFormat::Markdown,
        extracted_text: None,
        system_load: None,
        fixture_family: None,
    }
}

pub(super) fn pdf_metadata_with_page_count(page_count: u32) -> PdfMetadata {
    PdfMetadata {
        has_text_layer: true,
        detection_method: "pdftotext".to_string(),
        page_count: Some(page_count),
        ocr_enabled: false,
        text_quality_score: None,
    }
}

pub(super) fn ranking_value(ranking: &[RankedFramework], framework: &str) -> f64 {
    ranking
        .iter()
        .find(|entry| entry.framework_mode.contains(framework))
        .unwrap_or_else(|| panic!("missing ranking entry for {framework}"))
        .value
}
