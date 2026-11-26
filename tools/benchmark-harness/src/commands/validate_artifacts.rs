//! `validate-artifacts`: validates the benchmark artifact/aggregate contract for one cohort.

use crate::cli::{CliCohort, CliMatrixProfile};
use benchmark_harness::Result;
use benchmark_harness::validate_artifacts::{ValidateArtifactsArgs, validate};
use clap::Args;
use std::path::PathBuf;

/// `validate-artifacts` subcommand arguments -- pulled out of the `Commands` enum (via the
/// tuple-variant `ValidateArtifacts(ValidateArtifactsCliArgs)`) purely to keep `Commands` under
/// the file's type-length limit; the flag grammar itself is unchanged.
///
/// Two modes: with `--aggregated-file`, validates one consolidated `aggregated.json`; otherwise
/// validates a directory of raw per-framework `run/{provenance,results}.json` artifacts, which
/// requires `--artifacts-dir`, `--cohort-manifest`, `--fixtures-root`, `--source-sha`, and
/// `--run-id`.
#[derive(Args)]
pub(crate) struct ValidateArtifactsCliArgs {
    /// Which cohort's release contract to validate
    #[arg(long)]
    pub(crate) cohort: CliCohort,

    /// Path to a consolidated aggregated.json; when set, validates the aggregate instead
    /// of raw per-framework artifacts
    #[arg(long)]
    pub(crate) aggregated_file: Option<PathBuf>,

    /// Directory containing one subdirectory per expected artifact
    #[arg(long)]
    pub(crate) artifacts_dir: Option<PathBuf>,

    /// Path to the pinned cohort manifest JSON
    #[arg(long)]
    pub(crate) cohort_manifest: Option<PathBuf>,

    /// Root directory fixture paths in the manifest are resolved against
    #[arg(long)]
    pub(crate) fixtures_root: Option<PathBuf>,

    /// Benchmark source revision every provenance.json must record
    #[arg(long)]
    pub(crate) source_sha: Option<String>,

    /// Run identifier suffix shared by every expected artifact directory
    #[arg(long)]
    pub(crate) run_id: Option<String>,

    /// Benchmark iterations every provenance.json/results.json must record
    #[arg(long, default_value = "3")]
    pub(crate) iterations: usize,

    /// Which matrix profile the run produced; `smoke` expects exactly the smoke cells
    #[arg(long, value_enum, default_value = "full")]
    pub(crate) profile: CliMatrixProfile,
}

pub(crate) fn execute(cli_args: ValidateArtifactsCliArgs) -> Result<()> {
    let args = ValidateArtifactsArgs {
        cohort: cli_args.cohort.into(),
        aggregated_file: cli_args.aggregated_file,
        artifacts_dir: cli_args.artifacts_dir,
        cohort_manifest: cli_args.cohort_manifest,
        fixtures_root: cli_args.fixtures_root,
        source_sha: cli_args.source_sha,
        run_id: cli_args.run_id,
        iterations: cli_args.iterations,
        profile: cli_args.profile.into(),
    };

    let message = validate(&args)?;
    println!("{message}");
    Ok(())
}
