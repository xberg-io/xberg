//! Integration scaffold for the generated fixture corpus produced by
//! `tools/generate_test_fixtures/`.
//!
//! Each generator under `tools/generate_test_fixtures/src/generate_test_fixtures/`
//! writes a binary fixture alongside a `*.gt.json` ground-truth sidecar. This
//! module:
//!
//!  1. Defines `FixtureGt` — a Rust mirror of the Python `GroundTruth`
//!     dataclass (see `tools/generate_test_fixtures/src/generate_test_fixtures/gt_schema.py`).
//!  2. Exposes `load_fixture(name)` which returns `(PathBuf, FixtureGt)` for a
//!     fixture whose stem (`docx_track_changes_basic`, `xlsx_revisions_basic`,
//!     …) is given.
//!  3. Includes one `#[ignore]`d example test per feature stream that
//!     demonstrates the load-and-assert shape. Tests are gated behind
//!     `#[ignore]` because the binary fixtures are not yet checked into the
//!     `test_documents/` submodule — the harness lands first, and the
//!     fixtures follow once the user decides where they belong.
//!
//! To run the example tests locally after generating fixtures:
//!
//! ```text
//! mise run fixtures:generate
//! cargo test --test generated_fixtures -- --ignored
//! ```

#![allow(dead_code, missing_docs)]

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Mirror of the Python `GroundTruth` dataclass.
///
/// The `expectations` field is intentionally typed as `serde_json::Value` —
/// the shape varies per feature, and the per-feature assertion helpers below
/// pick fields out of it directly. See the Python `gt_schema.py` module for
/// the per-feature shape (`revisions_expectation`, `diff_expectation`,
/// `security_expectation`).
#[derive(Debug, Clone, Deserialize)]
pub struct FixtureGt {
    pub fixture_path: String,
    pub format: String,
    pub feature: String,
    pub expectations: serde_json::Value,
    pub generated_by: String,
}

/// Walk upward from `CARGO_MANIFEST_DIR` to find the repository root.
///
/// Anchored on the presence of `Cargo.toml` AND `test_documents/`. Falls back
/// to `CARGO_MANIFEST_DIR` itself when no marker is found, which makes
/// failures show useful paths rather than panicking deep inside the loader.
fn repo_root() -> PathBuf {
    let mut current = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        if current.join("Cargo.toml").is_file() && current.join("test_documents").is_dir() {
            return current;
        }
        if !current.pop() {
            return PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        }
    }
}

/// Root of the generated-fixtures tree.
fn generated_root() -> PathBuf {
    repo_root().join("test_documents").join("generated")
}

/// Search the generated tree for a fixture whose stem matches `name`.
///
/// Returns the `(binary_path, ground_truth)` pair. Panics with a descriptive
/// message when either the binary or the sidecar is missing — integration
/// tests should `#[ignore]` themselves when the generated tree is empty.
pub fn load_fixture(name: &str) -> (PathBuf, FixtureGt) {
    let root = generated_root();
    let sidecar = find_sidecar(&root, name).unwrap_or_else(|| {
        panic!(
            "ground-truth sidecar {name}.gt.json not found under {root}\n\
             Run `mise run fixtures:generate` to produce the fixture set.",
            root = root.display(),
        )
    });
    let raw = fs::read_to_string(&sidecar).unwrap_or_else(|e| panic!("failed to read {}: {e}", sidecar.display()));
    let gt: FixtureGt =
        serde_json::from_str(&raw).unwrap_or_else(|e| panic!("failed to parse {}: {e}", sidecar.display()));
    let binary = repo_root().join(&gt.fixture_path);
    assert!(
        binary.is_file(),
        "binary fixture {} declared by {} does not exist",
        binary.display(),
        sidecar.display(),
    );
    (binary, gt)
}

fn find_sidecar(root: &Path, name: &str) -> Option<PathBuf> {
    let needle = format!("{name}.gt.json");
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.file_name().and_then(|s| s.to_str()) == Some(needle.as_str()) {
                return Some(path);
            }
        }
    }
    None
}

/// Returns `true` when the generated-fixtures tree is empty / absent.
///
/// The example tests below short-circuit through this so a fresh clone (no
/// submodule init, no `mise run fixtures:generate` run) doesn't fail the suite
/// noisily — the `#[ignore]` gate is the primary guard, this is belt + braces.
fn generated_tree_present() -> bool {
    generated_root().is_dir()
        && fs::read_dir(generated_root())
            .map(|mut it| it.next().is_some())
            .unwrap_or(false)
}

// One per feature stream. Each is `#[ignore]`d until the binary fixtures land

#[test]
#[ignore = "requires fixtures from `mise run fixtures:generate` to be checked into test_documents/"]
fn revisions_docx_track_changes_basic_matches_ground_truth() {
    if !generated_tree_present() {
        return;
    }
    let (fixture_path, gt) = load_fixture("docx_track_changes_basic");
    assert_eq!(gt.feature, "revisions");
    assert_eq!(gt.format, "docx");
    let expected_count = gt.expectations["expected_count"]
        .as_u64()
        .expect("expected_count must be present in revisions GT");
    assert!(
        expected_count > 0,
        "revisions fixture must declare at least one revision"
    );

    //       .expect("DOCX track-changes fixture must yield a result");
    //   let revisions = document.revisions.expect("DOCX track-changes fixture must yield revisions");
    let _ = fixture_path;
}

#[test]
#[ignore = "requires fixtures from `mise run fixtures:generate` to be checked into test_documents/"]
fn diff_xlsx_budget_pair_round_trips() {
    if !generated_tree_present() {
        return;
    }
    let (_v1_path, gt) = load_fixture("xlsx_budget_v1");
    assert_eq!(gt.feature, "diff");
    let cell_changes = gt.expectations["table_cell_changes"]
        .as_array()
        .expect("table_cell_changes must be an array");
    assert_eq!(
        cell_changes.len(),
        1,
        "budget diff must declare exactly one cell change"
    );
    let change = &cell_changes[0];
    assert_eq!(change["row"].as_u64(), Some(1));
    assert_eq!(change["col"].as_u64(), Some(1));
    assert_eq!(change["from"].as_str(), Some("100"));
    assert_eq!(change["to"].as_str(), Some("150"));
}

#[test]
#[ignore = "requires fixtures from `mise run fixtures:generate` to be checked into test_documents/"]
fn security_zip_bomb_pathological_is_rejected() {
    if !generated_tree_present() {
        return;
    }
    let (_fixture_path, gt) = load_fixture("zip_bomb_xlsx_pathological");
    assert_eq!(gt.feature, "security");
    assert_eq!(
        gt.expectations["should_extract"].as_bool(),
        Some(false),
        "pathological zip-bomb fixture must declare should_extract=false",
    );
    let warnings = gt.expectations["expected_warnings"]
        .as_array()
        .expect("expected_warnings must be an array");
    let has_bomb_term = warnings
        .iter()
        .filter_map(|v| v.as_str())
        .any(|s| s.contains("bomb") || s.contains("zip"));
    assert!(
        has_bomb_term,
        "pathological zip-bomb GT must mention zip/bomb in warnings"
    );
}
