//! Shared test helpers used across the subprocess adapter test suite.

use crate::types::{BatchCapability, BatchEntryPoint};

pub(super) fn test_batch_capability(per_item_timing: bool) -> BatchCapability {
    BatchCapability {
        entry_point: BatchEntryPoint::DoclingJobkit,
        timing_scope: crate::types::BatchTimingScope::ColdEndToEndSubprocess,
        per_item_timing,
    }
}

pub(super) fn test_xberg_batch_capability(per_item_timing: bool) -> BatchCapability {
    BatchCapability {
        entry_point: BatchEntryPoint::XbergCliExtractBatch,
        timing_scope: crate::types::BatchTimingScope::ColdEndToEndSubprocess,
        per_item_timing,
    }
}

/// Identity of a stand-in executable, for building the real xberg adapters without a binary.
pub(super) fn test_build_identity() -> crate::adapter::ExecutableBuildIdentity {
    crate::adapter::ExecutableBuildIdentity {
        build_id: "test-build".to_string(),
        path: std::path::PathBuf::from("xberg"),
    }
}
