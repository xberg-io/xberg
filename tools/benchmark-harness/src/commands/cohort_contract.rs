//! `cohort-contract`: prints one cohort's pinned release contract as a single JSON line.
//!
//! The release workflow consumes this so cohort metadata and workflow coverage have a single
//! source of truth in `bench_matrix`.

use benchmark_harness::Result;
use benchmark_harness::bench_matrix::{Cohort, MatrixProfile};

pub(crate) fn cohort_contract_summary(cohort: Cohort, profile: MatrixProfile) -> serde_json::Value {
    let contract = cohort.contract_for(profile);
    let required = contract.matrix.iter().filter(|entry| !entry.optional).count();
    let optional = contract.matrix.iter().filter(|entry| entry.optional).count();
    let matrix = contract
        .matrix
        .iter()
        .map(|entry| {
            serde_json::json!({
                "artifact": entry.artifact,
                "framework": entry.framework,
                "output_format": entry.output_format.to_string(),
                "mode": entry.mode.artifact_slug(),
                "optional": entry.optional,
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "cohort": cohort.as_str(),
        "manifest_name": contract.manifest_name,
        "manifest_blake3": contract.manifest_blake3,
        "batch_size": contract.batch_size,
        "expects_ocr": cohort.expects_ocr(),
        "expected_matrix_keys": required,
        "optional_matrix_keys": optional,
        "matrix": matrix,
    })
}

pub(crate) fn execute(cohort: Cohort, profile: MatrixProfile) -> Result<()> {
    let summary = cohort_contract_summary(cohort, profile);
    let json = serde_json::to_string(&summary)
        .map_err(|error| benchmark_harness::Error::Benchmark(format!("serialize cohort contract: {error}")))?;
    println!("{json}");
    Ok(())
}
