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
//!   "expected_frameworks": ["kreuzberg", "docling"],
//!   // Note: frameworks can be Kreuzberg language bindings or open source extraction alternatives
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
use std::path::{Path, PathBuf};

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
    /// (can be Kreuzberg language bindings or open source extraction alternatives)
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
    /// Path to ground truth text file
    pub text_file: PathBuf,

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
    /// - File type validation (non-empty)
    /// - Ground truth validation:
    ///   - Relative path requirement
    ///   - Valid source type
    ///   - File existence check (relative to fixture directory)
    fn validate(&self, fixture_path: &Path) -> Result<()> {
        if self.document.is_absolute() {
            return Err(Error::InvalidFixture {
                path: fixture_path.to_path_buf(),
                reason: "document path must be relative".to_string(),
            });
        }

        if self.file_type.is_empty() {
            return Err(Error::InvalidFixture {
                path: fixture_path.to_path_buf(),
                reason: "file_type cannot be empty".to_string(),
            });
        }

        if let Some(gt) = &self.ground_truth {
            if gt.text_file.is_absolute() {
                return Err(Error::InvalidFixture {
                    path: fixture_path.to_path_buf(),
                    reason: "ground_truth.text_file must be relative".to_string(),
                });
            }

            if !matches!(
                gt.source.as_str(),
                "pdf_text_layer"
                    | "markdown_file"
                    | "manual"
                    | "pdftotext"
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
            ) {
                return Err(Error::InvalidFixture {
                    path: fixture_path.to_path_buf(),
                    reason: format!("invalid ground_truth.source: {}", gt.source),
                });
            }

            // Validate that ground truth file exists at load time
            // Use fixture directory as the base for relative paths
            if let Some(fixture_dir) = fixture_path.parent() {
                let ground_truth_path = fixture_dir.join(&gt.text_file);
                if !ground_truth_path.exists() {
                    return Err(Error::InvalidFixture {
                        path: fixture_path.to_path_buf(),
                        reason: format!(
                            "ground truth file not found: {} (resolved to {})",
                            gt.text_file.display(),
                            ground_truth_path.display()
                        ),
                    });
                }
            }
        }

        Ok(())
    }

    /// Resolve document path relative to fixture file
    pub fn resolve_document_path(&self, fixture_dir: &Path) -> PathBuf {
        fixture_dir.join(&self.document)
    }

    /// Resolve ground truth path relative to fixture file
    pub fn resolve_ground_truth_path(&self, fixture_dir: &Path) -> Option<PathBuf> {
        self.ground_truth.as_ref().map(|gt| fixture_dir.join(&gt.text_file))
    }

    /// Determine if this fixture requires OCR based on file type and metadata
    pub fn requires_ocr(&self) -> bool {
        // Check if explicitly marked in metadata
        if let Some(requires_ocr) = self.metadata.get("requires_ocr").and_then(|v| v.as_bool()) {
            return requires_ocr;
        }

        // Infer from file type - images always need OCR
        matches!(
            self.file_type.to_lowercase().as_str(),
            "jpg" | "jpeg" | "png" | "gif" | "bmp" | "tiff" | "webp"
        )
    }
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
            .and_then(|set: HashSet<String>| if set.is_empty() { None } else { Some(set) })
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
            } else if path.extension().and_then(|s| s.to_str()) == Some("json") {
                all_fixtures.push(path);
            }
        }

        let total_fixtures = all_fixtures.len();
        let mut failed_fixtures: Vec<(PathBuf, String)> = Vec::new();

        if apply_filter {
            if let Some(profiling_set) = Self::get_profiling_fixtures() {
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
                    for fixture_path in all_fixtures {
                        match self.load_fixture(&fixture_path) {
                            Ok(()) => {
                                // Successfully loaded
                            }
                            Err(e) => {
                                failed_fixtures.push((fixture_path.clone(), e.to_string()));
                            }
                        }
                    }
                }
            } else {
                for fixture_path in all_fixtures {
                    match self.load_fixture(&fixture_path) {
                        Ok(()) => {
                            // Successfully loaded
                        }
                        Err(e) => {
                            failed_fixtures.push((fixture_path.clone(), e.to_string()));
                        }
                    }
                }
            }
        } else {
            for fixture_path in all_fixtures {
                match self.load_fixture(&fixture_path) {
                    Ok(()) => {
                        // Successfully loaded
                    }
                    Err(e) => {
                        failed_fixtures.push((fixture_path.clone(), e.to_string()));
                    }
                }
            }
        }

        // Report failed fixtures if any occurred
        if !failed_fixtures.is_empty() {
            eprintln!(
                "Warning: {} of {} fixtures failed to load:",
                failed_fixtures.len(),
                total_fixtures
            );
            for (path, error) in failed_fixtures {
                eprintln!("  - {}: {}", path.display(), error);
            }
        }

        Ok(())
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
        // Sort by path for deterministic assignment across jobs
        self.fixtures.sort_by(|a, b| a.0.cmp(&b.0));
        let shard_index = index - 1; // convert to 0-based
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
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tempfile::TempDir;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_fixture_validation() {
        let fixture = Fixture {
            document: PathBuf::from("test.pdf"),
            file_type: "pdf".to_string(),
            file_size: 1024,
            expected_frameworks: vec!["kreuzberg".to_string()],
            metadata: HashMap::new(),
            ground_truth: None,
        };

        assert!(fixture.validate(Path::new("fixture.json")).is_ok());
    }

    #[test]
    fn test_absolute_path_rejected() {
        #[cfg(windows)]
        let absolute_path = PathBuf::from("C:\\absolute\\path\\test.pdf");
        #[cfg(not(windows))]
        let absolute_path = PathBuf::from("/absolute/path/test.pdf");

        let fixture = Fixture {
            document: absolute_path,
            file_type: "pdf".to_string(),
            file_size: 1024,
            expected_frameworks: vec![],
            metadata: HashMap::new(),
            ground_truth: None,
        };

        assert!(fixture.validate(Path::new("fixture.json")).is_err());
    }

    #[test]
    fn test_fixture_manager_load() {
        let temp_dir = TempDir::new().unwrap();
        let fixture_path = temp_dir.path().join("test.json");

        let fixture = Fixture {
            document: PathBuf::from("test.pdf"),
            file_type: "pdf".to_string(),
            file_size: 1024,
            expected_frameworks: vec![],
            metadata: HashMap::new(),
            ground_truth: None,
        };

        std::fs::write(&fixture_path, serde_json::to_string(&fixture).unwrap()).unwrap();

        let mut manager = FixtureManager::new();
        assert!(manager.load_fixture(&fixture_path).is_ok());
        assert_eq!(manager.len(), 1);
    }

    #[test]
    fn test_profiling_fixtures_with_env_var() {
        let _lock = ENV_LOCK.lock().unwrap();
        let temp_dir = TempDir::new().unwrap();

        let fixtures = vec!["pdf_small", "pdf_medium", "docx_simple", "html_simple"];
        for fixture_name in &fixtures {
            let fixture_path = temp_dir.path().join(format!("{}.json", fixture_name));
            let fixture = Fixture {
                document: PathBuf::from(format!("{}.pdf", fixture_name)),
                file_type: "pdf".to_string(),
                file_size: 1024,
                expected_frameworks: vec![],
                metadata: HashMap::new(),
                ground_truth: None,
            };
            std::fs::write(&fixture_path, serde_json::to_string(&fixture).unwrap()).unwrap();
        }

        unsafe {
            std::env::set_var("PROFILING_FIXTURES", "pdf_small,docx_simple");
        }

        let mut manager = FixtureManager::new();
        manager.load_fixtures_from_dir(temp_dir.path()).unwrap();

        assert_eq!(manager.len(), 2);

        let loaded_names: Vec<String> = manager
            .fixtures()
            .iter()
            .filter_map(|(path, _)| path.file_stem().and_then(|s| s.to_str()).map(|s| s.to_string()))
            .collect();

        assert!(loaded_names.contains(&"pdf_small".to_string()));
        assert!(loaded_names.contains(&"docx_simple".to_string()));
        assert!(!loaded_names.contains(&"pdf_medium".to_string()));
        assert!(!loaded_names.contains(&"html_simple".to_string()));

        unsafe {
            std::env::remove_var("PROFILING_FIXTURES");
        }
    }

    #[test]
    fn test_profiling_fixtures_all_when_env_not_set() {
        let _lock = ENV_LOCK.lock().unwrap();
        let temp_dir = TempDir::new().unwrap();

        let fixtures = vec!["pdf_small", "pdf_medium", "docx_simple"];
        for fixture_name in &fixtures {
            let fixture_path = temp_dir.path().join(format!("{}.json", fixture_name));
            let fixture = Fixture {
                document: PathBuf::from(format!("{}.pdf", fixture_name)),
                file_type: "pdf".to_string(),
                file_size: 1024,
                expected_frameworks: vec![],
                metadata: HashMap::new(),
                ground_truth: None,
            };
            std::fs::write(&fixture_path, serde_json::to_string(&fixture).unwrap()).unwrap();
        }

        unsafe {
            std::env::remove_var("PROFILING_FIXTURES");
        }

        let mut manager = FixtureManager::new();
        manager.load_fixtures_from_dir(temp_dir.path()).unwrap();

        assert_eq!(manager.len(), 3);
    }

    #[test]
    fn test_profiling_fixtures_with_whitespace() {
        let _lock = ENV_LOCK.lock().unwrap();
        let temp_dir = TempDir::new().unwrap();

        let fixtures = vec!["pdf_small", "pdf_medium", "docx_simple"];
        for fixture_name in &fixtures {
            let fixture_path = temp_dir.path().join(format!("{}.json", fixture_name));
            let fixture = Fixture {
                document: PathBuf::from(format!("{}.pdf", fixture_name)),
                file_type: "pdf".to_string(),
                file_size: 1024,
                expected_frameworks: vec![],
                metadata: HashMap::new(),
                ground_truth: None,
            };
            std::fs::write(&fixture_path, serde_json::to_string(&fixture).unwrap()).unwrap();
        }

        unsafe {
            std::env::set_var("PROFILING_FIXTURES", "pdf_small , pdf_medium , docx_simple");
        }

        let mut manager = FixtureManager::new();
        manager.load_fixtures_from_dir(temp_dir.path()).unwrap();

        assert_eq!(manager.len(), 3);

        unsafe {
            std::env::remove_var("PROFILING_FIXTURES");
        }
    }

    #[test]
    fn test_profiling_fixtures_partial_match() {
        let _lock = ENV_LOCK.lock().unwrap();
        let temp_dir = TempDir::new().unwrap();

        let fixtures = vec!["pdf_small", "pdf_medium", "docx_simple"];
        for fixture_name in &fixtures {
            let fixture_path = temp_dir.path().join(format!("{}.json", fixture_name));
            let fixture = Fixture {
                document: PathBuf::from(format!("{}.pdf", fixture_name)),
                file_type: "pdf".to_string(),
                file_size: 1024,
                expected_frameworks: vec![],
                metadata: HashMap::new(),
                ground_truth: None,
            };
            std::fs::write(&fixture_path, serde_json::to_string(&fixture).unwrap()).unwrap();
        }

        unsafe {
            std::env::set_var("PROFILING_FIXTURES", "pdf_small,nonexistent_fixture");
        }

        let mut manager = FixtureManager::new();
        manager.load_fixtures_from_dir(temp_dir.path()).unwrap();

        assert_eq!(manager.len(), 1);

        let loaded_names: Vec<String> = manager
            .fixtures()
            .iter()
            .filter_map(|(path, _)| path.file_stem().and_then(|s| s.to_str()).map(|s| s.to_string()))
            .collect();

        assert!(loaded_names.contains(&"pdf_small".to_string()));

        unsafe {
            std::env::remove_var("PROFILING_FIXTURES");
        }
    }

    #[test]
    fn test_requires_ocr_for_image_types() {
        let image_types = vec!["jpg", "jpeg", "png", "gif", "bmp", "tiff", "webp"];

        for file_type in image_types {
            let fixture = Fixture {
                document: PathBuf::from(format!("test.{}", file_type)),
                file_type: file_type.to_string(),
                file_size: 1024,
                expected_frameworks: vec![],
                metadata: HashMap::new(),
                ground_truth: None,
            };

            assert!(
                fixture.requires_ocr(),
                "Expected file type {} to require OCR",
                file_type
            );
        }
    }

    #[test]
    fn test_requires_ocr_for_non_image_types() {
        let non_image_types = vec!["pdf", "docx", "txt", "html", "md"];

        for file_type in non_image_types {
            let fixture = Fixture {
                document: PathBuf::from(format!("test.{}", file_type)),
                file_type: file_type.to_string(),
                file_size: 1024,
                expected_frameworks: vec![],
                metadata: HashMap::new(),
                ground_truth: None,
            };

            assert!(
                !fixture.requires_ocr(),
                "Expected file type {} to not require OCR",
                file_type
            );
        }
    }

    #[test]
    fn test_requires_ocr_explicit_metadata_true() {
        let mut metadata = HashMap::new();
        metadata.insert("requires_ocr".to_string(), serde_json::json!(true));

        let fixture = Fixture {
            document: PathBuf::from("test.pdf"),
            file_type: "pdf".to_string(),
            file_size: 1024,
            expected_frameworks: vec![],
            metadata,
            ground_truth: None,
        };

        // PDF normally doesn't require OCR, but metadata overrides this
        assert!(fixture.requires_ocr());
    }

    #[test]
    fn test_requires_ocr_explicit_metadata_false() {
        let mut metadata = HashMap::new();
        metadata.insert("requires_ocr".to_string(), serde_json::json!(false));

        let fixture = Fixture {
            document: PathBuf::from("test.png"),
            file_type: "png".to_string(),
            file_size: 1024,
            expected_frameworks: vec![],
            metadata,
            ground_truth: None,
        };

        // PNG normally requires OCR, but metadata overrides this
        assert!(!fixture.requires_ocr());
    }

    #[test]
    fn test_requires_ocr_case_insensitive() {
        let fixture = Fixture {
            document: PathBuf::from("test.JPG"),
            file_type: "JPG".to_string(),
            file_size: 1024,
            expected_frameworks: vec![],
            metadata: HashMap::new(),
            ground_truth: None,
        };

        assert!(fixture.requires_ocr());
    }

    #[test]
    fn test_ground_truth_file_existence_validation() {
        let temp_dir = TempDir::new().unwrap();
        let fixture_path = temp_dir.path().join("test.json");

        let fixture = Fixture {
            document: PathBuf::from("test.pdf"),
            file_type: "pdf".to_string(),
            file_size: 1024,
            expected_frameworks: vec![],
            metadata: HashMap::new(),
            ground_truth: Some(GroundTruth {
                text_file: PathBuf::from("nonexistent_ground_truth.txt"),
                source: "manual".to_string(),
            }),
        };

        std::fs::write(&fixture_path, serde_json::to_string(&fixture).unwrap()).unwrap();

        // Should fail because ground truth file doesn't exist
        let result = Fixture::from_file(&fixture_path);
        assert!(result.is_err());
        match result {
            Err(Error::InvalidFixture { reason, .. }) => {
                assert!(reason.contains("ground truth file not found"));
            }
            _ => panic!("Expected InvalidFixture error with 'ground truth file not found'"),
        }
    }

    #[test]
    fn test_ground_truth_file_existence_validation_success() {
        let temp_dir = TempDir::new().unwrap();
        let fixture_path = temp_dir.path().join("test.json");
        let ground_truth_path = temp_dir.path().join("ground_truth.txt");

        // Create the ground truth file
        std::fs::write(&ground_truth_path, "Sample ground truth text").unwrap();

        let fixture = Fixture {
            document: PathBuf::from("test.pdf"),
            file_type: "pdf".to_string(),
            file_size: 1024,
            expected_frameworks: vec![],
            metadata: HashMap::new(),
            ground_truth: Some(GroundTruth {
                text_file: PathBuf::from("ground_truth.txt"),
                source: "manual".to_string(),
            }),
        };

        std::fs::write(&fixture_path, serde_json::to_string(&fixture).unwrap()).unwrap();

        // Should succeed because ground truth file exists
        let result = Fixture::from_file(&fixture_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fixture_load_with_mixed_success_and_failure() {
        let _lock = ENV_LOCK.lock().unwrap();
        let temp_dir = TempDir::new().unwrap();

        // Create valid fixture
        let valid_fixture_path = temp_dir.path().join("valid.json");
        let valid_fixture = Fixture {
            document: PathBuf::from("test.pdf"),
            file_type: "pdf".to_string(),
            file_size: 1024,
            expected_frameworks: vec![],
            metadata: HashMap::new(),
            ground_truth: None,
        };
        std::fs::write(&valid_fixture_path, serde_json::to_string(&valid_fixture).unwrap()).unwrap();

        // Create invalid fixture (missing ground truth file)
        let invalid_fixture_path = temp_dir.path().join("invalid.json");
        let invalid_fixture = Fixture {
            document: PathBuf::from("test.pdf"),
            file_type: "pdf".to_string(),
            file_size: 1024,
            expected_frameworks: vec![],
            metadata: HashMap::new(),
            ground_truth: Some(GroundTruth {
                text_file: PathBuf::from("nonexistent.txt"),
                source: "manual".to_string(),
            }),
        };
        std::fs::write(&invalid_fixture_path, serde_json::to_string(&invalid_fixture).unwrap()).unwrap();

        unsafe {
            std::env::remove_var("PROFILING_FIXTURES");
        }

        let mut manager = FixtureManager::new();
        // Should succeed overall (returns Ok), but report failed fixtures
        let result = manager.load_fixtures_from_dir(temp_dir.path());
        assert!(result.is_ok());

        // Should have loaded only the valid fixture
        assert_eq!(manager.len(), 1);
    }
}
