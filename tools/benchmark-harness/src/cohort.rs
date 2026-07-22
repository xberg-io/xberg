//! Exact, ordered benchmark cohort manifests.

use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::fixture::FixtureManager;
use crate::{Error, Result};

const COHORT_SCHEMA_VERSION: u32 = 1;

/// A reproducible ordered fixture selection and its fixed native batch size.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CohortManifest {
    pub schema_version: u32,
    pub name: String,
    pub batch_size: usize,
    pub fixtures: Vec<PathBuf>,
}

impl CohortManifest {
    /// Load and validate a cohort manifest.
    pub fn from_file(path: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path)?;
        let manifest: Self = serde_json::from_str(&raw)?;
        manifest.validate(path)?;
        Ok(manifest)
    }

    /// Load the manifest's fixtures in exactly the declared order.
    pub fn load_fixtures(&self, fixture_root: &Path, manifest_path: &Path) -> Result<FixtureManager> {
        if !fixture_root.is_dir() {
            return Err(Error::Config(format!(
                "cohort fixture root must be a directory: {}",
                fixture_root.display()
            )));
        }

        let mut manager = FixtureManager::new();
        for relative in &self.fixtures {
            let resolved = fixture_root.join(relative);
            manager.load_fixture(&resolved).map_err(|error| {
                Error::Config(format!(
                    "cohort '{}' fixture '{}' from {} failed to load: {error}",
                    self.name,
                    relative.display(),
                    manifest_path.display()
                ))
            })?;
        }
        Ok(manager)
    }

    fn validate(&self, path: &Path) -> Result<()> {
        if self.schema_version != COHORT_SCHEMA_VERSION {
            return Err(Error::Config(format!(
                "unsupported cohort schema_version {} in {}; expected {}",
                self.schema_version,
                path.display(),
                COHORT_SCHEMA_VERSION
            )));
        }
        if self.name.trim().is_empty() {
            return Err(Error::Config(format!(
                "cohort name must not be empty in {}",
                path.display()
            )));
        }
        if self.batch_size == 0 {
            return Err(Error::Config(format!(
                "cohort batch_size must be greater than zero in {}",
                path.display()
            )));
        }
        if self.fixtures.is_empty() {
            return Err(Error::Config(format!(
                "cohort fixtures must not be empty in {}",
                path.display()
            )));
        }
        if !self.fixtures.len().is_multiple_of(self.batch_size) {
            return Err(Error::Config(format!(
                "cohort '{}' contains {} fixtures, which is not divisible by fixed batch_size {}",
                self.name,
                self.fixtures.len(),
                self.batch_size
            )));
        }

        let mut seen = HashSet::with_capacity(self.fixtures.len());
        for fixture in &self.fixtures {
            let valid_relative = !fixture.as_os_str().is_empty()
                && !fixture.is_absolute()
                && fixture
                    .components()
                    .all(|component| matches!(component, Component::Normal(_)));
            if !valid_relative {
                return Err(Error::Config(format!(
                    "cohort fixture paths must be normalized relative paths without '..': {}",
                    fixture.display()
                )));
            }
            if fixture.extension().and_then(|extension| extension.to_str()) != Some("json") {
                return Err(Error::Config(format!(
                    "cohort fixture path must name a JSON descriptor: {}",
                    fixture.display()
                )));
            }
            if !seen.insert(fixture.clone()) {
                return Err(Error::Config(format!(
                    "cohort fixture path is duplicated: {}",
                    fixture.display()
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_manifest(dir: &Path, value: serde_json::Value) -> PathBuf {
        let path = dir.join("cohort.json");
        std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        path
    }

    #[test]
    fn validates_ordered_fixed_size_cohort() {
        let temp = tempfile::tempdir().unwrap();
        let path = write_manifest(
            temp.path(),
            serde_json::json!({
                "schema_version": 1,
                "name": "ordered",
                "batch_size": 2,
                "fixtures": ["b.json", "a.json"]
            }),
        );
        let manifest = CohortManifest::from_file(&path).unwrap();
        assert_eq!(manifest.fixtures, [PathBuf::from("b.json"), PathBuf::from("a.json")]);
    }

    #[test]
    fn rejects_partial_duplicate_and_parent_paths() {
        let temp = tempfile::tempdir().unwrap();
        for fixtures in [
            serde_json::json!(["a.json"]),
            serde_json::json!(["a.json", "a.json"]),
            serde_json::json!(["../a.json", "b.json"]),
        ] {
            let path = write_manifest(
                temp.path(),
                serde_json::json!({
                    "schema_version": 1,
                    "name": "invalid",
                    "batch_size": 2,
                    "fixtures": fixtures
                }),
            );
            assert!(CohortManifest::from_file(&path).is_err());
        }
    }
}
