//! Failure counting and the cohort-wide failure roll-up.

use super::types::{FailureCounts, FailureSummary};
use crate::types::{BenchmarkResult, ErrorKind};

impl FailureCounts {
    /// Fold one result's error into the counts (a successful result contributes nothing) and keep
    /// the framework-fault / infra totals in sync.
    pub(super) fn record(&mut self, result: &BenchmarkResult) {
        match result.error_kind {
            ErrorKind::FrameworkError => self.framework_errors += 1,
            ErrorKind::EmptyContent => self.empty_content += 1,
            ErrorKind::ZeroOverlap => self.zero_overlap += 1,
            ErrorKind::LayoutFailed => self.layout_failures += 1,
            ErrorKind::Timeout => self.timeouts += 1,
            ErrorKind::HarnessError => self.harness_errors += 1,
            ErrorKind::ConfigSetupError => self.config_setup_errors += 1,
            ErrorKind::None => {}
        }
        self.framework_fault_total =
            self.framework_errors + self.empty_content + self.zero_overlap + self.layout_failures + self.timeouts;
        self.infra_total = self.harness_errors + self.config_setup_errors;
    }
}

/// Roll every result's error up to the cohort total plus per-framework-mode and per-file-type
/// breakdowns, mirroring the keys used by [`super::aggregate_new_format`].
pub(super) fn build_failure_summary(results: &[BenchmarkResult]) -> FailureSummary {
    let mut summary = FailureSummary::default();
    for result in results {
        summary.total.record(result);
        let (framework, mode) = super::extract_framework_and_mode(&result.framework);
        let key = super::make_aggregate_key(framework, result.output_format, mode);
        summary.by_framework_mode.entry(key).or_default().record(result);
        summary
            .by_file_type
            .entry(result.file_extension.clone())
            .or_default()
            .record(result);
    }
    summary
}

/// A failed result the framework itself is accountable for: it was handed a document in a format
/// it declares support for and still failed to extract it (hard error, empty output, or timeout).
/// These are scored as quality 0 so partial failures penalize the aggregate. Harness/config-setup
/// failures are our own infrastructure's fault and are deliberately excluded — they neither
/// penalize quality nor count against the success rate.
pub(super) fn is_framework_fault_failure(result: &BenchmarkResult) -> bool {
    !result.success
        && matches!(
            result.error_kind,
            ErrorKind::FrameworkError
                | ErrorKind::EmptyContent
                | ErrorKind::ZeroOverlap
                | ErrorKind::LayoutFailed
                | ErrorKind::Timeout
        )
}
