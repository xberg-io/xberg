//! Fixture loading and management
//!
//! Fixtures are JSON files that describe test documents and their metadata.
//!
//! ## Fixture Format
//!
//! ```json
//! {
//!   "document": "path/to/document.pdf",
//!   "file_type": "pdf",
//!   "file_size": 1024000,
//!   "expected_frameworks": ["xberg", "docling"],
//!   // Note: frameworks can be Xberg language bindings or open source extraction alternatives
//!   "metadata": {
//!     "title": "Test Document",
//!     "pages": 10,
//!     "requires_ocr": false  // Optional: override OCR requirement detection
//!   },
//!   "ground_truth": {
//!     "text_file": "path/to/ground_truth.txt",
//!     "source": "pdf_text_layer"
//!   }
//! }
//! ```

use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};

/// The full set of extensions xberg's MIME table treats as interchangeable aliases of
/// `extension`'s format — e.g. `jpg` and `jpeg`, or `dbk`/`docbook`/`docbook4`/`docbook5`.
///
/// Returns `None` when xberg's MIME table doesn't recognize `extension` at all, so callers can
/// treat unrecognized extensions as "nothing to check" instead of a validation failure.
pub(crate) fn canonical_extension_family(extension: &str) -> Option<Vec<String>> {
    let probe_path = PathBuf::from(format!("probe.{extension}"));
    let mime_type = xberg::core::mime::detect_mime_type(&probe_path, false).ok()?;
    xberg::get_extensions_for_mime(&mime_type).ok()
}

pub(crate) fn is_split_sidecar(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(".split.json"))
}

/// A fixture describing a test document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fixture {
    /// Path to the test document (relative to fixture file)
    pub document: PathBuf,

    /// File type (extension without dot, e.g., "pdf")
    pub file_type: String,

    /// File size in bytes
    pub file_size: u64,

    /// Extraction frameworks that should be able to process this file
    /// (can be Xberg language bindings or open source extraction alternatives)
    #[serde(default)]
    pub expected_frameworks: Vec<String>,

    /// Additional metadata about the document
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,

    /// Ground truth for quality assessment (optional)
    #[serde(default)]
    pub ground_truth: Option<GroundTruth>,
}

/// Ground truth data for quality assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundTruth {
    /// Path to ground truth text file (optional — some fixtures only have markdown GT)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_file: Option<PathBuf>,

    /// Path to ground truth markdown file for structural quality scoring (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub markdown_file: Option<PathBuf>,

    /// Path to a flat `{ "field_name": "value" }` JSON file for form-field quality scoring.
    ///
    /// Used by the `field-quality` subcommand `FormFields` mode.
    /// Path is relative to the fixture file's directory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fields_json: Option<PathBuf>,

    /// Path to a `{ "formulas": ["latex1", ...] }` JSON file for formula quality scoring.
    ///
    /// Used by the `field-quality` subcommand `Formula` mode.
    /// Path is relative to the fixture file's directory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formulas_json: Option<PathBuf>,

    /// Source of the ground truth ("pdf_text_layer", "markdown_file", "manual")
    pub source: String,
}

impl Fixture {
    /// Load a fixture from a JSON file
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let contents = std::fs::read_to_string(path).map_err(Error::Io)?;
        let fixture: Fixture = serde_json::from_str(&contents)?;
        fixture.validate(path)?;
        Ok(fixture)
    }

    /// Validate the fixture
    ///
    /// Performs comprehensive validation including:
    /// - Path validation (relative paths only)
    /// - File type validation (non-empty, and naming the same format as the document's
    ///   extension per xberg's MIME alias table)
    /// - Ground truth validation:
    ///   - Relative path requirement
    ///   - Valid source type
    ///   - File existence check (relative to fixture directory)
    fn validate(&self, fixture_path: &Path) -> Result<()> {
        Self::validate_fixture_relative_path(fixture_path, &self.document, "document")?;

        if self.file_type.is_empty() {
            return Err(Error::InvalidFixture {
                path: fixture_path.to_path_buf(),
                reason: "file_type cannot be empty".to_string(),
            });
        }

        Self::validate_file_type_matches_document(fixture_path, &self.file_type, &self.document)?;
        Self::validate_ocr_language_metadata(fixture_path, &self.metadata)?;

        if let Some(gt) = &self.ground_truth {
            Self::validate_ground_truth_block(fixture_path, gt)?;
        }

        Ok(())
    }

    /// Validate the optional `metadata.ocr_language` override: when present, it must be a
    /// string of `+`-separated Tesseract language codes.
    fn validate_ocr_language_metadata(
        fixture_path: &Path,
        metadata: &HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        let Some(language) = metadata.get("ocr_language") else {
            return Ok(());
        };
        let Some(language) = language.as_str() else {
            return Err(Error::InvalidFixture {
                path: fixture_path.to_path_buf(),
                reason: "metadata.ocr_language must be a string".to_string(),
            });
        };
        let codes = crate::adapter::canonicalize_ocr_languages(language);
        if codes.is_empty()
            || codes
                .iter()
                .any(|code| !crate::adapter::is_valid_ocr_language_code(code))
        {
            return Err(Error::InvalidFixture {
                path: fixture_path.to_path_buf(),
                reason: "metadata.ocr_language must contain '+'-separated Tesseract language codes".to_string(),
            });
        }
        Ok(())
    }

    /// Validate a fixture's `ground_truth` block: relative-path shape of every referenced
    /// file, a recognized `source` value, and existence of the text/markdown ground truth
    /// files relative to the fixture directory.
    fn validate_ground_truth_block(fixture_path: &Path, gt: &GroundTruth) -> Result<()> {
        for (field, relative_path) in [
            ("ground_truth.text_file", gt.text_file.as_deref()),
            ("ground_truth.markdown_file", gt.markdown_file.as_deref()),
            ("ground_truth.fields_json", gt.fields_json.as_deref()),
            ("ground_truth.formulas_json", gt.formulas_json.as_deref()),
        ] {
            if let Some(relative_path) = relative_path {
                Self::validate_fixture_relative_path(fixture_path, relative_path, field)?;
            }
        }

        if !matches!(
            gt.source.as_str(),
            "pdf_text_layer"
                | "markdown_file"
                | "manual"
                | "vision"
                | "python-docx"
                | "python-pptx"
                | "openpyxl"
                | "codex-vision"
                | "raw_source"
                | "pandoc"
                | "python_email"
                | "extract_msg"
                | "nbformat"
                | "xml_parse"
                | "beautifulsoup"
                | "xlrd"
                | "antiword"
                | "libreoffice"
                | "odfpy"
                | "ebooklib"
                | "striprtf"
                | "pyxlsb"
                | "olefile"
                | "omnidocbench"
                | "mistral-pixtral"
                | "nougat"
                | "readoc"
                | "parsebench"
                | "fintabnet"
                | "federal_register"
                | "apple_preview"
        ) {
            return Err(Error::InvalidFixture {
                path: fixture_path.to_path_buf(),
                reason: format!("invalid ground_truth.source: {}", gt.source),
            });
        }

        Self::validate_ground_truth_file(fixture_path, gt.text_file.as_deref(), "text")?;
        Self::validate_ground_truth_file(fixture_path, gt.markdown_file.as_deref(), "markdown")?;

        Ok(())
    }

    /// Reject a `file_type` that names a format unrelated to the document's own extension.
    ///
    /// `file_type` need not literally equal the document's extension: this corpus deliberately
    /// keeps documents under several MIME-legal extension aliases of the same format (for
    /// example `docbook4`/`docbook5` documents labelled `"docbook"`, mirroring xberg's own
    /// `dbk`/`docbook`/`docbook4`/`docbook5` alias group), and some of those normalized labels
    /// are pinned by `guardrails.json`. What must never happen is `file_type` naming a
    /// *different* format than the one the document's own extension resolves to — e.g. a
    /// `.jpg` file labelled `"pdf"`, or (the bug this guards against) fourteen `.jpg` images
    /// labelled `"jpeg"` while every other `.jpg` fixture in the corpus correctly says `"jpg"`.
    /// Extensions xberg's MIME table doesn't recognize are skipped rather than rejected, so a
    /// legitimately new format never gets blocked by this check.
    fn validate_file_type_matches_document(fixture_path: &Path, file_type: &str, document: &Path) -> Result<()> {
        let Some(document_extension) = document.extension().and_then(|ext| ext.to_str()) else {
            return Ok(());
        };
        let file_type_lower = file_type.to_lowercase();
        let document_extension_lower = document_extension.to_lowercase();
        if file_type_lower == document_extension_lower {
            return Ok(());
        }
        let Some(family) = canonical_extension_family(&document_extension_lower) else {
            return Ok(());
        };
        if family.contains(&file_type_lower) {
            return Ok(());
        }
        Err(Error::InvalidFixture {
            path: fixture_path.to_path_buf(),
            reason: format!(
                "file_type '{file_type}' does not match document extension '.{document_extension}' and is not one \
                 of its recognized aliases ({}); use one of these",
                family.join(", ")
            ),
        })
    }

    /// Resolve a fixture-owned relative path without allowing it to escape its trust boundary.
    ///
    /// Repository fixtures intentionally use `../../../test_documents/...`, so their boundary is
    /// the repository root. Standalone fixture trees use the fixture file's directory. Existing
    /// paths are canonicalized as well, preventing a symlink inside either boundary from escaping.
    /// ~keep
    fn validate_fixture_relative_path(fixture_path: &Path, relative_path: &Path, field: &str) -> Result<PathBuf> {
        if relative_path.is_absolute() {
            return Err(Error::InvalidFixture {
                path: fixture_path.to_path_buf(),
                reason: format!("{field} must be relative"),
            });
        }

        // A bare filename has an empty parent path, which must resolve relative to the current
        // directory just like a missing parent. Keep the canonical trust boundary check below. ~keep
        let fixture_dir = fixture_path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let canonical_fixture_dir = fixture_dir.canonicalize().map_err(|error| Error::InvalidFixture {
            path: fixture_path.to_path_buf(),
            reason: format!("unable to resolve fixture directory {}: {error}", fixture_dir.display()),
        })?;
        // Resolve the checked-out repository from the runtime fixture location. The harness
        // binary is cached and executed by separate CI jobs, so its build-time source path may
        // not exist on the runner that validates fixtures. ~keep
        let repository_root = repository_fixture_root(&canonical_fixture_dir);
        let allowed_root = repository_root.as_deref().unwrap_or(&canonical_fixture_dir);
        let resolved = normalize_path(&canonical_fixture_dir.join(relative_path));
        if !resolved.starts_with(allowed_root) {
            return Err(Error::InvalidFixture {
                path: fixture_path.to_path_buf(),
                reason: format!(
                    "{field} escapes the fixture trust boundary: {}",
                    relative_path.display()
                ),
            });
        }

        if resolved.exists() {
            let canonical = resolved.canonicalize().map_err(|error| Error::InvalidFixture {
                path: fixture_path.to_path_buf(),
                reason: format!("unable to resolve {field} {}: {error}", relative_path.display()),
            })?;
            if !canonical.starts_with(allowed_root) {
                return Err(Error::InvalidFixture {
                    path: fixture_path.to_path_buf(),
                    reason: format!(
                        "{field} resolves outside the fixture trust boundary: {}",
                        relative_path.display()
                    ),
                });
            }
            return Ok(canonical);
        }
        Ok(resolved)
    }

    pub(crate) fn validated_document_path(&self, fixture_path: &Path) -> Result<PathBuf> {
        Self::validate_fixture_relative_path(fixture_path, &self.document, "document")
    }

    fn validate_ground_truth_file(fixture_path: &Path, relative_path: Option<&Path>, kind: &str) -> Result<()> {
        let Some(relative_path) = relative_path else {
            return Ok(());
        };
        let resolved_path = Self::validate_fixture_relative_path(fixture_path, relative_path, kind)?;
        std::fs::read_to_string(&resolved_path).map_err(|error| Error::InvalidFixture {
            path: fixture_path.to_path_buf(),
            reason: format!(
                "unable to read ground truth {kind} file {} (resolved to {}): {error}",
                relative_path.display(),
                resolved_path.display()
            ),
        })?;
        Ok(())
    }

    /// Resolve document path relative to fixture file
    pub fn resolve_document_path(&self, fixture_dir: &Path) -> PathBuf {
        fixture_dir.join(&self.document)
    }

    /// Resolve ground truth path relative to fixture file
    pub fn resolve_ground_truth_path(&self, fixture_dir: &Path) -> Option<PathBuf> {
        self.ground_truth
            .as_ref()
            .and_then(|gt| gt.text_file.as_ref().map(|tf| fixture_dir.join(tf)))
    }

    /// Resolve ground truth markdown path relative to fixture file
    pub fn resolve_ground_truth_markdown_path(&self, fixture_dir: &Path) -> Option<PathBuf> {
        self.ground_truth
            .as_ref()
            .and_then(|gt| gt.markdown_file.as_ref().map(|mf| fixture_dir.join(mf)))
    }

    /// Determine if this fixture requires OCR based on file type and metadata
    pub fn requires_ocr(&self) -> bool {
        if let Some(requires_ocr) = self.metadata.get("requires_ocr").and_then(|v| v.as_bool()) {
            return requires_ocr;
        }

        matches!(
            self.file_type.to_lowercase().as_str(),
            "jpg" | "jpeg" | "png" | "gif" | "bmp" | "tiff" | "tif" | "webp" | "jp2" | "jpx" | "jpm" | "mj2"
        )
    }

    /// Return the fixture-specific OCR language code, if configured.
    pub fn ocr_language(&self) -> Option<&str> {
        self.metadata.get("ocr_language").and_then(|value| value.as_str())
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

fn repository_fixture_root(fixture_dir: &Path) -> Option<PathBuf> {
    fixture_dir
        .ancestors()
        .find(|ancestor| {
            let harness_root = ancestor.join("tools/benchmark-harness");
            harness_root.join("Cargo.toml").is_file() && fixture_dir.starts_with(harness_root.join("fixtures"))
        })
        .map(Path::to_path_buf)
}

/// Manages loading and accessing fixtures
pub struct FixtureManager {
    fixtures: Vec<(PathBuf, Fixture)>,
}

impl FixtureManager {
    /// Create a new empty fixture manager
    pub fn new() -> Self {
        Self { fixtures: Vec::new() }
    }

    /// Load a single fixture file
    pub fn load_fixture(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(Error::FixtureNotFound(path.to_path_buf()));
        }

        let fixture = Fixture::from_file(path)?;
        self.fixtures.push((path.to_path_buf(), fixture));

        Ok(())
    }

    /// Parse profiling fixtures from environment variable
    ///
    /// Reads the `PROFILING_FIXTURES` environment variable (comma-separated fixture names).
    /// Returns a HashSet of fixture names to use during profiling runs.
    ///
    /// # Examples
    ///
    /// ```text
    /// PROFILING_FIXTURES="pdf_small,pdf_medium,docx_simple" -> {pdf_small, pdf_medium, docx_simple}
    /// ```
    fn get_profiling_fixtures() -> Option<HashSet<String>> {
        std::env::var("PROFILING_FIXTURES")
            .ok()
            .map(|fixtures_str| {
                fixtures_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect::<HashSet<String>>()
            })
            .filter(|set| !set.is_empty())
    }

    /// Load all fixtures from a directory (recursively)
    ///
    /// If the `PROFILING_FIXTURES` environment variable is set, only fixtures matching
    /// the specified names (comma-separated) will be loaded. Otherwise, all fixtures are loaded.
    pub fn load_fixtures_from_dir(&mut self, dir: impl AsRef<Path>) -> Result<()> {
        self.load_fixtures_from_dir_internal(dir, true)
    }

    /// Internal method for loading fixtures from a directory (with filter control)
    fn load_fixtures_from_dir_internal(&mut self, dir: impl AsRef<Path>, apply_filter: bool) -> Result<()> {
        let dir = dir.as_ref();

        if !dir.exists() {
            return Err(Error::FixtureNotFound(dir.to_path_buf()));
        }

        let all_fixtures = Self::collect_fixture_paths(dir)?;
        let total_fixtures = all_fixtures.len();
        let mut failed_fixtures: Vec<(PathBuf, String)> = Vec::new();

        if apply_filter {
            self.load_with_profiling_filter(all_fixtures, total_fixtures, &mut failed_fixtures);
        } else {
            self.load_all(all_fixtures, &mut failed_fixtures);
        }

        Self::fail_on_any_failure(total_fixtures, failed_fixtures)
    }

    /// Recursively collect every non-sidecar fixture JSON path under `dir`, sorted for
    /// deterministic ordering. Subdirectories are walked by loading (and thereby validating)
    /// a scratch [`FixtureManager`] and harvesting its fixture paths.
    fn collect_fixture_paths(dir: &Path) -> Result<Vec<PathBuf>> {
        let mut all_fixtures: Vec<PathBuf> = Vec::new();

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                let mut temp_manager = FixtureManager::new();
                temp_manager.load_fixtures_from_dir_internal(&path, false)?;
                for (fixture_path, _) in temp_manager.fixtures {
                    all_fixtures.push(fixture_path);
                }
            } else if path.extension().and_then(|s| s.to_str()) == Some("json") && !is_split_sidecar(&path) {
                all_fixtures.push(path);
            }
        }
        all_fixtures.sort();
        Ok(all_fixtures)
    }

    /// Load only fixtures named by `PROFILING_FIXTURES`, falling back to loading everything
    /// when the variable is unset or matches nothing.
    fn load_with_profiling_filter(
        &mut self,
        all_fixtures: Vec<PathBuf>,
        total_fixtures: usize,
        failed_fixtures: &mut Vec<(PathBuf, String)>,
    ) {
        let Some(profiling_set) = Self::get_profiling_fixtures() else {
            self.load_all(all_fixtures, failed_fixtures);
            return;
        };

        let mut loaded_count = 0;
        let mut fixture_names = Vec::new();

        for fixture_path in &all_fixtures {
            if let Some(stem) = fixture_path.file_stem().and_then(|s| s.to_str())
                && profiling_set.contains(stem)
            {
                match self.load_fixture(fixture_path) {
                    Ok(()) => {
                        loaded_count += 1;
                        fixture_names.push(stem.to_string());
                    }
                    Err(e) => {
                        failed_fixtures.push((fixture_path.clone(), e.to_string()));
                    }
                }
            }
        }

        if loaded_count > 0 {
            fixture_names.sort();
            eprintln!(
                "Profiling mode: Using {} of {} fixtures: {}",
                loaded_count,
                total_fixtures,
                fixture_names.join(", ")
            );
        } else {
            eprintln!(
                "Warning: PROFILING_FIXTURES set but no matching fixtures found. \
                Loading all {} fixtures.",
                total_fixtures
            );
            self.load_all(all_fixtures, failed_fixtures);
        }
    }

    /// Load every fixture in `fixture_paths`, recording failures instead of aborting the scan.
    fn load_all(&mut self, fixture_paths: Vec<PathBuf>, failed_fixtures: &mut Vec<(PathBuf, String)>) {
        for fixture_path in fixture_paths {
            match self.load_fixture(&fixture_path) {
                Ok(()) => {}
                Err(e) => {
                    failed_fixtures.push((fixture_path.clone(), e.to_string()));
                }
            }
        }
    }

    /// Turn accumulated per-fixture failures into a single aggregate error, or `Ok(())` when
    /// there were none.
    fn fail_on_any_failure(total_fixtures: usize, failed_fixtures: Vec<(PathBuf, String)>) -> Result<()> {
        if failed_fixtures.is_empty() {
            return Ok(());
        }
        let first_path = failed_fixtures[0].0.clone();
        let details = failed_fixtures
            .iter()
            .take(10)
            .map(|(path, error)| format!("{}: {error}", path.display()))
            .collect::<Vec<_>>()
            .join("; ");
        Err(Error::InvalidFixture {
            path: first_path,
            reason: format!(
                "{} of {} requested fixtures failed validation: {}",
                failed_fixtures.len(),
                total_fixtures,
                details
            ),
        })
    }

    /// Get all loaded fixtures
    pub fn fixtures(&self) -> &[(PathBuf, Fixture)] {
        &self.fixtures
    }

    /// Get count of loaded fixtures
    pub fn len(&self) -> usize {
        self.fixtures.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.fixtures.is_empty()
    }

    /// Filter fixtures by file type
    pub fn filter_by_type(&self, file_types: &[String]) -> Vec<(PathBuf, Fixture)> {
        self.fixtures
            .iter()
            .filter(|(_, fixture)| file_types.contains(&fixture.file_type))
            .cloned()
            .collect()
    }

    /// Retain only the fixtures belonging to shard `index` of `total` shards.
    ///
    /// Fixtures are sorted by path for deterministic ordering, then assigned
    /// round-robin to shards. This ensures even distribution across shards
    /// regardless of file type or size ordering.
    ///
    /// `index` is 1-based (1..=total).
    pub fn retain_shard(&mut self, index: usize, total: usize) {
        assert!(index >= 1 && index <= total, "shard index must be 1..=total");
        self.fixtures.sort_by(|a, b| a.0.cmp(&b.0));
        let shard_index = index - 1;
        self.fixtures = self
            .fixtures
            .drain(..)
            .enumerate()
            .filter(|(i, _)| i % total == shard_index)
            .map(|(_, f)| f)
            .collect();
    }
}

impl Default for FixtureManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
