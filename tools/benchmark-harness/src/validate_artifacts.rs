//! Validate the exact benchmark artifact contract for one fixed cohort.
//!
//! Ported from `scripts/ci/benchmarks/validate-benchmark-artifacts.py`. Two modes are
//! supported, matching the Python CLI:
//!
//! - **artifact mode** (`aggregated_file` is `None`): validates one directory of raw
//!   per-framework `run/{provenance,results}.json` artifacts against [`crate::bench_matrix`]'s
//!   pinned cohort contract.
//! - **aggregate mode** (`aggregated_file` is `Some`): validates one consolidated
//!   `aggregated.json` (as produced by the `consolidate` subcommand) against the same contract.

use std::collections::{HashMap, HashSet};
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::adapter::declared_ocr_language_policy;
use crate::aggregate::{
    NewConsolidatedResults, PerFixtureRow, PerformancePercentiles, SCHEMA_VERSION, comparison_for_cohort,
    extract_framework_and_mode,
};
use crate::bench_matrix::{Cohort, CohortContract, ExecutionMode, MatrixEntry, MatrixProfile};
use crate::fixture::Fixture;
use crate::provenance::RunProvenance;
use crate::types::{BenchmarkResult, ErrorKind, OcrStatus, OutputFormat, PerformanceMetrics, QualityMetrics};
use crate::{Error, Result};

mod aggregate_contract;
mod raw_artifacts;

/// Provenance schema version every `provenance.json` must record.
///
/// Mirrors the private `PROVENANCE_SCHEMA_VERSION` constant in [`crate::provenance`]; kept as a
/// named constant here rather than repeating an unexplained version number because that constant is not exported. ~keep
const EXPECTED_PROVENANCE_SCHEMA_VERSION: u32 = 3;

/// Inputs for [`validate`], mirroring the Python script's `argparse` surface.
#[derive(Debug, Clone)]
pub struct ValidateArtifactsArgs {
    /// Which cohort's release contract to validate against.
    pub cohort: Cohort,
    /// Path to a consolidated `aggregated.json`. When set, aggregate mode runs. `fixtures_root`
    /// may override the checked-in fixture directory used for quality-contract metadata.
    pub aggregated_file: Option<PathBuf>,
    /// Directory containing one subdirectory per expected artifact (artifact mode only).
    pub artifacts_dir: Option<PathBuf>,
    /// Path to the pinned cohort manifest JSON (artifact mode only).
    pub cohort_manifest: Option<PathBuf>,
    /// Root directory fixture paths in the manifest are resolved against (artifact mode only).
    pub fixtures_root: Option<PathBuf>,
    /// Benchmark source revision every `provenance.json` must record (artifact mode only).
    pub source_sha: Option<String>,
    /// Run identifier suffix shared by every expected artifact directory (artifact mode only).
    pub run_id: Option<String>,
    /// Benchmark iterations every `provenance.json`/`results.json` must record.
    pub iterations: usize,
    /// Which cell set the run produced; see [`Cohort::contract_for`].
    pub profile: MatrixProfile,
}

/// Validate one cohort's benchmark artifact contract, dispatching to artifact or aggregate mode.
///
/// Returns a human-readable summary line on success, matching the Python script's stdout.
pub fn validate(args: &ValidateArtifactsArgs) -> Result<String> {
    let contract = args.cohort.contract_for(args.profile);
    // An empty contract would accept an empty directory, so a cohort the profile does not run
    // is a caller error, never a pass.
    require(
        !contract.matrix.is_empty(),
        format!(
            "cohort {} has no cells in the {:?} profile",
            args.cohort.as_str(),
            args.profile
        ),
    )?;
    match &args.aggregated_file {
        Some(aggregated_file) => {
            let default_fixtures_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures");
            let fixtures_root = args.fixtures_root.as_deref().unwrap_or(&default_fixtures_root);
            let quality_expectations = fixture_quality_expectations(fixtures_root, &contract)?;
            aggregate_contract::validate_aggregate(
                aggregated_file,
                args.cohort,
                &contract,
                args.iterations,
                &quality_expectations,
            )
        }
        None => {
            let (artifacts_dir, cohort_manifest, fixtures_root, source_sha, run_id) = require_artifact_args(args)?;
            raw_artifacts::validate_raw_artifacts(&raw_artifacts::RawArtifactsArgs {
                artifacts_dir,
                cohort_manifest,
                fixtures_root,
                source_sha,
                run_id,
                iterations: args.iterations,
                cohort: args.cohort,
                contract: &contract,
            })
        }
    }
}

fn require_artifact_args(args: &ValidateArtifactsArgs) -> Result<(&Path, &Path, &Path, &str, &str)> {
    let mut missing = Vec::new();
    if args.artifacts_dir.is_none() {
        missing.push("artifacts-dir");
    }
    if args.cohort_manifest.is_none() {
        missing.push("cohort-manifest");
    }
    if args.fixtures_root.is_none() {
        missing.push("fixtures-root");
    }
    if args.source_sha.as_deref().unwrap_or_default().is_empty() {
        missing.push("source-sha");
    }
    if args.run_id.as_deref().unwrap_or_default().is_empty() {
        missing.push("run-id");
    }
    if !missing.is_empty() {
        return Err(Error::Config(format!(
            "artifact validation requires: {}",
            missing.join(", ")
        )));
    }
    Ok((
        args.artifacts_dir.as_deref().expect("checked above"),
        args.cohort_manifest.as_deref().expect("checked above"),
        args.fixtures_root.as_deref().expect("checked above"),
        args.source_sha.as_deref().expect("checked above"),
        args.run_id.as_deref().expect("checked above"),
    ))
}

fn require(condition: bool, message: impl Into<String>) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(Error::Benchmark(message.into()))
    }
}

fn contract_error(message: impl Into<String>) -> Error {
    Error::Benchmark(message.into())
}

/// One fixture's expected identity, derived from the real fixture descriptor and document bytes
/// referenced by the pinned cohort contract.
#[derive(Debug)]
struct ExpectedFixture {
    fixture: String,
    fixture_blake3: String,
    document_blake3: String,
    document_bytes: u64,
    document_name: String,
    has_quality_ground_truth: bool,
    has_structural_ground_truth: bool,
    ocr_language: Option<String>,
}

#[derive(Debug, Clone)]
struct FixtureQualityExpectation {
    has_quality_ground_truth: bool,
    has_structural_ground_truth: bool,
    ocr_language: Option<String>,
}

fn fixture_quality_expectations(
    fixtures_root: &Path,
    contract: &CohortContract,
) -> Result<Vec<FixtureQualityExpectation>> {
    contract
        .fixtures
        .iter()
        .map(|fixture| {
            let descriptor: Fixture = load_typed_json(&fixtures_root.join(fixture))?;
            Ok(FixtureQualityExpectation {
                has_quality_ground_truth: descriptor.ground_truth.as_ref().is_some_and(|ground_truth| {
                    ground_truth.text_file.is_some() || ground_truth.markdown_file.is_some()
                }),
                has_structural_ground_truth: descriptor
                    .ground_truth
                    .as_ref()
                    .is_some_and(|ground_truth| ground_truth.markdown_file.is_some()),
                ocr_language: descriptor.ocr_language().map(str::to_string),
            })
        })
        .collect()
}

/// Compute a BLAKE3 digest identical to the `b3sum` CLI output the Python script shelled out to.
fn blake3_file(path: &Path) -> Result<String> {
    if !path.is_file() {
        return Err(contract_error(format!("{}: expected a regular file", path.display())));
    }
    let mut file = std::fs::File::open(path).map_err(Error::Io)?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer).map_err(Error::Io)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

fn load_json_text(path: &Path) -> Result<String> {
    std::fs::read_to_string(path)
        .map_err(|error| contract_error(format!("{}: malformed or unreadable JSON: {error}", path.display())))
}

fn load_json_value(path: &Path) -> Result<serde_json::Value> {
    let text = load_json_text(path)?;
    serde_json::from_str(&text)
        .map_err(|error| contract_error(format!("{}: malformed or unreadable JSON: {error}", path.display())))
}

fn load_typed_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let text = load_json_text(path)?;
    serde_json::from_str(&text)
        .map_err(|error| contract_error(format!("{}: malformed or unreadable JSON: {error}", path.display())))
}

/// Basename of a `/`- or `\`-separated path string, matching `PurePosixPath(...).name`.
fn posix_basename(raw: &str) -> String {
    raw.replace('\\', "/").rsplit('/').next().unwrap_or(raw).to_string()
}

fn describe_set_mismatch<'a>(
    label: &str,
    expected: impl Iterator<Item = &'a str>,
    actual: impl Iterator<Item = &'a str>,
) -> String {
    let expected: HashSet<&str> = expected.collect();
    let actual: HashSet<&str> = actual.collect();
    let mut missing: Vec<&str> = expected.difference(&actual).copied().collect();
    missing.sort_unstable();
    let mut unexpected: Vec<&str> = actual.difference(&expected).copied().collect();
    unexpected.sort_unstable();
    format!("{label} mismatch; missing={missing:?}, unexpected={unexpected:?}")
}

fn validate_manifest(path: &Path, contract: &CohortContract) -> Result<String> {
    let manifest = load_json_value(path)?;
    let object = manifest
        .as_object()
        .ok_or_else(|| contract_error(format!("{}: manifest must be an object", path.display())))?;
    require(
        object.get("schema_version").and_then(serde_json::Value::as_u64) == Some(1),
        format!("{}: unexpected schema_version", path.display()),
    )?;
    require(
        object.get("name").and_then(serde_json::Value::as_str) == Some(contract.manifest_name),
        format!("{}: unexpected cohort name", path.display()),
    )?;
    require(
        object.get("batch_size").and_then(serde_json::Value::as_u64) == Some(contract.batch_size as u64),
        format!("{}: unexpected batch_size", path.display()),
    )?;
    let fixtures = object
        .get("fixtures")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| contract_error(format!("{}: fixtures must be an array", path.display())))?;
    let fixtures_match = fixtures
        .iter()
        .map(serde_json::Value::as_str)
        .collect::<Option<Vec<&str>>>()
        .is_some_and(|values| values == contract.fixtures);
    require(
        fixtures_match,
        format!("{}: fixture order/content mismatch", path.display()),
    )?;
    let digest = blake3_file(path)?;
    require(
        digest == contract.manifest_blake3,
        format!("{}: cohort manifest BLAKE3 mismatch", path.display()),
    )?;
    Ok(digest)
}

fn expected_fixtures(fixtures_root: &Path, contract: &CohortContract) -> Result<Vec<ExpectedFixture>> {
    let mut expected = Vec::with_capacity(contract.fixtures.len());
    for fixture in contract.fixtures {
        let descriptor_path = fixtures_root.join(fixture);
        let descriptor = Fixture::from_file(&descriptor_path)?;
        require(
            !descriptor.document.as_os_str().is_empty(),
            format!("{}: document must be a non-empty string", descriptor_path.display()),
        )?;
        let document_path = descriptor.validated_document_path(&descriptor_path)?;
        let document_name = descriptor
            .document
            .to_str()
            .map(posix_basename)
            .ok_or_else(|| contract_error(format!("{}: document path is not UTF-8", descriptor_path.display())))?;
        let has_quality_ground_truth = descriptor
            .ground_truth
            .as_ref()
            .is_some_and(|ground_truth| ground_truth.text_file.is_some() || ground_truth.markdown_file.is_some());
        let has_structural_ground_truth = descriptor
            .ground_truth
            .as_ref()
            .is_some_and(|ground_truth| ground_truth.markdown_file.is_some());
        expected.push(ExpectedFixture {
            fixture: (*fixture).to_string(),
            fixture_blake3: blake3_file(&descriptor_path)?,
            document_blake3: blake3_file(&document_path)?,
            document_bytes: std::fs::metadata(&document_path).map_err(Error::Io)?.len(),
            document_name,
            has_quality_ground_truth,
            has_structural_ground_truth,
            ocr_language: descriptor.ocr_language().map(str::to_string),
        });
    }

    let names: Vec<&str> = expected.iter().map(|item| item.document_name.as_str()).collect();
    let unique_names: HashSet<&str> = names.iter().copied().collect();
    require(
        unique_names.len() == names.len(),
        "cohort document basenames must be unique",
    )?;

    let stems: Vec<&str> = names
        .iter()
        .map(|name| {
            Path::new(name)
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or(name)
        })
        .collect();
    require(
        stems == contract.document_stems,
        "cohort document identities do not match the release contract",
    )?;

    let extensions: Vec<String> = names
        .iter()
        .map(|name| {
            Path::new(name)
                .extension()
                .and_then(|extension| extension.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase()
        })
        .collect();
    require(
        extensions
            .iter()
            .map(String::as_str)
            .eq(contract.document_extensions.iter().copied()),
        "cohort document extensions do not match the release contract",
    )?;

    Ok(expected)
}

fn read_artifact_dirs(root: &Path) -> Result<std::collections::HashMap<String, PathBuf>> {
    let mut dirs = std::collections::HashMap::new();
    for entry in std::fs::read_dir(root).map_err(Error::Io)? {
        let entry = entry.map_err(Error::Io)?;
        let path = entry.path();
        if path.is_dir()
            && let Some(name) = path.file_name().and_then(|name| name.to_str())
        {
            dirs.insert(name.to_string(), path);
        }
    }
    Ok(dirs)
}

fn only_file(root: &Path, filename: &str) -> Result<PathBuf> {
    let mut matches = Vec::new();
    collect_matching_files(root, filename, &mut matches)?;
    matches.sort();
    if matches.len() != 1 {
        return Err(contract_error(format!(
            "{}: expected exactly one {filename}, found {}",
            root.display(),
            matches.len()
        )));
    }
    Ok(matches.remove(0))
}

fn collect_matching_files(dir: &Path, filename: &str, matches: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir).map_err(Error::Io)? {
        let entry = entry.map_err(Error::Io)?;
        let path = entry.path();
        if path.is_dir() {
            collect_matching_files(&path, filename, matches)?;
        } else if path.file_name().and_then(|name| name.to_str()) == Some(filename) {
            matches.push(path);
        }
    }
    Ok(())
}

fn validate_release_quality_contract(
    quality: Option<&QualityMetrics>,
    output_format: OutputFormat,
    has_quality_ground_truth: bool,
    has_structural_ground_truth: bool,
    path: &Path,
    context: &str,
) -> Result<()> {
    require(
        quality.is_some() == has_quality_ground_truth,
        format!("{}: {context} quality presence mismatch", path.display()),
    )?;
    if let Some(quality) = quality {
        let expected_sf1 = output_format == OutputFormat::Markdown && has_structural_ground_truth;
        require(
            quality.f1_score_layout.is_some() == expected_sf1,
            format!("{}: {context} SF1 presence mismatch", path.display()),
        )?;
    }
    Ok(())
}

fn supported_fixture_indexes(
    entry: &MatrixEntry,
    contract: &CohortContract,
    ocr_languages: &[Option<String>],
) -> Vec<usize> {
    let supported = crate::adapters::external::declared_supported_formats(&entry.framework);
    contract
        .document_extensions
        .iter()
        .enumerate()
        .filter_map(|(index, extension)| {
            let format_supported =
                entry.framework.starts_with("xberg-") || supported.iter().any(|item| item == extension);
            let language_supported =
                declared_ocr_language_policy(&entry.framework).supports(ocr_languages[index].as_deref());
            (format_supported && language_supported).then_some(index)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expected_fixture_hashing_rejects_document_path_escape() {
        const FIXTURES: &[&str] = &["escape.json"];
        const STEMS: &[&str] = &["secret"];
        const EXTENSIONS: &[&str] = &["txt"];

        let root = tempfile::tempdir().unwrap();
        let fixtures_root = root.path().join("fixtures");
        std::fs::create_dir(&fixtures_root).unwrap();
        std::fs::write(root.path().join("secret.txt"), "outside fixture boundary").unwrap();
        std::fs::write(
            fixtures_root.join("escape.json"),
            serde_json::json!({
                "document": "../secret.txt",
                "file_type": "txt",
                "file_size": 24,
                "expected_frameworks": [],
                "metadata": {}
            })
            .to_string(),
        )
        .unwrap();
        let contract = CohortContract {
            manifest_name: "test",
            manifest_blake3: "unused",
            batch_size: 1,
            fixtures: FIXTURES,
            document_stems: STEMS,
            document_extensions: EXTENSIONS,
            matrix: Vec::new(),
        };

        let error = expected_fixtures(&fixtures_root, &contract).unwrap_err().to_string();

        assert!(error.contains("document escapes the fixture trust boundary"), "{error}");
    }
}
